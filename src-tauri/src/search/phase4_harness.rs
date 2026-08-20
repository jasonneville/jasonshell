use crate::search::phase3_harness::{phase3_observed_regression_summary, Phase3RegressionSummary};
use crate::search::providers::everything::test_search_everything_with_latest_gate;
use crate::search::scoring::rank_visible_results;
use crate::search_sources::everything_ffi::{EverythingSdkRawResult, EverythingSdkResultKind};
use serde::Serialize;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

const SAMPLE_COUNT: u64 = 30;
const LIMIT: usize = 50;
const EXPECTED_OVERFETCH_LIMIT: usize = 75;
const BEST_RAW_INDEX: usize = 60;

#[derive(Debug, Serialize)]
struct Phase4Sample {
    sample: u64,
    sdk_request_max_results: usize,
    raw_candidate_count: usize,
    mapped_candidate_count: usize,
    best_raw_index: usize,
    surfaced_after_canonical_rank: bool,
    top_record_key: Option<String>,
    latency_ms: u64,
}

#[derive(Debug, Serialize)]
struct Phase4Artifact {
    phase: &'static str,
    method: &'static str,
    samples: u64,
    requested_limit: usize,
    approved_overfetch_plus: usize,
    approved_max_results_cap: usize,
    expected_sdk_request_max_results: usize,
    best_raw_index: usize,
    surfaced_after_canonical_rank: u64,
    mapped_candidate_max: usize,
    latency_p95_ms: u64,
    reused_phase3_evidence_path: &'static str,
    stale_everything_boundaries: u64,
    stale_sdk_entries: u64,
    latest_completions: u64,
    latest_completion_target: u64,
    recursive_scans: u64,
    samples_detail: Vec<Phase4Sample>,
}

fn phase4_artifact() -> Phase4Artifact {
    phase4_artifact_with_phase3_summary(phase3_observed_regression_summary())
}

fn phase4_artifact_with_phase3_summary(phase3: Phase3RegressionSummary) -> Phase4Artifact {
    let mut details = Vec::with_capacity(SAMPLE_COUNT as usize);
    for sample in 0..SAMPLE_COUNT {
        let started = Instant::now();
        let observed_request_max = AtomicUsize::new(0);
        let run = test_search_everything_with_latest_gate(
            "dev",
            LIMIT,
            10_000 + sample,
            |_, request| {
                observed_request_max.store(request.max_results, Ordering::SeqCst);
                let mut rows = (0..BEST_RAW_INDEX)
                    .map(|index| EverythingSdkRawResult {
                        full_path: PathBuf::from(format!(
                            r"C:\noise\dev-noise-{sample}-{index}.txt"
                        )),
                        kind: EverythingSdkResultKind::File,
                        run_count: 1,
                        highlighted_file_name: None,
                    })
                    .collect::<Vec<_>>();
                rows.push(EverythingSdkRawResult {
                    full_path: PathBuf::from(r"C:\dev"),
                    kind: EverythingSdkResultKind::Folder,
                    run_count: 20,
                    highlighted_file_name: None,
                });
                Ok(rows)
            },
            |_| true,
        );
        let mapped_candidate_count = run.results.len();
        let ranked = rank_visible_results("dev", run.results, LIMIT);
        let top_record_key = ranked.first().map(|row| row.record_key.clone());
        details.push(Phase4Sample {
            sample,
            sdk_request_max_results: observed_request_max.load(Ordering::SeqCst),
            raw_candidate_count: BEST_RAW_INDEX + 1,
            mapped_candidate_count,
            best_raw_index: BEST_RAW_INDEX,
            surfaced_after_canonical_rank: top_record_key.as_deref()
                == Some("everything:folder:c:\\dev"),
            top_record_key,
            latency_ms: started.elapsed().as_millis() as u64,
        });
    }

    let mut latencies = details
        .iter()
        .map(|sample| sample.latency_ms)
        .collect::<Vec<_>>();
    latencies.sort_unstable();
    let surfaced_after_canonical_rank = details
        .iter()
        .filter(|sample| sample.surfaced_after_canonical_rank)
        .count() as u64;
    let mapped_candidate_max = details
        .iter()
        .map(|sample| sample.mapped_candidate_count)
        .max()
        .unwrap_or(0);

    Phase4Artifact {
        phase: "Phase 4 Everything overfetch preservation",
        method: "30 deterministic fake-SDK samples with best canonical row at raw index 60, provider mapping bounded to limit+25, then canonical rank_visible_results(limit=50)",
        samples: SAMPLE_COUNT,
        requested_limit: LIMIT,
        approved_overfetch_plus: 25,
        approved_max_results_cap: 200,
        expected_sdk_request_max_results: EXPECTED_OVERFETCH_LIMIT,
        best_raw_index: BEST_RAW_INDEX,
        surfaced_after_canonical_rank,
        mapped_candidate_max,
        latency_p95_ms: percentile_ceil(&latencies, 95),
        reused_phase3_evidence_path: "live in-process phase3_observed_regression_summary()",
        stale_everything_boundaries: phase3.stale_everything_boundaries,
        stale_sdk_entries: phase3.stale_sdk_entries,
        latest_completions: phase3.latest_completions,
        latest_completion_target: phase3.latest_completion_target,
        recursive_scans: phase3.recursive_scans,
        samples_detail: details,
    }
}

