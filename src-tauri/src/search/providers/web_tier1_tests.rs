//! Tier 1 (PRD-02a/b) RED tests for the Web search provider (Open URL row + web-search fallback row).
//!
//! UNWIRED on purpose. The fixer adds, in `providers/mod.rs`:
//!   `pub(crate) mod web;`
//!   `#[cfg(test)] mod web_tier1_tests;`
//! (and the `SearchProviderInputs` / `run_search_engine_with_inputs` seam described in
//! `calculator_tier1_tests.rs`).
//!
//! Required seams:
//!   crate::settings:
//!     #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
//!     #[serde(rename_all = "lowercase")]
//!     pub enum WebSearchEngine { #[default] Google, DuckDuckGo, Bing, Brave }   // json: google|duckduckgo|bing|brave
//!       (rename DuckDuckGo => "duckduckgo")
//!     SearchSettings gains `#[serde(default)] pub web_engine: WebSearchEngine` (json `webEngine`);
//!     `SearchSettings::default().web_engine == Google`; an unknown value is a deserialize error.
//!   providers/web.rs:
//!     pub(crate) const MAX_WEB_QUERY_CHARS: usize = 256;
//!     pub(crate) const MAX_OPEN_URL_CHARS: usize = 2048;
//!     pub(crate) fn normalize_open_url_query(query: &str) -> Option<String>
//!         // URL-like query -> normalized http(s) URL, else None
//!     pub(crate) fn validate_search_open_url(url: &str) -> Result<String, String>
//!         // backend revalidation right before ShellExecute; returns the URL to open
//!     pub(crate) fn web_search_url(engine: WebSearchEngine, query: &str) -> Option<String>
//!     pub(crate) struct WebSearchRun { results, timing, health }
//!     pub(crate) fn search_web(query: &str, limit: usize, engine: WebSearchEngine) -> WebSearchRun
//!   search/contracts.rs: `SearchResultAction::is_safe` for OpenWebUrl/OpenBookmark must apply the
//!     same rules as `validate_search_open_url` (no credentials, no whitespace/control chars,
//!     non-empty host, <= MAX_OPEN_URL_CHARS, lowercase-insensitive scheme http/https only).
//!   search/scoring.rs: rows with provider_id == Web and id == "web:search" (the fallback) are
//!     pinned last and bypass the text matcher; they always keep one slot under `limit`.
//!
//! Decided rules (tests are the spec):
//! * Bare tokens (no scheme) are URL-like only when the host has a dot AND (starts with `www.` OR its
//!   last label is in a small well-known TLD set: com org net io dev app co ai edu gov uk de fr nl
//!   info xyz me tv us ca au eu ru jp cn in br ch). So `readme.md`, `main.rs`, `index.html`,
//!   `script.py`, `notes.txt`, `v1.2.3`, `1.2.3.4`, `localhost:3000` are NOT URLs. Bare domains get https.
//! * Explicit scheme must be http or https (case-insensitive). Anything else that looks like
//!   `scheme:` (javascript, file, data, ftp, mailto, ...) is rejected, as are credentials
//!   (`user@`, `user:pw@`), whitespace/control chars inside, backslashes, empty host, > 2048 chars.
//!   Leading/trailing whitespace is trimmed before checks.
//! * Web-search fallback: any non-empty trimmed query up to 256 chars without control chars.
//!   URL is `<engine base>?q=<percent-encoded query>` using %20 for space and uppercase hex;
//!   unreserved set is A-Z a-z 0-9 - . _ ~ only.

use super::web::{
    normalize_open_url_query, search_web, validate_search_open_url, web_search_url,
    MAX_OPEN_URL_CHARS, MAX_WEB_QUERY_CHARS,
};
use crate::search::contracts::{
    iso_now, SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState,
    SearchProviderId, SearchProviderTiming, SearchQueryRequest, SearchResult, SearchResultAction,
    SearchResultKind,
};
use crate::search::providers::everything::test_everything_run;
use crate::search::scoring::rank_visible_results;
use crate::settings::{SearchSettings, WebSearchEngine};

