//! Tier 1 (PRD-03) RED tests for the Quick Commands search provider and its launch path.
//!
//! UNWIRED on purpose. The fixer adds, in `providers/mod.rs`:
//!   `pub(crate) mod quick_commands;`
//!   `#[cfg(test)] mod quick_commands_tier1_tests;`
//! (and the `SearchProviderInputs` seam described in `calculator_tier1_tests.rs`).
//!
//! Required seams:
//!   providers/quick_commands.rs
//!     pub(crate) struct QuickCommandsSearchRun { pub(crate) results: Vec<SearchResult>,
//!         pub(crate) timing: SearchProviderTiming, pub(crate) health: SearchProviderHealth }
//!     pub(crate) fn search_quick_commands(query: &str, limit: usize,
//!         entries: &[crate::settings::QuickCommandEntry]) -> QuickCommandsSearchRun
//!   crate::quick_commands (pure planner, no I/O, no spawn)
//!     #[derive(Clone, Debug, PartialEq, Eq)]
//!     pub(crate) enum SearchQuickCommandLaunch { Run(QuickCommandEntry), OpenPanel(QuickCommandEntry) }
//!     pub(crate) fn plan_search_quick_command_launch(settings: &crate::settings::ShellSettings,
//!         id: &str, running_command_ids: &[String]) -> Result<SearchQuickCommandLaunch, String>
//!     #[tauri::command] pub fn run_search_quick_command(window: WebviewWindow, app_handle: AppHandle,
//!         request: RunSearchQuickCommandRequest) -> Result<QuickCommandSpawnResult, String>
//!     pub struct RunSearchQuickCommandRequest { pub id: String }      // camelCase json; ID ONLY
//!       caller policy: StackCommandAuth::AllowedCallers { command: RUN_SEARCH_QUICK_COMMAND,
//!       callers: &[crate::shell_windows::TOP_BAR_LABEL] } as the FIRST statement (search-row
//!       activation is processed by the top-bar webview); it goes through the same job-tracked
//!       `spawn_quick_command_with_completion` path as `run_quick_command`, honours graceful-exit
//!       admission, and when the plan is `OpenPanel` returns Err("...open the Quick Commands panel...")
//!       without launching. Handler must also refuse (Err containing "already running") when a run
//!       for the same command id is live (double-Enter guard).
//!
//! Provider rules: only saved entries that pass `validate_quick_command_entry` produce rows (there is
//! no per-entry `enabled` flag in the model today; "enabled" == valid). Row: id/record_key
//! `quick:<id>`, provider Commands, kind Command, title = label, subtitle Some("Quick command"),
//! action RunCommand{command_id:"quick:<id>"}. Matching reuses the shared matcher over label +
//! command-block lines + target file name. NEVER put args, cwd, artifact location or full target
//! path into any serialized field. Launch rule: entries whose command lines emit the quick-command
//! input marker (`\x1b]777;JasonShellQuickCommandInput;`) need the interactive panel => OpenPanel.

use super::quick_commands::search_quick_commands;
use crate::quick_commands::{plan_search_quick_command_launch, SearchQuickCommandLaunch};
use crate::search::contracts::{
    iso_now, SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState,
    SearchProviderId, SearchProviderTiming, SearchQueryRequest, SearchResultAction,
    SearchResultKind,
};
use crate::search::providers::everything::test_everything_run;
use crate::settings::{QuickCommandEntry, QuickCommandMode, ShellSettings};

fn direct(id: &str, label: &str, target: &str) -> QuickCommandEntry {
    QuickCommandEntry {
        id: id.to_string(),
        label: label.to_string(),
        mode: QuickCommandMode::Direct,
        target_path: target.to_string(),
        args: Vec::new(),
        commands: Vec::new(),
        cwd: None,
        artifact_location: None,
    }
}

fn block(id: &str, label: &str, commands: &[&str]) -> QuickCommandEntry {
    QuickCommandEntry {
        id: id.to_string(),
        label: label.to_string(),
        mode: QuickCommandMode::CommandBlock,
        target_path: String::new(),
        args: Vec::new(),
        commands: commands.iter().map(|c| c.to_string()).collect(),
        cwd: None,
        artifact_location: None,
    }
}

fn titles(run: &super::quick_commands::QuickCommandsSearchRun) -> Vec<&str> {
    run.results.iter().map(|r| r.title.as_str()).collect()
}

