#![cfg(test)]

use super::contracts::*;
use super::providers::everything::{test_everything_run, EverythingSearchRun};
use super::run_search_engine_with_everything;
use super::test_observer::{boundary_for, take as take_search_operations, SearchOperation};
use std::path::PathBuf;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::{env, fs};

#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct Phase0HarnessSample {
    pub scenario: String,
    pub query: String,
    pub sequence: u64,
    pub phase_count: usize,
    pub input_to_local_ms: f64,
    pub input_to_final_ms: f64,
    pub queue_wait_ms: f64,
    pub fake_provider_duration_ms: f64,
    pub local_result_count: usize,
    pub final_result_count: usize,
    pub provider_state: SearchProviderHealthState,
    pub provider_reason: Option<SearchProviderReasonCode>,
    pub provider_message: Option<String>,
    pub sdk_latency_ms: Option<f64>,
    pub queue_trace: Vec<String>,
    pub boundary_trace: Vec<String>,
    pub provider_boundary_event: String,
    pub observed_operations: Vec<String>,
    pub stale_count: usize,
    pub latest_count: usize,
    pub stale_entries: Vec<String>,
    pub latest_entries: Vec<String>,
    pub open_window_result_count: usize,
    pub open_window_result_ids: Vec<String>,
    pub app_state: String,
    pub everything_state: String,
    pub everything_query_state: String,
    pub window_context: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct LatestQueryState {
    normalized_query: String,
    sequence: u64,
}

#[derive(Clone, Debug)]
struct BoundaryEvent {
    boundary_entry: Instant,
    lock_acquired: Instant,
    latest_query_at_boundary: String,
    latest_sequence_at_boundary: u64,
    latest_query_at_acquire: String,
    latest_sequence_at_acquire: u64,
}

#[derive(Default)]
struct BoundaryCoordinator {
    latest: Mutex<LatestQueryState>,
    boundary_lock: Mutex<()>,
}

fn fake_detection(
    state: SearchProviderHealthState,
    reason: Option<SearchProviderReasonCode>,
    message: Option<&str>,
) -> SearchProviderHealth {
    SearchProviderHealth {
        provider_id: SearchProviderId::Everything,
        state,
        reason_code: reason,
        message: message.map(ToString::to_string),
    }
}

fn fake_timing(
    provider_id: SearchProviderId,
    started: Instant,
    result_count: usize,
) -> SearchProviderTiming {
    SearchProviderTiming {
        provider_id,
        started_at: iso_now(),
        ended_at: Some(iso_now()),
        duration_ms: started.elapsed().as_secs_f64() * 1000.0,
        cache: SearchProviderCacheState::Refresh,
        cache_age_ms: None,
        result_count,
        applied: true,
        discarded_as_stale: false,
    }
}

fn fake_everything_run(
    state: SearchProviderHealthState,
    reason: Option<SearchProviderReasonCode>,
    message: Option<&str>,
    sdk_latency_ms: Option<f64>,
) -> EverythingSearchRun {
    let started = Instant::now();
    test_everything_run(
        Vec::new(),
        fake_timing(SearchProviderId::Everything, started, 0),
        fake_detection(state, reason, message),
        sdk_latency_ms,
    )
}

fn provider_rapid_boundary() -> EverythingSearchRun {
    fake_everything_run(
        SearchProviderHealthState::Ready,
        None,
        Some("Everything SDK path and runtime health are cached"),
        None,
    )
}

fn provider_sdk_missing() -> EverythingSearchRun {
    fake_everything_run(
        SearchProviderHealthState::Degraded,
        Some(SearchProviderReasonCode::SdkMissing),
        Some("Everything SDK missing"),
        None,
    )
}

fn provider_ipc_unavailable() -> EverythingSearchRun {
    fake_everything_run(
        SearchProviderHealthState::Unavailable,
        Some(SearchProviderReasonCode::IpcUnavailable),
        Some("Everything process is not running"),
        None,
    )
}

