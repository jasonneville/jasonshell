use crate::search::contracts::{
    SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState, SearchProviderId,
    SearchProviderReasonCode, SearchProviderTiming, SearchResult, SearchResultAction,
    SearchResultKind,
};
use crate::search::icons::icon_data_url_for_path;
use crate::search_sources::everything_ffi::{
    self, EverythingSdkError, EverythingSdkRawResult, EverythingSdkRequest, EverythingSdkResultKind,
};
use crate::search_sources::everything_install;
use crate::settings::EverythingSortMode;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const EVERYTHING_HEALTH_TTL: Duration = Duration::from_secs(30);
const EVERYTHING_MAX_QUERY_RESULTS: usize = 200;
const EVERYTHING_OVERFETCH: usize = 25;
const FOLDER_NAVIGATION_TERMS: &[&str] = &[
    "dev",
    "desktop",
    "downloads",
    "documents",
    "docs",
    "home",
    "profile",
    "repo",
    "repos",
    "projects",
];

static EVERYTHING_STATE: OnceLock<Mutex<Option<CachedEverythingState>>> = OnceLock::new();

#[derive(Clone, Debug)]
struct CachedEverythingState {
    refreshed_at: Instant,
    dll_path: Option<PathBuf>,
    health: SearchProviderHealth,
}

impl CachedEverythingState {
    fn is_fresh(&self, now: Instant) -> bool {
        now.duration_since(self.refreshed_at) <= EVERYTHING_HEALTH_TTL
    }
}

#[derive(Clone, Debug)]
struct EverythingDetection {
    dll_path: Option<PathBuf>,
    installed_exe_path: Option<PathBuf>,
    process_running: bool,
    service_running: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EverythingQueryMode {
    SimpleName,
    PathLike,
    FolderNavigation,
    AppLike,
}

#[derive(Clone, Debug)]
pub(crate) struct EverythingSearchRun {
    pub(crate) results: Vec<SearchResult>,
    pub(crate) timing: SearchProviderTiming,
    pub(crate) health: SearchProviderHealth,
    sdk_latency_ms: Option<f64>,
}

pub(crate) fn search_everything(query: &str, limit: usize) -> EverythingSearchRun {
    search_everything_with(query, limit, detect_everything, run_everything_query)
}

fn search_everything_with<D, R>(
    query: &str,
    limit: usize,
    detect: D,
    run_query: R,
) -> EverythingSearchRun
where
    D: FnOnce() -> EverythingDetection,
    R: Fn(&Path, &EverythingSdkRequest) -> Result<Vec<EverythingSdkRawResult>, EverythingSdkError>,
{
    let started_at = crate::search::contracts::iso_now();
    let started = Instant::now();
    let normalized_query = query.trim();
    let (state, cache_state) = cached_everything_state(Instant::now(), detect);

    let mut health = state.health.clone();
    let mut results = Vec::new();
    let mut sdk_latency_ms = None;
    if !normalized_query.is_empty() {
        if let Some(dll_path) = state.dll_path.clone() {
            let mode = classify_everything_query(normalized_query);
            let request = everything_request_for_mode(normalized_query, limit, mode);
            let sdk_started = Instant::now();
            match run_query(&dll_path, &request) {
                Ok(raw_results) => {
                    results = raw_results
                        .iter()
                        .map(|result| map_sdk_result_for_mode(result, mode, normalized_query))
                        .take(limit)
                        .collect();
                }
                Err(error) => {
                    health = health_for_error(&error);
                }
            }
            sdk_latency_ms = Some(sdk_started.elapsed().as_secs_f64() * 1000.0);
        }
    }

    let result_count = results.len();
    EverythingSearchRun {
        results,
        timing: SearchProviderTiming {
            provider_id: SearchProviderId::Everything,
            started_at,
            ended_at: Some(crate::search::contracts::iso_now()),
            duration_ms: started.elapsed().as_secs_f64() * 1000.0,
            cache: cache_state,
            cache_age_ms: None,
            result_count,
            applied: true,
            discarded_as_stale: false,
        },
        health,
        sdk_latency_ms,
    }
}

fn cached_everything_state<D>(
    now: Instant,
    detect: D,
) -> (CachedEverythingState, SearchProviderCacheState)
where
    D: FnOnce() -> EverythingDetection,
{
    let cache = EVERYTHING_STATE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.as_ref().filter(|cached| cached.is_fresh(now)) {
            return (cached.clone(), SearchProviderCacheState::Hit);
        }
    }

