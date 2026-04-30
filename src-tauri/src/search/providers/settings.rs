use crate::search::contracts::{
    SearchProviderHealth, SearchProviderHealthState, SearchProviderId, SearchResult,
    SearchResultAction, SearchResultKind,
};
use crate::search::matcher::{
    best_match, query_tokens as match_query_tokens, MatchData, MatchField,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SettingsProviderRow {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub path: &'static str,
    pub category: &'static str,
    pub priority: i32,
    pub terms: &'static [&'static str],
    pub aliases: &'static [&'static str],
    pub control_panel_applet: Option<&'static str>,
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
        terms: &["settings", "windows settings", "system settings"],
        aliases: &["settings app", "pc settings", "modern settings"],
        control_panel_applet: None,
    },
    SettingsProviderRow {
        id: "setting:display",
        title: "Display Settings",
        subtitle: "Open Windows display settings",
        path: "ms-settings:display",
        category: "system",
        priority: 980,
        terms: &["display", "screen", "monitor", "resolution", "brightness"],
        aliases: &["display settings", "screen settings", "monitor settings"],
        control_panel_applet: None,
    },
    SettingsProviderRow {
        id: "setting:sound",
        title: "Sound Settings",
        subtitle: "Open Windows sound settings",
        path: "ms-settings:sound",
        category: "system",
        priority: 980,
        terms: &["sound", "audio", "volume", "speaker", "microphone"],
        aliases: &["sound settings", "audio settings", "volume settings"],
        control_panel_applet: None,
    },
    SettingsProviderRow {
        id: "setting:control-panel",
        title: "Control Panel",
        subtitle: "Open classic Control Panel",
        path: "control.exe",
        category: "classic",
        priority: 960,
        terms: &["control", "panel", "classic settings"],
        aliases: &["control panel", "classic control panel", "classic settings"],
        control_panel_applet: Some("control.exe"),
    },
    SettingsProviderRow {
        id: "setting:network",
        title: "Network Settings",
        subtitle: "Open Windows network settings",
        path: "ms-settings:network",
        category: "network",
        priority: 900,
        terms: &["network", "internet", "wifi", "ethernet"],
        aliases: &["network settings", "internet settings", "wifi settings"],
        control_panel_applet: None,
    },
    SettingsProviderRow {
        id: "setting:bluetooth",
        title: "Bluetooth Settings",
        subtitle: "Open Bluetooth and devices settings",
        path: "ms-settings:bluetooth",
        category: "devices",
        priority: 900,
        terms: &["bluetooth", "devices", "pairing"],
        aliases: &["bluetooth settings", "device settings"],
        control_panel_applet: None,
    },
    SettingsProviderRow {
        id: "setting:apps",
        title: "Apps Settings",
        subtitle: "Open installed apps settings",
        path: "ms-settings:appsfeatures",
        category: "apps",
        priority: 900,
        terms: &["apps", "installed apps", "programs", "features"],
        aliases: &["apps settings", "app settings", "programs settings"],
        control_panel_applet: None,
    },
    SettingsProviderRow {
        id: "setting:privacy",
        title: "Privacy Settings",
        subtitle: "Open Windows privacy settings",
        path: "ms-settings:privacy",
        category: "privacy",
        priority: 900,
        terms: &["privacy", "permissions", "security"],
        aliases: &["privacy settings", "permission settings"],
        control_panel_applet: None,
    },
    SettingsProviderRow {
        id: "setting:windows-update",
        title: "Windows Update",
        subtitle: "Open Windows Update settings",
        path: "ms-settings:windowsupdate",
        category: "system",
        priority: 910,
        terms: &["update", "windows update", "patches"],
        aliases: &["update settings", "windows update settings"],
        control_panel_applet: None,
    },
    SettingsProviderRow {
        id: "setting:power-sleep",
        title: "Power and Sleep",
        subtitle: "Open power and sleep settings",
        path: "ms-settings:powersleep",
        category: "system",
        priority: 900,
        terms: &["power", "sleep", "battery", "energy"],
        aliases: &["power settings", "sleep settings", "power and sleep"],
        control_panel_applet: None,
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
    let tokens = query_tokens(query);
    if tokens.is_empty() || tokens.iter().any(|token| token.len() < 2) {
        return Vec::new();
    }

    let mut results = SETTINGS_ROWS
        .iter()
        .filter_map(|row| {
            score_row(row, &tokens).map(|(score, matched)| row_to_result(row, score, matched))
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.title.cmp(&right.title))
            .then(left.id.cmp(&right.id))
    });
    results.truncate(limit);
    results
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
        terms: row.terms.iter().map(|term| (*term).to_string()).collect(),
        aliases: row
            .aliases
            .iter()
            .map(|alias| (*alias).to_string())
            .collect(),
        score,
        match_reason: matched.reason.to_string(),
        record_key: row.id.to_string(),
        title_highlight_data: if matched.field == MatchField::Title {
            matched.highlight_data.clone()
        } else {
            Vec::new()
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
            args: None,
        }
    }
}

fn score_row(row: &SettingsProviderRow, tokens: &[String]) -> Option<(i32, MatchData)> {
    let query = tokens.join(" ");
    let mut hidden = Vec::with_capacity(5 + row.terms.len() + row.aliases.len());
    hidden.push(row.id.to_string());
    hidden.push(row.path.to_string());
    hidden.push(row.category.to_string());
    if let Some(applet) = row.control_panel_applet {
        hidden.push(applet.to_string());
    }
    hidden.extend(row.terms.iter().map(|value| (*value).to_string()));
    hidden.extend(row.aliases.iter().map(|value| (*value).to_string()));
    let matched = best_match(row.title, Some(row.subtitle), &hidden, &query, tokens, true)?;
    Some((row.priority + matched.score, matched))
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
            "network settings",
            "bluetooth settings",
            "apps settings",
            "privacy settings",
            "update settings",
            "power settings",
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
}
