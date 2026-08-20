use crate::search::contracts::{
    iso_now, SearchOpenWindowContext, SearchProviderCacheState, SearchProviderHealth,
    SearchProviderHealthState, SearchProviderId, SearchProviderTiming, SearchResult,
    SearchResultAction, SearchResultKind,
};
use crate::search::icons::icon_data_url_for_path;
#[cfg(test)]
use crate::search::test_observer::{record, SearchOperation};
use std::path::Path;
use std::time::Instant;

#[derive(Clone, Debug)]
pub(crate) struct OpenWindowsSearchRun {
    pub(crate) results: Vec<SearchResult>,
    pub(crate) timing: SearchProviderTiming,
    pub(crate) health: SearchProviderHealth,
}

pub(crate) fn search_open_windows(
    query: &str,
    limit: usize,
    windows: &[SearchOpenWindowContext],
) -> OpenWindowsSearchRun {
    #[cfg(test)]
    record(SearchOperation::OpenWindows);
    let started_at = iso_now();
    let started = Instant::now();
    let results = rank_open_windows(query, windows, limit);
    let result_count = results.len();

    OpenWindowsSearchRun {
        results,
        timing: SearchProviderTiming {
            provider_id: SearchProviderId::OpenWindows,
            started_at,
            ended_at: Some(iso_now()),
            duration_ms: started.elapsed().as_secs_f64() * 1000.0,
            cache: SearchProviderCacheState::Hit,
            cache_age_ms: None,
            result_count,
            applied: true,
            discarded_as_stale: false,
        },
        health: SearchProviderHealth {
            provider_id: SearchProviderId::OpenWindows,
            state: SearchProviderHealthState::Ready,
            reason_code: None,
            message: Some(format!(
                "{} open window context rows supplied",
                windows.len()
            )),
        },
    }
}

fn rank_open_windows(
    query: &str,
    windows: &[SearchOpenWindowContext],
    limit: usize,
) -> Vec<SearchResult> {
    let tokens = query_tokens(query);
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut results = windows
        .iter()
        .filter_map(|window| {
            score_window(window, &tokens)
                .map(|(score, reason)| window_result(window, score, reason))
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.title.cmp(&right.title))
            .then(left.record_key.cmp(&right.record_key))
    });
    results.truncate(limit);
    results
}

fn window_result(
    window: &SearchOpenWindowContext,
    score: i32,
    reason: &'static str,
) -> SearchResult {
    let app_name = window.app_name.clone().unwrap_or_default();
    let executable_path = window.executable_path.clone().unwrap_or_default();
    let icon_data_url = window.icon_data_url.clone().or_else(|| {
        window
            .executable_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .and_then(|path| icon_data_url_for_path(Path::new(path)))
    });
    SearchResult {
        id: format!("window:{}", window.id),
        provider_id: SearchProviderId::OpenWindows,
        kind: SearchResultKind::Window,
        title: window.title.clone(),
        subtitle: Some(if app_name.is_empty() {
            "Open window".to_string()
        } else {
            format!("Open window - {app_name}")
        }),
        path: if executable_path.is_empty() {
            None
        } else {
            Some(executable_path.clone())
        },
        action: SearchResultAction::FocusWindow {
            window_id: window.id.clone(),
        },
        terms: token_terms(&format!("{} {app_name} {executable_path}", window.title)),
        aliases: if app_name.is_empty() {
            Vec::new()
        } else {
            vec![app_name]
        },
        score,
        provider_signal: 0,
        match_reason: reason.to_string(),
        record_key: format!("window:{}", window.id),
        title_highlight_data: Vec::new(),
        subtitle_highlight_data: Vec::new(),
        icon_data_url,
    }
}

fn score_window(
    window: &SearchOpenWindowContext,
    tokens: &[String],
) -> Option<(i32, &'static str)> {
    let query = tokens.join(" ");
    let title = normalize(&window.title);
    let app_name = window
        .app_name
        .as_deref()
        .map(normalize)
        .unwrap_or_default();
    let executable_path = window
        .executable_path
        .as_deref()
        .map(normalize)
        .unwrap_or_default();

    if title == query || app_name == query {
        return Some((1_600, "exactWindow"));
    }
    if title.starts_with(&query) || app_name.starts_with(&query) {
        return Some((1_350, "prefixWindow"));
    }
    let searchable = format!("{title} {app_name} {executable_path}");
    if tokens.iter().all(|token| searchable.contains(token)) {
        return Some((920, "tokenWindow"));
    }
    None
}

fn query_tokens(query: &str) -> Vec<String> {
    normalize(query)
        .split(' ')
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn token_terms(value: &str) -> Vec<String> {
    normalize(value)
        .split(' ')
        .filter(|token| !token.is_empty())
        .take(16)
        .map(str::to_string)
        .collect()
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

    #[test]
    fn matches_open_window_title_and_app_name() {
        let windows = vec![SearchOpenWindowContext {
            id: "123".to_string(),
            title: "JasonShell - Visual Studio Code".to_string(),
            app_name: Some("Code".to_string()),
            executable_path: Some(r"C:\Users\me\AppData\Local\Programs\Code.exe".to_string()),
            icon_data_url: None,
        }];

        let results = rank_open_windows("jasonshell", &windows, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, SearchResultKind::Window);
        assert_eq!(
            results[0].action,
            SearchResultAction::FocusWindow {
                window_id: "123".to_string()
            }
        );

        assert_eq!(rank_open_windows("code", &windows, 10).len(), 1);
    }
}
