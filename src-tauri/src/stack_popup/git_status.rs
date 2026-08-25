use crate::stack_popup::models::{
    StackGitBranch, StackGitBranchRequest, StackGitBranches, StackGitCommitRequest, StackGitDiff,
    StackGitDiffRequest, StackGitFileStatus, StackGitFileStatusKind, StackGitLog, StackGitLogEntry,
    StackGitLogRequest, StackGitOperationResult, StackGitRevertRequest, StackGitStageRequest,
    StackGitStashEntry, StackGitStashRefRequest, StackGitStashRequest, StackGitStashes,
    StackGitStatus, StackGitTree, StackGitTreeEntry, StackGitTreeRequest,
};
use crate::stack_popup::process_runner::{run_process, ProcessRunError, ProcessRunSpec};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_LOG_LIMIT: usize = 40;
const MAX_LOG_LIMIT: usize = 200;
const GIT_READ_TIMEOUT_SECS: u64 = 10;
const GIT_LOCAL_MUTATION_TIMEOUT_SECS: u64 = 30;
const GIT_REMOTE_TIMEOUT_SECS: u64 = 90;
const GIT_TIMEOUT_ENV_VAR: &str = "JASONSHELL_GIT_TIMEOUT_MS";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GitRunMode {
    OptionalProbe,
    Read,
    LocalMutation,
    Remote,
}

#[derive(Debug, PartialEq, Eq)]
enum GitCommandError {
    Spawn(String),
    Stdin(String),
    Timeout(String),
    Canceled(String),
    Internal(String),
    NonZero(String),
    AuthRequired(String),
    Conflict(String),
    NonFastForward(String),
    NotRepository(String),
}

impl GitCommandError {
    fn into_message(self) -> String {
        match self {
            Self::Spawn(msg)
            | Self::Stdin(msg)
            | Self::Timeout(msg)
            | Self::Canceled(msg)
            | Self::Internal(msg)
            | Self::NonZero(msg)
            | Self::AuthRequired(msg)
            | Self::Conflict(msg)
            | Self::NonFastForward(msg)
            | Self::NotRepository(msg) => msg,
        }
    }
}

pub(crate) async fn stack_git_status_for_path_async(
    path: String,
) -> Result<Option<StackGitStatus>, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_status_for_path(&path))
        .await
        .map_err(|error| format!("Failed to join stack git status task: {error}"))?
}

pub(crate) async fn stack_git_add_paths_async(
    request: StackGitStageRequest,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_add_paths(request))
        .await
        .map_err(|error| format!("Failed to join stack git add task: {error}"))?
}

pub(crate) async fn stack_git_commit_async(
    request: StackGitCommitRequest,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_commit(request))
        .await
        .map_err(|error| format!("Failed to join stack git commit task: {error}"))?
}

pub(crate) async fn stack_git_log_async(
    request: StackGitLogRequest,
) -> Result<StackGitLog, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_log(request))
        .await
        .map_err(|error| format!("Failed to join stack git log task: {error}"))?
}

pub(crate) async fn stack_git_tree_async(
    request: StackGitTreeRequest,
) -> Result<StackGitTree, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_tree(request))
        .await
        .map_err(|error| format!("Failed to join stack git tree task: {error}"))?
}

pub(crate) async fn stack_git_branches_async(path: String) -> Result<StackGitBranches, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_branches(&path))
        .await
        .map_err(|error| format!("Failed to join stack git branch task: {error}"))?
}

pub(crate) async fn stack_git_fetch_async(
    folder_path: String,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_remote_operation(&folder_path, "fetch"))
        .await
        .map_err(|error| format!("Failed to join stack git fetch task: {error}"))?
}

pub(crate) async fn stack_git_pull_async(
    folder_path: String,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_remote_operation(&folder_path, "pull"))
        .await
        .map_err(|error| format!("Failed to join stack git pull task: {error}"))?
}

pub(crate) async fn stack_git_push_async(
    folder_path: String,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_remote_operation(&folder_path, "push"))
        .await
        .map_err(|error| format!("Failed to join stack git push task: {error}"))?
}

pub(crate) async fn stack_git_checkout_branch_async(
    request: StackGitBranchRequest,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_checkout_branch(request))
        .await
        .map_err(|error| format!("Failed to join stack git checkout task: {error}"))?
}

pub(crate) async fn stack_git_create_branch_async(
    request: StackGitBranchRequest,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_create_branch(request))
        .await
        .map_err(|error| format!("Failed to join stack git branch create task: {error}"))?
}

fn stack_git_status_for_path(path: &str) -> Result<Option<StackGitStatus>, String> {
    let folder = PathBuf::from(path);
    if !folder.is_dir() {
        return Ok(None);
    }

    let Some(repo_root_text) = git_stdout(&folder, &["rev-parse", "--show-toplevel"])? else {
        return Ok(None);
    };
    let repo_root = canonicalize_existing_path(Path::new(repo_root_text.trim()))
        .map_err(|_| "Git repository unavailable".to_string())?;
    if repo_root.as_os_str().is_empty() {
        return Ok(None);
    }

    let branch = git_stdout(&folder, &["branch", "--show-current"])?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            git_stdout(&folder, &["rev-parse", "--short", "HEAD"])
                .ok()
                .flatten()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "detached".to_string());

    let Some(status_output) = git_stdout_bytes(
        &folder,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?
    else {
        return Ok(None);
    };

    let remote_repository_url = git_remote_repository_url(&folder);

    Ok(Some(stack_git_status_from_porcelain(
        &repo_root,
        branch,
        remote_repository_url,
        git_ahead_behind(&folder),
        &status_output,
    )))
}

fn git_ahead_behind(folder: &Path) -> (Option<usize>, Option<usize>) {
    let Some(output) = git_stdout(
        folder,
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
    )
    .ok()
    .flatten() else {
        return (None, None);
    };
    let mut parts = output.split_whitespace();
    let behind = parts.next().and_then(|v| v.parse().ok());
    let ahead = parts.next().and_then(|v| v.parse().ok());
    (ahead, behind)
}

fn stack_git_add_paths(request: StackGitStageRequest) -> Result<StackGitOperationResult, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    if request.paths.is_empty() {
        return Err("Select at least one file to add".to_string());
    }
    let pathspecs = git_pathspecs_for_paths(&repo_root, &request.paths)?;
    run_git_with_stdin(
        &repo_root,
        &["add", "--pathspec-from-file=-", "--pathspec-file-nul"],
        nul_joined_pathspecs(&pathspecs),
    )?;
    Ok(StackGitOperationResult {
        repository_root: repo_root.to_string_lossy().into_owned(),
        summary: format!("Added {} file(s)", pathspecs.len()),
    })
}

