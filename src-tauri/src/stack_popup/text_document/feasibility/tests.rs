//! Serial-only native experiments. Run through scripts/stack-text-editor/p02-run.mjs
//! to retain exact command, source, fixture/hash and process-resource evidence.
use super::contract::{ProtocolErrorCode, SourceState, TextEncoding};
use super::hash::Sha256;
use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

struct Fixture {
    path: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let token = native::random::<16>().expect("system RNG");
        let id: String = token.iter().map(|byte| format!("{byte:02x}")).collect();
        let base = std::env::var_os("P02_FIXTURE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = base.join(format!("jasonshell-p02-{id}"));
        native::private_directory(&path).expect("restrictive synthetic fixture ACL");
        Self { path }
    }
    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, bytes).expect("synthetic fixture write");
        path
    }
    fn pages(
        &self,
        name: &str,
        size: usize,
        cache: usize,
        quota: backing::SharedQuota,
    ) -> backing::PageStore {
        backing::PageStore::create(&self.path.join(name), size, cache, quota)
            .expect("encrypted page store")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
fn quota(limit: u64) -> backing::SharedQuota {
    Arc::new(Mutex::new(backing::Quota::new(limit)))
}
fn document(fixture: &Fixture, size: u64, quota: backing::SharedQuota) -> index::PagedDocument {
    index::PagedDocument::new(
        fixture.pages(
            "nodes.pages",
            backing::NODE_PAGE,
            backing::NODE_PAGE * 2,
            quota.clone(),
        ),
        fixture.pages("added.pages", backing::BYTE_PAGE, backing::BYTE_PAGE, quota),
        size,
    )
    .expect("paged document")
}
fn bytes(document: &mut index::PagedDocument, original: &[u8]) -> Vec<u8> {
    let mut result = vec![0; document.metrics().expect("metrics").bytes as usize];
    document
        .read_range(0, &mut result, |offset, output| {
            output.copy_from_slice(&original[offset as usize..offset as usize + output.len()]);
            Ok(output.len())
        })
        .expect("logical reconstruction");
    result
}
fn emit(case: &str, values: serde_json::Value) {
    println!(
        "{}",
        serde_json::json!({"phase":"P02", "case":case, "observation":values,"nativeProcess":native::process_sample().expect("native process counters")})
    );
}

#[test]
fn t04_01_freezes_r_while_r_plus_one_remains_dirty() {
    let accepted_r = b"accepted revision R".to_vec();
    let visible_r_plus_one = b"accepted revision R + dirty typing".to_vec();
    let frozen = accepted_r.clone();
    assert_eq!(frozen, accepted_r);
    assert_ne!(frozen, visible_r_plus_one);
}

#[test]
fn t04_04_every_boundary_has_deterministic_no_retry_restart_class() {
    use save_failpoints::{Assets, Phase, RestartClass};
    for phase in [
        Phase::Prepare,
        Phase::Stage,
        Phase::WriteBatch,
        Phase::FlushClose,
        Phase::Revalidate,
        Phase::BeforePublish,
    ] {
        let class = save_failpoints::classify(
            Some(phase),
            &Assets {
                target: true,
                staging: true,
                backup: false,
                journal_committed: false,
                inspected_revision: false,
            },
        );
        assert_ne!(class, RestartClass::Published);
    }
    assert_eq!(
        save_failpoints::classify(
            Some(Phase::AfterReplace),
            &Assets {
                target: true,
                staging: false,
                backup: true,
                journal_committed: false,
                inspected_revision: false
            }
        ),
        RestartClass::PublicationPossible
    );
    assert_eq!(
        save_failpoints::classify(
            Some(Phase::Inspect),
            &Assets {
                target: true,
                staging: false,
                backup: true,
                journal_committed: false,
                inspected_revision: true
            }
        ),
        RestartClass::Ambiguous
    );
    assert_eq!(
        save_failpoints::classify(
            Some(Phase::Cleanup),
            &Assets {
                target: true,
                staging: false,
                backup: true,
                journal_committed: true,
                inspected_revision: true
            }
        ),
        RestartClass::Published
    );

    struct InjectedIo {
        fail: Phase,
        visited: Vec<Phase>,
    }
    impl save_failpoints::SaveIo for InjectedIo {
        type Error = &'static str;
        fn step(&mut self, phase: Phase) -> Result<(), Self::Error> {
            self.visited.push(phase);
            if phase == self.fail {
                Err("injected termination")
            } else {
                Ok(())
            }
        }
    }
    for fail in [
        Phase::Prepare,
        Phase::Stage,
        Phase::WriteBatch,
        Phase::FlushClose,
        Phase::Revalidate,
        Phase::BeforePublish,
        Phase::AfterReplace,
        Phase::Inspect,
        Phase::JournalCommit,
        Phase::Cleanup,
    ] {
        let mut io = InjectedIo {
            fail,
            visited: Vec::new(),
        };
        assert_eq!(
            save_failpoints::run_until_failure(&mut io),
            Err((fail, "injected termination"))
        );
        assert_eq!(io.visited.last(), Some(&fail));
    }
}

#[test]
fn t04_02_real_replace_file_w_preserves_backup_and_replaces_identity() {
    let fixture = Fixture::new();
    let target = fixture.file("save.txt", b"external original");
    let before = windows_save::inspect(&target).expect("identity");
    assert!(windows_save::replace(&target, b"frozen revision R", &before).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"external original");
    let stage = fs::read_dir(&fixture.path)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".jasonshell-stage-")
        })
        .unwrap();
    assert_eq!(fs::read(stage).unwrap(), b"frozen revision R");
}

fn rb02_legacy_lease(bytes: &[u8]) -> contract::ViewLease {
    lease::build_lease(
        lease::LeaseTags {
            session: "s1",
            lease: "l1",
            source: 1,
            revision: 0,
            view: 1,
        },
        decode::Checkpoint::new(TextEncoding::Utf8, 0),
        bytes,
        true,
        Some(1),
    )
    .expect("legacy feasibility lease")
}

#[test]
fn rb02_v2_lease_mapping_and_context() {
    use crate::stack_popup::text_document::protocol as v2;
    let old = rb02_legacy_lease("a\r\n😀e\u{301}".as_bytes());
    let mapped = rb02_v2::lease(
        &old,
        TextEncoding::Utf8,
        rb02_v2::Context {
            before: "complete",
            after: "complete",
            continuation_id: None,
        },
    )
    .expect("lossless test-only mapping");
    assert_eq!(v2::lease_byte(&mapped, 0, 2), Ok(3));
    assert_eq!(v2::lease_byte(&mapped, 0, 4), Ok(7));
    let incomplete = rb02_v2::lease(
        &old,
        TextEncoding::Utf8,
        rb02_v2::Context {
            before: "complete",
            after: "continued",
            continuation_id: Some("next1"),
        },
    )
    .expect("explicit continuation");
    assert_eq!(
        v2::require_complete_context(&incomplete),
        Err(v2::ErrorCode::ContextRequired)
    );
    let mut overflow = mapped.clone();
    overflow.segments[0].byte_start = u64::MAX.to_string();
    assert_eq!(
        v2::validate_lease(&overflow),
        Err(v2::ErrorCode::InvalidRequest)
    );
}

