use crate::search::contracts::{
    SearchProviderHealth, SearchProviderHealthState, SearchProviderId, SearchResult,
    SearchResultAction, SearchResultKind,
};
use crate::search::matcher::{
    best_match, full_highlight, query_tokens as match_query_tokens, MatchData, MatchField,
};
#[cfg(test)]
use crate::search::test_observer::{record, SearchOperation};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SettingsProviderRow {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub path: &'static str,
    pub category: &'static str,
    pub priority: i32,
    pub keywords: &'static [&'static str],
    pub aliases: &'static [&'static str],
    pub control_panel_args: &'static [&'static str],
    pub windows_min_build: Option<u32>,
    pub windows_max_build: Option<u32>,
    pub icon_glyph: Option<&'static str>,
}

pub(crate) const SETTINGS_PROVIDER_ID: SearchProviderId = SearchProviderId::Settings;

const SETTINGS_ROWS: &[SettingsProviderRow] = &[
    SettingsProviderRow {
        id: "setting:windows-settings",
        title: "Windows Settings",
        subtitle: "Open Windows Settings",
        path: "ms-settings:",
        category: "system",
        priority: 930,
        keywords: &["settings", "windows settings", "system settings"],
        aliases: &["settings app", "pc settings", "modern settings"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("settings"),
    },
    SettingsProviderRow {
        id: "setting:display",
        title: "Display Settings",
        subtitle: "Open Windows display settings",
        path: "ms-settings:display",
        category: "system",
        priority: 980,
        keywords: &["display", "screen", "monitor", "resolution", "brightness"],
        aliases: &["display settings", "screen settings", "monitor settings"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("monitor"),
    },
    SettingsProviderRow {
        id: "setting:sound",
        title: "Sound Settings",
        subtitle: "Open Windows sound settings",
        path: "ms-settings:sound",
        category: "system",
        priority: 980,
        keywords: &["sound", "audio", "volume", "speaker", "microphone"],
        aliases: &["sound settings", "audio settings", "volume settings"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("volume-2"),
    },
    SettingsProviderRow {
        id: "setting:control-panel",
        title: "Control Panel",
        subtitle: "Open classic Control Panel",
        path: "control.exe",
        category: "classic",
        priority: 960,
        keywords: &["control", "panel", "classic settings"],
        aliases: &["control panel", "classic control panel", "classic settings"],
        control_panel_args: &[],
        windows_min_build: None,
        windows_max_build: None,
        icon_glyph: Some("panel-top"),
    },
    SettingsProviderRow {
        id: "setting:control-panel-sound",
        title: "Sound Control Panel",
        subtitle: "Open classic sound control panel",
        path: "control.exe",
        category: "classic",
        priority: 920,
        keywords: &["sound", "control", "panel", "speaker", "playback"],
        aliases: &["sound control panel", "classic sound", "mmsys"],
        control_panel_args: &["mmsys.cpl"],
        windows_min_build: None,
        windows_max_build: None,
        icon_glyph: Some("speaker"),
    },
    SettingsProviderRow {
        id: "setting:control-panel-programs",
        title: "Programs and Features",
        subtitle: "Open classic programs and features",
        path: "control.exe",
        category: "classic",
        priority: 920,
        keywords: &[
            "programs",
            "features",
            "uninstall",
            "applications",
            "appwiz",
        ],
        aliases: &[
            "programs and features",
            "uninstall programs",
            "classic programs",
        ],
        control_panel_args: &["appwiz.cpl"],
        windows_min_build: None,
        windows_max_build: None,
        icon_glyph: Some("list"),
    },
    SettingsProviderRow {
        id: "setting:network",
        title: "Network Settings",
        subtitle: "Open Windows network settings",
        path: "ms-settings:network",
        category: "network",
        priority: 900,
        keywords: &["network", "internet", "wifi", "ethernet"],
        aliases: &["network settings", "internet settings", "wifi settings"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("network"),
    },
    SettingsProviderRow {
        id: "setting:bluetooth",
        title: "Bluetooth Settings",
        subtitle: "Open Bluetooth and devices settings",
        path: "ms-settings:bluetooth",
        category: "devices",
        priority: 900,
        keywords: &["bluetooth", "devices", "pairing"],
        aliases: &["bluetooth settings", "device settings"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("bluetooth"),
    },
    SettingsProviderRow {
        id: "setting:apps",
        title: "Installed Apps",
        subtitle: "Open installed apps and features settings",
        path: "ms-settings:appsfeatures",
        category: "apps",
        priority: 900,
        keywords: &[
            "apps",
            "installed apps",
            "add or remove programs",
            "programs",
            "features",
        ],
        aliases: &[
            "apps settings",
            "app settings",
            "programs settings",
            "add or remove programs",
        ],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("package"),
    },
    SettingsProviderRow {
        id: "setting:night-light",
        title: "Night light",
        subtitle: "Open night light settings",
        path: "ms-settings:nightlight",
        category: "system",
        priority: 915,
        keywords: &["night light", "night mode", "blue light"],
        aliases: &["night light settings", "blue light settings"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("moon"),
    },
    SettingsProviderRow {
        id: "setting:default-apps",
        title: "Default apps",
        subtitle: "Open default apps settings",
        path: "ms-settings:defaultapps",
        category: "apps",
        priority: 915,
        keywords: &["default apps", "file associations", "default programs"],
        aliases: &["default apps settings", "default programs"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("app-window"),
    },
    SettingsProviderRow {
        id: "setting:startup-apps",
        title: "Startup apps",
        subtitle: "Open startup apps settings",
        path: "ms-settings:startupapps",
        category: "apps",
        priority: 915,
        keywords: &["startup apps", "startup", "startup programs"],
        aliases: &["startup apps settings", "startup programs"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("rocket"),
    },
    SettingsProviderRow {
        id: "setting:optional-features",
        title: "Optional features",
        subtitle: "Open optional features settings",
        path: "ms-settings:optionalfeatures",
        category: "apps",
        priority: 915,
        keywords: &["optional features", "windows features", "features"],
        aliases: &["optional features settings", "windows features"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("plus-square"),
    },
    SettingsProviderRow {
        id: "setting:privacy",
        title: "Privacy Settings",
        subtitle: "Open Windows privacy settings",
        path: "ms-settings:privacy",
        category: "privacy",
        priority: 900,
        keywords: &["privacy", "permissions", "security"],
        aliases: &["privacy settings", "permission settings"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("shield"),
    },
    SettingsProviderRow {
        id: "setting:windows-update",
        title: "Windows Update",
        subtitle: "Open Windows Update settings",
        path: "ms-settings:windowsupdate",
        category: "system",
        priority: 910,
        keywords: &["update", "windows update", "patches"],
        aliases: &["update settings", "windows update settings"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("refresh-cw"),
    },
    SettingsProviderRow {
        id: "setting:power-sleep",
        title: "Power and Sleep",
        subtitle: "Open power and sleep settings",
        path: "ms-settings:powersleep",
        category: "system",
        priority: 900,
        keywords: &["power", "sleep", "battery", "energy"],
        aliases: &["power settings", "sleep settings", "power and sleep"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("battery"),
    },
    SettingsProviderRow {
        id: "setting:taskbar",
        title: "Taskbar Settings",
        subtitle: "Open Windows taskbar settings",
        path: "ms-settings:taskbar",
        category: "personalization",
        priority: 925,
        keywords: &["taskbar", "start", "pins", "tray"],
        aliases: &["taskbar settings", "start bar settings"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("layout"),
    },
    SettingsProviderRow {
        id: "setting:storage",
        title: "Storage Settings",
        subtitle: "Open storage settings",
        path: "ms-settings:storagesense",
        category: "system",
        priority: 920,
        keywords: &["storage", "disk", "space", "cleanup"],
        aliases: &["storage settings", "disk settings", "storage sense"],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("hard-drive"),
    },
    SettingsProviderRow {
        id: "setting:windows-security",
        title: "Windows Security",
        subtitle: "Open Windows Security settings",
        path: "ms-settings:windowsdefender",
        category: "security",
        priority: 930,
        keywords: &["security", "defender", "virus", "firewall"],
        aliases: &["windows security", "defender settings", "security settings"],
        control_panel_args: &[],
        windows_min_build: Some(16_299),
        windows_max_build: None,
        icon_glyph: Some("shield-check"),
    },
    SettingsProviderRow {
        id: "setting:personalization",
        title: "Personalization",
        subtitle: "Open personalization settings",
        path: "ms-settings:personalization",
        category: "personalization",
        priority: 920,
        keywords: &["personalization", "theme", "background", "colors"],
        aliases: &[
            "personalization settings",
            "theme settings",
            "background settings",
        ],
        control_panel_args: &[],
        windows_min_build: Some(10_240),
        windows_max_build: None,
        icon_glyph: Some("palette"),
    },
];

pub(crate) fn settings_rows() -> &'static [SettingsProviderRow] {
    SETTINGS_ROWS
}

pub(crate) fn settings_provider_health() -> SearchProviderHealth {
    SearchProviderHealth {
        provider_id: SETTINGS_PROVIDER_ID,
        state: if SETTINGS_ROWS.is_empty() {
            SearchProviderHealthState::Unavailable
        } else {
            SearchProviderHealthState::Ready
        },
        reason_code: None,
        message: Some("settings provider uses bundled Windows settings dataset".to_string()),
    }
}

pub(crate) fn search_settings(query: &str, limit: usize) -> Vec<SearchResult> {
    #[cfg(test)]
    record(SearchOperation::Settings);
    search_settings_for_build(query, limit, None)
}

fn search_settings_for_build(
    query: &str,
    limit: usize,
    windows_build: Option<u32>,
) -> Vec<SearchResult> {
    if is_path_like_settings_query(query) {
        return Vec::new();
    }
    let tokens = query_tokens(query);
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut results = SETTINGS_ROWS
        .iter()
        .filter(|row| row_supported_on_build(row, windows_build))
        .filter(|row| short_settings_tokens_are_allowed_for_row(row, &tokens))
        .filter_map(|row| {
            score_row(row, &tokens).map(|(score, matched)| {
                row_to_result(row, score + phase1_prefix_boost(row, &tokens), matched)
            })
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.title.cmp(&right.title))
            .then(left.id.cmp(&right.id))
    });
    results.truncate(limit.min(short_prefix_settings_limit(&tokens)));
    results
}

fn is_path_like_settings_query(query: &str) -> bool {
    query.contains(['*', '/', '\\', ':'])
}

fn short_settings_tokens_are_allowed_for_row(row: &SettingsProviderRow, tokens: &[String]) -> bool {
    if tokens.iter().all(|token| token.len() >= 2) {
        return true;
    }
    if tokens.len() == 1 && tokens[0].len() == 1 {
        return is_canonical_windows_root_prefix(row, &tokens[0]);
    }
    row_has_ordered_static_token_prefix(row, tokens)
}

fn short_prefix_settings_limit(tokens: &[String]) -> usize {
    if tokens.iter().any(|token| {
        token.len() <= 2 || token == "windows" || token == "control" || token == "display"
    }) {
        3
    } else {
        usize::MAX
    }
}

fn phase1_prefix_boost(row: &SettingsProviderRow, tokens: &[String]) -> i32 {
    if row.path == "ms-settings:" && row_has_ordered_static_token_prefix(row, tokens) {
        350
    } else {
        0
    }
}

fn is_canonical_windows_root_prefix(row: &SettingsProviderRow, token: &str) -> bool {
    row.path == "ms-settings:"
        && static_row_phrases(row).iter().any(|phrase| {
            phrase == "windows settings"
                && phrase_token_prefix_matches(phrase, &[token.to_string()])
        })
}

fn row_has_ordered_static_token_prefix(row: &SettingsProviderRow, tokens: &[String]) -> bool {
    static_row_phrases(row)
        .iter()
        .any(|phrase| phrase_token_prefix_matches(phrase, tokens))
}

fn static_row_phrases(row: &SettingsProviderRow) -> Vec<String> {
    let mut phrases = Vec::with_capacity(2 + row.keywords.len() + row.aliases.len());
    phrases.push(row.title.to_string());
    phrases.extend(row.keywords.iter().map(|value| (*value).to_string()));
    phrases.extend(row.aliases.iter().map(|value| (*value).to_string()));
    phrases
}

fn phrase_token_prefix_matches(phrase: &str, tokens: &[String]) -> bool {
    let phrase_tokens = query_tokens(phrase);
    if tokens.is_empty() || tokens.len() > phrase_tokens.len() {
        return false;
    }
    tokens
        .iter()
        .zip(phrase_tokens.iter())
        .all(|(token, phrase_token)| phrase_token.starts_with(token))
}

fn row_to_result(row: &SettingsProviderRow, score: i32, matched: MatchData) -> SearchResult {
    SearchResult {
        id: row.id.to_string(),
        provider_id: SETTINGS_PROVIDER_ID,
        kind: SearchResultKind::Setting,
        title: row.title.to_string(),
        subtitle: Some(row.subtitle.to_string()),
        path: Some(row.path.to_string()),
        action: action_for_row(row),
        terms: row
            .keywords
            .iter()
            .map(|term| (*term).to_string())
            .collect(),
        aliases: row
            .aliases
            .iter()
            .map(|alias| (*alias).to_string())
            .collect(),
        score,
        provider_signal: 0,
        match_reason: matched.reason.to_string(),
        record_key: row.id.to_string(),
        title_highlight_data: match matched.field {
            MatchField::Title => matched.highlight_data.clone(),
            MatchField::Hidden => full_highlight(row.title),
            MatchField::Subtitle => Vec::new(),
        },
        subtitle_highlight_data: if matched.field == MatchField::Subtitle {
            matched.highlight_data
        } else {
            Vec::new()
        },
        icon_data_url: None,
    }
}

fn action_for_row(row: &SettingsProviderRow) -> SearchResultAction {
    if row.path.starts_with("ms-settings:") {
        SearchResultAction::OpenSetting {
            uri: row.path.to_string(),
        }
    } else {
        SearchResultAction::RunControlPanel {
            executable: "control.exe".to_string(),
            args: (!row.control_panel_args.is_empty()).then(|| {
                row.control_panel_args
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect()
            }),
        }
    }
}

fn score_row(row: &SettingsProviderRow, tokens: &[String]) -> Option<(i32, MatchData)> {
    let query = tokens.join(" ");
    let mut hidden = Vec::with_capacity(8 + row.keywords.len() + row.aliases.len());
    hidden.push(row.id.to_string());
    hidden.push(row.path.to_string());
    hidden.push(row.category.to_string());
    if let Some(glyph) = row.icon_glyph {
        hidden.push(glyph.to_string());
    }
    if let Some(min_build) = row.windows_min_build {
        hidden.push(min_build.to_string());
    }
    if let Some(max_build) = row.windows_max_build {
        hidden.push(max_build.to_string());
    }
    hidden.extend(
        row.control_panel_args
            .iter()
            .map(|value| (*value).to_string()),
    );
    hidden.extend(row.keywords.iter().map(|value| (*value).to_string()));
    hidden.extend(row.aliases.iter().map(|value| (*value).to_string()));
    let matched = best_match(row.title, Some(row.subtitle), &hidden, &query, tokens, true)?;
    Some((row.priority + matched.score, matched))
}

fn row_supported_on_build(row: &SettingsProviderRow, windows_build: Option<u32>) -> bool {
    let Some(build) = windows_build else {
        return true;
    };
    if let Some(min_build) = row.windows_min_build {
        if build < min_build {
            return false;
        }
    }
    if let Some(max_build) = row.windows_max_build {
        if build > max_build {
            return false;
        }
    }
    true
}

fn query_tokens(query: &str) -> Vec<String> {
    match_query_tokens(query)
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

    fn first_id(query: &str) -> Option<String> {
        search_settings(query, 5)
            .first()
            .map(|result| result.id.clone())
    }

    fn result_ids(query: &str, limit: usize) -> Vec<String> {
        search_settings(query, limit)
            .into_iter()
            .map(|result| result.id)
            .collect()
    }

    #[test]
    fn dataset_contains_required_windows_settings_rows() {
        let paths = settings_rows()
            .iter()
            .map(|row| row.path)
            .collect::<Vec<_>>();

        assert!(paths.contains(&"ms-settings:"));
        assert!(paths.contains(&"ms-settings:display"));
        assert!(paths.contains(&"ms-settings:sound"));
        assert!(paths.contains(&"control.exe"));
        assert!(paths.contains(&"ms-settings:network"));
        assert!(paths.contains(&"ms-settings:bluetooth"));
        assert!(paths.contains(&"ms-settings:appsfeatures"));
        assert!(paths.contains(&"ms-settings:privacy"));
        assert!(paths.contains(&"ms-settings:windowsupdate"));
        assert!(paths.contains(&"ms-settings:powersleep"));
        assert!(paths.contains(&"ms-settings:taskbar"));
        assert!(paths.contains(&"ms-settings:storagesense"));
        assert!(paths.contains(&"ms-settings:windowsdefender"));
        assert!(paths.contains(&"ms-settings:personalization"));
    }

    #[test]
    fn settings_catalog_schema_and_action_safety_are_valid() {
        for row in settings_rows() {
            assert!(!row.id.trim().is_empty());
            assert!(!row.title.trim().is_empty());
            assert!(!row.subtitle.trim().is_empty());
            assert!(!row.path.trim().is_empty());
            assert!(!row.category.trim().is_empty());
            assert!(!row.keywords.is_empty());
            assert!(row
                .keywords
                .iter()
                .all(|keyword| !keyword.trim().is_empty()));
            assert!(row.aliases.iter().all(|alias| !alias.trim().is_empty()));
            if let (Some(min), Some(max)) = (row.windows_min_build, row.windows_max_build) {
                assert!(min <= max, "{} has valid build range", row.id);
            }
            if row.path.starts_with("ms-settings:") {
                assert!(
                    crate::search::contracts::is_safe_ms_settings_uri(row.path),
                    "{} has safe uri",
                    row.id
                );
                assert!(row.control_panel_args.is_empty());
            } else {
                assert_eq!(row.path, "control.exe");
                assert!(!row
                    .control_panel_args
                    .iter()
                    .any(|arg| arg.trim().is_empty()));
            }
            let action = super::action_for_row(row);
            assert!(action.is_safe(), "{} action is safe", row.id);
        }
    }

    #[test]
    fn display_settings_matches_display_screen_and_monitor_aliases() {
        assert_eq!(
            first_id("display settings").as_deref(),
            Some("setting:display")
        );
        assert_eq!(
            first_id("screen settings").as_deref(),
            Some("setting:display")
        );
        assert_eq!(
            first_id("monitor settings").as_deref(),
            Some("setting:display")
        );
    }

    #[test]
    fn sound_settings_matches_sound_audio_and_volume_aliases() {
        assert_eq!(first_id("sound settings").as_deref(), Some("setting:sound"));
        assert_eq!(first_id("audio settings").as_deref(), Some("setting:sound"));
        assert_eq!(
            first_id("volume settings").as_deref(),
            Some("setting:sound")
        );
    }

    #[test]
    fn control_panel_matches_classic_control_intents() {
        let result = search_settings("control panel", 1)
            .pop()
            .expect("control panel result");

        assert_eq!(result.id, "setting:control-panel");
        assert_eq!(
            result.action,
            SearchResultAction::RunControlPanel {
                executable: "control.exe".to_string(),
                args: None
            }
        );
    }

    #[test]
    fn windows_settings_matches_settings_app_intents() {
        assert_eq!(
            first_id("windows settings").as_deref(),
            Some("setting:windows-settings")
        );
        assert_eq!(
            first_id("settings app").as_deref(),
            Some("setting:windows-settings")
        );
    }

    #[test]
    fn settings_actions_use_shell_uri_for_ms_settings() {
        let result = search_settings("display settings", 1)
            .pop()
            .expect("display settings result");

        assert_eq!(
            result.action,
            SearchResultAction::OpenSetting {
                uri: "ms-settings:display".to_string()
            }
        );
    }

    #[test]
    fn all_settings_actions_are_rust_side_safe() {
        for query in [
            "display settings",
            "sound settings",
            "windows settings",
            "control panel",
            "sound control panel",
            "programs and features",
            "network settings",
            "bluetooth settings",
            "apps settings",
            "privacy settings",
            "update settings",
            "power settings",
            "taskbar settings",
            "storage settings",
            "windows security",
            "personalization settings",
            "installed apps",
            "add or remove programs",
            "night light",
            "default apps",
            "startup apps",
            "optional features",
        ] {
            let result = search_settings(query, 1)
                .pop()
                .unwrap_or_else(|| panic!("{query} returns a settings result"));
            assert!(result.action.is_safe(), "{query} action is safe");
        }
    }

    #[test]
    fn c_dev_query_does_not_emit_fake_settings_result() {
        assert!(search_settings(r"C:\dev", 5).is_empty());
        assert!(search_settings("c dev", 5).is_empty());
    }

    #[test]
    fn settings_provider_reports_ready_health() {
        let health = settings_provider_health();

        assert_eq!(health.provider_id, SearchProviderId::Settings);
        assert_eq!(health.state, SearchProviderHealthState::Ready);
    }

    #[test]
    fn display_settings_matches_disp_set_abbreviation() {
        let result = search_settings("disp set", 1)
            .pop()
            .expect("display settings");

        assert_eq!(result.id, "setting:display");
        assert_eq!(result.match_reason, "tokenPrefix");
    }

    #[test]
    fn alias_only_setting_match_gets_visible_highlight_fallback() {
        let result = search_settings("screen settings", 1)
            .pop()
            .expect("screen settings result");

        assert_eq!(result.id, "setting:display");
        assert!(
            !result.title_highlight_data.is_empty() || !result.subtitle_highlight_data.is_empty()
        );
    }

    #[test]
    fn build_guard_excludes_rows_above_current_build() {
        let results = super::search_settings_for_build("windows security", 5, Some(12_000));

        assert!(results
            .iter()
            .all(|result| result.id != "setting:windows-security"));
    }

    #[test]
    fn expanded_catalog_includes_common_windows_settings_intents() {
        assert_eq!(
            first_id("taskbar settings").as_deref(),
            Some("setting:taskbar")
        );
        assert_eq!(
            first_id("storage settings").as_deref(),
            Some("setting:storage")
        );
        assert_eq!(
            first_id("windows security").as_deref(),
            Some("setting:windows-security")
        );
        assert_eq!(
            first_id("personalization settings").as_deref(),
            Some("setting:personalization")
        );
        assert_eq!(first_id("night light").as_deref(), Some("setting:night-light"));
        assert_eq!(first_id("default apps").as_deref(), Some("setting:default-apps"));
        assert_eq!(first_id("startup apps").as_deref(), Some("setting:startup-apps"));
        assert_eq!(
            first_id("optional features").as_deref(),
            Some("setting:optional-features")
        );
    }

    #[test]
    fn installed_apps_row_covers_add_or_remove_programs_and_appsfeatures_uri() {
        assert_eq!(first_id("installed apps").as_deref(), Some("setting:apps"));
        assert_eq!(first_id("appsfeatures").as_deref(), Some("setting:apps"));
        assert_eq!(
            first_id("add or remove programs").as_deref(),
            Some("setting:apps")
        );
        let result = search_settings("add or remove programs", 1)
            .pop()
            .expect("installed apps result");
        assert_eq!(
            result.action,
            SearchResultAction::OpenSetting {
                uri: "ms-settings:appsfeatures".to_string()
            }
        );
    }

    #[test]
    fn new_catalog_rows_rank_intended_settings_top_for_exact_and_alias_queries() {
        for (query, expected) in [
            ("installed apps", "setting:apps"),
            ("add or remove programs", "setting:apps"),
            ("night light", "setting:night-light"),
            ("default apps", "setting:default-apps"),
            ("startup apps", "setting:startup-apps"),
            ("optional features", "setting:optional-features"),
        ] {
            assert_eq!(first_id(query).as_deref(), Some(expected), "{query}");
        }
    }

    #[test]
    fn phase1_short_prefix_corpus_returns_expected_top_settings_without_flooding() {
        for (query, expected_top_id, max_settings_rows) in [
            ("w", "setting:windows-settings", 3usize),
            ("wi", "setting:windows-settings", 3usize),
            ("windows s", "setting:windows-settings", 3usize),
            ("control p", "setting:control-panel", 3usize),
            ("display s", "setting:display", 1usize),
        ] {
            let ids = result_ids(query, 10);
            assert_eq!(
                ids.first().map(String::as_str),
                Some(expected_top_id),
                "{query} top result"
            );
            assert!(
                ids.len() <= max_settings_rows,
                "{query} returns at most {max_settings_rows} settings rows, got {ids:?}"
            );
        }
    }

    #[test]
    fn phase1_short_prefix_corpus_rejects_broad_or_pathlike_settings_noise() {
        for query in ["", "   ", "a", "*", r"c:\", "src/"] {
            assert!(
                search_settings(query, 10).is_empty(),
                "{query:?} returns zero settings rows"
            );
        }
    }

    #[test]
    fn phase1_short_token_policy_is_static_row_ordered_prefix_not_exact_query_allowlist() {
        let network = search_settings("network s", 3)
            .pop()
            .expect("network settings short phrase");
        assert_eq!(network.id, "setting:network");

        let sound_control = search_settings("sound c", 3)
            .into_iter()
            .find(|result| result.id == "setting:control-panel-sound")
            .expect("sound control panel short phrase");
        assert_eq!(sound_control.path.as_deref(), Some("control.exe"));

        assert!(search_settings("network a", 10).is_empty());
        assert!(search_settings("control x", 10).is_empty());
    }

    #[test]
    fn phase1_one_character_policy_is_limited_to_canonical_windows_root_intent() {
        assert_eq!(first_id("w").as_deref(), Some("setting:windows-settings"));
        for query in ["a", "p", "s", "c"] {
            assert!(
                search_settings(query, 10).is_empty(),
                "{query} remains conservative"
            );
        }
    }

    #[test]
    fn control_panel_subtasks_use_control_exe_with_safe_applet_args() {
        let sound = search_settings("sound control panel", 1)
            .pop()
            .expect("sound control panel");
        assert_eq!(sound.id, "setting:control-panel-sound");
        assert_eq!(
            sound.action,
            SearchResultAction::RunControlPanel {
                executable: "control.exe".to_string(),
                args: Some(vec!["mmsys.cpl".to_string()]),
            }
        );

        let programs = search_settings("programs and features", 1)
            .pop()
            .expect("programs and features");
        assert_eq!(programs.id, "setting:control-panel-programs");
        assert_eq!(
            programs.action,
            SearchResultAction::RunControlPanel {
                executable: "control.exe".to_string(),
                args: Some(vec!["appwiz.cpl".to_string()]),
            }
        );
    }
}
