use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static SEARCH_ICON_CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();

pub(crate) fn icon_data_url_for_path(path: &Path) -> Option<String> {
    let key = normalize_icon_cache_key(path)?;
    let cache = SEARCH_ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut cache) = cache.lock() else {
        return resolve_icon_data_url(path);
    };
    if let Some(cached) = cache.get(&key) {
        return cached.clone();
    }

    let resolved = resolve_icon_data_url(path);
    cache.insert(key, resolved.clone());
    resolved
}

fn normalize_icon_cache_key(path: &Path) -> Option<String> {
    let as_text = path.as_os_str().to_string_lossy().trim().to_string();
    if as_text.is_empty() {
        return None;
    }
    Some(as_text.replace('/', r"\").to_lowercase())
}

#[cfg(target_os = "windows")]
fn resolve_icon_data_url(path: &Path) -> Option<String> {
    crate::task_windows::shell_file_icon_data_url(path).ok()
}

#[cfg(not(target_os = "windows"))]
fn resolve_icon_data_url(_path: &Path) -> Option<String> {
    None
}
