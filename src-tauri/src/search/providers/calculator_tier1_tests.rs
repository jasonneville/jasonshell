//! Tier 1 (PRD-01) RED tests for the Calculator search provider.
//!
//! UNWIRED on purpose. The fixer adds, in `providers/mod.rs`:
//!   `pub(crate) mod calculator;`
//!   `#[cfg(test)] mod calculator_tier1_tests;`
//!
//! Required seams (all in `providers/calculator.rs`, no new dependency, no shell/eval):
//!   pub(crate) const MAX_CALCULATOR_QUERY_CHARS: usize = 256;   // raw, untrimmed char count
//!   pub(crate) const MAX_CALCULATOR_NESTING_DEPTH: usize = 32;  // parens AND stacked prefix ops
//!   pub(crate) struct CalculatorValue { pub(crate) text: String,
//!       pub(crate) hex: Option<String>, pub(crate) bin: Option<String> }
//!   pub(crate) fn evaluate_calculator_query(query: &str) -> Option<CalculatorValue>
//!   pub(crate) struct CalculatorSearchRun { pub(crate) results: Vec<SearchResult>,
//!       pub(crate) timing: SearchProviderTiming, pub(crate) health: SearchProviderHealth }
//!   pub(crate) fn search_calculator(query: &str, limit: usize) -> CalculatorSearchRun
//! plus, in `search/mod.rs`:
//!   #[derive(Clone, Debug, Default)]
//!   pub(crate) struct SearchProviderInputs {
//!       pub(crate) web_engine: crate::settings::WebSearchEngine,
//!       pub(crate) quick_commands: Vec<crate::settings::QuickCommandEntry>,
//!       pub(crate) bookmarks_local_app_data: Option<std::path::PathBuf>, // None = no bookmarks; production passes Some(%LOCALAPPDATA%)
//!   }
//!   pub(crate) fn run_search_engine_with_inputs(request: SearchQueryRequest,
//!       inputs: &SearchProviderInputs, on_progress: impl FnMut(SearchProgressPayload),
//!       search_everything_impl: impl Fn(&str, usize) -> providers::everything::EverythingSearchRun)
//!       -> SearchEngineResponse
//!   (`run_search_engine_with_everything` must delegate to it with `SearchProviderInputs::default()`.)
//!
//! Decided semantics (tests are the spec):
//! * Precedence low->high: `|`, `xor`, `&`, `<< >>`, `+ -`, `* / %`, unary (`-`, `~`), `^` (right assoc).
//!   So `-2^2 == -4`, `2^-1 == 0.5`, `2^3^2 == 512`, `1 + 2 << 3 == 24`.
//! * `%` is binary remainder only (sign of the dividend). There is NO postfix percent: `50%` and
//!   `50% * 2` are not calculations.
//! * Bitwise operands must be integers; shift counts 0..=63 else no result. `>>` is arithmetic.
//! * Query must contain a binary operator or `~`. A lone leading minus does not count.
//! * Ambiguous patterns are rejected (no result), digits and one operator class only, no whitespace:
//!   one hyphen evaluates (`5-3`=2, `100-37`=63) EXCEPT phone/date shapes: 3 digits-hyphen-4 digits
//!   (`555-1234`), 4 digits-hyphen-1or2 digits (`2026-11`), and any 2+ hyphens (`2026-11-23`, `1-800-555`).
//!   One slash evaluates (`3/4`); 2+ slashes (`2026/11/23`) give no result. A leading `=` forces evaluation
//!   and bypasses these heuristics and the operator requirement.
//! * Copy validation: `crate::search_panel::validate_copy_search_text(&str) -> Result<(), String>`
//!   (pure; nonempty, <= 4096 chars, no NUL) backs the `copy_search_text` command.
//! * Numbers: 12 significant digits, no float noise; integers exact up to 2^53; larger magnitudes
//!   use scientific text (`1.84467440737e19`); `-0` prints `0`. hex/bin only for 0 <= n <= 2^53 integers.

