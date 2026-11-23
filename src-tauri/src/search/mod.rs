//! Clean-slate Rust search subsystem for the search overhaul.
//!
//! This module is intentionally separate from `search_sources`; early overhaul
//! phases define contracts and local providers before the production hot path
//! is migrated.

#![allow(dead_code)]

pub(crate) mod contracts;
pub(crate) mod icons;
pub(crate) mod matcher;
#[cfg(test)]
pub(crate) mod phase0_harness;
#[cfg(test)]
pub(crate) mod phase3_harness;
#[cfg(test)]
pub(crate) mod phase4_harness;
pub(crate) mod providers;
pub(crate) mod scoring;

#[cfg(test)]
pub(crate) mod test_observer {
    use super::contracts::SearchProviderId;
    use std::cell::RefCell;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) enum SearchOperation {
        Settings,
        Apps,
        Local,
        OpenWindows,
        RecursiveFilesystemScan,
        EverythingBoundary,
    }

    thread_local! {
        static EVENTS: RefCell<Vec<SearchOperation>> = const { RefCell::new(Vec::new()) };
    }

    pub(crate) fn record(operation: SearchOperation) {
        EVENTS.with(|events| events.borrow_mut().push(operation));
    }

    pub(crate) fn take() -> Vec<SearchOperation> {
        EVENTS.with(|events| events.replace(Vec::new()))
    }

    pub(crate) fn boundary_for(provider_id: SearchProviderId) {
        if provider_id == SearchProviderId::Everything {
            record(SearchOperation::EverythingBoundary);
        }
    }
}

use contracts::{
    iso_now, SearchDiagnostics, SearchEngineResponse, SearchProgressPayload, SearchProgressPhase,
    SearchProviderCacheState, SearchProviderHealth, SearchProviderId, SearchProviderTiming,
    SearchQueryRequest, SearchResult,
};
use providers::apps::search_apps;
use providers::bookmarks::{search_known_bookmarks, BookmarkCache};
use providers::calculator::search_calculator;
use providers::everything::{search_everything, search_everything_latest_only};
use providers::local::search_local;
use providers::open_windows::search_open_windows;
use providers::quick_commands::search_quick_commands;
use providers::settings::{search_settings, settings_provider_health};
use providers::web::{search_web, WEB_SEARCH_ID};
use scoring::{is_final_only_row, rank_visible_results};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(test)]
use std::sync::MutexGuard;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const SEARCH_ENGINE_PROGRESS_EVENT: &str = "search-engine:progress";
static LATEST_SEARCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const SETTINGS_RETRY_INTERVAL: Duration = Duration::from_secs(2);
static BOOKMARK_CACHE: OnceLock<Arc<BookmarkCache>> = OnceLock::new();
static PROVIDER_INPUTS_CACHE: Mutex<ProviderInputsCache> = Mutex::new(ProviderInputsCache {
    stamp: None,
    inputs: None,
    failed: None,
});

/// Inputs parsed from one exact settings-file revision, so a keystroke costs a stat, not a parse.
struct ProviderInputsCache {
    stamp: Option<crate::settings::SettingsFileStamp>,
    inputs: Option<Arc<SearchProviderInputs>>,
    failed: Option<(Option<crate::settings::SettingsFileStamp>, Instant)>,
}
#[cfg(test)]
static PHASE3_REGISTRY_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Clone, Debug, Default)]
pub(crate) struct SearchProviderInputs {
    pub(crate) web_engine: crate::settings::WebSearchEngine,
    pub(crate) web_search_disabled: bool,
    pub(crate) quick_commands: Vec<crate::settings::QuickCommandEntry>,
    /// `None` disables the bookmarks provider.
    pub(crate) bookmarks_local_app_data: Option<std::path::PathBuf>,
}

#[tauri::command]
pub(crate) async fn search_engine(
    app_handle: AppHandle,
    request: SearchQueryRequest,
) -> Result<SearchEngineResponse, String> {
    begin_search_engine_request(&request);
    tauri::async_runtime::spawn_blocking(move || {
        let inputs = load_search_provider_inputs(&app_handle);
        run_search_engine_latest_only(request, |payload| {
            let _ = app_handle.emit(SEARCH_ENGINE_PROGRESS_EVENT, payload);
        }, &inputs)
    })
    .await
    .map_err(|error| format!("search engine worker failed: {error}"))
}

