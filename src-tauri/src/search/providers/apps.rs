use crate::search::contracts::{
    SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState, SearchProviderId,
    SearchProviderTiming, SearchResult, SearchResultAction, SearchResultKind,
};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const APP_INDEX_TTL: Duration = Duration::from_secs(60);
const MAX_INDEXED_APPS: usize = 4_000;
const MAX_VISITED_APP_DIRS: usize = 8_000;

static APP_INDEX_CACHE: OnceLock<Mutex<Option<CachedAppIndex>>> = OnceLock::new();
static APP_INDEX_REFRESH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug)]
struct AppIndexEntry {
    title: String,
    path: PathBuf,
    source: &'static str,
    aliases: Vec<String>,
    priority: i32,
}

#[derive(Clone, Debug)]
struct CachedAppIndex {
    indexed_at: Instant,
    entries: Vec<AppIndexEntry>,
}

impl CachedAppIndex {
    fn is_fresh(&self, now: Instant) -> bool {
        now.duration_since(self.indexed_at) <= APP_INDEX_TTL
    }
}

#[derive(Clone, Debug)]
struct AppRoot {
    path: PathBuf,
    extensions: &'static [&'static str],
    max_depth: usize,
    priority: i32,
    source: &'static str,
}

#[derive(Clone, Debug)]
pub(crate) struct AppsSearchRun {
    pub(crate) results: Vec<SearchResult>,
    pub(crate) timing: SearchProviderTiming,
    pub(crate) health: SearchProviderHealth,
}

pub(crate) fn search_apps(query: &str, limit: usize) -> AppsSearchRun {
    let started_at = crate::search::contracts::iso_now();
    let started = Instant::now();
    let (entries, cache_state, refresh_needed) = cached_app_entries(Instant::now());
    if refresh_needed {
        warm_app_index_async();
    }
    let results = rank_apps(&entries, query, limit);
    let result_count = results.len();
    let health_state = if entries.is_empty() && refresh_needed {
        SearchProviderHealthState::Indexing
    } else {
        SearchProviderHealthState::Ready
    };

    AppsSearchRun {
        results,
        timing: SearchProviderTiming {
            provider_id: SearchProviderId::Apps,
            started_at,
            ended_at: Some(crate::search::contracts::iso_now()),
            duration_ms: started.elapsed().as_secs_f64() * 1000.0,
            cache: cache_state,
            result_count,
            applied: true,
            discarded_as_stale: false,
        },
        health: SearchProviderHealth {
            provider_id: SearchProviderId::Apps,
            state: health_state,
            reason_code: None,
            message: Some(format!("cached app index has {} rows", entries.len())),
        },
    }
}

pub(crate) fn warm_app_index_async() {
    if APP_INDEX_REFRESH_IN_FLIGHT.swap(true, Ordering::AcqRel) {
        return;
    }

    thread::spawn(|| {
        refresh_app_index_cache();
        APP_INDEX_REFRESH_IN_FLIGHT.store(false, Ordering::Release);
    });
}

fn refresh_app_index_cache() {
    let entries = build_app_index(app_roots(), MAX_INDEXED_APPS);
    let cache = APP_INDEX_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(CachedAppIndex {
            indexed_at: Instant::now(),
            entries,
        });
    }
}

fn cached_app_entries(now: Instant) -> (Vec<AppIndexEntry>, SearchProviderCacheState, bool) {
    let cache = APP_INDEX_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        return cached_app_entries_from_cache(guard.as_ref(), now);
    }
    (Vec::new(), SearchProviderCacheState::Disabled, false)
}

fn cached_app_entries_from_cache(
    cached: Option<&CachedAppIndex>,
    now: Instant,
) -> (Vec<AppIndexEntry>, SearchProviderCacheState, bool) {
    match cached {
        Some(cached) if cached.is_fresh(now) => {
            (cached.entries.clone(), SearchProviderCacheState::Hit, false)
        }
        Some(cached) => (
            cached.entries.clone(),
            SearchProviderCacheState::Refresh,
            true,
        ),
        None => (Vec::new(), SearchProviderCacheState::Miss, true),
    }
}