    let detected = detect();
    let state = CachedEverythingState {
        refreshed_at: now,
        dll_path: detected.dll_path.clone(),
        health: health_from_detection(&detected),
    };
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(state.clone());
    }
    (state, SearchProviderCacheState::Refresh)
}

fn detect_everything() -> EverythingDetection {
    let install = everything_install::detect_everything_installation();
    let sdk = everything_ffi::detect_system_sdk(install.installed_exe_path.as_deref());
    EverythingDetection {
        dll_path: sdk.dll_path,
        installed_exe_path: install.installed_exe_path,
        process_running: install.process_running,
        service_running: install.service_running,
    }
}

fn health_from_detection(detection: &EverythingDetection) -> SearchProviderHealth {
    if detection.dll_path.is_none() {
        return SearchProviderHealth {
            provider_id: SearchProviderId::Everything,
            state: SearchProviderHealthState::Degraded,
            reason_code: Some(SearchProviderReasonCode::SdkMissing),
            message: Some(everything_ffi::sdk_missing_message().to_string()),
        };
    }
    if !detection.process_running {
        return SearchProviderHealth {
            provider_id: SearchProviderId::Everything,
            state: if detection.installed_exe_path.is_some() {
                SearchProviderHealthState::Degraded
            } else {
                SearchProviderHealthState::Unavailable
            },
            reason_code: Some(SearchProviderReasonCode::IpcUnavailable),
            message: Some("Everything process is not running".to_string()),
        };
    }
    if !detection.service_running {
        return SearchProviderHealth {
            provider_id: SearchProviderId::Everything,
            state: SearchProviderHealthState::Degraded,
            reason_code: Some(SearchProviderReasonCode::IpcUnavailable),
            message: Some("Everything service is not reported as running".to_string()),
        };
    }

    SearchProviderHealth {
        provider_id: SearchProviderId::Everything,
        state: SearchProviderHealthState::Ready,
        reason_code: None,
        message: Some("Everything SDK path and runtime health are cached".to_string()),
    }
}

fn run_everything_query(
    dll_path: &Path,
    request: &EverythingSdkRequest,
) -> Result<Vec<EverythingSdkRawResult>, EverythingSdkError> {
    let _guard = EVERYTHING_QUERY_LOCK.lock().map_err(|_| {
        EverythingSdkError::QueryFailed("Everything provider lock failed".to_string())
    })?;
    if request.query.trim().is_empty() {
        return Ok(Vec::new());
    }
    if request.content_search_enabled {
        return Err(EverythingSdkError::QueryFailed(
            "Everything content search is disabled for realtime search".to_string(),
        ));
    }
    everything_ffi::query_everything_sdk(dll_path, request)
}

static EVERYTHING_QUERY_LOCK: Mutex<()> = Mutex::new(());

fn health_for_error(error: &EverythingSdkError) -> SearchProviderHealth {
    let reason_code = match error {
        EverythingSdkError::IpcUnavailable => SearchProviderReasonCode::IpcUnavailable,
        EverythingSdkError::QueryFailed(_) => SearchProviderReasonCode::ProviderError,
    };
    SearchProviderHealth {
        provider_id: SearchProviderId::Everything,
        state: SearchProviderHealthState::Degraded,
        reason_code: Some(reason_code),
        message: Some(format!("Everything query failed: {error:?}")),
    }
}

