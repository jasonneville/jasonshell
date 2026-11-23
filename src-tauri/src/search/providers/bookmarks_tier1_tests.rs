//! Tier 1 (PRD-02c) RED tests for the Bookmarks search provider (Chrome + Edge, read-only).
//!
//! UNWIRED on purpose. The fixer adds, in `providers/mod.rs`:
//!   `pub(crate) mod bookmarks;`
//!   `#[cfg(test)] mod bookmarks_tier1_tests;`
//! (and the `SearchProviderInputs` seam described in `calculator_tier1_tests.rs`; `bookmarks_local_app_data: None` means NO bookmarks, so unit tests with Default
//! inputs never read real profile files; the production caller passes `Some(%LOCALAPPDATA%)`).
//!
//! Required seams, all in `providers/bookmarks.rs`:
//!   pub(crate) const MAX_BOOKMARK_FILE_BYTES: u64 = 16 * 1024 * 1024;   // files LARGER are skipped unread
//!   pub(crate) const MAX_BOOKMARK_ENTRIES: usize = 20_000;
//!   #[derive(Clone, Debug, PartialEq, Eq)]
//!   pub(crate) struct BookmarkEntry { pub(crate) title: String, pub(crate) url: String }
//!   pub(crate) fn bookmark_file_candidates(local_app_data: &Path) -> Vec<PathBuf>
//!       // existing regular files only: <root>\Google\Chrome\User Data\{Default|Profile <digits>}\Bookmarks
//!       // and <root>\Microsoft\Edge\User Data\{Default|Profile <digits>}\Bookmarks, sorted
//!   pub(crate) fn parse_bookmarks_json(json: &str, max_entries: usize) -> Vec<BookmarkEntry>
//!       // malformed/odd JSON => empty; http/https only (same URL rules as web::validate_search_open_url);
//!       // folders flattened; blank title falls back to the URL; never more than max_entries
//!   pub(crate) struct BookmarkSnapshot { pub(crate) entries: Vec<BookmarkEntry>,
//!       pub(crate) cache: SearchProviderCacheState }   // entries de-duplicated by URL
//!   #[derive(Default)] pub(crate) struct BookmarkCache { .. }
//!   impl BookmarkCache { pub(crate) fn new() -> Self;
//!       pub(crate) fn snapshot(&self, local_app_data: &Path) -> BookmarkSnapshot;
//!       pub(crate) fn file_reads(&self) -> usize }   // number of file reads (parses) ever performed
//!       // cache key per file = (path, modified time, length); Miss on (re)read, Hit otherwise;
//!       // a file that disappears or becomes unreadable/oversized drops out of the snapshot
//!   pub(crate) struct BookmarksSearchRun { pub(crate) results: Vec<SearchResult>,
//!       pub(crate) timing: SearchProviderTiming, pub(crate) health: SearchProviderHealth }
//!   pub(crate) fn search_bookmarks(query: &str, limit: usize, local_app_data: &Path,
//!       cache: &BookmarkCache) -> BookmarksSearchRun
//!
//! Provider rules: the file is never written/locked for write; executed inside the existing blocking
//! search worker (no UI-thread work); all query tokens must match title+URL (case-insensitive);
//! blank query => no rows; rows: id `bookmark:<url>`, record_key starts with `bookmark:`,
//! kind Bookmark, provider Bookmarks, title = bookmark title, subtitle = Some(url), path None,
//! action OpenBookmark{url}; title-prefix matches sort before URL-only matches; `limit` honoured.

use super::bookmarks::{
    bookmark_file_candidates, parse_bookmarks_json, search_bookmarks, BookmarkCache,
    MAX_BOOKMARK_ENTRIES, MAX_BOOKMARK_FILE_BYTES,
};
use crate::search::contracts::{
    iso_now, SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState,
    SearchProviderId, SearchProviderTiming, SearchQueryRequest, SearchResultAction,
    SearchResultKind,
};
use crate::search::providers::everything::test_everything_run;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempRoot(PathBuf);