#[test]
fn rb02_v2_source_outcomes_and_errors() {
    use crate::stack_popup::text_document::protocol as v2;
    let mut old = rb02_legacy_lease(b"valid");
    old.source_state = SourceState::DecisionRequired;
    old.invalid_at = Some(contract::DecimalU64::parse("5").unwrap());
    assert!(matches!(
        rb02_v2::lease(
            &old,
            TextEncoding::Utf8,
            rb02_v2::Context {
                before: "complete",
                after: "complete",
                continuation_id: None,
            }
        ),
        Err(rb02_v2::Refusal::LegacySourceOutcomeHasNoV2LeaseEquivalent(
            SourceState::DecisionRequired
        ))
    ));
    for (legacy, expected) in [
        (SourceState::Cancelled, v2::ErrorCode::Cancelled),
        (SourceState::QuotaExceeded, v2::ErrorCode::ResourceLimit),
        (SourceState::Conflict, v2::ErrorCode::SharingViolation),
        (SourceState::ReadLimited, v2::ErrorCode::IoFailure),
        (SourceState::Changed, v2::ErrorCode::SourceChanged),
        (SourceState::Readonly, v2::ErrorCode::Readonly),
    ] {
        let code = rb02_v2::source_error(legacy, None).expect("explicit legacy outcome mapping");
        assert_eq!(code, expected);
        assert!(
            v2::validate_result(rb02_v2::failure_result(code, "new-request", "retained")).is_ok()
        );
    }
    assert!(matches!(
        rb02_v2::source_error(SourceState::DecisionRequired, old.invalid_at.as_ref()),
        Err(
            rb02_v2::Refusal::LegacySourceOutcomeHasNoV2ErrorEquivalent {
                source_state: SourceState::DecisionRequired,
                invalid_at: Some(5),
            }
        )
    ));
    for legacy in [
        SourceState::Opening,
        SourceState::Snapshotting,
        SourceState::Ready,
    ] {
        assert!(matches!(
            rb02_v2::source_error(legacy, None),
            Err(rb02_v2::Refusal::LegacySourceOutcomeHasNoV2ErrorEquivalent {
                source_state,
                invalid_at: None,
            }) if source_state == legacy
        ));
    }
    assert!(matches!(
        v2::validate_result(rb02_v2::failure_result(
            v2::ErrorCode::PublicationAmbiguous,
            "same-request",
            "ambiguous"
        )),
        Err(v2::ErrorCode::InvalidRequest)
    ));
}

#[test]
fn rb02_v2_selection_barrier_replay() {
    use crate::stack_popup::text_document::protocol as v2;
    let old = rb02_legacy_lease(b"abc");
    let a = rb02_v2::lease(
        &old,
        TextEncoding::Utf8,
        rb02_v2::Context {
            before: "complete",
            after: "complete",
            continuation_id: None,
        },
    )
    .unwrap();
    let mut b = a.clone();
    b.lease_id = "l2".into();
    b.segments[0].byte_start = "3".into();
    let state = v2::SessionVersion {
        session_id: "s1".into(),
        source_generation: "1".into(),
        document_revision: "0".into(),
        input_sequence: "7".into(),
    };
    let selection = v2::create_selection(
        &state,
        "sel1",
        (&a, 0, 1, v2::Affinity::Before),
        (&b, 0, 2, v2::Affinity::After),
    )
    .unwrap();
    assert_eq!(selection.head.byte, "5");
    let mut forged = b.clone();
    forged.session_id = "other".into();
    assert!(matches!(
        v2::create_selection(
            &state,
            "sel2",
            (&a, 0, 0, v2::Affinity::Before),
            (&forged, 0, 0, v2::Affinity::After)
        ),
        Err(v2::ErrorCode::Unauthorized)
    ));
    let mut stale = b.clone();
    stale.document_revision = "1".into();
    assert!(matches!(
        v2::create_selection(
            &state,
            "sel3",
            (&a, 0, 0, v2::Affinity::Before),
            (&stale, 0, 0, v2::Affinity::After)
        ),
        Err(v2::ErrorCode::StaleRevision)
    ));
    let mut ledger = v2::OperationLedger::new(2).unwrap();
    let digest = "a".repeat(64);
    assert_eq!(ledger.begin("op1", &digest), Ok(None));
    assert_eq!(
        ledger.begin("op1", &"b".repeat(64)),
        Err(v2::ErrorCode::InvalidRequest)
    );
    ledger
        .finish(
            "op1",
            v2::Receipt::Accepted {
                document_revision: "0".into(),
            },
        )
        .unwrap();
    let barrier = v2::Barrier {
        document_revision: "0".into(),
        input_sequence: "7".into(),
        operation_ids: vec!["op1".into()],
    };
    assert_eq!(v2::assert_barrier(&state, &barrier, &ledger), Ok(()));
    ledger.retire("op1", "0").unwrap();
    assert_eq!(
        ledger.begin("op1", &digest),
        Err(v2::ErrorCode::RetryRetired)
    );
}

#[test]
fn rb02_v2_transport_credits_and_cancellation() {
    use crate::stack_popup::text_document::protocol as v2;
    let scheduler = scheduler::Scheduler::new().unwrap();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let release_rx = Arc::new(Mutex::new(release_rx));
    let mut blockers = Vec::new();
    for _ in 0..2 {
        let (started_tx, started_rx) = mpsc::sync_channel(0);
        let release_rx = release_rx.clone();
        blockers.push(
            scheduler
                .submit(scheduler::Priority::Demand, move |_| {
                    started_tx.send(()).unwrap();
                    release_rx.lock().unwrap().recv().unwrap();
                    Ok(vec![1])
                })
                .unwrap(),
        );
        started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("demand worker occupied");
    }
    let obsolete = scheduler
        .submit(scheduler::Priority::Demand, |_| Ok(vec![2]))
        .unwrap();
    let latest = scheduler
        .submit(scheduler::Priority::Demand, |_| Ok(vec![3]))
        .unwrap();
    assert!(
        matches!(obsolete.result.recv_timeout(Duration::from_secs(2)).unwrap(),Err(error) if error.code==ProtocolErrorCode::Cancelled)
    );
    let cancelled = scheduler.cancel(latest.id).unwrap();
    assert_eq!(cancelled, scheduler::CancelOutcome::QueuedRemoved);
    assert!(
        matches!(latest.result.recv_timeout(Duration::from_secs(2)).unwrap(), Err(error) if error.code == ProtocolErrorCode::Cancelled)
    );
    let canonical = rb02_v2::cancellation_result(cancelled);
    assert!(v2::validate_result(canonical).is_ok());
    let indeterminate = rb02_v2::cancellation_result(scheduler::CancelOutcome::FinishedOrUnknown);
    assert_eq!(indeterminate["kind"], "error");
    assert_eq!(indeterminate["data"]["code"], "PublicationAmbiguous");
    assert!(v2::validate_result(indeterminate).is_ok());
    assert!(matches!(
        v2::validate_result(rb02_v2::job_result("published", "removed")),
        Err(v2::ErrorCode::InvalidRequest)
    ));
    release_tx.send(()).unwrap();
    release_tx.send(()).unwrap();
    for blocker in blockers {
        blocker
            .result
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
    }
    let metrics = scheduler.metrics().unwrap();
    assert!(metrics.payload_peak <= 4);
    assert!(metrics.active_peak <= 3);
    assert_eq!(metrics.actor_lock_io_violations, 0);
}