#[test]
fn saved_entry_is_found_by_label_with_the_documented_row_shape() {
    let entries = vec![direct("build", "Build project", r"C:\tools\build.exe")];
    let run = search_quick_commands("build", 10, &entries);
    assert_eq!(run.results.len(), 1, "{:#?}", run.results);
    let row = &run.results[0];
    assert_eq!(row.id, "quick:build");
    assert_eq!(row.record_key, "quick:build");
    assert_eq!(row.provider_id, SearchProviderId::Commands);
    assert_eq!(row.kind, SearchResultKind::Command);
    assert_eq!(row.title, "Build project");
    assert_eq!(row.subtitle.as_deref(), Some("Quick command"));
    assert_eq!(row.action, SearchResultAction::RunCommand { command_id: "quick:build".to_string() });
    assert!(row.action.is_safe());
    assert_eq!(run.timing.provider_id, SearchProviderId::Commands);
    assert_eq!(run.timing.result_count, 1);
    assert_eq!(run.health.provider_id, SearchProviderId::Commands);
    assert_eq!(run.health.state, SearchProviderHealthState::Ready);
}

// Review H2: quick commands match the visible LABEL only. Command lines, args, cwd and target
// paths are never searchable (a short query would otherwise match nearly any command text).
#[test]
fn command_text_target_path_and_fuzzy_subsequences_never_match() {
    let ship = block("deploy", "Ship it", &["npm run release-candidate", "git push origin main"]);
    for query in ["main", "release-candidate", "git push", "origin"] {
        assert!(search_quick_commands(query, 10, &[ship.clone()]).results.is_empty(), "{query}");
    }
    let target = direct("tool", "Open tool", r"C:\tools\secret-tool.exe");
    assert!(search_quick_commands("secret", 10, &[target]).results.is_empty());
    let run_e2e = direct("e2e", "Run e2e deploy", r"C:\tools\a.exe");
    assert!(search_quick_commands("rep", 10, &[run_e2e]).results.is_empty(), "no fuzzy subsequence");
    assert!(search_quick_commands("wkly", 10, &[direct("w", "Weekly report", r"C:\tools\a.exe")]).results.is_empty());
}

#[test]
fn label_tiers_are_exact_prefix_word_prefix_and_substring() {
    let entries = vec![
        direct("a", "Report", r"C:\tools\a.exe"),
        direct("b", "Reports archive", r"C:\tools\b.exe"),
        direct("c", "Weekly report", r"C:\tools\c.exe"),
        direct("d", "Xreportx", r"C:\tools\d.exe"),
    ];
    let run = search_quick_commands("report", 10, &entries);
    let order = run.results.iter().map(|r| (r.title.as_str(), r.match_reason.as_str())).collect::<Vec<_>>();
    assert_eq!(
        order,
        vec![
            ("Report", "exact"),
            ("Reports archive", "prefix"),
            ("Weekly report", "wordPrefix"),
            ("Xreportx", "substring"),
        ]
    );
}

#[test]
fn args_cwd_and_artifact_location_never_enter_any_serialized_field() {
    let mut entry = direct("secretive", "Needs token", r"C:\tools\secret-tool.exe");
    entry.args = vec!["--flavor=UNIQUEARG7731".to_string()];
    entry.cwd = Some(r"C:\Users\someone\private-cwd".to_string());
    entry.artifact_location = Some(r"C:\Users\someone\private-artifacts".to_string());
    let run = search_quick_commands("needs", 10, &[entry]);
    assert_eq!(run.results.len(), 1);
    let json = serde_json::to_string(&run.results).unwrap();
    for secret in ["UNIQUEARG7731", "private-cwd", "private-artifacts", r"C:\\tools\\secret-tool.exe", r"C:\tools"] {
        assert!(!json.contains(secret), "{secret} leaked into {json}");
    }
}

#[test]
fn invalid_saved_entries_produce_no_rows() {
    let entries = vec![
        direct("Bad Id", "Uppercase and space id", r"C:\tools\a.exe"),
        direct("blank-label", "   ", r"C:\tools\a.exe"),
        direct("no-target", "Missing target", ""),
        block("empty-block", "Empty block", &[]),
        direct("good", "Good entry", r"C:\tools\a.exe"),
    ];
    let run = search_quick_commands("e", 10, &entries);
    assert_eq!(titles(&run), vec!["Good entry"]);
    for query in ["Uppercase", "Missing target", "Empty block"] {
        assert!(search_quick_commands(query, 10, &entries).results.is_empty(), "{query}");
    }
}

#[test]
fn empty_query_empty_catalog_and_limit_are_handled() {
    let entries = vec![
        direct("one", "Run alpha", r"C:\t\a.exe"),
        direct("two", "Run beta", r"C:\t\b.exe"),
        direct("three", "Run gamma", r"C:\t\c.exe"),
    ];
    assert!(search_quick_commands("", 10, &entries).results.is_empty());
    assert!(search_quick_commands("   ", 10, &entries).results.is_empty());
    assert!(search_quick_commands("run", 10, &[]).results.is_empty());
    assert_eq!(search_quick_commands("run", 2, &entries).results.len(), 2);
    assert_eq!(search_quick_commands("run", 10, &entries).results.len(), 3);
}

