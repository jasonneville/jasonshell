use std::path::Path;

pub(crate) fn icon_data_url_for_path(path: &Path) -> Option<String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
        return None;
    }

    #[cfg(target_os = "windows")]
    {
        windows_cache::windows_icon_data_url_for_path(path)
    }
}

#[cfg(target_os = "windows")]
mod windows_cache {
    use super::normalize_icon_cache_key;
    use crate::task_windows::bounded_string_cache::BoundedStringCache;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    static SEARCH_ICON_CACHE: OnceLock<Mutex<BoundedStringCache<String>>> = OnceLock::new();
    const SEARCH_ICON_CACHE_CAPACITY: usize = 128;
    const SEARCH_ICON_CACHE_TTL: Duration = Duration::from_secs(30 * 60);
    const SEARCH_ICON_CACHE_NEGATIVE_TTL: Duration = Duration::from_secs(30);

    pub(super) fn windows_icon_data_url_for_path(path: &Path) -> Option<String> {
        let key = normalize_icon_cache_key(path)?;
        let cache = SEARCH_ICON_CACHE.get_or_init(|| {
            Mutex::new(BoundedStringCache::new(
                SEARCH_ICON_CACHE_CAPACITY,
                SEARCH_ICON_CACHE_TTL,
                SEARCH_ICON_CACHE_NEGATIVE_TTL,
            ))
        });
        let cached = cache
            .lock()
            .ok()
            .and_then(|mut cache| cache.get_cloned(&key));
        if let Some(cached) = cached {
            return cached;
        }
        let resolved = super::resolve_icon_data_url(path);
        if let Ok(mut cache) = cache.lock() {
            cache.insert(key, resolved.clone());
        }
        resolved
    }
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