fn everything_request(query: &str, limit: usize) -> EverythingSdkRequest {
    everything_request_for_mode(query, limit, classify_everything_query(query))
}

fn everything_request_for_mode(
    query: &str,
    limit: usize,
    mode: EverythingQueryMode,
) -> EverythingSdkRequest {
    EverythingSdkRequest {
        query: query.to_string(),
        max_results: bounded_everything_limit(limit),
        full_path_search: mode == EverythingQueryMode::PathLike,
        sort: everything_sort_for_mode(mode),
        content_search_enabled: false,
    }
}

fn everything_sort_for_mode(mode: EverythingQueryMode) -> EverythingSortMode {
    match mode {
        EverythingQueryMode::PathLike => EverythingSortMode::PathAsc,
        EverythingQueryMode::AppLike => EverythingSortMode::RunCountDesc,
        EverythingQueryMode::SimpleName | EverythingQueryMode::FolderNavigation => {
            EverythingSortMode::NameAsc
        }
    }
}

fn classify_everything_query(query: &str) -> EverythingQueryMode {
    let trimmed = query.trim();
    let normalized = normalize(trimmed);
    let tokens = normalized
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if is_path_like_query(trimmed) {
        return EverythingQueryMode::PathLike;
    }
    if tokens
        .iter()
        .any(|token| FOLDER_NAVIGATION_TERMS.contains(token))
    {
        return EverythingQueryMode::FolderNavigation;
    }
    if is_app_like_query(trimmed, &tokens) {
        return EverythingQueryMode::AppLike;
    }
    EverythingQueryMode::SimpleName
}

fn is_path_like_query(query: &str) -> bool {
    let trimmed = query.trim();
    trimmed.contains('\\')
        || trimmed.contains('/')
        || trimmed.starts_with('~')
        || trimmed.starts_with('.')
        || trimmed.chars().nth(1).map(|ch| ch == ':').unwrap_or(false)
}

fn is_app_like_query(query: &str, tokens: &[&str]) -> bool {
    let lower = query.to_ascii_lowercase();
    if lower.ends_with(".exe") || lower.ends_with(".lnk") {
        return true;
    }
    if tokens
        .iter()
        .any(|token| matches!(*token, "app" | "apps" | "program" | "programs" | "launch"))
    {
        return true;
    }
    if tokens.is_empty() || tokens.len() > 3 {
        return false;
    }
    if tokens
        .iter()
        .any(|token| token.chars().any(|ch| ch.is_ascii_digit()))
    {
        return false;
    }

    tokens
        .iter()
        .all(|token| token.chars().all(|ch| ch.is_ascii_alphabetic()))
}

fn bounded_everything_limit(limit: usize) -> usize {
    limit
        .saturating_add(EVERYTHING_OVERFETCH)
        .clamp(1, EVERYTHING_MAX_QUERY_RESULTS)
}

fn map_sdk_result(result: &EverythingSdkRawResult) -> SearchResult {
    map_sdk_result_for_mode(result, EverythingQueryMode::SimpleName, "")
}

