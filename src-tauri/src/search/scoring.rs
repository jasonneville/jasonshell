use crate::search::contracts::{
    SearchProviderId, SearchResult, SearchResultAction, SearchResultKind,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchQuality {
    Exact,
    Prefix,
    Acronym,
    Token,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MatchScore {
    quality: MatchQuality,
    score: i32,
    reason: &'static str,
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
    let mut score = match_quality_score(&fields, query, tokens)?;

    score.score += kind_base(row.kind);
    score.score += provider_base(row.provider_id);
    score.score += intent_boost(row.kind, intent);
    score.score += open_window_match_boost(row.kind, score.quality);
    score.score += important_folder_boost(row, query, tokens, intent);

    Some(score)
}

fn match_quality_score(
    fields: &SearchFields,
    query: &str,
    tokens: &[String],
) -> Option<MatchScore> {
    if fields.exact_values.iter().any(|value| value == query) {
        return Some(MatchScore {
            quality: MatchQuality::Exact,
            score: 2_000,
            reason: "exact",
        });
    }
    if fields
        .prefix_values
        .iter()
        .any(|value| value.starts_with(query))
    {
        return Some(MatchScore {
            quality: MatchQuality::Prefix,
            score: 1_650,
            reason: "prefix",
        });
    }
    if !fields.acronym.is_empty() && fields.acronym == query {
        return Some(MatchScore {
            quality: MatchQuality::Acronym,
            score: 1_500,
            reason: "acronym",
        });
    }
    if tokens.iter().all(|token| fields.searchable.contains(token)) {
        return Some(MatchScore {
            quality: MatchQuality::Token,
            score: 900,
            reason: "token",
        });
    }
    None
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
        MatchQuality::Token => 300,
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
    exact_values: Vec<String>,
    prefix_values: Vec<String>,
    searchable: String,
    acronym: String,
}

impl SearchFields {
    fn from_result(row: &SearchResult) -> Self {
        let title = normalize(&row.title);
        let aliases = row.aliases.iter().map(|value| normalize(value));
        let terms = row.terms.iter().map(|value| normalize(value));
        let path = row.path.as_deref().map(normalize).unwrap_or_default();
        let record_key = normalize(&row.record_key);
        let mut exact_values = Vec::with_capacity(4 + row.aliases.len() + row.terms.len());
        exact_values.push(title.clone());
        exact_values.push(path.clone());
        exact_values.push(record_key.clone());
        exact_values.extend(aliases.clone());
        exact_values.extend(terms.clone());

        let mut prefix_values = Vec::with_capacity(exact_values.len());
        prefix_values.extend(exact_values.iter().cloned());

        let searchable = exact_values
            .iter()
            .filter(|value| !value.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");

        Self {
            exact_values,
            prefix_values,
            searchable,
            acronym: acronym(&title),
        }
    }
}

fn query_tokens(query: &str) -> Vec<String> {
    normalize(query)
        .split(' ')
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn acronym(value: &str) -> String {
    value
        .split_whitespace()
        .filter_map(|token| token.chars().next())
        .collect()
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
            match_reason: "fixture".to_string(),
            record_key: id.to_string(),
            icon_data_url: None,
        }
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
}