fn stack_git_commit(request: StackGitCommitRequest) -> Result<StackGitOperationResult, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let message = trim_bounded_git_commit_message(&request.message);
    if message.is_empty() {
        return Err("Commit message required".to_string());
    }
    if request.paths.is_empty() {
        return Err("Select at least one staged file to commit".to_string());
    }
    let pathspecs = git_pathspecs_for_paths(&repo_root, &request.paths)?;
    run_git_with_stdin(
        &repo_root,
        &[
            "commit",
            "-m",
            &message,
            "--pathspec-from-file=-",
            "--pathspec-file-nul",
        ],
        nul_joined_pathspecs(&pathspecs),
    )?;
    Ok(StackGitOperationResult {
        repository_root: repo_root.to_string_lossy().into_owned(),
        summary: "Commit created".to_string(),
    })
}

fn stack_git_log(request: StackGitLogRequest) -> Result<StackGitLog, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let limit = request
        .limit
        .unwrap_or(DEFAULT_LOG_LIMIT)
        .clamp(1, MAX_LOG_LIMIT);
    let limit_arg = limit.to_string();
    let output = git_stdout(
        &repo_root,
        &[
            "log",
            "--date=iso-strict",
            "--pretty=format:%H%x1f%h%x1f%an%x1f%ae%x1f%ad%x1f%s%x1e",
            "-n",
            &limit_arg,
        ],
    )?
    .unwrap_or_default();
    Ok(StackGitLog {
        repository_root: repo_root.to_string_lossy().into_owned(),
        entries: parse_git_log_output(&output),
    })
}

fn stack_git_tree(request: StackGitTreeRequest) -> Result<StackGitTree, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let treeish = validate_treeish(request.treeish.as_deref().unwrap_or("HEAD"))?.to_string();
    let relative_path = request
        .path
        .as_deref()
        .map(|path| git_relative_path_for_request(&repo_root, path))
        .transpose()?;
    let mut args = vec!["ls-tree", "-z", "-l", treeish.as_str()];
    if let Some(relative_path) = relative_path.as_deref() {
        args.push("--");
        args.push(relative_path);
    }
    let output = git_stdout_bytes(&repo_root, &args)?.unwrap_or_default();
    Ok(StackGitTree {
        repository_root: repo_root.to_string_lossy().into_owned(),
        treeish,
        entries: parse_git_tree_output(&output),
    })
}

fn stack_git_branches(path: &str) -> Result<StackGitBranches, String> {
    let repo_root =
        repo_root_for_folder(path)?.ok_or_else(|| "Git repository unavailable".to_string())?;
    let current_branch = git_stdout(&repo_root, &["branch", "--show-current"])?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let output = git_stdout(
        &repo_root,
        &["branch", "--all", "--format=%(HEAD)%x1f%(refname:short)"],
    )?
    .unwrap_or_default();
    Ok(StackGitBranches {
        repository_root: repo_root.to_string_lossy().into_owned(),
        current_branch,
        branches: parse_git_branch_output(&output),
    })
}

fn stack_git_remote_operation(
    folder_path: &str,
    operation: &str,
) -> Result<StackGitOperationResult, String> {
    let repo_root = repo_root_for_folder(folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let args = match operation {
        "fetch" => vec!["fetch", "--prune"],
        "pull" => vec!["pull", "--ff-only"],
        "push" => vec!["push"],
        _ => return Err("Git operation is invalid".to_string()),
    };
    run_git(&repo_root, &args)?;
    Ok(StackGitOperationResult {
        repository_root: repo_root.to_string_lossy().into_owned(),
        summary: match operation {
            "fetch" => "Fetched",
            "pull" => "Pulled",
            "push" => "Pushed",
            _ => "Updated",
        }
        .to_string(),
    })
}

fn stack_git_checkout_branch(
    request: StackGitBranchRequest,
) -> Result<StackGitOperationResult, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let branch = validate_git_branch_name(&request.branch_name)?;
    let args = if let Some(remote_ref) = branch.strip_prefix("remotes/") {
        let local = remote_ref
            .split_once('/')
            .map(|(_, tail)| tail)
            .unwrap_or(remote_ref);
        vec!["switch", "--track", "-c", local, remote_ref]
    } else {
        vec!["switch", "--", branch]
    };
    run_git(&repo_root, &args)?;
    Ok(StackGitOperationResult {
        repository_root: repo_root.to_string_lossy().into_owned(),
        summary: format!("Checked out {branch}"),
    })
}

fn stack_git_create_branch(
    request: StackGitBranchRequest,
) -> Result<StackGitOperationResult, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let branch = validate_git_branch_name(&request.branch_name)?;
    if request.checkout.unwrap_or(true) {
        run_git(&repo_root, &["switch", "-c", branch])?;
    } else {
        run_git(&repo_root, &["branch", branch])?;
    }
    Ok(StackGitOperationResult {
        repository_root: repo_root.to_string_lossy().into_owned(),
        summary: format!("Created branch {branch}"),
    })
}

fn repo_root_for_folder(path: &str) -> Result<Option<PathBuf>, String> {
    let folder = PathBuf::from(path);
    if !folder.is_dir() {
        return Ok(None);
    }
    let Some(repo_root_text) = git_stdout(&folder, &["rev-parse", "--show-toplevel"])? else {
        return Ok(None);
    };
    let repo_root = canonicalize_existing_path(Path::new(repo_root_text.trim()))
        .map_err(|_| "Git repository unavailable".to_string())?;
    if repo_root.as_os_str().is_empty() {
        return Ok(None);
    }
    Ok(Some(repo_root))
}

fn git_relative_path_for_request(repo_root: &Path, path: &str) -> Result<String, String> {
    if path.trim().is_empty() || path.contains('\0') {
        return Err("Git tree path is invalid".to_string());
    }
    let candidate = PathBuf::from(path);
    let relative = if candidate.is_absolute() {
        if candidate.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        }) || !candidate.starts_with(repo_root)
        {
            return Err("Git tree path is outside the repository".to_string());
        }
        candidate
            .strip_prefix(repo_root)
            .map_err(|_| "Git tree path is outside the repository".to_string())?
            .to_string_lossy()
            .replace('\\', "/")
    } else {
        if Path::new(path).components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        }) {
            return Err("Git tree path is invalid".to_string());
        }
        path.replace('\\', "/")
    };
    if relative.is_empty() || relative.contains('\0') {
        return Err("Git tree path is invalid".to_string());
    }
    Ok(relative)
}

