// Included inside git_status's cfg(test)-only refresh acceptance module.
use super::*;

thread_local! {
    static COMMANDS: std::cell::RefCell<Vec<Vec<String>>> = const { std::cell::RefCell::new(Vec::new()) };
    static UNTRACKED_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static PROBE_FAILURE: std::cell::RefCell<Option<(String, bool)>> = const { std::cell::RefCell::new(None) };
}
pub(super) fn record_command(args: &[&str]) -> Result<(), GitCommandError> {
    COMMANDS.with(|commands| commands.borrow_mut().push(args.iter().map(|arg| (*arg).to_owned()).collect()));
    PROBE_FAILURE.with(|failure| {
        let mut failure = failure.borrow_mut();
        if failure.as_ref().is_some_and(|(command, _)| args.first().copied() == Some(command.as_str())) {
            let (command, timeout) = failure.take().unwrap();
            let message = format!("fixture metadata {command} {}", if timeout { "timeout" } else { "probe failed" });
            return Err(if timeout { GitCommandError::Timeout(message) } else { GitCommandError::NonZero(message) });
        }
        Ok(())
    })
}
pub(super) fn record_untracked_read() { UNTRACKED_READS.with(|reads| reads.set(reads.get() + 1)); }
fn reset_work() { COMMANDS.with(|commands| commands.borrow_mut().clear()); UNTRACKED_READS.with(|reads| reads.set(0)); PROBE_FAILURE.with(|failure| *failure.borrow_mut() = None); }
fn commands() -> Vec<Vec<String>> { COMMANDS.with(|commands| commands.borrow().clone()) }

struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
            .join("Temp").join("opencode").join(format!("stack-git-refresh-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn repo(&self, name: &str) -> std::path::PathBuf {
        let path = self.0.join(name); std::fs::create_dir_all(&path).unwrap();
        git(&path, &["init", "-b", "main"]);
        git(&path, &["config", "user.email", "fixture@example.invalid"]);
        git(&path, &["config", "user.name", "Fixture"]);
        std::fs::write(path.join("tracked.txt"), "before\n").unwrap();
        git(&path, &["add", "tracked.txt"]); git(&path, &["commit", "-m", "fixture"]);
        path
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
fn git(path: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git").arg("-C").arg(path).args(args)
        .env("GIT_TERMINAL_PROMPT", "0").output().unwrap();
    assert!(output.status.success(), "fixture git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}
fn canonical(path: &std::path::Path) -> String { std::fs::canonicalize(path).unwrap().to_string_lossy().into_owned() }

#[test]
fn context_resolves_children_nested_repositories_and_nonrepositories() {
    let fixture = Fixture::new(); let repo = fixture.repo("outer");
    let child = repo.join("src/deep"); std::fs::create_dir_all(&child).unwrap();
    let nested = repo.join("nested"); std::fs::create_dir_all(&nested).unwrap(); git(&nested, &["init", "-b", "nested"]);
    let root = stack_git_context_for_path(repo.to_str().unwrap()).unwrap().unwrap();
    let inner = stack_git_context_for_path(child.to_str().unwrap()).unwrap().unwrap();
    assert_eq!(root.repository_root, canonical(&repo)); assert_eq!(root.git_dir, inner.git_dir);
    assert_eq!(inner.canonical_path, canonical(&child));
    let other = stack_git_context_for_path(nested.to_str().unwrap()).unwrap().unwrap();
    assert_ne!(root.repository_root, other.repository_root); assert_ne!(root.git_dir, other.git_dir);
    assert!(stack_git_context_for_path(fixture.0.to_str().unwrap()).unwrap().is_none());
    assert!(stack_git_context_for_path(fixture.0.join("missing").to_str().unwrap()).unwrap().is_none());
}

#[test]
fn linked_worktree_and_submodule_have_distinct_verified_git_identity() {
    let fixture = Fixture::new(); let repo = fixture.repo("outer"); let source = fixture.repo("source");
    let linked = fixture.0.join("linked");
    git(&repo, &["worktree", "add", "-b", "linked", linked.to_str().unwrap()]);
    git(&repo, &["-c", "protocol.file.allow=always", "submodule", "add", source.to_str().unwrap(), "module"]);
    let root = stack_git_context_for_path(repo.to_str().unwrap()).unwrap().unwrap();
    for path in [&linked, &repo.join("module")] {
        let context = stack_git_context_for_path(path.to_str().unwrap()).unwrap().unwrap();
        assert_eq!(context.repository_root, canonical(path)); assert_ne!(context.git_dir, root.git_dir);
    }
}

#[cfg(windows)]
#[test]
fn junction_into_another_repository_uses_target_not_lexical_parent() {
    let fixture = Fixture::new(); let repo = fixture.repo("outer"); let other = fixture.repo("other");
    let junction = repo.join("escape");
    // cmd's mklink requires native separators, unlike Git and std::fs.
    let junction_argument = junction.to_string_lossy().replace('/', "\\");
    let target_argument = other.to_string_lossy().replace('/', "\\");
    let output = std::process::Command::new("cmd.exe").args(["/d", "/c", "mklink", "/J"])
        .arg(&junction_argument).arg(&target_argument).output().unwrap();
    assert!(output.status.success(), "fixture junction: {}", String::from_utf8_lossy(&output.stderr));
    let result = stack_git_context_for_path(junction.to_str().unwrap()).unwrap().unwrap();
    assert_eq!(result.repository_root, canonical(&other)); assert_eq!(result.canonical_path, canonical(&other));
    // Remove the junction entry before RAII cleanup; never recurse through it.
    std::fs::remove_dir(junction).unwrap();
}

#[test]
fn summary_keeps_porcelain_badges_but_not_line_stats_or_remote_enrichment() {
    let fixture = Fixture::new(); let repo = fixture.repo("outer");
    std::fs::write(repo.join("tracked.txt"), "after\nmore\n").unwrap();
    std::fs::write(repo.join("untracked.txt"), "one\ntwo\nthree\n").unwrap();
    git(&repo, &["config", "remote.origin.url", "https://github.com/fixture/repo.git"]);
    reset_work();
    let summary = stack_git_summary_for_path(repo.to_str().unwrap()).unwrap().unwrap();
    assert_eq!(summary.branch, "main"); assert_eq!(summary.modified, 1); assert_eq!(summary.untracked, 1);
    assert!(summary.remote_repository_url.is_none()); assert!(summary.ahead.is_none()); assert!(summary.behind.is_none());
    assert!(summary.entries.iter().all(|entry| entry.staged_additions.unwrap_or(0) == 0 && entry.staged_deletions.unwrap_or(0) == 0 && entry.unstaged_additions.unwrap_or(0) == 0 && entry.unstaged_deletions.unwrap_or(0) == 0));
    let summary_commands = commands();
    assert_eq!(summary_commands.iter().filter(|args| args.first().map(String::as_str) == Some("status")).count(), 1, "summary must perform exactly one status scan: {summary_commands:?}");
    assert!(!summary_commands.iter().flatten().any(|arg| matches!(arg.as_str(), "diff" | "--numstat" | "config" | "rev-list")), "summary performed enrichment: {summary_commands:?}");
    UNTRACKED_READS.with(|reads| assert_eq!(reads.get(), 0, "summary read untracked file contents"));
    reset_work();
    let full = stack_git_status_for_path(repo.to_str().unwrap()).unwrap().unwrap();
    assert!(full.entries.iter().any(|entry| entry.unstaged_additions.unwrap_or(0) > 0));
    assert!(full.remote_repository_url.is_some());
    assert!(commands().iter().any(|args| args.first().map(String::as_str) == Some("diff")));
    UNTRACKED_READS.with(|reads| assert!(reads.get() > 0, "full workbench retained untracked enrichment"));
    reset_work();
    let metadata = stack_git_metadata_for_path(repo.to_str().unwrap()).unwrap().unwrap();
    assert_eq!(metadata.remote_repository_url, full.remote_repository_url);
    assert!(!commands().iter().flatten().any(|arg| matches!(arg.as_str(), "status" | "diff" | "--numstat")));
}

#[test]
fn metadata_absent_remote_and_upstream_are_legitimate_nullable_values() {
    let fixture = Fixture::new(); let repo = fixture.repo("no-metadata");
    let metadata = stack_git_metadata_for_path(repo.to_str().unwrap()).expect("absence is not a probe failure").unwrap();
    assert!(metadata.remote_repository_url.is_none()); assert!(metadata.ahead.is_none()); assert!(metadata.behind.is_none());
}

fn assert_metadata_probe_errors_propagate(command: &str, timeout: bool) {
    let fixture = Fixture::new(); let repo = fixture.repo("probe-errors");
    git(&repo, &["config", "remote.origin.url", "https://github.com/fixture/repo.git"]);
    // A real valid local upstream ensures rev-list is required, even if production
    // first checks whether an upstream exists before querying its counts.
    git(&repo, &["config", "branch.main.remote", "."]);
    git(&repo, &["config", "branch.main.merge", "refs/heads/main"]);
    reset_work();
    PROBE_FAILURE.with(|failure| *failure.borrow_mut() = Some((command.to_owned(), timeout)));
    let result = stack_git_metadata_for_path(repo.to_str().unwrap());
    assert!(PROBE_FAILURE.with(|failure| failure.borrow().is_none()), "test must reach the selected production probe");
    let error = result.expect_err("operational metadata probe failure must not become a successful null response");
    assert!(error.contains(&format!("fixture metadata {command}")), "probe diagnostic must survive: {error}");
}

#[test]
fn metadata_remote_config_failed_probe_propagates_error() { assert_metadata_probe_errors_propagate("config", false); }

#[test]
fn metadata_remote_config_timed_out_probe_propagates_error() { assert_metadata_probe_errors_propagate("config", true); }

#[test]
fn metadata_upstream_failed_probe_propagates_error() { assert_metadata_probe_errors_propagate("rev-list", false); }

#[test]
fn metadata_upstream_timed_out_probe_propagates_error() { assert_metadata_probe_errors_propagate("rev-list", true); }

#[test]
#[ignore = "opt-in read-only checkout measurement; select JASONSHELL_GIT_BENCH_LANE and GIT_TRACE2_EVENT, run alone"]
fn measure_split_checkout_lane() {
    let lane = std::env::var("JASONSHELL_GIT_BENCH_LANE").expect("choose context, summary, initial, metadata or full");
    let checkout = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let path = checkout.to_str().unwrap();
    for sample in 0..6 {
        reset_work();
        let started = std::time::Instant::now();
        let entries = match lane.as_str() {
            "context" => { stack_git_context_for_path(path).expect("context").expect("repository"); 0 },
            "summary" => stack_git_summary_for_path(path).expect("summary").expect("repository").entries.len(),
            "initial" => {
                stack_git_context_for_path(path).expect("initial context").expect("repository");
                stack_git_summary_for_path(path).expect("initial summary").expect("repository").entries.len()
            },
            "metadata" => { stack_git_metadata_for_path(path).expect("metadata").expect("repository"); 0 },
            "full" => stack_git_status_for_path(path).expect("full status").expect("repository").entries.len(),
            _ => panic!("unsupported benchmark lane"),
        };
        let elapsed_us = started.elapsed().as_micros();
        let direct = commands();
        let untracked_reads = UNTRACKED_READS.with(|reads| reads.get());
        eprintln!("STACK_GIT_SPLIT lane={lane} sample={sample} elapsed_us={elapsed_us} direct={} untracked_reads={untracked_reads} entries={entries}", direct.len());
        eprintln!("STACK_GIT_SPLIT_COMMANDS lane={lane} sample={sample} commands={direct:?}");
    }
}