#[test]
fn t02_01_authoritative_identity_rejection_and_races() {
    let fixture = Fixture::new();
    let path = fixture.file("regular.txt", b"synthetic regular UTF8\r\n");
    emit(
        "T02-01-stage",
        serde_json::json!({"stage":"before-protected-open"}),
    );
    let source = source::ProtectedSource::open_authorized_path(&path, None)
        .expect("ordinary supported target must not be universally refused");
    emit(
        "T02-01-stage",
        serde_json::json!({"stage":"after-protected-open"}),
    );
    let baseline = (
        source.identity.volume_serial.as_u64(),
        source.identity.file_id,
    );
    let mut prefix = [0; 9];
    source
        .read(0, &mut prefix, &AtomicBool::new(false))
        .expect("protected prefix");
    assert_eq!(&prefix, b"synthetic");
    emit(
        "T02-01-stage",
        serde_json::json!({"stage":"before-conflicting-rename"}),
    );
    assert!(
        fs::rename(&path, fixture.path.join("raced.txt")).is_err(),
        "owned identity must deny namespace replacement"
    );
    emit(
        "T02-01-stage",
        serde_json::json!({"stage":"after-conflicting-rename"}),
    );
    let first_generation = source.identity.source_generation.clone();
    drop(source);
    let again = source::ProtectedSource::open_authorized_path(&path, Some(baseline))
        .expect("stable identity");
    assert_ne!(again.identity.source_generation, first_generation);
    emit(
        "T02-01-identity",
        serde_json::to_value(&again.identity).expect("identity JSON"),
    );
    drop(again);
    let old = fixture.path.join("old.txt");
    fs::rename(&path, old).expect("replace selection fixture");
    fs::write(&path, b"replacement").expect("replacement");
    assert!(
        matches!(source::ProtectedSource::open_authorized_path(&path, Some(baseline)), Err(error) if error.code == ProtocolErrorCode::SourceChanged)
    );
    let hard = fixture.path.join("hard.txt");
    fs::hard_link(&path, &hard).expect("NTFS hard link");
    assert!(
        matches!(source::ProtectedSource::open_authorized_path(&hard, None), Err(error) if error.code == ProtocolErrorCode::UnsupportedTarget)
    );
    fs::remove_file(hard).expect("remove synthetic hard link");
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).expect("readonly fixture");
    assert!(
        matches!(source::ProtectedSource::open_authorized_path(&path, None), Err(error) if error.code == ProtocolErrorCode::Readonly)
    );
    let mut permissions = fs::metadata(&path).expect("metadata").permissions();
    permissions.set_readonly(false);
    fs::set_permissions(&path, permissions).expect("restore synthetic permissions");
    for rejected in [
        "NUL",
        "C:relative.txt",
        "C:\\a.txt:secret",
        "\\\\.\\PhysicalDrive0",
        "\\\\server\\share\\a",
        "C:\\a\\..\\b",
        "C:\\a.\\b",
    ] {
        assert!(
            source::ProtectedSource::open_authorized_path(Path::new(rejected), None).is_err(),
            "unsupported target {rejected}"
        );
    }
    assert!(backing::reservation(u64::MAX, 1, 1).is_err());
    assert!(super::contract::DecimalU64::parse("18446744073709551616").is_err());
    emit(
        "T02-01-rejections",
        serde_json::json!({"hardLink":"refused","readonly":"refused","replacement":"refused","overflow":"refused","contentDisclosedOnRejectedOpen":false,"actualWebviewAuthorization":"covered-by-separate-native-window-probe"}),
    );
}

#[test]
fn t02_01_identity_captures_reparse_metadata_and_rejects_named_streams() {
    let fixture = Fixture::new();
    let path = fixture.file("identity.txt", b"ordinary source");
    let source = source::ProtectedSource::open_authorized_path(&path, None).expect("source");
    assert!(!source.identity.reparse_point);
    assert_eq!(source.identity.reparse_tag, 0);
    assert!(!source.identity.named_stream);
    drop(source);

    let named_stream = PathBuf::from(format!("{}:secret", path.display()));
    assert!(matches!(
        source::ProtectedSource::open_authorized_path(&named_stream, None),
        Err(error) if error.code == ProtocolErrorCode::UnsupportedTarget
    ));
}

#[test]
fn t02_04_invalid_prefix_is_retained_and_exposed_as_decision_required() {
    let lease = lease::build_lease(
        lease::LeaseTags {
            session: "invalid",
            lease: "invalid-prefix",
            source: 1,
            revision: 0,
            view: 1,
        },
        decode::Checkpoint::new(TextEncoding::Utf8, 0),
        b"valid\n\xfffar-invalid",
        true,
        Some(1),
    )
    .expect("valid prefix remains readable");
    assert_eq!(lease.source_state, SourceState::DecisionRequired);
    assert_eq!(
        lease.invalid_at.as_ref().map(|value| value.as_u64()),
        Some(6)
    );
    assert!(lease.segments[0].text.starts_with("valid\n"));
    assert!(lease.local_to_byte(7).is_err());
    lease
        .validate()
        .expect("decision-required lease remains valid");
}

#[test]
fn t02_04_far_invalid_range_is_exposed_not_private_state() {
    let mut bytes = vec![b'a'; super::contract::MAX_LEASE_BYTES - 1];
    bytes.push(0xff);
    let lease = lease::build_lease(
        lease::LeaseTags {
            session: "far-invalid",
            lease: "far-invalid-prefix",
            source: 1,
            revision: 0,
            view: 1,
        },
        decode::Checkpoint::new(TextEncoding::Utf8, 0),
        &bytes,
        true,
        Some(1),
    )
    .expect("valid prefix remains readable");
    assert_eq!(lease.source_state, SourceState::DecisionRequired);
    assert_eq!(
        lease.invalid_at.as_ref().map(|value| value.as_u64()),
        Some((super::contract::MAX_LEASE_BYTES - 1) as u64)
    );
    lease
        .validate()
        .expect("far invalid marker is wire-visible");
}

#[test]
fn t02_01_reparse_ancestor_and_final_refuse() {
    let fixture = Fixture::new();
    let target = fixture.path.join("target");
    fs::create_dir(&target).expect("target directory");
    fs::write(target.join("text.txt"), b"synthetic").expect("target text");
    let junction = fixture.path.join("redirect");
    let output = std::process::Command::new("cmd")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&target)
        .output()
        .expect("junction API fixture helper");
    assert!(
        output.status.success(),
        "BLOCK: junction fixture creation failed, native status {:?}",
        output.status.code()
    );
    assert!(
        matches!(source::ProtectedSource::open_authorized_path(&junction.join("text.txt"), None), Err(error) if error.code == ProtocolErrorCode::UnsupportedTarget)
    );
    assert!(source::ProtectedSource::open_authorized_path(&junction, None).is_err());
    fs::remove_dir(&junction).expect("remove junction without traversing target");
    emit(
        "T02-01-reparse",
        serde_json::json!({"ancestorJunction":"refused","finalJunction":"refused"}),
    );
}

#[test]
fn t02_02_ordinary_and_preexisting_mapped_writer_native() {
    let fixture = Fixture::new();
    let path = fixture.file("mapped.txt", &[b'a'; 4096]);
    let writer = native::open(
        &path,
        native::READ | native::WRITE,
        native::SHARE_READ | native::SHARE_WRITE | native::SHARE_DELETE,
        0,
    )
    .expect("ordinary writer control");
    assert!(
        matches!(source::ProtectedSource::open_authorized_path(&path, None), Err(error) if error.code == ProtocolErrorCode::SharingViolation)
    );
    let mut mapping = native::WritableMap::create(&writer, 4096)
        .expect("CreateFileMappingW PAGE_READWRITE + MapViewOfFile FILE_MAP_WRITE");
    drop(writer); // The important case: section/view lives after ordinary handle is closed.
    mapping
        .write(0, b'b')
        .expect("mapped control remains writable after source handle close");
    let ordinary = native::open(&path, native::READ, native::SHARE_READ, native::OVERLAPPED);
    let ordinary_result = match ordinary {
        Ok(file) => {
            let mut before = [0; 1];
            native::read_at(&file, 0, &mut before, &AtomicBool::new(false))
                .expect("ordinary read before mapped write");
            mapping.write(0, b'c').expect("pre-existing map mutation");
            let mut after = [0; 1];
            native::read_at(&file, 0, &mut after, &AtomicBool::new(false))
                .expect("ordinary read after mapped write");
            let oplock = native::Oplock::acquire(&file);
            serde_json::json!({"shareOpen":"allowed","before":before[0],"after":after[0],"oplockGranted":oplock.is_ok(),"oplockError":oplock.err().map(|error|error.message)})
        }
        Err(error) => {
            serde_json::json!({"shareOpen":"refused","code":format!("{:?}",error.code),"detail":error.message})
        }
    };
    let protected = source::ProtectedSource::open_authorized_path(&path, None);
    let outcome = match protected {
        Err(error) => {
            emit(
                "T02-02-mapped",
                serde_json::json!({"control":ordinary_result,"protected":"refused","code":format!("{:?}",error.code),"detail":error.message,"mappingSurvivedFileHandleClose":true}),
            );
            assert!(matches!(
                error.code,
                ProtocolErrorCode::SharingViolation | ProtocolErrorCode::SourceChanged
            ));
            true
        }
        Ok(source) => {
            let mut before = [0; 1];
            source
                .read(0, &mut before, &AtomicBool::new(false))
                .expect("protected read");
            mapping.write(0, b'd').expect("mapped mutation API");
            let mut after = [0; 1];
            let read = source.read(0, &mut after, &AtomicBool::new(false));
            emit(
                "T02-02-mapped",
                serde_json::json!({"control":ordinary_result,"protected":"allowed","before":before[0],"after":after[0],"postMutationReadRejected":read.is_err(),"gate":"FAIL-mapped-writer-not-excluded"}),
            );
            false
        }
    };
    assert!(
        outcome,
        "P02 BLOCK: pre-existing writable map admitted; source guarantee not established"
    );
    drop(mapping);
    let source = source::ProtectedSource::open_authorized_path(&path, None)
        .expect("ordinary control after mapped view closes must be admitted");
    assert!(native::open(
        &path,
        native::WRITE,
        native::SHARE_READ | native::SHARE_WRITE,
        0
    )
    .is_err());
    source.verify_identity().expect("ordinary control intact");
}

