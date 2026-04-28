use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GitWorkspaceStatus {
    pub is_repository: bool,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub head_oid: Option<String>,
    pub is_clean: bool,
    pub has_changes: bool,
    pub ahead: u32,
    pub behind: u32,
    pub has_conflicts: bool,
    pub is_rebasing: bool,
    pub is_merging: bool,
    pub summary: String,
}

#[tauri::command]
pub fn get_workspace_git_status(path: String) -> Result<GitWorkspaceStatus, String> {
    let workspace = PathBuf::from(path);
    if !workspace.is_dir() {
        return Err("workspace path must be an existing directory".to_string());
    }
    let status_output = Command::new("git")
        .arg("-C")
        .arg(&workspace)
        .arg("status")
        .arg("--porcelain=v2")
        .arg("--branch")
        .output()
        .map_err(|error| format!("failed to run git status: {error}"))?;

    if !status_output.status.success() {
        return Ok(GitWorkspaceStatus {
            is_repository: false,
            is_clean: true,
            summary: "not a git repository".to_string(),
            ..GitWorkspaceStatus::default()
        });
    }

    let status_text = String::from_utf8_lossy(&status_output.stdout);
    let git_dir = resolve_git_dir(&workspace).ok();
    let state = git_dir
        .as_deref()
        .map(detect_git_operation_state)
        .unwrap_or_default();
    let mut parsed = parse_git_status_output(&status_text);
    parsed.is_repository = true;
    parsed.is_merging = state.is_merging;
    parsed.is_rebasing = state.is_rebasing;
    parsed.summary = summarize_git_status(&parsed);
    Ok(parsed)
}

pub fn parse_git_status_output(output: &str) -> GitWorkspaceStatus {
    let mut status = GitWorkspaceStatus {
        is_repository: true,
        is_clean: true,
        summary: "clean".to_string(),
        ..GitWorkspaceStatus::default()
    };

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("# branch.oid ") {
            status.head_oid = Some(value.to_string());
            continue;
        }
        if let Some(value) = line.strip_prefix("# branch.head ") {
            if value != "(detached)" {
                status.branch = Some(value.to_string());
            }
            continue;
        }
        if let Some(value) = line.strip_prefix("# branch.upstream ") {
            status.upstream = Some(value.to_string());
            continue;
        }
        if let Some(value) = line.strip_prefix("# branch.ab ") {
            parse_ahead_behind(value, &mut status);
            continue;
        }
        if is_change_line(line) {
            status.is_clean = false;
            status.has_changes = true;
            if is_conflict_line(line) {
                status.has_conflicts = true;
            }
        }
    }

    status.summary = summarize_git_status(&status);
    status
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GitOperationState {
    pub is_merging: bool,
    pub is_rebasing: bool,
}

pub fn detect_git_operation_state(git_dir: &Path) -> GitOperationState {
    GitOperationState {
        is_merging: git_dir.join("MERGE_HEAD").exists(),
        is_rebasing: git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists(),
    }
}

fn parse_ahead_behind(value: &str, status: &mut GitWorkspaceStatus) {
    for part in value.split_whitespace() {
        if let Some(ahead) = part.strip_prefix('+').and_then(|value| value.parse().ok()) {
            status.ahead = ahead;
        } else if let Some(behind) = part.strip_prefix('-').and_then(|value| value.parse().ok()) {
            status.behind = behind;
        }
    }
}

fn is_change_line(line: &str) -> bool {
    line.starts_with("1 ")
        || line.starts_with("2 ")
        || line.starts_with("u ")
        || line.starts_with("? ")
        || line.starts_with("! ")
}

fn is_conflict_line(line: &str) -> bool {
    if line.starts_with("u ") {
        return true;
    }
    let mut parts = line.split_whitespace();
    let record_type = parts.next();
    let xy = parts.next();
    matches!(record_type, Some("1") | Some("2"))
        && xy.is_some_and(|value| matches!(value, "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU"))
}

fn summarize_git_status(status: &GitWorkspaceStatus) -> String {
    if !status.is_repository {
        return "not a git repository".to_string();
    }
    let mut parts = Vec::new();
    if status.has_conflicts {
        parts.push("conflicts".to_string());
    } else if status.has_changes {
        parts.push("dirty".to_string());
    } else {
        parts.push("clean".to_string());
    }
    if status.ahead > 0 {
        parts.push(format!("ahead {}", status.ahead));
    }
    if status.behind > 0 {
        parts.push(format!("behind {}", status.behind));
    }
    if status.is_rebasing {
        parts.push("rebasing".to_string());
    }
    if status.is_merging {
        parts.push("merging".to_string());
    }
    parts.join(", ")
}

fn resolve_git_dir(workspace: &Path) -> Result<PathBuf, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .arg("rev-parse")
        .arg("--git-dir")
        .output()
        .map_err(|error| format!("failed to run git rev-parse: {error}"))?;
    if !output.status.success() {
        return Err("not a git repository".to_string());
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(workspace.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_clean_dirty_ahead_behind_and_conflicts() {
        let output = "\
# branch.oid abc123
# branch.head main
# branch.upstream origin/main
# branch.ab +2 -1
1 .M N... 100644 100644 100644 abc abc file.txt
u UU N... 100644 100644 100644 100644 a b c d conflict.txt
? untracked.txt
";

        let status = parse_git_status_output(output);

        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.upstream.as_deref(), Some("origin/main"));
        assert_eq!(status.head_oid.as_deref(), Some("abc123"));
        assert!(!status.is_clean);
        assert!(status.has_changes);
        assert_eq!(status.ahead, 2);
        assert_eq!(status.behind, 1);
        assert!(status.has_conflicts);
        assert!(status.summary.contains("conflicts"));
    }

    #[test]
    fn detects_merge_and_rebase_files() {
        let dir = test_dir("git-state");
        fs::create_dir_all(dir.join("rebase-merge")).unwrap();
        fs::write(dir.join("MERGE_HEAD"), "abc").unwrap();

        let state = detect_git_operation_state(&dir);

        assert!(state.is_merging);
        assert!(state.is_rebasing);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reads_status_from_temp_git_repo_when_git_is_available() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let dir = test_dir("repo");
        let init = Command::new("git").arg("init").arg(&dir).output().unwrap();
        if !init.status.success() {
            let _ = fs::remove_dir_all(dir);
            return;
        }
        fs::write(dir.join("tracked.txt"), "hello").unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(&dir)
            .arg("add")
            .arg("tracked.txt")
            .output();

        let status = get_workspace_git_status(dir.to_string_lossy().to_string()).unwrap();

        assert!(status.is_repository);
        assert!(status.has_changes);
        assert!(!status.is_clean);
        let _ = fs::remove_dir_all(dir);
    }

    fn test_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("jasonshell-{name}-{suffix}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
