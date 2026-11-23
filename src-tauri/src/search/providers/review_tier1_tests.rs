//! Review-round RED tests (H2, M1, M2, L4, L5) for the new search providers.
//!
//! UNWIRED on purpose (new symbols). Wire in `providers/mod.rs` with
//! `#[cfg(test)] mod review_tier1_tests;`.
//!
//! New seams: see the M2 section comment (bookmarks reader/snapshot_at/last_known/refresh_if_due),
//!   crate::quick_commands::SearchQuickCommandRunResult {Started{run_id,process_id}, OpenPanel{command_id}}
//!   (tag=outcome, camelCase), crate::search::web_engine_after_settings_load(last_good, Result<ShellSettings,String>).
//!   Reasons: quick rows exact|prefix|wordPrefix|substring; web:open-url url|bareDomain. Weak quick rows
//!   and bareDomain rank below every visible non-web row of other providers and are absent from Local.

use super::bookmarks::{
    BookmarkCache, BookmarkFileReader, BOOKMARK_DISCOVERY_INTERVAL, BOOKMARK_FAILURE_TTL,
    MAX_BOOKMARK_FILE_BYTES, MAX_BOOKMARK_PROFILES, MAX_BOOKMARK_TOTAL_BYTES,
};
use super::quick_commands::search_quick_commands;
use super::web::search_web;
use crate::search::contracts::{
    iso_now, SearchProgressPayload, SearchProgressPhase, SearchProviderCacheState,
    SearchProviderHealth, SearchProviderHealthState, SearchProviderId, SearchProviderTiming,
    SearchQueryRequest, SearchResult, SearchResultAction, SearchResultKind,
};
use crate::search::providers::everything::test_everything_run;
use crate::search::scoring::rank_visible_results;
use crate::search::SearchProviderInputs;
use crate::settings::{QuickCommandEntry, QuickCommandMode, ShellSettings, WebSearchEngine};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

fn direct(id: &str, label: &str) -> QuickCommandEntry {
    QuickCommandEntry {
        id: id.to_string(),
        label: label.to_string(),
        mode: QuickCommandMode::Direct,
        target_path: r"C:\tools\a.exe".to_string(),
        args: Vec::new(),
        commands: Vec::new(),
        cwd: None,
        artifact_location: None,
    }
}

fn row(id: &str, provider_id: SearchProviderId, kind: SearchResultKind, title: &str, action: SearchResultAction) -> SearchResult {
    SearchResult {
        id: id.to_string(),
        provider_id,
        kind,
        title: title.to_string(),
        subtitle: None,
        path: None,
        action,
        terms: Vec::new(),
        aliases: Vec::new(),
        score: 0,
        provider_signal: 0,
        match_reason: String::new(),
        record_key: id.to_string(),
        title_highlight_data: Vec::new(),
        subtitle_highlight_data: Vec::new(),
        icon_data_url: None,
    }
}

fn app(title: &str) -> SearchResult {
    row(
        &format!("app:{title}"),
        SearchProviderId::Apps,
        SearchResultKind::App,
        title,
        SearchResultAction::OpenApp { path: format!(r"C:\Apps\{title}.lnk") },
    )
}

fn file(title: &str) -> SearchResult {
    let mut file = row(
        &format!("everything:{title}"),
        SearchProviderId::Everything,
        SearchResultKind::File,
        title,
        SearchResultAction::OpenFile { path: format!(r"C:\data\{title}") },
    );
    file.path = Some(format!(r"C:\data\{title}"));
    file
}

fn ids(rows: &[SearchResult]) -> Vec<&str> {
    rows.iter().map(|r| r.id.as_str()).collect()
}

fn no_everything(_: &str, _: usize) -> crate::search::providers::everything::EverythingSearchRun {
    test_everything_run(
        Vec::new(),
        SearchProviderTiming {
            provider_id: SearchProviderId::Everything,
            started_at: iso_now(),
            ended_at: Some(iso_now()),
            duration_ms: 0.0,
            cache: SearchProviderCacheState::Miss,
            cache_age_ms: None,
            result_count: 0,
            applied: true,
            discarded_as_stale: false,
        },
        SearchProviderHealth {
            provider_id: SearchProviderId::Everything,
            state: SearchProviderHealthState::Ready,
            reason_code: None,
            message: None,
        },
        None,
    )
}

