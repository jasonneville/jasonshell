//! Bounded actual binary-module runner. Never runs production main or fake runtime.
use std::{fs,process::Command,thread,time::{Duration,Instant}};
const CASES: &[&str] = &[
    "speech_folder_runtime_busy_rejects_both_kinds_before_all_dependencies",
    "speech_folder_runtime_cancel_both_kinds_resets_invalidated_pool_and_releases_once",
    "speech_folder_runtime_chooser_error_releases_without_resolving_data_or_loading",
    "speech_folder_runtime_data_error_occurs_only_after_selection_and_releases",
    "speech_folder_runtime_folder_loader_failure_preserves_prior_disk_generation",
    "speech_folder_runtime_archive_loader_failure_preserves_prior_disk_generation",
    "speech_folder_runtime_folder_reservation_blocks_archive_without_clearing_owner",
    "speech_folder_runtime_archive_reservation_blocks_folder_without_clearing_owner",
    "speech_folder_runtime_unwind_releases_admission_and_retries_without_disk",
    "speech_folder_runtime_stale_error_warmup_cannot_overwrite_cancel_or_failed_import",
];
struct Log(std::path::PathBuf);
impl Drop for Log {fn drop(&mut self) {let _=fs::remove_file(&self.0);}}
fn actual_binary(args:&[&str],phase:&str)->String {
    let path=std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode")
        .join(format!("speech-runtime-folder-runner-{}-{phase}.log",std::process::id()));
    let file=fs::OpenOptions::new().create_new(true).write(true).open(&path).unwrap();let log=Log(path);
    let mut child=Command::new(env!("CARGO")).current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(args).stdout(file.try_clone().unwrap()).stderr(file).spawn().unwrap();
    let deadline=Instant::now()+Duration::from_secs(100);
    let status=loop {
        if let Some(status)=child.try_wait().unwrap() {break status;}
        if Instant::now()>=deadline {child.kill().expect("terminate only owned Cargo child");child.wait().unwrap();panic!("bounded actual runtime {phase} timeout");}
        thread::sleep(Duration::from_millis(25));
    };
    let text=fs::read_to_string(&log.0).unwrap();drop(log);
    assert!(status.success(),"actual runtime {phase} failed (no production main run):\n{}",text.lines().rev().take(60).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"));text
}
#[test]
fn actual_speech_folder_runtime_requires_ten_embedded_bodies() {
    let listing=actual_binary(&["test","--bin","jason-shell","speech_folder_runtime_","--","--list"],"list");
    for case in CASES {
        assert!(listing.lines().any(|line|line==format!("speech_runtime::tests::{case}: test")),
            "missing actual State cfg(test) bridge/case {case}; zero-test GREEN forbidden");
    }
    assert_eq!(listing.lines().filter(|line|line.starts_with("speech_runtime::tests::speech_folder_runtime_")&&line.ends_with(": test")).count(),CASES.len());
    let run=actual_binary(&["test","--bin","jason-shell","speech_folder_runtime_","--","--test-threads=1"],"run");
    for case in CASES {assert!(run.lines().any(|line|line.contains(&format!("test speech_runtime::tests::{case} ..."))&&line.ends_with("ok")),"actual runtime body did not pass: {case}");}
    assert!(run.contains("test result: ok. 10 passed; 0 failed; 0 ignored;"),"all ten real runtime bodies must execute, none ignored");
}