fn trimmed(url: Option<String>) -> Option<String> {
    url.map(|url| url.trim_end_matches('/').to_string())
}

#[test]
fn explicit_http_and_https_urls_are_accepted() {
    assert_eq!(
        trimmed(normalize_open_url_query("https://github.com/rust-lang/rust")).as_deref(),
        Some("https://github.com/rust-lang/rust")
    );
    assert_eq!(
        trimmed(normalize_open_url_query("http://example.org/a?b=c#d")).as_deref(),
        Some("http://example.org/a?b=c#d")
    );
    assert!(normalize_open_url_query("HTTPS://Example.com/Path").is_some());
    assert!(normalize_open_url_query("http://192.168.1.1/admin").is_some());
    assert!(normalize_open_url_query("https://localhost:3000/").is_some());
    assert!(normalize_open_url_query("  https://example.com  ").is_some());
}

#[test]
fn bare_domains_with_known_tld_or_www_get_https() {
    assert_eq!(trimmed(normalize_open_url_query("github.com")).as_deref(), Some("https://github.com"));
    assert_eq!(
        trimmed(normalize_open_url_query("example.org/path?q=1")).as_deref(),
        Some("https://example.org/path?q=1")
    );
    assert_eq!(
        trimmed(normalize_open_url_query("www.rust-lang.org")).as_deref(),
        Some("https://www.rust-lang.org")
    );
    assert_eq!(trimmed(normalize_open_url_query("docs.rs.dev")).as_deref(), Some("https://docs.rs.dev"));
    assert!(normalize_open_url_query("www.example.md").is_some(), "www. prefix allows any TLD");
}

#[test]
fn filenames_versions_and_ip_literals_are_not_treated_as_domains() {
    for query in [
        "readme.md", "main.rs", "index.html", "script.py", "notes.txt", "setup.sh", "app.exe",
        "Cargo.toml", "package.json", "v1.2.3", "1.2.3", "1.2.3.4", "node-v20", "c++", "42",
        "localhost:3000", "hello world", "github", "a.b", ".com", "com.", "github..com",
    ] {
        assert_eq!(normalize_open_url_query(query), None, "query {query:?}");
    }
}

#[test]
fn dangerous_or_non_web_schemes_are_rejected() {
    for query in [
        "javascript:alert(1)",
        "JaVaScRiPt:alert(1)",
        " javascript:alert(1)",
        "java\tscript:alert(1)",
        "file:///C:/Windows/System32/calc.exe",
        "FILE://server/share",
        "data:text/html,<script>alert(1)</script>",
        "ftp://example.com/file",
        "mailto:someone@example.com",
        "vbscript:msgbox(1)",
        "ms-settings:display",
        "chrome://settings",
        "about:blank",
        "//example.com",
        "http:example.com",
    ] {
        assert_eq!(normalize_open_url_query(query), None, "query {query:?}");
    }
}

#[test]
fn credentials_whitespace_control_characters_and_malformed_hosts_are_rejected() {
    for query in [
        "https://user:pass@example.com",
        "https://user@example.com",
        "user@example.com",
        "https://exa mple.com",
        "https://example.com/a b",
        "example.com/a b",
        "https://exa\u{7}mple.com",
        "https://example.com/\u{0}",
        "https://example.com/\n",
        "example.com\nevil.com",
        "https://example.com\\@evil.com",
        "https://",
        "http://",
        "https:///path",
        "https://.com",
        "https://exa_mple..com",
    ] {
        assert_eq!(normalize_open_url_query(query), None, "query {query:?}");
    }
}

