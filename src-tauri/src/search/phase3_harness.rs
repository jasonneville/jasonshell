use crate::search::contracts::{SearchProgressPhase, SearchProviderId, SearchQueryRequest};
use crate::search::providers::everything::{
    clear_everything_cache_for_test, test_search_everything_with_latest_gate,
};
use crate::search::test_observer::{self, SearchOperation};
use crate::search_sources::everything_ffi::{EverythingSdkRawResult, EverythingSdkResultKind};
use serde::Serialize;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

const QUERY_COUNT: u64 = 30;
const INPUTS_PER_QUERY: u64 = 4;
const BASELINE_STALE_TARGET: u64 = QUERY_COUNT * (INPUTS_PER_QUERY - 1);

#[derive(Clone, Debug, Serialize)]
struct Phase3ObservedEvent {
    group: u64,
    sequence: u64,
    latest_for_group: bool,
    everything_boundary_count: u64,
    sdk_entry_count: u64,
    discarded_as_stale: bool,
    completed: bool,
    queue_wait_ms: u64,
}

#[derive(Serialize)]
struct Phase3HarnessArtifact {
    phase: &'static str,
    method: &'static str,
    query_groups: u64,
    inputs_per_group: u64,
    baseline_stale_completions: u64,
    baseline_total_completions: u64,
    post_stale_everything_boundaries: u64,
    post_stale_sdk_entries: u64,
    post_total_everything_boundaries: u64,
    post_latest_completions: u64,
    latest_completion_target: u64,
    latest_queue_p95_ms: u64,
    recursive_scans: u64,
    stale_reduction_formula: &'static str,
    stale_reduction_score: f64,
    observed_events: Vec<Phase3ObservedEvent>,
    limitations: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) struct Phase3RegressionSummary {
    pub(crate) stale_everything_boundaries: u64,
    pub(crate) stale_sdk_entries: u64,
    pub(crate) latest_completions: u64,
    pub(crate) latest_completion_target: u64,
    pub(crate) recursive_scans: u64,
}

pub(crate) fn phase3_observed_regression_summary() -> Phase3RegressionSummary {
    Phase3RegressionSummary::from(&phase3_harness_artifact())
}

impl From<&Phase3HarnessArtifact> for Phase3RegressionSummary {
    fn from(artifact: &Phase3HarnessArtifact) -> Self {
        Self {
            stale_everything_boundaries: artifact.post_stale_everything_boundaries,
            stale_sdk_entries: artifact.post_stale_sdk_entries,
            latest_completions: artifact.post_latest_completions,
            latest_completion_target: artifact.latest_completion_target,
            recursive_scans: artifact.recursive_scans,
        }
    }
}

