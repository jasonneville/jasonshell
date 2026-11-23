//! Tier 0 WIN-04 acceptance: crash-safe taskbar restore marker.
//!
//! Required seams in `explorer.rs` (all pure / dir-injected):
//!
//! ```ignore
//! pub(crate) const HIDDEN_MARKER_FILE_NAME: &str = "taskbar-hidden-by-jasonshell.marker";
//! /// observed_visible || marker_present. A leftover marker means a previous
//! /// JasonShell run hid the taskbar and died: treat as originally visible.
//! pub(crate) fn effective_originally_visible(observed_visible: bool, marker_present: bool) -> bool;
//! /// Creates `dir` if missing, then the marker file. Idempotent.
//! pub(crate) fn write_hidden_marker(dir: &Path) -> std::io::Result<()>;
//! pub(crate) fn hidden_marker_present(dir: &Path) -> bool;
//! /// Idempotent: Ok(()) when the marker (or dir) does not exist.
//! pub(crate) fn clear_hidden_marker(dir: &Path) -> std::io::Result<()>;
//! ```
use super::*;
use std::path::PathBuf;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "jasonshell-tier0-marker-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn effective_visibility_truth_table() {
    assert!(effective_originally_visible(true, false));
    assert!(effective_originally_visible(true, true));
    assert!(
        effective_originally_visible(false, true),
        "stale marker => hidden by us => treat visible"
    );
    assert!(
        !effective_originally_visible(false, false),
        "user-hidden taskbar stays hidden"
    );
}

#[test]
fn marker_lifecycle_write_detect_clear() {
    let dir = scratch("lifecycle");
    assert!(!hidden_marker_present(&dir), "missing dir => no marker");
    write_hidden_marker(&dir).unwrap();
    assert!(hidden_marker_present(&dir));
    assert!(dir.join(HIDDEN_MARKER_FILE_NAME).is_file());
    clear_hidden_marker(&dir).unwrap();
    assert!(!hidden_marker_present(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn marker_write_creates_nested_missing_directories_and_is_idempotent() {
    let root = scratch("nested");
    let dir = root.join("a").join("b");
    write_hidden_marker(&dir).unwrap();
    write_hidden_marker(&dir).unwrap();
    assert!(hidden_marker_present(&dir));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn clear_is_idempotent_when_marker_or_dir_missing() {
    let dir = scratch("clear-missing");
    clear_hidden_marker(&dir).unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    clear_hidden_marker(&dir).unwrap();
    clear_hidden_marker(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn clearing_one_dir_does_not_touch_another() {
    let a = scratch("iso-a");
    let b = scratch("iso-b");
    write_hidden_marker(&a).unwrap();
    write_hidden_marker(&b).unwrap();
    clear_hidden_marker(&a).unwrap();
    assert!(!hidden_marker_present(&a));
    assert!(hidden_marker_present(&b));
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

// ---- follow-up: marker is cleared only after EVERY owned taskbar restored ----
//
// Required seams in `explorer.rs`:
//   pub(crate) fn should_clear_hidden_marker(restore_results: &[bool]) -> bool
//       // any false => false; all true => true; empty => true (nothing owned,
//       // a stale marker is harmless to clear).
//   pub(crate) fn finish_taskbar_restore(restore_results: &[bool])
//       // clears the marker (via clear_hidden_marker) iff should_clear_hidden_marker.
//   `restore_taskbar` (single snapshot) and `emergency_restore_taskbar` must NOT clear it.

#[test]
fn marker_clears_only_when_every_restore_succeeded() {
    assert!(should_clear_hidden_marker(&[true]));
    assert!(should_clear_hidden_marker(&[true, true, true]));
    assert!(!should_clear_hidden_marker(&[true, false]));
    assert!(!should_clear_hidden_marker(&[false, true]));
    assert!(!should_clear_hidden_marker(&[false]));
}

#[test]
fn marker_with_no_owned_taskbars_may_be_cleared() {
    assert!(should_clear_hidden_marker(&[]));
}