#[test]
fn over_long_urls_are_rejected() {
    let at_cap = format!("https://example.com/{}", "a".repeat(MAX_OPEN_URL_CHARS - 20));
    assert!(at_cap.chars().count() <= MAX_OPEN_URL_CHARS);
    assert!(normalize_open_url_query(&at_cap).is_some());
    let over = format!("https://example.com/{}", "a".repeat(MAX_OPEN_URL_CHARS));
    assert_eq!(normalize_open_url_query(&over), None);
}

#[test]
fn backend_revalidation_accepts_only_clean_http_and_https_urls() {
    assert!(validate_search_open_url("https://github.com/").is_ok());
    assert!(validate_search_open_url("http://example.org/x?y=z").is_ok());
    assert!(validate_search_open_url("HTTPS://example.com/").is_ok());
    for url in [
        "",
        "   ",
        "javascript:alert(1)",
        "file:///C:/Windows/notepad.exe",
        "data:text/html,hi",
        "ftp://example.com/",
        "ms-settings:display",
        "https://",
        "https://user:pw@example.com/",
        "https://user@example.com/",
        " https://example.com/",
        "https://example.com/ ",
        "https://example.com/a b",
        "https://example.com/\n",
        "https://example.com/\u{7}",
        "https://example.com\\evil",
        "example.com",
        "https:example.com",
    ] {
        assert!(validate_search_open_url(url).is_err(), "url {url:?} must be rejected");
    }
    let over = format!("https://example.com/{}", "a".repeat(MAX_OPEN_URL_CHARS));
    assert!(validate_search_open_url(&over).is_err());
}

#[test]
fn action_safety_matches_backend_revalidation_for_web_and_bookmark_actions() {
    for url in ["https://github.com/", "http://example.org/"] {
        assert!(SearchResultAction::OpenWebUrl { url: url.to_string() }.is_safe(), "{url}");
        assert!(SearchResultAction::OpenBookmark { url: url.to_string() }.is_safe(), "{url}");
    }
    for url in [
        "https://user:pw@example.com/",
        "https://",
        "http://",
        "javascript:alert(1)",
        "https://example.com/a b",
        "https://example.com/\n",
        "file:///C:/x",
    ] {
        assert!(!SearchResultAction::OpenWebUrl { url: url.to_string() }.is_safe(), "{url}");
        assert!(!SearchResultAction::OpenBookmark { url: url.to_string() }.is_safe(), "{url}");
    }
}

#[test]
fn web_search_urls_use_the_fixed_engine_allowlist_and_percent_encode_the_query() {
    assert_eq!(
        web_search_url(WebSearchEngine::Google, "rust lang").as_deref(),
        Some("https://www.google.com/search?q=rust%20lang")
    );
    assert_eq!(
        web_search_url(WebSearchEngine::DuckDuckGo, "rust lang").as_deref(),
        Some("https://duckduckgo.com/?q=rust%20lang")
    );
    assert_eq!(
        web_search_url(WebSearchEngine::Bing, "rust lang").as_deref(),
        Some("https://www.bing.com/search?q=rust%20lang")
    );
    assert_eq!(
        web_search_url(WebSearchEngine::Brave, "rust lang").as_deref(),
        Some("https://search.brave.com/search?q=rust%20lang")
    );
    assert_eq!(
        web_search_url(WebSearchEngine::Google, "c++ & rust/%#?=").as_deref(),
        Some("https://www.google.com/search?q=c%2B%2B%20%26%20rust%2F%25%23%3F%3D")
    );
    assert_eq!(
        web_search_url(WebSearchEngine::Google, "café").as_deref(),
        Some("https://www.google.com/search?q=caf%C3%A9")
    );
    assert_eq!(
        web_search_url(WebSearchEngine::Google, "a-b_c.d~e").as_deref(),
        Some("https://www.google.com/search?q=a-b_c.d~e")
    );
}

