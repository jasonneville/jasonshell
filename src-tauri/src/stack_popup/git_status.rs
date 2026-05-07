use crate::stack_popup::models::{
    StackGitBranch, StackGitBranchRequest, StackGitBranches, StackGitCommitRequest,
    StackGitFileStatus, StackGitFileStatusKind, StackGitLog, StackGitLogEntry, StackGitLogRequest,
    StackGitOperationResult, StackGitStageRequest, StackGitStatus, StackGitTree, StackGitTreeEntry,
    StackGitTreeRequest,
};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DEFAULT_LOG_LIMIT: usize = 40;
const MAX_LOG_LIMIT: usize = 200;

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
    let repo_root = PathBuf::from(repo_root_text.trim());
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

    Ok(Some(stack_git_status_from_porcelain(
        &repo_root,
        branch,
        &status_output,
    )))
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
    let message = request.message.trim();
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
            message,
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
    run_git(&repo_root, &["switch", "--", branch])?;
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
    let repo_root = PathBuf::from(repo_root_text.trim());
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
    let mut pathspecs = Vec::with_capacity(paths.len());
    for path in paths {
        let candidate = PathBuf::from(path);
        if candidate.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        }) {
            return Err("Git path is invalid".to_string());
        }
        if !candidate.is_absolute() || !candidate.starts_with(repo_root) {
            return Err("Git path is outside the repository".to_string());
        }
        let relative = candidate
            .strip_prefix(repo_root)
            .map_err(|_| "Git path is outside the repository".to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if relative.is_empty() || relative.contains('\0') {
            return Err("Git path is invalid".to_string());
        }
        pathspecs.push(relative);
    }
    Ok(pathspecs)
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
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|error| format!("Failed to run git: {error}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(output.stdout))
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("Failed to run git: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("Git failed with status {}", output.status)
    } else {
        stderr
    })
}

fn run_git_with_stdin(cwd: &Path, args: &[&str], stdin: Vec<u8>) -> Result<(), String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to run git: {error}"))?;
    if let Some(mut input) = child.stdin.take() {
        input
            .write_all(&stdin)
            .map_err(|error| format!("Failed to write git input: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("Failed to wait for git: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("Git failed with status {}", output.status)
    } else {
        stderr
    })
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
            })
        })
        .collect()
}

fn stack_git_status_from_porcelain(
    repo_root: &Path,
    branch: String,
    porcelain: &[u8],
) -> StackGitStatus {
    let mut status = StackGitStatus {
        repository_root: repo_root.to_string_lossy().into_owned(),
        branch,
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
        if matches!(xy.as_bytes().first(), Some(b'R' | b'C')) {
            index += 1;
        }
        if let Some(kind) = status_kind {
            increment_status_count(&mut status, kind);
            status.entries.push(StackGitFileStatus {
                path: absolute_git_status_path(repo_root, &relative_path),
                relative_path,
                status: kind,
                staged: git_status_has_staged_change(xy),
            });
        }

        index += 1;
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

#[cfg(test)]
mod tests {
    use super::{
        git_pathspecs_for_paths, git_relative_path_for_request, git_status_kind,
        nul_joined_pathspecs, parse_git_branch_output, parse_git_log_output, parse_git_tree_output,
        stack_git_status_from_porcelain, validate_git_branch_name, validate_treeish,
    };
    use crate::stack_popup::models::StackGitFileStatusKind;
    use std::path::Path;

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
    fn porcelain_parser_returns_counts_and_absolute_paths() {
        let status = stack_git_status_from_porcelain(
            Path::new(r"C:\repo"),
            "main".to_string(),
            b" M src/lib.rs\0A  app/main.rs\0?? notes/todo.md\0D  old.txt\0UU conflict.txt\0",
        );

        assert_eq!(status.branch, "main");
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
    fn git_pathspecs_reject_paths_outside_repo_and_use_nul_input() {
        let repo = Path::new(r"C:\repo");
        let paths = vec![
            r"C:\repo\src\lib.rs".to_string(),
            r"C:\repo\old file.txt".to_string(),
        ];
        let specs = git_pathspecs_for_paths(repo, &paths).expect("valid repo paths");

        assert_eq!(specs, vec!["src/lib.rs", "old file.txt"]);
        assert_eq!(nul_joined_pathspecs(&specs), b"src/lib.rs\0old file.txt\0");
        assert!(git_pathspecs_for_paths(repo, &[r"C:\other\file.rs".to_string()]).is_err());
        assert!(git_pathspecs_for_paths(repo, &[r"C:\repo\..\other\file.rs".to_string()]).is_err());
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
}