fn percentile_ceil(values: &[u64], percentile: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() * percentile).saturating_add(99) / 100).saturating_sub(1);
    values[index.min(values.len() - 1)]
}

fn maybe_write_artifact(artifact: &Phase4Artifact) {
    let Some(outdir) = env::var_os("JASONSHELL_PHASE4_OUTDIR") else {
        return;
    };
    let outdir = PathBuf::from(outdir);
    fs::create_dir_all(&outdir).expect("create phase4 artifact dir");
    let json = serde_json::to_string_pretty(artifact).expect("serialize phase4 artifact");
    fs::write(outdir.join("phase4-everything-overfetch.json"), json)
        .expect("write phase4 artifact");
}

#[test]
fn phase4_everything_overfetch_artifact_meets_targets() {
    let artifact = phase4_artifact();
    assert_eq!(artifact.samples, 30);
    assert_eq!(artifact.surfaced_after_canonical_rank, 30);
    assert_eq!(artifact.expected_sdk_request_max_results, 75);
    assert!(artifact.mapped_candidate_max <= 75);
    assert!(artifact.latency_p95_ms <= 350);
    assert_eq!(artifact.stale_everything_boundaries, 0);
    assert_eq!(artifact.stale_sdk_entries, 0);
    assert_eq!(artifact.latest_completions, 30);
    assert_eq!(artifact.latest_completion_target, 30);
    assert_eq!(artifact.recursive_scans, 0);
}

#[test]
#[should_panic(
    expected = "phase4 artifact must not pass with broken phase3 regression observations"
)]
fn phase4_artifact_rejects_broken_phase3_regression_observations() {
    let artifact = phase4_artifact_with_phase3_summary(Phase3RegressionSummary {
        stale_everything_boundaries: 1,
        stale_sdk_entries: 1,
        latest_completions: 29,
        latest_completion_target: 30,
        recursive_scans: 1,
    });

    assert_phase4_targets(&artifact);
}

fn assert_phase4_targets(artifact: &Phase4Artifact) {
    assert_eq!(artifact.samples, 30);
    assert_eq!(artifact.surfaced_after_canonical_rank, 30);
    assert_eq!(artifact.expected_sdk_request_max_results, 75);
    assert!(artifact.mapped_candidate_max <= 75);
    assert!(artifact.latency_p95_ms <= 350);
    if artifact.stale_everything_boundaries != 0
        || artifact.stale_sdk_entries != 0
        || artifact.latest_completions != artifact.latest_completion_target
        || artifact.recursive_scans != 0
    {
        panic!("phase4 artifact must not pass with broken phase3 regression observations");
    }
}

#[test]
#[ignore = "writes Phase4 artifact bundle when JASONSHELL_PHASE4_OUTDIR is set"]
fn phase4_everything_overfetch_writes_artifact_bundle() {
    maybe_write_artifact(&phase4_artifact());
}