#[test]
fn web_search_url_rejects_empty_oversized_and_control_character_queries() {
    assert_eq!(web_search_url(WebSearchEngine::Google, ""), None);
    assert_eq!(web_search_url(WebSearchEngine::Google, "   "), None);
    assert_eq!(web_search_url(WebSearchEngine::Google, "a\nb"), None);
    assert_eq!(web_search_url(WebSearchEngine::Google, "a\u{0}b"), None);
    assert!(web_search_url(WebSearchEngine::Google, &"a".repeat(MAX_WEB_QUERY_CHARS)).is_some());
    assert_eq!(web_search_url(WebSearchEngine::Google, &"a".repeat(MAX_WEB_QUERY_CHARS + 1)), None);
}

#[test]
fn every_generated_search_url_passes_backend_revalidation() {
    for engine in [
        WebSearchEngine::Google,
        WebSearchEngine::DuckDuckGo,
        WebSearchEngine::Bing,
        WebSearchEngine::Brave,
    ] {
        let url = web_search_url(engine, "hello world & more").expect("url");
        assert_eq!(validate_search_open_url(&url).as_deref(), Ok(url.as_str()));
        assert!(SearchResultAction::OpenWebUrl { url }.is_safe());
    }
}

#[test]
fn fallback_row_exists_for_any_non_empty_query_and_never_carries_the_raw_query_in_the_url() {
    let run = search_web("hello world", 10, WebSearchEngine::Google);
    assert_eq!(run.results.len(), 1, "{:#?}", run.results);
    let row = &run.results[0];
    assert_eq!(row.id, "web:search");
    assert!(row.record_key.starts_with("web:search"), "{}", row.record_key);
    assert_eq!(row.provider_id, SearchProviderId::Web);
    assert_eq!(row.kind, SearchResultKind::Web);
    assert!(row.title.contains("Search the web for") && row.title.contains("hello world"), "{}", row.title);
    assert_eq!(
        row.action,
        SearchResultAction::OpenWebUrl {
            url: "https://www.google.com/search?q=hello%20world".to_string()
        }
    );
    assert!(row.action.is_safe());
    assert_eq!(run.timing.provider_id, SearchProviderId::Web);
    assert_eq!(run.health.provider_id, SearchProviderId::Web);
    assert_eq!(run.health.state, SearchProviderHealthState::Ready);
}

#[test]
fn fallback_row_follows_the_configured_engine() {
    for (engine, prefix) in [
        (WebSearchEngine::Google, "https://www.google.com/search?q="),
        (WebSearchEngine::DuckDuckGo, "https://duckduckgo.com/?q="),
        (WebSearchEngine::Bing, "https://www.bing.com/search?q="),
        (WebSearchEngine::Brave, "https://search.brave.com/search?q="),
    ] {
        let run = search_web("tauri", 10, engine);
        match &run.results.last().expect("fallback").action {
            SearchResultAction::OpenWebUrl { url } => assert!(url.starts_with(prefix), "{url}"),
            other => panic!("unexpected action {other:?}"),
        }
    }
}

#[test]
fn fallback_row_is_absent_for_blank_oversized_or_control_character_queries() {
    let oversized = "x".repeat(MAX_WEB_QUERY_CHARS + 1);
    for query in ["", "   ", "a\nb", oversized.as_str()] {
        assert!(search_web(query, 10, WebSearchEngine::Google).results.is_empty(), "{query:?}");
    }
}

#[test]
fn url_like_query_yields_open_url_row_before_the_fallback_row() {
    let run = search_web("github.com", 10, WebSearchEngine::Google);
    assert_eq!(run.results.len(), 2, "{:#?}", run.results);
    assert_eq!(run.results[0].id, "web:open-url");
    assert_eq!(run.results[0].kind, SearchResultKind::Web);
    assert_eq!(run.results[0].provider_id, SearchProviderId::Web);
    match &run.results[0].action {
        SearchResultAction::OpenWebUrl { url } => {
            assert_eq!(url.trim_end_matches('/'), "https://github.com");
        }
        other => panic!("unexpected action {other:?}"),
    }
    assert!(run.results[0].action.is_safe());
    assert_eq!(run.results[1].id, "web:search");
}

