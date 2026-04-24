#[cfg(test)]
use super::scoring::search_ranked_results;
use super::scoring::{display_name, should_skip_dir};
use super::SystemSearchResult;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_INDEXED_FILES: usize = 25_000;

struct FileRoot {
    path: PathBuf,
    max_depth: usize,
    priority: i32,
}

pub(crate) fn index_files() -> Vec<SystemSearchResult> {
    index_files_in_roots(file_roots(), MAX_INDEXED_FILES)
}

fn file_roots() -> Vec<FileRoot> {
    let mut roots = Vec::new();
    let Some(profile) = env::var_os("USERPROFILE").map(PathBuf::from) else {
        return roots;
    };

    roots.push(FileRoot {
        path: profile.clone(),
        max_depth: 2,
        priority: 62,
    });

    for name in [
        "Desktop",
        "Documents",
        "Downloads",
        "Pictures",
        "Music",
        "Videos",
    ] {
        roots.push(FileRoot {
            path: profile.join(name),
            max_depth: 6,
            priority: 76,
        });
    }

    for name in ["OneDrive", "OneDriveCommercial", "OneDriveConsumer"] {
        let path = profile.join(name);
        if path.exists() {
            roots.push(FileRoot {
                path,
                max_depth: 5,
                priority: 70,
            });
        }
    }

    roots
}

#[cfg(test)]
fn search_files_in_roots(
    roots: Vec<FileRoot>,
    query: &str,
    limit: usize,
) -> Vec<SystemSearchResult> {
    let entries = index_files_in_roots(roots, MAX_INDEXED_FILES);
    search_ranked_results(&entries, query, limit)
}

fn index_files_in_roots(roots: Vec<FileRoot>, limit: usize) -> Vec<SystemSearchResult> {
    let mut seen = HashSet::new();
    let mut results = Vec::new();

    for root in roots {
        if !root.path.exists() {
            continue;
        }

        collect_file_matches(&root, limit, &mut seen, &mut results);
    }

    results.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then(left.title.cmp(&right.title))
    });
    results.truncate(limit);
    results
}

fn collect_file_matches(
    root: &FileRoot,
    limit: usize,
    seen: &mut HashSet<String>,
    results: &mut Vec<SystemSearchResult>,
) {
    let mut stack = vec![(root.path.clone(), 0)];
    let mut visited = 0;

    while let Some((dir, depth)) = stack.pop() {
        visited += 1;
        if visited > 12_000 || results.len() >= limit {
            break;
        }

        let mut entries = read_sorted_dir(&dir);
        entries.reverse();
        for entry in entries {
            let path = entry.path();
            let is_dir = path.is_dir();
            if is_dir && depth < root.max_depth && !should_skip_dir(&path) {
                stack.push((path.clone(), depth + 1));
            }
            push_file_result(path, is_dir, root.priority, seen, results);
        }
    }
}

fn push_file_result(
    path: PathBuf,
    is_dir: bool,
    base_priority: i32,
    seen: &mut HashSet<String>,
    results: &mut Vec<SystemSearchResult>,
) {
    let key = path.to_string_lossy().to_lowercase();
    if !seen.insert(key) {
        return;
    }

    let title = display_name(&path);
    let kind = if is_dir { "folder" } else { "file" };
    let label = if is_dir { "Folder" } else { "File" };
    let subtitle = path
        .parent()
        .map(|parent| format!("{label} - {}", parent.display()))
        .unwrap_or_else(|| label.to_string());
    results.push(SystemSearchResult::new(
        kind,
        title,
        subtitle,
        path,
        base_priority,
    ));
}

fn read_sorted_dir(path: &Path) -> Vec<fs::DirEntry> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn finds_matching_file_in_user_roots() {
        let root = test_dir("files");
        fs::create_dir_all(root.join("Documents")).unwrap();
        fs::write(root.join("Documents").join("spotify notes.txt"), b"notes").unwrap();

        let results = search_files_in_roots(
            vec![FileRoot {
                path: root.clone(),
                max_depth: 4,
                priority: 76,
            }],
            "spotify",
            8,
        );

        fs::remove_dir_all(root).ok();
        assert_eq!(
            results.first().map(|result| result.kind.as_str()),
            Some("file")
        );
        assert_eq!(
            results.first().map(|result| result.title.as_str()),
            Some("spotify notes")
        );
    }

    fn test_dir(name: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("search-{name}-{id}"))
    }
}
