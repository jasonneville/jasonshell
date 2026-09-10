//! P02 end-to-end developer kernel. Privileged entry requires an actual Tauri
//! window. No command registration and no production caller exist in this phase.
use super::contract::{
    LineBreakKind, ProtocolErrorCode, SessionEnvelope, SourceState, TextEncoding, ViewLease,
    FIRST_READ_BYTES,
};
use super::{
    backing::{self, PageStore, Quota, BYTE_PAGE, NODE_PAGE},
    decode::{self, Checkpoint},
    failure,
    index::{CheckpointIndex, PagedDocument},
    lease::{self, LeaseTags},
    native,
    source::{CopyPhase, ProtectedCopy, ProtectedSource},
    Result,
};
use std::{
    fs,
    path::Path,
    sync::{atomic::AtomicBool, Arc, Mutex},
};

pub(crate) struct FeasibilitySession {
    session_id: String,
    source_generation: u64,
    view_generation: u64,
    pub copy: ProtectedCopy,
    pub document: PagedDocument,
    pub index: CheckpointIndex,
    encoding: TextEncoding,
    bom: usize,
    newline: LineBreakKind,
    lease: ViewLease,
    invalid_at: Option<u64>,
    // Pins the private directory against replacement; dirty stores are never
    // deleted by Drop. P04 owns recover/discard classification and cleanup.
    _directory: fs::File,
}
impl FeasibilitySession {
    pub(crate) fn open(window: &tauri::WebviewWindow, path: &Path) -> Result<Self> {
        let source = ProtectedSource::open(window, path)?;
        let source_generation = source.identity.source_generation.as_u64();
        let total = source.identity.bytes.as_u64();
        let local = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
            failure(
                ProtocolErrorCode::ResourceLimit,
                "private per-user application data unavailable",
            )
        })?;
        let local = std::path::PathBuf::from(local);
        let required = backing::reservation(total, 64 * 1024 * 1024, 64 * 1024 * 1024)?;
        if native::free_space(&local)? < required {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "insufficient private backing/staging/backup reservation",
            ));
        }
        let token = native::random::<16>()?;
        let session_id: String = token.iter().map(|byte| format!("{byte:02x}")).collect();
        let directory = local.join(format!("JasonShell-P02-{session_id}"));
        native::private_directory(&directory)?;
        let directory_handle = native::open(
            &directory,
            0x80,
            native::SHARE_READ,
            native::BACKUP | native::OPEN_REPARSE,
        )?;
        let quota = Arc::new(Mutex::new(Quota::new(required)));
        let original = PageStore::create(
            &directory.join("original.pages"),
            BYTE_PAGE,
            2 * BYTE_PAGE,
            quota.clone(),
        )?;
        let nodes = PageStore::create(
            &directory.join("nodes.pages"),
            NODE_PAGE,
            2 * NODE_PAGE,
            quota.clone(),
        )?;
        let added = PageStore::create(
            &directory.join("added.pages"),
            BYTE_PAGE,
            BYTE_PAGE,
            quota.clone(),
        )?;
        let index_pages = PageStore::create(
            &directory.join("index.pages"),
            NODE_PAGE,
            2 * NODE_PAGE,
            quota,
        )?;
        let mut copy = ProtectedCopy::new(source, original);
        let document = PagedDocument::new(nodes, added, total)?;
        let mut prefix = vec![0; total.min(FIRST_READ_BYTES as u64) as usize];
        copy.read_original(0, &mut prefix, &AtomicBool::new(false))?;
        let (encoding, bom) = decode::detect(&prefix, None)?;
        let checkpoint = Checkpoint::new(encoding, bom as u64);
        let lease = lease::build_lease(
            LeaseTags {
                session: &session_id,
                lease: "prefix-1",
                source: source_generation,
                revision: 0,
                view: 1,
            },
            checkpoint.clone(),
            &prefix[bom..],
            total == prefix.len() as u64,
            Some(1),
        )?;
        let newline = lease::insertion_newline(&lease);
        let invalid_at = lease.invalid_at.as_ref().map(|value| value.as_u64());
        let index = CheckpointIndex::new(index_pages, checkpoint)?;
        Ok(Self {
            session_id,
            source_generation,
            view_generation: 1,
            copy,
            document,
            index,
            encoding,
            bom,
            newline,
            lease,
            invalid_at,
            _directory: directory_handle,
        })
    }
    pub(crate) fn lease(&self) -> &ViewLease {
        &self.lease
    }
    pub(crate) fn source_state(&self) -> SourceState {
        if self.lease.source_state == SourceState::DecisionRequired {
            SourceState::DecisionRequired
        } else {
            self.copy.state()
        }
    }
    pub(crate) fn replace(
        &mut self,
        window: &tauri::WebviewWindow,
        envelope: &SessionEnvelope,
        start: usize,
        end: usize,
        text: &str,
    ) -> Result<()> {
        self.lease.authorize_for(envelope, window.label())?;
        if self.invalid_at.is_some()
            || matches!(self.copy.phase, CopyPhase::Failed | CopyPhase::Cancelled)
        {
            return Err(failure(
                ProtocolErrorCode::EncodingRequired,
                "source is not safe for further edits; accepted draft retained",
            ));
        }
        lease::replace_local(
            &mut self.document,
            &self.lease,
            envelope,
            window,
            start,
            end,
            text,
            self.encoding,
            self.newline,
        )
    }
    pub(crate) fn refresh_prefix(&mut self, window: &tauri::WebviewWindow) -> Result<&ViewLease> {
        if window.label() != "stack-popup" {
            return Err(failure(
                ProtocolErrorCode::Unauthorized,
                "session is owned by stack-popup",
            ));
        }
        let total = self.document.metrics()?.bytes;
        let mut prefix = vec![0; total.min(FIRST_READ_BYTES as u64) as usize];
        let cancel = AtomicBool::new(false);
        self.document.read_range(0, &mut prefix, |offset, bytes| {
            self.copy.read_original(offset, bytes, &cancel)
        })?;
        self.view_generation = super::checked_add(self.view_generation, 1)?;
        self.lease = lease::build_lease(
            LeaseTags {
                session: &self.session_id,
                lease: &format!("prefix-{}", self.view_generation),
                source: self.source_generation,
                revision: self.document.revision,
                view: self.view_generation,
            },
            Checkpoint::new(self.encoding, self.bom as u64),
            &prefix[self.bom..],
            total == prefix.len() as u64,
            Some(1),
        )?;
        self.invalid_at = self.lease.invalid_at.as_ref().map(|value| value.as_u64());
        Ok(&self.lease)
    }
    /// Off-thread bounded work unit; never call under an actor/session-map lock.
    pub(crate) fn index_step(&mut self, cancel: &AtomicBool) -> Result<()> {
        if self.index.complete || self.invalid_at.is_some() {
            return Ok(());
        }
        let total = self
            .copy
            .source()
            .map(|source| source.identity.bytes.as_u64())
            .unwrap_or(self.copy.copied);
        let offset = self.index.covered;
        let count = (total - offset).min(BYTE_PAGE as u64) as usize;
        let mut bytes = vec![0; count];
        self.copy.read_original(offset, &mut bytes, cancel)?;
        match self.index.push(&bytes, offset + count as u64 == total) {
            Ok(()) => Ok(()),
            Err(error) => {
                if error.code == ProtocolErrorCode::EncodingRequired {
                    self.invalid_at = self.index.decoder.invalid_at;
                    self.lease.invalid_at = self.invalid_at.map(Into::into);
                    self.lease.source_state = SourceState::DecisionRequired;
                    self.lease.validate()?;
                }
                Err(error)
            }
        }
    }
}