fn phase3_harness_artifact() -> Phase3HarnessArtifact {
    let _guard = super::phase3_registry_test_guard();
    clear_everything_cache_for_test();
    super::reset_latest_search_sequence_for_test(0);
    let _ = test_observer::take();

    let sdk_entry_counter = Arc::new(AtomicU64::new(0));
    let mut observed_events = Vec::with_capacity((QUERY_COUNT * INPUTS_PER_QUERY) as usize);
    let mut recursive_scans = 0;

    for group in 0..QUERY_COUNT {
        let group_base = group * INPUTS_PER_QUERY;
        let latest_sequence = group_base + INPUTS_PER_QUERY;
        let mut published_at = Vec::with_capacity(INPUTS_PER_QUERY as usize);

        for offset in 1..=INPUTS_PER_QUERY {
            let sequence = group_base + offset;
            let request = SearchQueryRequest::new(format!("phase3-{group}"), sequence);
            let queued_at = Instant::now();
            super::begin_search_engine_request(&request);
            published_at.push((request, queued_at));
        }

        let mut workers = Vec::with_capacity(INPUTS_PER_QUERY as usize);
        for (request, queued_at) in published_at {
            let sdk_entry_counter_for_run = Arc::clone(&sdk_entry_counter);
            workers.push(thread::spawn(move || {
                let worker_started_at = queued_at.elapsed().as_millis() as u64;
                let sequence = request.sequence;
                let sdk_entries_for_worker = Arc::new(AtomicU64::new(0));
                let sdk_entries_for_worker_run = Arc::clone(&sdk_entries_for_worker);
                let (boundary_before, recursive_before) = observed_operation_counts();
                let mut progress_events = Vec::new();

                let response = super::run_search_engine_latest_only_with_everything(
                    request,
                    |payload| progress_events.push(payload.phase),
                    |query, limit| {
                        test_search_everything_with_latest_gate(
                            query,
                            limit,
                            sequence,
                            |_, _| {
                                sdk_entry_counter_for_run.fetch_add(1, Ordering::SeqCst);
                                sdk_entries_for_worker_run.fetch_add(1, Ordering::SeqCst);
                                Ok(vec![EverythingSdkRawResult {
                                    full_path: PathBuf::from(format!(r"C:\phase3\{sequence}.txt")),
                                    kind: EverythingSdkResultKind::File,
                                    run_count: 1,
                                    highlighted_file_name: None,
                                }])
                            },
                            super::search_sequence_is_latest,
                        )
                    },
                );

                let everything_timing = response
                    .provider_timings
                    .iter()
                    .find(|timing| timing.provider_id == SearchProviderId::Everything)
                    .expect("coordinator must return Everything timing");
                let (boundary_after, recursive_after) = observed_operation_counts();
                let completed = progress_events
                    .iter()
                    .any(|phase| *phase == SearchProgressPhase::Complete);
                (
                    Phase3ObservedEvent {
                        group,
                        sequence,
                        latest_for_group: sequence == latest_sequence,
                        everything_boundary_count: boundary_after - boundary_before,
                        sdk_entry_count: sdk_entries_for_worker.load(Ordering::SeqCst),
                        discarded_as_stale: everything_timing.discarded_as_stale,
                        completed,
                        queue_wait_ms: worker_started_at,
                    },
                    recursive_before + recursive_after,
                )
            }));
        }

        for worker in workers {
            let (event, recursive_count) = worker.join().expect("phase3 worker");
            recursive_scans += recursive_count;
            observed_events.push(event);
        }
    }

    build_artifact(observed_events, recursive_scans)
}

fn observed_operation_counts() -> (u64, u64) {
    let events = test_observer::take();
    let boundary_count = events
        .iter()
        .filter(|event| **event == SearchOperation::EverythingBoundary)
        .count() as u64;
    let recursive_count = events
        .iter()
        .filter(|event| **event == SearchOperation::RecursiveFilesystemScan)
        .count() as u64;
    (boundary_count, recursive_count)
}

fn build_artifact(
    observed_events: Vec<Phase3ObservedEvent>,
    recursive_scans: u64,
) -> Phase3HarnessArtifact {
    let stale_events = observed_events
        .iter()
        .filter(|event| !event.latest_for_group)
        .collect::<Vec<_>>();
    let latest_events = observed_events
        .iter()
        .filter(|event| event.latest_for_group)
        .collect::<Vec<_>>();
    let post_stale_everything_boundaries = stale_events
        .iter()
        .map(|event| event.everything_boundary_count)
        .sum::<u64>();
    let post_stale_sdk_entries = stale_events
        .iter()
        .map(|event| event.sdk_entry_count)
        .sum::<u64>();
    let post_total_everything_boundaries = observed_events
        .iter()
        .map(|event| event.everything_boundary_count)
        .sum::<u64>();
    let post_latest_completions = latest_events
        .iter()
        .filter(|event| event.completed && !event.discarded_as_stale)
        .count() as u64;
    let mut latest_waits = latest_events
        .iter()
        .map(|event| event.queue_wait_ms)
        .collect::<Vec<_>>();
    latest_waits.sort_unstable();
    let latest_queue_p95_ms = percentile_ceil(&latest_waits, 95);
    let stale_reduction_score = (BASELINE_STALE_TARGET - post_stale_everything_boundaries) as f64
        / BASELINE_STALE_TARGET as f64;

    Phase3HarnessArtifact {
        phase: "Phase 3 latest-only backend cutoffs",
        method: "deterministic 30x4 orchestration through coordinator, latest registry, provider lock gate, and fake SDK seam",
        query_groups: QUERY_COUNT,
        inputs_per_group: INPUTS_PER_QUERY,
        baseline_stale_completions: BASELINE_STALE_TARGET,
        baseline_total_completions: QUERY_COUNT * INPUTS_PER_QUERY,
        post_stale_everything_boundaries,
        post_stale_sdk_entries,
        post_total_everything_boundaries,
        post_latest_completions,
        latest_completion_target: QUERY_COUNT,
        latest_queue_p95_ms,
        recursive_scans,
        stale_reduction_formula: "(90-post)/90",
        stale_reduction_score,
        observed_events,
        limitations: vec![
            "deterministic harness validates backend latest-only boundary behavior with fake SDK, not real Everything process latency",
            "no cancellation is claimed; stale work is skipped at backend gates only",
            "frontend immediate pending and stale gates are preserved by existing Node coverage, not measured here",
        ],
    }
}