/// Runs on the blocking search worker. Unreadable settings never fail a search.
fn load_search_provider_inputs(app_handle: &AppHandle) -> Arc<SearchProviderInputs> {
    // Stamped before the read: a concurrent save can only make the cache look stale, never fresh.
    let stamp = crate::settings::settings_file_stamp(app_handle);
    let mut cache = PROVIDER_INPUTS_CACHE
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    let retry_later = cache
        .failed
        .as_ref()
        .is_some_and(|(failed, at)| *failed == stamp && at.elapsed() < SETTINGS_RETRY_INTERVAL);
    let current = stamp.is_some() && cache.stamp == stamp;
    if let Some(inputs) = cache.inputs.as_ref().filter(|_| current || retry_later) {
        return Arc::clone(inputs);
    }
    if !retry_later {
        match crate::settings::load_shell_settings_for_app(app_handle) {
            Ok(settings) => {
                let inputs = Arc::new(provider_inputs_from(Ok(settings)));
                *cache = ProviderInputsCache {
                    stamp,
                    inputs: Some(Arc::clone(&inputs)),
                    failed: None,
                };
                return inputs;
            }
            // Keep the last good inputs; a failing file is not re-parsed on every keystroke.
            Err(_) => cache.failed = Some((stamp, Instant::now())),
        }
    }
    match &cache.inputs {
        Some(inputs) => Arc::clone(inputs),
        None => Arc::new(provider_inputs_from(Err(String::new()))),
    }
}

fn provider_inputs_from(
    loaded: Result<crate::settings::ShellSettings, String>,
) -> SearchProviderInputs {
    SearchProviderInputs {
        // Without any known choice, do not guess which engine receives the query.
        web_search_disabled: loaded.is_err(),
        quick_commands: loaded
            .as_ref()
            .map(|settings| settings.quick_commands.entries.clone())
            .unwrap_or_default(),
        web_engine: web_engine_after_settings_load(Default::default(), loaded),
        bookmarks_local_app_data: std::env::var_os("LOCALAPPDATA")
            .filter(|root| !root.is_empty())
            .map(std::path::PathBuf::from),
    }
}

pub(crate) fn web_engine_after_settings_load(
    last_good: crate::settings::WebSearchEngine,
    loaded: Result<crate::settings::ShellSettings, String>,
) -> crate::settings::WebSearchEngine {
    loaded.map_or(last_good, |settings| settings.search.web_engine)
}

#[cfg(test)]
pub(crate) fn run_published_search_engine_latest_only_with_everything(
    request: SearchQueryRequest,
    on_progress: impl FnMut(SearchProgressPayload),
    search_everything_impl: impl Fn(&str, usize) -> providers::everything::EverythingSearchRun,
) -> SearchEngineResponse {
    begin_search_engine_request(&request);
    run_search_engine_latest_only_with_everything(request, on_progress, search_everything_impl)
}

fn run_search_engine(
    request: SearchQueryRequest,
    on_progress: impl FnMut(SearchProgressPayload),
) -> SearchEngineResponse {
    run_search_engine_with_everything(request, on_progress, search_everything)
}

fn run_search_engine_latest_only(
    request: SearchQueryRequest,
    on_progress: impl FnMut(SearchProgressPayload),
    inputs: &SearchProviderInputs,
) -> SearchEngineResponse {
    let sequence = request.sequence;
    run_search_engine_core(
        request,
        inputs,
        || search_sequence_is_latest(sequence),
        on_progress,
        move |query, limit| {
            search_everything_latest_only(query, limit, sequence, search_sequence_is_latest)
        },
    )
}

#[cfg(test)]
pub(crate) fn run_search_engine_latest_only_with_everything(
    request: SearchQueryRequest,
    on_progress: impl FnMut(SearchProgressPayload),
    search_everything_impl: impl Fn(&str, usize) -> providers::everything::EverythingSearchRun,
) -> SearchEngineResponse {
    run_search_engine_with_everything(request, on_progress, search_everything_impl)
}

#[cfg(not(test))]
fn run_search_engine_latest_only_with_everything(
    request: SearchQueryRequest,
    on_progress: impl FnMut(SearchProgressPayload),
    search_everything_impl: impl Fn(&str, usize) -> providers::everything::EverythingSearchRun,
) -> SearchEngineResponse {
    run_search_engine_with_everything(request, on_progress, search_everything_impl)
}

