//! Authoritative NTFS source acquisition and bounded protected snapshot handoff.
use super::contract::{DecimalU64, ProtocolErrorCode, SourceState};
use super::{
    backing::{PageStore, BYTE_PAGE},
    checked_add, failure,
    hash::Sha256,
    native, Result,
};
use serde::Serialize;
use std::{
    fs::File,
    path::{Component, Path, PathBuf, Prefix},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceIdentity {
    pub volume_serial: DecimalU64,
    pub file_id: [u8; 16],
    pub bytes: DecimalU64,
    pub creation_time: DecimalU64,
    pub modified_time: DecimalU64,
    pub links: u32,
    pub attributes: u32,
    pub reparse_point: bool,
    pub reparse_tag: u32,
    pub named_stream: bool,
    pub share_mode: u32,
    pub source_generation: DecimalU64,
}
fn timestamp(parts: [u32; 2]) -> u64 {
    (parts[1] as u64) << 32 | parts[0] as u64
}
fn next_generation() -> Result<u64> {
    NEXT_GENERATION
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
            value.checked_add(1)
        })
        .map_err(|_| {
            failure(
                ProtocolErrorCode::ResourceLimit,
                "source generation exhausted",
            )
        })
}

pub(crate) struct ProtectedSource {
    file: File,
    _ancestors: Vec<File>,
    oplock: native::Oplock,
    pub identity: SourceIdentity,
    baseline: native::FileInfo,
    authoritative_path: Vec<u16>,
    pub bytes_read: AtomicU64,
}
impl ProtectedSource {
    /// No caller-controlled label is accepted. Authorization runs before even path validation.
    pub(crate) fn open(window: &tauri::WebviewWindow, path: &Path) -> Result<Self> {
        crate::stack_popup::auth::authorize_stack_command(
            window,
            crate::stack_popup::auth::StackCommandAuth::AllowedCallers {
                command: "open_stack_text_document",
                callers: &["stack-popup"],
            },
        )
        .map_err(|_| {
            failure(
                ProtocolErrorCode::Unauthorized,
                "text document caller is not authorized",
            )
        })?;
        Self::open_authorized_path(path, None)
    }
    // The experiment entry is crate-private and not IPC-registered. Expected identity
    // represents an earlier selection observation; it is never used as authority.
    pub(super) fn open_authorized_path(
        path: &Path,
        expected: Option<(u64, [u8; 16])>,
    ) -> Result<Self> {
        let (root, components) = approved_path(path)?;
        let mut ancestors = Vec::with_capacity(components.len());
        let mut current = root.clone();
        for component in components.iter().take(components.len().saturating_sub(1)) {
            let directory = native::open(
                &current,
                0x80,
                native::SHARE_READ,
                native::OPEN_REPARSE | native::BACKUP,
            )?;
            let info = native::info(&directory)?;
            if info.attributes & 0x400 != 0 || info.attributes & 0x10 == 0 {
                return Err(failure(
                    ProtocolErrorCode::UnsupportedTarget,
                    "redirecting ancestor refused",
                ));
            }
            ancestors.push(directory);
            current.push(component);
        }
        // Also pin the direct parent (the loop starts with the root).
        let directory = native::open(
            &current,
            0x80,
            native::SHARE_READ,
            native::OPEN_REPARSE | native::BACKUP,
        )?;
        let parent_info = native::info(&directory)?;
        if parent_info.attributes & 0x400 != 0 || parent_info.attributes & 0x10 == 0 {
            return Err(failure(
                ProtocolErrorCode::UnsupportedTarget,
                "redirecting parent refused",
            ));
        }
        ancestors.push(directory);
        let file = native::open(
            path,
            native::READ | native::WRITE,
            native::SHARE_READ,
            native::OPEN_REPARSE | native::OVERLAPPED,
        )?;
        native::regular_ntfs(&file, &root)?;
        let baseline = native::info(&file)?;
        let (tag_attributes, reparse_tag) = native::attribute_tag(&file)?;
        classify(&baseline, tag_attributes, reparse_tag)?;
        let (volume, id) = native::file_id(&file)?;
        if expected.is_some_and(|expected| expected != (volume, id)) {
            return Err(failure(
                ProtocolErrorCode::SourceChanged,
                "selected target identity changed before open",
            ));
        }
        let authoritative_path = native::final_path(&file)?;
        let oplock = native::Oplock::acquire_read(&file)?;
        oplock.intact()?;
        let identity = SourceIdentity {
            volume_serial: volume.into(),
            file_id: id,
            bytes: (((baseline.size_high as u64) << 32) | baseline.size_low as u64).into(),
            creation_time: timestamp(baseline.creation).into(),
            modified_time: timestamp(baseline.write).into(),
            links: baseline.links,
            attributes: baseline.attributes,
            reparse_point: tag_attributes & 0x400 != 0,
            reparse_tag,
            // approved_path rejects named streams before any content-bearing open.
            named_stream: false,
            share_mode: native::SHARE_READ,
            source_generation: next_generation()?.into(),
        };
        Ok(Self {
            file,
            _ancestors: ancestors,
            oplock,
            identity,
            baseline,
            authoritative_path,
            bytes_read: AtomicU64::new(0),
        })
    }
    pub(crate) fn read(
        &self,
        offset: u64,
        output: &mut [u8],
        cancel: &AtomicBool,
    ) -> Result<usize> {
        self.oplock.intact()?;
        let end = checked_add(offset, output.len() as u64)?;
        if offset > self.identity.bytes.as_u64() || end > self.identity.bytes.as_u64() {
            return Err(failure(
                ProtocolErrorCode::InvalidTextBoundary,
                "read outside authoritative extent",
            ));
        }
        let count = native::read_at(&self.file, offset, output, cancel)?;
        self.oplock.intact()?;
        self.bytes_read
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(count as u64)
            })
            .map_err(|_| failure(ProtocolErrorCode::ResourceLimit, "I/O counter overflow"))?;
        if count != output.len() {
            return Err(failure(
                ProtocolErrorCode::SourceChanged,
                "protected source short read",
            ));
        }
        Ok(count)
    }
    pub(crate) fn verify_identity(&self) -> Result<()> {
        self.oplock.intact()?;
        let current = native::info(&self.file)?;
        let (tag_attributes, reparse_tag) = native::attribute_tag(&self.file)?;
        let (volume, id) = native::file_id(&self.file)?;
        if current.size_high != self.baseline.size_high
            || current.size_low != self.baseline.size_low
            || current.write != self.baseline.write
            || current.links != 1
            || current.attributes != self.baseline.attributes
            || tag_attributes != self.identity.attributes
            || reparse_tag != self.identity.reparse_tag
            || volume != self.identity.volume_serial.as_u64()
            || id != self.identity.file_id
            || native::final_path(&self.file)? != self.authoritative_path
        {
            return Err(failure(
                ProtocolErrorCode::SourceChanged,
                "source identity or metadata changed during protected copy",
            ));
        }
        Ok(())
    }
}