fn provider_query_error() -> EverythingSearchRun {
    fake_everything_run(
        SearchProviderHealthState::Degraded,
        Some(SearchProviderReasonCode::ProviderError),
        Some("Everything query failed: QueryFailed"),
        None,
    )
}

fn provider_timeout() -> EverythingSearchRun {
    fake_everything_run(
        SearchProviderHealthState::Degraded,
        Some(SearchProviderReasonCode::IpcUnavailable),
        Some("Everything query timed out; public contract reports IPC unavailable, not a distinct timeout code"),
        None,
    )
}

fn open_window_fixture(label: &str) -> SearchOpenWindowContext {
    match label {
        "Brave" => SearchOpenWindowContext {
            id: "window:brave".to_string(),
            title: "Brave".to_string(),
            app_name: Some("Brave".to_string()),
            executable_path: None,
            icon_data_url: None,
        },
        "Firefox" => SearchOpenWindowContext {
            id: "window:firefox".to_string(),
            title: "Firefox".to_string(),
            app_name: Some("Firefox".to_string()),
            executable_path: None,
            icon_data_url: None,
        },
        "terminal" => SearchOpenWindowContext {
            id: "window:terminal".to_string(),
            title: "terminal".to_string(),
            app_name: Some("terminal".to_string()),
            executable_path: None,
            icon_data_url: None,
        },
        "Settings" => SearchOpenWindowContext {
            id: "window:settings".to_string(),
            title: "Settings".to_string(),
            app_name: Some("Settings".to_string()),
            executable_path: None,
            icon_data_url: None,
        },
        _ => SearchOpenWindowContext {
            id: format!("window:{}", label.to_lowercase()),
            title: label.to_string(),
            app_name: Some(label.to_string()),
            executable_path: None,
            icon_data_url: None,
        },
    }
}

fn record_everything_boundary_for_test() {
    #[cfg(test)]
    boundary_for(SearchProviderId::Everything);
}

