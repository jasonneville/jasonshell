use crate::search::contracts::{
    SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState, SearchProviderId,
    SearchProviderReasonCode, SearchProviderTiming, SearchResult, SearchResultAction,
    SearchResultKind,
};
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

#[derive(Clone, Debug)]
pub(crate) struct EverythingSearchRun {
    pub(crate) results: Vec<SearchResult>,
    pub(crate) timing: SearchProviderTiming,
    pub(crate) health: SearchProviderHealth,
}

pub(crate) fn search_everything(query: &str, limit: usize) -> EverythingSearchRun {
    let started_at = crate::search::contracts::iso_now();
    let started = Instant::now();
    let normalized_query = query.trim();
    let (state, cache_state) = cached_everything_state(Instant::now());

    let mut health = state.health.clone();
    let mut results = Vec::new();
    if !normalized_query.is_empty() {
        if let Some(dll_path) = state.dll_path.clone() {
            let request = everything_request(normalized_query, limit);
            match run_everything_query(&dll_path, &request) {
                Ok(raw_results) => {
                    results = raw_results
                        .iter()
                        .take(limit)
                        .map(map_sdk_result)
                        .collect();
                }
                Err(error) => {
                    health = health_for_error(&error);
                }
            }
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
            result_count,
            applied: true,
            discarded_as_stale: false,
        },
        health,
    }
}

fn cached_everything_state(now: Instant) -> (CachedEverythingState, SearchProviderCacheState) {
    let cache = EVERYTHING_STATE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.as_ref().filter(|cached| cached.is_fresh(now)) {
            return (cached.clone(), SearchProviderCacheState::Hit);
        }
    }

    let detected = detect_everything();
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
    EverythingSdkRequest {
        query: query.to_string(),
        max_results: bounded_everything_limit(limit),
        full_path_search: false,
        sort: EverythingSortMode::NameAsc,
        content_search_enabled: false,
    }
}

fn bounded_everything_limit(limit: usize) -> usize {
    limit
        .saturating_add(EVERYTHING_OVERFETCH)
        .clamp(1, EVERYTHING_MAX_QUERY_RESULTS)
}

fn map_sdk_result(result: &EverythingSdkRawResult) -> SearchResult {
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
        SearchResultKind::App => SearchResultAction::OpenApp {
            path: path.clone(),
        },
        SearchResultKind::Folder => SearchResultAction::OpenFolder {
            path: path.clone(),
        },
        _ => SearchResultAction::OpenFile {
            path: path.clone(),
        },
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
        score: score_everything_result(result, kind),
        match_reason: "everythingName".to_string(),
        record_key: format!("everything:{legacy_kind}:{}", normalize_record_key(&path)),
        icon_data_url: None,
    }
}

fn score_everything_result(result: &EverythingSdkRawResult, kind: SearchResultKind) -> i32 {
    let base = match kind {
        SearchResultKind::App => 1_300,
        SearchResultKind::Folder => 880,
        _ => 720,
    };
    base + result.run_count.min(20) as i32
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
    let mut actual_index = 0usize;
    let chars = value.chars().collect::<Vec<_>>();
    let mut index = 0usize;

    while index < chars.len() {
        if chars[index] == '*' {
            if chars.get(index + 1) == Some(&'*') {
                if in_highlight {
                    highlighted.push(actual_index);
                }
                actual_index += 1;
                index += 2;
                continue;
            }
            in_highlight = !in_highlight;
            index += 1;
            continue;
        }

        if in_highlight {
            highlighted.push(actual_index);
        }
        actual_index += 1;
        index += 1;
    }

    highlighted
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
mod tests {
    use super::*;
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
        let request = everything_request("j", 10);

        assert_eq!(request.query, "j");
        assert_eq!(request.max_results, 35);
        assert!(!request.full_path_search);
        assert!(!request.content_search_enabled);
        assert_eq!(request.sort, EverythingSortMode::NameAsc);
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
        let result = map_sdk_result(&EverythingSdkRawResult {
            full_path: PathBuf::from(r"C:\dev"),
            kind: EverythingSdkResultKind::Folder,
            run_count: 3,
            highlighted_file_name: None,
        });

        assert_eq!(result.provider_id, SearchProviderId::Everything);
        assert_eq!(result.kind, SearchResultKind::Folder);
        assert!(matches!(
            result.action,
            SearchResultAction::OpenFolder { .. }
        ));
        assert!(result.score >= 880);
    }
}