fn app_roots() -> Vec<AppRoot> {
    let mut roots = Vec::new();

    if let Some(appdata) = env_path("APPDATA") {
        roots.push(AppRoot {
            path: appdata.join(r"Microsoft\Windows\Start Menu\Programs"),
            extensions: &["lnk", "appref-ms", "url"],
            max_depth: 8,
            priority: 1_550,
            source: "currentUserStartMenu",
        });
        roots.push(AppRoot {
            path: appdata.join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar"),
            extensions: &["lnk"],
            max_depth: 3,
            priority: 1_560,
            source: "pinnedTaskbar",
        });
    }

    if let Some(programdata) = env_path("PROGRAMDATA") {
        roots.push(AppRoot {
            path: programdata.join(r"Microsoft\Windows\Start Menu\Programs"),
            extensions: &["lnk", "appref-ms", "url"],
            max_depth: 8,
            priority: 1_545,
            source: "allUsersStartMenu",
        });
    }

    if let Some(local_appdata) = env_path("LOCALAPPDATA") {
        roots.push(AppRoot {
            path: local_appdata.join(r"Microsoft\WindowsApps"),
            extensions: &["exe", "lnk", "appref-ms"],
            max_depth: 2,
            priority: 1_500,
            source: "windowsApps",
        });
    }

    for name in ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(path) = env_path(name) {
            roots.push(AppRoot {
                path: path.join("Programs"),
                extensions: &["exe", "lnk", "appref-ms"],
                max_depth: 5,
                priority: 1_450,
                source: "programs",
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

fn build_app_index(roots: Vec<AppRoot>, limit: usize) -> Vec<AppIndexEntry> {
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    for root in roots {
        if !root.path.exists() {
            continue;
        }
        collect_app_entries(&root, limit, &mut seen, &mut entries);
    }

    entries.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then(left.title.cmp(&right.title))
            .then(left.path.cmp(&right.path))
    });
    entries.truncate(limit);
    entries
}

fn collect_app_entries(
    root: &AppRoot,
    limit: usize,
    seen: &mut HashSet<String>,
    entries: &mut Vec<AppIndexEntry>,
) {
    let mut stack = vec![(root.path.clone(), 0usize)];
    let mut visited = 0usize;

    while let Some((dir, depth)) = stack.pop() {
        visited += 1;
        if visited > MAX_VISITED_APP_DIRS || entries.len() >= limit {
            break;
        }

        let mut children = read_sorted_dir(&dir);
        children.reverse();
        for child in children {
            let path = child.path();
            if path.is_dir() {
                if depth < root.max_depth && !should_skip_dir(&path) {
                    stack.push((path, depth + 1));
                }
            } else if has_extension(&path, root.extensions) {
                push_app_entry(path, root, seen, entries);
            }
        }
    }
}

fn push_app_entry(
    path: PathBuf,
    root: &AppRoot,
    seen: &mut HashSet<String>,
    entries: &mut Vec<AppIndexEntry>,
) {
    let key = path.to_string_lossy().to_lowercase();
    if !seen.insert(key) {
        return;
    }

    let title = display_name(&path);
    entries.push(AppIndexEntry {
        aliases: app_aliases(&title, &path),
        title,
        path,
        source: root.source,
        priority: root.priority,
    });
}

fn rank_apps(entries: &[AppIndexEntry], query: &str, limit: usize) -> Vec<SearchResult> {
    let tokens = query_tokens(query);
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut results = entries
        .iter()
        .filter_map(|entry| {
            score_app(entry, &tokens).map(|(score, reason)| app_result(entry, score, reason))
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.title.cmp(&right.title))
            .then(left.record_key.cmp(&right.record_key))
    });
    results.truncate(limit);
    results
}

fn app_result(entry: &AppIndexEntry, score: i32, reason: &'static str) -> SearchResult {
    let path = entry.path.display().to_string();
    let record_key = format!("app:{}", normalize_record_key(&path));
    SearchResult {
        id: record_key.clone(),
        provider_id: SearchProviderId::Apps,
        kind: SearchResultKind::App,
        title: entry.title.clone(),
        subtitle: Some(format!("Application - {}", entry.source)),
        path: Some(path.clone()),
        action: SearchResultAction::OpenApp { path },
        terms: token_terms(&format!("{} {}", entry.title, entry.path.display())),
        aliases: entry.aliases.clone(),
        score,
        match_reason: reason.to_string(),
        record_key,
        icon_data_url: None,
    }
}

fn score_app(entry: &AppIndexEntry, tokens: &[String]) -> Option<(i32, &'static str)> {
    let query = tokens.join(" ");
    let title = normalize(&entry.title);
    let aliases = entry
        .aliases
        .iter()
        .map(|alias| normalize(alias))
        .collect::<Vec<_>>();
    let path = normalize(&entry.path.display().to_string());

    if title == query || aliases.iter().any(|alias| alias == &query) {
        return Some((entry.priority + 900, "exactApp"));
    }
    if title.starts_with(&query) || aliases.iter().any(|alias| alias.starts_with(&query)) {
        return Some((entry.priority + 650, "prefixApp"));
    }
    if acronym(&title) == query {
        return Some((entry.priority + 500, "acronymApp"));
    }
    let searchable = format!("{} {} {}", title, aliases.join(" "), path);
    if tokens.iter().all(|token| searchable.contains(token)) {
        return Some((entry.priority + 220, "tokenApp"));
    }
    None
}

fn app_aliases(title: &str, path: &Path) -> Vec<String> {
    let mut aliases = vec![title.to_string()];
    if let Some(stem) = path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
    {
        if stem != title {
            aliases.push(stem);
        }
    }
    aliases
}

fn display_name(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn should_skip_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    matches!(name.as_str(), "node_modules" | "target" | ".git")
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .map(|extension| extensions.iter().any(|expected| extension == *expected))
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

fn query_tokens(query: &str) -> Vec<String> {
    normalize(query)
        .split(' ')
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn token_terms(value: &str) -> Vec<String> {
    normalize(value)
        .split(' ')
        .filter(|token| !token.is_empty())
        .take(16)
        .map(str::to_string)
        .collect()
}

fn acronym(value: &str) -> String {
    value
        .split_whitespace()
        .filter_map(|token| token.chars().next())
        .collect()
}

fn normalize_record_key(path: &str) -> String {
    path.trim().replace('/', r"\").to_lowercase()
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['_', '-', '.', '/', '\\', ':'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn app_index_cache_freshness_is_bounded_by_ttl() {
        let cache = CachedAppIndex {
            indexed_at: Instant::now(),
            entries: Vec::new(),
        };

        assert!(cache.is_fresh(cache.indexed_at + Duration::from_secs(5)));
        assert!(!cache.is_fresh(cache.indexed_at + Duration::from_secs(120)));
    }

    #[test]
    fn indexes_start_menu_shortcuts_once_then_searches_in_memory() {
        let root = test_dir("apps");
        fs::create_dir_all(root.join("Media")).unwrap();
        fs::write(root.join("Media").join("Spotify.lnk"), b"shortcut").unwrap();
        let entries = build_app_index(
            vec![AppRoot {
                path: root.clone(),
                extensions: &["lnk"],
                max_depth: 4,
                priority: 1_550,
                source: "testStartMenu",
            }],
            100,
        );

        let results = rank_apps(&entries, "spotify", 5);

        fs::remove_dir_all(root).ok();
        assert_eq!(
            results.first().map(|result| result.title.as_str()),
            Some("Spotify")
        );
        assert_eq!(results[0].provider_id, SearchProviderId::Apps);
        assert!(results[0].score > 2_000);
    }

    #[test]
    fn cold_query_path_returns_cache_miss_without_scanning() {
        let now = Instant::now();
        let (entries, cache_state, refresh_needed) = cached_app_entries_from_cache(None, now);

        assert!(entries.is_empty());
        assert_eq!(cache_state, SearchProviderCacheState::Miss);
        assert!(refresh_needed);
    }

    #[test]
    fn stale_app_cache_returns_existing_rows_while_refresh_is_deferred() {
        let now = Instant::now();
        let cached = CachedAppIndex {
            indexed_at: now - Duration::from_secs(120),
            entries: vec![AppIndexEntry {
                title: "Spotify".to_string(),
                path: PathBuf::from(r"C:\Apps\Spotify.lnk"),
                source: "test",
                aliases: vec!["Spotify".to_string()],
                priority: 1_550,
            }],
        };
        let (entries, cache_state, refresh_needed) =
            cached_app_entries_from_cache(Some(&cached), now);

        assert_eq!(entries.len(), 1);
        assert_eq!(cache_state, SearchProviderCacheState::Refresh);
        assert!(refresh_needed);
    }

    #[test]
    fn app_results_outrank_incidental_everything_scores() {
        let entry = AppIndexEntry {
            title: "Spotify".to_string(),
            path: PathBuf::from(
                r"C:\Users\me\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Spotify.lnk",
            ),
            source: "testStartMenu",
            aliases: vec!["Spotify".to_string()],
            priority: 1_550,
        };

        let result = rank_apps(&[entry], "spotify", 1).pop().unwrap();

        assert!(result.score > 2_000);
    }

    fn test_dir(name: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("search-phase4-{name}-{id}"))
    }
}
