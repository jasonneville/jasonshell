use super::SystemSearchResult;
use std::path::Path;

#[allow(dead_code)]
pub fn query_tokens(query: &str) -> Vec<String> {
    normalize(query)
        .split(' ')
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
pub fn score_path(path: &Path, tokens: &[String], base_priority: i32) -> Option<i32> {
    if tokens.is_empty() {
        return None;
    }

    let title = normalize(&display_name(path));
    let haystack = normalize(&format!("{} {}", title, path.display()));
    if !tokens.iter().all(|token| haystack.contains(token)) {
        return None;
    }

    let mut score = base_priority + 20;
    for token in tokens {
        if title == *token {
            score += 80;
        } else if title.starts_with(token) {
            score += 46;
        } else if title.contains(token) {
            score += 24;
        } else {
            score += 8;
        }
    }

    Some(score)
}

#[allow(dead_code)]
pub fn search_ranked_results(
    entries: &[SystemSearchResult],
    query: &str,
    limit: usize,
) -> Vec<SystemSearchResult> {
    let tokens = query_tokens(query);
    let mut results = entries
        .iter()
        .filter_map(|entry| {
            let priority = score_result(entry, &tokens)?;
            let mut result = entry.clone();
            result.priority = priority;
            Some(result)
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then(left.title.cmp(&right.title))
    });
    results.truncate(limit);
    results
}

#[allow(dead_code)]
pub fn score_result(result: &SystemSearchResult, tokens: &[String]) -> Option<i32> {
    if tokens.is_empty() {
        return None;
    }

    let title = normalize(&result.title);
    let haystack = searchable_text(result);
    if !tokens
        .iter()
        .all(|token| haystack.contains(token) || fuzzy_token_match(&title, token))
    {
        return None;
    }

    let mut score = result
        .priority
        .saturating_add(20)
        .saturating_add(result_type_priority(&result.kind))
        .saturating_add(provider_priority(result))
        .saturating_add(intent_priority(result, tokens));
    let query_text = tokens.join(" ");
    for token in tokens {
        if title == *token {
            score = score.saturating_add(120);
        } else if title.starts_with(token) {
            score = score.saturating_add(56);
        } else if title.contains(token) {
            score = score.saturating_add(28);
        } else if fuzzy_token_match(&title, token) {
            score = score.saturating_add(16);
        } else {
            score = score.saturating_add(8);
        }
    }
    let is_fuzzy_app =
        result.kind == "app" && tokens.iter().any(|token| fuzzy_token_match(&title, token));
    if is_launch_intent_kind(&result.kind)
        && (title == query_text
            || (tokens.len() > 1 && haystack.contains(&query_text))
            || is_fuzzy_app)
    {
        score = score.saturating_add(2_000);
    }

    Some(score)
}

fn searchable_text(result: &SystemSearchResult) -> String {
    let mut text = normalize(&format!(
        "{} {} {} {} {}",
        result.id, result.title, result.subtitle, result.terms, result.path
    ));
    if is_system_control_result(result) {
        text.push_str(" windows settings system settings control panel control pane settings app");
    }
    text
}

#[allow(dead_code)]
fn provider_priority(result: &SystemSearchResult) -> i32 {
    if result.terms.contains("everything") || result.terms.contains("voidtools") {
        60
    } else if result.terms.contains("windows search") || result.terms.contains("systemindex") {
        -20
    } else {
        0
    }
}

#[allow(dead_code)]
fn result_type_priority(kind: &str) -> i32 {
    match kind {
        "app" => 180,
        "window" => 30,
        "folder" => 26,
        "file" => 20,
        "command" | "setting" => 150,
        "calculator" => 14,
        "web" | "bookmark" => 8,
        _ => 0,
    }
}

fn is_launch_intent_kind(kind: &str) -> bool {
    matches!(kind, "app" | "setting" | "command")
}

fn fuzzy_token_match(value: &str, token: &str) -> bool {
    if token.len() < 3 || token.len() > value.len() {
        return false;
    }
    let mut chars = token.chars();
    let Some(mut expected) = chars.next() else {
        return false;
    };
    for actual in value.chars() {
        if actual == expected {
            let Some(next) = chars.next() else {
                return true;
            };
            expected = next;
        }
    }
    false
}

fn intent_priority(result: &SystemSearchResult, tokens: &[String]) -> i32 {
    if is_system_control_result(result) && is_system_control_query(tokens) {
        return 2_200;
    }
    0
}

fn is_system_control_query(tokens: &[String]) -> bool {
    let query_text = tokens.join(" ");
    query_text == "settings"
        || query_text == "windows settings"
        || query_text == "system settings"
        || query_text == "control panel"
        || query_text == "control pane"
        || tokens.iter().any(|token| token == "settings")
        || (tokens.iter().any(|token| token == "control")
            && tokens
                .iter()
                .any(|token| token == "panel" || token == "pane"))
}

fn is_system_control_result(result: &SystemSearchResult) -> bool {
    if !is_launch_intent_kind(&result.kind) {
        return false;
    }
    let text = normalize(&format!(
        "{} {} {} {}",
        result.id, result.title, result.subtitle, result.terms
    ));
    text.contains("settings")
        || text.contains("control panel")
        || text.contains("control plane")
        || text.contains("system settings")
}

pub fn display_name(path: &Path) -> String {
    let stem = path.file_stem().or_else(|| path.file_name());
    stem.map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

pub fn should_skip_dir(path: &Path) -> bool {
    let Some(name) = path
        .file_name()
        .map(|value| normalize(&value.to_string_lossy()))
    else {
        return false;
    };

    matches!(
        name.as_str(),
        "$recycle.bin"
            | ".git"
            | ".svn"
            | "appdata"
            | "cache"
            | "debug"
            | "dist"
            | "node_modules"
            | "release"
            | "target"
            | "temp"
            | "tmp"
    )
}

pub fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['_', '-', '.'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn scores_spotify_shortcut_as_exact_match() {
        let path = PathBuf::from(
            r"C:\Users\me\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Spotify.lnk",
        );
        let tokens = query_tokens("spotify");

        assert!(score_path(&path, &tokens, 100).is_some());
        assert_eq!(display_name(&path), "Spotify");
    }

    #[test]
    fn ranks_cached_results_without_touching_filesystem() {
        let result = SystemSearchResult {
            id: "system:file:C:\\Users\\me\\Documents\\Quarterly Plan.docx".to_string(),
            provider_id: Some("warmedCache".to_string()),
            kind: "file".to_string(),
            title: "Quarterly Plan".to_string(),
            subtitle: "File - Documents".to_string(),
            terms: "quarterly plan document".to_string(),
            priority: 76,
            path: "C:\\Users\\me\\Documents\\Quarterly Plan.docx".to_string(),
            record_key: Some("file:c:\\users\\me\\documents\\quarterly plan.docx".to_string()),
            run_count: None,
            top_most: None,
        };

        let results = search_ranked_results(&[result], "quarter plan", 8);

        assert_eq!(results.len(), 1);
        assert!(results[0].priority > 76);
    }

    #[test]
    fn everything_provider_and_type_boosts_are_saturating() {
        let result = SystemSearchResult {
            id: "system:file:C:\\Docs\\Plan.txt".to_string(),
            provider_id: Some("everything".to_string()),
            kind: "file".to_string(),
            title: "Plan".to_string(),
            subtitle: "File".to_string(),
            terms: "plan everything voidtools".to_string(),
            priority: i32::MAX - 10,
            path: "C:\\Docs\\Plan.txt".to_string(),
            record_key: Some("file:c:\\docs\\plan.txt".to_string()),
            run_count: Some(10),
            top_most: None,
        };
        let tokens = query_tokens("plan");

        assert_eq!(score_result(&result, &tokens), Some(i32::MAX));
    }

    #[test]
    fn exact_app_intent_outranks_high_priority_folder_match() {
        let folder = SystemSearchResult {
            id: "system:folder:C:\\Docs\\Spotify".to_string(),
            provider_id: Some("everything".to_string()),
            kind: "folder".to_string(),
            title: "Spotify".to_string(),
            subtitle: "Folder".to_string(),
            terms: "spotify folder everything".to_string(),
            priority: 999,
            path: "C:\\Docs\\Spotify".to_string(),
            record_key: Some("folder:c:\\docs\\spotify".to_string()),
            run_count: None,
            top_most: None,
        };
        let app = SystemSearchResult {
            id: "system:app:C:\\Apps\\Spotify.exe".to_string(),
            provider_id: Some("apps".to_string()),
            kind: "app".to_string(),
            title: "Spotify".to_string(),
            subtitle: "Installed app".to_string(),
            terms: "spotify application launch".to_string(),
            priority: 100,
            path: "C:\\Apps\\Spotify.exe".to_string(),
            record_key: Some("app:c:\\apps\\spotify.exe".to_string()),
            run_count: None,
            top_most: None,
        };
        let results = search_ranked_results(&[folder, app], "spotify", 2);

        assert_eq!(
            results.first().map(|result| result.kind.as_str()),
            Some("app")
        );
    }

    #[test]
    fn fuzzy_app_token_matches_launcher_intent() {
        let app = SystemSearchResult {
            id: "system:app:C:\\Apps\\Spotify.exe".to_string(),
            provider_id: Some("apps".to_string()),
            kind: "app".to_string(),
            title: "Spotify".to_string(),
            subtitle: "Installed app".to_string(),
            terms: "spotify application launch".to_string(),
            priority: 100,
            path: "C:\\Apps\\Spotify.exe".to_string(),
            record_key: Some("app:c:\\apps\\spotify.exe".to_string()),
            run_count: None,
            top_most: None,
        };
        let results = search_ranked_results(&[app], "sptfy", 2);

        assert_eq!(
            results.first().map(|result| result.id.as_str()),
            Some("system:app:C:\\Apps\\Spotify.exe")
        );
    }

    #[test]
    fn control_panel_query_matches_control_plane_command_alias() {
        let file = SystemSearchResult {
            id: "system:file:C:\\Docs\\Control Panel Notes.txt".to_string(),
            provider_id: Some("everything".to_string()),
            kind: "file".to_string(),
            title: "Control Panel Notes".to_string(),
            subtitle: "File".to_string(),
            terms: "control panel notes everything".to_string(),
            priority: 999,
            path: "C:\\Docs\\Control Panel Notes.txt".to_string(),
            record_key: Some("file:c:\\docs\\control panel notes.txt".to_string()),
            run_count: Some(100),
            top_most: None,
        };
        let command = SystemSearchResult {
            id: "command:open-control-plane".to_string(),
            provider_id: Some("commands".to_string()),
            kind: "command".to_string(),
            title: "Open developer dashboard".to_string(),
            subtitle: "Open settings and developer dashboard".to_string(),
            terms: "developer dashboard settings control plane providers diagnostics".to_string(),
            priority: 92,
            path: String::new(),
            record_key: Some("command:open-control-plane".to_string()),
            run_count: None,
            top_most: None,
        };
        let results = search_ranked_results(&[file, command], "control panel", 2);

        assert_eq!(
            results.first().map(|result| result.id.as_str()),
            Some("command:open-control-plane")
        );
    }

    #[test]
    fn bare_settings_query_outranks_exact_incidental_folder() {
        let folder = SystemSearchResult {
            id: "system:folder:C:\\Docs\\Settings".to_string(),
            provider_id: Some("everything".to_string()),
            kind: "folder".to_string(),
            title: "Settings".to_string(),
            subtitle: "Folder".to_string(),
            terms: "settings folder everything".to_string(),
            priority: 999,
            path: "C:\\Docs\\Settings".to_string(),
            record_key: Some("folder:c:\\docs\\settings".to_string()),
            run_count: Some(100),
            top_most: None,
        };
        let setting = SystemSearchResult {
            id: "setting:windows-settings".to_string(),
            provider_id: Some("commands".to_string()),
            kind: "setting".to_string(),
            title: "Windows Settings".to_string(),
            subtitle: "Open Windows Settings".to_string(),
            terms: "windows settings system settings control panel".to_string(),
            priority: 118,
            path: "ms-settings:".to_string(),
            record_key: Some("setting:windows-settings".to_string()),
            run_count: None,
            top_most: None,
        };
        let results = search_ranked_results(&[folder, setting], "settings", 2);

        assert_eq!(
            results.first().map(|result| result.id.as_str()),
            Some("setting:windows-settings")
        );
    }
}