#[test]
fn exact_and_prefix_label_matches_sort_before_weaker_matches() {
    let entries = vec![
        direct("a", "Rebuild everything", r"C:\t\a.exe"),
        direct("b", "Build", r"C:\t\b.exe"),
        direct("c", "Build docs", r"C:\t\c.exe"),
    ];
    let run = search_quick_commands("build", 10, &entries);
    assert_eq!(run.results.first().map(|r| r.title.as_str()), Some("Build"));
}

fn settings_with(entries: Vec<QuickCommandEntry>) -> ShellSettings {
    let mut settings = ShellSettings::default();
    settings.quick_commands.entries = entries;
    settings
}

#[test]
fn launch_plan_runs_a_saved_valid_entry_by_id_only() {
    let settings = settings_with(vec![direct("build", "Build", r"C:\tools\build.exe")]);
    match plan_search_quick_command_launch(&settings, "build", &[]) {
        Ok(SearchQuickCommandLaunch::Run(entry)) => assert_eq!(entry.id, "build"),
        other => panic!("expected Run, got {other:?}"),
    }
}

#[test]
fn launch_plan_reports_unknown_deleted_blank_and_invalid_ids_truthfully() {
    let settings = settings_with(vec![
        direct("build", "Build", r"C:\tools\build.exe"),
        direct("Bad Id", "Broken", r"C:\tools\x.exe"),
    ]);
    let unknown = plan_search_quick_command_launch(&settings, "deleted-command", &[]).unwrap_err();
    assert!(unknown.contains("deleted-command") && unknown.contains("not configured"), "{unknown}");
    for id in ["", "   "] {
        assert!(plan_search_quick_command_launch(&settings, id, &[]).is_err(), "{id:?}");
    }
    assert!(plan_search_quick_command_launch(&settings, "Bad Id", &[]).is_err());
    // The renderer must not be able to smuggle command text through the id.
    assert!(plan_search_quick_command_launch(&settings, "quick:build", &[]).is_err());
    assert!(plan_search_quick_command_launch(&settings, "build && calc.exe", &[]).is_err());
}

#[test]
fn launch_plan_refuses_a_second_launch_while_the_same_command_is_running() {
    let settings = settings_with(vec![
        direct("build", "Build", r"C:\tools\build.exe"),
        direct("test", "Test", r"C:\tools\test.exe"),
    ]);
    let running = vec!["build".to_string()];
    let error = plan_search_quick_command_launch(&settings, "build", &running).unwrap_err();
    assert!(error.contains("already running"), "{error}");
    assert!(matches!(
        plan_search_quick_command_launch(&settings, "test", &running),
        Ok(SearchQuickCommandLaunch::Run(_))
    ));
}

#[test]
fn launch_plan_opens_the_panel_instead_of_running_input_driven_commands() {
    let prompting = block(
        "ask",
        "Ask me",
        &["Write-Host \"$([char]27)]777;JasonShellQuickCommandInput;eyJ2ZXJzaW9uIjoxfQ==$([char]7)\""],
    );
    let marker_literal = block("ask2", "Ask literal", &["echo \u{1b}]777;JasonShellQuickCommandInput;abc\u{7}"]);
    let settings = settings_with(vec![prompting, marker_literal, block("plain", "Plain", &["echo hi"])]);
    for id in ["ask", "ask2"] {
        assert!(
            matches!(plan_search_quick_command_launch(&settings, id, &[]), Ok(SearchQuickCommandLaunch::OpenPanel(_))),
            "{id}"
        );
    }
    assert!(matches!(
        plan_search_quick_command_launch(&settings, "plain", &[]),
        Ok(SearchQuickCommandLaunch::Run(_))
    ));
}

#[test]
fn launch_plan_opens_the_panel_for_the_injected_input_helper() {
    let settings = settings_with(vec![
        block("helper", "Helper", &["$x = Request-JasonShellInput -RequestId a -Prompt 'Name'"]),
        block("helper-lower", "Helper lower", &["request-jasonshellinput -RequestId b"]),
    ]);
    for id in ["helper", "helper-lower"] {
        assert!(
            matches!(plan_search_quick_command_launch(&settings, id, &[]), Ok(SearchQuickCommandLaunch::OpenPanel(_))),
            "{id}"
        );
    }
}

