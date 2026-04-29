use super::scoring::{normalize, search_ranked_results};
use super::{apps, files, provider, SystemSearchResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

const CACHE_VERSION: u32 = 1;
const INDEX_LIMIT: usize = 40;
const REFRESH_TTL: Duration = Duration::from_secs(300);
const PROVIDER_EMPTY_TTL: Duration = Duration::from_secs(5);
const PROVIDER_RESULTS_TTL: Duration = Duration::from_secs(60);
const MAX_PROVIDER_CACHE_QUERIES: usize = 64;
const MAX_PROVIDER_IN_FLIGHT: usize = 4;
const SEARCH_INDEX_REFRESHED_EVENT: &str = "search-index:refreshed";

#[derive(Default)]
pub struct SearchIndexRuntimeState {
    entries: Vec<SystemSearchResult>,
    provider_results_by_query: HashMap<String, CachedProviderResults>,
    provider_queries_in_flight: HashSet<String>,
    latest_provider_generation: u64,
    next_provider_generation: u64,
    loaded_cache: bool,
    refreshing: bool,
    refreshed_at: Option<SystemTime>,
}

#[derive(Clone, Debug)]
struct CachedProviderResults {
    results: Vec<SystemSearchResult>,
    stored_at: SystemTime,
}

#[derive(Deserialize, Serialize)]
struct SearchIndexCache {
    version: u32,
    generated_at_epoch_secs: u64,
    entries: Vec<SystemSearchResult>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchIndexRefreshedPayload {
    entry_count: usize,
    generated_at_epoch_secs: u64,
}

pub fn warm_search_index(app_handle: AppHandle) {
    ensure_refresh(&app_handle, false);
}

pub fn search_index(
    app_handle: &AppHandle,
    state: &Mutex<SearchIndexRuntimeState>,
    query: &str,
) -> Result<Vec<SystemSearchResult>, String> {
    ensure_refresh(app_handle, false);
    ensure_provider_search(app_handle, query);

    let guard = state
        .lock()
        .map_err(|_| "Search index state is unavailable".to_string())?;
    Ok(search_ranked_snapshot(
        &guard.entries,
        guard
            .provider_results_by_query
            .get(&provider_query_key(query)),
        query,
        INDEX_LIMIT,
    ))
}

fn ensure_refresh(app_handle: &AppHandle, force: bool) {
    let state = app_handle.state::<Mutex<SearchIndexRuntimeState>>();
    let should_refresh = {
        let Ok(mut guard) = state.lock() else {
            return;
        };
        let stale = guard
            .refreshed_at
            .and_then(|timestamp| timestamp.elapsed().ok())
            .map(|age| age > REFRESH_TTL)
            .unwrap_or(true);

        if guard.refreshing || (!force && !stale && !guard.entries.is_empty()) {
            false
        } else {
            guard.refreshing = true;
            true
        }
    };

    if !should_refresh {
        return;
    }

    let app_handle = app_handle.clone();
    thread::spawn(move || {
        if let Some(entry_count) = load_cache_for_refresh(&app_handle) {
            let _ = app_handle.emit(
                SEARCH_INDEX_REFRESHED_EVENT,
                search_index_refreshed_payload(entry_count, current_epoch_secs()),
            );
        }

        let entries = build_index();
        let entry_count = entries.len();
        if let Some(path) = cache_path(&app_handle) {
            let _ = write_cache(&path, &entries);
        }

        let generated_at_epoch_secs = current_epoch_secs();
        let state = app_handle.state::<Mutex<SearchIndexRuntimeState>>();
        if let Ok(mut guard) = state.lock() {
            guard.entries = entries;
            guard.loaded_cache = true;
            guard.refreshing = false;
            guard.refreshed_at = Some(SystemTime::now());
        };

        let _ = app_handle.emit(
            SEARCH_INDEX_REFRESHED_EVENT,
            search_index_refreshed_payload(entry_count, generated_at_epoch_secs),
        );
    });
}

fn ensure_provider_search(app_handle: &AppHandle, query: &str) {
    let key = provider_query_key(query);
    if key.len() < 2 {
        return;
    }

    let state = app_handle.state::<Mutex<SearchIndexRuntimeState>>();
    let should_search = {
        let Ok(mut guard) = state.lock() else {
            return;
        };
        prune_provider_cache(&mut guard.provider_results_by_query, SystemTime::now());
        if guard.provider_queries_in_flight.contains(&key)
            || guard.provider_queries_in_flight.len() >= MAX_PROVIDER_IN_FLIGHT
            || guard
                .provider_results_by_query
                .get(&key)
                .map(|cached| provider_cache_is_fresh(cached, SystemTime::now()))
                .unwrap_or(false)
        {
            None
        } else {
            guard.next_provider_generation = guard.next_provider_generation.saturating_add(1);
            let generation = guard.next_provider_generation;
            guard.latest_provider_generation = generation;
            guard.provider_queries_in_flight.insert(key.clone());
            Some(generation)
        }
    };

    let Some(generation) = should_search else {
        return;
    };

    let app_handle = app_handle.clone();
    let query = query.trim().to_string();
    thread::spawn(move || {
        let settings = crate::settings::load_shell_settings_for_app(&app_handle)
            .unwrap_or_else(|_| crate::settings::ShellSettings::default());
        let batch = provider::search_provider_results(&query, &settings);
        let results = batch.results;
        let entry_count = results.len();
        let state = app_handle.state::<Mutex<SearchIndexRuntimeState>>();
        if let Ok(mut guard) = state.lock() {
            guard.provider_queries_in_flight.remove(&key);
            if !provider::should_apply_provider_generation(
                generation,
                guard.latest_provider_generation,
            ) {
                return;
            }
            prune_provider_cache(&mut guard.provider_results_by_query, SystemTime::now());
            guard.provider_results_by_query.insert(
                key,
                CachedProviderResults {
                    results,
                    stored_at: SystemTime::now(),
                },
            );
        }

        if entry_count > 0 {
            let _ = app_handle.emit(
                SEARCH_INDEX_REFRESHED_EVENT,
                search_index_refreshed_payload(entry_count, current_epoch_secs()),
            );
        }
    });
}

fn search_ranked_snapshot(
    entries: &[SystemSearchResult],
    provider_results: Option<&CachedProviderResults>,
    query: &str,
    limit: usize,
) -> Vec<SystemSearchResult> {
    let local_results = search_ranked_results(entries, query, limit);
    let Some(provider_results) = provider_results else {
        return local_results;
    };

    merge_provider_and_local_results(&provider_results.results, local_results, limit)
}

fn merge_provider_and_local_results(
    provider_results: &[SystemSearchResult],
    local_results: Vec<SystemSearchResult>,
    limit: usize,
) -> Vec<SystemSearchResult> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for result in provider_results.iter().cloned().chain(local_results) {
        if seen.insert(result_identity_key(&result)) {
            merged.push(result);
        }
    }

    merged.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then(left.title.to_lowercase().cmp(&right.title.to_lowercase()))
    });
    merged.truncate(limit);
    merged
}