fn map_sdk_result_for_mode(
    result: &EverythingSdkRawResult,
    mode: EverythingQueryMode,
    query: &str,
) -> SearchResult {
    let kind = match result.kind {
        EverythingSdkResultKind::File if is_app_candidate(&result.full_path) => {
            SearchResultKind::App
        }
        EverythingSdkResultKind::File => SearchResultKind::File,
        EverythingSdkResultKind::Folder | EverythingSdkResultKind::Volume => {
            SearchResultKind::Folder
        }
    };
    let path = result.full_path.display().to_string();
    let kind_label = result_kind_label(kind);
    let title = display_name(&result.full_path, kind);
    let action = match kind {
        SearchResultKind::App => SearchResultAction::OpenApp { path: path.clone() },
        SearchResultKind::Folder => SearchResultAction::OpenFolder { path: path.clone() },
        _ => SearchResultAction::OpenFile { path: path.clone() },
    };
    let subtitle = result
        .full_path
        .parent()
        .map(|parent| format!("{kind_label} - {}", parent.display()))
        .unwrap_or_else(|| kind_label.to_string());
    let highlight_terms = result
        .highlighted_file_name
        .as_deref()
        .map(parse_everything_highlight_indexes)
        .unwrap_or_default()
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    let legacy_kind = legacy_kind(kind);
    SearchResult {
        id: format!("everything:{legacy_kind}:{}", normalize_record_key(&path)),
        provider_id: SearchProviderId::Everything,
        kind,
        title: title.clone(),
        subtitle: Some(subtitle),
        path: Some(path.clone()),
        action,
        terms: token_terms(&format!(
            "{title} {path} everything voidtools local filesystem {highlight_terms}"
        )),
        aliases: Vec::new(),
        score: score_everything_result(result, kind, mode, query),
        match_reason: "everythingName".to_string(),
        record_key: format!("everything:{legacy_kind}:{}", normalize_record_key(&path)),
        title_highlight_data: result
            .highlighted_file_name
            .as_deref()
            .map(parse_everything_highlight_indexes)
            .unwrap_or_default(),
        subtitle_highlight_data: Vec::new(),
        icon_data_url: icon_data_url_for_path(&result.full_path),
    }
}

fn score_everything_result(
    result: &EverythingSdkRawResult,
    kind: SearchResultKind,
    mode: EverythingQueryMode,
    query: &str,
) -> i32 {
    let base = match kind {
        SearchResultKind::App => 1_300,
        SearchResultKind::Folder => 880,
        _ => 720,
    };
    base + result.run_count.min(20) as i32
        + folder_navigation_boost(result, kind, mode, query)
        + app_like_boost(kind, mode)
}

fn folder_navigation_boost(
    result: &EverythingSdkRawResult,
    kind: SearchResultKind,
    mode: EverythingQueryMode,
    query: &str,
) -> i32 {
    if mode != EverythingQueryMode::FolderNavigation || kind != SearchResultKind::Folder {
        return 0;
    }
    let normalized_query = normalize(query);
    let path = normalize(&result.full_path.display().to_string());
    let title = display_name(&result.full_path, kind);
    let normalized_title = normalize(&title);
    let important_root = normalized_query == "dev"
        && (path == "c dev" || normalized_title == "dev" || normalized_title == "c dev");
    if important_root {
        900
    } else {
        220
    }
}

fn app_like_boost(kind: SearchResultKind, mode: EverythingQueryMode) -> i32 {
    if mode == EverythingQueryMode::AppLike && kind == SearchResultKind::App {
        350
    } else {
        0
    }
}

fn display_name(path: &Path, kind: SearchResultKind) -> String {
    let name = if kind == SearchResultKind::Folder {
        path.file_name()
    } else {
        path.file_stem().or_else(|| path.file_name())
    };
    name.map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn result_kind_label(kind: SearchResultKind) -> &'static str {
    match kind {
        SearchResultKind::App => "Application",
        SearchResultKind::Folder => "Folder",
        _ => "File",
    }
}

fn legacy_kind(kind: SearchResultKind) -> &'static str {
    match kind {
        SearchResultKind::App => "app",
        SearchResultKind::Folder => "folder",
        _ => "file",
    }
}