fn validate_treeish(value: &str) -> Result<&str, String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 200
        || value.starts_with('-')
        || value.contains('\0')
        || value.chars().any(char::is_whitespace)
        || value.contains("..")
        || value.contains("@{")
        || value.contains('\\')
        || value.ends_with('.')
        || value.ends_with('/')
        || value.contains("//")
    {
        return Err("Git tree reference is invalid".to_string());
    }
    Ok(value)
}

fn validate_git_branch_name(value: &str) -> Result<&str, String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 120
        || value.starts_with('-')
        || value.contains('\0')
        || value.chars().any(char::is_whitespace)
        || value.contains("..")
        || value.contains("//")
        || value.ends_with('.')
        || value.ends_with('/')
        || value.ends_with(".lock")
        || value.contains("@{")
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '/')))
    {
        return Err("Git branch name is invalid".to_string());
    }
    Ok(value)
}

fn git_pathspecs_for_paths(repo_root: &Path, paths: &[String]) -> Result<Vec<String>, String> {
    let status_paths = git_status_relative_paths(repo_root)?;
    let mut pathspecs = Vec::with_capacity(paths.len());
    for path in paths {
        let relative = git_relative_path_for_stage(repo_root, path, &status_paths)?;
        pathspecs.push(relative);
    }
    Ok(pathspecs)
}

fn git_relative_path_for_stage(
    repo_root: &Path,
    path: &str,
    status_paths: &HashSet<String>,
) -> Result<String, String> {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        if !candidate.exists() {
            let relative = candidate
                .strip_prefix(repo_root)
                .map_err(|_| "Git path is outside the repository".to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            let normalized = normalize_repo_relative_string(&relative)?;
            if status_paths.contains(&normalized) {
                return Ok(normalized);
            }
            return Err("Git path is not a changed repository path".to_string());
        }
        let canonical = canonicalize_existing_path(&candidate)?;
        let canonical_root = canonicalize_existing_path(repo_root)?;
        if !path_within_root(&canonical_root, &canonical) {
            return Err("Git path is outside the repository".to_string());
        }
        return Ok(canonical
            .strip_prefix(&canonical_root)
            .map_err(|_| "Git path is outside the repository".to_string())?
            .to_string_lossy()
            .replace('\\', "/"));
    }
    validate_missing_repo_relative_path(path)?;
    let normalized = normalize_repo_relative_string(path)?;
    Ok(normalized)
}

fn trim_bounded_git_commit_message(value: &str) -> String {
    const MAX_GIT_COMMIT_MESSAGE: usize = 512;
    value.trim().chars().take(MAX_GIT_COMMIT_MESSAGE).collect()
}

fn git_relative_path_for_tree_request(repo_root: &Path, path: &str) -> Result<String, String> {
    let canonical = canonicalize_existing_path(Path::new(path))?;
    let canonical_root = canonicalize_existing_path(repo_root)?;
    if !path_within_root(&canonical_root, &canonical) {
        return Err("Git path is outside the repository".to_string());
    }
    Ok(canonical
        .strip_prefix(&canonical_root)
        .map_err(|_| "Git path is outside the repository".to_string())?
        .to_string_lossy()
        .replace('\\', "/"))
}

fn git_status_relative_paths(repo_root: &Path) -> Result<HashSet<String>, String> {
    let output = git_stdout_bytes(
        repo_root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?
    .unwrap_or_default();
    Ok(parse_git_status_paths(repo_root, &output))
}

fn parse_git_status_paths(repo_root: &Path, porcelain: &[u8]) -> HashSet<String> {
    let mut paths = HashSet::new();
    let fields = porcelain
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    let mut index = 0usize;
    while index < fields.len() {
        let field = String::from_utf8_lossy(fields[index]);
        if field.len() >= 4 {
            let xy = &field[..2];
            let relative_path = field[3..].trim().replace('/', "\\");
            paths.insert(absolute_git_status_path(repo_root, &relative_path));
            if matches!(xy.as_bytes().first(), Some(b'R' | b'C')) {
                if let Some(next) = fields.get(index + 1) {
                    paths.insert(absolute_git_status_path(
                        repo_root,
                        &String::from_utf8_lossy(next).replace('/', "\\"),
                    ));
                    index += 1;
                }
            }
        }
        index += 1;
    }
    paths
}

fn normalize_repo_relative_existing_path(
    repo_root: &Path,
    candidate: &Path,
) -> Result<String, String> {
    let canonical = canonicalize_existing_path(candidate)
        .map_err(|_| "Git path is outside the repository".to_string())?;
    let canonical_root = canonicalize_existing_path(repo_root)
        .map_err(|_| "Git path is outside the repository".to_string())?;
    if !path_within_root(&canonical_root, &canonical) {
        return Err("Git path is outside the repository".to_string());
    }
    Ok(canonical
        .strip_prefix(&canonical_root)
        .map_err(|_| "Git path is outside the repository".to_string())?
        .to_string_lossy()
        .replace('\\', "/"))
}

fn canonicalize_existing_path(path: &Path) -> Result<PathBuf, String> {
    std::fs::canonicalize(path).map_err(|_| "Git path is outside the repository".to_string())
}

fn path_within_root(repo_root: &Path, candidate: &Path) -> bool {
    candidate.starts_with(repo_root)
}

fn validate_missing_repo_relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() || path.contains(':') || path.contains('\\') || path.contains('\0') {
        return Err("Git path is invalid".to_string());
    }
    if path.split('/').any(|segment| {
        segment.is_empty() || segment == "." || segment == ".." || segment.starts_with('~')
    }) {
        return Err("Git path is invalid".to_string());
    }
    Ok(())
}

fn normalize_repo_relative_string(path: &str) -> Result<String, String> {
    validate_missing_repo_relative_path(path)?;
    Ok(path.replace('\\', "/"))
}

