//! Tier 0 SEC-03 + RS-14 acceptance for `stack_popup/git_status.rs`.
//!
//! Required seams in `git_status.rs`:
//!
//! ```ignore
//! /// Full git argv (WITHOUT the program):
//! /// ["-C", <cwd>, <hardening "-c", "k=v" pairs...>, <args, diff flags injected after "diff">].
//! fn hardened_git_args(mode: GitRunMode, cwd: &Path, args: &[&str]) -> Vec<String>;
//!
//! const GIT_STDOUT_CAP_BYTES: usize; // >= 8 MiB, used as stdout_cap in run_git_command
//!
//! /// Err (message contains "too many changes") when output.stdout_truncated.
//! fn ensure_stdout_complete(output: ProcessRunOutput) -> Result<ProcessRunOutput, String>;
//! ```
use super::*;
use crate::stack_popup::process_runner::ProcessRunOutput;
use std::path::Path;

const ALL_MODES: [GitRunMode; 4] = [
    GitRunMode::OptionalProbe,
    GitRunMode::Read,
    GitRunMode::LocalMutation,
    GitRunMode::Remote,
];

fn has_config(args: &[String], pair: &str) -> bool {
    args.windows(2).any(|w| w[0] == "-c" && w[1] == pair)
}

/// Index just past the last `-c k=v` pair that follows `-C <cwd>`.
fn config_end(args: &[String]) -> usize {
    let mut i = 2;
    while i + 1 < args.len() && args[i] == "-c" {
        i += 2;
    }
    i
}

#[test]
fn every_mode_disables_fsmonitor_and_ext_protocol() {
    for mode in ALL_MODES {
        let args = hardened_git_args(mode, Path::new(r"C:\repo"), &["status", "--porcelain=v2"]);
        assert_eq!(args[0], "-C");
        assert_eq!(args[1], r"C:\repo");
        assert!(has_config(&args, "core.fsmonitor=false"), "{mode:?}: {args:?}");
        assert!(
            has_config(&args, "protocol.ext.allow=never"),
            "{mode:?}: {args:?}"
        );
    }
}

#[test]
fn read_modes_disable_untracked_cache_external_diff_and_pager() {
    for mode in [GitRunMode::OptionalProbe, GitRunMode::Read] {
        let args = hardened_git_args(mode, Path::new(r"C:\repo"), &["status"]);
        for pair in [
            "core.untrackedCache=false",
            "diff.external=",
            "core.pager=cat",
            "core.fsmonitor=false",
            "protocol.ext.allow=never",
        ] {
            assert!(has_config(&args, pair), "{mode:?} missing {pair}: {args:?}");
        }
    }
}

#[test]
fn hardening_precedes_the_subcommand_and_user_args_are_preserved() {
    for mode in ALL_MODES {
        let user = ["log", "--format=%H", "-n", "5"];
        let args = hardened_git_args(mode, Path::new(r"C:\repo"), &user);
        let end = config_end(&args);
        assert_eq!(&args[end..], user, "{mode:?}: {args:?}");
    }
}

#[test]
fn diff_invocations_pass_no_ext_diff_and_no_textconv() {
    for mode in ALL_MODES {
        let args = hardened_git_args(mode, Path::new(r"C:\repo"), &["diff", "--stat", "HEAD"]);
        let end = config_end(&args);
        let tail = &args[end..];
        assert_eq!(tail[0], "diff", "{mode:?}: {args:?}");
        assert!(
            tail.contains(&"--no-ext-diff".to_string()),
            "{mode:?}: {args:?}"
        );
        assert!(
            tail.contains(&"--no-textconv".to_string()),
            "{mode:?}: {args:?}"
        );
        let stripped: Vec<_> = tail
            .iter()
            .filter(|a| *a != "--no-ext-diff" && *a != "--no-textconv")
            .cloned()
            .collect();
        assert_eq!(stripped, ["diff", "--stat", "HEAD"]);
    }
}

#[test]
fn non_diff_commands_do_not_gain_diff_flags() {
    let args = hardened_git_args(GitRunMode::Read, Path::new(r"C:\repo"), &["status"]);
    assert!(!args.contains(&"--no-ext-diff".to_string()));
}

#[test]
fn user_initiated_mutations_keep_hooks() {
    for (mode, sub) in [
        (GitRunMode::LocalMutation, "commit"),
        (GitRunMode::Remote, "push"),
        (GitRunMode::Remote, "fetch"),
        (GitRunMode::Remote, "pull"),
    ] {
        let args = hardened_git_args(mode, Path::new(r"C:\repo"), &[sub]);
        assert!(
            !args
                .iter()
                .any(|a| a.to_ascii_lowercase().contains("hookspath")),
            "{sub} must keep hooks: {args:?}"
        );
    }
}

#[test]
fn hostile_cwd_stays_a_single_argument() {
    let cwd = r"C:\repo\x;calc & $(calc)";
    let args = hardened_git_args(GitRunMode::Read, Path::new(cwd), &["status"]);
    assert_eq!(args[0], "-C");
    assert_eq!(args[1], cwd);
}

#[cfg(windows)]
fn output(stdout: Vec<u8>, truncated: bool) -> ProcessRunOutput {
    use std::os::windows::process::ExitStatusExt;
    ProcessRunOutput {
        status: std::process::ExitStatus::from_raw(0),
        stdout_total_bytes: stdout.len() as u64,
        stdout,
        stderr: Vec::new(),
        stderr_total_bytes: 0,
        stdout_truncated: truncated,
        stderr_truncated: false,
    }
}