fn result_identity_key(result: &SystemSearchResult) -> String {
    if !result.path.trim().is_empty() {
        return format!(
            "{}:{}",
            result.kind,
            normalize(&result.path.replace('/', r"\"))
        );
    }
    normalize(&result.id)
}

fn provider_query_key(query: &str) -> String {
    normalize(query)
}

fn provider_cache_is_fresh(cached: &CachedProviderResults, now: SystemTime) -> bool {
    let ttl = if cached.results.is_empty() {
        PROVIDER_EMPTY_TTL
    } else {
        PROVIDER_RESULTS_TTL
    };
    now.duration_since(cached.stored_at)
        .map(|age| age < ttl)
        .unwrap_or(true)
}

fn prune_provider_cache(cache: &mut HashMap<String, CachedProviderResults>, now: SystemTime) {
    cache.retain(|_, cached| provider_cache_is_fresh(cached, now));
    while cache.len() >= MAX_PROVIDER_CACHE_QUERIES {
        let Some(oldest_key) = cache
            .iter()
            .min_by_key(|(_, cached)| cached.stored_at)
            .map(|(key, _)| key.clone())
        else {
            break;
        };
        cache.remove(&oldest_key);
    }
}

fn load_cache_for_refresh(app_handle: &AppHandle) -> Option<usize> {
    let state = app_handle.state::<Mutex<SearchIndexRuntimeState>>();
    if state.lock().map(|guard| guard.loaded_cache).unwrap_or(true) {
        return None;
    }

    let cached = cache_path(app_handle)
        .as_deref()
        .and_then(read_cache)
        .unwrap_or_default();
    let entry_count = cached.len();

    if let Ok(mut guard) = state.lock() {
        if !guard.loaded_cache {
            if guard.entries.is_empty() {
                guard.entries = cached;
            }
            guard.loaded_cache = true;
        }
    };

    (entry_count > 0).then_some(entry_count)
}

fn build_index() -> Vec<SystemSearchResult> {
    let mut seen = HashSet::new();
    let mut entries = Vec::new();

    for entry in apps::index_apps()
        .into_iter()
        .chain(files::index_files().into_iter())
    {
        let key = entry.id.to_lowercase();
        if seen.insert(key) {
            entries.push(entry);
        }
    }

    entries.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then(left.title.cmp(&right.title))
    });
    entries
}

