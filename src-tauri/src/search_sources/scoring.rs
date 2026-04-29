use super::SystemSearchResult;
use std::path::Path;

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

pub fn score_result(result: &SystemSearchResult, tokens: &[String]) -> Option<i32> {
    if tokens.is_empty() {
        return None;
    }

    let title = normalize(&result.title);
    let haystack = normalize(&format!(
        "{} {} {} {}",
        result.title, result.subtitle, result.terms, result.path
    ));
    if !tokens.iter().all(|token| haystack.contains(token)) {
        return None;
    }

    let mut score = result
        .priority
        .saturating_add(20)
        .saturating_add(result_type_priority(&result.kind))
        .saturating_add(provider_priority(result));
    for token in tokens {
        if title == *token {
            score = score.saturating_add(120);
        } else if title.starts_with(token) {
            score = score.saturating_add(56);
        } else if title.contains(token) {
            score = score.saturating_add(28);
        } else {
            score = score.saturating_add(8);
        }
    }

    Some(score)
}

fn provider_priority(result: &SystemSearchResult) -> i32 {
    if result.terms.contains("everything") || result.terms.contains("voidtools") {
        60
    } else if result.terms.contains("windows search") || result.terms.contains("systemindex") {
        -20
    } else {
        0
    }
}

fn result_type_priority(kind: &str) -> i32 {
    match kind {
        "app" => 35,
        "window" => 30,
        "folder" => 26,
        "file" => 20,
        "command" | "setting" => 16,
        "calculator" => 14,
        "web" | "bookmark" => 8,
        _ => 0,
    }
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
}
