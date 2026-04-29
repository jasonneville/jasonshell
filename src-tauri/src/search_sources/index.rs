use super::scoring::normalize;
#[cfg(test)]
use super::scoring::search_ranked_results;
use super::{apps, files, provider, SystemSearchResult};
use provider::ProviderHealthState;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

const CACHE_VERSION: u32 = 1;
const REFRESH_TTL: Duration = Duration::from_secs(300);
const SEARCH_INDEX_REFRESHED_EVENT: &str = "search-index:refreshed";

#[derive(Default)]
pub struct SearchIndexRuntimeState {
    entries: Vec<SystemSearchResult>,
    loaded_cache: bool,
    refreshing: bool,
    refreshed_at: Option<SystemTime>,
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
    _state: &Mutex<SearchIndexRuntimeState>,
    query: &str,
) -> Result<Vec<SystemSearchResult>, String> {
    ensure_refresh(app_handle, false);
    if !provider_query_key(query).is_empty() {
        let settings = crate::settings::load_shell_settings_for_app(app_handle)
            .unwrap_or_else(|_| crate::settings::ShellSettings::default());
        let batch = provider::search_provider_results(query, &settings);
        let everything_ready = batch.health.iter().any(|health| {
            health.provider_id == provider::SearchProviderId::Everything
                && health.state == ProviderHealthState::Ready
        });
        if everything_ready || !batch.results.is_empty() {
            return Ok(batch.results);
        }
        return Ok(Vec::new());
    }

    Ok(Vec::new())
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

fn provider_query_key(query: &str) -> String {
    normalize(query)
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
    fn provider_query_key_matches_cairo_style_normalization() {
        assert_eq!(
            provider_query_key("  Visual-Studio.Code  "),
            "visual studio code"
        );
    }

    #[test]
    fn refreshed_event_payload_reports_cache_generation() {
        let payload = search_index_refreshed_payload(42, 1_773_910_800);

        assert_eq!(payload.entry_count, 42);
        assert_eq!(payload.generated_at_epoch_secs, 1_773_910_800);
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