#[test]
fn local_phase_payload_withholds_a_lone_web_fallback_but_final_response_keeps_it() {
    use crate::search::contracts::SearchProgressPhase;
    let mut phases = Vec::new();
    let response = crate::search::run_search_engine_with_inputs(
        SearchQueryRequest::new("zzqxnomatchanywhere", 7_302),
        &crate::search::SearchProviderInputs::default(),
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
    let local = phases.iter().find(|p| p.phase == SearchProgressPhase::Local).expect("local phase");
    assert!(local.results.is_empty(), "lone web:search must not flash in the Local phase: {:#?}", local.results);
    assert_eq!(response.results.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(), vec!["web:search"]);
}

#[test]
fn search_launch_handler_is_top_bar_only_id_only_and_authorizes_first() {
    let source = include_str!("../../quick_commands.rs").replace("\r\n", "\n");
    let production = source.split("#[cfg(test)]\nmod tests").next().unwrap();

    let request = production
        .split("pub struct RunSearchQuickCommandRequest")
        .nth(1)
        .expect("RunSearchQuickCommandRequest must exist")
        .split('}')
        .next()
        .unwrap();
    let fields = request.lines().filter(|l| l.contains("pub ")).map(str::trim).collect::<Vec<_>>();
    assert_eq!(fields, vec!["pub id: String,"], "request must carry the command ID only");

    let body = production
        .split("pub fn run_search_quick_command(")
        .nth(1)
        .expect("run_search_quick_command must exist")
        .split("#[tauri::command]")
        .next()
        .unwrap();
    assert!(body.contains("callers: &[crate::shell_windows::TOP_BAR_LABEL]"), "top-bar-only policy");
    assert!(!body.contains("COMMAND_PANEL_LABEL"), "must not widen to command-panel");
    let params = body.split(") ->").next().unwrap();
    assert!(params.contains("WebviewWindow"));
    let guard = body.find("authorize_stack_command(").expect("must authorize caller");
    let first_statement = body.find("{\n").unwrap() + 2;
    assert!(body[first_statement..guard].trim().is_empty(), "authorize before any work");
    assert!(body.contains("admit_mutation"), "graceful-exit admission");
    assert!(body.contains("plan_search_quick_command_launch"), "must use the pure planner");
    assert!(body.contains("spawn_quick_command_with_completion"), "same job-tracked launch path");
}

#[test]
fn engine_returns_quick_command_rows_in_the_commands_group_for_injected_entries() {
    use crate::search::SearchProviderInputs;
    let inputs = SearchProviderInputs {
        quick_commands: vec![direct("zzbuild", "Zzbuild pipeline", r"C:\tools\build.exe")],
        ..Default::default()
    };
    let response = crate::search::run_search_engine_with_inputs(
        SearchQueryRequest::new("zzbuild", 7_301),
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
    let row = response.results.iter().find(|r| r.id == "quick:zzbuild").expect("quick command row");
    assert_eq!(row.kind, SearchResultKind::Command);
    assert_eq!(row.provider_id, SearchProviderId::Commands);
    assert!(response.provider_timings.iter().any(|t| t.provider_id == SearchProviderId::Commands && t.result_count >= 1));
}

// Review follow-up: the prefix tier needs >= 3 typed characters; exact matches work at any length.
#[test]
fn prefix_tier_requires_three_typed_characters_but_exact_works_at_any_length() {
    let reason = |query: &str, label: &str| {
        search_quick_commands(query, 10, &[direct("x", label, r"C:\tools\a.exe")])
            .results
            .first()
            .map(|row| row.match_reason.clone())
    };
    assert_eq!(reason("re", "Reboot PC").as_deref(), Some("wordPrefix"));
    assert_eq!(reason("r", "Reboot PC").as_deref(), Some("wordPrefix"));
    assert_eq!(reason("reb", "Reboot PC").as_deref(), Some("prefix"));
    assert_eq!(reason("rebo", "Reboot PC").as_deref(), Some("prefix"));
    assert_eq!(reason("ab", "AB").as_deref(), Some("exact"));
    assert_eq!(reason("a", "A").as_deref(), Some("exact"));
}

#[test]
fn a_two_character_prefix_is_withheld_from_local_and_never_implicitly_selectable() {
    use crate::search::contracts::SearchProgressPhase;
    let inputs = crate::search::SearchProviderInputs {
        quick_commands: vec![direct("reboot", "Reboot PC", r"C:\tools\a.exe")],
        ..Default::default()
    };
    let mut phases = Vec::new();
    let response = crate::search::run_search_engine_with_inputs(
        SearchQueryRequest::new("re", 7_303),
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
    let local = phases.iter().find(|p| p.phase == SearchProgressPhase::Local).expect("local");
    assert!(local.results.iter().all(|r| r.id != "quick:reboot"));
    let row = response.results.iter().find(|r| r.id == "quick:reboot").expect("kept in final response");
    assert_eq!(row.match_reason, "wordPrefix");
}