use super::calculator::{
    evaluate_calculator_query, search_calculator, MAX_CALCULATOR_NESTING_DEPTH,
    MAX_CALCULATOR_QUERY_CHARS,
};
use crate::search::contracts::{
    iso_now, SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState,
    SearchProviderId, SearchProviderTiming, SearchQueryRequest, SearchResult, SearchResultAction,
    SearchResultKind,
};
use crate::search::providers::everything::test_everything_run;
use crate::search::scoring::rank_visible_results;

fn calc(query: &str) -> Option<String> {
    evaluate_calculator_query(query).map(|value| value.text)
}

fn assert_calc(query: &str, expected: &str) {
    assert_eq!(calc(query).as_deref(), Some(expected), "query {query:?}");
}

fn assert_no_calc(query: &str) {
    assert_eq!(calc(query), None, "query {query:?} must not produce a result");
}

#[test]
fn arithmetic_operators_and_precedence() {
    assert_calc("1+2", "3");
    assert_calc("7/2", "3.5");
    assert_calc("10 / 4", "2.5");
    assert_calc("2 * (3 + 4)", "14");
    assert_calc("(2+3)*(4-1)", "15");
    assert_calc("2+3*4", "14");
    assert_calc("-3 + 5", "2");
    assert_calc("5 - -3", "8");
    assert_calc("-(2+3)", "-5");
}

#[test]
fn power_is_right_associative_and_binds_tighter_than_unary_minus() {
    assert_calc("2^10", "1024");
    assert_calc("2^3^2", "512");
    assert_calc("(2^3)^2", "64");
    assert_calc("-2^2", "-4");
    assert_calc("(-2)^2", "4");
    assert_calc("2^-1", "0.5");
}

#[test]
fn percent_is_remainder_only_and_postfix_percent_is_not_a_calculation() {
    assert_calc("10 % 3", "1");
    assert_calc("-7 % 3", "-1");
    assert_calc("7.5 % 2", "1.5");
    assert_no_calc("5 % 0");
    assert_no_calc("50%");
    assert_no_calc("50% * 2");
    assert_no_calc("200 * 10%");
}

#[test]
fn decimals_scientific_notation_and_deterministic_formatting() {
    assert_calc("0.1+0.2", "0.3");
    assert_calc("0.1*3", "0.3");
    assert_calc("1/3", "0.333333333333");
    assert_calc("2^0.5", "1.41421356237");
    assert_calc("1e3+1", "1001");
    assert_calc("1.5e-3*2", "0.003");
    assert_calc("2E3 * 1", "2000");
    assert_calc("123456789*1000", "123456789000");
    assert_calc("0*-1", "0");
}

#[test]
fn large_integers_are_exact_up_to_two_to_the_fifty_third_then_scientific() {
    assert_calc("2^53", "9007199254740992");
    assert_calc("2^53 - 1", "9007199254740991");
    assert_calc("1e15 + 1", "1000000000000001");
    assert_calc("2^64", "1.84467440737e19");
}

#[test]
fn radix_literals_bitwise_operators_and_xor_keyword() {
    assert_calc("0xFF + 1", "256");
    assert_calc("0XFF+1", "256");
    assert_calc("0b101 * 2", "10");
    assert_calc("0o17 + 1", "16");
    assert_calc("5 & 3", "1");
    assert_calc("5 | 3", "7");
    assert_calc("5 xor 3", "6");
    assert_calc("5 XOR 3", "6");
    assert_calc("~5", "-6");
    assert_calc("1 << 4", "16");
    assert_calc("256 >> 4", "16");
    assert_calc("-8 >> 1", "-4");
    assert_calc("0xFF & 0x0F", "15");
    assert_calc("1 + 2 << 3", "24");
    assert_calc("1 | 2 & 3", "3");
    assert_calc("6 & 3 | 8", "10");
}

#[test]
fn malformed_radix_and_bitwise_operands_yield_no_result() {
    assert_no_calc("0x + 1");
    assert_no_calc("0xG1 + 1");
    assert_no_calc("0b102 + 1");
    assert_no_calc("0o8 + 1");
    assert_no_calc("1.5 & 1");
    assert_no_calc("1 << 64");
    assert_no_calc("1 << -1");
    assert_no_calc("5 xor");
}