pub(crate) fn begin_search_engine_request(request: &SearchQueryRequest) {
    publish_latest_search_sequence(request.sequence);
}

pub(crate) fn publish_latest_search_sequence(sequence: u64) {
    let mut current = LATEST_SEARCH_SEQUENCE.load(Ordering::Acquire);
    while sequence > current {
        match LATEST_SEARCH_SEQUENCE.compare_exchange(
            current,
            sequence,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return,
            Err(next) => current = next,
        }
    }
}

pub(crate) fn search_sequence_is_latest(sequence: u64) -> bool {
    sequence >= LATEST_SEARCH_SEQUENCE.load(Ordering::Acquire)
}

#[cfg(test)]
fn reset_latest_search_sequence_for_test(sequence: u64) {
    LATEST_SEARCH_SEQUENCE.store(sequence, Ordering::Release);
}

#[cfg(test)]
pub(crate) fn phase3_registry_test_guard() -> MutexGuard<'static, ()> {
    PHASE3_REGISTRY_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("phase3 registry test lock")
}

fn run_search_engine_with_everything(
    request: SearchQueryRequest,
    on_progress: impl FnMut(SearchProgressPayload),
    search_everything_impl: impl Fn(&str, usize) -> providers::everything::EverythingSearchRun,
) -> SearchEngineResponse {
    run_search_engine_with_inputs(
        request,
        &SearchProviderInputs::default(),
        on_progress,
        search_everything_impl,
    )
}

pub(crate) fn run_search_engine_with_inputs(
    request: SearchQueryRequest,
    inputs: &SearchProviderInputs,
    on_progress: impl FnMut(SearchProgressPayload),
    search_everything_impl: impl Fn(&str, usize) -> providers::everything::EverythingSearchRun,
) -> SearchEngineResponse {
    run_search_engine_core(request, inputs, || true, on_progress, search_everything_impl)
}

