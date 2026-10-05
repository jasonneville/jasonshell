//! Entry point for test-owned acceptance children compiled in the ACTUAL binary
//! settings/registry modules. No copied validator/registry or Tauri shell launch.
//! Production owner must add the two cfg(test) include bridges documented in
//! tests/snipping-product-handoff.md. Missing bridge is an explicit failure, not 0-test GREEN.
use std::{process::Command, time::{Duration, Instant}, thread};
fn cargo(args: &[&str]) -> std::process::Output {
    let output = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from).unwrap()
        .join("Temp/opencode").join(format!("snip-hotkey-child-{}.log",std::process::id()));
    let stdout=std::fs::File::create(&output).unwrap(); let stderr=stdout.try_clone().unwrap();
    let mut child=Command::new(env!("CARGO")).current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(args).stdout(stdout).stderr(stderr).spawn().unwrap();
    let deadline=Instant::now()+Duration::from_secs(90);
    let status=loop {
        if let Some(status)=child.try_wait().unwrap() { break status; }
        if Instant::now()>=deadline { let _=child.kill(); let _=child.wait(); panic!("bounded acceptance compiler/runner timeout"); }
        thread::sleep(Duration::from_millis(25));
    };
    let bytes=std::fs::read(&output).unwrap(); let _=std::fs::remove_file(output);
    std::process::Output { status, stdout:bytes, stderr:Vec::new() }
}
#[test]
fn snipping_hotkey_actual_production_settings_registry_acceptance() {
    let listed=cargo(&["test","--bin","jason-shell","snip_hotkey_","--","--list"]);
    assert!(listed.status.success(),"production acceptance compilation failed: {}",String::from_utf8_lossy(&listed.stdout));
    let listing=String::from_utf8_lossy(&listed.stdout);
    let expected=["snip_hotkey_default_fifth_preserves_old_four_canonical_chords",
        "snip_hotkey_legacy_missing_fifth_load_adds_default_without_file_rewrite",
        "snip_hotkey_explicit_five_way_canonical_duplicate_is_rejected_not_defaulted",
        "snip_hotkey_unrelated_save_of_legacy_conflict_never_registers_or_persists",
        "snip_hotkey_repair_registers_before_persist_and_failure_restores_exact_prior_chords",
        "snip_hotkey_registry_preserves_ids_one_to_four_adds_fifth_no_repeat_alt_s",
        "snip_hotkey_legacy_conflict_leaves_fifth_vacant_until_repaired"];
    for name in expected { assert!(listing.lines().any(|line|line.ends_with(&format!("{name}: test"))),
        "missing actual production acceptance bridge/case {name}; zero collected tests is NOT a pass"); }
    let result=cargo(&["test","--bin","jason-shell","snip_hotkey_","--","--test-threads=1"]);
    assert!(result.status.success(),"actual settings/registry acceptance failed: {}",String::from_utf8_lossy(&result.stdout));
}