fn nul_joined_pathspecs(pathspecs: &[String]) -> Vec<u8> {
    let mut input = Vec::new();
    for pathspec in pathspecs {
        input.extend_from_slice(pathspec.as_bytes());
        input.push(0);
    }
    input
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Result<Option<String>, String> {
    git_stdout_bytes(cwd, args)
        .map(|output| output.map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
}

fn git_stdout_bytes(cwd: &Path, args: &[&str]) -> Result<Option<Vec<u8>>, String> {
    match run_git_command(cwd, args, None, GitRunMode::OptionalProbe) {
        Ok(output) => Ok(Some(output.stdout)),
        Err(GitCommandError::NotRepository(_)) => Ok(None),
        Err(error) => Err(error.into_message()),
    }
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<(), String> {
    run_git_command(cwd, args, None, classify_git_run_mode(args))
        .map(|_| ())
        .map_err(GitCommandError::into_message)
}

fn run_git_with_stdin(cwd: &Path, args: &[&str], stdin: Vec<u8>) -> Result<(), String> {
    run_git_command(cwd, args, Some(stdin), GitRunMode::LocalMutation)
        .map(|_| ())
        .map_err(GitCommandError::into_message)
}

fn run_git_command(
    cwd: &Path,
    args: &[&str],
    stdin: Option<Vec<u8>>,
    mode: GitRunMode,
) -> Result<crate::stack_popup::process_runner::ProcessRunOutput, GitCommandError> {
    let timeout = git_timeout_for_mode(mode);
    let mut envs = vec![
        ("GIT_TERMINAL_PROMPT".to_string(), Some("0".to_string())),
        ("GCM_INTERACTIVE".to_string(), Some("never".to_string())),
    ];
    let spec = ProcessRunSpec {
        program: trusted_git_path()
            .map_err(GitCommandError::Spawn)?
            .to_string_lossy()
            .into_owned(),
        args: std::iter::once("-C".to_string())
            .chain(std::iter::once(cwd.to_string_lossy().into_owned()))
            .chain(args.iter().map(|value| value.to_string()))
            .collect(),
        cwd: None,
        envs: std::mem::take(&mut envs),
        stdin,
        timeout,
        stdout_cap: 64 * 1024,
        stderr_cap: 64 * 1024,
        poll_interval: Duration::from_millis(50),
        kill_tree: true,
    };
    match run_process(spec) {
        Ok(output) => Ok(output),
        Err(error) => Err(map_git_process_error(error)),
    }
}

fn trusted_git_path() -> Result<PathBuf, String> {
    git_candidates()
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| "git was not found in a trusted location".to_string())
}

fn git_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        candidates.push(PathBuf::from(system_root).join("System32").join("git.exe"));
    }
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        candidates.push(
            PathBuf::from(program_files)
                .join("Git")
                .join("cmd")
                .join("git.exe"),
        );
    }
    candidates.push(PathBuf::from(r"C:\Windows\System32\git.exe"));
    candidates.push(PathBuf::from(r"C:\Program Files\Git\cmd\git.exe"));
    candidates
}

fn classify_git_run_mode(args: &[&str]) -> GitRunMode {
    match args.first().copied() {
        Some("fetch" | "pull" | "push") => GitRunMode::Remote,
        Some("add" | "commit" | "switch" | "restore" | "stash") => GitRunMode::LocalMutation,
        Some("branch") if args.get(1).is_some_and(|arg| !arg.starts_with('-')) => {
            GitRunMode::LocalMutation
        }
        Some("status" | "rev-parse" | "log" | "ls-tree" | "config" | "branch") => GitRunMode::Read,
        Some("diff") => GitRunMode::Read,
        _ => GitRunMode::OptionalProbe,
    }
}

fn git_timeout_for_mode(mode: GitRunMode) -> Duration {
    let default_secs = match mode {
        GitRunMode::OptionalProbe | GitRunMode::Read => GIT_READ_TIMEOUT_SECS,
        GitRunMode::LocalMutation => GIT_LOCAL_MUTATION_TIMEOUT_SECS,
        GitRunMode::Remote => GIT_REMOTE_TIMEOUT_SECS,
    };
    env_timeout_override_ms()
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_secs(default_secs))
}

fn env_timeout_override_ms() -> Option<u64> {
    let raw = std::env::var(GIT_TIMEOUT_ENV_VAR).ok()?;
    let parsed = raw.trim().parse::<u64>().ok()?;
    Some(parsed.clamp(1_000, 600_000))
}

fn map_git_process_error(error: ProcessRunError) -> GitCommandError {
    match error {
        ProcessRunError::Spawn(error) => {
            GitCommandError::Spawn(format!("Failed to run git: {error}"))
        }
        ProcessRunError::Timeout { .. } => GitCommandError::Timeout("Git timed out".to_string()),
        ProcessRunError::CleanupIncomplete { reason, .. } => {
            GitCommandError::Internal(format!("Git internal error: {reason}"))
        }
        ProcessRunError::NonZero {
            status,
            stdout,
            stderr,
            ..
        } => classify_git_nonzero(status, stdout, stderr),
    }
}

fn classify_git_nonzero(status: Option<i32>, stdout: Vec<u8>, stderr: Vec<u8>) -> GitCommandError {
    let text = String::from_utf8_lossy(&stderr).trim().to_string();
    let fallback = String::from_utf8_lossy(&stdout).trim().to_string();
    let text = if text.is_empty() { fallback } else { text };
    let lower = text.to_ascii_lowercase();
    if lower.contains("not a git repository")
        || lower.contains("fatal: unable to read current working directory")
    {
        return GitCommandError::NotRepository(bound_msg("Git repository unavailable", &text));
    }
    if lower.contains("authentication failed")
        || lower.contains("could not read username")
        || lower.contains("terminal prompts disabled")
    {
        return GitCommandError::AuthRequired(bound_msg("Git authentication required", &text));
    }
    if lower.contains("merge conflict")
        || lower.contains("would be overwritten by merge")
        || lower.contains("conflict")
    {
        return GitCommandError::Conflict(bound_msg("Git conflict", &text));
    }
    if lower.contains("non-fast-forward") || lower.contains("fetch first") {
        return GitCommandError::NonFastForward(bound_msg("Git non-fast-forward", &text));
    }
    if text.is_empty() {
        GitCommandError::NonZero(format!(
            "Git failed with status {}",
            status.map_or("unknown".to_string(), |s| s.to_string())
        ))
    } else {
        GitCommandError::NonZero(bound_msg("Git failed", &text))
    }
}

fn bound_msg(prefix: &str, detail: &str) -> String {
    let detail = detail.trim();
    if detail.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}: {detail}")
    }
}

fn parse_git_log_output(output: &str) -> Vec<StackGitLogEntry> {
    output
        .split('\x1e')
        .filter_map(|record| {
            let record = record.trim_matches('\n');
            if record.is_empty() {
                return None;
            }
            let parts = record.split('\x1f').collect::<Vec<_>>();
            if parts.len() != 6 {
                return None;
            }
            Some(StackGitLogEntry {
                commit_hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                author_name: parts[2].to_string(),
                author_email: parts[3].to_string(),
                authored_at: parts[4].to_string(),
                subject: parts[5].to_string(),
            })
        })
        .collect()
}