pub(super) fn native_probe_root(requested: &Path) -> Result<std::path::PathBuf> {
    use std::os::windows::fs::MetadataExt;
    if !requested.is_absolute() {
        return Err(failure(
            ProtocolErrorCode::UnsupportedTarget,
            "native probe requires absolute evidence root",
        ));
    }
    for ancestor in requested.ancestors() {
        if fs::symlink_metadata(ancestor)
            .map_err(super::io_failure)?
            .file_attributes()
            & 0x400
            != 0
        {
            return Err(failure(
                ProtocolErrorCode::UnsupportedTarget,
                "native probe evidence ancestry cannot redirect",
            ));
        }
    }
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| {
            failure(
                ProtocolErrorCode::UnsupportedTarget,
                "compiled repository root unavailable",
            )
        })?;
    let approved = repository
        .join("test-results")
        .join("stack-text-editor")
        .join("P02")
        .canonicalize()
        .map_err(super::io_failure)?;
    let actual = requested.canonicalize().map_err(super::io_failure)?;
    if actual == approved || !actual.starts_with(&approved) || !actual.is_dir() {
        return Err(failure(
            ProtocolErrorCode::UnsupportedTarget,
            "native probe root must be an existing P02 run subdirectory",
        ));
    }
    // Windows canonicalize returns extended syntax; the source policy requires
    // ordinary DOS syntax. Convert only after authoritative root validation.
    let text = actual.to_str().ok_or_else(|| {
        failure(
            ProtocolErrorCode::UnsupportedTarget,
            "native probe root is not Unicode",
        )
    })?;
    Ok(std::path::PathBuf::from(
        crate::stack_popup::paths::stack_display_path_string(text),
    ))
}