fn percentile_ceil(values: &[u64], percentile: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() * percentile).saturating_add(99) / 100).saturating_sub(1);
    values[index.min(values.len() - 1)]
}

fn maybe_write_artifact(artifact: &Phase3HarnessArtifact) {
    let Some(outdir) = env::var_os("JASONSHELL_PHASE3_OUTDIR") else {
        return;
    };
    let outdir = PathBuf::from(outdir);
    fs::create_dir_all(&outdir).expect("create phase3 artifact dir");
    let json = serde_json::to_string_pretty(artifact).expect("serialize phase3 artifact");
    fs::write(outdir.join("phase3-latest-only-harness.json"), json).expect("write phase3 artifact");
}

#[test]
fn phase3_latest_only_harness_meets_user_targets() {
    let artifact = phase3_harness_artifact();

    assert_eq!(artifact.observed_events.len(), 120);
    assert_eq!(artifact.baseline_stale_completions, 90);
    assert_eq!(artifact.baseline_total_completions, 120);
    assert_eq!(artifact.post_stale_everything_boundaries, 0);
    assert_eq!(artifact.post_stale_sdk_entries, 0);
    assert_eq!(
        artifact.post_total_everything_boundaries, 30,
        "observed events: {:?}",
        artifact.observed_events
    );
    assert_eq!(artifact.post_latest_completions, 30);
    assert!(artifact.latest_queue_p95_ms <= 350);
    assert_eq!(artifact.recursive_scans, 0);
    assert_eq!(artifact.stale_reduction_score, 1.0);
    assert!(artifact
        .observed_events
        .iter()
        .filter(|event| event.latest_for_group)
        .all(|event| {
            event.everything_boundary_count == 1
                && event.sdk_entry_count == 1
                && !event.discarded_as_stale
        }));
    assert!(artifact
        .observed_events
        .iter()
        .filter(|event| !event.latest_for_group)
        .all(|event| {
            event.everything_boundary_count == 0
                && event.sdk_entry_count == 0
                && event.discarded_as_stale
        }));
}

#[test]
fn phase3_artifact_metrics_are_derived_from_observed_events() {
    let events = vec![
        Phase3ObservedEvent {
            group: 0,
            sequence: 1,
            latest_for_group: false,
            everything_boundary_count: 7,
            sdk_entry_count: 11,
            discarded_as_stale: false,
            completed: true,
            queue_wait_ms: 1,
        },
        Phase3ObservedEvent {
            group: 0,
            sequence: 4,
            latest_for_group: true,
            everything_boundary_count: 13,
            sdk_entry_count: 17,
            discarded_as_stale: false,
            completed: true,
            queue_wait_ms: 42,
        },
    ];

    let artifact = build_artifact(events, 5);

    assert_eq!(artifact.post_stale_everything_boundaries, 7);
    assert_eq!(artifact.post_stale_sdk_entries, 11);
    assert_eq!(artifact.post_total_everything_boundaries, 20);
    assert_eq!(artifact.post_latest_completions, 1);
    assert_eq!(artifact.latest_queue_p95_ms, 42);
    assert_eq!(artifact.recursive_scans, 5);
}

#[test]
#[ignore = "writes Phase3 artifact bundle when JASONSHELL_PHASE3_OUTDIR is set"]
fn phase3_latest_only_harness_writes_artifact_bundle() {
    let artifact = phase3_harness_artifact();
    maybe_write_artifact(&artifact);
}
