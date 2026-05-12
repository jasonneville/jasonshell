use crate::stack_popup::models::{StackItemIconResolution, StackItemIconResolutionBatch};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub(crate) const STACK_ICON_RESOLVE_BATCH_LIMIT: usize = 24;

static STACK_POPUP_ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

pub(crate) fn resolve_stack_item_icons_for_paths(
    paths: Vec<String>,
) -> Result<StackItemIconResolutionBatch, String> {
    let started = Instant::now();
    let requested_count = paths.len();
    let batch = resolve_stack_item_icons_batch(paths, STACK_ICON_RESOLVE_BATCH_LIMIT);
    let truncated = batch.len() < requested_count;

    let mut cache_hits = 0usize;
    let mut cache_misses = 0usize;
    let mut items = Vec::with_capacity(batch.len());

    for path in batch {
        let resolution = resolve_stack_item_icon(&path);
        if resolution.cache_hit {
            cache_hits += 1;
        } else {
            cache_misses += 1;
        }
        items.push(resolution);
    }

    Ok(StackItemIconResolutionBatch {
        requested_count,
        resolved_count: items.len(),
        cache_hits,
        cache_misses,
        truncated,
        max_batch_size: STACK_ICON_RESOLVE_BATCH_LIMIT,
        total_duration_ms: started.elapsed().as_millis(),
        items,
    })
}

pub(crate) async fn resolve_stack_item_icons_for_paths_async(
    paths: Vec<String>,
) -> Result<StackItemIconResolutionBatch, String> {
    tauri::async_runtime::spawn_blocking(move || resolve_stack_item_icons_for_paths(paths))
        .await
        .map_err(|error| format!("Failed to join stack icon resolver: {error}"))?
}

pub(crate) fn resolve_stack_item_icons_batch(
    paths: Vec<String>,
    max_batch_size: usize,
) -> Vec<String> {
    let bounded_limit = max_batch_size.max(1);
    let mut seen = HashSet::new();
    let mut batch = Vec::with_capacity(paths.len().min(bounded_limit));

    for raw_path in paths {
        if batch.len() >= bounded_limit {
            break;
        }
        let normalized = normalize_icon_cache_key(&raw_path);
        if normalized.is_empty() || !seen.insert(normalized) {
            continue;
        }
        batch.push(raw_path);
    }

    batch
}

fn resolve_stack_item_icon(path: &str) -> StackItemIconResolution {
    let trimmed_path = path.trim().to_string();
    let started = Instant::now();
    if trimmed_path.is_empty() {
        return StackItemIconResolution {
            path: path.to_string(),
            icon_data_url: None,
            cache_hit: false,
            resolution_duration_ms: started.elapsed().as_millis(),
        };
    }

    let cache_key = normalize_icon_cache_key(&trimmed_path);
    let (icon_data_url, cache_hit) = if let Some(cached) = cached_stack_icon_lookup(&cache_key) {
        (cached, true)
    } else {
        let icon_data_url = resolve_shell_icon_data_url(&trimmed_path);
        store_stack_icon_cache_result(cache_key, icon_data_url.clone());
        (icon_data_url, false)
    };

    StackItemIconResolution {
        path: trimmed_path,
        icon_data_url,
        cache_hit,
        resolution_duration_ms: started.elapsed().as_millis(),
    }
}

fn cached_stack_icon_lookup(cache_key: &str) -> Option<Option<String>> {
    let cache = STACK_POPUP_ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    cache
        .lock()
        .ok()
        .and_then(|cache_guard| cache_guard.get(cache_key).cloned())
}

fn store_stack_icon_cache_result(cache_key: String, icon_data_url: Option<String>) {
    let cache = STACK_POPUP_ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut cache_guard) = cache.lock() {
        cache_guard.insert(cache_key, icon_data_url);
    }
}

fn normalize_icon_cache_key(path: &str) -> String {
    path.trim().replace('/', "\\").to_lowercase()
}

#[cfg(target_os = "windows")]
fn resolve_shell_icon_data_url(path: &str) -> Option<String> {
    crate::task_windows::shell_file_icon_data_url(Path::new(path)).ok()
}

#[cfg(not(target_os = "windows"))]
fn resolve_shell_icon_data_url(_path: &str) -> Option<String> {
    None
}
