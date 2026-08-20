use crate::search::contracts::{
    SearchProviderId, SearchResult, SearchResultAction, SearchResultKind,
};
use crate::search::matcher::{
    best_match, full_highlight, query_tokens as match_query_tokens, MatchField, MatchTier,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchQuality {
    Exact,
    Prefix,
    Acronym,
    TokenPrefix,
    Subsequence,
    EditDistance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MatchScore {
    quality: MatchQuality,
    score: i32,
    reason: &'static str,
    title_highlight_data: Vec<usize>,
    subtitle_highlight_data: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct QueryIntent {
    app: bool,
    setting: bool,
    folder: bool,
}

pub(crate) fn rank_visible_results(
    query: &str,
    rows: Vec<SearchResult>,
    limit: usize,
) -> Vec<SearchResult> {
    let query_text = normalize(query);
    let tokens = query_tokens(query);
    if tokens.is_empty() {
        return Vec::new();
    }
    let intent = classify_intent(query, &tokens);
    let mut deduped: HashMap<String, SearchResult> = HashMap::new();

    for mut row in rows {
        let Some(score) = score_row(&row, &query_text, &tokens, intent) else {
            continue;
        };
        row.score = score.score;
        row.match_reason = score.reason.to_string();
        row.title_highlight_data = score.title_highlight_data;
        row.subtitle_highlight_data = score.subtitle_highlight_data;
        let key = duplicate_key(&row);
        match deduped.get(&key) {
            Some(existing) if compare_results(&row, existing).is_ge() => {}
            _ => {
                deduped.insert(key, row);
            }
        }
    }

    let mut results = deduped.into_values().collect::<Vec<_>>();
    results.sort_by(compare_results);
    results.truncate(limit);
    results
}

fn score_row(
    row: &SearchResult,
    query: &str,
    tokens: &[String],
    intent: QueryIntent,
) -> Option<MatchScore> {
    let fields = SearchFields::from_result(row);
    let mut score = match_quality_score(row, &fields, row.provider_id, query, tokens)?;

    score.score += kind_base(row.kind);
    score.score += provider_base(row.provider_id);
    score.score += intent_boost(row.kind, intent);
    score.score += open_window_match_boost(row.kind, score.quality);
    score.score += important_folder_boost(row, query, tokens, intent);
    score.score += provider_signal_bonus(row.provider_signal);

    Some(score)
}

fn provider_signal_bonus(provider_signal: i32) -> i32 {
    provider_signal.clamp(0, 50)
}

fn match_quality_score(
    row: &SearchResult,
    fields: &SearchFields,
    provider_id: SearchProviderId,
    query: &str,
    tokens: &[String],
) -> Option<MatchScore> {
    let matched = best_match(
        &row.title,
        row.subtitle.as_deref(),
        &fields.hidden_values,
        query,
        tokens,
        supports_fuzzy(provider_id),
    )?;

    let (quality, title_highlight_data, subtitle_highlight_data) = match matched.tier {
        MatchTier::Exact => match matched.field {
            MatchField::Title => (MatchQuality::Exact, matched.highlight_data, Vec::new()),
            MatchField::Subtitle => (MatchQuality::Exact, Vec::new(), matched.highlight_data),
            MatchField::Hidden => (MatchQuality::Exact, full_highlight(&row.title), Vec::new()),
        },
        MatchTier::Prefix => match matched.field {
            MatchField::Title => (MatchQuality::Prefix, matched.highlight_data, Vec::new()),
            MatchField::Subtitle => (MatchQuality::Prefix, Vec::new(), matched.highlight_data),
            MatchField::Hidden => (MatchQuality::Prefix, full_highlight(&row.title), Vec::new()),
        },
        MatchTier::Acronym => match matched.field {
            MatchField::Title => (MatchQuality::Acronym, matched.highlight_data, Vec::new()),
            MatchField::Subtitle => (MatchQuality::Acronym, Vec::new(), matched.highlight_data),
            MatchField::Hidden => (
                MatchQuality::Acronym,
                full_highlight(&row.title),
                Vec::new(),
            ),
        },
        MatchTier::TokenPrefix => match matched.field {
            MatchField::Title => (
                MatchQuality::TokenPrefix,
                matched.highlight_data,
                Vec::new(),
            ),
            MatchField::Subtitle => (
                MatchQuality::TokenPrefix,
                Vec::new(),
                matched.highlight_data,
            ),
            MatchField::Hidden => (
                MatchQuality::TokenPrefix,
                full_highlight(&row.title),
                Vec::new(),
            ),
        },
        MatchTier::Subsequence => match matched.field {
            MatchField::Title => (
                MatchQuality::Subsequence,
                matched.highlight_data,
                Vec::new(),
            ),
            MatchField::Subtitle => (
                MatchQuality::Subsequence,
                Vec::new(),
                matched.highlight_data,
            ),
            MatchField::Hidden => (
                MatchQuality::Subsequence,
                full_highlight(&row.title),
                Vec::new(),
            ),
        },
        MatchTier::EditDistance => match matched.field {
            MatchField::Title => (
                MatchQuality::EditDistance,
                matched.highlight_data,
                Vec::new(),
            ),
            MatchField::Subtitle => (
                MatchQuality::EditDistance,
                Vec::new(),
                matched.highlight_data,
            ),
            MatchField::Hidden => (
                MatchQuality::EditDistance,
                full_highlight(&row.title),
                Vec::new(),
            ),
        },
    };

    Some(MatchScore {
        quality,
        score: matched.score,
        reason: matched.reason,
        title_highlight_data,
        subtitle_highlight_data,
    })
}

fn supports_fuzzy(provider_id: SearchProviderId) -> bool {
    matches!(
        provider_id,
        SearchProviderId::Apps
            | SearchProviderId::Settings
            | SearchProviderId::Commands
            | SearchProviderId::LocalFolders
    )
}

fn intent_boost(kind: SearchResultKind, intent: QueryIntent) -> i32 {
    match kind {
        SearchResultKind::Setting if intent.setting => 2_200,
        SearchResultKind::App if intent.app => 2_000,
        SearchResultKind::Folder if intent.folder => 1_900,
        SearchResultKind::Command if intent.setting => 1_100,
        SearchResultKind::Folder => 300,
        SearchResultKind::App => 450,
        SearchResultKind::Setting => 500,
        SearchResultKind::Command => 350,
        _ => 0,
    }
}

fn open_window_match_boost(kind: SearchResultKind, quality: MatchQuality) -> i32 {
    if kind != SearchResultKind::Window {
        return 0;
    }
    match quality {
        MatchQuality::Exact => 900,
        MatchQuality::Prefix => 650,
        MatchQuality::Acronym => 500,
        MatchQuality::TokenPrefix | MatchQuality::Subsequence => 300,
        MatchQuality::EditDistance => 0,
    }
}

fn important_folder_boost(
    row: &SearchResult,
    query: &str,
    tokens: &[String],
    intent: QueryIntent,
) -> i32 {
    if row.kind != SearchResultKind::Folder || !intent.folder {
        return 0;
    }
    let path = row.path.as_deref().map(normalize).unwrap_or_default();
    let title = normalize(&row.title);
    let is_dev_root = path == "c dev" || title == "c dev" || title == "dev";
    if is_dev_root && (query == "dev" || query == "c dev" || tokens.iter().any(|t| t == "dev")) {
        let local_root_bonus = if row.provider_id == SearchProviderId::LocalFolders {
            1_200
        } else {
            0
        };
        return 1_000 + local_root_bonus;
    }
    if path.contains("jasonshell") && tokens.iter().any(|t| t == "jasonshell" || t == "repo") {
        return 900;
    }
    0
}

fn kind_base(kind: SearchResultKind) -> i32 {
    match kind {
        SearchResultKind::Setting => 700,
        SearchResultKind::App => 650,
        SearchResultKind::Command => 550,
        SearchResultKind::Folder => 500,
        SearchResultKind::Window => 450,
        SearchResultKind::File => 200,
        SearchResultKind::Calculator => 180,
        SearchResultKind::Bookmark | SearchResultKind::Web => 120,
    }
}

fn provider_base(provider_id: SearchProviderId) -> i32 {
    match provider_id {
        SearchProviderId::Settings => 200,
        SearchProviderId::Apps => 180,
        SearchProviderId::Commands => 150,
        SearchProviderId::LocalFolders => 140,
        SearchProviderId::OpenWindows => 120,
        SearchProviderId::Everything => 80,
        SearchProviderId::Calculator => 40,
        SearchProviderId::Bookmarks | SearchProviderId::Web | SearchProviderId::Diagnostics => 0,
    }
}

fn classify_intent(raw_query: &str, tokens: &[String]) -> QueryIntent {
    let raw = raw_query.trim();
    let setting = tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "settings"
                | "setting"
                | "control"
                | "panel"
                | "display"
                | "screen"
                | "monitor"
                | "sound"
                | "audio"
                | "volume"
                | "network"
                | "bluetooth"
                | "privacy"
                | "update"
                | "power"
        )
    });
    let folder = raw.contains(':')
        || raw.contains('\\')
        || raw.contains('/')
        || tokens.iter().any(|token| {
            matches!(
                token.as_str(),
                "dev" | "downloads" | "desktop" | "documents" | "docs" | "home" | "profile"
            )
        });
    QueryIntent {
        app: !setting && !folder,
        setting,
        folder,
    }
}

