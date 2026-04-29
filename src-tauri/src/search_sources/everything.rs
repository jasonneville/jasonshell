use super::everything_ffi;
use super::everything_install;
use super::provider::{
    provider_health, ProviderHealthContract, ProviderHealthState, ProviderReasonCode,
    SearchProviderId,
};
use super::SystemSearchResult;
use crate::settings::{EverythingSearchSettings, EverythingSortMode};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const EVERYTHING_REQUEST_LIMIT_FALLBACK: usize = 200;

#[derive(Clone, Debug)]
pub(crate) struct EverythingSearchOutcome {
    pub results: Vec<SystemSearchResult>,
    pub health: ProviderHealthContract,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EverythingSearchRequest {
    pub query: String,
    pub max_results: usize,
    pub full_path_search: bool,
    pub sort: EverythingSortMode,
    pub content_search_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EverythingRawResult {
    pub full_path: PathBuf,
    pub kind: EverythingResultKind,
    pub run_count: u32,
    pub highlighted_file_name: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EverythingResultKind {
    File,
    Folder,
    Volume,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EverythingProviderError {
    Disabled,
    SdkMissing,
    IpcUnavailable,
    NotRunning,
    QueryFailed(String),
}

pub(crate) trait EverythingSdk: Send {
    fn query(
        &mut self,
        request: &EverythingSearchRequest,
    ) -> Result<Vec<EverythingRawResult>, EverythingProviderError>;
    fn reset(&mut self);
}

pub(crate) struct EverythingProvider<Sdk> {
    sdk: Mutex<Sdk>,
}

impl<Sdk> EverythingProvider<Sdk>
where
    Sdk: EverythingSdk,
{
    pub(crate) fn new(sdk: Sdk) -> Self {
        Self {
            sdk: Mutex::new(sdk),
        }
    }

    pub(crate) fn search(
        &self,
        request: &EverythingSearchRequest,
    ) -> Result<Vec<SystemSearchResult>, EverythingProviderError> {
        if request.query.trim().is_empty() {
            return Ok(Vec::new());
        }
        if request.content_search_enabled {
            return Err(EverythingProviderError::QueryFailed(
                "Everything content search is disabled for realtime search".to_string(),
            ));
        }

        let mut sdk = self.sdk.lock().map_err(|_| {
            EverythingProviderError::QueryFailed("Everything SDK lock failed".to_string())
        })?;
        let raw_results = sdk.query(request);
        sdk.reset();
        raw_results.map(|results| map_everything_results(&results, request.max_results))
    }
}

pub(crate) fn search_system_everything(
    query: &str,
    settings: &EverythingSearchSettings,
) -> EverythingSearchOutcome {
    if !settings.enabled {
        return EverythingSearchOutcome {
            results: Vec::new(),
            health: provider_health(
                SearchProviderId::Everything,
                ProviderHealthState::Disabled,
                Some(ProviderReasonCode::UserDisabled),
                "Everything search is disabled in shell settings",
                false,
            ),
        };
    }

    let health = current_everything_health(settings);
    if health.state != ProviderHealthState::Ready {
        return EverythingSearchOutcome {
            results: Vec::new(),
            health,
        };
    }

    let install = everything_install::detect_everything_installation();
    let Some(dll_path) =
        everything_ffi::detect_system_sdk(install.installed_exe_path.as_deref()).dll_path
    else {
        return EverythingSearchOutcome {
            results: Vec::new(),
            health: provider_health(
                SearchProviderId::Everything,
                ProviderHealthState::Degraded,
                Some(ProviderReasonCode::SdkMissing),
                everything_ffi::sdk_missing_message(),
                true,
            ),
        };
    };

    let request = EverythingSearchRequest {
        query: query.to_string(),
        max_results: bounded_everything_request_limit(settings.max_results),
        full_path_search: settings.full_path_search,
        sort: settings.sort,
        content_search_enabled: settings.content_search_enabled,
    };
    let provider = EverythingProvider::new(everything_ffi::DynamicEverythingSdk::new(dll_path));
    match provider.search(&request) {
        Ok(results) => EverythingSearchOutcome { results, health },
        Err(error) => EverythingSearchOutcome {
            results: Vec::new(),
            health: provider_health(
                SearchProviderId::Everything,
                ProviderHealthState::Degraded,
                reason_for_error(&error),
                message_for_error(&error),
                true,
            ),
        },
    }
}

fn bounded_everything_request_limit(limit: usize) -> usize {
    limit.clamp(1, EVERYTHING_REQUEST_LIMIT_FALLBACK)
}

pub(crate) fn current_everything_health(
    settings: &EverythingSearchSettings,
) -> ProviderHealthContract {
    if !settings.enabled {
        return provider_health(
            SearchProviderId::Everything,
            ProviderHealthState::Disabled,
            Some(ProviderReasonCode::UserDisabled),
            "Everything search is disabled in shell settings",
            false,
        );
    }

    let install = everything_install::detect_everything_installation();
    let sdk = everything_ffi::detect_system_sdk(install.installed_exe_path.as_deref());
    everything_health_from_detection(&install, sdk.dll_path.is_some())
}

fn everything_health_from_detection(
    install: &everything_install::EverythingInstallationStatus,
    sdk_available: bool,
) -> ProviderHealthContract {
    if !sdk_available {
        return provider_health(
            SearchProviderId::Everything,
            ProviderHealthState::Degraded,
            Some(ProviderReasonCode::SdkMissing),
            everything_ffi::sdk_missing_message(),
            true,
        );
    }
    if !install.process_running {
        if install.installed_exe_path.is_none() {
            return provider_health(
                SearchProviderId::Everything,
                ProviderHealthState::Unavailable,
                Some(ProviderReasonCode::NotInstalled),
                "Everything executable was not found in approved install locations and no running Everything process was detected",
                true,
            );
        }
        return provider_health(
            SearchProviderId::Everything,
            ProviderHealthState::Degraded,
            Some(ProviderReasonCode::NotRunning),
            "Everything is installed but not running",
            true,
        );
    }

    if !install.service_running {
        return provider_health(
            SearchProviderId::Everything,
            ProviderHealthState::Degraded,
            Some(ProviderReasonCode::ServiceUnavailable),
            "Everything service is not reported as running; search is unavailable",
            true,
        );
    }

    provider_health(
        SearchProviderId::Everything,
        ProviderHealthState::Ready,
        None,
        "Everything process, service, and approved SDK DLL were detected",
        false,
    )
}

pub(crate) fn parse_everything_highlight_indexes(value: &str) -> Vec<usize> {
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

fn map_everything_results(
    raw_results: &[EverythingRawResult],
    limit: usize,
) -> Vec<SystemSearchResult> {
    raw_results
        .iter()
        .take(limit)
        .map(map_everything_result)
        .collect()
}

fn map_everything_result(raw: &EverythingRawResult) -> SystemSearchResult {
    let kind = match raw.kind {
        EverythingResultKind::File if is_app_candidate(&raw.full_path) => "app",
        EverythingResultKind::File => "file",
        EverythingResultKind::Folder | EverythingResultKind::Volume => "folder",
    };
    let path_text = raw.full_path.display().to_string();
    let title = display_name(&raw.full_path, kind);
    let subtitle = raw
        .full_path
        .parent()
        .map(|parent| format!("{} - {}", label_for_kind(kind), parent.display()))
        .unwrap_or_else(|| label_for_kind(kind).to_string());
    let highlight_terms = raw
        .highlighted_file_name
        .as_deref()
        .map(parse_everything_highlight_indexes)
        .unwrap_or_default()
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(" ");

    SystemSearchResult {
        id: format!("system:{kind}:{path_text}"),
        provider_id: Some("everything".to_string()),
        kind: kind.to_string(),
        title: title.clone(),
        subtitle,
        terms: format!(
            "{title} {path_text} everything voidtools local filesystem {highlight_terms}"
        ),
        priority: base_priority(kind) + raw.run_count.min(25) as i32,
        path: path_text.clone(),
        record_key: Some(format!(
            "{}:{}",
            kind,
            path_text.trim().replace('/', r"\").to_lowercase()
        )),
        run_count: Some(raw.run_count),
        top_most: None,
    }
}

fn display_name(path: &Path, kind: &str) -> String {
    let name = if kind == "folder" {
        path.file_name()
    } else {
        path.file_stem().or_else(|| path.file_name())
    };
    name.map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn label_for_kind(kind: &str) -> &'static str {
    match kind {
        "app" => "Application",
        "folder" => "Folder",
        _ => "File",
    }
}

fn base_priority(kind: &str) -> i32 {
    match kind {
        "app" => 170,
        "folder" => 82,
        _ => 90,
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

fn reason_for_error(error: &EverythingProviderError) -> Option<ProviderReasonCode> {
    match error {
        EverythingProviderError::Disabled => Some(ProviderReasonCode::UserDisabled),
        EverythingProviderError::SdkMissing => Some(ProviderReasonCode::SdkMissing),
        EverythingProviderError::IpcUnavailable => Some(ProviderReasonCode::IpcUnavailable),
        EverythingProviderError::NotRunning => Some(ProviderReasonCode::NotRunning),
        EverythingProviderError::QueryFailed(_) => Some(ProviderReasonCode::FallbackActive),
    }
}

fn message_for_error(error: &EverythingProviderError) -> String {
    match error {
        EverythingProviderError::Disabled => "Everything search is disabled".to_string(),
        EverythingProviderError::SdkMissing => everything_ffi::sdk_missing_message().to_string(),
        EverythingProviderError::IpcUnavailable => {
            "Everything IPC is unavailable; search is unavailable".to_string()
        }
        EverythingProviderError::NotRunning => {
            "Everything is not running; search is unavailable".to_string()
        }
        EverythingProviderError::QueryFailed(message) => message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockSdk {
        results: Vec<EverythingRawResult>,
        error: Option<EverythingProviderError>,
        reset_count: usize,
    }

    impl EverythingSdk for MockSdk {
        fn query(
            &mut self,
            _request: &EverythingSearchRequest,
        ) -> Result<Vec<EverythingRawResult>, EverythingProviderError> {
            if let Some(error) = self.error.clone() {
                return Err(error);
            }
            Ok(self.results.clone())
        }

        fn reset(&mut self) {
            self.reset_count += 1;
        }
    }

    #[test]
    fn parses_everything_highlight_markers_to_zero_based_indexes() {
        assert_eq!(
            parse_everything_highlight_indexes("abc*123*xy"),
            vec![3, 4, 5]
        );
        assert_eq!(parse_everything_highlight_indexes("*a**b*"), vec![0, 1, 2]);
    }

    #[test]
    fn maps_file_folder_and_volume_results_to_system_results() {
        let raw = vec![
            EverythingRawResult {
                full_path: PathBuf::from(r"C:\Docs\Plan.txt"),
                kind: EverythingResultKind::File,
                run_count: 7,
                highlighted_file_name: Some("*Plan*.txt".to_string()),
            },
            EverythingRawResult {
                full_path: PathBuf::from(r"C:\Docs"),
                kind: EverythingResultKind::Folder,
                run_count: 0,
                highlighted_file_name: None,
            },
            EverythingRawResult {
                full_path: PathBuf::from(r"D:\"),
                kind: EverythingResultKind::Volume,
                run_count: 0,
                highlighted_file_name: None,
            },
        ];

        let results = map_everything_results(&raw, 10);

        assert_eq!(results[0].kind, "file");
        assert_eq!(results[0].title, "Plan");
        assert_eq!(results[1].kind, "folder");
        assert_eq!(results[2].kind, "folder");
    }

    #[test]
    fn maps_installed_app_candidates_from_everything_as_apps() {
        let raw = vec![EverythingRawResult {
            full_path: PathBuf::from(r"C:\Users\me\AppData\Roaming\Spotify\Spotify.exe"),
            kind: EverythingResultKind::File,
            run_count: 3,
            highlighted_file_name: Some("*Spotify*.exe".to_string()),
        }];

        let results = map_everything_results(&raw, 10);

        assert_eq!(results[0].kind, "app");
        assert_eq!(results[0].title, "Spotify");
        assert!(results[0].priority >= 170);
    }

    #[test]
    fn request_limit_allows_larger_everything_result_sets() {
        assert_eq!(bounded_everything_request_limit(0), 1);
        assert_eq!(bounded_everything_request_limit(200), 200);
        assert_eq!(bounded_everything_request_limit(400), 200);
    }

    #[test]
    fn provider_resets_sdk_after_successful_query() {
        let provider = EverythingProvider::new(MockSdk {
            results: vec![EverythingRawResult {
                full_path: PathBuf::from(r"C:\Docs\Plan.txt"),
                kind: EverythingResultKind::File,
                run_count: 0,
                highlighted_file_name: None,
            }],
            ..MockSdk::default()
        });
        let request = request("plan");

        let results = provider.search(&request).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(provider.sdk.lock().unwrap().reset_count, 1);
    }

    #[test]
    fn provider_allows_single_character_everything_queries() {
        let provider = EverythingProvider::new(MockSdk {
            results: vec![EverythingRawResult {
                full_path: PathBuf::from(r"C:\Docs\a.txt"),
                kind: EverythingResultKind::File,
                run_count: 0,
                highlighted_file_name: None,
            }],
            ..MockSdk::default()
        });
        let request = request("a");

        let results = provider.search(&request).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "a");
    }

    #[test]
    fn health_accepts_portable_running_everything_with_repo_sdk() {
        let install = everything_install::EverythingInstallationStatus {
            installed_exe_path: None,
            process_running: true,
            service_running: true,
        };

        let health = everything_health_from_detection(&install, true);

        assert_eq!(health.state, ProviderHealthState::Ready);
        assert_eq!(health.reason_code, None);
    }

    #[test]
    fn provider_resets_sdk_after_failed_query() {
        let provider = EverythingProvider::new(MockSdk {
            error: Some(EverythingProviderError::IpcUnavailable),
            ..MockSdk::default()
        });
        let request = request("plan");

        let error = provider.search(&request).unwrap_err();

        assert_eq!(error, EverythingProviderError::IpcUnavailable);
        assert_eq!(provider.sdk.lock().unwrap().reset_count, 1);
    }

    fn request(query: &str) -> EverythingSearchRequest {
        EverythingSearchRequest {
            query: query.to_string(),
            max_results: 20,
            full_path_search: true,
            sort: EverythingSortMode::NameAsc,
            content_search_enabled: false,
        }
    }
}