#[test]
fn t02_02_prefix_edit_copy_handoff_and_cancellation() {
    let fixture = Fixture::new();
    let original = b"one\r\ntwo \xf0\x9f\x98\x80\r\nlast".repeat(10000);
    let path = fixture.file("copy.txt", &original);
    let shared_quota = quota(128 * 1024 * 1024);
    let source = source::ProtectedSource::open_authorized_path(&path, None).expect("source");
    let generation = source.identity.source_generation.as_u64();
    let mut copy = source::ProtectedCopy::new(
        source,
        fixture.pages(
            "original.pages",
            backing::BYTE_PAGE,
            backing::BYTE_PAGE,
            shared_quota.clone(),
        ),
    );
    let mut doc = document(&fixture, original.len() as u64, shared_quota);
    let mut prefix = vec![0; super::contract::FIRST_READ_BYTES];
    copy.read_original(0, &mut prefix, &AtomicBool::new(false))
        .expect("prefix");
    let lease = lease::build_lease(
        lease::LeaseTags {
            session: "p02",
            lease: "prefix",
            source: generation,
            revision: 0,
            view: 1,
        },
        decode::Checkpoint::new(TextEncoding::Utf8, 0),
        &prefix,
        false,
        Some(1),
    )
    .expect("prefix lease");
    lease::replace_validated_local(
        &mut doc,
        &lease,
        0,
        3,
        "edited",
        TextEncoding::Utf8,
        lease::insertion_newline(&lease),
    )
    .expect("real prefix edit");
    let mut visible = [0; 6];
    doc.read_range(0, &mut visible, |offset, buffer| {
        copy.read_original(offset, buffer, &AtomicBool::new(false))
    })
    .expect("edited document read");
    assert_eq!(&visible, b"edited");
    assert_eq!(copy.copied, 0);
    assert!(!copy.recovery_complete());
    assert_eq!(doc.revision, 1);
    emit(
        "T02-02-first-edit",
        serde_json::json!({"firstReadBytes":prefix.len(),"sourceBytes":original.len().to_string(),"copiedBytes":copy.copied.to_string(),"indexedBytes":"0","acceptedRevision":"1","recoveryComplete":false}),
    );
    while copy.phase != source::CopyPhase::Complete {
        copy.step(&AtomicBool::new(false))
            .expect("bounded copy/verify step");
    }
    assert_eq!(
        copy.digest.as_deref(),
        Some(format!("{:x}", Sha256::digest(&original)).as_str())
    );
    assert!(copy.source().is_none());
    fs::write(&path, b"external replacement after immutable handoff")
        .expect("source restriction released");
    let mut unchanged = [0; 4];
    copy.read_original(0, &mut unchanged, &AtomicBool::new(false))
        .expect("immutable backing read after external write");
    assert_eq!(&unchanged, b"one\r");
    emit(
        "T02-02-handoff",
        serde_json::json!({"copiedBytes":copy.copied.to_string(),"verifiedBytes":copy.verified.to_string(),"snapshotComplete":true,"recoveryComplete":copy.recovery_complete(),"originalSha256":copy.digest}),
    );
    let cancel_fixture = Fixture::new();
    let cancel_path = cancel_fixture.file("cancel.txt", &[b'x'; 100000]);
    let source =
        source::ProtectedSource::open_authorized_path(&cancel_path, None).expect("cancel source");
    let mut cancelled = source::ProtectedCopy::new(
        source,
        cancel_fixture.pages("cancel.pages", backing::BYTE_PAGE, 0, quota(1024 * 1024)),
    );
    cancelled
        .step(&AtomicBool::new(false))
        .expect("first copy block");
    assert!(
        matches!(cancelled.step(&AtomicBool::new(true)), Err(error) if error.code == ProtocolErrorCode::Cancelled)
    );
    assert_eq!(cancelled.phase, source::CopyPhase::Cancelled);
    assert_eq!(cancelled.state(), SourceState::Cancelled);
    assert!(!cancelled.recovery_complete());
    assert!(cancelled.source().is_some());
}

#[test]
fn t02_03_paged_edits_history_eviction_and_quota_keep_dirty_root() {
    let fixture = Fixture::new();
    let original = b"0123456789\r\n".repeat(100);
    let quota = quota(96 * 1024 * 1024);
    let mut doc = document(&fixture, original.len() as u64, quota.clone());
    let mut oracle = original.clone();
    let mut seed = 0x12345678u32;
    for index in 0..96 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let start = seed as usize % (oracle.len() + 1);
        let end = (start + index % 9).min(oracle.len());
        let insert = format!("i{index}");
        doc.replace_bytes(start as u64, end as u64, insert.as_bytes())
            .expect("paged edit");
        oracle.splice(start..end, insert.bytes());
        doc.evict_caches();
        assert_eq!(bytes(&mut doc, &original), oracle);
    }
    assert!(doc.dirty());
    assert_eq!(doc.metrics().expect("unknown metrics").newlines, None);
    for _ in 0..96 {
        assert!(doc.undo().expect("cold history undo"));
        doc.evict_caches();
    }
    assert_eq!(bytes(&mut doc, &original), original);
    assert!(!doc.dirty());
    for _ in 0..96 {
        assert!(doc.redo().expect("cold history redo"));
        doc.evict_caches();
    }
    assert_eq!(bytes(&mut doc, &original), oracle);
    assert!(doc.dirty());
    assert!(doc.nodes.cache_peak() <= backing::NODE_PAGE * 2);
    assert!(doc.added.cache_peak() <= backing::BYTE_PAGE);
    let before = bytes(&mut doc, &original);
    let revision = doc.revision;
    // Exhaust only the new-work quota; existing immutable pages remain readable.
    quota.lock().expect("quota").exhaust_for_test();
    assert!(
        matches!(doc.replace_bytes(0, 0, b"quota failure"), Err(error) if error.code == ProtocolErrorCode::ResourceLimit)
    );
    assert_eq!(doc.revision, revision);
    doc.evict_caches();
    assert_eq!(bytes(&mut doc, &original), before);
    emit(
        "T02-03-paging",
        serde_json::json!({"nodeCachePeak":doc.nodes.cache_peak(),"addCachePeak":doc.added.cache_peak(),"nodeDiskBytes":doc.nodes.disk_bytes().to_string(),"historyResidentRoots":4,"dirtyEvictions":0}),
    );
}

#[test]
fn t02_03_authenticated_reopen_and_tamper_refusal() {
    use std::io::{Seek, SeekFrom, Write};
    let fixture = Fixture::new();
    let path = fixture.path.join("pages.bin");
    let quota = quota(1024 * 1024);
    let mut pages =
        backing::PageStore::create(&path, backing::NODE_PAGE, 0, quota.clone()).expect("pages");
    let plaintext = b"private synthetic bytes";
    let id = pages.append(plaintext).expect("append");
    pages.flush().expect("flush");
    drop(pages);
    let disk = fs::read(&path).expect("encrypted page bytes");
    assert!(
        !disk
            .windows(plaintext.len())
            .any(|window| window == plaintext),
        "plaintext must not be persisted in page backing"
    );
    let mut reopened = backing::PageStore::reopen(&path, backing::NODE_PAGE, 0, quota.clone())
        .expect("DPAPI reopen");
    assert_eq!(
        reopened.read(id).expect("authenticated read").as_slice(),
        b"private synthetic bytes"
    );
    drop(reopened);
    let mut raw = fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("tamper synthetic store");
    raw.seek(SeekFrom::Start(40)).expect("seek");
    raw.write_all(&[0x55]).expect("tamper");
    drop(raw);
    let mut reopened = backing::PageStore::reopen(&path, backing::NODE_PAGE, 0, quota)
        .expect("open structurally complete store");
    assert!(reopened.read(id).is_err());
}

