use super::scoring::{display_name, search_ranked_results, should_skip_dir};
use super::SystemSearchResult;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_INDEXED_APPS: usize = 4_000;
const APP_INDEX_TTL: Duration = Duration::from_secs(60);

static APP_INDEX_CACHE: OnceLock<Mutex<Option<CachedAppIndex>>> = OnceLock::new();

struct AppRoot {
    path: PathBuf,
    extensions: &'static [&'static str],
    max_depth: usize,
    priority: i32,
}

#[derive(Clone)]
struct CachedAppIndex {
    indexed_at: Instant,
    entries: Vec<SystemSearchResult>,
}

impl CachedAppIndex {
    fn is_fresh(&self, now: Instant) -> bool {
        now.duration_since(self.indexed_at) <= APP_INDEX_TTL
    }
}

pub(crate) fn index_apps() -> Vec<SystemSearchResult> {
    index_apps_in_roots(app_roots(), MAX_INDEXED_APPS)
}

pub(crate) fn search_apps(query: &str, limit: usize) -> Vec<SystemSearchResult> {
    let entries = cached_index_apps();
    search_ranked_results(&entries, query, limit)
}

fn cached_index_apps() -> Vec<SystemSearchResult> {
    let cache = APP_INDEX_CACHE.get_or_init(|| Mutex::new(None));
    let now = Instant::now();
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.as_ref().filter(|cached| cached.is_fresh(now)) {
            return cached.entries.clone();
        }
    }

    let entries = index_apps();
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(CachedAppIndex {
            indexed_at: now,
            entries: entries.clone(),
        });
    }
    entries
}

fn app_roots() -> Vec<AppRoot> {
    let mut roots = Vec::new();

    if let Some(appdata) = env_path("APPDATA") {
        roots.push(AppRoot {
            path: appdata.join(r"Microsoft\Windows\Start Menu\Programs"),
            extensions: &["lnk", "appref-ms", "url"],
            max_depth: 8,
            priority: 112,
        });
        roots.push(AppRoot {
            path: appdata.join("Spotify"),
            extensions: &["exe"],
            max_depth: 2,
            priority: 96,
        });
    }

    if let Some(programdata) = env_path("PROGRAMDATA") {
        roots.push(AppRoot {
            path: programdata.join(r"Microsoft\Windows\Start Menu\Programs"),
            extensions: &["lnk", "appref-ms", "url"],
            max_depth: 8,
            priority: 112,
        });
    }

    if let Some(local_appdata) = env_path("LOCALAPPDATA") {
        roots.push(AppRoot {
            path: local_appdata.join(r"Microsoft\WindowsApps"),
            extensions: &["exe", "lnk", "appref-ms"],
            max_depth: 2,
            priority: 94,
        });
    }

    for name in ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(path) = env_path(name) {
            roots.push(AppRoot {
                path: path.join("Programs"),
                extensions: &["exe", "lnk", "appref-ms"],
                max_depth: 5,
                priority: 92,
            });
            roots.push(AppRoot {
                path,
                extensions: &["exe"],
                max_depth: 3,
                priority: 84,
            });
        }
    }

    roots
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.exists())
}

#[cfg(test)]
fn search_apps_in_roots(roots: Vec<AppRoot>, query: &str, limit: usize) -> Vec<SystemSearchResult> {
    let entries = index_apps_in_roots(roots, MAX_INDEXED_APPS);
    search_ranked_results(&entries, query, limit)
}

fn index_apps_in_roots(roots: Vec<AppRoot>, limit: usize) -> Vec<SystemSearchResult> {
    let mut seen = HashSet::new();
    let mut results = Vec::new();

    for root in roots {
        if !root.path.exists() {
            continue;
        }

        collect_app_matches(&root, limit, &mut seen, &mut results);
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

fn collect_app_matches(
    root: &AppRoot,
    limit: usize,
    seen: &mut HashSet<String>,
    results: &mut Vec<SystemSearchResult>,
) {
    let mut stack = vec![(root.path.clone(), 0)];
    let mut visited = 0;

    while let Some((dir, depth)) = stack.pop() {
        visited += 1;
        if visited > 8_000 || results.len() >= limit {
            break;
        }

        let mut entries = read_sorted_dir(&dir);
        entries.reverse();
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                if depth < root.max_depth && !should_skip_dir(&path) {
                    stack.push((path, depth + 1));
                }
            } else if has_extension(&path, root.extensions) {
                push_app_result(path, root.priority, seen, results);
            }
        }
    }
}

fn push_app_result(
    path: PathBuf,
    base_priority: i32,
    seen: &mut HashSet<String>,
    results: &mut Vec<SystemSearchResult>,
) {
    let key = path.to_string_lossy().to_lowercase();
    if !seen.insert(key) {
        return;
    }

    let title = display_name(&path);
    let subtitle = path
        .parent()
        .map(|parent| format!("Installed app - {}", parent.display()))
        .unwrap_or_else(|| "Installed app".to_string());
    results.push(SystemSearchResult::new(
        "app",
        title,
        subtitle,
        path,
        base_priority,
    ));
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .map(|extension| {
            let extension = extension.to_string_lossy().to_lowercase();
            extensions.iter().any(|expected| extension == *expected)
        })
        .unwrap_or(false)
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
    fn finds_unpinned_start_menu_app_shortcut() {
        let root = test_dir("apps");
        fs::create_dir_all(root.join("Media")).unwrap();
        fs::write(root.join("Media").join("Spotify.lnk"), b"shortcut").unwrap();

        let results = search_apps_in_roots(
            vec![AppRoot {
                path: root.clone(),
                extensions: &["lnk"],
                max_depth: 4,
                priority: 112,
            }],
            "spotify",
            8,
        );

        fs::remove_dir_all(root).ok();
        assert_eq!(
            results.first().map(|result| result.title.as_str()),
            Some("Spotify")
        );
    }

    #[test]
    fn cached_app_index_freshness_is_bounded() {
        let indexed_at = Instant::now();
        let cache = CachedAppIndex {
            indexed_at,
            entries: Vec::new(),
        };

        assert!(cache.is_fresh(indexed_at + Duration::from_secs(10)));
        assert!(!cache.is_fresh(indexed_at + Duration::from_secs(120)));
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
