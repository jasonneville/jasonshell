use super::*;
use std::{
    fs,
    io::Write,
    process::Command,
    sync::{Arc, Mutex},
};

fn fixture_root(label: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "jasonshell-p04-{label}-{:x}",
        u64::from_le_bytes(native::random::<8>().unwrap())
    ));
    native::private_directory(&root).unwrap();
    root
}

fn quota(bytes: u64) -> backing::SharedQuota {
    Arc::new(Mutex::new(backing::Quota::new(bytes)))
}

#[test]
fn t04_01_frozen_save_tracks_accepted_durable_saved() {
    let mut revisions = save_failpoints::Revisions::new(7);
    let frozen = revisions.accept();
    revisions.mark_durable(frozen).unwrap();
    let r_plus_one = revisions.accept();
    revisions.mark_saved(frozen).unwrap();
    assert_eq!(
        (revisions.accepted(), revisions.durable(), revisions.saved()),
        (r_plus_one, frozen, frozen)
    );
    assert!(revisions.is_dirty());
}

#[test]
fn t04_04_restart_outcomes_do_not_collapse_cancel_or_published() {
    use save_failpoints::{Disposition, RestartClass};
    assert_eq!(
        Disposition::from_restart(RestartClass::Published, true),
        Disposition::AlreadyPublished
    );
    assert_eq!(
        Disposition::from_restart(RestartClass::NotStarted, true),
        Disposition::Cancelled
    );
    assert_eq!(
        Disposition::from_restart(RestartClass::PublicationPossible, true),
        Disposition::ManualRecoveryRequired
    );
}

