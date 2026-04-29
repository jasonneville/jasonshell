use super::apps;
use super::everything;
use super::query;
use super::windows_search;
use super::SystemSearchResult;
use crate::settings::ShellSettings;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

const INSTALLED_APP_RESULT_LIMIT: usize = 16;
const WINDOWS_APP_RESULT_LIMIT: usize = 24;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SearchProviderId {
    Apps,
    OpenWindows,
    Everything,
    WindowsSearch,
    WarmedCache,
    Commands,
    Calculator,
    Web,
    Bookmarks,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ProviderHealthState {
    Ready,
    Degraded,
    Unavailable,
    Indexing,
    AdminRequired,
    Disabled,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ProviderReasonCode {
    SdkMissing,
    IpcUnavailable,
    ServiceUnavailable,
    NotInstalled,
    NotRunning,
    UserDisabled,
    ChecksumBlocked,
    LicenseBlocked,
    FallbackActive,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderHealthContract {
    pub provider_id: SearchProviderId,
    pub state: ProviderHealthState,
    pub reason_code: Option<ProviderReasonCode>,
    pub message: String,
    pub can_request_setup: bool,
    pub checked_at_iso: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ProviderSearchBatch {
    pub results: Vec<SystemSearchResult>,
    pub health: Vec<ProviderHealthContract>,
}

pub(crate) fn search_provider_results(
    query: &str,
    settings: &ShellSettings,
) -> ProviderSearchBatch {
    let mut batch = ProviderSearchBatch::default();
    let parsed_query = query::parse_search_query(query, &["f", "file", "folder"], false);
    if parsed_query.is_home_query || parsed_query.search.is_empty() {
        return batch;
    }

    batch.results.extend(apps::search_apps(
        &parsed_query.search,
        INSTALLED_APP_RESULT_LIMIT,
    ));

    let everything_outcome =
        everything::search_system_everything(&parsed_query.search, &settings.search.everything);
    batch.health.push(everything_outcome.health);
    batch.results.extend(everything_outcome.results);

    if !has_app_result(&batch.results) {
        append_windows_app_results(&mut batch, &parsed_query.search);
    }

    batch
}

fn has_app_result(results: &[SystemSearchResult]) -> bool {
    results.iter().any(|result| result.kind == "app")
}

fn append_windows_app_results(batch: &mut ProviderSearchBatch, query: &str) {
    match windows_search::search_windows(query, WINDOWS_APP_RESULT_LIMIT) {
        windows_search::ProviderSearchOutcome::Results(results) => {
            let apps = results
                .into_iter()
                .filter(|result| result.kind == "app")
                .collect::<Vec<_>>();
            if !apps.is_empty() {
                batch.results.extend(apps);
                batch.health.push(provider_health(
                    SearchProviderId::WindowsSearch,
                    ProviderHealthState::Ready,
                    None,
                    "Windows Search returned supplemental app results",
                    false,
                ));
            }
        }
        windows_search::ProviderSearchOutcome::Fallback { reason } => {
            batch.health.push(provider_health(
                SearchProviderId::WindowsSearch,
                ProviderHealthState::Degraded,
                Some(ProviderReasonCode::FallbackActive),
                reason,
                false,
            ));
        }
    }
}

pub(crate) fn current_provider_health(settings: &ShellSettings) -> Vec<ProviderHealthContract> {
    vec![everything::current_everything_health(
        &settings.search.everything,
    )]
}

pub(crate) fn provider_health(
    provider_id: SearchProviderId,
    state: ProviderHealthState,
    reason_code: Option<ProviderReasonCode>,
    message: impl Into<String>,
    can_request_setup: bool,
) -> ProviderHealthContract {
    ProviderHealthContract {
        provider_id,
        state,
        reason_code,
        message: message.into(),
        can_request_setup,
        checked_at_iso: checked_at_iso(),
    }
}

fn checked_at_iso() -> String {
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
    fn checked_at_uses_iso_utc_shape() {
        let value = provider_health(
            SearchProviderId::Everything,
            ProviderHealthState::Ready,
            None,
            "ok",
            false,
        )
        .checked_at_iso;

        assert_eq!(value.len(), 20);
        assert!(value.ends_with('Z'));
        assert_eq!(&value[4..5], "-");
        assert_eq!(&value[10..11], "T");
    }
}
