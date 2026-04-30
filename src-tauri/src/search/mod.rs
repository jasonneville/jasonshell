//! Clean-slate Rust search subsystem for the search overhaul.
//!
//! This module is intentionally separate from `search_sources`; early overhaul
//! phases define contracts and local providers before the production hot path
//! is migrated.

#![allow(dead_code)]

pub(crate) mod contracts;
pub(crate) mod providers;
pub(crate) mod scoring;

use contracts::{
    iso_now, SearchDiagnostics, SearchEngineResponse, SearchProviderCacheState, SearchProviderId,
    SearchProviderTiming, SearchQueryRequest,
};
use providers::apps::search_apps;
use providers::everything::search_everything;
use providers::local::search_local;
use providers::open_windows::search_open_windows;
use providers::settings::{search_settings, settings_provider_health};
use scoring::rank_visible_results;
use std::time::Instant;

#[tauri::command]
pub(crate) fn search_engine(request: SearchQueryRequest) -> Result<SearchEngineResponse, String> {
    let query = request.query.trim().to_string();
    let limit = request.limit.clamp(1, 50);
    let mut results = Vec::new();
    let mut provider_timings = Vec::new();
    let mut health = Vec::new();

    let settings_started_at = iso_now();
    let settings_started = Instant::now();
    let settings_results = search_settings(&query, limit);
    provider_timings.push(SearchProviderTiming {
        provider_id: SearchProviderId::Settings,
        started_at: settings_started_at,
        ended_at: Some(iso_now()),
        duration_ms: settings_started.elapsed().as_secs_f64() * 1000.0,
        cache: SearchProviderCacheState::Hit,
        result_count: settings_results.len(),
        applied: true,
        discarded_as_stale: false,
    });
    health.push(settings_provider_health());
    results.extend(settings_results);

    let apps_run = search_apps(&query, limit);
    provider_timings.push(apps_run.timing);
    health.push(apps_run.health);
    results.extend(apps_run.results);

    let local_run = search_local(&query, limit, &request.context);
    provider_timings.push(local_run.timing);
    health.push(local_run.health);
    results.extend(local_run.results);

    let open_windows_run = search_open_windows(&query, limit, &request.context.open_windows);
    provider_timings.push(open_windows_run.timing);
    health.push(open_windows_run.health);
    results.extend(open_windows_run.results);

    let everything_run = search_everything(&query, limit);
    provider_timings.push(everything_run.timing);
    health.push(everything_run.health);
    results.extend(everything_run.results);

    let results = rank_visible_results(&query, results, limit);

    Ok(SearchEngineResponse {
        query,
        sequence: request.sequence,
        results,
        provider_timings,
        health,
        generated_at: iso_now(),
        diagnostics: Some(SearchDiagnostics {
            coordinator: "search_engine.phase5.rust_ranked".to_string(),
            legacy_hot_path_used: false,
            notes: vec![
                "phase 5 command gathers settings/apps/local/Everything rows, then Rust scores and dedupes once".to_string(),
                "Legacy index, Windows Search, and cache display paths are not used by this command".to_string(),
            ],
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::contracts::{SearchEngineResponse, SearchQueryRequest};
    use super::search_engine;

    #[test]
    fn empty_response_preserves_query_identity() {
        let request = SearchQueryRequest::new("display settings", 42);
        let response = SearchEngineResponse::empty(&request);

        assert_eq!(response.query, "display settings");
        assert_eq!(response.sequence, 42);
        assert!(response.results.is_empty());
        assert!(response.provider_timings.is_empty());
        assert!(response.health.is_empty());
    }

    #[test]
    fn command_returns_settings_results_without_legacy_search_sources() {
        let request = SearchQueryRequest::new("display settings", 42);
        let response = search_engine(request).expect("search engine response");

        assert_eq!(response.sequence, 42);
        assert_eq!(
            response.results.first().map(|result| result.id.as_str()),
            Some("setting:display")
        );
        assert_eq!(response.diagnostics.unwrap().legacy_hot_path_used, false);
    }

    #[test]
    fn command_reports_phase_four_providers_and_timings() {
        let request = SearchQueryRequest::new("spotify", 43);
        let response = search_engine(request).expect("search engine response");
        let providers = response
            .provider_timings
            .iter()
            .map(|timing| timing.provider_id)
            .collect::<Vec<_>>();

        assert!(providers.contains(&super::contracts::SearchProviderId::Settings));
        assert!(providers.contains(&super::contracts::SearchProviderId::Apps));
        assert!(providers.contains(&super::contracts::SearchProviderId::LocalFolders));
        assert!(providers.contains(&super::contracts::SearchProviderId::OpenWindows));
        assert!(providers.contains(&super::contracts::SearchProviderId::Everything));
        assert_eq!(
            response.diagnostics.unwrap().coordinator,
            "search_engine.phase5.rust_ranked"
        );
    }

    #[test]
    fn command_keeps_legacy_hot_paths_out_of_visible_engine() {
        let request = SearchQueryRequest::new("control panel", 44);
        let response = search_engine(request).expect("search engine response");
        let diagnostics = response.diagnostics.expect("diagnostics");

        assert!(!diagnostics.legacy_hot_path_used);
        assert!(diagnostics
            .notes
            .iter()
            .any(|note| note.contains("Legacy index, Windows Search, and cache")));
    }

    #[test]
    fn command_returns_open_window_context_results() {
        let mut request = SearchQueryRequest::new("terminal", 45);
        request.context.open_windows.push(super::contracts::SearchOpenWindowContext {
            id: "hwnd-1".to_string(),
            title: "Terminal - JasonShell".to_string(),
            app_name: Some("Windows Terminal".to_string()),
            executable_path: None,
        });

        let response = search_engine(request).expect("search engine response");

        assert_eq!(
            response
                .results
                .iter()
                .find(|result| result.id == "window:hwnd-1")
                .map(|result| result.kind),
            Some(super::contracts::SearchResultKind::Window)
        );
    }
}