#[test]
fn status_stdout_cap_is_at_least_8_mib() {
    assert!(
        GIT_STDOUT_CAP_BYTES >= 8 * 1024 * 1024,
        "cap {GIT_STDOUT_CAP_BYTES}"
    );
}

#[cfg(windows)]
#[test]
fn truncated_stdout_is_an_explicit_error_never_parsed() {
    let err = ensure_stdout_complete(output(b"M a.txt\0".to_vec(), true)).unwrap_err();
    assert!(err.to_ascii_lowercase().contains("too many changes"), "{err}");
}

#[cfg(windows)]
#[test]
fn complete_stdout_passes_through_unchanged() {
    let ok = ensure_stdout_complete(output(b"M a.txt\0".to_vec(), false)).unwrap();
    assert_eq!(ok.stdout, b"M a.txt\0");
}

// ---- follow-up: show / log -p / stash show must not run external diff or textconv ----

fn assert_diff_hardened(mode: GitRunMode, user: &[&str]) {
    let args = hardened_git_args(mode, Path::new(r"C:\repo"), user);
    let end = config_end(&args);
    let tail = &args[end..];
    assert!(tail.contains(&"--no-ext-diff".to_string()), "{user:?}: {args:?}");
    assert!(tail.contains(&"--no-textconv".to_string()), "{user:?}: {args:?}");
    let stripped: Vec<_> = tail
        .iter()
        .filter(|a| *a != "--no-ext-diff" && *a != "--no-textconv")
        .cloned()
        .collect();
    assert_eq!(stripped, user, "original args must survive in order");
    assert_eq!(tail[0], user[0], "flags must follow the subcommand: {args:?}");
}

#[test]
fn show_gets_no_ext_diff_and_no_textconv() {
    for mode in ALL_MODES {
        assert_diff_hardened(mode, &["show", "--format=", "HEAD:src/a.rs"]);
        assert_diff_hardened(mode, &["show", "abc123"]);
    }
}

#[test]
fn log_with_patch_flags_gets_no_ext_diff_and_no_textconv() {
    assert_diff_hardened(GitRunMode::Read, &["log", "-p", "-n", "1"]);
    assert_diff_hardened(GitRunMode::Read, &["log", "--patch", "-n", "1"]);
}

#[test]
fn stash_show_gets_no_ext_diff_and_no_textconv_but_stash_push_does_not() {
    assert_diff_hardened(GitRunMode::LocalMutation, &["stash", "show", "-p", "stash@{0}"]);
    let args = hardened_git_args(
        GitRunMode::LocalMutation,
        Path::new(r"C:\repo"),
        &["stash", "push", "-m", "x"],
    );
    assert!(!args.contains(&"--no-ext-diff".to_string()), "{args:?}");
}

#[test]
fn plain_log_without_patch_is_unchanged() {
    let args = hardened_git_args(GitRunMode::Read, Path::new(r"C:\repo"), &["log", "--format=%H"]);
    assert_eq!(&args[config_end(&args)..], ["log", "--format=%H"]);
}

// ---- follow-up: trusted git discovery includes per-user, x86 and scoop installs ----
//
// Required seam in git_status.rs (pure; `git_candidates()` becomes a thin wrapper
// passing `|k| std::env::var_os(k)`):
//   fn git_candidate_paths(env: impl Fn(&str) -> Option<std::ffi::OsString>) -> Vec<PathBuf>

fn fake_env(key: &str) -> Option<std::ffi::OsString> {
    let value = match key {
        "SystemRoot" => r"C:\Windows",
        "ProgramFiles" => r"C:\Program Files",
        "ProgramFiles(x86)" => r"C:\Program Files (x86)",
        "LOCALAPPDATA" => r"C:\Users\u\AppData\Local",
        "USERPROFILE" => r"C:\Users\u",
        _ => return None,
    };
    Some(value.into())
}

#[test]
fn git_candidates_include_all_trusted_install_locations() {
    let c = git_candidate_paths(fake_env);
    for expected in [
        r"C:\Program Files\Git\cmd\git.exe",
        r"C:\Program Files (x86)\Git\cmd\git.exe",
        r"C:\Users\u\AppData\Local\Programs\Git\cmd\git.exe",
        r"C:\Users\u\scoop\apps\git\current\cmd\git.exe",
    ] {
        assert!(
            c.contains(&std::path::PathBuf::from(expected)),
            "missing {expected}: {c:?}"
        );
    }
}

#[test]
fn git_candidates_are_absolute_and_drop_relative_or_missing_env() {
    assert!(git_candidate_paths(fake_env).iter().all(|p| p.is_absolute()));
    let hostile = |key: &str| -> Option<std::ffi::OsString> {
        match key {
            "ProgramFiles" | "ProgramFiles(x86)" | "LOCALAPPDATA" | "USERPROFILE" | "SystemRoot" => {
                Some(r"relative\dir".into())
            }
            _ => None,
        }
    };
    assert!(
        git_candidate_paths(hostile).iter().all(|p| p.is_absolute()),
        "relative env values must not yield cwd-relative git"
    );
    assert!(git_candidate_paths(|_| None).iter().all(|p| p.is_absolute()));
}