#[test]
fn integer_results_carry_hex_and_binary_secondary_text() {
    let value = evaluate_calculator_query("6*7").expect("6*7");
    assert_eq!(value.text, "42");
    assert_eq!(value.hex.as_deref(), Some("0x2A"));
    assert_eq!(value.bin.as_deref(), Some("0b101010"));

    let shifted = evaluate_calculator_query("1<<4").expect("1<<4");
    assert_eq!(shifted.hex.as_deref(), Some("0x10"));

    let zero = evaluate_calculator_query("0*1").expect("0*1");
    assert_eq!(zero.hex.as_deref(), Some("0x0"));
    assert_eq!(zero.bin.as_deref(), Some("0b0"));

    let literal = evaluate_calculator_query("0xFF + 1").expect("0xFF+1");
    assert_eq!(literal.hex.as_deref(), Some("0x100"));
    assert_eq!(literal.bin.as_deref(), Some("0b100000000"));
}

#[test]
fn non_integer_and_negative_results_have_no_hex_or_binary() {
    for query in ["0.5+1", "~5", "-3 + 1", "7/2"] {
        let value = evaluate_calculator_query(query).unwrap_or_else(|| panic!("{query}"));
        assert_eq!((value.hex, value.bin), (None, None), "query {query:?}");
    }
}

#[test]
fn division_by_zero_overflow_and_nan_yield_no_result() {
    assert_no_calc("1/0");
    assert_no_calc("0/0");
    assert_no_calc("1e308*10");
    assert_no_calc("2^1024");
    assert_no_calc("1e400 + 1");
    assert_no_calc("0^-1");
    assert_no_calc("(-8)^0.5");
    assert_no_calc("1 / (2 - 2)");
}

#[test]
fn queries_without_an_operator_are_not_calculations() {
    for query in [
        "42", "2024", "-5", "- 5", "(5)", "0xFF", "1e3", "hello world", "readme.md",
        "1.2.3", "10.0.0.1", "c++", "node-v20", "a-b", "pi", "x*2", "a+b",
    ] {
        assert_no_calc(query);
    }
}

#[test]
fn ambiguous_date_phone_and_version_patterns_are_rejected() {
    for query in [
        "2026-11-23",
        "1-800-555",
        "555-123-4567",
        "555-1234",
        "2026-11",
        "2026-1",
        "5--3",
        "2026/11/23",
        "1/2/3",
        "8/2/2",
        "(555) 123-4567",
        "+1 555 123 4567",
        "1.2.3+1",
    ] {
        assert_no_calc(query);
    }
    // Same text with explicit spacing, other operators, or a forced '=' is a calculation.
    assert_calc("5-3", "2");
    assert_calc("100-37", "63");
    assert_calc("10-3", "7");
    assert_calc("5 - 3", "2");
    assert_calc("5-3*1", "2");
    assert_calc("8 / 2 / 2", "2");
    assert_calc("3/4", "0.75");
    assert_calc("=5-3", "2");
    assert_calc("=2026-11-23", "1992");
}

#[test]
fn malformed_syntax_yields_no_result() {
    for query in [
        "2+", "+2", "*2", "2**3", "2 +* 3", "()", "(2", "2)", "(2+3", "2(3)", "1,000+2",
        "2 3 +", "2 + + 3x",
    ] {
        assert_no_calc(query);
    }
}

#[test]
fn leading_equals_forces_evaluation_of_plain_values() {
    assert_calc("=2+3", "5");
    assert_calc("= 2 + 3", "5");
    assert_calc("=42", "42");
    assert_calc("= 2024", "2024");
    assert_calc("=0xFF", "255");
    assert_eq!(
        evaluate_calculator_query("=0xFF").and_then(|value| value.hex).as_deref(),
        Some("0xFF")
    );
    for query in ["=", "= ", "==1+1", "=abc", "=2+", "=1/0"] {
        assert_no_calc(query);
    }
}