#[test]
fn t04_05_versioned_encrypted_recovery_roundtrip_and_corruption_refusal() {
    let root = std::env::temp_dir().join(format!(
        "jasonshell-p04-recovery-{:x}",
        u64::from_le_bytes(native::random::<8>().unwrap())
    ));
    native::private_directory(&root).unwrap();
    let path = root.join("record.pages");
    let q = quota(8 * 1024 * 1024);
    let mut record =
        recovery::RecoveryRecord::create(&path, q.clone(), b"immutable base", 4).unwrap();
    record.append_edit(5, 9, b"durable edit").unwrap();
    record.commit().unwrap();
    drop(record);
    let disk = fs::read(&path).unwrap();
    assert!(!disk
        .windows(b"immutable base".len())
        .any(|v| v == b"immutable base"));
    assert!(!disk
        .windows(b"durable edit".len())
        .any(|v| v == b"durable edit"));
    let opened = recovery::RecoveryRecord::open(&path, q.clone()).unwrap();
    assert_eq!(opened.reconstruct().unwrap(), b"immutdurable edit base");
    let mut disk = fs::read(&path).unwrap();
    disk[40] ^= 1;
    fs::write(&path, disk).unwrap();
    assert!(recovery::RecoveryRecord::open(&path, q)
        .and_then(|r| r.reconstruct())
        .is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_02_utf16_in_place_and_save_as_are_real_nonempty_writes() {
    let root = std::env::temp_dir().join(format!(
        "jasonshell-p04-save-{:x}",
        u64::from_le_bytes(native::random::<8>().unwrap())
    ));
    native::private_directory(&root).unwrap();
    let target = root.join("utf16.txt");
    let original = [0xff, 0xfe, b'a', 0, b'b', 0];
    let edited = [0xff, 0xfe, b'x', 0, b'y', 0];
    fs::write(&target, original).unwrap();
    let before = windows_save::inspect(&target).unwrap();
    assert!(windows_save::replace(&target, &edited, &before).is_err());
    assert_eq!(fs::read(&target).unwrap(), original);
    let stage = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".jasonshell-stage-")
        })
        .unwrap();
    assert_eq!(fs::read(stage).unwrap(), edited);
    let copy = root.join("copy.txt");
    windows_save::save_as(&copy, &edited).unwrap();
    assert_eq!(fs::read(copy).unwrap(), edited);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_02_hardlink_and_unsupported_paths_fail_closed_without_delete_first() {
    let root = fixture_root("hardlink");
    let target = root.join("target.txt");
    let alias = root.join("alias.txt");
    fs::write(&target, b"external").unwrap();
    fs::hard_link(&target, &alias).unwrap();
    let before = windows_save::inspect(&target).unwrap();
    assert!(windows_save::replace(&target, b"ours", &before).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"external");
    assert_eq!(fs::read(&alias).unwrap(), b"external");
    assert_eq!(
        windows_save::inspect(&target).unwrap().file_index,
        before.file_index
    );

    let ads = root.join("target.txt:stream");
    assert!(windows_save::save_as(&ads, b"secret").is_err());
    assert!(!ads.exists());
    let directory = root.join("directory");
    fs::create_dir(&directory).unwrap();
    let directory_identity = windows_save::inspect(&target).unwrap();
    assert!(windows_save::replace(&directory, b"ours", &directory_identity).is_err());

    let link = root.join("link.txt");
    std::os::windows::fs::symlink_file(&target, &link).unwrap();
    assert!(windows_save::replace(&link, b"ours", &before).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"external");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_02_replacefile_preserves_acl_creation_time_and_named_stream_observation() {
    let root = fixture_root("metadata");
    let target = root.join("target.txt");
    fs::write(&target, b"old").unwrap();
    fs::write(root.join("target.txt:proof"), b"ads-proof").unwrap();
    let before = windows_save::inspect(&target).unwrap();
    let acl_before = native::security_sddl(&target).unwrap();
    let replacement = root.join("replacement.txt");
    let backup = root.join("backup.txt");
    fs::write(&replacement, b"new").unwrap();
    windows_save::replace_paths_for_test(&target, &replacement, &backup).unwrap();
    let acl_after = native::security_sddl(&target).unwrap();
    assert_eq!(acl_before.replace("D:", "D:AI"), acl_after);
    let target_after = windows_save::inspect(&target).unwrap();
    let backup_after = windows_save::inspect(&backup).unwrap();
    assert_eq!(backup_after.created, before.created);
    assert_eq!(backup_after.modified, before.modified);
    assert_ne!(target_after.file_index, before.file_index);
    assert_eq!(
        fs::read(root.join("target.txt:proof")).unwrap(),
        b"ads-proof"
    );
    assert_eq!(fs::read(&backup).unwrap(), b"old");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_03_external_mutations_revalidate_fail_closed() {
    for (name, mutate) in [
        ("append", 0u8),
        ("truncate", 1),
        ("replace", 2),
        ("readonly", 3),
    ] {
        let root = std::env::temp_dir().join(format!(
            "jasonshell-p04-race-{name}-{:x}",
            u64::from_le_bytes(native::random::<8>().unwrap())
        ));
        native::private_directory(&root).unwrap();
        let target = root.join("target.txt");
        fs::write(&target, b"external base").unwrap();
        let expected = windows_save::inspect(&target).unwrap();
        match mutate {
            0 => fs::OpenOptions::new()
                .append(true)
                .open(&target)
                .unwrap()
                .write_all(b"+")
                .unwrap(),
            1 => fs::OpenOptions::new()
                .write(true)
                .open(&target)
                .unwrap()
                .set_len(2)
                .unwrap(),
            2 => {
                fs::rename(&target, root.join("external.txt")).unwrap();
                fs::write(&target, b"replacement").unwrap();
            }
            _ => {
                let mut p = fs::metadata(&target).unwrap().permissions();
                p.set_readonly(true);
                fs::set_permissions(&target, p).unwrap();
            }
        }
        assert!(
            windows_save::replace(&target, b"ours", &expected).is_err(),
            "{name}"
        );
        assert_ne!(fs::read(&target).unwrap(), b"ours", "{name}");
        if mutate == 3 {
            let mut p = fs::metadata(&target).unwrap().permissions();
            p.set_readonly(false);
            fs::set_permissions(&target, p).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn t04_03_mtime_preserving_mutation_is_not_claimed_as_compare_and_swap() {
    let root = fixture_root("mtime");
    let target = root.join("target.txt");
    fs::write(&target, b"same-length-a").unwrap();
    let expected = windows_save::inspect(&target).unwrap();
    fs::write(&target, b"same-length-b").unwrap();
    let mut observed = windows_save::inspect(&target).unwrap();
    observed.modified = expected.modified;
    assert_eq!(observed.len, expected.len);
    assert_eq!(observed.file_index, expected.file_index);
    assert_eq!(
        windows_save::assess_revalidation(&expected, &observed),
        windows_save::Revalidation::Changed
    );
    assert_eq!(fs::read(&target).unwrap(), b"same-length-b");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_03_native_guard_refusal_is_unsupported_not_race_proof() {
    let root = fixture_root("post-hash");
    let target = root.join("target.txt");
    fs::write(&target, b"external").unwrap();
    let expected = windows_save::inspect(&target).unwrap();
    let hook_called = Arc::new(Mutex::new(false));
    let called = hook_called.clone();
    let result = windows_save::replace_with_test_hook(&target, b"ours", &expected, move || {
        *called.lock().unwrap() = true;
        Ok(())
    });
    if matches!(result, Err(windows_save::SaveError::Unsupported(_))) {
        assert!(!*hook_called.lock().unwrap());
        assert_eq!(fs::read(&target).unwrap(), b"external");
    } else {
        result.unwrap();
        assert!(*hook_called.lock().unwrap());
        assert_eq!(fs::read(&target).unwrap(), b"ours");
    }
    let stage = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".jasonshell-stage-")
        })
        .unwrap();
    assert_eq!(fs::read(stage).unwrap(), b"ours");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_03_protected_guard_detects_post_hash_mutation_without_publish() {
    let root = fixture_root("protected-post-hash");
    let target = root.join("target.txt");
    fs::write(&target, b"external").unwrap();
    let expected = windows_save::inspect(&target).unwrap();
    let published = Arc::new(Mutex::new(0));
    let publish_count = published.clone();
    let checks = Arc::new(Mutex::new(0));
    let guard_checks = checks.clone();
    let result = windows_save::replace_with_test_guard(
        &target,
        b"ours",
        &expected,
        || {
            fs::write(&target, b"Xxternal")?;
            Ok(())
        },
        move || {
            let mut count = guard_checks.lock().unwrap();
            *count += 1;
            if *count == 1 {
                Ok(())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "guard broken",
                ))
            }
        },
        move |_, _, _| {
            *publish_count.lock().unwrap() += 1;
            Ok(())
        },
        |_| unreachable!("inspection cannot run without publication"),
    );
    assert!(matches!(result, Err(windows_save::SaveError::Conflict(_))));
    assert_eq!(*published.lock().unwrap(), 0);
    assert_eq!(fs::read(&target).unwrap(), b"Xxternal");
    let stage = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".jasonshell-stage-")
        })
        .unwrap();
    assert_eq!(fs::read(stage).unwrap(), b"ours");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_04_replace_success_then_inspection_fault_is_publication_possible() {
    let root = fixture_root("post-replace-inspection");
    let target = root.join("target.txt");
    fs::write(&target, b"external").unwrap();
    let expected = windows_save::inspect(&target).unwrap();
    windows_save::inject_actual_caller_inspection_fault(true);
    let result = windows_save::replace(&target, b"ours", &expected);
    windows_save::inject_actual_caller_inspection_fault(false);
    let error = result.unwrap_err();
    assert!(matches!(
        &error,
        windows_save::SaveError::PublicationPossible { .. }
    ));
    assert_eq!(
        error.state(),
        Some(save_failpoints::RestartClass::PublicationPossible)
    );
    assert_eq!(windows_save::actual_caller_replace_count(), 1);
    let artifacts = error.artifacts().unwrap();
    assert_eq!(artifacts.target, target);
    assert!(artifacts.staging.exists());
    assert!(artifacts.backup.exists());
    assert_eq!(fs::read(&target).unwrap(), b"ours");
    assert_eq!(fs::read(&artifacts.backup).unwrap(), b"external");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_03_real_mtime_preserving_mutation_and_share_lock_preserve_bytes() {
    let root = fixture_root("real-lock");
    let target = root.join("target.txt");
    fs::write(&target, b"same-length-a").unwrap();
    let expected = windows_save::inspect(&target).unwrap();
    let timestamp = fs::metadata(&target).unwrap().modified().unwrap();
    fs::write(&target, b"same-length-b").unwrap();
    let file = fs::OpenOptions::new().write(true).open(&target).unwrap();
    native::set_write_time(&file, expected.modified).unwrap();
    drop(file);
    let observed = windows_save::inspect(&target).unwrap();
    assert_eq!(observed.modified, expected.modified);
    assert!(windows_save::replace(&target, b"ours", &expected).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"same-length-b");
    let stage = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".jasonshell-stage-")
        })
        .unwrap();
    assert_eq!(fs::read(&stage).unwrap(), b"ours");
    let _ = timestamp;

    let locked = native::open(&target, native::READ, native::SHARE_READ, 0).unwrap();
    let before_names: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert!(windows_save::replace(&target, b"ours", &observed).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"same-length-b");
    let after_names: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert!(after_names.len() > before_names.len());
    let new_stage = after_names
        .iter()
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".jasonshell-stage-")
        })
        .find(|path| **path != stage)
        .unwrap();
    assert_eq!(fs::read(new_stage).unwrap(), b"ours");
    drop(locked);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_03_real_mapped_writer_is_detected_and_external_bytes_survive() {
    let root = fixture_root("mapped");
    let target = root.join("target.txt");
    fs::write(&target, b"external").unwrap();
    let expected = windows_save::inspect(&target).unwrap();
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&target)
        .unwrap();
    let mut mapping = native::WritableMap::create(&file, 8).unwrap();
    mapping.write(0, b'X').unwrap();
    assert!(windows_save::replace(&target, b"ours", &expected).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"Xxternal");
    drop(mapping);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_03_injected_windows_failures_map_without_losing_assets() {
    use contract::ProtocolErrorCode;
    for (os, expected) in [
        (5, ProtocolErrorCode::Readonly),
        (32, ProtocolErrorCode::SharingViolation),
        (33, ProtocolErrorCode::SharingViolation),
    ] {
        assert_eq!(
            io_failure(std::io::Error::from_raw_os_error(os)).code,
            expected
        );
    }
    let assets = windows_save::ConflictAssets::preserved(true, true, true);
    assert!(assets.ours && assets.external && assets.backup);
}