#[test]
fn dangerous_url_like_queries_only_ever_produce_the_search_fallback() {
    for query in ["javascript:alert(1)", "file:///C:/Windows/notepad.exe", "https://u:p@example.com", "readme.md"] {
        let run = search_web(query, 10, WebSearchEngine::Google);
        assert!(run.results.iter().all(|row| row.id != "web:open-url"), "{query:?}");
        for row in &run.results {
            assert!(row.action.is_safe(), "{query:?}: {:?}", row.action);
            match &row.action {
                SearchResultAction::OpenWebUrl { url } => {
                    assert!(url.starts_with("https://www.google.com/search?q="), "{url}");
                }
                other => panic!("unexpected action {other:?}"),
            }
        }
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

fn app_row(index: usize, title: &str) -> SearchResult {
    row(
        &format!("app:{index}"),
        SearchProviderId::Apps,
        SearchResultKind::App,
        title,
        SearchResultAction::OpenApp { path: format!(r"C:\Apps\{title}.lnk") },
    )
}

fn file_row(title: &str) -> SearchResult {
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

#[test]
fn fallback_row_never_outranks_any_other_result_even_exact_weak_matches() {
    let fallback = search_web("foo", 10, WebSearchEngine::Google).results;
    let mut rows = vec![file_row("a-very-weak-foo-match.txt")];
    rows.extend(fallback);
    rows.push(app_row(1, "Foo"));
    let ranked = rank_visible_results("foo", rows, 10);
    assert_eq!(ranked.last().map(|r| r.id.as_str()), Some("web:search"), "{ranked:#?}");
    assert!(ranked.len() >= 3, "{ranked:#?}");
}

#[test]
fn fallback_row_is_kept_in_last_slot_when_the_limit_truncates_other_results() {
    let mut rows = vec![app_row(1, "Foo One"), app_row(2, "Foo Two"), app_row(3, "Foo Three")];
    rows.extend(search_web("foo", 10, WebSearchEngine::Google).results);
    let ranked = rank_visible_results("foo", rows, 2);
    assert_eq!(ranked.len(), 2, "{ranked:#?}");
    assert_eq!(ranked[1].id, "web:search");
    assert_ne!(ranked[0].id, "web:search");
}

#[test]
fn fallback_row_is_the_only_result_when_nothing_else_matches() {
    let ranked = rank_visible_results("zzqx", search_web("zzqx", 10, WebSearchEngine::Google).results, 10);
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].id, "web:search");
}

#[test]
fn open_url_row_ranks_above_the_fallback_row_and_survives_the_text_matcher() {
    let ranked = rank_visible_results("github.com", search_web("github.com", 10, WebSearchEngine::Google).results, 10);
    let ids = ranked.iter().map(|r| r.id.as_str()).collect::<Vec<_>>();
    assert_eq!(ids, vec!["web:open-url", "web:search"]);
}

#[test]
fn engine_appends_the_web_fallback_last_and_honours_the_configured_engine() {
    use crate::search::SearchProviderInputs;
    let inputs = SearchProviderInputs { web_engine: WebSearchEngine::DuckDuckGo, ..Default::default() };
    let response = crate::search::run_search_engine_with_inputs(
        SearchQueryRequest::new("display settings", 7_101),
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
    assert!(response.results.len() >= 2, "{:#?}", response.results);
    let last = response.results.last().unwrap();
    assert_eq!(last.id, "web:search");
    assert_ne!(response.results[0].id, "web:search", "fallback must not be first when others exist");
    match &last.action {
        SearchResultAction::OpenWebUrl { url } => assert!(url.starts_with("https://duckduckgo.com/?q="), "{url}"),
        other => panic!("unexpected {other:?}"),
    }
    assert!(response.provider_timings.iter().any(|t| t.provider_id == SearchProviderId::Web));
}

#[test]
fn web_engine_setting_defaults_to_google_and_omitted_field_migrates() {
    assert_eq!(SearchSettings::default().web_engine, WebSearchEngine::Google);
    let migrated: SearchSettings = serde_json::from_value(serde_json::json!({ "resultLimit": 10 })).expect("legacy search settings");
    assert_eq!(migrated.web_engine, WebSearchEngine::Google);
    assert_eq!(migrated.result_limit, 10);
    let empty: SearchSettings = serde_json::from_value(serde_json::json!({})).expect("empty search settings");
    assert_eq!(empty.web_engine, WebSearchEngine::Google);
}

#[test]
fn web_engine_setting_round_trips_exactly_the_four_allowlisted_names() {
    for (name, engine) in [
        ("google", WebSearchEngine::Google),
        ("duckduckgo", WebSearchEngine::DuckDuckGo),
        ("bing", WebSearchEngine::Bing),
        ("brave", WebSearchEngine::Brave),
    ] {
        let parsed: SearchSettings = serde_json::from_value(serde_json::json!({ "webEngine": name })).expect(name);
        assert_eq!(parsed.web_engine, engine);
        let json = serde_json::to_value(&parsed).expect("serialize");
        assert_eq!(json["webEngine"], name);
    }
    for invalid in ["yahoo", "Google", "", "https://evil.example/?q=", "ecosia"] {
        let result = serde_json::from_value::<SearchSettings>(serde_json::json!({ "webEngine": invalid }));
        assert!(result.is_err(), "{invalid:?} must not deserialize");
    }
}

// ---- Review H1: canonical action wire shape shared with the TS tests (tests/fixtures/search-actions.json)

const ACTION_FIXTURE: &str = include_str!("../../../../tests/fixtures/search-actions.json");

#[test]
fn every_action_variant_serializes_to_the_shared_fixture_shape() {
    let fixture: serde_json::Value = serde_json::from_str(ACTION_FIXTURE).expect("fixture json");
    let actions = fixture["actions"].as_object().expect("actions map");
    let mut kinds = std::collections::BTreeSet::new();
    for (name, wire) in actions {
        let parsed: SearchResultAction =
            serde_json::from_value(wire.clone()).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(parsed.is_safe(), "{name} must be safe");
        let serialized = serde_json::to_value(&parsed).expect("serialize");
        assert_eq!(&serialized, wire, "{name}: serialized action differs from fixture");
        kinds.insert(wire["kind"].as_str().unwrap().to_string());
    }
    let expected = [
        "openApp", "focusWindow", "openFile", "openFolder", "runCommand", "openSetting",
        "runControlPanel", "copyText", "openWebUrl", "openBookmark",
    ];
    assert_eq!(kinds, expected.iter().map(|s| s.to_string()).collect());
}

#[test]
fn control_panel_action_without_args_omits_the_args_key() {
    let action = SearchResultAction::RunControlPanel { executable: "control.exe".to_string(), args: None };
    let json = serde_json::to_value(&action).unwrap();
    assert!(json.get("args").is_none(), "args must be omitted when None: {json}");
    let with_args = SearchResultAction::RunControlPanel {
        executable: "control.exe".to_string(),
        args: Some(vec!["Microsoft.Display".to_string()]),
    };
    assert_eq!(serde_json::to_value(&with_args).unwrap()["args"][0], "Microsoft.Display");
}

#[test]
fn action_fields_are_camel_case_on_the_wire() {
    let focus = serde_json::to_value(SearchResultAction::FocusWindow { window_id: "1".into() }).unwrap();
    assert_eq!(focus["windowId"], "1");
    let run = serde_json::to_value(SearchResultAction::RunCommand { command_id: "quick:a".into() }).unwrap();
    assert_eq!(run["commandId"], "quick:a");
    for json in [focus, run] {
        assert!(json.as_object().unwrap().keys().all(|k| !k.contains('_')), "{json}");
    }
}