#[test]
fn whitespace_is_tolerated_but_control_characters_are_not() {
    assert_calc("  2  +\t3 ", "5");
    assert_no_calc("2+\n3");
    assert_no_calc("2+\u{7}3");
    assert_no_calc("2+\u{0}3");
}

#[test]
fn query_length_cap_counts_the_raw_untrimmed_query() {
    let at_cap = format!("1{}", "+1".repeat(127)) + " ";
    assert_eq!(at_cap.chars().count(), MAX_CALCULATOR_QUERY_CHARS);
    assert_calc(&at_cap, "128");
    let over_cap = format!("{at_cap} ");
    assert_eq!(over_cap.chars().count(), MAX_CALCULATOR_QUERY_CHARS + 1);
    assert_no_calc(&over_cap);
    assert_no_calc(&"9".repeat(5_000));
}

#[test]
fn nesting_depth_cap_covers_parentheses_and_stacked_prefix_operators() {
    let nested = |depth: usize| format!("{}1+1{}", "(".repeat(depth), ")".repeat(depth));
    assert_calc(&nested(MAX_CALCULATOR_NESTING_DEPTH), "2");
    assert_no_calc(&nested(MAX_CALCULATOR_NESTING_DEPTH + 1));
    assert_no_calc(&nested(10_000));

    let tildes = |depth: usize| format!("{}1", "~".repeat(depth));
    assert_calc(&tildes(MAX_CALCULATOR_NESTING_DEPTH), "1");
    assert_no_calc(&tildes(MAX_CALCULATOR_NESTING_DEPTH + 1));
}

#[test]
fn evaluation_is_pure_and_deterministic() {
    for query in ["6*7", "0.1+0.2", "2^3^2", "0xFF & 0x0F", "not math"] {
        assert_eq!(calc(query), calc(query));
    }
}

fn run_engine(query: &str) -> crate::search::contracts::SearchEngineResponse {
    let request = SearchQueryRequest::new(query, 7_001);
    crate::search::run_search_engine_with_inputs(
        request,
        &crate::search::SearchProviderInputs::default(),
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
    )
}

#[test]
fn provider_row_copies_the_plain_result_through_copy_text() {
    let run = search_calculator("6*7", 10);
    assert_eq!(run.results.len(), 1);
    let row = &run.results[0];
    assert_eq!(row.id, "calculator:result");
    assert!(row.record_key.starts_with("calculator:result"), "{}", row.record_key);
    assert_eq!(row.provider_id, SearchProviderId::Calculator);
    assert_eq!(row.kind, SearchResultKind::Calculator);
    assert_eq!(row.title, "42");
    assert_eq!(row.action, SearchResultAction::CopyText { text: "42".to_string() });
    assert!(row.action.is_safe());
    let subtitle = row.subtitle.as_deref().unwrap_or_default();
    assert!(subtitle.contains("0x2A") && subtitle.contains("0b101010"), "{subtitle:?}");
    assert_eq!(run.timing.provider_id, SearchProviderId::Calculator);
    assert_eq!(run.timing.result_count, 1);
    assert_eq!(run.health.provider_id, SearchProviderId::Calculator);
    assert_eq!(run.health.state, SearchProviderHealthState::Ready);
}

#[test]
fn provider_row_for_fractions_has_no_radix_text_and_non_calculations_have_no_rows() {
    let run = search_calculator("0.5+1", 10);
    let subtitle = run.results[0].subtitle.clone().unwrap_or_default();
    assert!(!subtitle.contains("0x") && !subtitle.contains("0b"), "{subtitle:?}");
    assert_eq!(run.results[0].action, SearchResultAction::CopyText { text: "1.5".to_string() });
    for query in ["42", "c++", "", "   ", "2026-11-23", "1/0"] {
        let run = search_calculator(query, 10);
        assert!(run.results.is_empty(), "query {query:?}");
        assert_eq!(run.timing.result_count, 0);
    }
}

