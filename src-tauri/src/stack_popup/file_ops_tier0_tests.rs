//! Tier 0 SEC-01 acceptance: "Open Terminal Here" must never place the
//! directory in a shell command line that a shell parses as script text.
//!
//! Required seam (in `file_ops.rs`, pure, not cfg(windows)-gated):
//!
//! ```ignore
//! #[derive(Clone, Debug, PartialEq, Eq)]
//! pub(crate) struct TerminalLaunchPlan {
//!     pub program: PathBuf,
//!     pub args: Vec<OsString>,
//!     pub current_dir: PathBuf,
//! }
//! /// Order: wt (only when `wt` is Some), powershell, cmd.
//! pub(crate) fn terminal_launch_plans(
//!     directory: &Path,
//!     system32: &Path,
//!     wt: Option<PathBuf>,
//! ) -> Vec<TerminalLaunchPlan>;
//! ```
use super::*;
use std::ffi::OsString;
use std::path::PathBuf;

const HOSTILE_NAMES: &[&str] = &[
    "x;calc",
    "x$(calc)",
    "a&calc",
    "100%path%",
    "a^b",
    "quote\"d",
    "sp ace;calc",
    "a|b",
];

fn system32() -> PathBuf {
    PathBuf::from(r"C:\Windows\System32")
}

fn wt_path() -> PathBuf {
    PathBuf::from(r"C:\Users\u\AppData\Local\Microsoft\WindowsApps\wt.exe")
}

fn file_name_of(plan: &TerminalLaunchPlan) -> String {
    plan.program
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_ascii_lowercase()
}

fn lossy(args: &[OsString]) -> Vec<String> {
    args.iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

fn plan_for<'a>(plans: &'a [TerminalLaunchPlan], exe: &str) -> &'a TerminalLaunchPlan {
    plans
        .iter()
        .find(|plan| file_name_of(plan) == exe)
        .unwrap_or_else(|| panic!("no launch plan for {exe}: {plans:?}"))
}

#[test]
fn plans_are_ordered_wt_then_powershell_then_cmd() {
    let dir = PathBuf::from(r"C:\work\proj");
    let plans = terminal_launch_plans(&dir, &system32(), Some(wt_path()));
    let names: Vec<_> = plans.iter().map(file_name_of).collect();
    assert_eq!(names, ["wt.exe", "powershell.exe", "cmd.exe"]);

    let without_wt = terminal_launch_plans(&dir, &system32(), None);
    let names: Vec<_> = without_wt.iter().map(file_name_of).collect();
    assert_eq!(names, ["powershell.exe", "cmd.exe"]);
}

#[test]
fn programs_resolve_under_system32_except_wt() {
    let dir = PathBuf::from(r"C:\work\proj");
    let plans = terminal_launch_plans(&dir, &system32(), Some(wt_path()));
    assert_eq!(plan_for(&plans, "wt.exe").program, wt_path());
    assert!(plan_for(&plans, "powershell.exe")
        .program
        .starts_with(system32()));
    assert!(plan_for(&plans, "cmd.exe").program.starts_with(system32()));
}

#[test]
fn powershell_plan_never_contains_directory_or_script_text() {
    for name in HOSTILE_NAMES {
        let dir = PathBuf::from(format!(r"C:\work\{name}"));
        let plans = terminal_launch_plans(&dir, &system32(), None);
        let ps = plan_for(&plans, "powershell.exe");
        assert_eq!(lossy(&ps.args), ["-NoExit", "-NoLogo"], "dir {name}");
        assert_eq!(ps.current_dir, dir, "dir {name}");
        for arg in lossy(&ps.args) {
            assert!(!arg.contains(name), "arg {arg:?} leaks dir {name}");
            assert!(
                !arg.eq_ignore_ascii_case("-Command"),
                "no script text allowed"
            );
        }
    }
}

#[test]
fn cmd_plan_never_contains_directory() {
    for name in HOSTILE_NAMES {
        let dir = PathBuf::from(format!(r"C:\work\{name}"));
        let plans = terminal_launch_plans(&dir, &system32(), None);
        let cmd = plan_for(&plans, "cmd.exe");
        assert_eq!(lossy(&cmd.args), ["/K"], "dir {name}");
        assert_eq!(cmd.current_dir, dir, "dir {name}");
    }
}

#[test]
fn wt_plan_sets_current_dir_and_escapes_semicolons_in_start_dir() {
    for name in HOSTILE_NAMES {
        let dir = PathBuf::from(format!(r"C:\work\{name}"));
        let plans = terminal_launch_plans(&dir, &system32(), Some(wt_path()));
        let wt = plan_for(&plans, "wt.exe");
        assert_eq!(wt.current_dir, dir, "dir {name}");
        let args = lossy(&wt.args);
        assert_eq!(args.first().map(String::as_str), Some("-d"), "dir {name}");
        // wt treats a bare `;` as a command separator (injects a second
        // subcommand). Every `;` reaching wt must be backslash-escaped.
        let joined = args.join(" ").replace("\\;", "");
        assert!(
            !joined.contains(';'),
            "unescaped ';' reaches wt for {name}: {args:?}"
        );
    }
}

#[test]
fn plain_directory_round_trips_for_every_plan() {
    let dir = PathBuf::from(r"C:\work\plain dir");
    let plans = terminal_launch_plans(&dir, &system32(), Some(wt_path()));
    for plan in &plans {
        assert_eq!(plan.current_dir, dir, "{plan:?}");
    }
    let wt = plan_for(&plans, "wt.exe");
    assert_eq!(wt.args, [OsString::from("-d"), dir.clone().into_os_string()]);
}

// ---- follow-up: cmd.exe refuses a UNC working directory ----
// Contract: for a directory whose path starts with `\\` (UNC or `\\?\UNC\`) the
// cmd plan is OMITTED (wt and powershell plans remain, powershell last).

#[test]
fn unc_directory_omits_cmd_plan_but_keeps_other_plans() {
    for dir in [r"\\server\share\x", r"\\?\UNC\server\share\x"] {
        let dir = PathBuf::from(dir);
        let plans = terminal_launch_plans(&dir, &system32(), Some(wt_path()));
        let names: Vec<_> = plans.iter().map(file_name_of).collect();
        assert_eq!(names, ["wt.exe", "powershell.exe"], "{dir:?}");
        for plan in &plans {
            assert_eq!(plan.current_dir, dir);
        }
        let without_wt = terminal_launch_plans(&dir, &system32(), None);
        let names: Vec<_> = without_wt.iter().map(file_name_of).collect();
        assert_eq!(names, ["powershell.exe"], "{dir:?}");
    }
}

#[test]
fn drive_letter_directory_keeps_cmd_plan() {
    let plans = terminal_launch_plans(Path::new(r"D:\share\x"), &system32(), None);
    assert!(plans.iter().any(|p| file_name_of(p) == "cmd.exe"));
}
