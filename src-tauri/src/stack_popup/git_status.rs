use crate::stack_popup::models::{
    StackGitCommitRequest, StackGitFileStatus, StackGitFileStatusKind, StackGitOperationResult,
    StackGitStageRequest, StackGitStatus,
};
use std::path::{Path, PathBuf};
use std::io::Write;
use std::process::{Command, Stdio};

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
        git_pathspecs_for_paths, git_status_kind, nul_joined_pathspecs,
        stack_git_status_from_porcelain,
    };
    use crate::stack_popup::models::StackGitFileStatusKind;
    use std::path::Path;

    #[test]
    fn porcelain_status_kinds_cover_stack_badges() {
        assert_eq!(git_status_kind(" M"), Some(StackGitFileStatusKind::Modified));
        assert_eq!(git_status_kind("A "), Some(StackGitFileStatusKind::Added));
        assert_eq!(git_status_kind(" D"), Some(StackGitFileStatusKind::Deleted));
        assert_eq!(git_status_kind("??"), Some(StackGitFileStatusKind::Untracked));
        assert_eq!(git_status_kind("UU"), Some(StackGitFileStatusKind::Conflict));
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
        let paths = vec![r"C:\repo\src\lib.rs".to_string(), r"C:\repo\old file.txt".to_string()];
        let specs = git_pathspecs_for_paths(repo, &paths).expect("valid repo paths");

        assert_eq!(specs, vec!["src/lib.rs", "old file.txt"]);
        assert_eq!(nul_joined_pathspecs(&specs), b"src/lib.rs\0old file.txt\0");
        assert!(git_pathspecs_for_paths(repo, &[r"C:\other\file.rs".to_string()]).is_err());
        assert!(git_pathspecs_for_paths(repo, &[r"C:\repo\..\other\file.rs".to_string()]).is_err());
    }
}
