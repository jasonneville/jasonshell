//! Clean-slate Rust search subsystem for the search overhaul.
//!
//! This module is intentionally separate from `search_sources`; early overhaul
//! phases define contracts and local providers before the production hot path
//! is migrated.

#![allow(dead_code)]

pub(crate) mod contracts;
pub(crate) mod icons;
pub(crate) mod matcher;
pub(crate) mod providers;
pub(crate) mod scoring;

use contracts::{
    iso_now, SearchDiagnostics, SearchEngineResponse, SearchProgressPayload, SearchProgressPhase,
    SearchProviderCacheState, SearchProviderHealth, SearchProviderId, SearchProviderTiming,
    SearchQueryRequest, SearchResult,
};
use providers::apps::search_apps;
use providers::everything::search_everything;
use providers::local::search_local;
use providers::open_windows::search_open_windows;
use providers::settings::{search_settings, settings_provider_health};
use scoring::rank_visible_results;
use std::collections::HashMap;
use std::time::Instant;
use tauri::{AppHandle, Emitter};

const SEARCH_ENGINE_PROGRESS_EVENT: &str = "search-engine:progress";

#[tauri::command]
pub(crate) async fn search_engine(
    app_handle: AppHandle,
    request: SearchQueryRequest,
) -> Result<SearchEngineResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_search_engine(request, |payload| {
            let _ = app_handle.emit(SEARCH_ENGINE_PROGRESS_EVENT, payload);
        })
    })
    .await
    .map_err(|error| format!("search engine worker failed: {error}"))
}

fn run_search_engine(
    request: SearchQueryRequest,
    mut on_progress: impl FnMut(SearchProgressPayload),
) -> SearchEngineResponse {
    let query = request.query.trim().to_string();
    let limit = request.limit.clamp(1, 50);
    let mut local_rows = Vec::new();
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
        cache_age_ms: None,
        result_count: settings_results.len(),
        applied: true,
        discarded_as_stale: false,
    });
    health.push(settings_provider_health());
    local_rows.extend(settings_results);

    let apps_run = search_apps(&query, limit);
    provider_timings.push(apps_run.timing.clone());
    health.push(apps_run.health.clone());
    local_rows.extend(apps_run.results);

    let local_run = search_local(&query, limit, &request.context);
    provider_timings.push(local_run.timing.clone());
    health.push(local_run.health.clone());
    local_rows.extend(local_run.results);

    let open_windows_run = search_open_windows(&query, limit, &request.context.open_windows);
    provider_timings.push(open_windows_run.timing.clone());
    health.push(open_windows_run.health.clone());
    local_rows.extend(open_windows_run.results);

    let mut merged_rows = merge_result_batches(Vec::new(), local_rows);
    let local_ranked = rank_visible_results(&query, merged_rows.clone(), limit);
    on_progress(progress_payload(
        &request,
        SearchProgressPhase::Local,
        local_ranked.clone(),
        provider_timings.clone(),
        local_status_message(&local_ranked),
        false,
    ));

    let everything_run = search_everything(&query, limit);
    provider_timings.push(everything_run.timing.clone());
    let everything_health = everything_run.health.clone();
    let everything_results = everything_run.results;
    health.push(everything_health.clone());

    if !everything_results.is_empty() {
        merged_rows = merge_result_batches(merged_rows, everything_results);
        let provider_ranked = rank_visible_results(&query, merged_rows.clone(), limit);
        on_progress(progress_payload(
            &request,
            SearchProgressPhase::Provider,
            provider_ranked,
            provider_timings.clone(),
            "Merged Everything results".to_string(),
            false,
        ));
    } else if everything_health.state != contracts::SearchProviderHealthState::Ready {
        on_progress(progress_payload(
            &request,
            SearchProgressPhase::Error,
            local_ranked.clone(),
            provider_timings.clone(),
            provider_error_message(&everything_health),
            false,
        ));
    } else {
        on_progress(progress_payload(
            &request,
            SearchProgressPhase::Provider,
            local_ranked.clone(),
            provider_timings.clone(),
            "Everything finished with no new matches".to_string(),
            false,
        ));
    }

    let results = rank_visible_results(&query, merged_rows, limit);

    let response = SearchEngineResponse {
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
                "phase 2 emits progressive local-first updates before the final compatibility response".to_string(),
                "local providers publish before Everything, and Rust merges batches by stable record key".to_string(),
                "Legacy index, Windows Search, and cache display paths are not used by this command".to_string(),
            ],
        }),
    };

    on_progress(progress_payload(
        &request,
        SearchProgressPhase::Complete,
        response.results.clone(),
        response.provider_timings.clone(),
        if response.results.is_empty() {
            "No search results matched".to_string()
        } else {
            "Showing search results".to_string()
        },
        false,
    ));

    response
}