fn engine(query: &str, sequence: u64, inputs: &SearchProviderInputs) -> (Vec<SearchProgressPayload>, crate::search::contracts::SearchEngineResponse) {
    let mut phases = Vec::new();
    let response = crate::search::run_search_engine_with_inputs(
        SearchQueryRequest::new(query, sequence),
        inputs,
        |payload| phases.push(payload),
        no_everything,
    );
    (phases, response)
}

fn local_phase(phases: &[SearchProgressPayload]) -> &SearchProgressPayload {
    phases.iter().find(|p| p.phase == SearchProgressPhase::Local).expect("local phase")
}

// ---------------------------------------------------------------- H2 ranking

#[test]
fn match_reasons_of_quick_command_and_web_rows_survive_ranking() {
    let mut rows = search_quick_commands("report", 10, &[direct("r", "Report"), direct("w", "Weekly report")]).results;
    rows.extend(search_web("report", 10, WebSearchEngine::Google).results);
    let ranked = rank_visible_results("report", rows, 10);
    let reason = |id: &str| ranked.iter().find(|r| r.id == id).map(|r| r.match_reason.clone());
    assert_eq!(reason("quick:r").as_deref(), Some("exact"));
    assert_eq!(reason("quick:w").as_deref(), Some("wordPrefix"));
}

#[test]
fn exact_and_prefix_quick_commands_rank_normally_above_prefix_files() {
    let mut rows = vec![file("report.docx")];
    rows.extend(search_quick_commands("report", 10, &[direct("r", "Report")]).results);
    assert_eq!(ids(&rank_visible_results("report", rows, 10)).first().copied(), Some("quick:r"));
}

#[test]
fn word_prefix_quick_command_ranks_below_a_prefix_file() {
    let mut rows = search_quick_commands("report", 10, &[direct("w", "Weekly report")]).results;
    rows.push(file("report.docx"));
    rows.extend(search_web("report", 10, WebSearchEngine::Google).results);
    let ranked = rank_visible_results("report", rows, 10);
    assert_eq!(ids(&ranked), vec!["everything:report.docx", "quick:w", "web:search"]);
}

#[test]
fn weak_quick_commands_rank_below_every_visible_non_web_row_from_other_providers() {
    let mut rows = search_quick_commands("report", 10, &[direct("s", "Xreportx"), direct("w", "Weekly report")]).results;
    rows.push(file("my-report-notes-2024.txt"));
    rows.push(app("Reporting Tool"));
    rows.extend(search_web("report", 10, WebSearchEngine::Google).results);
    let ranked = rank_visible_results("report", rows, 10);
    let pos = |id: &str| ranked.iter().position(|r| r.id == id).unwrap_or_else(|| panic!("{id} in {:?}", ids(&ranked)));
    for weak in ["quick:s", "quick:w"] {
        assert!(pos(weak) > pos("everything:my-report-notes-2024.txt"), "{weak}: {:?}", ids(&ranked));
        assert!(pos(weak) > pos("app:Reporting Tool"), "{weak}: {:?}", ids(&ranked));
        assert!(pos(weak) < pos("web:search"), "{weak}: {:?}", ids(&ranked));
    }
}

#[test]
fn bare_url_rows_rank_below_exact_and_prefix_file_and_app_matches() {
    let mut rows = search_web("logo.ai", 10, WebSearchEngine::Google).results;
    rows.push(file("logo.ai"));
    rows.push(app("Logo.ai Studio"));
    let ranked = rank_visible_results("logo.ai", rows, 10);
    let open = ranked.iter().position(|r| r.id == "web:open-url").expect("open-url row kept");
    assert_eq!(ranked[open].match_reason, "bareDomain");
    assert!(ranked.iter().position(|r| r.id == "everything:logo.ai").unwrap() < open);
    assert!(ranked.iter().position(|r| r.id == "app:Logo.ai Studio").unwrap() < open);
    assert_eq!(ranked.last().map(|r| r.id.as_str()), Some("web:search"));
}