fn cache_path(app_handle: &AppHandle) -> Option<PathBuf> {
    app_handle
        .path()
        .app_local_data_dir()
        .ok()
        .map(|dir| dir.join("search-index-v1.json"))
}

fn read_cache(path: &Path) -> Option<Vec<SystemSearchResult>> {
    let bytes = fs::read(path).ok()?;
    let cache = serde_json::from_slice::<SearchIndexCache>(&bytes).ok()?;
    (cache.version == CACHE_VERSION).then_some(cache.entries)
}

fn write_cache(path: &Path, entries: &[SystemSearchResult]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create search index cache directory: {error}"))?;
    }

    let generated_at_epoch_secs = current_epoch_secs();
    let cache = SearchIndexCache {
        version: CACHE_VERSION,
        generated_at_epoch_secs,
        entries: entries.to_vec(),
    };
    let bytes = serde_json::to_vec(&cache)
        .map_err(|error| format!("Failed to serialize search index cache: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("Failed to write search index cache: {error}"))
}

fn current_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn search_index_refreshed_payload(
    entry_count: usize,
    generated_at_epoch_secs: u64,
) -> SearchIndexRefreshedPayload {
    SearchIndexRefreshedPayload {
        entry_count,
        generated_at_epoch_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn cached_index_round_trips_file_and_app_results() {
        let root = test_dir("cache");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("index.json");
        let entries = vec![
            SystemSearchResult {
                id: "system:app:C:\\Tools\\DevBox.lnk".to_string(),
                provider_id: Some("apps".to_string()),
                kind: "app".to_string(),
                title: "DevBox".to_string(),
                subtitle: "Installed app - Start Menu".to_string(),
                terms: "devbox installed program".to_string(),
                priority: 112,
                path: "C:\\Tools\\DevBox.lnk".to_string(),
                record_key: Some("app:c:\\tools\\devbox.lnk".to_string()),
                run_count: None,
                top_most: None,
            },
            SystemSearchResult {
                id: "system:folder:C:\\Users\\me\\Documents\\Plans".to_string(),
                provider_id: Some("warmedCache".to_string()),
                kind: "folder".to_string(),
                title: "Plans".to_string(),
                subtitle: "Folder - Documents".to_string(),
                terms: "plans folder".to_string(),
                priority: 76,
                path: "C:\\Users\\me\\Documents\\Plans".to_string(),
                record_key: Some("folder:c:\\users\\me\\documents\\plans".to_string()),
                run_count: None,
                top_most: None,
            },
        ];

        write_cache(&path, &entries).unwrap();
        let loaded = read_cache(&path).unwrap();

        fs::remove_dir_all(root).ok();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].kind, "app");
        assert_eq!(loaded[1].kind, "folder");
    }

    #[test]
    fn query_uses_cached_entries() {
        let entries = vec![SystemSearchResult {
            id: "system:file:C:\\Users\\me\\Downloads\\Invoice.pdf".to_string(),
            provider_id: Some("warmedCache".to_string()),
            kind: "file".to_string(),
            title: "Invoice".to_string(),
            subtitle: "File - Downloads".to_string(),
            terms: "invoice pdf downloads".to_string(),
            priority: 76,
            path: "C:\\Users\\me\\Downloads\\Invoice.pdf".to_string(),
            record_key: Some("file:c:\\users\\me\\downloads\\invoice.pdf".to_string()),
            run_count: None,
            top_most: None,
        }];

        let results = search_ranked_results(&entries, "invoice", 8);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, "file");
    }

    #[test]
    fn provider_results_merge_with_local_snapshot_without_duplicates() {
        let provider_results = CachedProviderResults {
            results: vec![
                search_result("system:app:C:\\Apps\\Terminal.lnk", "app", "Terminal", 118),
                search_result("system:file:C:\\Docs\\Plan.docx", "file", "Plan", 80),
            ],
            stored_at: SystemTime::now(),
        };
        let local_results = vec![
            search_result("system:app:C:\\Apps\\Terminal.lnk", "app", "Terminal", 150),
            search_result("system:file:C:\\Docs\\Notes.txt", "file", "Notes", 90),
        ];

        let results = merge_provider_and_local_results(&provider_results.results, local_results, 8);

        assert_eq!(results.len(), 3);
        assert_eq!(
            results
                .iter()
                .filter(|result| result.id == "system:app:C:\\Apps\\Terminal.lnk")
                .count(),
            1
        );
        assert!(results.iter().any(|result| result.title == "Notes"));
    }

    #[test]
    fn provider_results_collapse_same_path_with_different_id_shapes() {
        let provider_results = vec![search_result(
            "system:file:C:\\Docs\\Plan.docx",
            "file",
            "Plan",
            120,
        )];
        let local_results = vec![search_result(
            "system:file:c:/docs/plan.docx",
            "file",
            "Plan Local",
            90,
        )];

        let results = merge_provider_and_local_results(&provider_results, local_results, 8);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Plan");
    }

    #[test]
    fn provider_query_key_matches_cairo_style_normalization() {
        assert_eq!(
            provider_query_key("  Visual-Studio.Code  "),
            "visual studio code"
        );
    }

    #[test]
    fn provider_empty_results_expire_quickly_for_retry() {
        let now = SystemTime::now();
        let fresh_empty = CachedProviderResults {
            results: Vec::new(),
            stored_at: now - Duration::from_secs(2),
        };
        let stale_empty = CachedProviderResults {
            results: Vec::new(),
            stored_at: now - Duration::from_secs(6),
        };

        assert!(provider_cache_is_fresh(&fresh_empty, now));
        assert!(!provider_cache_is_fresh(&stale_empty, now));
    }

    #[test]
    fn provider_positive_results_have_longer_ttl() {
        let now = SystemTime::now();
        let fresh_positive = CachedProviderResults {
            results: vec![search_result(
                "system:file:C:\\Docs\\Plan.docx",
                "file",
                "Plan",
                80,
            )],
            stored_at: now - Duration::from_secs(45),
        };
        let stale_positive = CachedProviderResults {
            results: vec![search_result(
                "system:file:C:\\Docs\\Notes.txt",
                "file",
                "Notes",
                80,
            )],
            stored_at: now - Duration::from_secs(61),
        };

        assert!(provider_cache_is_fresh(&fresh_positive, now));
        assert!(!provider_cache_is_fresh(&stale_positive, now));
    }

    #[test]
    fn provider_cache_prunes_expired_and_bounds_query_count() {
        let now = SystemTime::now();
        let mut cache = HashMap::new();
        cache.insert(
            "expired".to_string(),
            CachedProviderResults {
                results: Vec::new(),
                stored_at: now - Duration::from_secs(10),
            },
        );
        for index in 0..MAX_PROVIDER_CACHE_QUERIES {
            cache.insert(
                format!("query-{index}"),
                CachedProviderResults {
                    results: vec![search_result(
                        &format!("system:file:C:\\Docs\\File{index}.txt"),
                        "file",
                        &format!("File{index}"),
                        80,
                    )],
                    stored_at: now - Duration::from_secs(index as u64),
                },
            );
        }

        prune_provider_cache(&mut cache, now);

        assert!(!cache.contains_key("expired"));
        assert!(cache.len() < MAX_PROVIDER_CACHE_QUERIES);
    }

    #[test]
    fn refreshed_event_payload_reports_cache_generation() {
        let payload = search_index_refreshed_payload(42, 1_773_910_800);

        assert_eq!(payload.entry_count, 42);
        assert_eq!(payload.generated_at_epoch_secs, 1_773_910_800);
    }

    fn search_result(id: &str, kind: &str, title: &str, priority: i32) -> SystemSearchResult {
        SystemSearchResult {
            id: id.to_string(),
            provider_id: Some("everything".to_string()),
            kind: kind.to_string(),
            title: title.to_string(),
            subtitle: title.to_string(),
            terms: title.to_string(),
            priority,
            path: id.replace(&format!("system:{kind}:"), ""),
            record_key: None,
            run_count: None,
            top_most: None,
        }
    }

    fn test_dir(name: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("search-index-{name}-{id}"))
    }
}