fn row(
    id: &str,
    provider_id: SearchProviderId,
    kind: SearchResultKind,
    title: &str,
    action: SearchResultAction,
) -> SearchResult {
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

#[test]
fn valid_calculation_is_ranked_first_and_is_not_dropped_by_the_text_matcher() {
    // Title "42" does not textually match query "6*7"; ranking must still keep and top it.
    let calculator = search_calculator("6*7", 10).results;
    let app = row(
        "app:six-seven",
        SearchProviderId::Apps,
        SearchResultKind::App,
        "6*7",
        SearchResultAction::OpenApp { path: r"C:\Apps\Six.lnk".to_string() },
    );
    let mut rows = vec![app];
    rows.extend(calculator);
    let ranked = rank_visible_results("6*7", rows, 10);
    assert_eq!(ranked.first().map(|r| r.id.as_str()), Some("calculator:result"), "{ranked:#?}");
}

#[test]
fn leading_equals_query_still_ranks_the_calculation_first() {
    let ranked = rank_visible_results("=2+3", search_calculator("=2+3", 10).results, 10);
    assert_eq!(ranked.first().map(|r| r.title.as_str()), Some("5"));
}

#[test]
fn engine_returns_calculation_first_and_reports_calculator_timing_and_health() {
    let response = run_engine("6*7");
    let first = response.results.first().expect("calculator result");
    assert_eq!(first.id, "calculator:result");
    assert_eq!(first.action, SearchResultAction::CopyText { text: "42".to_string() });
    assert!(response
        .provider_timings
        .iter()
        .any(|timing| timing.provider_id == SearchProviderId::Calculator));
    assert!(response
        .health
        .iter()
        .any(|health| health.provider_id == SearchProviderId::Calculator));
}

#[test]
fn engine_does_not_pollute_ordinary_queries_with_calculator_rows() {
    for query in ["42", "2024", "c++", "node-v20", "readme.md", "2026-11-23", "display settings"] {
        let response = run_engine(query);
        assert!(
            response.results.iter().all(|r| r.kind != SearchResultKind::Calculator),
            "query {query:?}: {:#?}",
            response.results
        );
    }
}

use crate::search_panel::validate_copy_search_text;

#[test]
fn copy_text_validator_accepts_plain_results_and_rejects_empty_oversized_and_nul() {
    assert!(validate_copy_search_text("42").is_ok());
    assert!(validate_copy_search_text("1.84467440737e19").is_ok());
    assert!(validate_copy_search_text(&"a".repeat(4096)).is_ok());
    assert!(validate_copy_search_text("").is_err());
    assert!(validate_copy_search_text(&"a".repeat(4097)).is_err());
    assert!(validate_copy_search_text("a\0b").is_err());
}

// ---- Review L2: results above 2^53 must be exact or absent, never silently rounded.
#[test]
fn integers_beyond_two_to_the_fifty_third_are_exact_or_no_result() {
    for (query, wrong) in [
        ("2^53 + 1", "9007199254740992"),
        ("9007199254740993 + 0", "9007199254740992"),
        ("~(2^53)", "-9007199254740992"),
        ("2^53 * 2 + 1", "18014398509481984"),
    ] {
        let value = calc(query);
        assert_ne!(value.as_deref(), Some(wrong), "{query} silently rounded");
    }
    // Whenever such a query does produce text it must be the exact integer.
    if let Some(text) = calc("2^53 + 1") {
        assert_eq!(text, "9007199254740993");
    }
}

#[test]
fn bitwise_and_on_large_literals_uses_exact_integer_arithmetic_or_gives_no_result() {
    let value = calc("0x20000000000001 & 1");
    assert!(value.as_deref() == Some("1") || value.is_none(), "{value:?}");
    let shifted = calc("0x20000000000001 >> 1");
    assert!(shifted.as_deref() == Some("4503599627370496") || shifted.is_none(), "{shifted:?}");
    // Hex/bin secondary text is only produced for exactly representable integers.
    assert!(evaluate_calculator_query("2^53 + 1").map_or(true, |v| v.hex.is_none() || v.text == "9007199254740993"));
}