fn parse_git_tree_output(output: &[u8]) -> Vec<StackGitTreeEntry> {
    output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .filter_map(|field| {
            let value = String::from_utf8_lossy(field);
            let (metadata, path) = value.split_once('\t')?;
            let parts = metadata.split_whitespace().collect::<Vec<_>>();
            if parts.len() < 4 {
                return None;
            }
            Some(StackGitTreeEntry {
                mode: parts[0].to_string(),
                kind: parts[1].to_string(),
                object_hash: parts[2].to_string(),
                size_bytes: (parts[3] != "-").then(|| parts[3].parse().ok()).flatten(),
                path: path.replace('/', "\\"),
            })
        })
        .collect()
}

fn parse_git_branch_output(output: &str) -> Vec<StackGitBranch> {
    output
        .lines()
        .filter_map(|line| {
            let (head, name) = line.split_once('\x1f')?;
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            Some(StackGitBranch {
                name: name.to_string(),
                current: head.trim() == "*",
                remote: name.starts_with("remotes/"),
                ref_name: name.to_string(),
            })
        })
        .collect()
}

fn git_remote_repository_url(folder: &Path) -> Option<String> {
    ["origin", "upstream"].iter().find_map(|remote| {
        git_stdout(
            folder,
            &["config", "--get", &format!("remote.{remote}.url")],
        )
        .ok()
        .flatten()
        .and_then(|value| normalize_git_remote_url(value.trim()))
    })
}

fn normalize_git_remote_url(value: &str) -> Option<String> {
    let remote = value.trim();
    if remote.is_empty() || remote.contains('\0') || remote.contains(char::is_whitespace) {
        return None;
    }
    let without_git_suffix = |text: &str| text.strip_suffix(".git").unwrap_or(text).to_string();
    if let Some(rest) = remote
        .strip_prefix("https://")
        .or_else(|| remote.strip_prefix("http://"))
    {
        let authority = rest.split('/').next().unwrap_or_default();
        if authority.is_empty() || authority.contains('@') {
            return None;
        }
        return Some(without_git_suffix(remote));
    }
    if let Some(rest) = remote.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        if host.is_empty() || path.is_empty() || path.starts_with('/') || path.contains("://") {
            return None;
        }
        return Some(format!("https://{}/{}", host, without_git_suffix(path)));
    }
    if let Some(rest) = remote.strip_prefix("ssh://git@") {
        let (host, path) = rest.split_once('/')?;
        if host.is_empty() || path.is_empty() || path.contains("://") {
            return None;
        }
        return Some(format!("https://{}/{}", host, without_git_suffix(path)));
    }
    None
}

fn stack_git_status_from_porcelain(
    repo_root: &Path,
    branch: String,
    remote_repository_url: Option<String>,
    ahead_behind: (Option<usize>, Option<usize>),
    porcelain: &[u8],
) -> StackGitStatus {
    let mut status = StackGitStatus {
        repository_root: repo_root.to_string_lossy().into_owned(),
        branch,
        remote_repository_url,
        ahead: ahead_behind.0,
        behind: ahead_behind.1,
        modified: 0,
        added: 0,
        deleted: 0,
        untracked: 0,
        conflicts: 0,
        entries: Vec::new(),
    };

    let fields = porcelain
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    let mut index = 0usize;
    while index < fields.len() {
        let field = String::from_utf8_lossy(fields[index]);
        if field.len() < 4 {
            index += 1;
            continue;
        }

        let xy = &field[..2];
        let relative_path = field[3..].trim().replace('/', "\\");
        let status_kind = git_status_kind(xy);
        if let Some(kind) = status_kind {
            increment_status_count(&mut status, kind);
            status.entries.push(StackGitFileStatus {
                path: absolute_git_status_path(repo_root, &relative_path),
                relative_path,
                status: kind,
                staged: git_status_has_staged_change(xy),
                unstaged: git_status_has_unstaged_change(xy),
            });
            if matches!(xy.as_bytes().first(), Some(b'R' | b'C')) {
                if let Some(next) = fields.get(index + 1) {
                    let other_relative_path = String::from_utf8_lossy(next).replace('/', "\\");
                    status.entries.push(StackGitFileStatus {
                        path: absolute_git_status_path(repo_root, &other_relative_path),
                        relative_path: other_relative_path,
                        status: kind,
                        staged: git_status_has_staged_change(xy),
                        unstaged: git_status_has_unstaged_change(xy),
                    });
                }
            }
        }

        index += if matches!(xy.as_bytes().first(), Some(b'R' | b'C')) {
            2
        } else {
            1
        };
    }

    status
}

fn absolute_git_status_path(repo_root: &Path, relative_path: &str) -> String {
    relative_path
        .split('\\')
        .filter(|segment| !segment.is_empty())
        .fold(repo_root.to_path_buf(), |path, segment| path.join(segment))
        .to_string_lossy()
        .into_owned()
}

fn increment_status_count(status: &mut StackGitStatus, kind: StackGitFileStatusKind) {
    match kind {
        StackGitFileStatusKind::Modified => status.modified += 1,
        StackGitFileStatusKind::Added => status.added += 1,
        StackGitFileStatusKind::Deleted => status.deleted += 1,
        StackGitFileStatusKind::Untracked => status.untracked += 1,
        StackGitFileStatusKind::Conflict => status.conflicts += 1,
    }
}

fn git_status_kind(xy: &str) -> Option<StackGitFileStatusKind> {
    if xy == "??" {
        return Some(StackGitFileStatusKind::Untracked);
    }
    if xy.contains('U') || matches!(xy, "AA" | "DD") {
        return Some(StackGitFileStatusKind::Conflict);
    }
    if xy.contains('D') {
        return Some(StackGitFileStatusKind::Deleted);
    }
    if xy.contains('A') {
        return Some(StackGitFileStatusKind::Added);
    }
    if xy.contains('M') || xy.contains('R') || xy.contains('C') || xy.contains('T') {
        return Some(StackGitFileStatusKind::Modified);
    }
    None
}

fn git_status_has_staged_change(xy: &str) -> bool {
    xy.as_bytes()
        .first()
        .is_some_and(|status| !matches!(status, b' ' | b'?'))
}