fn progress_payload(
    request: &SearchQueryRequest,
    phase: SearchProgressPhase,
    results: Vec<SearchResult>,
    provider_timings: Vec<SearchProviderTiming>,
    status_message: String,
    stale: bool,
) -> SearchProgressPayload {
    SearchProgressPayload {
        query: request.query.trim().to_string(),
        sequence: request.sequence,
        phase,
        results,
        provider_timings,
        status_message,
        generated_at: iso_now(),
        stale,
    }
}

fn local_status_message(results: &[SearchResult]) -> String {
    match results.len() {
        0 => "Searching local providers...".to_string(),
        1 => "1 local result".to_string(),
        count => format!("{count} local results"),
    }
}

fn provider_error_message(health: &SearchProviderHealth) -> String {
    health
        .message
        .clone()
        .unwrap_or_else(|| "Provider error while keeping local results".to_string())
}

fn merge_result_batches(
    current: Vec<SearchResult>,
    incoming: Vec<SearchResult>,
) -> Vec<SearchResult> {
    let mut merged = HashMap::new();
    let mut order = Vec::new();

    for result in current {
        let key = stable_result_key(&result);
        if !merged.contains_key(&key) {
            order.push(key.clone());
        }
        merged.insert(key, result);
    }

    for result in incoming {
        let key = stable_result_key(&result);
        if !merged.contains_key(&key) {
            order.push(key.clone());
        }
        merged.insert(key, result);
    }

    order
        .into_iter()
        .filter_map(|key| merged.remove(&key))
        .collect()
}

fn stable_result_key(result: &SearchResult) -> String {
    if result.record_key.trim().is_empty() {
        result.id.clone()
    } else {
        result.record_key.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::contracts::{SearchEngineResponse, SearchQueryRequest};
    use super::{run_search_engine, stable_result_key};

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
        let response = run_search_engine(request, |_| {});

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
        let response = run_search_engine(request, |_| {});
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
        let response = run_search_engine(request, |_| {});
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
        request
            .context
            .open_windows
            .push(super::contracts::SearchOpenWindowContext {
                id: "hwnd-1".to_string(),
                title: "Terminal - JasonShell".to_string(),
                app_name: Some("Windows Terminal".to_string()),
                executable_path: None,
                icon_data_url: None,
            });

        let response = run_search_engine(request, |_| {});

        assert_eq!(
            response
                .results
                .iter()
                .find(|result| result.id == "window:hwnd-1")
                .map(|result| result.kind),
            Some(super::contracts::SearchResultKind::Window)
        );
    }

    #[test]
    fn progressive_search_emits_local_then_provider_then_complete() {
        let request = SearchQueryRequest::new("display settings", 46);
        let mut phases = Vec::new();
        let response = run_search_engine(request, |payload| phases.push(payload));

        assert_eq!(
            phases.first().map(|payload| payload.phase),
            Some(super::contracts::SearchProgressPhase::Local)
        );
        assert!(phases
            .iter()
            .any(|payload| payload.phase == super::contracts::SearchProgressPhase::Provider));
        assert_eq!(
            phases.last().map(|payload| payload.phase),
            Some(super::contracts::SearchProgressPhase::Complete)
        );
        assert_eq!(
            phases.first().map(|payload| {
                payload.provider_timings.iter().any(|timing| {
                    timing.provider_id == super::contracts::SearchProviderId::Everything
                })
            }),
            Some(false)
        );
        assert_eq!(
            phases
                .last()
                .and_then(|payload| payload.results.first().map(|result| result.id.as_str())),
            response.results.first().map(|result| result.id.as_str())
        );
    }

    #[test]
    fn stable_result_key_prefers_record_key() {
        let result = super::contracts::SearchResult {
            id: "everything:file:c:/dev/test.txt".to_string(),
            provider_id: super::contracts::SearchProviderId::Everything,
            kind: super::contracts::SearchResultKind::File,
            title: "test.txt".to_string(),
            subtitle: None,
            path: Some(r"C:\dev\test.txt".to_string()),
            action: super::contracts::SearchResultAction::OpenFile {
                path: r"C:\dev\test.txt".to_string(),
            },
            terms: Vec::new(),
            aliases: Vec::new(),
            score: 0,
            match_reason: "token".to_string(),
            record_key: "file:c:\\dev\\test.txt".to_string(),
            title_highlight_data: Vec::new(),
            subtitle_highlight_data: Vec::new(),
            icon_data_url: None,
        };

        assert_eq!(stable_result_key(&result), "file:c:\\dev\\test.txt");
    }
}
