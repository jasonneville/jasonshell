use crate::stack_popup::items::stack_item_metadata_from_path;
use crate::stack_popup::models::{
    StackFolderPage, StackFolderPageDiagnostics, StackFolderWarning, StackItem,
};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
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
    let page_started_at = Instant::now();
    let page_limit = limit.max(1);
    let mut warnings = Vec::new();
    let mut session_started_at = page_started_at;

    let (effective_session_id, entries, total) = if offset == 0 {
        let (entries, discovered_warnings) = collect_stack_folder_entries(path)?;
        warnings.extend(discovered_warnings);
        let total = entries.len();
        let effective_session_id = with_session_store(|store| {
            store.start_session(path, entries.clone(), total, page_started_at)
        });
        (Some(effective_session_id), entries, total)
    } else {
        let requested_session_id = session_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Missing stack folder listing session id for continuation".to_string())?;
        let snapshot = with_session_store(|store| {
            store
                .continue_session(path, requested_session_id)
                .map(|session| {
                    (
                        session.id.clone(),
                        session.entries.clone(),
                        session.total,
                        session.started_at,
                    )
                })
        })?;
        session_started_at = snapshot.3;
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
        match entry.item {
            StackFolderEntryItem::Filesystem(path) => match stack_item_metadata_from_path(path.clone()) {
                Ok(item) => items.push(item),
                Err(message) => warnings.push(stack_folder_warning(Some(path), message)),
            },
            StackFolderEntryItem::Virtual(item) => items.push(item),
        }
    }

    let has_more = offset + page_len < total;
    if !has_more {
        if let Some(active_session_id) = effective_session_id.as_deref() {
            with_session_store(|store| store.finish_session(path, active_session_id));
        }
    }

    let diagnostics = StackFolderPageDiagnostics {
        folder_open_duration_ms: session_started_at.elapsed().as_millis(),
        page_duration_ms: page_started_at.elapsed().as_millis(),
        page_item_count: items.len(),
        icon_resolution_count: 0,
        icon_resolution_duration_ms: 0,
        icon_cache_hits: 0,
        icon_cache_misses: 0,
        icon_fallback_count: 0,
        payload_item_count: items.len(),
    };
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
    if let Some((archive_path, prefix)) = split_zip_virtual_path(path) {
        return collect_zip_folder_entries(&archive_path, &prefix);
    }

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

fn log_stack_folder_page_diagnostics(
    path: &str,
    offset: usize,
    requested_limit: usize,
    total: usize,
    diagnostics: &StackFolderPageDiagnostics,
) {
    eprintln!(
        "stack-folder-page path=\"{}\" offset={} requestedLimit={} total={} pageDurationMs={} folderOpenDurationMs={} iconResolutionCount={} iconResolutionDurationMs={} iconCacheHits={} iconCacheMisses={} iconFallbackCount={} payloadItemCount={}",
        path,
        offset,
        requested_limit,
        total,
        diagnostics.page_duration_ms,
        diagnostics.folder_open_duration_ms,
        diagnostics.icon_resolution_count,
        diagnostics.icon_resolution_duration_ms,
        diagnostics.icon_cache_hits,
        diagnostics.icon_cache_misses,
        diagnostics.icon_fallback_count,
        diagnostics.payload_item_count
    );
}