fn compare_results(left: &SearchResult, right: &SearchResult) -> std::cmp::Ordering {
    right
        .score
        .cmp(&left.score)
        .then(provider_order(left.provider_id).cmp(&provider_order(right.provider_id)))
        .then(left.title.cmp(&right.title))
        .then(left.record_key.cmp(&right.record_key))
}

fn provider_order(provider_id: SearchProviderId) -> usize {
    match provider_id {
        SearchProviderId::Settings => 0,
        SearchProviderId::Apps => 1,
        SearchProviderId::Commands => 2,
        SearchProviderId::LocalFolders => 3,
        SearchProviderId::OpenWindows => 4,
        SearchProviderId::Everything => 5,
        SearchProviderId::Calculator => 6,
        SearchProviderId::Web => 7,
        SearchProviderId::Bookmarks => 8,
        SearchProviderId::Diagnostics => 9,
    }
}

fn duplicate_key(row: &SearchResult) -> String {
    if let Some(path) = row.path.as_deref().filter(|path| !path.trim().is_empty()) {
        return format!("{:?}:{}", row.kind, normalize_record_key(path));
    }
    match &row.action {
        SearchResultAction::OpenSetting { uri } => format!("setting:{}", normalize_record_key(uri)),
        SearchResultAction::RunControlPanel { executable, args } => {
            format!(
                "control:{executable}:{}",
                args.clone().unwrap_or_default().join(" ")
            )
        }
        SearchResultAction::RunCommand { command_id } => format!("command:{command_id}"),
        SearchResultAction::FocusWindow { window_id } => format!("window:{window_id}"),
        _ => row.record_key.clone(),
    }
}