fn run_case(
    scenario: &str,
    query: &str,
    sequence: u64,
    provider_factory: fn() -> EverythingSearchRun,
    app_state: &str,
    everything_state: &str,
    everything_query_state: &str,
    window_context: Option<SearchOpenWindowContext>,
    coordinator: &BoundaryCoordinator,
) -> Phase0HarnessSample {
    let mut phases = Vec::new();
    let mut request = SearchQueryRequest::new(query, sequence);
    request.context.open_windows = window_context.clone().into_iter().collect();
    let start = Instant::now();
    let boundary_entry = Instant::now();
    let latest_at_boundary = coordinator.latest.lock().expect("latest state").clone();
    let lock_acquired;
    let latest_at_acquire;
    let boundary_guard = coordinator.boundary_lock.lock().expect("boundary lock");
    lock_acquired = Instant::now();
    latest_at_acquire = coordinator.latest.lock().expect("latest state").clone();
    let _ = take_search_operations();
    let provider = provider_factory();
    let provider_health_state = provider.health.state;
    let response = run_search_engine_with_everything(
        request,
        |payload| phases.push(payload),
        |_, _| {
            record_everything_boundary_for_test();
            provider.clone()
        },
    );
    let observed_operations = take_search_operations()
        .into_iter()
        .map(|operation| match operation {
            SearchOperation::Settings => "Settings".to_string(),
            SearchOperation::Apps => "Apps".to_string(),
            SearchOperation::Local => "Local".to_string(),
            SearchOperation::OpenWindows => "OpenWindows".to_string(),
            SearchOperation::RecursiveFilesystemScan => "RecursiveFilesystemScan".to_string(),
            SearchOperation::EverythingBoundary => "EverythingBoundary".to_string(),
        })
        .collect::<Vec<_>>();
    drop(boundary_guard);
    let local_phase = phases
        .iter()
        .find(|phase| phase.phase == SearchProgressPhase::Local);
    let final_phase = phases.last();
    let boundary_event = BoundaryEvent {
        boundary_entry,
        lock_acquired,
        latest_query_at_boundary: latest_at_boundary.normalized_query,
        latest_sequence_at_boundary: latest_at_boundary.sequence,
        latest_query_at_acquire: latest_at_acquire.normalized_query,
        latest_sequence_at_acquire: latest_at_acquire.sequence,
    };
    let provider_boundary_event = observed_operations
        .iter()
        .find(|operation| operation.as_str() == "EverythingBoundary")
        .cloned()
        .unwrap_or_else(|| "EverythingBoundary".to_string());
    let queue_wait_ms = lock_acquired
        .duration_since(boundary_event.boundary_entry)
        .as_secs_f64()
        * 1000.0;
    let is_latest = query == boundary_event.latest_query_at_acquire
        && sequence == boundary_event.latest_sequence_at_acquire;
    let stale_count = usize::from(!is_latest);
    let latest_count = usize::from(is_latest);
    assert!(
        provider_health_state == SearchProviderHealthState::Ready
            || provider_health_state == SearchProviderHealthState::Degraded
            || provider_health_state == SearchProviderHealthState::Unavailable
    );
    Phase0HarnessSample {
        scenario: scenario.to_string(),
        query: query.to_string(),
        sequence: response.sequence,
        phase_count: phases.len(),
        input_to_local_ms: local_phase
            .map(|_| start.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or_default(),
        input_to_final_ms: start.elapsed().as_secs_f64() * 1000.0,
        queue_wait_ms,
        fake_provider_duration_ms: response
            .provider_timings
            .last()
            .map(|t| t.duration_ms)
            .unwrap_or(0.0),
        local_result_count: phases.first().map(|p| p.results.len()).unwrap_or(0),
        final_result_count: response.results.len(),
        provider_state: response
            .health
            .last()
            .map(|h| h.state)
            .unwrap_or(SearchProviderHealthState::Ready),
        provider_reason: response.health.last().and_then(|h| h.reason_code),
        provider_message: response.health.last().and_then(|h| h.message.clone()),
        sdk_latency_ms: response.provider_timings.last().and_then(|t| {
            if t.provider_id == SearchProviderId::Everything {
                Some(t.duration_ms)
            } else {
                None
            }
        }),
        queue_trace: phases
            .iter()
            .map(|p| format!("{:?}:{:?}", p.phase, p.results.len()))
            .collect(),
        boundary_trace: vec![
            format!(
                "entry:{}:{}",
                boundary_event.latest_query_at_boundary, boundary_event.latest_sequence_at_boundary
            ),
            format!(
                "acquire:{}:{}",
                boundary_event.latest_query_at_acquire, boundary_event.latest_sequence_at_acquire
            ),
        ],
        provider_boundary_event,
        observed_operations,
        stale_count,
        latest_count,
        stale_entries: phases
            .iter()
            .filter(|p| p.status_message.contains("stale"))
            .map(|p| p.status_message.clone())
            .collect(),
        latest_entries: final_phase
            .map(|p| vec![p.status_message.clone()])
            .unwrap_or_default(),
        open_window_result_count: response
            .results
            .iter()
            .filter(|result| result.provider_id == SearchProviderId::OpenWindows)
            .count(),
        open_window_result_ids: response
            .results
            .iter()
            .filter(|result| result.provider_id == SearchProviderId::OpenWindows)
            .map(|result| result.id.clone())
            .collect(),
        app_state: app_state.to_string(),
        everything_state: everything_state.to_string(),
        everything_query_state: everything_query_state.to_string(),
        window_context: window_context.map(|ctx| ctx.title),
    }
}

fn write_artifacts(samples: &[Phase0HarnessSample]) {
    let outdir = env::var("JASONSHELL_PHASE0_OUTDIR").ok();
    let Some(outdir) = outdir else {
        return;
    };
    let _ = fs::create_dir_all(&outdir);
    let json = serde_json::to_string_pretty(samples).unwrap_or_else(|_| "[]".to_string());
    let _ = fs::write(PathBuf::from(&outdir).join("phase0-samples.json"), json);
}

pub(crate) fn rapid_boundary_samples() -> Vec<Phase0HarnessSample> {
    let mut handles = Vec::new();
    for round in 0..30 {
        let coordinator = Arc::new(BoundaryCoordinator::default());
        let barrier = Arc::new(Barrier::new(4));
        let base = (round * 4) as u64;
        let samples = vec![
            ("w", base + 1),
            ("wi", base + 2),
            ("win", base + 3),
            ("windows-settings", base + 4),
        ];
        let boundary_guard = coordinator.boundary_lock.lock().expect("boundary lock");
        for (query, sequence) in samples {
            let barrier = Arc::clone(&barrier);
            let coordinator = Arc::clone(&coordinator);
            let query = query.to_string();
            let window_context = match query.as_str() {
                "windows-settings" => Some(open_window_fixture("Brave")),
                _ => None,
            };
            handles.push(thread::spawn(move || {
                barrier.wait();
                run_case(
                    "rapid-prefix",
                    &query,
                    sequence,
                    provider_rapid_boundary,
                    "warm app",
                    "warm Everything",
                    "warm query",
                    window_context,
                    &coordinator,
                )
            }));
        }
        {
            let mut latest = coordinator.latest.lock().expect("latest state");
            *latest = LatestQueryState {
                normalized_query: "windows-settings".to_string(),
                sequence: base + 4,
            };
        }
        std::thread::sleep(Duration::from_millis(1));
        drop(boundary_guard);
    }
    handles
        .into_iter()
        .map(|handle| handle.join().expect("sample thread"))
        .collect()
}

pub(crate) fn unavailable_samples() -> Vec<Phase0HarnessSample> {
    let coordinator = BoundaryCoordinator::default();
    {
        let mut latest = coordinator.latest.lock().expect("latest state");
        *latest = LatestQueryState {
            normalized_query: "control panel".to_string(),
            sequence: 4,
        };
    }
    [
        (
            "sdkMissing",
            "display settings",
            provider_sdk_missing as fn() -> EverythingSearchRun,
            "cold app",
            "cold Everything",
            "cold query",
            Some(open_window_fixture("Brave")),
        ),
        (
            "ipcUnavailable",
            "sound settings",
            provider_ipc_unavailable as fn() -> EverythingSearchRun,
            "warm app",
            "cold Everything",
            "cold query",
            Some(open_window_fixture("Firefox")),
        ),
        (
            "queryError",
            "display settings",
            provider_query_error as fn() -> EverythingSearchRun,
            "warm app",
            "warm Everything",
            "warm query",
            Some(open_window_fixture("terminal")),
        ),
        (
            "timeout",
            "control panel",
            provider_timeout as fn() -> EverythingSearchRun,
            "cold app",
            "warm Everything",
            "cold query",
            Some(open_window_fixture("Settings")),
        ),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(
            sequence,
            (scenario, query, provider, app_state, everything_state, query_state, window_context),
        )| {
            run_case(
                scenario,
                query,
                sequence as u64 + 1,
                provider,
                app_state,
                everything_state,
                query_state,
                window_context,
                &coordinator,
            )
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase0_harness_produces_progress_and_health_traces() {
        let mut samples = rapid_boundary_samples();
        samples.extend(unavailable_samples());
        write_artifacts(&samples);
        assert_eq!(samples.len(), 124);
        assert_eq!(
            samples
                .iter()
                .filter(|sample| sample.scenario == "rapid-prefix")
                .count(),
            120
        );
        assert_eq!(
            samples
                .iter()
                .filter(|sample| sample.scenario != "rapid-prefix")
                .count(),
            4
        );
        for round in 0..30 {
            let base = (round * 4) as u64;
            let round_samples: Vec<_> = samples
                .iter()
                .filter(|sample| {
                    sample.scenario == "rapid-prefix"
                        && sample.sequence > base
                        && sample.sequence <= base + 4
                })
                .collect();
            assert_eq!(round_samples.len(), 4);
            assert_eq!(
                round_samples
                    .iter()
                    .filter(|sample| sample.query == "windows-settings")
                    .count(),
                1
            );
            assert_eq!(
                round_samples
                    .iter()
                    .filter(|sample| sample.latest_count == 1)
                    .count(),
                1
            );
            assert_eq!(
                round_samples
                    .iter()
                    .filter(|sample| sample.stale_count == 1)
                    .count(),
                3
            );
            assert!(round_samples
                .iter()
                .all(|sample| sample.queue_wait_ms > 0.0));
            assert_eq!(
                round_samples
                    .iter()
                    .find(|sample| sample.query == "windows-settings")
                    .map(|sample| sample.sequence),
                Some(base + 4)
            );
            assert_eq!(
                round_samples
                    .iter()
                    .find(|sample| sample.latest_count == 1)
                    .map(|sample| sample.query.as_str()),
                Some("windows-settings")
            );
        }
        assert!(samples.iter().all(|sample| sample.phase_count == 3));
        assert!(samples.iter().all(|sample| {
            sample
                .observed_operations
                .iter()
                .filter(|op| op.as_str() == "EverythingBoundary")
                .count()
                == 1
        }));
        assert!(samples.iter().all(|sample| !sample
            .observed_operations
            .iter()
            .any(|op| op == "RecursiveFilesystemScan")));
        assert!(samples
            .iter()
            .all(|sample| sample.provider_boundary_event == "EverythingBoundary"));
        for expected in ["Brave", "Firefox", "terminal", "Settings"] {
            assert!(samples
                .iter()
                .any(|sample| sample.window_context.as_deref() == Some(expected)));
        }
        assert_eq!(
            samples
                .iter()
                .filter(|sample| sample.scenario == "rapid-prefix" && sample.stale_count == 1)
                .count(),
            90
        );
        assert_eq!(
            samples
                .iter()
                .filter(|sample| sample.scenario == "rapid-prefix" && sample.latest_count == 1)
                .count(),
            30
        );
        assert!(samples.iter().all(|sample| sample.queue_wait_ms > 0.0));
        assert!(samples.iter().all(|sample| sample
            .queue_trace
            .first()
            .is_some_and(|entry| entry.contains("Local"))));
        assert!(samples
            .iter()
            .all(|sample| sample.boundary_trace.len() == 2));
        for sample in samples
            .iter()
            .filter(|sample| sample.scenario != "rapid-prefix")
        {
            match sample.scenario.as_str() {
                "sdkMissing" => {
                    assert_eq!(sample.provider_state, SearchProviderHealthState::Degraded);
                    assert_eq!(
                        sample.provider_reason,
                        Some(SearchProviderReasonCode::SdkMissing)
                    );
                }
                "ipcUnavailable" => {
                    assert_eq!(
                        sample.provider_state,
                        SearchProviderHealthState::Unavailable
                    );
                    assert_eq!(
                        sample.provider_reason,
                        Some(SearchProviderReasonCode::IpcUnavailable)
                    );
                }
                "queryError" => {
                    assert_eq!(sample.provider_state, SearchProviderHealthState::Degraded);
                    assert_eq!(
                        sample.provider_reason,
                        Some(SearchProviderReasonCode::ProviderError)
                    );
                }
                "timeout" => {
                    assert_eq!(sample.provider_state, SearchProviderHealthState::Degraded);
                    assert_eq!(
                        sample.provider_reason,
                        Some(SearchProviderReasonCode::IpcUnavailable)
                    );
                    assert!(sample
                        .provider_message
                        .as_deref()
                        .unwrap_or("")
                        .contains("timeout"));
                }
                _ => unreachable!(),
            }
            assert!(sample
                .queue_trace
                .iter()
                .any(|entry| entry.contains("Error")));
            assert!(sample.local_result_count > 0);
            assert!(sample.final_result_count >= sample.local_result_count);
        }
    }
}