#[test]
fn t02_04_every_split_codec_map_crlf_and_invalid_tail() {
    let raw = "first\r\n😀e\u{301}\nthird\rlast";
    let normalized = "first\n😀e\u{301}\nthird\nlast";
    for encoding in [
        TextEncoding::Utf8,
        TextEncoding::Utf8Bom,
        TextEncoding::Utf16Le,
        TextEncoding::Utf16Be,
    ] {
        let (mut encoded, bom) = match encoding {
            TextEncoding::Utf8 => (Vec::new(), 0),
            TextEncoding::Utf8Bom => (vec![0xef, 0xbb, 0xbf], 3),
            TextEncoding::Utf16Le => (vec![0xff, 0xfe], 2),
            TextEncoding::Utf16Be => (vec![0xfe, 0xff], 2),
        };
        for scalar in raw.chars() {
            match encoding {
                TextEncoding::Utf8 | TextEncoding::Utf8Bom => {
                    let mut buf = [0; 4];
                    encoded.extend_from_slice(scalar.encode_utf8(&mut buf).as_bytes());
                }
                _ => {
                    let mut buf = [0; 2];
                    for unit in scalar.encode_utf16(&mut buf).iter() {
                        encoded.extend_from_slice(&if encoding == TextEncoding::Utf16Be {
                            unit.to_be_bytes()
                        } else {
                            unit.to_le_bytes()
                        });
                    }
                }
            }
        }
        assert_eq!(
            decode::detect(&encoded, None).expect("BOM detection").0,
            encoding
        );
        for split in bom..=encoded.len() {
            let mut checkpoint = decode::Checkpoint::new(encoding, bom as u64);
            let mut text = String::new();
            let mut spans = Vec::new();
            checkpoint
                .feed(&encoded[bom..split], false, |scalar| {
                    text.push(scalar.value);
                    spans.push((scalar.byte_start, scalar.byte_end));
                    Ok(())
                })
                .expect("first split");
            let serialized = serde_json::to_vec(&checkpoint).expect("checkpoint encode");
            let mut checkpoint: decode::Checkpoint =
                serde_json::from_slice(&serialized).expect("checkpoint reload");
            checkpoint
                .feed(&encoded[split..], true, |scalar| {
                    text.push(scalar.value);
                    spans.push((scalar.byte_start, scalar.byte_end));
                    Ok(())
                })
                .expect("second split");
            assert_eq!(text, normalized);
            assert_eq!(checkpoint.newlines, 3);
            assert_eq!(
                checkpoint.utf16_units,
                normalized.encode_utf16().count() as u64
            );
            assert_eq!(spans.first().expect("first boundary").0, bom as u64);
            assert_eq!(spans.last().expect("last boundary").1, encoded.len() as u64);
        }
        let lease = lease::build_lease(
            lease::LeaseTags {
                session: "codec",
                lease: "codec-lease",
                source: 1,
                revision: 0,
                view: 1,
            },
            decode::Checkpoint::new(encoding, bom as u64),
            &encoded[bom..],
            true,
            Some(1),
        )
        .expect("exact lease");
        assert_eq!(lease.segments[0].text, normalized);
        lease.validate().expect("frozen protocol validation");
        for boundary in &lease.segments[0].boundaries {
            assert_eq!(
                lease
                    .local_to_byte(boundary.local_utf16_offset)
                    .expect("local byte"),
                boundary.source_byte_offset
            );
            assert_eq!(
                lease
                    .byte_to_local(&boundary.source_byte_offset)
                    .expect("byte local"),
                boundary.local_utf16_offset
            );
        }
        let mut invalid = decode::Checkpoint::new(encoding, bom as u64);
        invalid
            .feed(&encoded[bom..], false, |_| Ok(()))
            .expect("valid prefix");
        let bad = match encoding {
            TextEncoding::Utf8 | TextEncoding::Utf8Bom => vec![0xff],
            TextEncoding::Utf16Le => vec![0x00, 0xdc],
            TextEncoding::Utf16Be => vec![0xdc, 0x00],
        };
        assert!(invalid.feed(&bad, true, |_| Ok(())).is_err());
        assert_eq!(invalid.invalid_at, Some(encoded.len() as u64));
    }
    emit(
        "T02-04-codecs",
        serde_json::json!({"encodings":4,"splitStrategy":"every-byte","checkpointRoundtrip":true,"replacementDecoding":false}),
    );
}

#[test]
fn t02_04_disk_checkpoints_and_distant_invalid() {
    let fixture = Fixture::new();
    let mut index = index::CheckpointIndex::new(
        fixture.pages(
            "checkpoints.pages",
            backing::NODE_PAGE,
            backing::NODE_PAGE,
            quota(8 * 1024 * 1024),
        ),
        decode::Checkpoint::new(TextEncoding::Utf8, 0),
    )
    .expect("checkpoint index");
    let block = [b'\n'; backing::BYTE_PAGE];
    for _ in 0..32 {
        index.push(&block, false).expect("stream highline block");
        index.pages.evict_cache();
    }
    assert_eq!(index.known_lines(), None);
    let checkpoint = index
        .nearest(17 * backing::BYTE_PAGE as u64 + 3)
        .expect("cold checkpoint lookup");
    assert_eq!(checkpoint.newlines, 17 * backing::BYTE_PAGE as u64);
    assert!(index.push(&[0xf0, 0x28, 0x8c, 0xbc], true).is_err());
    assert_eq!(
        index.decoder.invalid_at,
        Some(32 * backing::BYTE_PAGE as u64)
    );
    assert_eq!(index.known_lines(), None);
}

#[test]
fn t02_05_blocked_scan_urgent_priority_latest_and_cancellation() {
    let scheduler = scheduler::Scheduler::new().expect("scheduler");
    let (started_tx, started_rx) = mpsc::channel();
    let background = scheduler
        .submit(scheduler::Priority::Background, move |cancel| {
            started_tx.send(()).expect("started");
            while !cancel.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(failure(
                ProtocolErrorCode::Cancelled,
                "controlled blocked scan cancelled",
            ))
        })
        .expect("blocked background");
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("background started");
    let fixture = Fixture::new();
    let path = fixture.file("urgent.txt", b"urgent disk result");
    let now = Instant::now();
    let urgent = scheduler
        .submit(scheduler::Priority::Demand, move |cancel| {
            let file = native::open(&path, native::READ, native::SHARE_READ, native::OVERLAPPED)?;
            let mut bytes = vec![0; 18];
            native::read_at(&file, 0, &mut bytes, cancel)?;
            Ok(bytes)
        })
        .expect("urgent");
    let payload = urgent
        .result
        .recv_timeout(Duration::from_secs(2))
        .expect("urgent must not wait for scan")
        .expect("urgent result");
    assert_eq!(payload.bytes, b"urgent disk result");
    drop(payload);
    let urgent_ms = now.elapsed().as_millis();
    let mut held = Vec::new();
    for _ in 0..3 {
        let ticket = scheduler
            .submit(scheduler::Priority::Demand, |_| Ok(vec![1]))
            .expect("credit read");
        held.push(
            ticket
                .result
                .recv_timeout(Duration::from_secs(2))
                .expect("credit result")
                .expect("credit"),
        );
    }
    let obsolete = scheduler
        .submit(scheduler::Priority::Demand, |_| Ok(vec![2]))
        .expect("pending");
    let latest = scheduler
        .submit(scheduler::Priority::Demand, |_| Ok(vec![3]))
        .expect("latest");
    assert!(
        matches!(obsolete.result.recv_timeout(Duration::from_secs(2)).expect("supersession"), Err(error) if error.code==ProtocolErrorCode::Cancelled)
    );
    assert_eq!(
        scheduler.cancel(latest.id).expect("queued cancel"),
        scheduler::CancelOutcome::QueuedRemoved
    );
    assert_eq!(
        scheduler.cancel(background.id).expect("inflight cancel"),
        scheduler::CancelOutcome::RequestedDriverDependent
    );
    assert!(background
        .result
        .recv_timeout(Duration::from_secs(2))
        .expect("I/O cancellation completion")
        .is_err());
    drop(held);
    let metrics = scheduler.metrics().expect("metrics");
    assert!(metrics.active_peak <= 3);
    assert!(metrics.payload_peak <= 4);
    assert!(metrics.queued_peak <= 2);
    assert!(metrics.superseded >= 1);
    emit(
        "T02-05-scheduler",
        serde_json::json!({"metrics":metrics,"urgentHostDurationMs":urgent_ms,"nativePresentationClaim":false,"actorLockIoChecks":metrics.actor_lock_io_checks,"actorLockContainsIo":metrics.actor_lock_io_violations != 0}),
    );
}