#[test]
fn explicit_scheme_url_rows_keep_their_normal_rank_and_reason() {
    let rows = search_web("https://github.com/rust-lang", 10, WebSearchEngine::Google).results;
    let ranked = rank_visible_results("https://github.com/rust-lang", rows, 10);
    assert_eq!(ranked[0].id, "web:open-url");
    assert_eq!(ranked[0].match_reason, "url");
}

#[test]
fn a_lone_bare_url_row_is_still_returned_in_the_final_response() {
    let (_, response) = engine("github.com", 7_401, &SearchProviderInputs::default());
    assert!(response.results.iter().any(|r| r.id == "web:open-url"), "{:#?}", ids(&response.results));
}

// ---------------------------------------------------------------- H2 / M1 Local phase

#[test]
fn weak_quick_commands_are_withheld_from_the_local_phase_but_kept_in_the_final_response() {
    let inputs = SearchProviderInputs {
        quick_commands: vec![direct("w", "Weekly report"), direct("x", "Xreportx"), direct("p", "Report builder")],
        ..Default::default()
    };
    let (phases, response) = engine("report", 7_402, &inputs);
    let local_ids = ids(&local_phase(&phases).results).into_iter().map(str::to_string).collect::<Vec<_>>();
    assert!(local_ids.contains(&"quick:p".to_string()), "prefix match stays in Local: {local_ids:?}");
    assert!(!local_ids.contains(&"quick:w".to_string()) && !local_ids.contains(&"quick:x".to_string()), "{local_ids:?}");
    let final_ids = ids(&response.results);
    assert!(final_ids.contains(&"quick:w") && final_ids.contains(&"quick:x"), "{final_ids:?}");
}

#[test]
fn bare_url_rows_are_withheld_from_the_local_phase_but_explicit_urls_are_not() {
    let (phases, response) = engine("logo.ai", 7_403, &SearchProviderInputs::default());
    assert!(local_phase(&phases).results.iter().all(|r| r.id != "web:open-url"));
    assert!(response.results.iter().any(|r| r.id == "web:open-url"));

    let (phases, _) = engine("https://logo.ai", 7_404, &SearchProviderInputs::default());
    assert!(local_phase(&phases).results.iter().any(|r| r.id == "web:open-url"));
}

#[test]
fn web_search_fallback_is_never_in_the_local_phase() {
    let inputs = SearchProviderInputs { quick_commands: vec![direct("p", "Report builder")], ..Default::default() };
    let (phases, _) = engine("report", 7_405, &inputs);
    assert!(local_phase(&phases).results.iter().all(|r| r.id != "web:search"));
}

// ---------------------------------------------------------------- M2 bookmark cache
// Seams (extends f-search's snapshot_at/last_known/refresh_if_due API):
//   trait BookmarkFileReader: Send + Sync { fn read(&self, path: &Path, max_bytes: u64) -> io::Result<String>; }
//   BookmarkCache::with_reader(reader: Arc<dyn BookmarkFileReader>) -> Self   (production uses the fs reader)
//   BookmarkCache::snapshot_at(&self, root: &Path, now: Instant) -> BookmarkSnapshot
//   BookmarkCache::last_known(&self, root: &Path) -> Option<BookmarkSnapshot>        (never does I/O)
//   BookmarkCache::refresh_if_due(self: &Arc<Self>, root: &Path)                      (background, single flight, throttled, never blocks)
//   BookmarkCache::discovery_scans(&self) -> usize
//   consts MAX_BOOKMARK_PROFILES=16, MAX_BOOKMARK_TOTAL_BYTES=32 MiB, BOOKMARK_FAILURE_TTL=5s, BOOKMARK_DISCOVERY_INTERVAL=2s

struct Temp(PathBuf);
static NEXT: AtomicUsize = AtomicUsize::new(0);

impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("jasonshell-review-bm-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn profile(&self, profile: &str, url: &str) -> PathBuf {
        let dir = self.0.join("Google").join("Chrome").join("User Data").join(profile);
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Bookmarks");
        let doc: Value = json!({"roots":{"bookmark_bar":{"type":"folder","children":[{"type":"url","name":"Rust site","url":url}]}}});
        fs::write(&file, doc.to_string()).unwrap();
        file
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const VALID: &str = r#"{"roots":{"bookmark_bar":{"type":"folder","children":[{"type":"url","name":"Rust site","url":"https://rust.example/"}]}}}"#;

struct CountingReader {
    reads: AtomicUsize,
    fail: bool,
    fixed: Option<&'static str>,
}

impl BookmarkFileReader for CountingReader {
    fn read(&self, path: &Path, _max_bytes: u64) -> io::Result<String> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "locked"));
        }
        match self.fixed {
            Some(text) => Ok(text.to_string()),
            None => fs::read_to_string(path),
        }
    }
}

fn counting(fail: bool, fixed: Option<&'static str>) -> Arc<CountingReader> {
    Arc::new(CountingReader { reads: AtomicUsize::new(0), fail, fixed })
}

#[test]
fn documented_limits_are_bounded() {
    assert_eq!(MAX_BOOKMARK_PROFILES, 16);
    assert_eq!(MAX_BOOKMARK_TOTAL_BYTES, 32 * 1024 * 1024);
    assert_eq!(BOOKMARK_FAILURE_TTL, Duration::from_secs(5));
    assert_eq!(BOOKMARK_DISCOVERY_INTERVAL, Duration::from_secs(2));
    assert!(MAX_BOOKMARK_TOTAL_BYTES >= MAX_BOOKMARK_FILE_BYTES);
}

#[test]
fn unreadable_files_are_not_retried_inside_the_failure_ttl_and_are_retried_after_it() {
    let root = Temp::new();
    root.profile("Default", "https://a.example/");
    let reader = counting(true, None);
    let cache = BookmarkCache::with_reader(reader.clone());
    let t0 = Instant::now();
    assert!(cache.snapshot_at(&root.0, t0).entries.is_empty());
    assert!(cache.snapshot_at(&root.0, t0 + Duration::from_secs(1)).entries.is_empty());
    assert!(cache.snapshot_at(&root.0, t0 + BOOKMARK_FAILURE_TTL - Duration::from_millis(1)).entries.is_empty());
    assert_eq!(reader.reads.load(Ordering::SeqCst), 1, "failure must be cached inside the TTL");
    let _ = cache.snapshot_at(&root.0, t0 + BOOKMARK_FAILURE_TTL + Duration::from_secs(1));
    assert_eq!(reader.reads.load(Ordering::SeqCst), 2, "retry after TTL expiry");
}

#[test]
fn directory_discovery_is_throttled_by_the_interval() {
    let root = Temp::new();
    root.profile("Default", "https://a.example/");
    let cache = BookmarkCache::with_reader(counting(false, None));
    let t0 = Instant::now();
    assert_eq!(cache.snapshot_at(&root.0, t0).entries.len(), 1);
    root.profile("Profile 1", "https://b.example/");
    let early = cache.snapshot_at(&root.0, t0 + Duration::from_secs(1));
    assert_eq!(cache.discovery_scans(), 1, "discovery reused inside the interval");
    assert_eq!(early.entries.len(), 1, "new profile not seen before the interval elapses");
    let later = cache.snapshot_at(&root.0, t0 + BOOKMARK_DISCOVERY_INTERVAL + Duration::from_secs(1));
    assert_eq!(cache.discovery_scans(), 2);
    assert_eq!(later.entries.len(), 2);
}

#[test]
fn profile_cap_limits_total_files_read() {
    let root = Temp::new();
    let mut names = vec!["Default".to_string()];
    names.extend((1..=20).map(|i| format!("Profile {i}")));
    for (i, name) in names.iter().enumerate() {
        root.profile(name, &format!("https://p{i}.example/"));
    }
    let reader = counting(false, None);
    let cache = BookmarkCache::with_reader(reader.clone());
    let snapshot = cache.snapshot_at(&root.0, Instant::now());
    assert_eq!(reader.reads.load(Ordering::SeqCst), MAX_BOOKMARK_PROFILES);
    assert_eq!(snapshot.entries.len(), MAX_BOOKMARK_PROFILES);
}