/// Opt-in native caller proof. This adds no IPC command and is absent from release.
#[cfg(debug_assertions)]
pub(crate) fn install_native_probe(app: &tauri::AppHandle) {
    use tauri::Manager;
    let Some(requested_root) = std::env::var_os("P02_NATIVE_PROBE_ROOT") else {
        return;
    };
    let Some(allowed) = app.get_webview_window("stack-popup") else {
        eprintln!("P02 native probe: stack-popup unavailable");
        return;
    };
    let Some(unauthorized) = app.get_webview_window("top-bar") else {
        eprintln!("P02 native probe: control window unavailable");
        return;
    };
    let probe_app = app.clone();
    let spawned = std::thread::Builder::new().name("p02-native-caller-probe".into()).spawn(move || {
        use std::{io::Write, os::windows::ffi::OsStringExt};
        use super::hash::Sha256;
        let root = match native_probe_root(Path::new(&requested_root)) { Ok(root) => root, Err(error) => { eprintln!("P02 native probe refused: {}",error.message); return; } };
        let run = || -> Result<serde_json::Value> {
            let fixture_directory = root.join("native-caller-fixture");
            native::private_directory(&fixture_directory)?;
            let fixture = fixture_directory.join("synthetic.txt");
            let raw = b"one\r\ntwo\nlast".repeat(8192);
            let mut target = fs::OpenOptions::new().write(true).create_new(true).open(&fixture).map_err(super::io_failure)?;
            target.write_all(&raw).map_err(super::io_failure)?; target.sync_all().map_err(super::io_failure)?; drop(target);
            // Both real-window negative calls precede any allowed content disclosure.
            for denied_path in [&fixture, &fixture_directory.join("nonexistent.txt")] {
                match FeasibilitySession::open(&unauthorized,denied_path) {
                    Err(error) if error.code == ProtocolErrorCode::Unauthorized => {}
                    _ => return Err(failure(ProtocolErrorCode::Unauthorized,"actual unauthorized window was not rejected before target processing")),
                }
            }
            let mut session = FeasibilitySession::open(&allowed,&fixture)?;
            let before = session.lease().clone();
            let envelope = SessionEnvelope { schema_revision: super::contract::SCHEMA_REVISION.into(), session_id: before.session_id.clone(), source_generation: before.source_generation.clone(), document_revision: before.document_revision.clone(), view_generation: before.view_generation.clone(), lease_id: Some(before.lease_id.clone()) };
            if session.copy.copied != 0 || session.index.covered != 0 { return Err(failure(ProtocolErrorCode::IoFailure,"prefix unexpectedly waited for copy/index")); }
            session.replace(&allowed,&envelope,0,3,"edited")?;
            let accepted_before_copy = session.document.revision == 1 && session.copy.copied == 0 && session.index.covered == 0;
            let view = session.refresh_prefix(&allowed)?;
            if !accepted_before_copy || !view.segments[0].text.starts_with("edited") { return Err(failure(ProtocolErrorCode::IoFailure,"native caller prefix edit was not observable before copy")); }
            while session.copy.phase != CopyPhase::Complete { session.copy.step(&AtomicBool::new(false))?; }
            while !session.index.complete { session.index_step(&AtomicBool::new(false))?; }
            session.document.flush()?;
            let private_path = std::path::PathBuf::from(std::ffi::OsString::from_wide(&native::final_path(&session._directory)?));
            let result = serde_json::json!({"phase":"P02","test":"T02-01/T02-02-native-caller","status":"PASS","authorizationMode":"actual-webview-window","actualAuthorizedWindow":allowed.label(),"actualUnauthorizedWindow":unauthorized.label(),"unauthorizedExistingAndMissingTargetRejected":true,"contentDisclosedOnRejectedOpen":false,"sourceBytes":raw.len().to_string(),"sourceSha256":format!("{:x}",Sha256::digest(&raw)),"firstReadCeiling":FIRST_READ_BYTES,"acceptedRevisionBeforeCopy":"1","copyBytesAtAcceptance":"0","indexedBytesAtAcceptance":"0","snapshotComplete":true,"recoveryComplete":session.copy.recovery_complete(),"schemaRevision":super::contract::SCHEMA_REVISION,"feasibilitySchemaRevision":super::contract::P02_FEASIBILITY_SCHEMA_REVISION,"nativeProcess":native::process_sample()?,"privateStoreId":session.session_id});
            drop(session);
            // Explicit discard of this probe's synthetic draft only; never generic eviction.
            fs::remove_dir_all(private_path).map_err(super::io_failure)?;
            Ok(result)
        };
        let result = match run() { Ok(value) => value, Err(error) => serde_json::json!({"phase":"P02","test":"T02-01/T02-02-native-caller","status":"FAIL","code":format!("{:?}",error.code),"message":error.message}) };
        let serialized = match serde_json::to_vec_pretty(&result) { Ok(bytes) => bytes, Err(_) => { eprintln!("P02 native probe serialization failed"); return; } };
        match fs::OpenOptions::new().write(true).create_new(true).open(root.join("native-caller-result.json")).and_then(|mut file| { file.write_all(&serialized)?; file.sync_all() }) {
            Ok(()) => eprintln!("P02 native caller probe completed; inspect native-caller-result.json"),
            Err(error) => eprintln!("P02 native caller probe evidence write failed (os={:?})",error.raw_os_error()),
        }
        probe_app.exit(0);
    });
    if let Err(error) = spawned {
        eprintln!(
            "P02 native caller probe worker unavailable (os={:?})",
            error.raw_os_error()
        );
    }
}