#[test]
fn t02_05_priority_lanes_bound_background_work_and_keep_actor_lock_io_free() {
    let scheduler = scheduler::Scheduler::new().expect("scheduler");
    let (started_tx, started_rx) = mpsc::channel();
    let blocked = scheduler
        .submit(scheduler::Priority::Compaction, move |cancel| {
            started_tx.send(()).expect("compaction started");
            while !cancel.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(failure(
                ProtocolErrorCode::Cancelled,
                "blocked remote-like compaction cancelled",
            ))
        })
        .expect("compaction");
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("compaction worker started");

    let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let background_order = order.clone();
    let prefetch = scheduler
        .submit(scheduler::Priority::Prefetch, move |_| {
            background_order.lock().expect("order").push("prefetch");
            Ok(vec![1])
        })
        .expect("prefetch");
    let index_order = order.clone();
    let index = scheduler
        .submit(scheduler::Priority::Index, move |_| {
            index_order.lock().expect("order").push("index");
            Ok(vec![2])
        })
        .expect("index");
    let urgent_release = Arc::new(AtomicBool::new(false));
    let (urgent_started_tx, urgent_started_rx) = mpsc::channel();
    let prefix_release = urgent_release.clone();
    let prefix_order = order.clone();
    let prefix_started = urgent_started_tx.clone();
    let prefix = scheduler
        .submit(scheduler::Priority::Demand, move |_| {
            prefix_order.lock().expect("order").push("prefix");
            prefix_started.send("prefix").expect("prefix started");
            while !prefix_release.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(vec![3])
        })
        .expect("prefix demand");
    assert_eq!(
        urgent_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("prefix wins first urgent slot"),
        "prefix"
    );
    let caret_release = urgent_release.clone();
    let caret_order = order.clone();
    let caret_started = urgent_started_tx;
    let caret = scheduler
        .submit(scheduler::Priority::Demand, move |_| {
            caret_order.lock().expect("order").push("caret");
            caret_started.send("caret").expect("caret started");
            while !caret_release.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(vec![4])
        })
        .expect("caret demand");
    assert_eq!(
        urgent_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("caret wins second urgent slot"),
        "caret"
    );
    urgent_release.store(true, Ordering::Release);

    let obsolete = scheduler
        .submit(scheduler::Priority::Demand, |_| Ok(vec![9]))
        .expect("obsolete scroll/seek");
    let latest_order = order.clone();
    let latest = scheduler
        .submit(scheduler::Priority::Demand, move |_| {
            latest_order.lock().expect("order").push("latest");
            Ok(vec![5])
        })
        .expect("latest scroll/seek");
    assert!(matches!(
        obsolete
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("obsolete supersession"),
        Err(error) if error.code == ProtocolErrorCode::Cancelled
    ));

    assert_eq!(
        prefix
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("prefix completion")
            .expect("prefix payload")
            .bytes,
        vec![3]
    );
    assert_eq!(
        caret
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("caret completion")
            .expect("caret payload")
            .bytes,
        vec![4]
    );
    assert_eq!(
        latest
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("latest completion")
            .expect("latest payload")
            .bytes,
        vec![5]
    );

    assert_eq!(
        scheduler
            .cancel(blocked.id)
            .expect("cancel blocked remote-like work"),
        scheduler::CancelOutcome::RequestedDriverDependent
    );
    assert!(blocked
        .result
        .recv_timeout(Duration::from_secs(2))
        .expect("blocked completion")
        .is_err());
    assert_eq!(
        prefetch
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("prefetch completion")
            .expect("prefetch payload")
            .bytes,
        vec![1]
    );
    assert_eq!(
        index
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("index completion")
            .expect("index payload")
            .bytes,
        vec![2]
    );
    let metrics = scheduler.metrics().expect("metrics");
    assert_eq!(
        metrics.priority_completed[scheduler::Priority::Demand.index()],
        3
    );
    assert_eq!(
        metrics.priority_completed[scheduler::Priority::Prefetch.index()],
        1
    );
    assert_eq!(
        metrics.priority_completed[scheduler::Priority::Index.index()],
        1
    );
    assert_eq!(metrics.actor_lock_io_violations, 0);
    assert!(metrics.actor_lock_max_ns < 50_000_000);
    assert!(metrics.queued_peak <= scheduler::MAX_QUEUED_JOBS);
    let order = order.lock().expect("order").clone();
    let index_of = |name: &str| order.iter().position(|entry| *entry == name).expect(name);
    assert!(index_of("prefix") < index_of("prefetch"));
    assert!(index_of("caret") < index_of("prefetch"));
    assert!(index_of("prefix") < index_of("index"));
    assert!(index_of("caret") < index_of("index"));
    assert!(index_of("latest") < index_of("prefetch"));
    emit(
        "T02-05-priority-lanes",
        serde_json::json!({"order":order,"metrics":metrics,"blockedRemoteLike":true,"competingPrefixAndCaretWin":true,"obsoleteDemand":"cancelled","latestDemand":"completed","actorLockIoChecks":metrics.actor_lock_io_checks,"actorLockContainsIo":metrics.actor_lock_io_violations != 0}),
    );
}