fn run_search_engine_core(
    request: SearchQueryRequest,
    inputs: &SearchProviderInputs,
    is_latest: impl Fn() -> bool,
    mut on_progress: impl FnMut(SearchProgressPayload),
    search_everything_impl: impl Fn(&str, usize) -> providers::everything::EverythingSearchRun,
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

    let calculator_run = search_calculator(&query, limit);
    provider_timings.push(calculator_run.timing);
    health.push(calculator_run.health);
    local_rows.extend(calculator_run.results);

    let quick_commands_run = search_quick_commands(&query, limit, &inputs.quick_commands);
    provider_timings.push(quick_commands_run.timing);
    health.push(quick_commands_run.health);
    local_rows.extend(quick_commands_run.results);

    // Only the last completed index here: bookmark disk I/O must never delay the Local payload.
    let bookmark_cache = BOOKMARK_CACHE.get_or_init(|| Arc::new(BookmarkCache::new()));
    if let Some(local_app_data) = inputs.bookmarks_local_app_data.as_deref() {
        let bookmarks_run = search_known_bookmarks(&query, limit, local_app_data, bookmark_cache);
        provider_timings.push(bookmarks_run.timing);
        health.push(bookmarks_run.health);
        local_rows.extend(bookmarks_run.results);
    }

    let web_run = search_web(&query, limit, inputs.web_engine);
    provider_timings.push(web_run.timing);
    health.push(web_run.health);
    local_rows.extend(
        web_run
            .results
            .into_iter()
            .filter(|row| !inputs.web_search_disabled || row.id != WEB_SEARCH_ID),
    );

    let mut merged_rows = merge_result_batches(Vec::new(), local_rows);
    let local_ranked = rank_interim_results(&query, &merged_rows, limit);
    on_progress(progress_payload(
        &request,
        SearchProgressPhase::Local,
        local_ranked.clone(),
        provider_timings.clone(),
        local_status_message(&local_ranked),
        false,
    ));
    if let Some(local_app_data) = inputs.bookmarks_local_app_data.as_deref() {
        if !query.is_empty() && is_latest() {
            bookmark_cache.refresh_if_due(local_app_data);
        }
    }

    let everything_run = search_everything_impl(&query, limit);
    provider_timings.push(everything_run.timing.clone());
    let everything_health = everything_run.health.clone();
    let everything_results = everything_run.results;
    health.push(everything_health.clone());

    if !everything_results.is_empty() {
        merged_rows = merge_result_batches(merged_rows, everything_results);
        let provider_ranked = rank_interim_results(&query, &merged_rows, limit);
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

    if let Some(local_app_data) = inputs.bookmarks_local_app_data.as_deref() {
        if !query.is_empty() && is_latest() {
            bookmark_cache.wait_for_first_index(local_app_data, &is_latest);
        }
        let bookmarks_run = search_known_bookmarks(&query, limit, local_app_data, bookmark_cache);
        if let Some(timing) = provider_timings
            .iter_mut()
            .find(|timing| timing.provider_id == SearchProviderId::Bookmarks)
        {
            *timing = bookmarks_run.timing;
        }
        merged_rows.retain(|row| row.provider_id != SearchProviderId::Bookmarks);
        merged_rows = merge_result_batches(merged_rows, bookmarks_run.results);
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

/// Ranking for every payload before the final one.
fn rank_interim_results(query: &str, rows: &[SearchResult], limit: usize) -> Vec<SearchResult> {
    let rows = rows
        .iter()
        .filter(|row| !is_final_only_row(row))
        .cloned()
        .collect();
    rank_visible_results(query, rows, limit)
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
    use super::{run_search_engine, run_search_engine_with_everything, stable_result_key};

    #[test]
    fn phase3_latest_registry_is_monotonic_and_older_sequences_become_stale() {
        let _guard = super::phase3_registry_test_guard();
        super::reset_latest_search_sequence_for_test(0);

        super::publish_latest_search_sequence(90);
        super::publish_latest_search_sequence(30);

        assert!(super::search_sequence_is_latest(90));
        assert!(!super::search_sequence_is_latest(89));
    }

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
        let response = run_search_engine_with_everything(
            request,
            |payload| phases.push(payload),
            |_, _| {
                super::providers::everything::test_everything_run(
                    Vec::new(),
                    super::contracts::SearchProviderTiming {
                        provider_id: super::contracts::SearchProviderId::Everything,
                        started_at: super::contracts::iso_now(),
                        ended_at: Some(super::contracts::iso_now()),
                        duration_ms: 0.0,
                        cache: super::contracts::SearchProviderCacheState::Miss,
                        cache_age_ms: None,
                        result_count: 0,
                        applied: true,
                        discarded_as_stale: false,
                    },
                    super::contracts::SearchProviderHealth {
                        provider_id: super::contracts::SearchProviderId::Everything,
                        state: super::contracts::SearchProviderHealthState::Ready,
                        reason_code: None,
                        message: Some("deterministic fake".to_string()),
                    },
                    None,
                )
            },
        );

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
    fn search_engine_preserves_local_results_when_everything_is_unavailable() {
        let request = SearchQueryRequest::new("display settings", 47);
        let response = run_search_engine_with_everything(
            request,
            |_| {},
            |_, _| {
                super::providers::everything::test_everything_run(
                    Vec::new(),
                    super::contracts::SearchProviderTiming {
                        provider_id: super::contracts::SearchProviderId::Everything,
                        started_at: super::contracts::iso_now(),
                        ended_at: Some(super::contracts::iso_now()),
                        duration_ms: 0.0,
                        cache: super::contracts::SearchProviderCacheState::Miss,
                        cache_age_ms: None,
                        result_count: 0,
                        applied: true,
                        discarded_as_stale: false,
                    },
                    super::contracts::SearchProviderHealth {
                        provider_id: super::contracts::SearchProviderId::Everything,
                        state: super::contracts::SearchProviderHealthState::Unavailable,
                        reason_code: Some(
                            super::contracts::SearchProviderReasonCode::IpcUnavailable,
                        ),
                        message: Some("Everything unavailable".to_string()),
                    },
                    None,
                )
            },
        );

        assert_eq!(
            response.results.first().map(|result| result.id.as_str()),
            Some("setting:display")
        );
        assert!(response
            .health
            .iter()
            .any(|health| health.provider_id == super::contracts::SearchProviderId::Everything));
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
            provider_signal: 0,
            match_reason: "token".to_string(),
            record_key: "file:c:\\dev\\test.txt".to_string(),
            title_highlight_data: Vec::new(),
            subtitle_highlight_data: Vec::new(),
            icon_data_url: None,
        };

        assert_eq!(stable_result_key(&result), "file:c:\\dev\\test.txt");
    }
}