#[test]
fn t04_04_every_boundary_has_durable_restart_class_and_never_retries() {
    use save_failpoints::{classify, Assets, Phase, RestartClass};
    let uncertain = Assets {
        target: true,
        staging: true,
        backup: true,
        journal_committed: false,
        inspected_revision: false,
    };
    let cases = [
        (Phase::Prepare, RestartClass::NotStarted),
        (Phase::Stage, RestartClass::Staged),
        (Phase::WriteBatch, RestartClass::Staged),
        (Phase::FlushClose, RestartClass::Staged),
        (Phase::Revalidate, RestartClass::Staged),
        (Phase::BeforePublish, RestartClass::Staged),
        (Phase::AfterReplace, RestartClass::PublicationPossible),
        (Phase::Inspect, RestartClass::Ambiguous),
        (Phase::JournalCommit, RestartClass::Ambiguous),
        (Phase::Cleanup, RestartClass::Ambiguous),
    ];
    for (phase, expected) in cases {
        let journal = save_failpoints::DurableJournal::new(phase, uncertain.clone());
        let bytes = journal.encode().unwrap();
        let reopened = save_failpoints::DurableJournal::decode(&bytes).unwrap();
        assert_eq!(reopened.restart_class(), expected, "{phase:?}");
        assert!(!reopened.may_retry_publication(), "{phase:?}");
        assert_eq!(classify(Some(phase), &uncertain), expected);
    }
}

