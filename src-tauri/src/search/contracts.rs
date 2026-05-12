use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchQueryRequest {
    pub query: String,
    pub sequence: u64,
    pub limit: usize,
    pub presentation: SearchPresentation,
    #[serde(default)]
    pub context: SearchQueryContext,
}

impl SearchQueryRequest {
    #[cfg(test)]
    pub(crate) fn new(query: impl Into<String>, sequence: u64) -> Self {
        Self {
            query: query.into(),
            sequence,
            limit: 50,
            presentation: SearchPresentation::Centered,
            context: SearchQueryContext::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchQueryContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_roots: Option<Vec<String>>,
    #[serde(default)]
    pub open_windows: Vec<SearchOpenWindowContext>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchOpenWindowContext {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_data_url: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SearchPresentation {
    Anchored,
    Centered,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchEngineResponse {
    pub query: String,
    pub sequence: u64,
    pub results: Vec<SearchResult>,
    pub provider_timings: Vec<SearchProviderTiming>,
    pub health: Vec<SearchProviderHealth>,
    pub generated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<SearchDiagnostics>,
}

impl SearchEngineResponse {
    #[cfg(test)]
    pub(crate) fn empty(request: &SearchQueryRequest) -> Self {
        Self {
            query: request.query.clone(),
            sequence: request.sequence,
            results: Vec::new(),
            provider_timings: Vec::new(),
            health: Vec::new(),
            generated_at: iso_now(),
            diagnostics: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchProgressPayload {
    pub query: String,
    pub sequence: u64,
    pub phase: SearchProgressPhase,
    pub results: Vec<SearchResult>,
    #[serde(default)]
    pub provider_timings: Vec<SearchProviderTiming>,
    pub status_message: String,
    pub generated_at: String,
    #[serde(default)]
    pub stale: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SearchProgressPhase {
    Typing,
    Local,
    Provider,
    Complete,
    Error,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResult {
    pub id: String,
    pub provider_id: SearchProviderId,
    pub kind: SearchResultKind,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub action: SearchResultAction,
    #[serde(default)]
    pub terms: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub score: i32,
    pub match_reason: String,
    pub record_key: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub title_highlight_data: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subtitle_highlight_data: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_data_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum SearchResultAction {
    OpenApp {
        path: String,
    },
    FocusWindow {
        window_id: String,
    },
    OpenFile {
        path: String,
    },
    OpenFolder {
        path: String,
    },
    RunCommand {
        command_id: String,
    },
    OpenSetting {
        uri: String,
    },
    RunControlPanel {
        executable: String,
        args: Option<Vec<String>>,
    },
    CopyText {
        text: String,
    },
    OpenWebUrl {
        url: String,
    },
    OpenBookmark {
        url: String,
    },
}

impl SearchResultAction {
    pub(crate) fn is_safe(&self) -> bool {
        match self {
            Self::OpenApp { path } | Self::OpenFile { path } | Self::OpenFolder { path } => {
                !path.trim().is_empty()
            }
            Self::FocusWindow { window_id } => !window_id.trim().is_empty(),
            Self::RunCommand { command_id } => {
                command_id.trim().chars().next().is_some()
                    && command_id
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-'))
            }
            Self::OpenSetting { uri } => is_safe_ms_settings_uri(uri),
            Self::RunControlPanel { executable, args } => {
                executable.eq_ignore_ascii_case("control.exe")
                    && args.as_ref().map_or(true, |args| {
                        args.iter().all(|arg| {
                            !arg.is_empty()
                                && arg.chars().all(|ch| {
                                    ch.is_ascii_alphanumeric()
                                        || matches!(ch, '.' | '_' | '{' | '}' | ',' | '-')
                                })
                        })
                    })
            }
            Self::CopyText { .. } => true,
            Self::OpenWebUrl { url } | Self::OpenBookmark { url } => {
                url.starts_with("https://") || url.starts_with("http://")
            }
        }
    }
}

pub(crate) fn is_safe_ms_settings_uri(uri: &str) -> bool {
    let Some(rest) = uri.strip_prefix("ms-settings:") else {
        return false;
    };
    rest.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SearchResultKind {
    App,
    Window,
    Folder,
    File,
    Command,
    Setting,
    Calculator,
    Web,
    Bookmark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SearchProviderId {
    Apps,
    OpenWindows,
    Everything,
    Settings,
    Commands,
    Calculator,
    Web,
    Bookmarks,
    LocalFolders,
    Diagnostics,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchProviderTiming {
    pub provider_id: SearchProviderId,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    pub duration_ms: f64,
    pub cache: SearchProviderCacheState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_age_ms: Option<u64>,
    pub result_count: usize,
    pub applied: bool,
    pub discarded_as_stale: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SearchProviderCacheState {
    Hit,
    Miss,
    Refresh,
    Indexing,
    Disabled,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchProviderHealth {
    pub provider_id: SearchProviderId,
    pub state: SearchProviderHealthState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<SearchProviderReasonCode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SearchProviderHealthState {
    Ready,
    Degraded,
    Unavailable,
    Indexing,
    Disabled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SearchProviderReasonCode {
    NotInitialized,
    SdkMissing,
    IpcUnavailable,
    EmptyDataset,
    ProviderError,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchDiagnostics {
    pub coordinator: String,
    pub legacy_hot_path_used: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

pub(crate) fn iso_now() -> String {
    let total_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let days = (total_secs / 86_400) as i64;
    let seconds_of_day = total_secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + i64::from(m <= 2), m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_use_camel_case_json_fields() {
        let request = SearchQueryRequest::new("sound settings", 7);
        let json = serde_json::to_value(request).expect("request serializes");

        assert_eq!(json["query"], "sound settings");
        assert_eq!(json["sequence"], 7);
        assert!(json.get("presentation").is_some());
        assert!(json.get("open_windows").is_none());
    }

    #[test]
    fn progress_payload_represents_pending_before_providers() {
        let payload = SearchProgressPayload {
            query: "display".to_string(),
            sequence: 11,
            phase: SearchProgressPhase::Typing,
            results: Vec::new(),
            provider_timings: Vec::new(),
            status_message: "searching".to_string(),
            generated_at: iso_now(),
            stale: false,
        };

        assert_eq!(payload.phase, SearchProgressPhase::Typing);
        assert!(payload.results.is_empty());
    }

    #[test]
    fn health_and_timing_contracts_keep_provider_diagnostics() {
        let timing = SearchProviderTiming {
            provider_id: SearchProviderId::Settings,
            started_at: iso_now(),
            ended_at: Some(iso_now()),
            duration_ms: 3.0,
            cache: SearchProviderCacheState::Hit,
            cache_age_ms: Some(25),
            result_count: 2,
            applied: true,
            discarded_as_stale: false,
        };
        let health = SearchProviderHealth {
            provider_id: SearchProviderId::Settings,
            state: SearchProviderHealthState::Ready,
            reason_code: None,
            message: Some("settings provider ready".to_string()),
        };

        assert_eq!(timing.provider_id, health.provider_id);
        assert_eq!(timing.result_count, 2);
        assert_eq!(timing.cache_age_ms, Some(25));
        assert_eq!(health.state, SearchProviderHealthState::Ready);
    }

    #[test]
    fn action_safety_rejects_empty_command_ids() {
        assert!(!SearchResultAction::RunCommand {
            command_id: String::new()
        }
        .is_safe());
        assert!(SearchResultAction::RunCommand {
            command_id: "command:open-control-plane".to_string()
        }
        .is_safe());
    }
}
