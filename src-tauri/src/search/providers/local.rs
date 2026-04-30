use crate::search::contracts::{
    SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState, SearchProviderId,
    SearchProviderTiming, SearchQueryContext, SearchResult, SearchResultAction, SearchResultKind,
};
use crate::search::icons::icon_data_url_for_path;
use crate::search::matcher::{best_match, query_tokens as match_query_tokens};
use std::env;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Debug)]
struct LocalRow {
    id: String,
    title: String,
    subtitle: String,
    path: Option<PathBuf>,
    kind: SearchResultKind,
    provider_id: SearchProviderId,
    action: SearchResultAction,
    aliases: Vec<String>,
    priority: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalSearchRun {
    pub(crate) results: Vec<SearchResult>,
    pub(crate) timing: SearchProviderTiming,
    pub(crate) health: SearchProviderHealth,
}

pub(crate) fn search_local(
    query: &str,
    limit: usize,
    context: &SearchQueryContext,
) -> LocalSearchRun {
    let started_at = crate::search::contracts::iso_now();
    let started = Instant::now();
    let rows = local_rows(context);
    let results = rank_local_rows(&rows, query, limit);
    let result_count = results.len();

    LocalSearchRun {
        results,
        timing: SearchProviderTiming {
            provider_id: SearchProviderId::LocalFolders,
            started_at,
            ended_at: Some(crate::search::contracts::iso_now()),
            duration_ms: started.elapsed().as_secs_f64() * 1000.0,
            cache: SearchProviderCacheState::Hit,
            cache_age_ms: None,
            result_count,
            applied: true,
            discarded_as_stale: false,
        },
        health: SearchProviderHealth {
            provider_id: SearchProviderId::LocalFolders,
            state: SearchProviderHealthState::Ready,
            reason_code: None,
            message: Some("local shell folder and command rows are bounded".to_string()),
        },
    }
}

fn local_rows(context: &SearchQueryContext) -> Vec<LocalRow> {
    let mut rows = Vec::new();
    push_existing_folder(
        &mut rows,
        "folder:dev-root",
        "C:\\dev",
        PathBuf::from(r"C:\dev"),
        &["dev", "developer", "code", "c dev", "c:\\dev", "c://dev"],
        1_420,
    );
    if let Ok(current_dir) = env::current_dir() {
        push_existing_folder(
            &mut rows,
            "folder:workspace-current",
            current_dir
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| current_dir.display().to_string()),
            current_dir,
            &["workspace", "repo", "project", "jasonshell"],
            1_430,
        );
    }
    if let Some(profile) = env_path("USERPROFILE") {
        push_existing_folder(
            &mut rows,
            "folder:user-profile",
            profile
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| "User Profile".to_string()),
            profile.clone(),
            &["home", "profile", "user"],
            1_300,
        );
        push_existing_folder(
            &mut rows,
            "folder:desktop",
            "Desktop",
            profile.join("Desktop"),
            &["desktop"],
            1_360,
        );
        push_existing_folder(
            &mut rows,
            "folder:downloads",
            "Downloads",
            profile.join("Downloads"),
            &["downloads", "download"],
            1_360,
        );
        push_existing_folder(
            &mut rows,
            "folder:documents",
            "Documents",
            profile.join("Documents"),
            &["documents", "docs"],
            1_340,
        );
    }
    if let Some(workspace_roots) = context.workspace_roots.as_ref() {
        for (index, root) in workspace_roots.iter().enumerate() {
            push_existing_folder(
                &mut rows,
                &format!("folder:workspace:{index}"),
                PathBuf::from(root)
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_else(|| root.clone()),
                PathBuf::from(root),
                &["workspace", "repo", "project"],
                1_410,
            );
        }
    }

    rows.push(LocalRow {
        id: "command:open-control-plane".to_string(),
        title: "Control Plane".to_string(),
        subtitle: "Open JasonShell Control Plane".to_string(),
        path: None,
        kind: SearchResultKind::Command,
        provider_id: SearchProviderId::Commands,
        action: SearchResultAction::RunCommand {
            command_id: "command:open-control-plane".to_string(),
        },
        aliases: vec![
            "control plane".to_string(),
            "jasonshell control".to_string(),
            "dashboard".to_string(),
        ],
        priority: 1_250,
    });

    rows
}

fn push_existing_folder(
    rows: &mut Vec<LocalRow>,
    id: &str,
    title: impl Into<String>,
    path: PathBuf,
    aliases: &[&str],
    priority: i32,
) {
    if !path.is_dir() {
        return;
    }
    rows.push(LocalRow {
        id: id.to_string(),
        title: title.into(),
        subtitle: format!("Folder - {}", path.display()),
        path: Some(path.clone()),
        kind: SearchResultKind::Folder,
        provider_id: SearchProviderId::LocalFolders,
        action: SearchResultAction::OpenFolder {
            path: path.display().to_string(),
        },
        aliases: aliases.iter().map(|alias| (*alias).to_string()).collect(),
        priority,
    });
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.exists())
}