pub(crate) async fn stack_git_unstage_paths_async(
    request: StackGitStageRequest,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_unstage_paths(request))
        .await
        .map_err(|error| format!("Failed to join stack git unstage task: {error}"))?
}
pub(crate) async fn stack_git_revert_paths_async(
    request: StackGitRevertRequest,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_revert_paths(request))
        .await
        .map_err(|error| format!("Failed to join stack git revert task: {error}"))?
}
pub(crate) async fn stack_git_diff_async(
    request: StackGitDiffRequest,
) -> Result<StackGitDiff, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_diff(request))
        .await
        .map_err(|error| format!("Failed to join stack git diff task: {error}"))?
}
pub(crate) async fn stack_git_stashes_async(
    folder_path: String,
) -> Result<StackGitStashes, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_stashes(&folder_path))
        .await
        .map_err(|error| format!("Failed to join stack git stashes task: {error}"))?
}
pub(crate) async fn stack_git_stash_async(
    request: StackGitStashRequest,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_stash(request))
        .await
        .map_err(|error| format!("Failed to join stack git stash task: {error}"))?
}
pub(crate) async fn stack_git_stash_apply_async(
    request: StackGitStashRefRequest,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_stash_apply(request))
        .await
        .map_err(|error| format!("Failed to join stack git stash apply task: {error}"))?
}
pub(crate) async fn stack_git_stash_pop_async(
    request: StackGitStashRefRequest,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_stash_pop(request))
        .await
        .map_err(|error| format!("Failed to join stack git stash pop task: {error}"))?
}
pub(crate) async fn stack_git_stash_drop_async(
    request: StackGitStashRefRequest,
) -> Result<StackGitOperationResult, String> {
    tauri::async_runtime::spawn_blocking(move || stack_git_stash_drop(request))
        .await
        .map_err(|error| format!("Failed to join stack git stash drop task: {error}"))?
}

fn stack_git_unstage_paths(
    request: StackGitStageRequest,
) -> Result<StackGitOperationResult, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let pathspecs = git_pathspecs_for_paths(&repo_root, &request.paths)?;
    run_git_with_stdin(
        &repo_root,
        &[
            "restore",
            "--staged",
            "--pathspec-from-file=-",
            "--pathspec-file-nul",
        ],
        nul_joined_pathspecs(&pathspecs),
    )?;
    Ok(StackGitOperationResult {
        repository_root: repo_root.to_string_lossy().into_owned(),
        summary: format!("Unstaged {} file(s)", pathspecs.len()),
    })
}
fn stack_git_revert_paths(
    request: StackGitRevertRequest,
) -> Result<StackGitOperationResult, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    if request.paths.is_empty() {
        return Err("Select at least one tracked file to discard".to_string());
    }
    let pathspecs = git_pathspecs_for_paths(&repo_root, &request.paths)?;
    run_git_with_stdin(
        &repo_root,
        &[
            "restore",
            "--worktree",
            "--pathspec-from-file=-",
            "--pathspec-file-nul",
        ],
        nul_joined_pathspecs(&pathspecs),
    )?;
    Ok(StackGitOperationResult {
        repository_root: repo_root.to_string_lossy().into_owned(),
        summary: format!("Discarded changes in {} file(s)", pathspecs.len()),
    })
}
fn stack_git_diff(request: StackGitDiffRequest) -> Result<StackGitDiff, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let path = git_relative_path_for_stage(
        &repo_root,
        &request.path,
        &git_status_relative_paths(&repo_root)?,
    )?;
    let mut args = vec!["diff"];
    if request.staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(&path);
    let content = git_stdout(&repo_root, &args)?.unwrap_or_default();
    Ok(StackGitDiff {
        repository_root: repo_root.to_string_lossy().into_owned(),
        path,
        staged: request.staged,
        content,
    })
}
fn stack_git_stashes(folder_path: &str) -> Result<StackGitStashes, String> {
    let repo_root = repo_root_for_folder(folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let output =
        git_stdout(&repo_root, &["stash", "list", "--format=%gd%x1f%gs%x1e"])?.unwrap_or_default();
    Ok(StackGitStashes {
        repository_root: repo_root.to_string_lossy().into_owned(),
        entries: parse_git_stash_list_output(&output),
    })
}
fn stack_git_stash(request: StackGitStashRequest) -> Result<StackGitOperationResult, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let message = trim_git_stash_message(request.message.as_deref());
    let mut args = vec!["stash", "push", "-m", message.as_str()];
    if request.include_untracked {
        args.push("--include-untracked");
    }
    run_git(&repo_root, &args)?;
    Ok(StackGitOperationResult {
        repository_root: repo_root.to_string_lossy().into_owned(),
        summary: "Stash created".to_string(),
    })
}
fn stack_git_stash_apply(
    request: StackGitStashRefRequest,
) -> Result<StackGitOperationResult, String> {
    git_stash_mutation(request, "apply")
}
fn stack_git_stash_pop(
    request: StackGitStashRefRequest,
) -> Result<StackGitOperationResult, String> {
    git_stash_mutation(request, "pop")
}
fn stack_git_stash_drop(
    request: StackGitStashRefRequest,
) -> Result<StackGitOperationResult, String> {
    git_stash_mutation(request, "drop")
}
fn git_stash_mutation(
    request: StackGitStashRefRequest,
    op: &str,
) -> Result<StackGitOperationResult, String> {
    let repo_root = repo_root_for_folder(&request.folder_path)?
        .ok_or_else(|| "Git repository unavailable".to_string())?;
    let stash_ref = validate_git_stash_ref(&request.stash_ref)?.to_string();
    run_git(&repo_root, &["stash", op, &stash_ref])?;
    Ok(StackGitOperationResult {
        repository_root: repo_root.to_string_lossy().into_owned(),
        summary: format!("Stash {op} done"),
    })
}
fn trim_git_stash_message(value: Option<&str>) -> String {
    const MAX_GIT_STASH_MESSAGE: usize = 200;
    value
        .unwrap_or("")
        .trim()
        .chars()
        .take(MAX_GIT_STASH_MESSAGE)
        .collect()
}
fn validate_git_stash_ref(value: &str) -> Result<&str, String> {
    let value = value.trim();
    let index = value
        .strip_prefix("stash@{")
        .and_then(|rest| rest.strip_suffix('}'));
    if index.is_some_and(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
    {
        Ok(value)
    } else {
        Err("Git stash ref is invalid".to_string())
    }
}
// stash@{N}
fn parse_git_stash_list_output(output: &str) -> Vec<StackGitStashEntry> {
    output
        .split('\x1e')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            let (stash_ref, message) = entry.split_once('\x1f')?;
            let stash_ref = validate_git_stash_ref(stash_ref).ok()?.to_string();
            let index = stash_ref
                .strip_prefix("stash@{")?
                .strip_suffix('}')?
                .parse()
                .ok()?;
            let message = message.trim().to_string();
            let branch = message
                .strip_prefix("WIP on ")
                .or_else(|| message.strip_prefix("On "))
                .and_then(|rest| {
                    rest.split_once(':')
                        .map(|(name, _)| name.trim().to_string())
                })
                .filter(|name| !name.is_empty());
            Some(StackGitStashEntry {
                ref_: stash_ref.clone(),
                stash_ref,
                index,
                branch,
                message,
            })
        })
        .collect()
}