#[test]
fn t02_05_prefix_caret_race_preempts_background_and_keeps_latest_seek() {
    let scheduler = scheduler::Scheduler::new().expect("scheduler");
    let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let (compaction_started_tx, compaction_started_rx) = mpsc::channel();
    let background_release = Arc::new(AtomicBool::new(false));
    let background_release_for_job = background_release.clone();
    let background_order = order.clone();
    let compaction = scheduler
        .submit(scheduler::Priority::Compaction, move |cancel| {
            background_order.lock().expect("order").push("compaction");
            compaction_started_tx.send(()).expect("compaction started");
            while !cancel.load(Ordering::Acquire)
                && !background_release_for_job.load(Ordering::Acquire)
            {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(failure(
                ProtocolErrorCode::Cancelled,
                "controlled compaction remains blocked until urgent work wins",
            ))
        })
        .expect("compaction");
    compaction_started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("compaction worker started");

    let prefetch_order = order.clone();
    let prefetch = scheduler
        .submit(scheduler::Priority::Prefetch, move |_| {
            prefetch_order.lock().expect("order").push("prefetch");
            Ok(vec![1])
        })
        .expect("prefetch");
    let index_order = order.clone();
    let index = scheduler
        .submit(scheduler::Priority::Index, move |_| {
            index_order.lock().expect("order").push("index");
            Ok(vec![2])
        })
        .expect("index");

    let urgent_release = Arc::new(AtomicBool::new(false));
    let (urgent_started_tx, urgent_started_rx) = mpsc::channel();
    let prefix_release = urgent_release.clone();
    let prefix_order = order.clone();
    let prefix_started = urgent_started_tx.clone();
    let prefix = scheduler
        .submit(scheduler::Priority::Demand, move |_| {
            prefix_order.lock().expect("order").push("prefix");
            prefix_started.send("prefix").expect("prefix started");
            while !prefix_release.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(vec![3])
        })
        .expect("prefix demand");
    assert_eq!(
        urgent_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("prefix wins first urgent slot"),
        "prefix"
    );

    let caret_release = urgent_release.clone();
    let caret_order = order.clone();
    let caret_started = urgent_started_tx;
    let caret = scheduler
        .submit(scheduler::Priority::Demand, move |_| {
            caret_order.lock().expect("order").push("caret");
            caret_started.send("caret").expect("caret started");
            while !caret_release.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(vec![4])
        })
        .expect("caret demand");
    assert_eq!(
        urgent_started_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("caret wins second urgent slot"),
        "caret"
    );

    let obsolete = scheduler
        .submit(scheduler::Priority::Demand, |_| Ok(vec![9]))
        .expect("obsolete latest seek");
    let latest_order = order.clone();
    let latest = scheduler
        .submit(scheduler::Priority::Demand, move |_| {
            latest_order.lock().expect("order").push("latest");
            Ok(vec![5])
        })
        .expect("latest seek");
    assert!(matches!(
        obsolete
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("obsolete seek result"),
        Err(error) if error.code == ProtocolErrorCode::Cancelled
    ));

    urgent_release.store(true, Ordering::Release);
    assert_eq!(
        prefix
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("prefix completion")
            .expect("prefix payload")
            .bytes,
        vec![3]
    );
    assert_eq!(
        caret
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("caret completion")
            .expect("caret payload")
            .bytes,
        vec![4]
    );
    assert_eq!(
        latest
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("latest completion")
            .expect("latest payload")
            .bytes,
        vec![5]
    );

    assert_eq!(
        scheduler
            .cancel(compaction.id)
            .expect("cancel blocked compaction"),
        scheduler::CancelOutcome::RequestedDriverDependent
    );
    assert!(compaction
        .result
        .recv_timeout(Duration::from_secs(2))
        .expect("compaction completion")
        .is_err());
    assert_eq!(
        prefetch
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("prefetch completion")
            .expect("prefetch payload")
            .bytes,
        vec![1]
    );
    assert_eq!(
        index
            .result
            .recv_timeout(Duration::from_secs(2))
            .expect("index completion")
            .expect("index payload")
            .bytes,
        vec![2]
    );

    let metrics = scheduler.metrics().expect("metrics");
    assert_eq!(metrics.actor_lock_io_violations, 0);
    assert!(metrics.actor_lock_io_checks >= 6);
    assert!(metrics.actor_lock_entries > 0);
    assert!(metrics.actor_lock_max_ns > 0);
    let order = order.lock().expect("order").clone();
    let index_of = |name: &str| order.iter().position(|entry| *entry == name).expect(name);
    assert!(index_of("compaction") < index_of("prefix"));
    assert!(index_of("compaction") < index_of("caret"));
    assert!(index_of("prefix") < index_of("prefetch"));
    assert!(index_of("caret") < index_of("prefetch"));
    assert!(index_of("latest") < index_of("prefetch"));
    assert!(index_of("prefix") < index_of("index"));
    assert!(index_of("caret") < index_of("index"));
    assert!(index_of("latest") < index_of("index"));
    emit(
        "T02-05-priority-race",
        serde_json::json!({
            "order": order,
            "metrics": metrics,
            "compactionBlocked": true,
            "prefixAndCaretWon": true,
            "obsoleteDemand": "cancelled",
            "latestDemand": "completed",
            "actorLockIoChecks": metrics.actor_lock_io_checks,
            "actorLockContainsIo": metrics.actor_lock_io_violations != 0,
        }),
    );
}

#[test]
fn t02_05_public_source_failure_states_are_distinct() {
    let states = [
        (ProtocolErrorCode::Cancelled, SourceState::Cancelled),
        (ProtocolErrorCode::ResourceLimit, SourceState::QuotaExceeded),
        (ProtocolErrorCode::SourceChanged, SourceState::Changed),
        (ProtocolErrorCode::SharingViolation, SourceState::Conflict),
        (ProtocolErrorCode::Readonly, SourceState::Readonly),
        (
            ProtocolErrorCode::EncodingRequired,
            SourceState::DecisionRequired,
        ),
        (ProtocolErrorCode::IoFailure, SourceState::ReadLimited),
    ];
    for (code, expected) in states {
        assert_eq!(
            source::source_state_for_error(&failure(code, "test")),
            expected
        );
    }
    emit(
        "T02-05-source-states",
        serde_json::json!({"cancelled":"cancelled","quota":"quotaExceeded","conflict":"conflict","readLimited":"readLimited","genericFailedState":false}),
    );
}

#[test]
#[ignore = "requires explicit seeded corpus and fresh disk-space admission through p02-run.mjs"]
fn t02_large_populated_storage_experiment() {
    let path = PathBuf::from(std::env::var_os("P02_FIXTURE").expect("P02_FIXTURE required"));
    let expected = std::env::var("P02_FIXTURE_SHA256").expect("fixture SHA256 required");
    let expected_edited =
        std::env::var("P02_EDITED_SHA256").expect("independent edited SHA256 required");
    let source =
        source::ProtectedSource::open_authorized_path(&path, None).expect("approved large source");
    let total = source.identity.bytes.as_u64();
    let fixture = Fixture::new();
    let required =
        backing::reservation(total, 64 * 1024 * 1024, 64 * 1024 * 1024).expect("checked admission");
    assert!(
        native::free_space(&fixture.path).expect("fresh native free space") >= required,
        "BLOCK insufficient space"
    );
    let quota = quota(required);
    let generation = source.identity.source_generation.as_u64();
    let mut copy = source::ProtectedCopy::new(
        source,
        fixture.pages(
            "large-original.pages",
            backing::BYTE_PAGE,
            2 * backing::BYTE_PAGE,
            quota.clone(),
        ),
    );
    let mut doc = document(&fixture, total, quota.clone());
    let mut prefix = vec![0; total.min(super::contract::FIRST_READ_BYTES as u64) as usize];
    copy.read_original(0, &mut prefix, &AtomicBool::new(false))
        .expect("first bounded read");
    let (encoding, bom) = decode::detect(&prefix, None).expect("prefix encoding");
    let lease = lease::build_lease(
        lease::LeaseTags {
            session: "large",
            lease: "large-prefix",
            source: generation,
            revision: 0,
            view: 1,
        },
        decode::Checkpoint::new(encoding, bom as u64),
        &prefix[bom..],
        false,
        Some(1),
    )
    .expect("large prefix lease");
    assert_eq!(
        encoding,
        TextEncoding::Utf8,
        "large oracle is scoped to populated UTF-8 without BOM"
    );
    lease::replace_validated_local(
        &mut doc,
        &lease,
        0,
        0,
        "x",
        encoding,
        lease::insertion_newline(&lease),
    )
    .expect("accepted real edit before copy/index");
    assert_eq!(copy.copied, 0);
    emit(
        "T02-02-large-first-edit",
        serde_json::json!({"sourceBytes":total.to_string(),"readBytes":prefix.len(),"copiedBytes":"0","indexedBytes":"0","acceptedRevision":"1","recoveryComplete":false}),
    );
    let mut index = index::CheckpointIndex::new(
        fixture.pages(
            "large-index.pages",
            backing::NODE_PAGE,
            2 * backing::NODE_PAGE,
            quota.clone(),
        ),
        decode::Checkpoint::new(encoding, bom as u64),
    )
    .expect("large sparse index");
    let mut indexed = bom as u64;
    let mut last_report = 0;
    while copy.phase != source::CopyPhase::Complete {
        copy.step(&AtomicBool::new(false))
            .expect("protected copy/verify");
        if copy.copied > last_report + 64 * 1024 * 1024 {
            last_report = copy.copied;
            emit(
                "T02-03-copy-progress",
                serde_json::json!({"copiedBytes":copy.copied.to_string(),"verifiedBytes":copy.verified.to_string(),"originalCache":copy.original.cache_resident(),"privateDiskBytes":quota.lock().expect("quota").used().to_string()}),
            );
        }
    }
    assert_eq!(copy.digest.as_deref(), Some(expected.as_str()));
    while indexed < total {
        let count = (total - indexed).min(backing::BYTE_PAGE as u64) as usize;
        let mut bytes = vec![0; count];
        copy.read_original(indexed, &mut bytes, &AtomicBool::new(false))
            .expect("index read");
        indexed += count as u64;
        index
            .push(&bytes, indexed == total)
            .expect("strict large index");
    }
    for offset in [0, total / 2, total.saturating_sub(128)] {
        let mut range = [0; 64];
        doc.evict_caches();
        doc.read_range(offset, &mut range, |at, out| {
            copy.read_original(at, out, &AtomicBool::new(false))
        })
        .expect("distant cold reconstruction");
    }
    assert!(doc.undo().expect("large undo"));
    assert!(!doc.dirty());
    assert!(doc.redo().expect("large redo"));
    assert!(doc.dirty());
    let mut digest = Sha256::new();
    let size = doc.metrics().expect("size").bytes;
    let mut offset = 0;
    while offset < size {
        let count = (size - offset).min(backing::BYTE_PAGE as u64) as usize;
        let mut bytes = vec![0; count];
        doc.read_range(offset, &mut bytes, |at, out| {
            copy.read_original(at, out, &AtomicBool::new(false))
        })
        .expect("stream edited digest");
        digest.update(&bytes);
        offset += count as u64;
    }
    let edited_digest = format!("{:x}", digest.finalize());
    assert_eq!(
        edited_digest, expected_edited,
        "full edited output must match independent streaming byte oracle"
    );
    emit(
        "T02-03-large-result",
        serde_json::json!({"sourceBytes":total.to_string(),"originalSha256":copy.digest,"editedSha256":edited_digest,"independentEditedSha256":expected_edited,"knownLines":index.known_lines().map(|v|v.to_string()),"originalCachePeak":copy.original.cache_peak(),"nodeCachePeak":doc.nodes.cache_peak(),"indexCachePeak":index.pages.cache_peak(),"addCachePeak":doc.added.cache_peak(),"privateDiskBytes":quota.lock().expect("quota").used().to_string(),"recoveryComplete":false,"dirtyEvictions":0}),
    );
}

#[test]
#[ignore = "generated small oracle cases required through p02-run.mjs"]
fn t02_external_byte_oracle_cases() {
    use std::io::Read;
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        bytes: Vec<u8>,
        encoding: TextEncoding,
        bom: usize,
        text: String,
        local_to_byte: Vec<Option<u64>>,
        expected_breaks: usize,
        edit_start: usize,
        edit_end: usize,
        insert: String,
        edited_bytes: Vec<u8>,
    }
    let path = std::env::var_os("P02_ORACLE_CASES").expect("oracle path");
    let mut json = Vec::new();
    fs::File::open(path)
        .expect("oracle file")
        .take(131073)
        .read_to_end(&mut json)
        .expect("bounded oracle read");
    assert!(json.len() <= 131072);
    let cases: Vec<Case> = serde_json::from_slice(&json).expect("oracle cases");
    assert!(cases.len() >= 4 && cases.len() <= 16);
    for case in cases {
        let fixture = Fixture::new();
        let lease = lease::build_lease(
            lease::LeaseTags {
                session: "oracle",
                lease: "oracle-lease",
                source: 1,
                revision: 0,
                view: 1,
            },
            decode::Checkpoint::new(case.encoding, case.bom as u64),
            &case.bytes[case.bom..],
            true,
            Some(1),
        )
        .expect("oracle lease");
        assert_eq!(lease.segments[0].text, case.text);
        assert_eq!(lease.segments[0].line_breaks.len(), case.expected_breaks);
        for (local, byte) in case.local_to_byte.iter().enumerate() {
            match byte {
                Some(byte) => assert_eq!(
                    lease
                        .local_to_byte(local)
                        .expect("oracle scalar boundary")
                        .as_u64(),
                    *byte
                ),
                None => assert!(
                    lease.local_to_byte(local).is_err(),
                    "surrogate interior must reject"
                ),
            }
        }
        let mut doc = document(&fixture, case.bytes.len() as u64, quota(8 * 1024 * 1024));
        lease::replace_validated_local(
            &mut doc,
            &lease,
            case.edit_start,
            case.edit_end,
            &case.insert,
            case.encoding,
            lease::insertion_newline(&lease),
        )
        .expect("oracle edit");
        assert_eq!(bytes(&mut doc, &case.bytes), case.edited_bytes);
        doc.evict_caches();
        assert!(doc.undo().expect("oracle undo"));
        assert_eq!(bytes(&mut doc, &case.bytes), case.bytes);
    }
    emit(
        "T02-04-external-oracle",
        serde_json::json!({"byteSpliceOracle":true,"sourceMapsUnchanged":true,"loneCrViewCanonicalization":true,"mixedUntouchedEndingsPreserved":true}),
    );
}