fn rank_local_rows(rows: &[LocalRow], query: &str, limit: usize) -> Vec<SearchResult> {
    let tokens = query_tokens(query);
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut results = rows
        .iter()
        .filter_map(|row| {
            score_local_row(row, &tokens).map(|(score, reason)| local_result(row, score, reason))
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

fn local_result(row: &LocalRow, score: i32, reason: &'static str) -> SearchResult {
    let path_text = row.path.as_ref().map(|path| path.display().to_string());
    let icon_data_url = row.path.as_deref().and_then(icon_data_url_for_path);
    SearchResult {
        id: row.id.clone(),
        provider_id: row.provider_id,
        kind: row.kind,
        title: row.title.clone(),
        subtitle: Some(row.subtitle.clone()),
        path: path_text.clone(),
        action: row.action.clone(),
        terms: token_terms(&format!(
            "{} {} {}",
            row.title,
            row.aliases.join(" "),
            path_text.clone().unwrap_or_default()
        )),
        aliases: row.aliases.clone(),
        score,
        match_reason: reason.to_string(),
        record_key: row
            .path
            .as_ref()
            .map(|path| {
                format!(
                    "folder:{}",
                    normalize_record_key(&path.display().to_string())
                )
            })
            .unwrap_or_else(|| row.id.clone()),
        title_highlight_data: Vec::new(),
        subtitle_highlight_data: Vec::new(),
        icon_data_url,
    }
}

fn score_local_row(row: &LocalRow, tokens: &[String]) -> Option<(i32, &'static str)> {
    let query = tokens.join(" ");
    let hidden = row
        .aliases
        .iter()
        .cloned()
        .chain(row.path.iter().map(|path| path.display().to_string()))
        .collect::<Vec<_>>();
    let matched = best_match(
        &row.title,
        Some(&row.subtitle),
        &hidden,
        &query,
        tokens,
        true,
    )?;
    Some((row.priority + matched.score, matched.reason))
}

fn query_tokens(query: &str) -> Vec<String> {
    match_query_tokens(query)
}

fn token_terms(value: &str) -> Vec<String> {
    normalize(value)
        .split(' ')
        .filter(|token| !token.is_empty())
        .take(16)
        .map(str::to_string)
        .collect()
}

fn normalize_record_key(path: &str) -> String {
    path.trim().replace('/', r"\").to_lowercase()
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
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn important_folder_queries_match_existing_dev_root_only() {
        let root = test_dir("dev-root");
        fs::create_dir_all(&root).unwrap();
        let rows = vec![LocalRow {
            id: "folder:test-dev".to_string(),
            title: "dev".to_string(),
            subtitle: format!("Folder - {}", root.display()),
            path: Some(root.clone()),
            kind: SearchResultKind::Folder,
            provider_id: SearchProviderId::LocalFolders,
            action: SearchResultAction::OpenFolder {
                path: root.display().to_string(),
            },
            aliases: vec![
                "dev".to_string(),
                "c dev".to_string(),
                "c:\\dev".to_string(),
                "c://dev".to_string(),
            ],
            priority: 1_420,
        }];

        for query in [r"C:\dev", "C://dev", "c dev", "dev"] {
            let result = rank_local_rows(&rows, query, 1).pop().unwrap();
            assert_eq!(result.kind, SearchResultKind::Folder);
            assert!(result.score >= 1_600, "{query} scores as important folder");
        }

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn missing_important_roots_emit_no_fake_rows() {
        let mut rows = Vec::new();
        push_existing_folder(
            &mut rows,
            "folder:missing",
            "Missing",
            PathBuf::from(r"C:\path-that-should-not-exist-jasonshell"),
            &["missing"],
            1_000,
        );

        assert!(rows.is_empty());
    }

    #[test]
    fn bounded_command_rows_match_without_filesystem_scan() {
        let rows = vec![LocalRow {
            id: "command:open-control-plane".to_string(),
            title: "Control Plane".to_string(),
            subtitle: "Open JasonShell Control Plane".to_string(),
            path: None,
            kind: SearchResultKind::Command,
            provider_id: SearchProviderId::Commands,
            action: SearchResultAction::RunCommand {
                command_id: "command:open-control-plane".to_string(),
            },
            aliases: vec!["control plane".to_string()],
            priority: 1_250,
        }];

        let result = rank_local_rows(&rows, "control plane", 1).pop().unwrap();

        assert_eq!(result.provider_id, SearchProviderId::Commands);
        assert!(result.action.is_safe());
    }

    fn test_dir(name: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("search-phase4-{name}-{id}"))
    }
}