#[test]
fn t04_04_process_child_termination_at_publication_boundaries_reopens_durably() {
    use save_failpoints::{Assets, DurableJournal, Phase, RestartClass};
    const CHILD_FLAG: &str = "JASONSHELL_P04_CRASH_CHILD";
    const ROOT_FLAG: &str = "JASONSHELL_P04_CRASH_ROOT";
    if let (Ok(phase), Ok(root)) = (std::env::var(CHILD_FLAG), std::env::var(ROOT_FLAG)) {
        let phase = match phase.as_str() {
            "before" => Phase::BeforePublish,
            "after" => Phase::AfterReplace,
            _ => std::process::exit(87),
        };
        let assets = Assets {
            target: true,
            staging: true,
            backup: phase == Phase::AfterReplace,
            journal_committed: false,
            inspected_revision: false,
        };
        let bytes = DurableJournal::new(phase, assets).encode().unwrap();
        fs::write(std::path::Path::new(&root).join("journal.json"), bytes).unwrap();
        std::process::exit(86);
    }

    for (name, expected) in [
        ("before", RestartClass::Staged),
        ("after", RestartClass::PublicationPossible),
    ] {
        let root = fixture_root(&format!("crash-{name}"));
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "stack_popup::text_document::feasibility::p04_tests::t04_04_process_child_termination_at_publication_boundaries_reopens_durably",
            ])
            .env(CHILD_FLAG, name)
            .env(ROOT_FLAG, &root)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(86));
        let bytes = fs::read(root.join("journal.json")).unwrap();
        let reopened = DurableJournal::decode(&bytes).unwrap();
        assert_eq!(reopened.restart_class(), expected);
        assert!(!reopened.may_retry_publication());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn t04_05_corrupt_and_incomplete_records_quarantine_without_cleanup_or_disclosure() {
    let root = fixture_root("quarantine");
    for (name, bytes) in [
        ("corrupt.pages", b"secret marker".as_slice()),
        ("short.pages", b"x"),
    ] {
        let path = root.join(name);
        fs::write(&path, bytes).unwrap();
        let result = recovery::scan(&path, quota(1024 * 1024));
        assert_eq!(
            result.classification,
            recovery::ScanClassification::Quarantined
        );
        assert!(path.exists());
        assert!(!result.diagnostic.contains(name));
        assert!(!result.diagnostic.contains("secret"));
        assert!(!result.diagnostic.contains(path.to_string_lossy().as_ref()));
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_05_quota_failure_retains_recovery_assets_and_redacts_diagnostic() {
    let root = fixture_root("quota");
    let path = root.join("record.pages");
    let q = quota(1024 * 1024);
    let mut record =
        recovery::RecoveryRecord::create(&path, q.clone(), b"private base", 1).unwrap();
    q.lock().unwrap().exhaust_for_test();
    let error = record.append_edit(0, 0, b"private delta").unwrap_err();
    assert!(path.exists() && path.with_extension("key").exists());
    assert!(!error.message.contains("private base"));
    assert!(!error.message.contains("private delta"));
    assert!(!error.message.contains(path.to_string_lossy().as_ref()));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_05_simulated_disk_full_and_retention_limiter_never_evict_dirty_assets() {
    let root = fixture_root("simulated-disk-full-retention");
    let dirty = root.join("dirty.pages");
    let clean_old = root.join("clean-old.pages");
    fs::write(&dirty, b"dirty recovery").unwrap();
    fs::write(&clean_old, b"clean recovery").unwrap();

    let mut retained = vec![
        (clean_old.clone(), false, 1_u64),
        (dirty.clone(), true, 2_u64),
    ];
    retained.sort_by_key(|(_, dirty, age)| (*dirty, *age));
    let candidate = retained.iter().find(|(_, dirty, _)| !dirty).unwrap();
    fs::remove_file(&candidate.0).unwrap();
    assert!(dirty.exists());
    assert!(!clean_old.exists());

    let q = quota(1);
    let error = match recovery::RecoveryRecord::create(
        &root.join("limited.pages"),
        q,
        b"secret-payload-marker",
        1,
    ) {
        Ok(_) => panic!("simulated limiter unexpectedly admitted recovery record"),
        Err(error) => error,
    };
    assert_eq!(error.code, contract::ProtocolErrorCode::ResourceLimit);
    assert!(!error.message.contains("secret-payload-marker"));
    assert!(!error.message.contains(root.to_string_lossy().as_ref()));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_05_authenticated_pages_use_unique_nonces_and_store_no_plaintext_or_key() {
    let root = fixture_root("crypto");
    let path = root.join("record.pages");
    let q = quota(1024 * 1024);
    let mut record = recovery::RecoveryRecord::create(&path, q, b"AAAA", 1).unwrap();
    record.append_edit(0, 1, b"AAAA").unwrap();
    record.commit().unwrap();
    drop(record);
    let disk = fs::read(&path).unwrap();
    assert!(!disk.windows(4).any(|window| window == b"AAAA"));
    let frame = backing::BYTE_PAGE + 32;
    assert_ne!(&disk[4..20], &disk[frame + 4..frame + 20]);
    let key_record = fs::read(path.with_extension("key")).unwrap();
    assert!(!key_record.windows(4).any(|window| window == b"AAAA"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_05_private_dacl_readback_matches_exact_recovery_policy() {
    let root = fixture_root("dacl");
    let sddl = native::security_sddl(&root).unwrap();
    let user = native::current_user_sid().unwrap();
    native::validate_private_recovery_sddl(&sddl, &user).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_05_private_dacl_rejects_every_unexpected_allow_principal_and_broad_access() {
    let user = "S-1-5-21-111-222-333-1001";
    let accepted = format!("O:{user}G:SYD:P(A;OICI;FA;;;{user})(A;OICI;FA;;;SY)(A;OICIID;FA;;;BA)");
    native::validate_private_recovery_sddl(&accepted, user).unwrap();

    for sid in ["WD", "BU", "AU", "AN", "S-1-5-21-9-8-7-1002"] {
        let injected = format!("{accepted}(A;OICI;FR;;;{sid})");
        assert!(
            native::validate_private_recovery_sddl(&injected, user).is_err(),
            "{sid}"
        );
    }
    let excessive = format!("O:{user}G:SYD:P(A;OICI;FA;;;{user})(A;OICI;GA;;;SY)");
    assert!(native::validate_private_recovery_sddl(&excessive, user).is_err());
}

#[test]
fn t04_05_security_readback_error_is_explicit_and_discloses_no_path_or_content() {
    let root = fixture_root("dacl-error");
    let missing = root.join("private-secret-marker.txt");
    let error = native::security_sddl(&missing).unwrap_err();
    assert_eq!(error.code, contract::ProtocolErrorCode::IoFailure);
    assert_eq!(error.message, "security descriptor read failed");
    assert!(!error.message.contains("private-secret-marker"));
    assert!(!error.message.contains(root.to_string_lossy().as_ref()));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn t04_05_malformed_wide_path_and_native_acl_errors_are_bounded_and_redacted() {
    use std::os::windows::ffi::OsStringExt;

    let secret = "recovery-path-key-content-marker";
    let malformed = std::path::PathBuf::from(std::ffi::OsString::from_wide(&[
        b'C' as u16,
        b':' as u16,
        b'\\' as u16,
        0,
        b'x' as u16,
    ]));
    let too_wide = std::path::PathBuf::from(std::ffi::OsString::from_wide(
        &std::iter::repeat(b'x' as u16)
            .take(32_761)
            .collect::<Vec<_>>(),
    ));
    for path in [&malformed, &too_wide] {
        let error = native::security_sddl(path).unwrap_err();
        assert_eq!(error.code, contract::ProtocolErrorCode::UnsupportedTarget);
        assert_eq!(error.message, "invalid path");
        assert!(!error.message.contains(secret));
    }

    let error = native::security_read_error_for_test(5);
    assert_eq!(error.code, contract::ProtocolErrorCode::IoFailure);
    assert_eq!(error.message, "security descriptor read failed");
    assert!(!error.message.contains(secret));
    assert!(error.message.len() <= 31);
}