struct SearchFields {
    hidden_values: Vec<String>,
}

impl SearchFields {
    fn from_result(row: &SearchResult) -> Self {
        Self {
            hidden_values: {
                let mut hidden = Vec::with_capacity(2 + row.aliases.len() + row.terms.len());
                hidden.push(row.record_key.clone());
                if let Some(path) = row.path.clone() {
                    hidden.push(path);
                }
                hidden.extend(row.aliases.iter().cloned());
                hidden.extend(row.terms.iter().cloned());
                hidden
            },
        }
    }
}

fn query_tokens(query: &str) -> Vec<String> {
    match_query_tokens(query)
}

fn normalize_record_key(value: &str) -> String {
    value.trim().replace('/', r"\").to_lowercase()
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['_', '-', '.', '/', '\\', ':'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::providers::{apps, everything};
    use crate::search_sources::everything_ffi::EverythingSdkResultKind;
    use serde::Serialize;
    use std::{env, fs, path::PathBuf};

    fn row(
        id: &str,
        provider_id: SearchProviderId,
        kind: SearchResultKind,
        title: &str,
        path: Option<&str>,
    ) -> SearchResult {
        let action = match kind {
            SearchResultKind::App => SearchResultAction::OpenApp {
                path: path.unwrap_or(id).to_string(),
            },
            SearchResultKind::Folder => SearchResultAction::OpenFolder {
                path: path.unwrap_or(id).to_string(),
            },
            SearchResultKind::File => SearchResultAction::OpenFile {
                path: path.unwrap_or(id).to_string(),
            },
            SearchResultKind::Setting => SearchResultAction::OpenSetting {
                uri: path.unwrap_or("ms-settings:").to_string(),
            },
            SearchResultKind::Command => SearchResultAction::RunCommand {
                command_id: id.to_string(),
            },
            SearchResultKind::Window => SearchResultAction::FocusWindow {
                window_id: id.to_string(),
            },
            _ => SearchResultAction::CopyText {
                text: title.to_string(),
            },
        };
        SearchResult {
            id: id.to_string(),
            provider_id,
            kind,
            title: title.to_string(),
            subtitle: None,
            path: path.map(str::to_string),
            action,
            terms: vec![title.to_string()],
            aliases: Vec::new(),
            score: 99_999,
            provider_signal: 0,
            match_reason: "fixture".to_string(),
            record_key: id.to_string(),
            title_highlight_data: Vec::new(),
            subtitle_highlight_data: Vec::new(),
            icon_data_url: None,
        }
    }

    #[derive(Clone, Serialize)]
    struct ProviderSignalMetadata {
        row_id: String,
        provider_id: SearchProviderId,
        title: String,
        raw_provider_metadata: String,
        derived_signal: i32,
    }

    #[derive(Clone)]
    struct SignalRow {
        row: SearchResult,
        metadata: ProviderSignalMetadata,
    }

    fn with_signal_metadata(
        row: SearchResult,
        raw_provider_metadata: String,
        derived_signal: i32,
    ) -> SignalRow {
        SignalRow {
            metadata: ProviderSignalMetadata {
                row_id: row.id.clone(),
                provider_id: row.provider_id,
                title: row.title.clone(),
                raw_provider_metadata,
                derived_signal,
            },
            row,
        }
    }

    fn app_signal_row(title: &str, path: &str, source: &str, query: &str) -> SignalRow {
        let row = apps::test_app_result_from_source(title, path, source, query);
        let derived_signal = row.provider_signal;
        with_signal_metadata(row, format!("apps.source={source}"), derived_signal)
    }

    fn everything_signal_row(
        path: &str,
        kind: EverythingSdkResultKind,
        run_count: u32,
        query: &str,
    ) -> SignalRow {
        let row = everything::test_everything_result_from_run_count(path, kind, run_count, query);
        let derived_signal = row.provider_signal;
        with_signal_metadata(
            row,
            format!("everything.kind={kind:?};everything.run_count={run_count}"),
            derived_signal,
        )
    }

    fn fixture_signal_row(row: SearchResult, raw_provider_metadata: &str) -> SignalRow {
        with_signal_metadata(row, raw_provider_metadata.to_string(), 0)
    }

    fn rank_visible_results_with_signal_cap(
        query: &str,
        rows: Vec<SignalRow>,
        limit: usize,
        cap: i32,
    ) -> Vec<SearchResult> {
        let query_text = normalize(query);
        let tokens = query_tokens(query);
        if tokens.is_empty() {
            return Vec::new();
        }
        let intent = classify_intent(query, &tokens);
        let mut deduped: HashMap<String, SearchResult> = HashMap::new();

        for signal_row in rows {
            let mut row = signal_row.row;
            let raw_signal = signal_row.metadata.derived_signal.max(0).min(cap.max(0));
            row.provider_signal = 0;
            let Some(mut score) = score_row(&row, &query_text, &tokens, intent) else {
                continue;
            };
            score.score += raw_signal;
            row.score = score.score;
            row.match_reason = score.reason.to_string();
            row.title_highlight_data = score.title_highlight_data;
            row.subtitle_highlight_data = score.subtitle_highlight_data;
            let key = duplicate_key(&row);
            match deduped.get(&key) {
                Some(existing) if compare_results(&row, existing).is_ge() => {}
                _ => {
                    deduped.insert(key, row);
                }
            }
        }

        let mut results = deduped.into_values().collect::<Vec<_>>();
        results.sort_by(compare_results);
        results.truncate(limit);
        results
    }

    #[derive(Clone)]
    struct ExperimentCase {
        name: &'static str,
        query: &'static str,
        rows: Vec<SignalRow>,
        relevant_ids: &'static [&'static str],
        expected_top1: Option<&'static str>,
        prefix_target: Option<&'static str>,
    }

    #[derive(Serialize)]
    struct CaseRankingArtifact {
        name: String,
        query: String,
        ranked_ids: Vec<String>,
        top1: Option<String>,
        relevant_found_at: Option<usize>,
        prefix_acquired: bool,
    }

    #[derive(Serialize)]
    struct CapArtifact {
        cap: i32,
        mrr_at_10: f64,
        recall_at_10: f64,
        prefix_acquisition: f64,
        top1_stability: f64,
        nonmatches_resurrected: bool,
        dedupe_tie_unchanged: bool,
        relevant_case_count: usize,
        cases: Vec<CaseRankingArtifact>,
    }

    #[derive(Serialize)]
    struct Phase2Artifact {
        phase: &'static str,
        method: &'static str,
        candidate_caps: Vec<i32>,
        recommendation: i32,
        recommendation_reason: String,
        provider_signals: Vec<ProviderSignalMetadata>,
        caps: Vec<CapArtifact>,
    }

    fn recommend_phase2_cap(cap_artifacts: &[CapArtifact]) -> i32 {
        let baseline = cap_artifacts
            .iter()
            .find(|cap| cap.cap == 0)
            .expect("cap 0");
        let mut improving = cap_artifacts
            .iter()
            .filter(|cap| {
                cap.top1_stability == 1.0
                    && !cap.nonmatches_resurrected
                    && cap.dedupe_tie_unchanged
                    && cap.mrr_at_10 > baseline.mrr_at_10
            })
            .collect::<Vec<_>>();
        improving.sort_by(|left, right| {
            right
                .mrr_at_10
                .partial_cmp(&left.mrr_at_10)
                .expect("finite mrr")
                .then_with(|| {
                    right
                        .recall_at_10
                        .partial_cmp(&left.recall_at_10)
                        .expect("finite recall")
                })
                .then_with(|| {
                    right
                        .prefix_acquisition
                        .partial_cmp(&left.prefix_acquisition)
                        .expect("finite prefix")
                })
                .then(left.cap.cmp(&right.cap))
        });
        improving.first().map_or(0, |cap| cap.cap)
    }

    fn phase2_experiment_cases() -> Vec<ExperimentCase> {
        vec![
            ExperimentCase {
                name: "close call pinned app source priority acquisition",
                query: "alpha",
                rows: vec![
                    app_signal_row("Alpha", r"C:\Apps\AAlpha.lnk", "windowsApps", "alpha"),
                    app_signal_row(
                        "Alpha",
                        r"C:\Start Menu\ZAlpha.lnk",
                        "pinnedTaskbar",
                        "alpha",
                    ),
                ],
                relevant_ids: &["app:c:\\start menu\\zalpha.lnk"],
                expected_top1: None,
                prefix_target: Some("app:c:\\start menu\\zalpha.lnk"),
            },
            ExperimentCase {
                name: "pinned app beats lower-quality incidental row",
                query: "spotify",
                rows: vec![
                    everything_signal_row(
                        r"C:\Users\me\Music\Spotify Cache",
                        EverythingSdkResultKind::Folder,
                        20,
                        "spotify",
                    ),
                    app_signal_row(
                        "Spotify",
                        r"C:\Start Menu\Spotify.lnk",
                        "pinnedTaskbar",
                        "spotify",
                    ),
                ],
                relevant_ids: &["app:c:\\start menu\\spotify.lnk"],
                expected_top1: Some("app:c:\\start menu\\spotify.lnk"),
                prefix_target: Some("app:c:\\start menu\\spotify.lnk"),
            },
            ExperimentCase {
                name: "exact setting beats high-signal unrelated everything row",
                query: "display settings",
                rows: vec![
                    everything_signal_row(
                        r"C:\Archive\Display Settings",
                        EverythingSdkResultKind::Folder,
                        20,
                        "display settings",
                    ),
                    fixture_signal_row(
                        row(
                            "setting:display",
                            SearchProviderId::Settings,
                            SearchResultKind::Setting,
                            "Display Settings",
                            Some("ms-settings:display"),
                        ),
                        "settings.static_priority=not_provider_signal",
                    ),
                ],
                relevant_ids: &["setting:display"],
                expected_top1: Some("setting:display"),
                prefix_target: Some("setting:display"),
            },
            ExperimentCase {
                name: "exact app beats high-run-count unrelated everything app",
                query: "code",
                rows: vec![
                    everything_signal_row(
                        r"C:\Tools\CodeHelper.exe",
                        EverythingSdkResultKind::File,
                        20,
                        "code",
                    ),
                    app_signal_row(
                        "Code",
                        r"C:\Start Menu\Code.lnk",
                        "currentUserStartMenu",
                        "code",
                    ),
                ],
                relevant_ids: &["app:c:\\start menu\\code.lnk"],
                expected_top1: Some("app:c:\\start menu\\code.lnk"),
                prefix_target: Some("app:c:\\start menu\\code.lnk"),
            },
            ExperimentCase {
                name: "nonmatching high-signal row never resurrects",
                query: "spotify",
                rows: vec![everything_signal_row(
                    r"C:\Docs\Budget.xlsx",
                    EverythingSdkResultKind::File,
                    20,
                    "spotify",
                )],
                relevant_ids: &[],
                expected_top1: None,
                prefix_target: None,
            },
            ExperimentCase {
                name: "dedupe and tie order stable",
                query: "dev",
                rows: vec![
                    everything_signal_row(r"C:\dev", EverythingSdkResultKind::Folder, 20, "dev"),
                    fixture_signal_row(
                        row(
                            "local:folder:c-dev",
                            SearchProviderId::LocalFolders,
                            SearchResultKind::Folder,
                            "C:\\dev",
                            Some(r"C:\dev"),
                        ),
                        "local.static_priority=not_provider_signal",
                    ),
                ],
                relevant_ids: &["local:folder:c-dev"],
                expected_top1: Some("local:folder:c-dev"),
                prefix_target: Some("local:folder:c-dev"),
            },
        ]
    }

    fn evaluate_phase2_caps(caps: &[i32]) -> Phase2Artifact {
        let cases = phase2_experiment_cases();
        let provider_signals = cases
            .iter()
            .flat_map(|case| case.rows.iter().map(|row| row.metadata.clone()))
            .collect::<Vec<_>>();
        let relevant_case_count = cases
            .iter()
            .filter(|case| !case.relevant_ids.is_empty())
            .count();
        let baseline_top1 = cases
            .iter()
            .map(|case| {
                (
                    case.name,
                    rank_visible_results_with_signal_cap(case.query, case.rows.clone(), 10, 0)
                        .first()
                        .map(|row| row.id.clone()),
                )
            })
            .collect::<HashMap<_, _>>();
        let baseline_dedupe_len = rank_visible_results_with_signal_cap(
            "dev",
            cases
                .iter()
                .find(|case| case.name == "dedupe and tie order stable")
                .expect("dedupe case")
                .rows
                .clone(),
            10,
            0,
        )
        .len();

        let cap_artifacts = caps
            .iter()
            .map(|cap| {
                let mut reciprocal_sum = 0.0;
                let mut recall_sum = 0.0;
                let mut top1_ok = 0usize;
                let mut top1_expected_count = 0usize;
                let mut prefix_hits = 0usize;
                let mut prefix_count = 0usize;
                let mut nonmatches_resurrected = false;
                let mut case_artifacts = Vec::new();

                for case in &cases {
                    let ranked = rank_visible_results_with_signal_cap(
                        case.query,
                        case.rows.clone(),
                        10,
                        *cap,
                    );
                    let ranked_ids = ranked.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
                    let relevant_found_at = ranked_ids.iter().position(|id| {
                        case.relevant_ids
                            .iter()
                            .any(|relevant| relevant == &id.as_str())
                    });
                    if !case.relevant_ids.is_empty() {
                        if let Some(index) = relevant_found_at {
                            reciprocal_sum += 1.0 / ((index + 1) as f64);
                            recall_sum += 1.0;
                        }
                    }
                    if case.relevant_ids.is_empty() && !ranked.is_empty() {
                        nonmatches_resurrected = true;
                    }
                    if let Some(expected) = case.expected_top1 {
                        top1_expected_count += 1;
                        if ranked_ids.first().map(String::as_str) == Some(expected)
                            && baseline_top1
                                .get(case.name)
                                .and_then(Clone::clone)
                                .as_deref()
                                == Some(expected)
                        {
                            top1_ok += 1;
                        }
                    }
                    let prefix_acquired = case.prefix_target.map_or(false, |target| {
                        ranked_ids.first().map(String::as_str) == Some(target)
                    });
                    if case.prefix_target.is_some() {
                        prefix_count += 1;
                        if prefix_acquired {
                            prefix_hits += 1;
                        }
                    }
                    case_artifacts.push(CaseRankingArtifact {
                        name: case.name.to_string(),
                        query: case.query.to_string(),
                        ranked_ids,
                        top1: ranked.first().map(|row| row.id.clone()),
                        relevant_found_at: relevant_found_at.map(|index| index + 1),
                        prefix_acquired,
                    });
                }

                let dedupe_len = rank_visible_results_with_signal_cap(
                    "dev",
                    cases
                        .iter()
                        .find(|case| case.name == "dedupe and tie order stable")
                        .expect("dedupe case")
                        .rows
                        .clone(),
                    10,
                    *cap,
                )
                .len();

                CapArtifact {
                    cap: *cap,
                    mrr_at_10: reciprocal_sum / relevant_case_count as f64,
                    recall_at_10: recall_sum / relevant_case_count as f64,
                    prefix_acquisition: if prefix_count == 0 {
                        0.0
                    } else {
                        prefix_hits as f64 / prefix_count as f64
                    },
                    top1_stability: if top1_expected_count == 0 {
                        1.0
                    } else {
                        top1_ok as f64 / top1_expected_count as f64
                    },
                    nonmatches_resurrected,
                    dedupe_tie_unchanged: dedupe_len == baseline_dedupe_len,
                    relevant_case_count,
                    cases: case_artifacts,
                }
            })
            .collect::<Vec<_>>();

        let recommendation = recommend_phase2_cap(&cap_artifacts);

        Phase2Artifact {
            phase: "Phase 2 provider-signal experiment only",
            method: "candidate cap evaluation plus approved production cap50 provider_signal bonus",
            candidate_caps: caps.to_vec(),
            recommendation,
            recommendation_reason: if recommendation == 0 {
                "No candidate improved MRR@10 over cap0 while preserving 100% top1 stability, nonmatch filtering, and dedupe invariants; recommend cap0/no behavior change.".to_string()
            } else {
                format!("MRR-qualified safe cap selected by MRR, then Recall, then prefix acquisition, then smallest cap tie-break: {recommendation}")
            },
            provider_signals,
            caps: cap_artifacts,
        }
    }

    fn maybe_write_phase2_artifact(artifact: &Phase2Artifact) {
        let Some(outdir) = env::var_os("JASONSHELL_PHASE2_OUTDIR") else {
            return;
        };
        let outdir = PathBuf::from(outdir);
        fs::create_dir_all(&outdir).expect("create phase2 artifact dir");
        let json = serde_json::to_string_pretty(artifact).expect("serialize phase2 artifact");
        fs::write(outdir.join("provider-signal-experiment.json"), json)
            .expect("write phase2 artifact");
    }

    fn cap_fixture(cap: i32, mrr: f64, recall: f64, prefix: f64) -> CapArtifact {
        CapArtifact {
            cap,
            mrr_at_10: mrr,
            recall_at_10: recall,
            prefix_acquisition: prefix,
            top1_stability: 1.0,
            nonmatches_resurrected: false,
            dedupe_tie_unchanged: true,
            relevant_case_count: 4,
            cases: Vec::new(),
        }
    }

    #[test]
    fn phase2_cap_recommendation_requires_mrr_gain_before_tie_breaks() {
        let caps = vec![
            cap_fixture(0, 0.75, 0.75, 0.50),
            cap_fixture(50, 0.75, 1.00, 1.00),
            cap_fixture(100, 0.80, 0.75, 0.50),
            cap_fixture(150, 0.80, 1.00, 0.50),
            cap_fixture(200, 0.80, 1.00, 1.00),
        ];

        assert_eq!(recommend_phase2_cap(&caps), 200);
    }

    #[test]
    fn phase2_provider_signal_experiment_preserves_invariants_and_recommends_cap() {
        let caps = [0, 50, 100, 150, 200, 300];
        let artifact = evaluate_phase2_caps(&caps);

        assert_eq!(artifact.recommendation, 50);
        assert_eq!(artifact.caps[0].relevant_case_count, 5);
        assert_eq!(artifact.caps[0].mrr_at_10, 0.9);
        assert_eq!(artifact.caps[0].recall_at_10, 1.0);
        let close_case_rank_at_cap0 = artifact.caps[0]
            .cases
            .iter()
            .find(|case| case.name == "close call pinned app source priority acquisition")
            .and_then(|case| case.relevant_found_at)
            .expect("close call cap0 rank");
        let recommended = artifact
            .caps
            .iter()
            .find(|cap| cap.cap == artifact.recommendation)
            .expect("recommended cap");
        let close_case_rank_at_recommended = recommended
            .cases
            .iter()
            .find(|case| case.name == "close call pinned app source priority acquisition")
            .and_then(|case| case.relevant_found_at)
            .expect("close call recommended rank");
        assert!(close_case_rank_at_cap0 > 1);
        assert_eq!(close_case_rank_at_recommended, 1);
        assert!(recommended.mrr_at_10 > artifact.caps[0].mrr_at_10);
        assert!(artifact
            .caps
            .iter()
            .filter(|cap| cap.cap >= artifact.recommendation)
            .all(|cap| cap.top1_stability == 1.0));
        assert!(artifact.caps.iter().all(|cap| !cap.nonmatches_resurrected));
        assert!(artifact.caps.iter().all(|cap| cap.dedupe_tie_unchanged));
        assert!(artifact.provider_signals.iter().any(|signal| {
            signal.raw_provider_metadata == "apps.source=pinnedTaskbar" && signal.derived_signal > 0
        }));
        assert!(artifact.provider_signals.iter().any(|signal| {
            signal.raw_provider_metadata == "settings.static_priority=not_provider_signal"
                && signal.derived_signal == 0
        }));
        assert!(artifact.provider_signals.iter().any(|signal| {
            signal.raw_provider_metadata == "everything.kind=Folder;everything.run_count=20"
                && signal.derived_signal == 50
        }));
        maybe_write_phase2_artifact(&artifact);
    }

    fn row_with_provider_signal(
        id: &str,
        provider_id: SearchProviderId,
        kind: SearchResultKind,
        title: &str,
        path: Option<&str>,
        provider_signal: i32,
    ) -> SearchResult {
        let mut result = row(id, provider_id, kind, title, path);
        result.provider_signal = provider_signal;
        result
    }

    #[test]
    fn phase2_provider_signal_cap50_acquires_source_priority_close_call() {
        let rows = vec![
            row_with_provider_signal(
                "app:c-apps-aalpha",
                SearchProviderId::Apps,
                SearchResultKind::App,
                "Alpha",
                Some(r"C:\Apps\AAlpha.lnk"),
                0,
            ),
            row_with_provider_signal(
                "app:c-start-menu-zalpha",
                SearchProviderId::Apps,
                SearchResultKind::App,
                "Alpha",
                Some(r"C:\Start Menu\ZAlpha.lnk"),
                50,
            ),
        ];

        let ranked = rank_visible_results("alpha", rows, 10);

        assert_eq!(ranked[0].id, "app:c-start-menu-zalpha");
    }

    #[test]
    fn phase2_provider_signal_is_clamped_and_never_resurrects_nonmatches() {
        let ranked = rank_visible_results(
            "alpha",
            vec![
                row_with_provider_signal(
                    "app:negative",
                    SearchProviderId::Apps,
                    SearchResultKind::App,
                    "Alpha",
                    Some(r"C:\Apps\Negative.lnk"),
                    -10,
                ),
                row_with_provider_signal(
                    "app:huge",
                    SearchProviderId::Apps,
                    SearchResultKind::App,
                    "Alpha",
                    Some(r"C:\Apps\Huge.lnk"),
                    999,
                ),
            ],
            10,
        );

        assert_eq!(ranked[0].id, "app:huge");
        assert_eq!(ranked[0].score - ranked[1].score, 50);
        assert!(rank_visible_results(
            "alpha",
            vec![row_with_provider_signal(
                "file:budget",
                SearchProviderId::Everything,
                SearchResultKind::File,
                "Budget.xlsx",
                Some(r"C:\Docs\Budget.xlsx"),
                999,
            )],
            10,
        )
        .is_empty());
    }

    #[test]
    fn provider_type_boosts_do_not_surface_non_matches() {
        let rows = vec![row(
            "setting:sound",
            SearchProviderId::Settings,
            SearchResultKind::Setting,
            "Sound Settings",
            Some("ms-settings:sound"),
        )];

        assert!(rank_visible_results("spotify", rows, 10).is_empty());
    }

    #[test]
    fn settings_intent_wins_settings_query() {
        let rows = vec![
            row(
                "file:display",
                SearchProviderId::Everything,
                SearchResultKind::Folder,
                "display settings",
                Some(r"C:\docs\display settings"),
            ),
            row(
                "setting:display",
                SearchProviderId::Settings,
                SearchResultKind::Setting,
                "Display Settings",
                Some("ms-settings:display"),
            ),
        ];

        let ranked = rank_visible_results("display settings", rows, 10);

        assert_eq!(ranked[0].provider_id, SearchProviderId::Settings);
        assert_eq!(ranked[0].id, "setting:display");
    }

    #[test]
    fn windows_settings_intent_beats_incidental_everything_rows() {
        let rows = vec![
            row(
                "everything:file:windows-settings-notes",
                SearchProviderId::Everything,
                SearchResultKind::File,
                "windows settings notes.txt",
                Some(r"C:\docs\windows settings notes.txt"),
            ),
            row(
                "setting:windows-settings",
                SearchProviderId::Settings,
                SearchResultKind::Setting,
                "Windows Settings",
                Some("ms-settings:"),
            ),
        ];

        let ranked = rank_visible_results("windows settings", rows, 10);

        assert_eq!(ranked[0].provider_id, SearchProviderId::Settings);
        assert_eq!(ranked[0].id, "setting:windows-settings");
    }

    #[test]
    fn control_panel_intent_beats_incidental_everything_rows() {
        let rows = vec![
            row(
                "everything:folder:control-panel-docs",
                SearchProviderId::Everything,
                SearchResultKind::Folder,
                "Control Panel",
                Some(r"C:\docs\Control Panel"),
            ),
            SearchResult {
                id: "setting:control-panel".to_string(),
                provider_id: SearchProviderId::Settings,
                kind: SearchResultKind::Setting,
                title: "Control Panel".to_string(),
                subtitle: Some("Open classic Control Panel".to_string()),
                path: Some("control.exe".to_string()),
                action: SearchResultAction::RunControlPanel {
                    executable: "control.exe".to_string(),
                    args: None,
                },
                terms: vec!["control".to_string(), "panel".to_string()],
                aliases: vec!["control panel".to_string()],
                score: 0,
                provider_signal: 0,
                match_reason: "fixture".to_string(),
                record_key: "setting:control-panel".to_string(),
                title_highlight_data: Vec::new(),
                subtitle_highlight_data: Vec::new(),
                icon_data_url: None,
            },
        ];

        let ranked = rank_visible_results("control panel", rows, 10);

        assert_eq!(ranked[0].provider_id, SearchProviderId::Settings);
        assert_eq!(ranked[0].id, "setting:control-panel");
    }

    #[test]
    fn app_intent_wins_app_query() {
        let rows = vec![
            row(
                "folder:spotify",
                SearchProviderId::Everything,
                SearchResultKind::Folder,
                "Spotify",
                Some(r"C:\Users\me\Spotify"),
            ),
            row(
                "app:spotify",
                SearchProviderId::Apps,
                SearchResultKind::App,
                "Spotify",
                Some(r"C:\Start Menu\Spotify.lnk"),
            ),
        ];

        let ranked = rank_visible_results("spotify", rows, 10);

        assert_eq!(ranked[0].kind, SearchResultKind::App);
        assert_eq!(ranked[0].provider_id, SearchProviderId::Apps);
    }

    #[test]
    fn folder_intent_wins_path_query() {
        let rows = vec![
            row(
                "file:dev-notes",
                SearchProviderId::Everything,
                SearchResultKind::File,
                "dev notes.txt",
                Some(r"C:\notes\dev notes.txt"),
            ),
            row(
                "folder:dev-root",
                SearchProviderId::LocalFolders,
                SearchResultKind::Folder,
                "C:\\dev",
                Some(r"C:\dev"),
            ),
        ];

        for query in [r"C:\dev", "C://dev", "c dev", "dev"] {
            let ranked = rank_visible_results(query, rows.clone(), 10);
            assert_eq!(ranked[0].id, "folder:dev-root", "{query}");
            assert_eq!(ranked[0].kind, SearchResultKind::Folder);
        }
    }

    #[test]
    fn duplicate_paths_collapse_once_with_best_provider_row() {
        let rows = vec![
            row(
                "everything:folder:c-dev",
                SearchProviderId::Everything,
                SearchResultKind::Folder,
                "dev",
                Some(r"C:\dev"),
            ),
            row(
                "local:folder:c-dev",
                SearchProviderId::LocalFolders,
                SearchResultKind::Folder,
                "C:\\dev",
                Some(r"C:\dev"),
            ),
        ];

        let ranked = rank_visible_results("dev", rows, 10);

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].provider_id, SearchProviderId::LocalFolders);
    }

    #[test]
    fn exact_open_window_hit_outranks_incidental_everything_folder() {
        let rows = vec![
            row(
                "everything:folder:terminal",
                SearchProviderId::Everything,
                SearchResultKind::Folder,
                "Terminal",
                Some(r"C:\docs\Terminal"),
            ),
            row(
                "window:terminal",
                SearchProviderId::OpenWindows,
                SearchResultKind::Window,
                "Terminal",
                None,
            ),
        ];

        let ranked = rank_visible_results("terminal", rows, 10);

        assert_eq!(ranked[0].provider_id, SearchProviderId::OpenWindows);
        assert_eq!(ranked[0].kind, SearchResultKind::Window);
    }

    #[test]
    fn fuzzy_apps_and_settings_match_abbreviations_without_helping_everything() {
        let rows = vec![
            row(
                "everything:file:spotify",
                SearchProviderId::Everything,
                SearchResultKind::File,
                "Spotify backup.zip",
                Some(r"C:\Temp\Spotify backup.zip"),
            ),
            row(
                "app:spotify",
                SearchProviderId::Apps,
                SearchResultKind::App,
                "Spotify",
                Some(r"C:\Start Menu\Spotify.lnk"),
            ),
        ];

        let ranked = rank_visible_results("sptfy", rows, 10);

        assert_eq!(ranked[0].id, "app:spotify");
        assert_eq!(ranked[0].match_reason, "subsequence");
        assert_eq!(ranked[0].title_highlight_data, vec![0, 2, 3, 1, 5, 2]);

        let everything_only = vec![row(
            "everything:file:spotify",
            SearchProviderId::Everything,
            SearchResultKind::File,
            "Spotify backup.zip",
            Some(r"C:\Temp\Spotify backup.zip"),
        )];

        assert!(rank_visible_results("sptfy", everything_only, 10).is_empty());
    }

    #[test]
    fn token_prefix_fuzzy_keeps_exact_setting_above_random_file() {
        let rows = vec![
            row(
                "everything:file:display-set",
                SearchProviderId::Everything,
                SearchResultKind::File,
                "display setup notes.txt",
                Some(r"C:\docs\display setup notes.txt"),
            ),
            row(
                "setting:display",
                SearchProviderId::Settings,
                SearchResultKind::Setting,
                "Display Settings",
                Some("ms-settings:display"),
            ),
        ];

        let ranked = rank_visible_results("disp set", rows, 10);

        assert_eq!(ranked[0].id, "setting:display");
        assert_eq!(ranked[0].match_reason, "tokenPrefix");
        assert_eq!(ranked[0].title_highlight_data, vec![0, 4, 8, 3]);
    }

    #[test]
    fn alias_only_app_match_gets_visible_highlight_fallback() {
        let mut app = row(
            "app:spotify",
            SearchProviderId::Apps,
            SearchResultKind::App,
            "Spotify",
            Some(r"C:\Start Menu\Spotify.lnk"),
        );
        app.aliases = vec!["music player".to_string()];

        let ranked = rank_visible_results("music player", vec![app], 10);

        assert_eq!(ranked[0].id, "app:spotify");
        assert!(
            !ranked[0].title_highlight_data.is_empty()
                || !ranked[0].subtitle_highlight_data.is_empty()
        );
    }
}