fn approved_path(path: &Path) -> Result<(PathBuf, Vec<std::ffi::OsString>)> {
    native::wide(path)?;
    let text = path.to_string_lossy();
    if text.get(2..).is_some_and(|rest| rest.contains(':')) {
        return Err(failure(
            ProtocolErrorCode::UnsupportedTarget,
            "named stream refused before content I/O",
        ));
    }
    if text.contains('/') || text.contains('\0') {
        return Err(failure(
            ProtocolErrorCode::UnsupportedTarget,
            "named stream or noncanonical path refused",
        ));
    }
    let mut iter = path.components();
    let drive = match iter.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(letter) => letter,
            _ => {
                return Err(failure(
                    ProtocolErrorCode::UnsupportedTarget,
                    "device/UNC namespace refused",
                ))
            }
        },
        _ => {
            return Err(failure(
                ProtocolErrorCode::UnsupportedTarget,
                "absolute disk path required",
            ))
        }
    };
    if !matches!(iter.next(), Some(Component::RootDir)) {
        return Err(failure(
            ProtocolErrorCode::UnsupportedTarget,
            "drive-relative path refused",
        ));
    }
    let mut components = Vec::new();
    // Explicit raw dot and trailing aliases are rejected rather than normalized.
    for raw in text.get(3..).unwrap_or("").split('\\') {
        if raw.is_empty() || raw == "." || raw == ".." || raw.ends_with('.') || raw.ends_with(' ') {
            return Err(failure(
                ProtocolErrorCode::UnsupportedTarget,
                "ambiguous path component refused",
            ));
        }
        let stem = raw.split('.').next().unwrap_or("").to_ascii_uppercase();
        if matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit()
        {
            return Err(failure(
                ProtocolErrorCode::UnsupportedTarget,
                "device alias refused",
            ));
        }
    }
    for part in iter {
        match part {
            Component::Normal(name) => components.push(name.to_os_string()),
            _ => {
                return Err(failure(
                    ProtocolErrorCode::UnsupportedTarget,
                    "noncanonical component refused",
                ))
            }
        }
    }
    if components.is_empty() || components.len() > 256 {
        return Err(failure(
            ProtocolErrorCode::UnsupportedTarget,
            "directory root or excessive ancestry refused",
        ));
    }
    Ok((PathBuf::from(format!("{}:\\", drive as char)), components))
}
fn classify(info: &native::FileInfo, tag_attributes: u32, reparse_tag: u32) -> Result<()> {
    if info.attributes & 1 != 0 {
        return Err(failure(
            ProtocolErrorCode::Readonly,
            "read-only target refused",
        ));
    }
    // Directory, reparse, sparse, compressed, encrypted, offline, recall-on-open/data.
    if info.attributes & (0x10 | 0x400 | 0x200 | 0x800 | 0x4000 | 0x1000 | 0x40000 | 0x400000) != 0
        || tag_attributes & 0x400 != 0
        || reparse_tag != 0
        || info.links != 1
    {
        return Err(failure(
            ProtocolErrorCode::UnsupportedTarget,
            "special or aliased target refused",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CopyPhase {
    Copying,
    Verifying,
    Complete,
    Cancelled,
    Failed,
}
pub(crate) struct ProtectedCopy {
    source: Option<ProtectedSource>,
    pub original: PageStore,
    pub phase: CopyPhase,
    pub copied: u64,
    pub verified: u64,
    total: u64,
    copied_hash: Sha256,
    verified_hash: Sha256,
    pub digest: Option<String>,
    failure_state: Option<SourceState>,
}
impl ProtectedCopy {
    pub(crate) fn new(source: ProtectedSource, original: PageStore) -> Self {
        let total = source.identity.bytes.as_u64();
        Self {
            source: Some(source),
            original,
            phase: CopyPhase::Copying,
            copied: 0,
            verified: 0,
            total,
            copied_hash: Sha256::new(),
            verified_hash: Sha256::new(),
            digest: None,
            failure_state: None,
        }
    }
    pub(crate) fn state(&self) -> SourceState {
        match self.phase {
            CopyPhase::Complete => SourceState::Ready,
            CopyPhase::Copying | CopyPhase::Verifying => SourceState::Snapshotting,
            CopyPhase::Cancelled => SourceState::Cancelled,
            CopyPhase::Failed => self.failure_state.unwrap_or(SourceState::ReadLimited),
        }
    }
    pub(crate) fn recovery_complete(&self) -> bool {
        false
    } // P04 must authenticate/verify a durable recovery root, not merely a base copy.
    pub(crate) fn source(&self) -> Option<&ProtectedSource> {
        self.source.as_ref()
    }
    pub(crate) fn read_original(
        &mut self,
        offset: u64,
        output: &mut [u8],
        cancel: &AtomicBool,
    ) -> Result<usize> {
        if let Some(source) = &self.source {
            source.read(offset, output, cancel)
        } else if self.phase == CopyPhase::Complete {
            super::backing::read_original(&mut self.original, offset, output)
        } else {
            Err(failure(
                ProtocolErrorCode::SourceChanged,
                "no verified original backing",
            ))
        }
    }
    /// One bounded job; caller's scheduler may interleave urgent demand between steps.
    pub(crate) fn step(&mut self, cancel: &AtomicBool) -> Result<CopyPhase> {
        let result = self.step_inner(cancel);
        if let Err(error) = &result {
            if error.code == ProtocolErrorCode::Cancelled {
                self.phase = CopyPhase::Cancelled;
                self.failure_state = Some(SourceState::Cancelled);
            } else {
                self.phase = CopyPhase::Failed;
                self.failure_state = Some(source_state_for_error(error));
            }
        }
        result
    }
    fn step_inner(&mut self, cancel: &AtomicBool) -> Result<CopyPhase> {
        if cancel.load(Ordering::Acquire) {
            return Err(failure(
                ProtocolErrorCode::Cancelled,
                "snapshot cancelled; incomplete backing retained",
            ));
        }
        match self.phase {
            CopyPhase::Copying => {
                if self.copied == self.total {
                    self.original.flush()?;
                    self.original.evict_cache();
                    self.phase = CopyPhase::Verifying;
                    return Ok(self.phase);
                }
                let count = (self.total - self.copied).min(BYTE_PAGE as u64) as usize;
                let mut bytes = vec![0; count];
                self.source
                    .as_ref()
                    .ok_or_else(|| {
                        failure(ProtocolErrorCode::SourceChanged, "protected source missing")
                    })?
                    .read(self.copied, &mut bytes, cancel)?;
                self.original.append(&bytes)?;
                self.copied_hash.update(&bytes);
                self.copied = checked_add(self.copied, count as u64)?;
            }
            CopyPhase::Verifying => {
                if self.verified < self.total {
                    let count = (self.total - self.verified).min(BYTE_PAGE as u64) as usize;
                    let stored = self.original.read(self.verified / BYTE_PAGE as u64)?;
                    let mut source_bytes = vec![0; count];
                    self.source
                        .as_ref()
                        .ok_or_else(|| {
                            failure(ProtocolErrorCode::SourceChanged, "protected source missing")
                        })?
                        .read(self.verified, &mut source_bytes, cancel)?;
                    if stored.get(..count) != Some(source_bytes.as_slice()) {
                        return Err(failure(
                            ProtocolErrorCode::SourceChanged,
                            "snapshot differs from protected original",
                        ));
                    }
                    self.verified_hash.update(&source_bytes);
                    self.verified = checked_add(self.verified, count as u64)?;
                } else {
                    self.source
                        .as_ref()
                        .ok_or_else(|| {
                            failure(ProtocolErrorCode::SourceChanged, "protected source missing")
                        })?
                        .verify_identity()?;
                    let copied = self.copied_hash.clone().finalize();
                    let verified = self.verified_hash.clone().finalize();
                    if copied != verified {
                        return Err(failure(
                            ProtocolErrorCode::SourceChanged,
                            "snapshot digest mismatch",
                        ));
                    }
                    self.digest = Some(format!("{verified:x}"));
                    self.phase = CopyPhase::Complete;
                    self.source.take(); // Restrictions released only after complete authenticated reread + identity verification.
                }
            }
            CopyPhase::Complete => {}
            CopyPhase::Cancelled | CopyPhase::Failed => {
                return Err(failure(
                    ProtocolErrorCode::SourceChanged,
                    "failed copy cannot become immutable",
                ))
            }
        }
        Ok(self.phase)
    }
}

pub(crate) fn source_state_for_error(error: &super::contract::ProtocolError) -> SourceState {
    match error.code {
        ProtocolErrorCode::Cancelled => SourceState::Cancelled,
        ProtocolErrorCode::ResourceLimit => SourceState::QuotaExceeded,
        ProtocolErrorCode::SourceChanged => SourceState::Changed,
        ProtocolErrorCode::SharingViolation => SourceState::Conflict,
        ProtocolErrorCode::Readonly => SourceState::Readonly,
        ProtocolErrorCode::EncodingRequired => SourceState::DecisionRequired,
        _ => SourceState::ReadLimited,
    }
}