fn is_app_candidate(path: &Path) -> bool {
    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if extension == "lnk" {
        return is_start_menu_or_desktop_shortcut(path);
    }
    if extension != "exe" {
        return false;
    }

    let path_text = path
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('/', r"\");
    if path_text.contains(r"\windows\") || path_text.contains(r"\windows.old\") {
        return false;
    }
    if path_text.contains(r"\uninstall")
        || path_text.contains(r"\installer")
        || path_text.contains(r"\setup")
        || path_text.contains(r"\update")
        || path_text.contains(r"\helper")
    {
        return false;
    }

    path_text.contains(r"\program files\")
        || path_text.contains(r"\program files (x86)\")
        || path_text.contains(r"\appdata\local\")
        || path_text.contains(r"\appdata\roaming\")
        || path_text.contains(r"\windowsapps\")
}

fn is_start_menu_or_desktop_shortcut(path: &Path) -> bool {
    let path_text = path
        .to_string_lossy()
        .to_ascii_lowercase()
        .replace('/', r"\");
    path_text.contains(r"\start menu\programs\") || path_text.contains(r"\desktop\")
}

fn parse_everything_highlight_indexes(value: &str) -> Vec<usize> {
    let mut highlighted = Vec::new();
    let mut in_highlight = false;
    let mut range_start: Option<usize> = None;
    let mut actual_index = 0usize;
    let chars = value.chars().collect::<Vec<_>>();
    let mut index = 0usize;

    while index < chars.len() {
        if chars[index] == '*' {
            if chars.get(index + 1) == Some(&'*') {
                if in_highlight {
                    range_start.get_or_insert(actual_index);
                }
                actual_index += 1;
                index += 2;
                continue;
            }
            if in_highlight {
                if let Some(start) = range_start.take() {
                    highlighted.extend([start, actual_index.saturating_sub(start)]);
                }
            }
            in_highlight = !in_highlight;
            if in_highlight {
                range_start = Some(actual_index);
            }
            index += 1;
            continue;
        }

        actual_index += 1;
        index += 1;
    }
    if in_highlight {
        if let Some(start) = range_start {
            highlighted.extend([start, actual_index.saturating_sub(start)]);
        }
    }

    highlighted
}

fn compress_highlight_indexes(indexes: Vec<usize>) -> Vec<usize> {
    if indexes.is_empty() {
        return Vec::new();
    }
    let mut indexes = indexes;
    indexes.sort_unstable();
    indexes.dedup();

    let mut compressed = Vec::new();
    let mut start = indexes[0];
    let mut previous = indexes[0];

    for index in indexes.into_iter().skip(1) {
        if index == previous + 1 {
            previous = index;
            continue;
        }
        compressed.extend([start, previous - start + 1]);
        start = index;
        previous = index;
    }
    compressed.extend([start, previous - start + 1]);
    compressed
}

fn token_terms(value: &str) -> Vec<String> {
    normalize(value)
        .split(' ')
        .filter(|token| !token.is_empty())
        .take(16)
        .map(str::to_string)
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
fn clear_everything_cache_for_test() {
    if let Some(cache) = EVERYTHING_STATE.get() {
        if let Ok(mut guard) = cache.lock() {
            *guard = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn cached_health_has_explicit_ttl() {
        let state = CachedEverythingState {
            refreshed_at: Instant::now(),
            dll_path: Some(PathBuf::from(r"C:\Everything64.dll")),
            health: SearchProviderHealth {
                provider_id: SearchProviderId::Everything,
                state: SearchProviderHealthState::Ready,
                reason_code: None,
                message: None,
            },
        };

        assert!(state.is_fresh(state.refreshed_at + Duration::from_secs(5)));
        assert!(!state.is_fresh(state.refreshed_at + Duration::from_secs(60)));
    }

    #[test]
    fn simple_name_request_keeps_full_path_and_content_search_off() {
        let request = everything_request("jnev1", 10);

        assert_eq!(
            classify_everything_query("jnev1"),
            EverythingQueryMode::SimpleName
        );
        assert_eq!(request.query, "jnev1");
        assert_eq!(request.max_results, 35);
        assert!(!request.full_path_search);
        assert!(!request.content_search_enabled);
        assert_eq!(request.sort, EverythingSortMode::NameAsc);
    }

    #[test]
    fn path_like_request_enables_full_path_search_only_for_paths() {
        for query in [
            r"C:\dev",
            "C:/dev",
            r".\src",
            "~/Downloads",
            r"\\server\share",
        ] {
            let request = everything_request(query, 10);

            assert_eq!(
                classify_everything_query(query),
                EverythingQueryMode::PathLike
            );
            assert!(request.full_path_search, "{query}");
            assert!(!request.content_search_enabled);
            assert_eq!(request.sort, EverythingSortMode::PathAsc);
        }
    }

    #[test]
    fn folder_navigation_queries_stay_name_fast_but_boost_folders() {
        let request = everything_request("dev", 10);
        let dev_root = EverythingSdkRawResult {
            full_path: PathBuf::from(r"C:\dev"),
            kind: EverythingSdkResultKind::Folder,
            run_count: 0,
            highlighted_file_name: Some("*dev*".to_string()),
        };
        let dev_notes = EverythingSdkRawResult {
            full_path: PathBuf::from(r"C:\notes\dev.txt"),
            kind: EverythingSdkResultKind::File,
            run_count: 20,
            highlighted_file_name: Some("*dev*.txt".to_string()),
        };

        let folder =
            map_sdk_result_for_mode(&dev_root, EverythingQueryMode::FolderNavigation, "dev");
        let file =
            map_sdk_result_for_mode(&dev_notes, EverythingQueryMode::FolderNavigation, "dev");

        assert_eq!(
            classify_everything_query("dev"),
            EverythingQueryMode::FolderNavigation
        );
        assert!(!request.full_path_search);
        assert!(!request.content_search_enabled);
        assert!(folder.score > file.score);
        assert_eq!(folder.title_highlight_data, vec![0, 3]);
    }

    #[test]
    fn app_like_queries_use_run_count_sort_without_content_search() {
        for query in ["spotify", "vs code", "code.exe"] {
            let request = everything_request(query, 20);

            assert_eq!(
                classify_everything_query(query),
                EverythingQueryMode::AppLike
            );
            assert!(!request.full_path_search);
            assert!(!request.content_search_enabled);
            assert_eq!(request.sort, EverythingSortMode::RunCountDesc);
        }
    }

    #[test]
    fn cached_health_reuses_fresh_state_without_redetecting() {
        clear_everything_cache_for_test();
        let detect_calls = AtomicUsize::new(0);
        let first_now = Instant::now();
        let (first_state, first_cache_state) = cached_everything_state(first_now, || {
            detect_calls.fetch_add(1, Ordering::SeqCst);
            EverythingDetection {
                dll_path: Some(PathBuf::from(r"C:\Everything64.dll")),
                installed_exe_path: Some(PathBuf::from(
                    r"C:\Program Files\Everything\Everything.exe",
                )),
                process_running: true,
                service_running: true,
            }
        });

        assert_eq!(first_cache_state, SearchProviderCacheState::Refresh);
        assert_eq!(detect_calls.load(Ordering::SeqCst), 1);
        assert!(first_state.dll_path.is_some());

        let (_, second_cache_state) =
            cached_everything_state(first_now + Duration::from_secs(1), || {
                detect_calls.fetch_add(1, Ordering::SeqCst);
                EverythingDetection {
                    dll_path: None,
                    installed_exe_path: None,
                    process_running: false,
                    service_running: false,
                }
            });

        assert_eq!(second_cache_state, SearchProviderCacheState::Hit);
        assert_eq!(detect_calls.load(Ordering::SeqCst), 1);
        clear_everything_cache_for_test();
    }

    #[test]
    fn search_everything_records_direct_sdk_latency_with_injected_runner() {
        clear_everything_cache_for_test();
        let run = search_everything_with(
            r"C:\dev",
            1,
            || EverythingDetection {
                dll_path: Some(PathBuf::from(r"C:\Everything64.dll")),
                installed_exe_path: Some(PathBuf::from(
                    r"C:\Program Files\Everything\Everything.exe",
                )),
                process_running: true,
                service_running: true,
            },
            |_, request| {
                assert!(request.full_path_search);
                assert_eq!(request.sort, EverythingSortMode::PathAsc);
                std::thread::sleep(Duration::from_millis(6));
                Ok(vec![EverythingSdkRawResult {
                    full_path: PathBuf::from(r"C:\dev"),
                    kind: EverythingSdkResultKind::Folder,
                    run_count: 2,
                    highlighted_file_name: None,
                }])
            },
        );

        assert!(run.sdk_latency_ms.is_some());
        assert!(run.sdk_latency_ms.unwrap() >= 5.0);
        assert_eq!(run.results.len(), 1);
        clear_everything_cache_for_test();
    }

    #[test]
    fn simple_name_query_returns_rows_without_path_mode() {
        clear_everything_cache_for_test();
        let run = search_everything_with(
            "jnev1",
            5,
            || EverythingDetection {
                dll_path: Some(PathBuf::from(r"C:\Everything64.dll")),
                installed_exe_path: Some(PathBuf::from(
                    r"C:\Program Files\Everything\Everything.exe",
                )),
                process_running: true,
                service_running: true,
            },
            |_, request| {
                assert_eq!(request.query, "jnev1");
                assert!(!request.full_path_search);
                assert_eq!(request.sort, EverythingSortMode::NameAsc);
                Ok(vec![EverythingSdkRawResult {
                    full_path: PathBuf::from(r"C:\Users\jnev1"),
                    kind: EverythingSdkResultKind::Folder,
                    run_count: 1,
                    highlighted_file_name: Some("*jnev1*".to_string()),
                }])
            },
        );

        assert_eq!(run.results.len(), 1);
        assert_eq!(run.results[0].provider_id, SearchProviderId::Everything);
        assert_eq!(run.results[0].kind, SearchResultKind::Folder);
        clear_everything_cache_for_test();
    }

    #[test]
    fn everything_request_limit_is_bounded_to_display_plus_overfetch() {
        assert_eq!(bounded_everything_limit(1), 26);
        assert_eq!(bounded_everything_limit(50), 75);
        assert_eq!(bounded_everything_limit(500), 200);
    }

    #[test]
    fn detection_health_prefers_cached_sdk_ready_path() {
        let health = health_from_detection(&EverythingDetection {
            dll_path: Some(PathBuf::from(r"C:\Everything64.dll")),
            installed_exe_path: None,
            process_running: true,
            service_running: true,
        });

        assert_eq!(health.provider_id, SearchProviderId::Everything);
        assert_eq!(health.state, SearchProviderHealthState::Ready);
    }

    #[test]
    fn maps_everything_sdk_rows_to_new_contract_rows() {
        let result = map_sdk_result_for_mode(
            &EverythingSdkRawResult {
                full_path: PathBuf::from(r"C:\dev"),
                kind: EverythingSdkResultKind::Folder,
                run_count: 3,
                highlighted_file_name: None,
            },
            EverythingQueryMode::SimpleName,
            "dev",
        );

        assert_eq!(result.provider_id, SearchProviderId::Everything);
        assert_eq!(result.kind, SearchResultKind::Folder);
        assert!(matches!(
            result.action,
            SearchResultAction::OpenFolder { .. }
        ));
        assert!(result.score >= 880);
    }

    #[test]
    fn compresses_everything_highlight_indexes_into_span_pairs() {
        assert_eq!(
            compress_highlight_indexes(vec![0, 1, 3, 4, 5]),
            vec![0, 2, 3, 3]
        );
    }

    #[test]
    fn highlighted_file_name_markers_become_span_pairs() {
        assert_eq!(parse_everything_highlight_indexes("*dev*.txt"), vec![0, 3]);
        assert_eq!(
            parse_everything_highlight_indexes("*Visual* *Studio* Code"),
            vec![0, 6, 7, 6]
        );
        assert_eq!(
            parse_everything_highlight_indexes("literal ** star"),
            Vec::<usize>::new()
        );
    }
}