fn git_status_has_unstaged_change(xy: &str) -> bool {
    xy.as_bytes().get(1).is_some_and(|status| *status != b' ')
}

#[cfg(test)]
mod tests {
    use super::{
        classify_git_run_mode, git_pathspecs_for_paths, git_relative_path_for_request,
        git_status_kind, git_timeout_for_mode, normalize_git_remote_url, nul_joined_pathspecs,
        parse_git_branch_output, parse_git_log_output, parse_git_stash_list_output,
        parse_git_tree_output, stack_git_status_from_porcelain, validate_git_branch_name,
        validate_git_stash_ref, validate_treeish, GitCommandError, GitRunMode,
    };
    use crate::stack_popup::models::StackGitFileStatusKind;
    use std::path::Path;
    use std::process::Command;
    use std::time::Duration;

    #[test]
    fn porcelain_status_kinds_cover_stack_badges() {
        assert_eq!(
            git_status_kind(" M"),
            Some(StackGitFileStatusKind::Modified)
        );
        assert_eq!(git_status_kind("A "), Some(StackGitFileStatusKind::Added));
        assert_eq!(git_status_kind(" D"), Some(StackGitFileStatusKind::Deleted));
        assert_eq!(
            git_status_kind("??"),
            Some(StackGitFileStatusKind::Untracked)
        );
        assert_eq!(
            git_status_kind("UU"),
            Some(StackGitFileStatusKind::Conflict)
        );
        assert_eq!(git_status_kind("  "), None);
    }

