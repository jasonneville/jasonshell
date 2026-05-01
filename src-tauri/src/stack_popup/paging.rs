use crate::stack_popup::items::stack_item_metadata_from_path;
use crate::stack_popup::models::{
    StackFolderPage, StackFolderPageDiagnostics, StackFolderWarning,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub(crate) const DEFAULT_PAGE_LIMIT: usize = 80;
const MAX_STACK_FOLDER_SESSIONS: usize = 32;

#[cfg(test)]
pub(crate) fn read_stack_folder_page(
    path: &str,
    offset: usize,
    limit: usize,
) -> Result<StackFolderPage, String> {
    read_stack_folder_page_with_session(path, None, offset, limit)
}

pub(crate) fn read_stack_folder_page_with_session(
    path: &str,
    session_id: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<StackFolderPage, String> {
    let tracker = StackFolderPageTracker::begin();
    let page_limit = limit.max(1);
    let mut warnings = Vec::new();

    let (effective_session_id, entries, total) = if offset == 0 {
        let (entries, discovered_warnings) = collect_stack_folder_entries(path)?;
        warnings.extend(discovered_warnings);
        let total = entries.len();
        let effective_session_id =
            with_session_store(|store| store.start_session(path, entries.clone(), total));
        (Some(effective_session_id), entries, total)
    } else {
        let requested_session_id = session_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Missing stack folder listing session id for continuation".to_string())?;
        let snapshot = with_session_store(|store| {
            store
                .continue_session(path, requested_session_id)
                .map(|session| (session.id.clone(), session.entries.clone(), session.total))
        })?;
        (Some(snapshot.0), snapshot.1, snapshot.2)
    };

    let page_entries = entries
        .iter()
        .skip(offset)
        .take(page_limit)
        .cloned()
        .collect::<Vec<_>>();
    let page_len = page_entries.len();
    let mut items = Vec::with_capacity(page_len);
    for entry in page_entries {
        match stack_item_metadata_from_path(entry.path.clone()) {
            Ok(item) => items.push(item),
            Err(message) => warnings.push(stack_folder_warning(Some(entry.path), message)),
        }
    }

    let has_more = offset + page_len < total;
    if !has_more {
        if let Some(active_session_id) = effective_session_id.as_deref() {
            with_session_store(|store| store.finish_session(path, active_session_id));
        }
    }

    let diagnostics = tracker.finish(items.len());
    log_stack_folder_page_diagnostics(path, offset, page_limit, total, &diagnostics);
    Ok(StackFolderPage {
        path: path.to_string(),
        items,
        offset,
        limit: page_len,
        total,
        has_more,
        warnings,
        diagnostics: Some(diagnostics),
        session_id: effective_session_id,
    })
}

fn collect_stack_folder_entries(
    path: &str,
) -> Result<(Vec<StackFolderEntrySummary>, Vec<StackFolderWarning>), String> {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for entry in
        fs::read_dir(path).map_err(|error| format!("Failed to read stack folder: {error}"))?
    {
        match entry {
            Ok(entry) => match stack_folder_entry_summary(entry) {
                Ok(summary) => entries.push(summary),
                Err((path, message)) => warnings.push(stack_folder_warning(path, message)),
            },
            Err(error) => warnings.push(stack_folder_warning(
                None,
                format!("Failed to read stack folder entry: {error}"),
            )),
        }
    }
    entries.sort_by(|a, b| {
        folder_sort_rank(a.is_dir)
            .cmp(&folder_sort_rank(b.is_dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok((entries, warnings))
}

#[derive(Debug)]
struct StackFolderPageTracker {
    started_at: Instant,
}

impl StackFolderPageTracker {
    fn begin() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }

    fn finish(self, payload_item_count: usize) -> StackFolderPageDiagnostics {
        let elapsed = self.started_at.elapsed();
        StackFolderPageDiagnostics {
            folder_open_duration_ms: elapsed.as_millis(),
            page_duration_ms: elapsed.as_millis(),
            page_item_count: payload_item_count,
            icon_resolution_count: 0,
            icon_resolution_duration_ms: 0,
            payload_item_count,
        }
    }
}

fn log_stack_folder_page_diagnostics(
    path: &str,
    offset: usize,
    requested_limit: usize,
    total: usize,
    diagnostics: &StackFolderPageDiagnostics,
) {
    eprintln!(
        "stack-folder-page path=\"{}\" offset={} requestedLimit={} total={} pageDurationMs={} iconResolutionCount={} iconResolutionDurationMs={} payloadItemCount={}",
        path,
        offset,
        requested_limit,
        total,
        diagnostics.page_duration_ms,
        diagnostics.icon_resolution_count,
        diagnostics.icon_resolution_duration_ms,
        diagnostics.payload_item_count
    );
}

#[derive(Clone, Debug)]
struct StackFolderEntrySummary {
    path: PathBuf,
    name: String,
    is_dir: bool,
}

#[derive(Clone, Debug)]
struct StackFolderListingSession {
    id: String,
    path: String,
    entries: Vec<StackFolderEntrySummary>,
    total: usize,
}

#[derive(Default)]
struct StackFolderListingSessionStore {
    next_id: u64,
    sessions: HashMap<String, StackFolderListingSession>,
    active_by_path: HashMap<String, String>,
}

impl StackFolderListingSessionStore {
    fn start_session(
        &mut self,
        path: &str,
        entries: Vec<StackFolderEntrySummary>,
        total: usize,
    ) -> String {
        self.next_id += 1;
        let session_id = format!("stack-listing-{}", self.next_id);
        let session = StackFolderListingSession {
            id: session_id.clone(),
            path: path.to_string(),
            entries,
            total,
        };
        self.sessions.insert(session_id.clone(), session);
        self.active_by_path
            .insert(path.to_string(), session_id.clone());
        self.trim_sessions();
        session_id
    }

    fn continue_session(
        &self,
        path: &str,
        session_id: &str,
    ) -> Result<&StackFolderListingSession, String> {
        let active_session = self
            .active_by_path
            .get(path)
            .ok_or_else(|| "No active stack folder listing session for path".to_string())?;
        if active_session != session_id {
            return Err("Stale stack folder listing session".to_string());
        }

        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| "Unknown stack folder listing session".to_string())?;
        if session.path != path {
            return Err("Stack folder listing session path mismatch".to_string());
        }
        Ok(session)
    }

    fn finish_session(&mut self, path: &str, session_id: &str) {
        self.sessions.remove(session_id);
        if self
            .active_by_path
            .get(path)
            .is_some_and(|active| active == session_id)
        {
            self.active_by_path.remove(path);
        }
    }

    fn trim_sessions(&mut self) {
        if self.sessions.len() <= MAX_STACK_FOLDER_SESSIONS {
            return;
        }
        let mut ordered = self.sessions.keys().cloned().collect::<Vec<_>>();
        ordered.sort();
        let trim = self
            .sessions
            .len()
            .saturating_sub(MAX_STACK_FOLDER_SESSIONS);
        for key in ordered.into_iter().take(trim) {
            self.sessions.remove(&key);
            self.active_by_path.retain(|_, active| active != &key);
        }
    }
}

static STACK_FOLDER_SESSIONS: OnceLock<Mutex<StackFolderListingSessionStore>> = OnceLock::new();

fn with_session_store<R>(callback: impl FnOnce(&mut StackFolderListingSessionStore) -> R) -> R {
    let state = STACK_FOLDER_SESSIONS.get_or_init(|| Mutex::new(StackFolderListingSessionStore::default()));
    let mut guard = state
        .lock()
        .expect("stack folder listing session state is poisoned");
    callback(&mut guard)
}

fn stack_folder_entry_summary(
    entry: fs::DirEntry,
) -> Result<StackFolderEntrySummary, (Option<PathBuf>, String)> {
    let path = entry.path();
    let file_type = entry.file_type().map_err(|error| {
        (
            Some(path.clone()),
            format!("Failed to inspect stack item: {error}"),
        )
    })?;
    let is_dir = if file_type.is_dir() {
        true
    } else if file_type.is_symlink() {
        fs::metadata(&path)
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false)
    } else {
        false
    };
    let name = entry
        .file_name()
        .to_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    Ok(StackFolderEntrySummary { path, name, is_dir })
}

fn folder_sort_rank(is_dir: bool) -> u8 {
    if is_dir {
        0
    } else {
        1
    }
}

pub(crate) fn stack_folder_warning(path: Option<PathBuf>, message: String) -> StackFolderWarning {
    StackFolderWarning {
        path: path.map(|path| path.to_string_lossy().into_owned()),
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::{read_stack_folder_page_with_session, StackFolderPageDiagnostics};
    use std::collections::HashSet;
    use std::fs;

    #[test]
    fn stack_folder_page_diagnostics_defaults_to_zeroed_metrics() {
        let diagnostics = StackFolderPageDiagnostics::default();

        assert_eq!(diagnostics.folder_open_duration_ms, 0);
        assert_eq!(diagnostics.page_duration_ms, 0);
        assert_eq!(diagnostics.page_item_count, 0);
        assert_eq!(diagnostics.icon_resolution_count, 0);
        assert_eq!(diagnostics.icon_resolution_duration_ms, 0);
        assert_eq!(diagnostics.payload_item_count, 0);
    }

    #[test]
    fn stack_folder_session_paging_returns_stable_unique_rows_across_pages() {
        let root = test_dir("stable-session-paging");
        fs::create_dir_all(&root).unwrap();
        for index in 0..505usize {
            fs::write(root.join(format!("row-{index:03}.txt")), b"x").unwrap();
        }

        let path = root.to_string_lossy().to_string();
        let first = read_stack_folder_page_with_session(&path, None, 0, 120).unwrap();
        let session_id = first.session_id.clone().unwrap();
        let second =
            read_stack_folder_page_with_session(&path, Some(&session_id), 120, 120).unwrap();
        let third =
            read_stack_folder_page_with_session(&path, Some(&session_id), 240, 120).unwrap();
        let fourth =
            read_stack_folder_page_with_session(&path, Some(&session_id), 360, 120).unwrap();
        let fifth =
            read_stack_folder_page_with_session(&path, Some(&session_id), 480, 120).unwrap();

        let all = [first.items, second.items, third.items, fourth.items, fifth.items]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let unique = all
            .iter()
            .map(|item| item.path.clone())
            .collect::<HashSet<_>>();

        assert_eq!(all.len(), 505);
        assert_eq!(unique.len(), 505);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn stale_stack_folder_session_is_rejected_after_new_session_starts() {
        let root = test_dir("stale-session");
        fs::create_dir_all(&root).unwrap();
        for index in 0..12usize {
            fs::write(root.join(format!("row-{index:03}.txt")), b"x").unwrap();
        }

        let path = root.to_string_lossy().to_string();
        let first_session = read_stack_folder_page_with_session(&path, None, 0, 4).unwrap();
        let stale_session_id = first_session.session_id.clone().unwrap();
        let second_session = read_stack_folder_page_with_session(&path, None, 0, 4).unwrap();
        assert_ne!(stale_session_id, second_session.session_id.clone().unwrap());

        let stale = read_stack_folder_page_with_session(&path, Some(&stale_session_id), 4, 4);
        assert!(stale.is_err());

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn session_continuation_uses_stable_snapshot_without_rereading_directory() {
        let root = test_dir("stable-snapshot");
        fs::create_dir_all(&root).unwrap();
        for index in 0..4usize {
            fs::write(root.join(format!("row-{index:03}.txt")), b"x").unwrap();
        }

        let path = root.to_string_lossy().to_string();
        let first = read_stack_folder_page_with_session(&path, None, 0, 2).unwrap();
        let session_id = first.session_id.clone().unwrap();
        fs::write(root.join("row-900.txt"), b"new").unwrap();

        let second = read_stack_folder_page_with_session(&path, Some(&session_id), 2, 2).unwrap();
        let names = second
            .items
            .iter()
            .map(|item| item.name.clone())
            .collect::<Vec<_>>();

        assert_eq!(second.total, 4);
        assert!(!names.iter().any(|name| name == "row-900.txt"));

        fs::remove_dir_all(root).ok();
    }

    fn test_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "jasonshell-stack-paging-{name}-{}",
            std::process::id()
        ))
    }
}
