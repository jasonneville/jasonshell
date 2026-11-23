//! Tier 0 SEC-02 acceptance: Everything SDK DLL candidate policy.
//!
//! Required seam in `everything_ffi.rs` (pure):
//!
//! ```ignore
//! pub(crate) fn everything_dll_candidates(
//!     env_dir: Option<PathBuf>,        // JASONSHELL_EVERYTHING_SDK_DIR
//!     cwd: Option<PathBuf>,            // current_dir()
//!     installed_dir: Option<PathBuf>,  // parent of installed Everything.exe
//!     program_files: Option<PathBuf>,  // %ProgramFiles%
//!     allow_dev_candidates: bool,      // cfg!(debug_assertions) in prod callers
//! ) -> Vec<PathBuf>;
//! ```
//! Order when allowed: env, cwd ancestors (take 4), installed, program files.
//! Release policy (allow_dev_candidates=false): env + cwd candidates excluded,
//! every returned path absolute (relative inputs are dropped).
use super::*;
use std::path::PathBuf;

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

type Inputs = (
    Option<PathBuf>,
    Option<PathBuf>,
    Option<PathBuf>,
    Option<PathBuf>,
);

fn inputs() -> Inputs {
    (
        Some(p(r"D:\evil\sdk")),
        Some(p(r"D:\evil\work\a\b")),
        Some(p(r"C:\Program Files\Everything")),
        Some(p(r"C:\Program Files")),
    )
}

#[test]
fn release_policy_excludes_env_and_cwd_candidates() {
    let (env, cwd, inst, pf) = inputs();
    let c = everything_dll_candidates(env, cwd, inst, pf, false);
    assert!(!c.is_empty());
    for path in &c {
        let s = path.to_string_lossy().to_ascii_lowercase();
        assert!(!s.contains(r"d:\evil"), "{path:?}");
        assert!(!s.contains(".local"), "{path:?}");
    }
}

#[test]
fn release_policy_keeps_installed_and_program_files_absolute_paths() {
    let (env, cwd, inst, pf) = inputs();
    let c = everything_dll_candidates(env, cwd, inst, pf, false);
    assert!(c.iter().all(|path| path.is_absolute()), "{c:?}");
    assert!(c.contains(&p(r"C:\Program Files\Everything\Everything64.dll")));
    assert!(c.contains(&p(r"C:\Program Files\Everything\Everything.dll")));
}

#[test]
fn release_policy_drops_relative_install_dirs() {
    let c = everything_dll_candidates(
        None,
        None,
        Some(p(r"relative\Everything")),
        Some(p(r"relative\Program Files")),
        false,
    );
    assert!(c.iter().all(|path| path.is_absolute()), "{c:?}");
}

#[test]
fn release_policy_with_no_inputs_is_empty() {
    assert!(everything_dll_candidates(None, None, None, None, false).is_empty());
}

#[test]
fn dev_policy_orders_env_then_cwd_ancestors_then_installed() {
    let (env, cwd, inst, pf) = inputs();
    let c = everything_dll_candidates(env, cwd, inst, pf, true);
    let pos = |needle: PathBuf| {
        c.iter()
            .position(|x| *x == needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in {c:?}"))
    };
    let env_i = pos(p(r"D:\evil\sdk\Everything64.dll"));
    let cwd_i = pos(p(r"D:\evil\work\a\b\.local\Everything-SDK\dll\Everything64.dll"));
    let inst_i = pos(p(r"C:\Program Files\Everything\Everything64.dll"));
    assert!(env_i < cwd_i && cwd_i < inst_i, "{c:?}");
}

#[test]
fn dev_policy_limits_cwd_ancestor_walk_to_four_levels() {
    let c = everything_dll_candidates(None, Some(p(r"D:\a\b\c\d\e\f")), None, None, true);
    let cwd_hits = c
        .iter()
        .filter(|x| x.to_string_lossy().contains(".local"))
        .count();
    // 4 ancestors x {Everything64.dll, Everything.dll}
    assert_eq!(cwd_hits, 8, "{c:?}");
}