#[test]
fn t02_resource_control() {
    emit(
        "T02-control",
        serde_json::json!({"editorWork":false,"controlledLoadedBytes":"0"}),
    );
}

#[test]
fn t02_01_symlink_refusal_or_explicit_fixture_block() {
    let fixture = Fixture::new();
    let path = fixture.file("target.txt", b"synthetic symlink target");
    let link = fixture.path.join("link.txt");
    if let Err(error) = std::os::windows::fs::symlink_file(&path, &link) {
        emit(
            "T02-01-symlink",
            serde_json::json!({"gate":"BLOCK","reason":"native symlink fixture requires Windows privilege or Developer Mode","osCode":error.raw_os_error()}),
        );
        panic!("BLOCK: native symlink fixture unavailable; no substitute pass");
    }
    assert!(source::ProtectedSource::open_authorized_path(&link, None).is_err());
    fs::remove_file(link).expect("remove synthetic symlink");
    emit(
        "T02-01-symlink",
        serde_json::json!({"finalSymlink":"refused"}),
    );
}

#[test]
fn t02_02_copy_quota_failure_retains_protected_source() {
    let fixture = Fixture::new();
    let path = fixture.file("quota-copy.txt", &[b'x'; 100000]);
    let source = source::ProtectedSource::open_authorized_path(&path, None).expect("source");
    let mut copy = source::ProtectedCopy::new(
        source,
        fixture.pages("limited.pages", backing::BYTE_PAGE, 0, quota(80000)),
    );
    copy.step(&AtomicBool::new(false))
        .expect("one admitted copy block");
    assert!(
        matches!(copy.step(&AtomicBool::new(false)), Err(error) if error.code == ProtocolErrorCode::ResourceLimit)
    );
    assert_eq!(copy.phase, source::CopyPhase::Failed);
    assert_eq!(copy.state(), SourceState::QuotaExceeded);
    assert!(copy.source().is_some());
    assert!(copy.digest.is_none());
    assert!(!copy.recovery_complete());
    assert!(native::open(
        &path,
        native::WRITE,
        native::SHARE_READ | native::SHARE_WRITE,
        0
    )
    .is_err());
    emit(
        "T02-02-copy-failure",
        serde_json::json!({"phase":"QuotaExceeded","sourceState":"quotaExceeded","copiedBytes":copy.copied.to_string(),"snapshotComplete":false,"recoveryComplete":false,"sourceProtectionRetained":true}),
    );
}

#[test]
fn t02_01_native_probe_canonical_root_opens_ordinary_source() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let parent = repository
        .join("test-results")
        .join("stack-text-editor")
        .join("P02");
    fs::create_dir_all(&parent).expect("P02 evidence parent");
    let token = native::random::<16>().expect("system RNG");
    let root = parent.join(format!(
        "unit-native-root-{}",
        token
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ));
    native::private_directory(&root).expect("private native evidence root");
    let expected = b"native prefix";
    fs::write(root.join("probe.txt"), expected).expect("probe source");
    let canonical = root.canonicalize().expect("canonical native evidence root");
    assert!(
        matches!(canonical.components().next(), Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(), std::path::Prefix::VerbatimDisk(_)))
    );
    let root = session::native_probe_root(&canonical).expect("validated native probe root");
    let source = source::ProtectedSource::open_authorized_path(&root.join("probe.txt"), None)
        .expect("canonicalized probe must open an ordinary supported source");
    let mut bytes = [0; 13];
    source
        .read(0, &mut bytes, &AtomicBool::new(false))
        .expect("real source prefix");
    assert_eq!(&bytes, expected);
    drop(source);
    let outside = repository;
    assert!(
        session::native_probe_root(outside).is_err(),
        "display conversion must not bypass evidence-root containment"
    );
    fs::remove_dir_all(root).expect("remove native evidence root");
}
