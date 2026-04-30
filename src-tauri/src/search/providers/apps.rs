use crate::search::contracts::{
    SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState, SearchProviderId,
    SearchProviderTiming, SearchResult, SearchResultAction, SearchResultKind,
};
use crate::search::matcher::{
    best_match, query_tokens as match_query_tokens, MatchData, MatchField,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const APP_INDEX_TTL: Duration = Duration::from_secs(60);
const APP_INDEX_CACHE_FILE: &str = "search-app-index-v1.json";
const APP_INDEX_CACHE_VERSION: u32 = 1;
const MAX_INDEXED_APPS: usize = 4_000;
const MAX_VISITED_APP_DIRS: usize = 8_000;

static APP_INDEX_RUNTIME: OnceLock<Mutex<AppIndexRuntimeState>> = OnceLock::new();
static APP_INDEX_REFRESH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppIndexEntry {
    title: String,
    path: PathBuf,
    source: String,
    aliases: Vec<String>,
    priority: i32,
}

#[derive(Clone, Debug)]
struct CachedAppIndex {
    indexed_at_epoch_secs: u64,
    entries: Vec<AppIndexEntry>,
}

impl CachedAppIndex {
    fn is_fresh(&self, now_epoch_secs: u64) -> bool {
        cache_age_secs(self.indexed_at_epoch_secs, now_epoch_secs) <= APP_INDEX_TTL.as_secs()
    }

    fn age_ms(&self, now_epoch_secs: u64) -> u64 {
        cache_age_secs(self.indexed_at_epoch_secs, now_epoch_secs) * 1_000
    }
}

#[derive(Default)]
struct AppIndexRuntimeState {
    cache: Option<CachedAppIndex>,
    cache_path: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedAppIndexCache {
    version: u32,
    indexed_at_epoch_secs: u64,
    entries: Vec<AppIndexEntry>,
}

impl From<PersistedAppIndexCache> for CachedAppIndex {
    fn from(value: PersistedAppIndexCache) -> Self {
        Self {
            indexed_at_epoch_secs: value.indexed_at_epoch_secs,
            entries: value.entries,
        }
    }
}

impl From<&CachedAppIndex> for PersistedAppIndexCache {
    fn from(value: &CachedAppIndex) -> Self {
        Self {
            version: APP_INDEX_CACHE_VERSION,
            indexed_at_epoch_secs: value.indexed_at_epoch_secs,
            entries: value.entries.clone(),
        }
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

struct CachedAppEntriesSnapshot {
    entries: Vec<AppIndexEntry>,
    cache_state: SearchProviderCacheState,
    cache_age_ms: Option<u64>,
    refresh_needed: bool,
}

pub(crate) fn search_apps(query: &str, limit: usize) -> AppsSearchRun {
    let started_at = crate::search::contracts::iso_now();
    let started = Instant::now();
    let snapshot = cached_app_entries(current_epoch_secs());
    let refresh_needed = snapshot.refresh_needed;
    if refresh_needed {
        warm_app_index_async();
    }
    let results = rank_apps(&snapshot.entries, query, limit);
    let result_count = results.len();
    let health_state = match (snapshot.entries.is_empty(), snapshot.cache_state) {
        (true, SearchProviderCacheState::Disabled) => SearchProviderHealthState::Disabled,
        (true, SearchProviderCacheState::Miss | SearchProviderCacheState::Indexing) => {
            SearchProviderHealthState::Indexing
        }
        _ => SearchProviderHealthState::Ready,
    };

    AppsSearchRun {
        results,
        timing: SearchProviderTiming {
            provider_id: SearchProviderId::Apps,
            started_at,
            ended_at: Some(crate::search::contracts::iso_now()),
            duration_ms: started.elapsed().as_secs_f64() * 1000.0,
            cache: snapshot.cache_state,
            cache_age_ms: snapshot.cache_age_ms,
            result_count,
            applied: true,
            discarded_as_stale: false,
        },
        health: SearchProviderHealth {
            provider_id: SearchProviderId::Apps,
            state: health_state,
            reason_code: None,
            message: Some(cache_health_message(
                snapshot.entries.len(),
                snapshot.cache_state,
                snapshot.cache_age_ms,
            )),
        },
    }
}

pub(crate) fn initialize_app_index_cache(app_handle: &AppHandle) {
    let cache_path = app_index_cache_path(app_handle).ok();
    let persisted_cache =
        cache_path
            .as_ref()
            .and_then(|path| match read_persisted_app_index(path) {
                Ok(cache) => cache,
                Err(error) => {
                    eprintln!(
                        "ignoring corrupt app index cache {}: {error}",
                        path.display()
                    );
                    None
                }
            });

    let runtime = app_index_runtime();
    if let Ok(mut guard) = runtime.lock() {
        guard.cache_path = cache_path;
        guard.cache = persisted_cache;
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
    let indexed_at_epoch_secs = current_epoch_secs();
    let entries = build_app_index(app_roots(), MAX_INDEXED_APPS);
    let cache = CachedAppIndex {
        indexed_at_epoch_secs,
        entries,
    };

    let cache_path = app_index_runtime()
        .lock()
        .ok()
        .and_then(|guard| guard.cache_path.clone());
    if let Some(path) = cache_path.as_ref() {
        let _ = write_persisted_app_index(path, &cache);
    }

    if let Ok(mut guard) = app_index_runtime().lock() {
        guard.cache = Some(cache);
    }
}

fn cached_app_entries(now_epoch_secs: u64) -> CachedAppEntriesSnapshot {
    if let Ok(guard) = app_index_runtime().lock() {
        return cached_app_entries_from_cache(guard.cache.as_ref(), now_epoch_secs);
    }
    CachedAppEntriesSnapshot {
        entries: Vec::new(),
        cache_state: SearchProviderCacheState::Disabled,
        cache_age_ms: None,
        refresh_needed: false,
    }
}

fn cached_app_entries_from_cache(
    cached: Option<&CachedAppIndex>,
    now_epoch_secs: u64,
) -> CachedAppEntriesSnapshot {
    match cached {
        Some(cached) if cached.is_fresh(now_epoch_secs) => CachedAppEntriesSnapshot {
            entries: cached.entries.clone(),
            cache_state: SearchProviderCacheState::Hit,
            cache_age_ms: Some(cached.age_ms(now_epoch_secs)),
            refresh_needed: false,
        },
        Some(cached) => CachedAppEntriesSnapshot {
            entries: cached.entries.clone(),
            cache_state: SearchProviderCacheState::Refresh,
            cache_age_ms: Some(cached.age_ms(now_epoch_secs)),
            refresh_needed: true,
        },
        None if APP_INDEX_REFRESH_IN_FLIGHT.load(Ordering::Acquire) => CachedAppEntriesSnapshot {
            entries: Vec::new(),
            cache_state: SearchProviderCacheState::Indexing,
            cache_age_ms: None,
            refresh_needed: false,
        },
        None => CachedAppEntriesSnapshot {
            entries: Vec::new(),
            cache_state: SearchProviderCacheState::Miss,
            cache_age_ms: None,
            refresh_needed: true,
        },
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
        source: root.source.to_string(),
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
            score_app(entry, &tokens).map(|(score, matched)| app_result(entry, score, matched))
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

fn app_result(entry: &AppIndexEntry, score: i32, matched: MatchData) -> SearchResult {
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
        match_reason: matched.reason.to_string(),
        record_key,
        title_highlight_data: if matched.field == MatchField::Title {
            matched.highlight_data
        } else {
            Vec::new()
        },
        subtitle_highlight_data: Vec::new(),
        icon_data_url: None,
    }
}

fn score_app(entry: &AppIndexEntry, tokens: &[String]) -> Option<(i32, MatchData)> {
    let query = tokens.join(" ");
    let hidden = entry
        .aliases
        .iter()
        .cloned()
        .chain(std::iter::once(entry.path.display().to_string()))
        .collect::<Vec<_>>();
    let matched = best_match(&entry.title, None, &hidden, &query, tokens, true)?;
    Some((entry.priority + matched.score, matched))
}

fn app_aliases(title: &str, path: &Path) -> Vec<String> {
    let mut aliases = vec![title.to_string()];
    let title_acronym = acronym(&normalize(title));
    if title_acronym.len() >= 2 {
        aliases.push(title_acronym.clone());
        aliases.push(
            title_acronym
                .chars()
                .map(|ch| ch.to_string())
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
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
    match_query_tokens(query)
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

fn app_index_runtime() -> &'static Mutex<AppIndexRuntimeState> {
    APP_INDEX_RUNTIME.get_or_init(|| Mutex::new(AppIndexRuntimeState::default()))
}

fn app_index_cache_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    app_handle
        .path()
        .app_local_data_dir()
        .map(|dir| dir.join(APP_INDEX_CACHE_FILE))
        .map_err(|error| format!("failed to resolve app index cache path: {error}"))
}

fn read_persisted_app_index(path: &Path) -> Result<Option<CachedAppIndex>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read persisted app index cache: {error}"))?;
    let persisted = serde_json::from_slice::<PersistedAppIndexCache>(&bytes)
        .map_err(|error| format!("failed to parse persisted app index cache: {error}"))?;
    if persisted.version != APP_INDEX_CACHE_VERSION {
        return Err(format!(
            "unsupported persisted app index cache version {}",
            persisted.version
        ));
    }
    Ok(Some(persisted.into()))
}

fn write_persisted_app_index(path: &Path, cache: &CachedAppIndex) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create app index cache directory: {error}"))?;
    }
    let bytes = serde_json::to_vec(&PersistedAppIndexCache::from(cache))
        .map_err(|error| format!("failed to serialize app index cache: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("failed to write app index cache: {error}"))
}

fn cache_health_message(
    entry_count: usize,
    cache_state: SearchProviderCacheState,
    cache_age_ms: Option<u64>,
) -> String {
    let age_suffix = cache_age_ms
        .map(|age_ms| format!(", age={}ms", age_ms))
        .unwrap_or_default();
    format!("cached app index has {entry_count} rows, state={cache_state:?}{age_suffix}")
}

fn cache_age_secs(indexed_at_epoch_secs: u64, now_epoch_secs: u64) -> u64 {
    now_epoch_secs.saturating_sub(indexed_at_epoch_secs)
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn app_index_cache_freshness_is_bounded_by_ttl() {
        let cache = CachedAppIndex {
            indexed_at_epoch_secs: 100,
            entries: Vec::new(),
        };

        assert!(cache.is_fresh(105));
        assert!(!cache.is_fresh(220));
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
        APP_INDEX_REFRESH_IN_FLIGHT.store(false, Ordering::Release);
        let snapshot = cached_app_entries_from_cache(None, 100);

        assert!(snapshot.entries.is_empty());
        assert_eq!(snapshot.cache_state, SearchProviderCacheState::Miss);
        assert!(snapshot.refresh_needed);
    }

    #[test]
    fn stale_app_cache_returns_existing_rows_while_refresh_is_deferred() {
        let cached = CachedAppIndex {
            indexed_at_epoch_secs: 100,
            entries: vec![AppIndexEntry {
                title: "Spotify".to_string(),
                path: PathBuf::from(r"C:\Apps\Spotify.lnk"),
                source: "test".to_string(),
                aliases: vec!["Spotify".to_string()],
                priority: 1_550,
            }],
        };
        let snapshot = cached_app_entries_from_cache(Some(&cached), 220);

        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.cache_state, SearchProviderCacheState::Refresh);
        assert_eq!(snapshot.cache_age_ms, Some(120_000));
        assert!(snapshot.refresh_needed);
    }

    #[test]
    fn empty_cache_reports_indexing_while_refresh_is_running() {
        APP_INDEX_REFRESH_IN_FLIGHT.store(true, Ordering::Release);

        let snapshot = cached_app_entries_from_cache(None, 100);

        APP_INDEX_REFRESH_IN_FLIGHT.store(false, Ordering::Release);
        assert!(snapshot.entries.is_empty());
        assert_eq!(snapshot.cache_state, SearchProviderCacheState::Indexing);
        assert!(!snapshot.refresh_needed);
    }

    #[test]
    fn persisted_cache_round_trips_non_secret_metadata() {
        let root = test_dir("persisted-cache");
        fs::create_dir_all(&root).unwrap();
        let path = root.join(APP_INDEX_CACHE_FILE);
        let cache = CachedAppIndex {
            indexed_at_epoch_secs: 200,
            entries: vec![AppIndexEntry {
                title: "VS Code".to_string(),
                path: PathBuf::from(r"C:\Apps\Code.exe"),
                source: "windowsApps".to_string(),
                aliases: vec!["Code".to_string(), "VS Code".to_string()],
                priority: 1_500,
            }],
        };

        write_persisted_app_index(&path, &cache).unwrap();
        let loaded = read_persisted_app_index(&path).unwrap().unwrap();

        fs::remove_dir_all(root).ok();
        assert_eq!(loaded.indexed_at_epoch_secs, 200);
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].title, "VS Code");
        assert_eq!(loaded.entries[0].aliases, vec!["Code", "VS Code"]);
    }

    #[test]
    fn corrupt_persisted_cache_is_ignored() {
        let root = test_dir("corrupt-cache");
        fs::create_dir_all(&root).unwrap();
        let path = root.join(APP_INDEX_CACHE_FILE);
        fs::write(&path, b"{not-json").unwrap();

        let read = read_persisted_app_index(&path);

        fs::remove_dir_all(root).ok();
        assert!(read.is_err());
    }

    #[test]
    fn app_results_outrank_incidental_everything_scores() {
        let entry = AppIndexEntry {
            title: "Spotify".to_string(),
            path: PathBuf::from(
                r"C:\Users\me\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Spotify.lnk",
            ),
            source: "testStartMenu".to_string(),
            aliases: vec!["Spotify".to_string()],
            priority: 1_550,
        };

        let result = rank_apps(&[entry], "spotify", 1).pop().unwrap();

        assert!(result.score > 2_000);
    }

    #[test]
    fn launcher_style_vs_code_query_matches_visual_studio_code() {
        let entry = AppIndexEntry {
            title: "Visual Studio Code".to_string(),
            path: PathBuf::from(r"C:\Apps\Code.exe"),
            source: "testStartMenu".to_string(),
            aliases: vec!["Code".to_string(), "Visual Studio Code".to_string()],
            priority: 1_550,
        };

        let result = rank_apps(&[entry], "vs code", 1).pop().unwrap();

        assert_eq!(result.title, "Visual Studio Code");
        assert_eq!(result.match_reason, "tokenPrefix");
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
