use crate::stack_popup::items::{metadata_modified_epoch_millis, stack_item_metadata_from_path};
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
pub(crate) const MAX_PAGE_LIMIT: usize = 200;
pub(crate) const MAX_FILESYSTEM_LISTING_ENTRIES: usize = 10_000;
pub(crate) const MAX_ZIP_RAW_ENTRIES_SCANNED: usize = 20_000;
pub(crate) const MAX_ZIP_ENTRY_NAME_BYTES: usize = 4 * 1024;
pub(crate) const MAX_STACK_FOLDER_SESSIONS: usize = 32;
pub(crate) const MAX_STACK_FOLDER_SESSION_ENTRIES: usize = 50_000;
pub(crate) const MAX_STACK_FOLDER_SESSION_BYTES: usize = 32 * 1024 * 1024;

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
    read_stack_folder_page_with_session_and_downloads_detector(
        path,
        session_id,
        offset,
        limit,
        is_default_downloads_folder_path,
    )
}

pub(crate) fn read_stack_folder_page_with_session_and_downloads_detector(
    path: &str,
    session_id: Option<&str>,
    offset: usize,
    limit: usize,
    downloads_detector: fn(&str) -> bool,
) -> Result<StackFolderPage, String> {
    let page_started_at = Instant::now();
    let page_limit = limit.clamp(1, MAX_PAGE_LIMIT);
    let mut warnings = Vec::new();
    let is_downloads = downloads_detector(path);

    let (effective_session_id, page_entries, total, session_started_at) = if offset == 0 {
        let (mut entries, discovered_warnings) = collect_stack_folder_entries(path, is_downloads)?;
        warnings.extend(discovered_warnings);
        truncate_entries_to_session_byte_budget(&mut entries, path, &mut warnings);
        let total = entries.len();
        let effective_session_id =
            with_session_store(|store| store.start_session(path, entries, total, page_started_at));
        let snapshot = with_session_store(|store| {
            store.session_page(path, &effective_session_id, offset, page_limit)
        })?;
        (
            Some(effective_session_id),
            snapshot.0,
            snapshot.1,
            snapshot.2,
        )
    } else {
        let requested_session_id = session_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "Missing stack folder listing session id for continuation".to_string()
            })?;
        let snapshot = with_session_store(|store| {
            store.session_page(path, requested_session_id, offset, page_limit)
        })?;
        (
            Some(requested_session_id.to_string()),
            snapshot.0,
            snapshot.1,
            snapshot.2,
        )
    };

    let page_len = page_entries.len();
    let mut items = Vec::with_capacity(page_len);
    for entry in page_entries {
        match entry.item {
            StackFolderEntryItem::Filesystem(path) => {
                match stack_item_metadata_from_path(path.clone()) {
                    Ok(item) => items.push(item),
                    Err(message) => warnings.push(stack_folder_warning(Some(path), message)),
                }
            }
            StackFolderEntryItem::Virtual(item) => items.push(item),
        }
    }

    let has_more = offset.checked_add(page_len).is_some_and(|sum| sum < total);
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
        sort_column: if is_downloads {
            "modified".to_string()
        } else {
            "name".to_string()
        },
        sort_direction: if is_downloads {
            "desc".to_string()
        } else {
            "asc".to_string()
        },
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
    is_downloads: bool,
) -> Result<(Vec<StackFolderEntrySummary>, Vec<StackFolderWarning>), String> {
    if let Some((archive_path, prefix)) = split_zip_virtual_path(path) {
        return collect_zip_folder_entries(&archive_path, &prefix);
    }

    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for entry in
        fs::read_dir(path).map_err(|error| format!("Failed to read stack folder: {error}"))?
    {
        if entries.len() >= MAX_FILESYSTEM_LISTING_ENTRIES {
            warnings.push(stack_folder_warning(
                Some(PathBuf::from(path)),
                format!(
                    "Stack folder filesystem discovery truncated at {MAX_FILESYSTEM_LISTING_ENTRIES} retained entries; retained/discovered total and global sort are capped and incomplete beyond the cap"
                ),
            ));
            break;
        }
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
    entries.sort_by(|a, b| compare_stack_entries(a, b, is_downloads));
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
    let file =
        File::open(archive_path).map_err(|error| format!("Failed to open zip archive: {error}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("Failed to read zip archive: {error}"))?;
    let mut by_name = HashMap::<String, StackItem>::new();
    let mut warnings = Vec::new();
    let mut raw_entries_scanned = 0usize;
    for index in 0..archive.len() {
        if raw_entries_scanned >= MAX_ZIP_RAW_ENTRIES_SCANNED {
            warnings.push(stack_folder_warning(
                Some(archive_path.to_path_buf()),
                format!(
                    "ZIP discovery truncated after {MAX_ZIP_RAW_ENTRIES_SCANNED} raw entries; retained/discovered total and global sort are capped and incomplete beyond the cap"
                ),
            ));
            break;
        }
        raw_entries_scanned += 1;
        let file = archive
            .by_index(index)
            .map_err(|error| format!("Failed to read zip entry: {error}"))?;
        if file.name().len() > MAX_ZIP_ENTRY_NAME_BYTES {
            if !warnings
                .iter()
                .any(|warning| warning.message.contains("entry-name byte cap"))
            {
                warnings.push(stack_folder_warning(
                    Some(archive_path.to_path_buf()),
                    format!(
                        "ZIP entries with names over the {MAX_ZIP_ENTRY_NAME_BYTES}-byte entry-name byte cap were skipped"
                    ),
                ));
            }
            continue;
        }
        let Some((name, is_dir)) = zip_child_entry(prefix, file.name(), file.is_dir()) else {
            continue;
        };
        match push_bounded_unique_zip_child(
            &mut by_name,
            archive_path,
            prefix,
            name,
            is_dir,
            file.size(),
            &mut warnings,
            Some(archive_path.to_path_buf()),
        ) {
            ZipChildPushResult::Inserted => {}
            ZipChildPushResult::Duplicate => {}
            ZipChildPushResult::RetainedCapReached => break,
        }
    }
    let entries = finalize_zip_children(by_name, archive_path, false, &mut warnings);
    Ok((entries, warnings))
}

fn compare_stack_entries(
    a: &StackFolderEntrySummary,
    b: &StackFolderEntrySummary,
    newest_first: bool,
) -> std::cmp::Ordering {
    if newest_first {
        compare_newest_first(a, b)
    } else {
        folder_sort_rank(a.is_dir)
            .cmp(&folder_sort_rank(b.is_dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    }
}

fn compare_newest_first(
    a: &StackFolderEntrySummary,
    b: &StackFolderEntrySummary,
) -> std::cmp::Ordering {
    match (a.modified_at, b.modified_at) {
        (Some(a_modified), Some(b_modified)) => b_modified
            .cmp(&a_modified)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, None) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    }
}

pub(crate) fn is_default_downloads_folder_path(path: &str) -> bool {
    let Some(profile) = crate::stack_popup::paths::user_profile_dir() else {
        return false;
    };
    let Some(downloads) =
        crate::stack_popup::paths::resolve_stack_alias_with_profile("shell:Downloads", &profile)
    else {
        return false;
    };
    is_downloads_folder_path(path, &downloads)
}

#[cfg(test)]
fn sort_stack_entries_for_test(
    entries: Vec<StackFolderEntrySummary>,
    newest_first: bool,
) -> Vec<StackFolderEntrySummary> {
    let mut entries = entries;
    entries.sort_by(|a, b| compare_stack_entries(a, b, newest_first));
    entries
}

fn is_downloads_folder_path(path: &str, downloads: &Path) -> bool {
    let actual = fs::canonicalize(path)
        .ok()
        .map(|value| value.to_string_lossy().into_owned());
    let expected = fs::canonicalize(downloads)
        .ok()
        .map(|value| value.to_string_lossy().into_owned());
    actual
        .zip(expected)
        .is_some_and(|(a, b)| a.eq_ignore_ascii_case(&b))
}

fn zip_child_entry(prefix: &str, name: &str, is_dir: bool) -> Option<(String, bool)> {
    if name.len() > MAX_ZIP_ENTRY_NAME_BYTES || prefix.len() > MAX_ZIP_ENTRY_NAME_BYTES {
        return None;
    }
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

fn virtual_zip_stack_item(
    archive_path: &Path,
    prefix: &str,
    name: &str,
    is_dir: bool,
    size: u64,
) -> StackItem {
    let relative_path = if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{}\\{}", prefix.trim_matches('\\'), name)
    };
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

enum ZipChildPushResult {
    Inserted,
    Duplicate,
    RetainedCapReached,
}

fn push_bounded_unique_zip_child(
    by_name: &mut HashMap<String, StackItem>,
    archive_path: &Path,
    prefix: &str,
    name: String,
    is_dir: bool,
    size: u64,
    warnings: &mut Vec<StackFolderWarning>,
    warning_path: Option<PathBuf>,
) -> ZipChildPushResult {
    if by_name.contains_key(&name) {
        return ZipChildPushResult::Duplicate;
    }
    if by_name.len() >= MAX_FILESYSTEM_LISTING_ENTRIES {
        warnings.push(stack_folder_warning(
            warning_path,
            format!(
                "ZIP discovery stopped before adding another unique child because retained unique children would exceed {MAX_FILESYSTEM_LISTING_ENTRIES}; retained/discovered total and global sort are capped and incomplete beyond the cap"
            ),
        ));
        return ZipChildPushResult::RetainedCapReached;
    }
    by_name.insert(
        name.clone(),
        virtual_zip_stack_item(archive_path, prefix, &name, is_dir, size),
    );
    ZipChildPushResult::Inserted
}

fn finalize_zip_children(
    by_name: HashMap<String, StackItem>,
    archive_path: &Path,
    newest_first: bool,
    warnings: &mut Vec<StackFolderWarning>,
) -> Vec<StackFolderEntrySummary> {
    let mut entries = by_name
        .into_values()
        .map(|item| StackFolderEntrySummary {
            name: item.name.clone(),
            is_dir: item.kind == "folder",
            modified_at: item.modified_at,
            item: StackFolderEntryItem::Virtual(item),
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| compare_stack_entries(a, b, newest_first));
    if entries.len() > MAX_FILESYSTEM_LISTING_ENTRIES {
        entries.truncate(MAX_FILESYSTEM_LISTING_ENTRIES);
        warnings.push(stack_folder_warning(
            Some(archive_path.to_path_buf()),
            format!(
                "ZIP retained/discovered total and global sort are capped at {MAX_FILESYSTEM_LISTING_ENTRIES}; retained ordering is deterministic only within the retained slice"
            ),
        ));
    }
    entries
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
    let prefix = path[zip_index..]
        .trim_start_matches(['\\', '/'])
        .to_string();
    Some((archive, prefix))
}

#[derive(Clone, Debug)]
struct StackFolderEntrySummary {
    name: String,
    is_dir: bool,
    modified_at: Option<u64>,
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
        mut entries: Vec<StackFolderEntrySummary>,
        total: usize,
        started_at: Instant,
    ) -> String {
        truncate_entries_to_byte_budget(&mut entries);
        let total = total.min(entries.len());
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

    fn session_page(
        &self,
        path: &str,
        session_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<StackFolderEntrySummary>, usize, Instant), String> {
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
        Ok((
            page_slice_for_test(&session.entries, offset, limit),
            session.total,
            session.started_at,
        ))
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
        self.enforce_session_budget();
    }

    fn enforce_session_budget(&mut self) {
        loop {
            let over_count = self.sessions.len() > MAX_STACK_FOLDER_SESSIONS;
            let over_entries = self
                .sessions
                .values()
                .map(|session| session.entries.len())
                .fold(0usize, usize::saturating_add)
                > MAX_STACK_FOLDER_SESSION_ENTRIES;
            let over_bytes = self.estimated_bytes() > MAX_STACK_FOLDER_SESSION_BYTES;
            if !(over_count || over_entries || over_bytes) {
                break;
            }
            let Some(oldest_id) = self
                .sessions
                .values()
                .min_by(|a, b| {
                    a.started_at
                        .cmp(&b.started_at)
                        .then_with(|| a.id.cmp(&b.id))
                })
                .map(|session| session.id.clone())
            else {
                break;
            };
            self.sessions.remove(&oldest_id);
            self.active_by_path.retain(|_, active| active != &oldest_id);
        }
    }

    fn estimated_bytes(&self) -> usize {
        self.sessions
            .values()
            .map(session_estimated_bytes)
            .fold(0usize, usize::saturating_add)
    }
}

static STACK_FOLDER_SESSIONS: OnceLock<Mutex<StackFolderListingSessionStore>> = OnceLock::new();

fn with_session_store<R>(callback: impl FnOnce(&mut StackFolderListingSessionStore) -> R) -> R {
    let state =
        STACK_FOLDER_SESSIONS.get_or_init(|| Mutex::new(StackFolderListingSessionStore::default()));
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

    Ok(StackFolderEntrySummary {
        name,
        is_dir,
        modified_at: fs::metadata(&path)
            .ok()
            .and_then(|metadata| metadata_modified_epoch_millis(&metadata)),
        item: StackFolderEntryItem::Filesystem(path),
    })
}

fn session_estimated_bytes(session: &StackFolderListingSession) -> usize {
    session
        .entries
        .iter()
        .map(stack_folder_entry_summary_estimated_bytes)
        .fold(0usize, usize::saturating_add)
}

fn stack_folder_entry_summary_estimated_bytes(entry: &StackFolderEntrySummary) -> usize {
    let name_bytes = entry.name.len();
    let path_bytes = match &entry.item {
        StackFolderEntryItem::Filesystem(path) => path.as_os_str().len(),
        StackFolderEntryItem::Virtual(item) => item.path.len(),
    };
    96 + name_bytes + path_bytes
}

fn truncate_entries_to_byte_budget(entries: &mut Vec<StackFolderEntrySummary>) -> bool {
    let mut retained_bytes = 0usize;
    let retained_count = entries
        .iter()
        .take_while(|entry| {
            let next =
                retained_bytes.saturating_add(stack_folder_entry_summary_estimated_bytes(entry));
            if next > MAX_STACK_FOLDER_SESSION_BYTES {
                return false;
            }
            retained_bytes = next;
            true
        })
        .count();
    let truncated = retained_count < entries.len();
    entries.truncate(retained_count);
    truncated
}

fn truncate_entries_to_session_byte_budget(
    entries: &mut Vec<StackFolderEntrySummary>,
    path: &str,
    warnings: &mut Vec<StackFolderWarning>,
) {
    if truncate_entries_to_byte_budget(entries) {
        warnings.push(stack_folder_warning(
            Some(PathBuf::from(path)),
            format!(
                "Stack folder retained snapshot truncated at the {MAX_STACK_FOLDER_SESSION_BYTES}-byte estimated session budget; retained/discovered total and global sort are capped and incomplete beyond the cap"
            ),
        ));
    }
}

fn page_slice_for_test(
    entries: &[StackFolderEntrySummary],
    offset: usize,
    limit: usize,
) -> Vec<StackFolderEntrySummary> {
    let start = offset.min(entries.len());
    let end = start.saturating_add(limit).min(entries.len());
    entries[start..end].to_vec()
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
    use super::{
        finalize_zip_children, push_bounded_unique_zip_child, read_stack_folder_page_with_session,
        sort_stack_entries_for_test, stack_folder_warning, zip_child_entry, StackFolderEntryItem,
        StackFolderEntrySummary, StackFolderListingSessionStore, StackFolderPageDiagnostics,
        StackFolderWarning, StackItem, MAX_FILESYSTEM_LISTING_ENTRIES, MAX_STACK_FOLDER_SESSIONS,
        MAX_STACK_FOLDER_SESSION_BYTES, MAX_ZIP_ENTRY_NAME_BYTES, MAX_ZIP_RAW_ENTRIES_SCANNED,
    };
    use std::collections::HashMap;
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};
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

        let all = [
            first.items,
            second.items,
            third.items,
            fourth.items,
            fifth.items,
        ]
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
                read_stack_folder_page_with_session(&path, session_id.as_deref(), offset, 75)
                    .unwrap();
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

    #[test]
    fn downloads_folder_page_one_contains_newest_entries_before_paging() {
        let root = test_dir("downloads-default-sort");
        let downloads = root.join("Downloads");
        fs::create_dir_all(&downloads).unwrap();
        let sorted = sort_stack_entries_for_test(
            vec![
                StackFolderEntrySummary {
                    name: "file-0.txt".into(),
                    is_dir: false,
                    modified_at: Some(1),
                    item: StackFolderEntryItem::Filesystem(downloads.join("file-0.txt")),
                },
                StackFolderEntrySummary {
                    name: "file-2.txt".into(),
                    is_dir: false,
                    modified_at: Some(3),
                    item: StackFolderEntryItem::Filesystem(downloads.join("file-2.txt")),
                },
                StackFolderEntrySummary {
                    name: "file-1.txt".into(),
                    is_dir: false,
                    modified_at: Some(2),
                    item: StackFolderEntryItem::Filesystem(downloads.join("file-1.txt")),
                },
            ],
            true,
        );
        assert_eq!(
            sorted
                .iter()
                .map(|item| item.name.clone())
                .collect::<Vec<_>>(),
            vec!["file-2.txt", "file-1.txt", "file-0.txt"]
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn ordinary_folder_pages_remain_name_sorted() {
        let root = test_dir("ordinary-name-sort");
        let folder = root.join("Documents");
        fs::create_dir_all(&folder).unwrap();
        let sorted = sort_stack_entries_for_test(
            vec![
                StackFolderEntrySummary {
                    name: "bravo.txt".into(),
                    is_dir: false,
                    modified_at: Some(2),
                    item: StackFolderEntryItem::Filesystem(folder.join("bravo.txt")),
                },
                StackFolderEntrySummary {
                    name: "alpha.txt".into(),
                    is_dir: false,
                    modified_at: Some(1),
                    item: StackFolderEntryItem::Filesystem(folder.join("alpha.txt")),
                },
            ],
            false,
        );
        assert_eq!(
            sorted
                .iter()
                .map(|item| item.name.clone())
                .collect::<Vec<_>>(),
            vec!["alpha.txt", "bravo.txt"]
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn downloads_folder_entries_sort_modified_desc_with_missing_times_last() {
        let sorted = sort_stack_entries_for_test(
            vec![
                StackFolderEntrySummary {
                    name: "old.txt".into(),
                    is_dir: false,
                    modified_at: Some(1),
                    item: StackFolderEntryItem::Filesystem(PathBuf::from("old.txt")),
                },
                StackFolderEntrySummary {
                    name: "missing.txt".into(),
                    is_dir: false,
                    modified_at: None,
                    item: StackFolderEntryItem::Filesystem(PathBuf::from("missing.txt")),
                },
                StackFolderEntrySummary {
                    name: "new.txt".into(),
                    is_dir: false,
                    modified_at: Some(2),
                    item: StackFolderEntryItem::Filesystem(PathBuf::from("new.txt")),
                },
            ],
            true,
        );

        assert_eq!(
            sorted.into_iter().map(|item| item.name).collect::<Vec<_>>(),
            vec!["new.txt", "old.txt", "missing.txt"]
        );
    }

    #[test]
    fn read_stack_folder_page_clamps_extreme_page_limit_and_preserves_limit_zero_semantics() {
        let root = test_dir("page-limit-clamp");
        fs::create_dir_all(&root).unwrap();
        for index in 0..250usize {
            fs::write(root.join(format!("row-{index:03}.txt")), b"x").unwrap();
        }

        let path = root.to_string_lossy().to_string();
        let zero_limit = read_stack_folder_page_with_session(&path, None, 0, 0).unwrap();
        let huge_limit = read_stack_folder_page_with_session(&path, None, 0, usize::MAX).unwrap();

        assert_eq!(
            zero_limit.items.len(),
            1,
            "limit=0 must still request one item"
        );
        assert_eq!(zero_limit.limit, 1, "effective limit=1 must be reported");
        assert!(
            huge_limit.items.len() <= 200,
            "usize::MAX limit must clamp to MAX_PAGE_LIMIT=200, got {}",
            huge_limit.items.len()
        );
        assert_eq!(
            huge_limit.limit, 200,
            "reported limit must be clamped max page size"
        );
        assert!(
            huge_limit.has_more,
            "250 retained rows with page cap 200 must have continuation"
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn filesystem_listing_stops_at_entry_bound_with_warning() {
        let (entries, warnings) = collect_synthetic_stack_folder_entries_for_test(10_005, true);

        assert_eq!(entries.len(), MAX_FILESYSTEM_LISTING_ENTRIES);
        assert!(!warnings.is_empty(), "missing truncation warning");
    }

    #[test]
    fn zip_listing_stops_at_raw_scan_bound_with_warning() {
        let scan = collect_zip_folder_entries_from_synthetic_names_for_test(
            (0..20_005usize).map(|_| ("child.txt".to_string(), false, 1)),
            "",
        )
        .unwrap();

        assert!(
            scan.raw_entries_scanned <= 20_000,
            "ZIP raw scan must stop at 20_000"
        );
        assert!(
            scan.entries.len() <= 10_000,
            "retained ZIP children must cap at 10_000"
        );
        assert!(
            scan.warnings.iter().any(|warning| {
                let message = warning.message.to_ascii_lowercase();
                message.contains("truncat") && message.contains("zip")
            }),
            "missing explicit ZIP truncation warning: {:?}",
            scan.warnings
        );
    }

    #[test]
    fn zip_unique_children_stop_at_first_retained_cap_overflow() {
        let mut entries = Vec::new();
        for index in 0..MAX_FILESYSTEM_LISTING_ENTRIES {
            entries.push((format!("child-{index:05}.txt"), false, 1));
        }
        for _ in 0..5_000usize {
            entries.push(("child-00000.txt".to_string(), false, 1));
        }
        entries.push(("child-overflow.txt".to_string(), false, 1));

        let scan = collect_zip_folder_entries_from_synthetic_names_for_test(entries, "").unwrap();

        assert_eq!(scan.entries.len(), MAX_FILESYSTEM_LISTING_ENTRIES);
        assert_eq!(scan.raw_entries_scanned, 15_001);
        assert_eq!(
            scan.warnings
                .iter()
                .filter(|warning| warning.message.contains("unique child"))
                .count(),
            1
        );
    }

    #[test]
    fn zip_entry_names_over_byte_bound_are_skipped_with_bounded_warning() {
        let oversized_name = format!("{}.txt", "x".repeat(MAX_ZIP_ENTRY_NAME_BYTES + 1));
        let scan = collect_zip_folder_entries_from_synthetic_names_for_test(
            [(oversized_name, false, 1)],
            "",
        )
        .unwrap();

        assert!(scan.entries.is_empty());
        assert_eq!(scan.warnings.len(), 1);
        assert!(scan.warnings[0].message.len() < 256);
        assert!(!scan.warnings[0].message.contains("xxxx"));
    }

    #[test]
    fn continuation_clones_only_requested_page_slice() {
        let mut store = StackFolderListingSessionStore::default();
        let session_id = store.start_session(
            "clone-probe",
            synthetic_entry_summaries_for_test(1_000),
            1_000,
            std::time::Instant::now(),
        );
        let page = store
            .session_page("clone-probe", &session_id, 400, 25)
            .unwrap();

        assert_eq!(page.0.len(), 25);
    }

    #[test]
    fn session_store_evicts_by_total_entry_budget_before_count_cap() {
        let mut store = StackFolderListingSessionStore::default();
        for index in 0..6usize {
            store.start_session(
                &format!("entry-budget-{index}"),
                synthetic_entry_summaries_for_test(10_001),
                10_001,
                std::time::Instant::now(),
            );
        }

        assert!(
            store.sessions.len() < 6,
            "entry budget eviction must run before 32-session cap"
        );
        assert!(
            store
                .sessions
                .values()
                .map(|session| session.entries.len())
                .sum::<usize>()
                <= 50_000,
            "total retained entries must stay within 50_000"
        );
    }

    #[test]
    fn session_store_evicts_by_estimated_byte_budget_before_count_cap() {
        let mut store = StackFolderListingSessionStore::default();
        for index in 0..3usize {
            store.start_session(
                &format!("byte-budget-{index}"),
                synthetic_named_entry_summaries_for_test(1, 11 * 1024 * 1024),
                1,
                std::time::Instant::now(),
            );
        }

        assert!(
            store.sessions.len() < 3,
            "byte budget eviction must run before 32-session cap"
        );
        assert!(
            estimated_session_store_bytes_for_test(&store) <= MAX_STACK_FOLDER_SESSION_BYTES,
            "estimated retained bytes must stay within 32 MiB"
        );
    }

    #[test]
    fn oversized_single_session_is_retained_within_byte_budget() {
        let mut store = StackFolderListingSessionStore::default();
        let session_id = store.start_session(
            "oversized-single-session",
            synthetic_named_entry_summaries_for_test(4, 10 * 1024 * 1024),
            4,
            std::time::Instant::now(),
        );

        assert!(store.sessions.contains_key(&session_id));
        assert!(estimated_session_store_bytes_for_test(&store) <= MAX_STACK_FOLDER_SESSION_BYTES);
        assert!(store
            .session_page("oversized-single-session", &session_id, 0, 1)
            .is_ok());
    }

    #[test]
    fn offset_overflow_and_beyond_total_return_empty_without_has_more() {
        let root = test_dir("offset-overflow");
        fs::create_dir_all(&root).unwrap();
        for index in 0..3usize {
            fs::write(root.join(format!("row-{index:03}.txt")), b"x").unwrap();
        }

        let path = root.to_string_lossy().to_string();
        let first = read_stack_folder_page_with_session(&path, None, 0, 2).unwrap();
        let session_id = first.session_id.clone().unwrap();
        let beyond = read_stack_folder_page_with_session(&path, Some(&session_id), usize::MAX, 2)
            .expect("offset beyond total must not overflow or fail");

        assert!(beyond.items.is_empty());
        assert_eq!(beyond.limit, 0);
        assert!(!beyond.has_more);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn session_path_mismatch_error_remains_unchanged() {
        let mut store = StackFolderListingSessionStore::default();
        let session_id = store.start_session(
            "original-path",
            synthetic_entry_summaries_for_test(1),
            1,
            std::time::Instant::now(),
        );
        store
            .active_by_path
            .insert("other-path".to_string(), session_id.clone());

        let error = store
            .session_page("other-path", &session_id, 0, 1)
            .unwrap_err();

        assert_eq!(error, "Stack folder listing session path mismatch");
    }

    #[test]
    fn active_by_path_tracks_oldest_tie_break_and_eviction() {
        let mut store = StackFolderListingSessionStore::default();
        let now = std::time::Instant::now();
        for index in 0..(MAX_STACK_FOLDER_SESSIONS + 1) {
            store.start_session(
                &format!("path-{index}"),
                synthetic_entry_summaries_for_test(1),
                1,
                now,
            );
        }

        assert_eq!(store.sessions.len(), MAX_STACK_FOLDER_SESSIONS);
        assert!(!store.active_by_path.contains_key("path-0"));
        assert!(store
            .active_by_path
            .contains_key(&format!("path-{}", MAX_STACK_FOLDER_SESSIONS)));
        assert_eq!(store.active_by_path.len(), store.sessions.len());
    }

    fn estimated_session_store_bytes_for_test(store: &StackFolderListingSessionStore) -> usize {
        store.estimated_bytes()
    }

    fn collect_synthetic_stack_folder_entries_for_test(
        count: usize,
        newest_first: bool,
    ) -> (Vec<StackFolderEntrySummary>, Vec<StackFolderWarning>) {
        let mut entries = synthetic_entry_summaries_for_test(count);
        let mut warnings = Vec::new();
        if entries.len() > MAX_FILESYSTEM_LISTING_ENTRIES {
            entries.truncate(MAX_FILESYSTEM_LISTING_ENTRIES);
            warnings.push(stack_folder_warning(
                Some(PathBuf::from("synthetic")),
                if newest_first {
                    format!(
                        "Stack folder filesystem discovery truncated at {MAX_FILESYSTEM_LISTING_ENTRIES} retained entries; retained/discovered total and global sort are capped and incomplete beyond the cap"
                    )
                } else {
                    format!(
                        "ZIP retained/discovered total and global sort are capped at {MAX_FILESYSTEM_LISTING_ENTRIES}; retained ordering is deterministic only within the retained slice"
                    )
                },
            ));
        }
        (entries, warnings)
    }

    struct SyntheticZipScanResult {
        raw_entries_scanned: usize,
        entries: Vec<StackFolderEntrySummary>,
        warnings: Vec<StackFolderWarning>,
    }

    fn collect_zip_folder_entries_from_synthetic_names_for_test(
        entries: impl IntoIterator<Item = (String, bool, u64)>,
        prefix: &str,
    ) -> Result<SyntheticZipScanResult, String> {
        let mut by_name = HashMap::<String, StackItem>::new();
        let mut raw_entries_scanned = 0usize;
        let mut warnings = Vec::new();
        for (name, is_dir, size) in entries {
            if raw_entries_scanned >= MAX_ZIP_RAW_ENTRIES_SCANNED {
                warnings.push(stack_folder_warning(
                    Some(PathBuf::from("synthetic.zip")),
                    format!(
                        "ZIP discovery truncated after {MAX_ZIP_RAW_ENTRIES_SCANNED} raw entries; retained/discovered total and global sort are capped and incomplete beyond the cap"
                    ),
                ));
                break;
            }
            raw_entries_scanned += 1;
            if name.len() > MAX_ZIP_ENTRY_NAME_BYTES {
                if !warnings
                    .iter()
                    .any(|warning| warning.message.contains("entry-name byte cap"))
                {
                    warnings.push(stack_folder_warning(
                        Some(PathBuf::from("synthetic.zip")),
                        format!(
                            "ZIP entries with names over the {MAX_ZIP_ENTRY_NAME_BYTES}-byte entry-name byte cap were skipped"
                        ),
                    ));
                }
                continue;
            }
            if let Some((child_name, child_is_dir)) = zip_child_entry(prefix, &name, is_dir) {
                let result = push_bounded_unique_zip_child(
                    &mut by_name,
                    Path::new("synthetic.zip"),
                    prefix,
                    child_name,
                    child_is_dir,
                    size,
                    &mut warnings,
                    Some(PathBuf::from("synthetic.zip")),
                );
                if matches!(result, super::ZipChildPushResult::RetainedCapReached) {
                    break;
                }
            }
        }
        let entries =
            finalize_zip_children(by_name, Path::new("synthetic.zip"), false, &mut warnings);
        Ok(SyntheticZipScanResult {
            raw_entries_scanned,
            entries,
            warnings,
        })
    }

    fn synthetic_entry_summaries_for_test(count: usize) -> Vec<StackFolderEntrySummary> {
        synthetic_named_entry_summaries_for_test(count, 8)
    }

    fn synthetic_named_entry_summaries_for_test(
        count: usize,
        name_bytes: usize,
    ) -> Vec<StackFolderEntrySummary> {
        (0..count)
            .map(|index| {
                let name = format!("{}-{index}", "x".repeat(name_bytes.max(1)));
                StackFolderEntrySummary {
                    name: name.clone(),
                    is_dir: false,
                    modified_at: None,
                    item: StackFolderEntryItem::Filesystem(PathBuf::from(name)),
                }
            })
            .collect()
    }

    fn test_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "jasonshell-stack-paging-{name}-{}",
            std::process::id()
        ))
    }
}