fn collect_zip_folder_entries(
    archive_path: &Path,
    prefix: &str,
) -> Result<(Vec<StackFolderEntrySummary>, Vec<StackFolderWarning>), String> {
    let file = File::open(archive_path).map_err(|error| format!("Failed to open zip archive: {error}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| format!("Failed to read zip archive: {error}"))?;
    let mut by_name = HashMap::<String, StackItem>::new();
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(|error| format!("Failed to read zip entry: {error}"))?;
        let Some((name, is_dir)) = zip_child_entry(prefix, file.name(), file.is_dir()) else {
            continue;
        };
        by_name.entry(name.clone()).or_insert_with(|| virtual_zip_stack_item(archive_path, prefix, &name, is_dir, file.size()));
    }
    let mut entries = by_name
        .into_values()
        .map(|item| StackFolderEntrySummary {
            name: item.name.clone(),
            is_dir: item.kind == "folder",
            item: StackFolderEntryItem::Virtual(item),
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        folder_sort_rank(a.is_dir)
            .cmp(&folder_sort_rank(b.is_dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok((entries, Vec::new()))
}

fn zip_child_entry(prefix: &str, name: &str, is_dir: bool) -> Option<(String, bool)> {
    let normalized = name.replace('/', "\\").trim_matches('\\').to_string();
    let normalized_prefix = prefix.replace('/', "\\").trim_matches('\\').to_string();
    let relative = if normalized_prefix.is_empty() {
        normalized.as_str()
    } else {
        normalized.strip_prefix(&format!("{}\\", normalized_prefix))?
    };
    if relative.is_empty() {
        return None;
    }
    let mut parts = relative.split('\\');
    let first = parts.next()?.to_string();
    Some((first, is_dir || parts.next().is_some()))
}

fn virtual_zip_stack_item(archive_path: &Path, prefix: &str, name: &str, is_dir: bool, size: u64) -> StackItem {
    let relative_path = if prefix.is_empty() { name.to_string() } else { format!("{}\\{}", prefix.trim_matches('\\'), name) };
    StackItem {
        path: format!("{}\\{}", archive_path.to_string_lossy(), relative_path),
        name: name.to_string(),
        kind: if is_dir { "folder" } else { "file" }.to_string(),
        type_label: if is_dir { "Folder" } else { "ZIP Entry" }.to_string(),
        icon_data_url: None,
        size_bytes: (!is_dir).then_some(size),
        modified_at: None,
        is_hidden: name.starts_with('.'),
        is_readonly: true,
        is_system: false,
        is_symlink: false,
        is_reparse_point: false,
    }
}

fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

fn split_zip_virtual_path(path: &str) -> Option<(PathBuf, String)> {
    let lower = path.to_ascii_lowercase();
    let zip_index = lower.find(".zip")? + 4;
    let archive = PathBuf::from(&path[..zip_index]);
    if !archive.is_file() || !is_zip_path(&archive) {
        return None;
    }
    let prefix = path[zip_index..].trim_start_matches(['\\', '/']).to_string();
    Some((archive, prefix))
}

#[derive(Clone, Debug)]
struct StackFolderEntrySummary {
    name: String,
    is_dir: bool,
    item: StackFolderEntryItem,
}

#[derive(Clone, Debug)]
enum StackFolderEntryItem {
    Filesystem(PathBuf),
    Virtual(StackItem),
}

#[derive(Clone, Debug)]
struct StackFolderListingSession {
    id: String,
    path: String,
    entries: Vec<StackFolderEntrySummary>,
    total: usize,
    started_at: Instant,
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
        started_at: Instant,
    ) -> String {
        self.next_id += 1;
        let session_id = format!("stack-listing-{}", self.next_id);
        let session = StackFolderListingSession {
            id: session_id.clone(),
            path: path.to_string(),
            entries,
            total,
            started_at,
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

    Ok(StackFolderEntrySummary { name, is_dir, item: StackFolderEntryItem::Filesystem(path) })
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
    use std::time::Duration;

    #[test]
    fn stack_folder_page_diagnostics_defaults_to_zeroed_metrics() {
        let diagnostics = StackFolderPageDiagnostics::default();

        assert_eq!(diagnostics.folder_open_duration_ms, 0);
        assert_eq!(diagnostics.page_duration_ms, 0);
        assert_eq!(diagnostics.page_item_count, 0);
        assert_eq!(diagnostics.icon_resolution_count, 0);
        assert_eq!(diagnostics.icon_resolution_duration_ms, 0);
        assert_eq!(diagnostics.icon_cache_hits, 0);
        assert_eq!(diagnostics.icon_cache_misses, 0);
        assert_eq!(diagnostics.icon_fallback_count, 0);
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

    #[test]
    fn mixed_large_folder_listing_completes_across_pages_without_duplicates_or_skips() {
        let root = test_dir("mixed-large-folder");
        fs::create_dir_all(&root).unwrap();

        for index in 0..180usize {
            fs::write(root.join(format!("archive-{index:03}.zip")), b"x").unwrap();
            fs::write(root.join(format!("tool-{index:03}.exe")), b"x").unwrap();
            fs::write(root.join(format!("note-{index:03}.txt")), b"x").unwrap();
        }

        let path = root.to_string_lossy().to_string();
        let mut offset = 0usize;
        let mut session_id: Option<String> = None;
        let mut all_paths = Vec::new();
        let mut total = 0usize;

        loop {
            let page =
                read_stack_folder_page_with_session(&path, session_id.as_deref(), offset, 75).unwrap();
            session_id = page.session_id.clone().or(session_id);
            if total == 0 {
                total = page.total;
            }
            all_paths.extend(page.items.iter().map(|item| item.path.clone()));
            if !page.has_more {
                break;
            }
            offset += page.limit;
        }

        let unique = all_paths.iter().cloned().collect::<HashSet<_>>();
        assert_eq!(total, 540);
        assert_eq!(all_paths.len(), 540);
        assert_eq!(unique.len(), 540);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn continuation_diagnostics_keep_session_elapsed_time_for_metadata_completion_timing() {
        let root = test_dir("session-elapsed-diagnostics");
        fs::create_dir_all(&root).unwrap();
        for index in 0..160usize {
            fs::write(root.join(format!("row-{index:03}.txt")), b"x").unwrap();
        }

        let path = root.to_string_lossy().to_string();
        let first = read_stack_folder_page_with_session(&path, None, 0, 80).unwrap();
        let session_id = first.session_id.clone().unwrap();
        std::thread::sleep(Duration::from_millis(25));
        let second = read_stack_folder_page_with_session(&path, Some(&session_id), 80, 80).unwrap();

        let first_elapsed = first
            .diagnostics
            .as_ref()
            .expect("first diagnostics")
            .folder_open_duration_ms;
        let second_elapsed = second
            .diagnostics
            .as_ref()
            .expect("second diagnostics")
            .folder_open_duration_ms;

        assert!(
            second_elapsed >= first_elapsed + 20,
            "expected continuation elapsed time to include inter-page delay; first={first_elapsed}, second={second_elapsed}"
        );

        fs::remove_dir_all(root).ok();
    }

    fn test_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "jasonshell-stack-paging-{name}-{}",
            std::process::id()
        ))
    }
}