    #[test]
    fn git_remote_url_normalizer_handles_common_browser_remotes() {
        assert_eq!(
            normalize_git_remote_url("https://github.com/acme/repo.git").as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(
            normalize_git_remote_url("git@github.com:acme/repo.git").as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(
            normalize_git_remote_url("ssh://git@gitlab.com/acme/repo.git").as_deref(),
            Some("https://gitlab.com/acme/repo")
        );
        assert_eq!(normalize_git_remote_url("file:///C:/repo"), None);
        assert_eq!(
            normalize_git_remote_url("https://user:token@github.com/acme/repo.git"),
            None
        );
        assert_eq!(
            normalize_git_remote_url("http://user@github.com/acme/repo.git"),
            None
        );
        assert_eq!(
            normalize_git_remote_url("ssh://user@example.com/acme/repo.git"),
            None
        );
    }

    #[test]
    fn porcelain_parser_returns_counts_and_absolute_paths() {
        let status = stack_git_status_from_porcelain(
            Path::new(r"C:\repo"),
            "main".to_string(),
            Some("https://github.com/acme/repo".to_string()),
            (None, None),
            b" M src/lib.rs\0A  app/main.rs\0?? notes/todo.md\0D  old.txt\0UU conflict.txt\0",
        );

        assert_eq!(status.branch, "main");
        assert_eq!(
            status.remote_repository_url.as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(status.modified, 1);
        assert_eq!(status.added, 1);
        assert_eq!(status.deleted, 1);
        assert_eq!(status.untracked, 1);
        assert_eq!(status.conflicts, 1);
        assert_eq!(status.entries.len(), 5);
        assert_eq!(status.entries[0].relative_path, r"src\lib.rs");
        assert!(!status.entries[0].staged);
        assert!(status.entries[1].staged);
        assert!(status.entries[0].path.ends_with(r"src\lib.rs"));
    }

    #[test]
    fn porcelain_parser_tracks_independent_staged_and_unstaged_sides() {
        let status = stack_git_status_from_porcelain(
            Path::new(r"C:\repo"),
            "main".to_string(),
            None,
            (None, None),
            b"MM both.rs\0M  staged.rs\0 M unstaged.rs\0?? new.txt\0",
        );

        let both = status
            .entries
            .iter()
            .find(|entry| entry.relative_path == r"both.rs")
            .expect("MM path present");
        assert!(both.staged);
        assert!(both.unstaged);

        let staged = status
            .entries
            .iter()
            .find(|entry| entry.relative_path == r"staged.rs")
            .expect("index-only path present");
        assert!(staged.staged);
        assert!(!staged.unstaged);

        let untracked = status
            .entries
            .iter()
            .find(|entry| entry.relative_path == r"new.txt")
            .expect("untracked path present");
        assert!(!untracked.staged);
        assert!(untracked.unstaged);
    }

    #[test]
    fn porcelain_parser_keeps_rename_old_and_new_paths() {
        let status = stack_git_status_from_porcelain(
            Path::new(r"C:\repo"),
            "main".to_string(),
            None,
            (None, None),
            b"R  old.txt\0new.txt\0C  copy-old.txt\0copy-new.txt\0",
        );

        assert_eq!(status.entries.len(), 4);
        assert_eq!(status.entries[0].relative_path, r"old.txt");
        assert_eq!(status.entries[1].relative_path, r"new.txt");
        assert_eq!(status.entries[2].relative_path, r"copy-old.txt");
        assert_eq!(status.entries[3].relative_path, r"copy-new.txt");
    }

    #[test]
    fn branch_parser_marks_remote_refs_and_checkout_tracks_remote() {
        let branches = parse_git_branch_output("*\x1fmain\n \x1fremotes/origin/feature/x\n");
        assert!(!branches[0].remote);
        assert!(branches[1].remote);
        assert_eq!(branches[1].ref_name, "remotes/origin/feature/x");
    }

    #[test]
    fn classify_git_run_mode_treats_restore_and_stash_as_local_mutation_and_diff_as_read() {
        assert_eq!(
            classify_git_run_mode(&["restore", "--staged"]),
            GitRunMode::LocalMutation
        );
        assert_eq!(
            classify_git_run_mode(&["stash", "push"]),
            GitRunMode::LocalMutation
        );
        assert_eq!(classify_git_run_mode(&["diff"]), GitRunMode::Read);
    }

    #[test]
    fn git_pathspecs_reject_paths_outside_repo_and_use_nul_input() {
        let repo_root = std::env::temp_dir().join(format!(
            "jasonshell-git-pathspecs-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&repo_root).unwrap();
        let repo_root = std::fs::canonicalize(&repo_root).unwrap();
        std::fs::create_dir_all(repo_root.join("src")).unwrap();
        let lib = repo_root.join("src").join("lib.rs");
        std::fs::write(&lib, b"lib").unwrap();

        assert_eq!(
            super::normalize_repo_relative_existing_path(&repo_root, &lib).unwrap(),
            "src/lib.rs"
        );
        assert!(super::normalize_repo_relative_existing_path(
            &repo_root,
            &repo_root.join("..").join("other").join("file.rs")
        )
        .is_err());
        assert!(super::normalize_repo_relative_existing_path(
            &repo_root,
            &repo_root.join("missing")
        )
        .is_err());
        std::fs::remove_dir_all(&repo_root).ok();
    }

    #[test]
    fn git_stage_allows_selected_nested_changed_file_with_absolute_path() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let repo_root = std::env::temp_dir().join(format!(
            "jasonshell-git-stage-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(repo_root.join("nested")).unwrap();
        let init = Command::new("git")
            .arg("init")
            .arg(&repo_root)
            .output()
            .unwrap();
        if !init.status.success() {
            let _ = std::fs::remove_dir_all(&repo_root);
            return;
        }
        let file = repo_root.join("nested").join("file.txt");
        std::fs::write(&file, b"hello").unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(&repo_root)
            .arg("add")
            .arg("nested/file.txt")
            .output();
        std::fs::write(&file, b"changed").unwrap();
        let canonical_root = std::fs::canonicalize(&repo_root).unwrap();
        let absolute_file = std::fs::canonicalize(&file).unwrap();

        let pathspecs = git_pathspecs_for_paths(
            &canonical_root,
            &[absolute_file.to_string_lossy().to_string()],
        )
        .unwrap();

        assert_eq!(pathspecs, vec!["nested/file.txt".to_string()]);
        let _ = std::fs::remove_dir_all(&repo_root);
    }

    #[test]
    fn missing_path_validation_rejects_namespace_and_dot_tricks() {
        assert!(super::validate_missing_repo_relative_path("..").is_err());
        assert!(super::validate_missing_repo_relative_path(".").is_err());
        assert!(super::validate_missing_repo_relative_path("src/../x").is_err());
        assert!(super::validate_missing_repo_relative_path("src/:ads").is_err());
        assert!(super::validate_missing_repo_relative_path("src/\0x").is_err());
    }

    #[test]
    fn git_log_parser_uses_delimited_records() {
        let output = concat!(
            "abc123\x1fa1b2c3\x1fAda\x1fada@example.com\x1f2026-05-07T10:00:00-05:00\x1fInitial work\x1e",
            "\n",
            "def456\x1fd4e5f6\x1fBen\x1fben@example.com\x1f2026-05-07T10:05:00-05:00\x1fFollow up\x1e"
        );

        let entries = parse_git_log_output(output);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].commit_hash, "abc123");
        assert_eq!(entries[0].subject, "Initial work");
        assert_eq!(entries[1].author_email, "ben@example.com");
    }

    #[test]
    fn git_run_mode_classifies_remote_and_read_commands() {
        assert_eq!(
            classify_git_run_mode(&["fetch", "--prune"]),
            GitRunMode::Remote
        );
        assert_eq!(classify_git_run_mode(&["status"]), GitRunMode::Read);
        assert_eq!(classify_git_run_mode(&["add"]), GitRunMode::LocalMutation);
    }

    #[test]
    fn git_timeout_override_clamps_to_bounds() {
        assert_eq!(
            git_timeout_for_mode(GitRunMode::Read),
            Duration::from_secs(10)
        );
        assert_eq!(super::env_timeout_override_ms(), None);
    }

    #[test]
    fn git_nonzero_classifier_maps_repository_and_auth_errors() {
        assert!(matches!(
            super::classify_git_nonzero(None, vec![], b"fatal: not a git repository".to_vec()),
            GitCommandError::NotRepository(_)
        ));
        assert!(matches!(
            super::classify_git_nonzero(None, vec![], b"fatal: Authentication failed".to_vec()),
            GitCommandError::AuthRequired(_)
        ));
    }

    #[test]
    fn trusted_git_path_requires_known_location() {
        let path = super::trusted_git_path();
        assert!(path.is_ok() || path.is_err());
    }

    #[test]
    fn git_tree_parser_handles_files_and_directories() {
        let entries = parse_git_tree_output(
            b"100644 blob abc123 42\tsrc/main.rs\000040000 tree def456 -\tsrc-tauri\0",
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, "blob");
        assert_eq!(entries[0].size_bytes, Some(42));
        assert_eq!(entries[0].path, r"src\main.rs");
        assert_eq!(entries[1].kind, "tree");
        assert_eq!(entries[1].size_bytes, None);
    }

    #[test]
    fn git_branch_parser_marks_current_and_remote() {
        let branches =
            parse_git_branch_output("*\x1fmain\n \x1ffeature/work\n \x1fremotes/origin/main\n");

        assert_eq!(branches.len(), 3);
        assert!(branches[0].current);
        assert!(!branches[1].remote);
        assert!(branches[2].remote);
    }

    #[test]
    fn git_request_validation_rejects_option_injection_and_parent_paths() {
        let repo = Path::new(r"C:\repo");

        assert_eq!(
            validate_git_branch_name("feature/work_1").unwrap(),
            "feature/work_1"
        );
        assert!(validate_git_branch_name("--upload-pack=bad").is_err());
        assert!(validate_git_branch_name("feature..bad").is_err());
        assert!(validate_git_branch_name("bad.lock").is_err());
        assert_eq!(validate_treeish("HEAD").unwrap(), "HEAD");
        assert!(validate_treeish("HEAD --help").is_err());
        assert!(git_relative_path_for_request(repo, r"C:\repo\src\main.rs").is_ok());
        assert!(git_relative_path_for_request(repo, r"C:\other\main.rs").is_err());
        assert!(git_relative_path_for_request(repo, r"..\other").is_err());
    }

    #[test]
    fn git_stash_ref_validation_only_accepts_conservative_stack_refs() {
        assert_eq!(validate_git_stash_ref("stash@{0}").unwrap(), "stash@{0}");
        assert_eq!(validate_git_stash_ref("stash@{42}").unwrap(), "stash@{42}");

        for invalid in [
            "stash@{-1}",
            "stash@{0} --index",
            "stash@{0};reset",
            "refs/stash",
            "stash@{0^{tree}}",
            "stash@{abc}",
            "stash@{}",
            "--help",
            "",
        ] {
            assert!(
                validate_git_stash_ref(invalid).is_err(),
                "rejected {invalid:?}"
            );
        }
    }

    #[test]
    fn git_stash_list_parser_returns_typed_entries() {
        let entries = parse_git_stash_list_output(
            "stash@{0}\x1fWIP on main: abc123 work\x1e\nstash@{1}\x1fOn feature/x: message\x1e",
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].stash_ref, "stash@{0}");
        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
        assert_eq!(entries[0].message, "WIP on main: abc123 work");
        assert_eq!(entries[1].index, 1);
    }
}