impl TempRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jasonshell-bookmarks-tier1-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp root");
        Self(path)
    }
    fn path(&self) -> &Path {
        &self.0
    }
    fn profile_dir(&self, browser: &str, profile: &str) -> PathBuf {
        let base = match browser {
            "chrome" => self.0.join("Google").join("Chrome").join("User Data"),
            "edge" => self.0.join("Microsoft").join("Edge").join("User Data"),
            other => panic!("unknown browser {other}"),
        };
        let dir = base.join(profile);
        fs::create_dir_all(&dir).expect("profile dir");
        dir
    }
    fn write_bookmarks(&self, browser: &str, profile: &str, json: &Value) -> PathBuf {
        let file = self.profile_dir(browser, profile).join("Bookmarks");
        fs::write(&file, serde_json::to_vec(json).unwrap()).expect("write bookmarks");
        file
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn url_node(name: &str, url: &str) -> Value {
    json!({ "type": "url", "name": name, "url": url, "id": "1", "guid": "g" })
}

fn doc(bar: Vec<Value>, other: Vec<Value>) -> Value {
    json!({
        "checksum": "x",
        "roots": {
            "bookmark_bar": { "type": "folder", "name": "Bookmarks bar", "children": bar },
            "other": { "type": "folder", "name": "Other bookmarks", "children": other },
            "synced": { "type": "folder", "name": "Mobile bookmarks", "children": [] }
        },
        "version": 1
    })
}

fn relative_names(root: &Path, files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .map(|file| {
            file.strip_prefix(root)
                .expect("under root")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

#[test]
fn discovers_default_and_numbered_profiles_for_chrome_and_edge_only() {
    let root = TempRoot::new();
    let empty = doc(vec![], vec![]);
    for profile in ["Default", "Profile 1", "Profile 12"] {
        root.write_bookmarks("chrome", profile, &empty);
    }
    for profile in ["Default", "Profile 3"] {
        root.write_bookmarks("edge", profile, &empty);
    }
    for ignored in ["Guest Profile", "System Profile", "Profile x", "Profile", "Profile 1 copy"] {
        root.write_bookmarks("chrome", ignored, &empty);
    }
    // A profile directory without a Bookmarks file, a backup file, and a directory named Bookmarks.
    root.profile_dir("chrome", "Profile 5");
    fs::write(root.profile_dir("edge", "Profile 6").join("Bookmarks.bak"), b"{}").unwrap();
    fs::create_dir_all(root.profile_dir("edge", "Profile 7").join("Bookmarks")).unwrap();

    let mut names = relative_names(root.path(), &bookmark_file_candidates(root.path()));
    names.sort();
    assert_eq!(
        names,
        vec![
            "Google/Chrome/User Data/Default/Bookmarks",
            "Google/Chrome/User Data/Profile 1/Bookmarks",
            "Google/Chrome/User Data/Profile 12/Bookmarks",
            "Microsoft/Edge/User Data/Default/Bookmarks",
            "Microsoft/Edge/User Data/Profile 3/Bookmarks",
        ]
    );
}

#[test]
fn missing_roots_yield_no_candidates_no_rows_and_no_panic() {
    let root = TempRoot::new();
    let missing = root.path().join("does-not-exist");
    assert!(bookmark_file_candidates(&missing).is_empty());
    let cache = BookmarkCache::new();
    let run = search_bookmarks("anything", 10, &missing, &cache);
    assert!(run.results.is_empty());
    assert_eq!(run.timing.provider_id, SearchProviderId::Bookmarks);
    assert_eq!(run.health.provider_id, SearchProviderId::Bookmarks);
}

#[test]
fn parser_flattens_nested_folders_and_ignores_folder_names() {
    let json = doc(
        vec![
            url_node("Rust", "https://www.rust-lang.org/"),
            json!({ "type": "folder", "name": "Dev", "children": [
                url_node("Docs", "https://docs.rs/"),
                json!({ "type": "folder", "name": "Deep", "children": [url_node("Crates", "https://crates.io/")] })
            ] }),
        ],
        vec![url_node("Example", "http://example.org/")],
    )
    .to_string();
    let entries = parse_bookmarks_json(&json, MAX_BOOKMARK_ENTRIES);
    let mut urls = entries.iter().map(|e| e.url.as_str()).collect::<Vec<_>>();
    urls.sort();
    assert_eq!(urls, vec!["http://example.org/", "https://crates.io/", "https://docs.rs/", "https://www.rust-lang.org/"]);
    assert!(entries.iter().all(|e| !["Dev", "Deep", "Bookmarks bar"].contains(&e.title.as_str())));
}

#[test]
fn parser_keeps_only_clean_http_and_https_urls() {
    let json = doc(
        vec![
            url_node("good", "https://ok.example/"),
            url_node("bookmarklet", "javascript:alert(1)"),
            url_node("local file", "file:///C:/Windows/notepad.exe"),
            url_node("chrome page", "chrome://settings"),
            url_node("edge page", "edge://settings"),
            url_node("data", "data:text/html,hi"),
            url_node("ftp", "ftp://example.com/"),
            url_node("creds", "https://user:pw@example.com/"),
            url_node("empty host", "https://"),
            url_node("spaces", "https://example.com/a b"),
            url_node("newline", "https://example.com/\n"),
            url_node("no url", ""),
        ],
        vec![],
    )
    .to_string();
    let entries = parse_bookmarks_json(&json, MAX_BOOKMARK_ENTRIES);
    assert_eq!(entries.iter().map(|e| e.url.as_str()).collect::<Vec<_>>(), vec!["https://ok.example/"]);
}

#[test]
fn parser_blank_title_falls_back_to_url() {
    let json = doc(vec![url_node("", "https://untitled.example/")], vec![]).to_string();
    let entries = parse_bookmarks_json(&json, MAX_BOOKMARK_ENTRIES);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "https://untitled.example/");
}

#[test]
fn parser_returns_empty_for_malformed_or_wrong_shaped_json() {
    for json in [
        "",
        "   ",
        "not json",
        "{",
        "[]",
        "null",
        "42",
        r#"{"roots": 5}"#,
        r#"{"roots": {"bookmark_bar": "oops"}}"#,
        r#"{"roots": {"bookmark_bar": {"children": 7}}}"#,
        r#"{"roots": {"bookmark_bar": {"children": [null, 1, "x", {"type": "url"}]}}}"#,
        "\u{feff}{\"roots\":",
    ] {
        assert!(parse_bookmarks_json(json, MAX_BOOKMARK_ENTRIES).is_empty(), "{json:?}");
    }
}

#[test]
fn parser_never_returns_more_than_the_entry_cap_and_survives_deep_nesting() {
    let many = (0..50).map(|i| url_node(&format!("n{i}"), &format!("https://e{i}.example/"))).collect();
    let json = doc(many, vec![]).to_string();
    assert_eq!(parse_bookmarks_json(&json, 5).len(), 5);
    assert_eq!(parse_bookmarks_json(&json, 0).len(), 0);

    // Hostile nesting, built as TEXT (no deep Value build/drop in the test): the parser must
    // survive on the default test-thread stack and return empty or partial results.
    let depth = 5_000;
    let hostile = format!(
        "{{\"roots\":{{\"bookmark_bar\":{}{}{}}}}}",
        "{\"type\":\"folder\",\"children\":[".repeat(depth),
        "{\"type\":\"url\",\"name\":\"leaf\",\"url\":\"https://leaf.example/\"}",
        "]}".repeat(depth)
    );
    let _ = parse_bookmarks_json(&hostile, MAX_BOOKMARK_ENTRIES);
}

#[test]
fn files_over_the_size_cap_are_skipped_but_files_at_the_cap_are_read() {
    let root = TempRoot::new();
    let valid = serde_json::to_string(&doc(vec![url_node("Big", "https://big.example/")], vec![])).unwrap();
    let pad_to = |len: u64| {
        let mut text = valid.clone();
        text.push_str(&" ".repeat((len as usize) - valid.len()));
        text
    };
    let dir = root.profile_dir("chrome", "Default");
    fs::write(dir.join("Bookmarks"), pad_to(MAX_BOOKMARK_FILE_BYTES + 1)).unwrap();
    let cache = BookmarkCache::new();
    assert!(cache.snapshot(root.path()).entries.is_empty(), "oversized file must be ignored");
    assert_eq!(cache.file_reads(), 0, "oversized file must not be read/parsed");

    fs::write(dir.join("Bookmarks"), pad_to(MAX_BOOKMARK_FILE_BYTES)).unwrap();
    let cache = BookmarkCache::new();
    let snapshot = cache.snapshot(root.path());
    assert_eq!(snapshot.entries.iter().map(|e| e.url.as_str()).collect::<Vec<_>>(), vec!["https://big.example/"]);
}

#[test]
fn malformed_and_empty_files_are_treated_as_empty_without_hiding_other_profiles() {
    let root = TempRoot::new();
    fs::write(root.profile_dir("chrome", "Default").join("Bookmarks"), b"{ definitely not json").unwrap();
    fs::write(root.profile_dir("chrome", "Profile 1").join("Bookmarks"), b"").unwrap();
    root.write_bookmarks("edge", "Default", &doc(vec![url_node("Still here", "https://still.example/")], vec![]));
    let cache = BookmarkCache::new();
    let snapshot = cache.snapshot(root.path());
    assert_eq!(snapshot.entries.iter().map(|e| e.url.as_str()).collect::<Vec<_>>(), vec!["https://still.example/"]);
}

#[test]
fn snapshot_deduplicates_by_url_across_profiles_and_browsers() {
    let root = TempRoot::new();
    root.write_bookmarks("chrome", "Default", &doc(vec![url_node("Rust", "https://www.rust-lang.org/")], vec![]));
    root.write_bookmarks("chrome", "Profile 1", &doc(vec![url_node("Rust again", "https://www.rust-lang.org/")], vec![]));
    root.write_bookmarks("edge", "Default", &doc(vec![url_node("Rust edge", "https://www.rust-lang.org/"), url_node("Docs", "https://docs.rs/")], vec![]));
    let cache = BookmarkCache::new();
    let mut urls = cache.snapshot(root.path()).entries.into_iter().map(|e| e.url).collect::<Vec<_>>();
    urls.sort();
    assert_eq!(urls, vec!["https://docs.rs/", "https://www.rust-lang.org/"]);
}

#[test]
fn snapshot_is_cached_by_mtime_and_length_and_reports_hit_then_miss() {
    let root = TempRoot::new();
    let file = root.write_bookmarks("chrome", "Default", &doc(vec![url_node("One", "https://one.example/")], vec![]));
    let cache = BookmarkCache::new();

    let first = cache.snapshot(root.path());
    assert_eq!(first.cache, SearchProviderCacheState::Miss);
    assert_eq!(cache.file_reads(), 1);

    let second = cache.snapshot(root.path());
    assert_eq!(second.cache, SearchProviderCacheState::Hit);
    assert_eq!(cache.file_reads(), 1, "unchanged file must not be re-read");
    assert_eq!(second.entries, first.entries);

    // Different length (and normally mtime) invalidates.
    fs::write(&file, serde_json::to_vec(&doc(vec![url_node("One", "https://one.example/"), url_node("Two", "https://two.example/")], vec![])).unwrap()).unwrap();
    let third = cache.snapshot(root.path());
    assert_eq!(third.cache, SearchProviderCacheState::Miss);
    assert_eq!(cache.file_reads(), 2);
    assert_eq!(third.entries.len(), 2);

    // Removing the file drops its entries.
    fs::remove_file(&file).unwrap();
    assert!(cache.snapshot(root.path()).entries.is_empty());
}

#[test]
fn provider_never_modifies_bookmark_files() {
    let root = TempRoot::new();
    let file = root.write_bookmarks("chrome", "Default", &doc(vec![url_node("Rust", "https://www.rust-lang.org/")], vec![]));
    let before = (fs::read(&file).unwrap(), fs::metadata(&file).unwrap().modified().unwrap());
    let cache = BookmarkCache::new();
    let _ = search_bookmarks("rust", 10, root.path(), &cache);
    let after = (fs::read(&file).unwrap(), fs::metadata(&file).unwrap().modified().unwrap());
    assert_eq!(before, after);
}

fn sample_root() -> TempRoot {
    let root = TempRoot::new();
    root.write_bookmarks(
        "chrome",
        "Default",
        &doc(
            vec![
                url_node("Rust Programming Language", "https://www.rust-lang.org/"),
                url_node("Crates registry", "https://crates.io/"),
                url_node("Tauri guides", "https://v2.tauri.app/start/"),
            ],
            vec![url_node("Everything about rust", "https://example.org/blog")],
        ),
    );
    root.write_bookmarks("edge", "Profile 2", &doc(vec![url_node("Svelte", "https://svelte.dev/docs")], vec![]));
    root
}

#[test]
fn search_matches_title_and_url_with_all_tokens_case_insensitively() {
    let root = sample_root();
    let cache = BookmarkCache::new();

    let by_title = search_bookmarks("RUST programming", 10, root.path(), &cache);
    assert_eq!(by_title.results.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(), vec!["Rust Programming Language"]);

    let by_url = search_bookmarks("tauri.app", 10, root.path(), &cache);
    assert_eq!(by_url.results.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(), vec!["Tauri guides"]);

    let none = search_bookmarks("rust tauri", 10, root.path(), &cache);
    assert!(none.results.is_empty(), "every token must match");

    for blank in ["", "   "] {
        assert!(search_bookmarks(blank, 10, root.path(), &cache).results.is_empty());
    }
}

#[test]
fn search_orders_title_prefix_before_url_only_matches_and_honours_limit() {
    let root = sample_root();
    let cache = BookmarkCache::new();
    let run = search_bookmarks("rust", 10, root.path(), &cache);
    let titles = run.results.iter().map(|r| r.title.as_str()).collect::<Vec<_>>();
    assert_eq!(titles.first().copied(), Some("Rust Programming Language"), "{titles:?}");
    assert!(titles.contains(&"Everything about rust"), "{titles:?}");
    assert_eq!(search_bookmarks("rust", 1, root.path(), &cache).results.len(), 1);
}

#[test]
fn result_rows_use_the_open_bookmark_contract() {
    let root = sample_root();
    let cache = BookmarkCache::new();
    let run = search_bookmarks("svelte", 10, root.path(), &cache);
    assert_eq!(run.results.len(), 1);
    let row = &run.results[0];
    assert_eq!(row.id, "bookmark:https://svelte.dev/docs");
    assert!(row.record_key.starts_with("bookmark:"), "{}", row.record_key);
    assert_eq!(row.provider_id, SearchProviderId::Bookmarks);
    assert_eq!(row.kind, SearchResultKind::Bookmark);
    assert_eq!(row.title, "Svelte");
    assert_eq!(row.subtitle.as_deref(), Some("https://svelte.dev/docs"));
    assert_eq!(row.path, None);
    assert_eq!(row.action, SearchResultAction::OpenBookmark { url: "https://svelte.dev/docs".to_string() });
    assert!(row.action.is_safe());
    assert_eq!(run.timing.provider_id, SearchProviderId::Bookmarks);
    assert_eq!(run.timing.result_count, 1);
    assert_eq!(run.health.provider_id, SearchProviderId::Bookmarks);
    assert_eq!(run.health.state, SearchProviderHealthState::Ready);
}

#[test]
fn timing_reports_cache_miss_then_hit() {
    let root = sample_root();
    let cache = BookmarkCache::new();
    assert_eq!(search_bookmarks("rust", 10, root.path(), &cache).timing.cache, SearchProviderCacheState::Miss);
    assert_eq!(search_bookmarks("rust", 10, root.path(), &cache).timing.cache, SearchProviderCacheState::Hit);
}

#[test]
fn engine_includes_bookmark_rows_from_the_injected_root() {
    use crate::search::SearchProviderInputs;
    let root = sample_root();
    let inputs = SearchProviderInputs { bookmarks_local_app_data: Some(root.path().to_path_buf()), ..Default::default() };
    let response = crate::search::run_search_engine_with_inputs(
        SearchQueryRequest::new("crates registry", 7_201),
        &inputs,
        |_| {},
        |_, _| {
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
        },
    );
    let bookmark = response.results.iter().find(|r| r.kind == SearchResultKind::Bookmark).expect("bookmark row");
    assert_eq!(bookmark.action, SearchResultAction::OpenBookmark { url: "https://crates.io/".to_string() });
    assert!(response.provider_timings.iter().any(|t| t.provider_id == SearchProviderId::Bookmarks));
    let web_pos = response.results.iter().position(|r| r.id == "web:search");
    let bookmark_pos = response.results.iter().position(|r| r.id == bookmark.id);
    assert!(web_pos.is_none() || web_pos > bookmark_pos, "web fallback must stay last");
}

#[test]
fn default_inputs_with_no_root_read_no_bookmarks() {
    use crate::search::SearchProviderInputs;
    let response = crate::search::run_search_engine_with_inputs(
        SearchQueryRequest::new("a", 7_202),
        &SearchProviderInputs::default(),
        |_| {},
        |_, _| {
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
        },
    );
    assert!(response.results.iter().all(|r| r.kind != SearchResultKind::Bookmark));
}

#[test]
fn large_but_in_budget_bookmark_files_parse_quickly_and_respect_the_entry_cap() {
    let root = TempRoot::new();
    let many = (0..(MAX_BOOKMARK_ENTRIES + 500))
        .map(|i| url_node(&format!("Bookmark number {i}"), &format!("https://host{i}.example/page")))
        .collect();
    root.write_bookmarks("chrome", "Default", &doc(many, vec![]));
    let cache = BookmarkCache::new();
    let started = std::time::Instant::now();
    let snapshot = cache.snapshot(root.path());
    assert!(snapshot.entries.len() <= MAX_BOOKMARK_ENTRIES);
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "cold parse took {:?}", started.elapsed());
    let started = std::time::Instant::now();
    let _ = search_bookmarks("number 7", 10, root.path(), &cache);
    assert!(started.elapsed() < std::time::Duration::from_millis(2000), "warm search took {:?}", started.elapsed());
}

// ---- Review M1: a query needs at least one alphanumeric character to match bookmarks.
#[test]
fn punctuation_only_queries_match_no_bookmarks() {
    let root = TempRoot::new();
    root.write_bookmarks(
        "chrome",
        "Default",
        &doc(vec![url_node("A.B - c/d", "https://a.b/c-d?x=1")], vec![]),
    );
    let cache = BookmarkCache::new();
    for query in [".", "..", "-", "/", "//", ":", "?", "- .", "._-"] {
        assert!(search_bookmarks(query, 10, root.path(), &cache).results.is_empty(), "{query:?}");
    }
    assert_eq!(search_bookmarks("a.b", 10, root.path(), &cache).results.len(), 1);
    assert_eq!(search_bookmarks("c/d", 10, root.path(), &cache).results.len(), 1);
}

// ---- Review M2: bookmark match tiers. Title start decides exact/prefix; URL-only is substring.
fn reason_for(query: &str, title: &str, url: &str) -> Option<String> {
    let root = TempRoot::new();
    root.write_bookmarks("chrome", "Default", &doc(vec![url_node(title, url)], vec![]));
    let cache = BookmarkCache::new();
    search_bookmarks(query, 10, root.path(), &cache).results.first().map(|r| r.match_reason.clone())
}

#[test]
fn bookmark_rows_carry_exact_prefix_token_prefix_or_substring_reasons() {
    assert_eq!(reason_for("main", "main", "https://x.example/").as_deref(), Some("exact"));
    assert_eq!(reason_for("main", "Main page", "https://x.example/").as_deref(), Some("prefix"));
    assert_eq!(reason_for("main", "The main page", "https://x.example/").as_deref(), Some("tokenPrefix"));
    assert_eq!(reason_for("main", "Some site", "https://domain.com/").as_deref(), Some("substring"));
    assert_eq!(reason_for("main", "Domain site", "https://x.example/").as_deref(), Some("substring"));
}

#[test]
fn weak_bookmark_matches_are_withheld_from_local_payloads_but_kept_in_the_final_response() {
    use crate::search::contracts::SearchProgressPhase;
    use crate::search::SearchProviderInputs;
    let root = TempRoot::new();
    root.write_bookmarks(
        "chrome",
        "Default",
        &doc(
            vec![
                url_node("Some site", "https://domain.com/"),
                url_node("The main page", "https://tp.example/"),
                url_node("Main page", "https://prefix.example/"),
            ],
            vec![],
        ),
    );
    let inputs = SearchProviderInputs { bookmarks_local_app_data: Some(root.path().to_path_buf()), ..Default::default() };
    let mut phases = Vec::new();
    let response = crate::search::run_search_engine_with_inputs(
        SearchQueryRequest::new("main", 7_204),
        &inputs,
        |payload| phases.push(payload),
        |_, _| {
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
        },
    );
    let final_urls = response
        .results
        .iter()
        .filter_map(|r| match &r.action {
            SearchResultAction::OpenBookmark { url } => Some(url.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(final_urls.contains(&"https://domain.com/") && final_urls.contains(&"https://tp.example/"), "{final_urls:?}");
    for phase in [SearchProgressPhase::Local, SearchProgressPhase::Provider] {
        for payload in phases.iter().filter(|p| p.phase == phase) {
            let urls = payload
                .results
                .iter()
                .filter_map(|r| match &r.action {
                    SearchResultAction::OpenBookmark { url } => Some(url.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(!urls.contains(&"https://domain.com/".to_string()), "{phase:?}: {urls:?}");
            assert!(!urls.contains(&"https://tp.example/".to_string()), "{phase:?}: {urls:?}");
        }
    }
}