#[test]
fn total_byte_cap_skips_files_beyond_the_cumulative_budget() {
    let root = Temp::new();
    // Four 11 MiB files (zero-filled; the injected reader returns valid JSON): two fit in 32 MiB.
    let size = 11 * 1024 * 1024;
    for (i, name) in ["Default", "Profile 1", "Profile 2", "Profile 3"].iter().enumerate() {
        let file = root.profile(name, &format!("https://p{i}.example/"));
        fs::OpenOptions::new().write(true).open(file).unwrap().set_len(size).unwrap();
    }
    let reader = counting(false, Some(VALID));
    let cache = BookmarkCache::with_reader(reader.clone());
    let _ = cache.snapshot_at(&root.0, Instant::now());
    assert_eq!(reader.reads.load(Ordering::SeqCst), 2);
}

struct GatedReader {
    gate: Mutex<mpsc::Receiver<()>>,
    entered: AtomicUsize,
}

impl BookmarkFileReader for GatedReader {
    fn read(&self, path: &Path, _max_bytes: u64) -> io::Result<String> {
        self.entered.fetch_add(1, Ordering::SeqCst);
        let _ = self.gate.lock().unwrap().recv();
        fs::read_to_string(path)
    }
}

#[test]
fn a_blocked_reader_never_stalls_last_known_or_refresh_if_due_and_results_arrive_later() {
    let root = Temp::new();
    root.profile("Default", "https://rust-lang.example/");
    let (release, gate) = mpsc::channel();
    let reader = Arc::new(GatedReader { gate: Mutex::new(gate), entered: AtomicUsize::new(0) });
    let cache = Arc::new(BookmarkCache::with_reader(reader.clone()));

    let started = Instant::now();
    assert!(cache.last_known(&root.0).is_none(), "no data and no I/O before the first refresh");
    for _ in 0..5 {
        cache.refresh_if_due(&root.0);
    }
    assert!(started.elapsed() < Duration::from_secs(2), "blocked reader stalled the caller for {:?}", started.elapsed());
    assert!(cache.last_known(&root.0).map_or(true, |s| s.entries.is_empty()));

    release.send(()).unwrap();
    drop(release);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if cache.last_known(&root.0).map_or(false, |s| s.entries.len() == 1) {
            break;
        }
        assert!(Instant::now() < deadline, "background refresh never completed");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(reader.entered.load(Ordering::SeqCst), 1, "single-flight refresh: exactly one read");
}

// ---------------------------------------------------------------- L4 / L5

#[test]
fn search_quick_command_outcome_serializes_with_a_typed_discriminant() {
    use crate::quick_commands::SearchQuickCommandRunResult as R;
    let started = serde_json::to_value(R::Started { run_id: "run-1".into(), process_id: 7 }).unwrap();
    assert_eq!(started, json!({"outcome": "started", "runId": "run-1", "processId": 7}));
    let panel = serde_json::to_value(R::OpenPanel { command_id: "build".into() }).unwrap();
    assert_eq!(panel, json!({"outcome": "openPanel", "commandId": "build"}));
}

#[test]
fn settings_load_failure_keeps_the_last_good_web_engine() {
    use crate::search::web_engine_after_settings_load as resolve;
    let mut settings = ShellSettings::default();
    settings.search.web_engine = WebSearchEngine::Brave;
    assert_eq!(resolve(WebSearchEngine::Google, Ok(settings)), WebSearchEngine::Brave);
    assert_eq!(resolve(WebSearchEngine::Bing, Err("settings locked".into())), WebSearchEngine::Bing);
    assert_eq!(resolve(WebSearchEngine::DuckDuckGo, Err(String::new())), WebSearchEngine::DuckDuckGo);
}
