use crate::shell_paths;
use crate::shell_windows::{STACK_POPUP_LABEL, TOP_BAR_LABEL};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State};

const PIN_STORE_FILE: &str = "stack-folders-v1.json";
const DEFAULT_PAGE_LIMIT: usize = 80;
const STACK_POPUP_WIDTH_LOGICAL: f64 = 980.0;
const STACK_POPUP_HEIGHT_RATIO: f64 = 0.35;
const EDGE_PADDING_PHYSICAL: i32 = 8;

#[derive(Default)]
pub struct StackPopupRuntimeState {
    latest_request: Option<ShowStackPopupRequest>,
    clipboard: Option<StackClipboard>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedStackFolder {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowStackPopupRequest {
    pub path: String,
    pub anchor_left: f64,
    pub anchor_width: f64,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackItem {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub type_label: String,
    pub icon_data_url: Option<String>,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<u64>,
    pub is_hidden: bool,
    pub is_readonly: bool,
    pub is_system: bool,
    pub is_symlink: bool,
    pub is_reparse_point: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFolderWarning {
    pub path: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFolderPage {
    pub path: String,
    pub items: Vec<StackItem>,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
    pub has_more: bool,
    pub warnings: Vec<StackFolderWarning>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackPasteResult {
    pub pasted: Vec<StackItem>,
    pub failures: Vec<StackPasteFailure>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackPasteFailure {
    pub path: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClipboardMode {
    Copy,
    Cut,
}

#[derive(Clone, Debug)]
struct StackClipboard {
    mode: ClipboardMode,
    paths: Vec<PathBuf>,
}

#[tauri::command]
pub fn list_pinned_stack_folders(app_handle: AppHandle) -> Result<Vec<PinnedStackFolder>, String> {
    load_pins_with_defaults(&app_handle)
}

#[tauri::command]
pub fn pin_stack_folder(
    app_handle: AppHandle,
    path: String,
) -> Result<Vec<PinnedStackFolder>, String> {
    let folder = pinned_folder_from_path(&path)?;
    let mut pins = load_pins_with_defaults(&app_handle)?;
    if !pins
        .iter()
        .any(|pin| pin.path.eq_ignore_ascii_case(&folder.path))
    {
        pins.push(folder);
        save_pins(&app_handle, &pins)?;
    }
    Ok(pins)
}

#[tauri::command]
pub fn unpin_stack_folder(
    app_handle: AppHandle,
    path: String,
) -> Result<Vec<PinnedStackFolder>, String> {
    let mut pins = load_pins_with_defaults(&app_handle)?;
    pins.retain(|pin| !paths_match_for_unpin(&pin.path, &path));
    save_pins(&app_handle, &pins)?;
    Ok(pins)
}

#[tauri::command]
pub fn reorder_pinned_stack_folders(
    app_handle: AppHandle,
    ordered_paths: Vec<String>,
) -> Result<Vec<PinnedStackFolder>, String> {
    let pins = load_pins_with_defaults(&app_handle)?;
    let pins = reorder_pins_by_paths(pins, &ordered_paths);
    save_pins(&app_handle, &pins)?;
    Ok(pins)
}

#[tauri::command]
pub fn show_stack_popup(
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    request: ShowStackPopupRequest,
) -> Result<(), String> {
    let request = normalize_show_stack_popup_request(request)?;
    store_latest_request(&state, request.clone());

    let popup = app_handle
        .get_webview_window(STACK_POPUP_LABEL)
        .ok_or_else(|| "Stack popup window is unavailable".to_string())?;
    let top = app_handle
        .get_webview_window(TOP_BAR_LABEL)
        .ok_or_else(|| "Top bar window is unavailable".to_string())?;
    let monitor = top
        .current_monitor()
        .map_err(|error| format!("Failed to inspect current monitor: {error}"))?
        .or_else(|| app_handle.primary_monitor().ok().flatten())
        .ok_or_else(|| "Primary monitor is unavailable".to_string())?;
    let scale_factor = monitor.scale_factor();
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let top_position = top
        .outer_position()
        .map_err(|error| format!("Failed to read top bar position: {error}"))?;
    let top_size = top
        .outer_size()
        .map_err(|error| format!("Failed to read top bar size: {error}"))?;

    let width = ((STACK_POPUP_WIDTH_LOGICAL * scale_factor).round() as u32).min(
        monitor_size
            .width
            .saturating_sub((EDGE_PADDING_PHYSICAL * 2) as u32),
    );
    let height = ((monitor_size.height as f64 * STACK_POPUP_HEIGHT_RATIO).round() as u32)
        .max((240.0 * scale_factor).round() as u32);
    let anchor_right = top_position.x
        + ((request.anchor_left + request.anchor_width) * scale_factor).round() as i32;
    let min_x = monitor_position.x + EDGE_PADDING_PHYSICAL;
    let max_x =
        monitor_position.x + monitor_size.width as i32 - width as i32 - EDGE_PADDING_PHYSICAL;
    let x = (anchor_right - width as i32).clamp(min_x, max_x.max(min_x));
    let y = top_position.y + top_size.height as i32;

    popup
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| format!("Failed to size the stack popup: {error}"))?;
    popup
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| format!("Failed to position the stack popup: {error}"))?;
    popup
        .show()
        .map_err(|error| format!("Failed to show the stack popup: {error}"))?;
    popup
        .set_focus()
        .map_err(|error| format!("Failed to focus the stack popup: {error}"))?;
    popup
        .emit("stack-popup:open", request)
        .map_err(|error| format!("Failed to publish stack popup path: {error}"))
}

#[tauri::command]
pub fn hide_stack_popup(app_handle: AppHandle) -> Result<(), String> {
    app_handle
        .get_webview_window(STACK_POPUP_LABEL)
        .ok_or_else(|| "Stack popup window is unavailable".to_string())?
        .hide()
        .map_err(|error| format!("Failed to hide the stack popup: {error}"))
}

#[tauri::command]
pub fn get_stack_popup_request(
    state: State<'_, Mutex<StackPopupRuntimeState>>,
) -> Result<Option<ShowStackPopupRequest>, String> {
    Ok(state
        .lock()
        .expect("stack popup runtime state is poisoned")
        .latest_request
        .clone())
}

#[tauri::command]
pub fn read_stack_folder(
    path: String,
    offset: usize,
    limit: Option<usize>,
) -> Result<StackFolderPage, String> {
    let folder = normalize_existing_dir(&path)?;
    read_stack_folder_page(&folder, offset, limit.unwrap_or(DEFAULT_PAGE_LIMIT))
}

#[tauri::command]
pub fn open_stack_item(path: String) -> Result<(), String> {
    shell_paths::open_shell_path(path)
}

#[tauri::command]
pub fn rename_stack_item(path: String, new_name: String) -> Result<StackItem, String> {
    let source = PathBuf::from(normalize_existing_path(&path)?);
    let new_name = validate_child_name(&new_name)?;
    let parent = source
        .parent()
        .ok_or_else(|| "Cannot rename a root path".to_string())?;
    let destination = parent.join(new_name);
    if destination.exists() {
        return Err("A file or folder with that name already exists".to_string());
    }
    fs::rename(&source, &destination)
        .map_err(|error| format!("Failed to rename stack item: {error}"))?;
    stack_item_from_path(destination)
}

#[tauri::command]
pub fn copy_stack_items(
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    paths: Vec<String>,
) -> Result<(), String> {
    set_stack_clipboard(&state, ClipboardMode::Copy, paths)
}

#[tauri::command]
pub fn cut_stack_items(
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    paths: Vec<String>,
) -> Result<(), String> {
    set_stack_clipboard(&state, ClipboardMode::Cut, paths)
}

#[tauri::command]
pub fn paste_stack_items(
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    destination: String,
) -> Result<StackPasteResult, String> {
    let destination = PathBuf::from(normalize_existing_dir(&destination)?);
    let used_internal_clipboard = state
        .lock()
        .expect("stack popup runtime state is poisoned")
        .clipboard
        .is_some();
    let clipboard = clipboard_for_paste(&state)?;
    let result = paste_clipboard_items(&clipboard, &destination);

    if matches!(clipboard.mode, ClipboardMode::Cut) {
        update_cut_clipboard_after_paste(&state, used_internal_clipboard, &result);
    }

    Ok(result)
}

fn update_cut_clipboard_after_paste(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    used_internal_clipboard: bool,
    result: &StackPasteResult,
) {
    if !used_internal_clipboard {
        return;
    }

    let mut state = state.lock().expect("stack popup runtime state is poisoned");
    if result.failures.is_empty() {
        state.clipboard = None;
    } else {
        state.clipboard = Some(StackClipboard {
            mode: ClipboardMode::Cut,
            paths: result
                .failures
                .iter()
                .map(|failure| PathBuf::from(&failure.path))
                .collect(),
        });
    }
}

fn clipboard_for_paste(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
) -> Result<StackClipboard, String> {
    if let Some(clipboard) = state
        .lock()
        .expect("stack popup runtime state is poisoned")
        .clipboard
        .clone()
    {
        return Ok(clipboard);
    }

    read_native_file_clipboard()?.ok_or_else(|| "Stack clipboard is empty".to_string())
}

fn paste_clipboard_items(clipboard: &StackClipboard, destination: &Path) -> StackPasteResult {
    let mut pasted = Vec::new();
    let mut failures = Vec::new();

    for source in &clipboard.paths {
        match paste_one_clipboard_item(clipboard.mode, source, destination) {
            Ok(item) => pasted.push(item),
            Err(message) => failures.push(StackPasteFailure {
                path: source.to_string_lossy().into_owned(),
                message,
            }),
        }
    }

    StackPasteResult { pasted, failures }
}

fn paste_one_clipboard_item(
    mode: ClipboardMode,
    source: &Path,
    destination: &Path,
) -> Result<StackItem, String> {
    ensure_paste_destination_allowed(source, destination)?;
    let target = available_destination_path(destination, source)?;
    match mode {
        ClipboardMode::Copy => copy_path(source, &target)?,
        ClipboardMode::Cut => move_path_with_fallback(source, &target)?,
    }
    stack_item_from_path(target)
}

fn store_latest_request(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    request: ShowStackPopupRequest,
) {
    state
        .lock()
        .expect("stack popup runtime state is poisoned")
        .latest_request = Some(request);
}

fn normalize_show_stack_popup_request(
    request: ShowStackPopupRequest,
) -> Result<ShowStackPopupRequest, String> {
    let path = normalize_existing_dir(&request.path)?;
    Ok(ShowStackPopupRequest { path, ..request })
}

fn set_stack_clipboard(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    mode: ClipboardMode,
    paths: Vec<String>,
) -> Result<(), String> {
    if paths.is_empty() {
        return Err("Select at least one stack item first".to_string());
    }
    let resolved = paths
        .iter()
        .map(|path| normalize_existing_path(path).map(PathBuf::from))
        .collect::<Result<Vec<_>, _>>()?;

    #[cfg(target_os = "windows")]
    set_native_file_clipboard(&resolved, mode)?;

    state
        .lock()
        .expect("stack popup runtime state is poisoned")
        .clipboard = Some(StackClipboard {
        mode,
        paths: resolved,
    });
    Ok(())
}

fn load_pins(app_handle: &AppHandle) -> Result<Vec<PinnedStackFolder>, String> {
    let Some(path) = pin_store_path(app_handle) else {
        return Ok(Vec::new());
    };
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes =
        fs::read(&path).map_err(|error| format!("Failed to read stack folder pins: {error}"))?;
    match serde_json::from_slice(&bytes) {
        Ok(pins) => Ok(pins),
        Err(error) => {
            backup_corrupt_pin_store(&path)?;
            eprintln!("Backed up corrupt stack folder pins after parse failure: {error}");
            Ok(Vec::new())
        }
    }
}

fn load_pins_with_defaults(app_handle: &AppHandle) -> Result<Vec<PinnedStackFolder>, String> {
    let store_exists = pin_store_path(app_handle)
        .map(|path| path.exists())
        .unwrap_or(false);
    let mut pins = load_pins(app_handle)?;
    if !store_exists {
        for default_folder in default_pinned_stack_folders() {
            if !pins
                .iter()
                .any(|pin| pin.path.eq_ignore_ascii_case(&default_folder.path))
            {
                pins.push(default_folder);
            }
        }
        if !pins.is_empty() {
            save_pins(app_handle, &pins)?;
        }
    }
    Ok(pins)
}

fn save_pins(app_handle: &AppHandle, pins: &[PinnedStackFolder]) -> Result<(), String> {
    let Some(path) = pin_store_path(app_handle) else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create stack pin directory: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(pins)
        .map_err(|error| format!("Failed to serialize stack folder pins: {error}"))?;
    write_file_atomic(&path, &bytes)
        .map_err(|error| format!("Failed to write stack folder pins: {error}"))
}

fn backup_corrupt_pin_store(path: &Path) -> Result<(), String> {
    let timestamp = UNIX_EPOCH
        .elapsed()
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let backup = path.with_extension(format!("json.corrupt-{timestamp}"));
    fs::rename(path, backup)
        .map_err(|error| format!("Failed to back up corrupt stack folder pins: {error}"))
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes)?;
    atomic_rename(&temp_path, path)
}

#[cfg(windows)]
fn atomic_rename(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: `source` and `destination` are NUL-terminated UTF-16 path buffers
    // that remain alive for the duration of the MoveFileExW call.
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(io::Error::other)
    }
}

#[cfg(not(windows))]
fn atomic_rename(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

fn reorder_pins_by_paths(
    pins: Vec<PinnedStackFolder>,
    ordered_paths: &[String],
) -> Vec<PinnedStackFolder> {
    let mut remaining = pins;
    let mut reordered = Vec::with_capacity(remaining.len());
    for path in ordered_paths {
        if let Some(index) = remaining
            .iter()
            .position(|pin| paths_match_for_unpin(&pin.path, path))
        {
            reordered.push(remaining.remove(index));
        }
    }
    reordered.extend(remaining);
    reordered
}

fn pin_store_path(app_handle: &AppHandle) -> Option<PathBuf> {
    app_handle
        .path()
        .app_local_data_dir()
        .ok()
        .map(|dir| dir.join(PIN_STORE_FILE))
}

fn pinned_folder_from_path(path: &str) -> Result<PinnedStackFolder, String> {
    let path = normalize_existing_dir(path)?;
    let name = Path::new(&path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(&path)
        .to_string();
    Ok(PinnedStackFolder {
        id: path.clone(),
        name,
        path,
    })
}

fn read_stack_folder_page(
    path: &str,
    offset: usize,
    limit: usize,
) -> Result<StackFolderPage, String> {
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

    let total = entries.len();
    let limit = limit.max(1);
    let page_entries = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let page_len = page_entries.len();
    let mut items = Vec::with_capacity(page_len);
    for entry in page_entries {
        match stack_item_from_path(entry.path.clone()) {
            Ok(item) => items.push(item),
            Err(message) => warnings.push(stack_folder_warning(Some(entry.path), message)),
        }
    }

    Ok(StackFolderPage {
        path: path.to_string(),
        has_more: offset + page_len < total,
        items,
        limit: page_len,
        offset,
        total,
        warnings,
    })
}

#[derive(Debug)]
struct StackFolderEntrySummary {
    path: PathBuf,
    name: String,
    is_dir: bool,
}

fn stack_folder_entry_summary(
    entry: fs::DirEntry,
) -> Result<StackFolderEntrySummary, (Option<PathBuf>, String)> {
    let path = entry.path();
    let file_type = entry.file_type().map_err(|error| {
        (Some(path.clone()), format!("Failed to inspect stack item: {error}"))
    })?;
    let is_dir = if file_type.is_dir() {
        true
    } else if file_type.is_symlink() {
        fs::metadata(&path).map(|metadata| metadata.is_dir()).unwrap_or(false)
    } else {
        false
    };
    let name = entry
        .file_name()
        .to_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    Ok(StackFolderEntrySummary {
        path,
        name,
        is_dir,
    })
}

fn folder_sort_rank(is_dir: bool) -> u8 {
    if is_dir {
        0
    } else {
        1
    }
}

fn stack_folder_warning(path: Option<PathBuf>, message: String) -> StackFolderWarning {
    StackFolderWarning {
        path: path.map(|path| path.to_string_lossy().into_owned()),
        message,
    }
}

fn stack_item_from_path(path: PathBuf) -> Result<StackItem, String> {
    let link_metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("Failed to inspect stack item: {error}"))?;
    let is_symlink = link_metadata.file_type().is_symlink();
    let is_reparse_point = metadata_is_reparse_point(&link_metadata);
    let target_metadata = if is_symlink || is_reparse_point {
        fs::metadata(&path).ok()
    } else {
        None
    };
    let metadata = target_metadata.as_ref().unwrap_or(&link_metadata);
    let is_dir = metadata.is_dir();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let modified_at = link_metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64);

    Ok(StackItem {
        path: path.to_string_lossy().into_owned(),
        type_label: type_label(&path, is_dir, is_symlink, is_reparse_point),
        icon_data_url: stack_item_icon_data_url(&path),
        size_bytes: (!is_dir).then_some(metadata.len()),
        modified_at,
        is_hidden: metadata_is_hidden(&link_metadata, &name),
        is_readonly: metadata.permissions().readonly(),
        is_system: metadata_is_system(&link_metadata),
        is_symlink,
        is_reparse_point,
        kind: if is_dir { "folder" } else { "file" }.to_string(),
        name,
    })
}

#[cfg(target_os = "windows")]
fn stack_item_icon_data_url(path: &Path) -> Option<String> {
    crate::task_windows::shell_file_icon_data_url(path).ok()
}

#[cfg(not(target_os = "windows"))]
fn stack_item_icon_data_url(_path: &Path) -> Option<String> {
    None
}

fn type_label(path: &Path, is_dir: bool, is_symlink: bool, is_reparse_point: bool) -> String {
    if is_symlink {
        return if is_dir {
            "Folder Symlink".to_string()
        } else {
            "File Symlink".to_string()
        };
    }
    if is_reparse_point {
        return if is_dir {
            "Reparse Folder".to_string()
        } else {
            "Reparse File".to_string()
        };
    }
    if is_dir {
        return "Folder".to_string();
    }
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| format!("{} File", extension.to_uppercase()))
        .unwrap_or_else(|| "File".to_string())
}

#[cfg(windows)]
fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT_BIT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT_BIT != 0
}

#[cfg(not(windows))]
fn metadata_is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn normalize_existing_dir(path: &str) -> Result<String, String> {
    let resolved = normalize_existing_path(path)?;
    if !Path::new(&resolved).is_dir() {
        return Err("Stack path is not a folder".to_string());
    }
    Ok(resolved)
}

fn normalize_existing_path(path: &str) -> Result<String, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Stack path is empty".to_string());
    }

    let candidate = normalize_stack_path_candidate(trimmed);

    let pathbuf =
        resolve_stack_alias_path(&candidate).unwrap_or_else(|| PathBuf::from(candidate.clone()));
    fs::canonicalize(&pathbuf)
        .map(|path| stack_display_path_string(&path.to_string_lossy()))
        .map_err(|error| format!("Failed to resolve stack path: {error}"))
}

fn stack_display_path_string(value: &str) -> String {
    let trimmed = value.trim();
    if let Some(rest) = trimmed.strip_prefix("\\\\?\\UNC\\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("\\??\\UNC\\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("\\\\?\\") {
        rest.to_string()
    } else if let Some(rest) = trimmed.strip_prefix("\\??\\") {
        rest.to_string()
    } else {
        trimmed.to_string()
    }
}

fn paths_match_for_unpin(pin_path: &str, requested_path: &str) -> bool {
    if let (Ok(pin), Ok(requested)) = (
        normalize_existing_path(pin_path),
        normalize_existing_path(requested_path),
    ) {
        return pin.eq_ignore_ascii_case(&requested);
    }

    raw_path_key(pin_path) == raw_path_key(requested_path)
}

fn raw_path_key(path: &str) -> String {
    normalize_stack_path_candidate(path)
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_lowercase()
}

fn normalize_stack_path_candidate(path: &str) -> String {
    let trimmed = path.trim().trim_matches('"');
    let mut candidate = file_uri_to_path(trimmed).unwrap_or_else(|| trimmed.to_string());

    // Strip common extended-path artifacts produced when constructing file:// URIs from
    // Windows canonical paths (which can include the "\\?\\" prefix). After
    // converting slashes, this can manifest as a leading "?/" or "?\\". Remove it.
    if candidate.starts_with("?\\") || candidate.starts_with("?/") {
        candidate = candidate[2..].to_string();
    }
    stack_display_path_string(&candidate)
}

fn file_uri_to_path(value: &str) -> Option<String> {
    if !value
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://"))
    {
        return None;
    }

    let rest = &value[7..];
    let (host, path) = if rest.starts_with('/') {
        ("", rest)
    } else {
        match rest.find('/') {
            Some(index) => (&rest[..index], &rest[index..]),
            None => (rest, ""),
        }
    };
    let host = percent_decode(host);
    let mut path = percent_decode(path);

    if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
        path = path.trim_start_matches('/').replace('/', "\\");
        return Some(if path.is_empty() {
            format!(r"\\{host}")
        } else {
            format!(r"\\{host}\{path}")
        });
    }

    #[cfg(windows)]
    {
        while path.starts_with('/') {
            path.remove(0);
        }
        Some(path.replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        Some(path)
    }
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    decoded.push(byte);
                    index += 3;
                    continue;
                }
            }
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[tauri::command]
pub fn delete_stack_item(path: String) -> Result<(), String> {
    let target = PathBuf::from(normalize_existing_path(&path)?);
    delete_path(&target)
}

#[tauri::command]
pub fn new_stack_folder(parent: String, name: String) -> Result<StackItem, String> {
    let parent = PathBuf::from(normalize_existing_dir(&parent)?);
    let name = validate_child_name(&name)?;
    let destination = parent.join(name);
    if destination.exists() {
        return Err("A file or folder with that name already exists".to_string());
    }
    fs::create_dir(&destination).map_err(|error| format!("Failed to create folder: {error}"))?;
    stack_item_from_path(destination)
}

#[tauri::command]
pub fn reveal_stack_item(path: String) -> Result<(), String> {
    let path = normalize_existing_path(&path)?;
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("explorer.exe")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to reveal stack item: {e}"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        shell_paths::open_shell_path(path)
    }
}

fn resolve_stack_alias_path(path: &str) -> Option<PathBuf> {
    let profile = user_profile_dir()?;
    resolve_stack_alias_with_profile(path, &profile)
}

fn user_profile_dir() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
}

fn resolve_stack_alias_with_profile(path: &str, profile: &Path) -> Option<PathBuf> {
    let alias = path.strip_prefix("shell:")?;
    if alias.eq_ignore_ascii_case("profile") {
        return Some(profile.to_path_buf());
    }
    if alias.eq_ignore_ascii_case("desktop") {
        return Some(profile.join("Desktop"));
    }
    if alias.eq_ignore_ascii_case("personal") || alias.eq_ignore_ascii_case("documents") {
        return Some(profile.join("Documents"));
    }
    if alias.eq_ignore_ascii_case("downloads") {
        return Some(profile.join("Downloads"));
    }
    None
}

fn default_pinned_stack_folders() -> Vec<PinnedStackFolder> {
    let Some(profile) = user_profile_dir() else {
        return Vec::new();
    };

    ["Desktop", "Downloads"]
        .iter()
        .filter_map(|name| profile.join(name).to_str().map(str::to_string))
        .filter_map(|path| pinned_folder_from_path(&path).ok())
        .collect()
}

fn validate_child_name(name: &str) -> Result<&str, String> {
    if name.trim().is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if name.contains('\\') || name.contains('/') {
        return Err("Name cannot contain path separators".to_string());
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err("Name cannot end with a dot or space".to_string());
    }
    if name.chars().any(|ch| ch.is_control()) {
        return Err("Name cannot contain control characters".to_string());
    }
    if name
        .chars()
        .any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Err("Name contains characters Windows does not allow".to_string());
    }
    let basename = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    if matches!(basename.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || reserved_numbered_device_name(&basename, "COM")
        || reserved_numbered_device_name(&basename, "LPT")
    {
        return Err("Name is reserved by Windows".to_string());
    }
    Ok(name)
}

fn reserved_numbered_device_name(name: &str, prefix: &str) -> bool {
    name.len() == 4
        && name.starts_with(prefix)
        && name
            .as_bytes()
            .get(3)
            .is_some_and(|digit| (b'1'..=b'9').contains(digit))
}

fn available_destination_path(destination: &Path, source: &Path) -> Result<PathBuf, String> {
    let file_name = source
        .file_name()
        .ok_or_else(|| "Stack item name is unavailable".to_string())?;
    let candidate = destination.join(file_name);
    if !candidate.exists() {
        return Ok(candidate);
    }
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Copy");
    let extension = source.extension().and_then(|value| value.to_str());
    for index in 1..1000 {
        let copy_name = match extension {
            Some(extension) => format!("{stem} - Copy ({index}).{extension}"),
            None => format!("{stem} - Copy ({index})"),
        };
        let candidate = destination.join(copy_name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err("Could not choose a paste destination name".to_string())
}

fn copy_path(source: &Path, destination: &Path) -> Result<(), String> {
    let mut visited = HashSet::new();
    copy_path_inner(source, destination, &mut visited)
}

fn move_path_with_fallback(source: &Path, destination: &Path) -> Result<(), String> {
    move_path_with_rename(source, destination, |source, destination| {
        fs::rename(source, destination)
    })
}

fn move_path_with_rename<F>(source: &Path, destination: &Path, rename: F) -> Result<(), String>
where
    F: FnOnce(&Path, &Path) -> io::Result<()>,
{
    match rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            copy_path(source, destination).map_err(|copy_error| {
                format!(
                    "Failed to move stack item: {rename_error}; fallback copy failed: {copy_error}"
                )
            })?;
            remove_after_move_copy(source).map_err(|delete_error| {
                format!("Failed to move stack item after fallback copy: {delete_error}")
            })
        }
    }
}

fn remove_after_move_copy(source: &Path) -> io::Result<()> {
    if source.is_dir() {
        fs::remove_dir_all(source)
    } else {
        fs::remove_file(source)
    }
}

#[cfg(test)]
fn copy_dir(source: &Path, destination: &Path) -> Result<(), String> {
    let mut visited = HashSet::new();
    copy_dir_inner(source, destination, &mut visited)
}

fn copy_path_inner(
    source: &Path,
    destination: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    ensure_paste_destination_allowed(source, destination)?;
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("Failed to inspect stack item before copy: {error}"))?;
    if metadata.file_type().is_symlink() || metadata_is_reparse_point(&metadata) {
        return Err(
            "Copying symbolic links or reparse points is not supported by Stack Browser yet"
                .to_string(),
        );
    }
    if metadata.is_dir() {
        copy_dir_inner(source, destination, visited)
    } else {
        fs::copy(source, destination)
            .map(|_| ())
            .map_err(|error| format!("Failed to copy stack item: {error}"))
    }
}

fn copy_dir_inner(
    source: &Path,
    destination: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    ensure_paste_destination_allowed(source, destination)?;
    let canonical_source = fs::canonicalize(source)
        .map_err(|error| format!("Failed to resolve source folder before copy: {error}"))?;
    if !visited.insert(canonical_source) {
        return Err("Cannot copy a folder cycle from Stack Browser".to_string());
    }
    fs::create_dir_all(destination)
        .map_err(|error| format!("Failed to create pasted folder: {error}"))?;
    for entry in fs::read_dir(source).map_err(|error| format!("Failed to copy folder: {error}"))? {
        let entry = entry.map_err(|error| format!("Failed to copy folder entry: {error}"))?;
        copy_path_inner(&entry.path(), &destination.join(entry.file_name()), visited)?;
    }
    Ok(())
}

fn ensure_paste_destination_allowed(source: &Path, destination: &Path) -> Result<(), String> {
    if is_real_directory(source) && path_starts_with(destination, source) {
        return Err("Cannot paste a folder into itself or one of its subfolders".to_string());
    }
    Ok(())
}

fn is_real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| {
            metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && !metadata_is_reparse_point(&metadata)
        })
        .unwrap_or(false)
}

fn path_starts_with(path: &Path, parent: &Path) -> bool {
    if path.starts_with(parent) {
        return true;
    }

    let parent = match fs::canonicalize(parent) {
        Ok(parent) => parent,
        Err(_) => return false,
    };
    let path = if path.exists() {
        fs::canonicalize(path).ok()
    } else {
        path.parent()
            .and_then(|parent| fs::canonicalize(parent).ok())
            .and_then(|parent| path.file_name().map(|name| parent.join(name)))
    };
    path.is_some_and(|path| path.starts_with(parent))
}

#[cfg(all(target_os = "windows", not(test)))]
fn delete_path(path: &Path) -> Result<(), String> {
    recycle_path(path)
}

#[cfg(any(not(target_os = "windows"), test))]
fn delete_path(path: &Path) -> Result<(), String> {
    permanent_delete_path(path)
}

#[cfg(any(not(target_os = "windows"), test))]
fn permanent_delete_path(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|error| format!("Failed to delete folder: {error}"))
    } else {
        fs::remove_file(path).map_err(|error| format!("Failed to delete file: {error}"))
    }
}

#[cfg(all(target_os = "windows", not(test)))]
fn recycle_path(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FO_DELETE,
        SHFILEOPSTRUCTW,
    };

    let mut from = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let mut operation = SHFILEOPSTRUCTW::default();
    operation.wFunc = FO_DELETE;
    operation.pFrom = PCWSTR(from.as_mut_ptr());
    operation.fFlags = (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI).0 as u16;

    // SAFETY: `operation` points at a valid SHFILEOPSTRUCTW for the duration of
    // the call, and `pFrom` references a double-NUL-terminated UTF-16 path list.
    let result = unsafe { SHFileOperationW(&mut operation) };
    if result != 0 {
        return Err(format!(
            "Failed to move stack item to Recycle Bin: shell error {result}"
        ));
    }
    if operation.fAnyOperationsAborted.as_bool() {
        return Err("Recycle Bin delete was cancelled".to_string());
    }
    Ok(())
}

fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.')
}

fn stack_file_attributes_from_bits(bits: u32) -> (bool, bool, bool) {
    const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x0000_0004;
    (
        bits & FILE_ATTRIBUTE_HIDDEN != 0,
        bits & FILE_ATTRIBUTE_READONLY != 0,
        bits & FILE_ATTRIBUTE_SYSTEM != 0,
    )
}

#[cfg(windows)]
fn metadata_file_attributes(metadata: &fs::Metadata) -> u32 {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes()
}

#[cfg(windows)]
fn metadata_is_hidden(metadata: &fs::Metadata, name: &str) -> bool {
    stack_file_attributes_from_bits(metadata_file_attributes(metadata)).0 || is_hidden_name(name)
}

#[cfg(not(windows))]
fn metadata_is_hidden(_metadata: &fs::Metadata, name: &str) -> bool {
    is_hidden_name(name)
}

#[cfg(windows)]
fn metadata_is_system(metadata: &fs::Metadata) -> bool {
    stack_file_attributes_from_bits(metadata_file_attributes(metadata)).2
}

#[cfg(not(windows))]
fn metadata_is_system(_metadata: &fs::Metadata) -> bool {
    false
}

fn clipboard_mode_from_drop_effect(effect: u32) -> ClipboardMode {
    const DROPEFFECT_MOVE: u32 = 0x2;

    if effect & DROPEFFECT_MOVE != 0 {
        ClipboardMode::Cut
    } else {
        ClipboardMode::Copy
    }
}

#[cfg(not(target_os = "windows"))]
fn read_native_file_clipboard() -> Result<Option<StackClipboard>, String> {
    Ok(None)
}

#[cfg(target_os = "windows")]
fn read_native_file_clipboard() -> Result<Option<StackClipboard>, String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HGLOBAL;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
        RegisterClipboardFormatW,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows::Win32::System::Ole::CF_HDROP;
    use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

    // SAFETY: Opening the process clipboard does not dereference raw pointers; it
    // establishes process-global clipboard access that is closed before return.
    unsafe { OpenClipboard(None).map_err(|error| format!("Failed to open clipboard: {error}"))? };

    // SAFETY: Clipboard handles returned by Win32 are checked before locking and
    // are only read while the clipboard is open. Buffers passed to DragQueryFileW
    // are sized from the API-reported UTF-16 length plus a terminating NUL.
    let result = (|| unsafe {
        if IsClipboardFormatAvailable(CF_HDROP.0 as u32).is_err() {
            return Ok(None);
        }

        let hdrop_handle = GetClipboardData(CF_HDROP.0 as u32)
            .map_err(|error| format!("Failed to read file clipboard data: {error}"))?;
        let hdrop = HDROP(hdrop_handle.0);
        let count = DragQueryFileW(hdrop, u32::MAX, None);
        if count == 0 {
            return Ok(None);
        }

        let mut paths = Vec::new();
        for index in 0..count {
            let len = DragQueryFileW(hdrop, index, None);
            if len == 0 {
                continue;
            }
            let mut buffer = vec![0u16; len as usize + 1];
            let written = DragQueryFileW(hdrop, index, Some(&mut buffer));
            if written > 0 {
                paths.push(PathBuf::from(OsString::from_wide(
                    &buffer[..written as usize],
                )));
            }
        }

        if paths.is_empty() {
            return Ok(None);
        }

        let effect_name = to_wide("Preferred DropEffect");
        let effect_format = RegisterClipboardFormatW(PCWSTR(effect_name.as_ptr()));
        let mode = if effect_format != 0 && IsClipboardFormatAvailable(effect_format).is_ok() {
            let effect_handle = GetClipboardData(effect_format)
                .map_err(|error| format!("Failed to read clipboard drop effect: {error}"))?;
            let memory = GlobalLock(HGLOBAL(effect_handle.0));
            if memory.is_null() {
                ClipboardMode::Copy
            } else {
                let effect = *(memory.cast::<u32>());
                GlobalUnlock(HGLOBAL(effect_handle.0)).ok();
                clipboard_mode_from_drop_effect(effect)
            }
        } else {
            ClipboardMode::Copy
        };

        Ok(Some(StackClipboard { mode, paths }))
    })();

    // SAFETY: Balances the successful OpenClipboard call above.
    unsafe { CloseClipboard().map_err(|error| format!("Failed to close clipboard: {error}"))? };
    result
}

#[cfg(target_os = "windows")]
fn set_native_file_clipboard(paths: &[PathBuf], mode: ClipboardMode) -> Result<(), String> {
    use std::mem::size_of;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HANDLE, POINT};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::CF_HDROP;
    use windows::Win32::UI::Shell::DROPFILES;

    let mut encoded_paths = Vec::<u16>::new();
    for path in paths {
        encoded_paths.extend(path.as_os_str().encode_wide());
        encoded_paths.push(0);
    }
    encoded_paths.push(0);

    let dropfiles_size = size_of::<DROPFILES>();
    let paths_size = encoded_paths.len() * size_of::<u16>();
    // SAFETY: Allocates a movable global memory block large enough for the
    // DROPFILES header plus the double-NUL-terminated UTF-16 path list, locks it,
    // writes initialized bytes, then unlocks before transferring ownership to the clipboard.
    let hdrop = unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, dropfiles_size + paths_size)
            .map_err(|error| format!("Failed to allocate clipboard memory: {error}"))?;
        let memory = GlobalLock(handle);
        if memory.is_null() {
            return Err("Failed to lock clipboard memory".to_string());
        }
        let dropfiles = memory.cast::<DROPFILES>();
        *dropfiles = DROPFILES {
            pFiles: dropfiles_size as u32,
            pt: POINT { x: 0, y: 0 },
            fNC: false.into(),
            fWide: true.into(),
        };
        std::ptr::copy_nonoverlapping(
            encoded_paths.as_ptr(),
            memory.add(dropfiles_size).cast::<u16>(),
            encoded_paths.len(),
        );
        GlobalUnlock(handle).ok();
        handle
    };

    // SAFETY: Allocates a movable global memory block for one u32 drop-effect
    // value, writes the initialized value while locked, then unlocks before
    // transferring ownership to the clipboard.
    let effect_handle = unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, size_of::<u32>())
            .map_err(|error| format!("Failed to allocate clipboard effect memory: {error}"))?;
        let memory = GlobalLock(handle);
        if memory.is_null() {
            return Err("Failed to lock clipboard effect memory".to_string());
        }
        let effect = match mode {
            ClipboardMode::Copy => 1u32,
            ClipboardMode::Cut => 2u32,
        };
        *(memory.cast::<u32>()) = effect;
        GlobalUnlock(handle).ok();
        handle
    };

    let format_name = to_wide("Preferred DropEffect");
    // SAFETY: Opens the process clipboard, publishes ownership of the allocated
    // HGLOBAL handles with SetClipboardData, then closes the clipboard before return.
    unsafe {
        OpenClipboard(None).map_err(|error| format!("Failed to open clipboard: {error}"))?;
        EmptyClipboard().map_err(|error| format!("Failed to empty clipboard: {error}"))?;
        SetClipboardData(CF_HDROP.0 as u32, Some(HANDLE(hdrop.0)))
            .map_err(|error| format!("Failed to set file clipboard data: {error}"))?;
        let effect_format = RegisterClipboardFormatW(PCWSTR(format_name.as_ptr()));
        if effect_format != 0 {
            SetClipboardData(effect_format, Some(HANDLE(effect_handle.0)))
                .map_err(|error| format!("Failed to set clipboard drop effect: {error}"))?;
        }
        CloseClipboard().map_err(|error| format!("Failed to close clipboard: {error}"))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn to_wide(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        available_destination_path, backup_corrupt_pin_store, clipboard_mode_from_drop_effect,
        copy_dir, move_path_with_rename, paste_clipboard_items, paths_match_for_unpin,
        read_stack_folder_page, reorder_pins_by_paths, resolve_stack_alias_with_profile,
        stack_file_attributes_from_bits, stack_folder_warning, stack_item_from_path,
        validate_child_name, ClipboardMode, PinnedStackFolder, ShowStackPopupRequest,
        StackClipboard, StackItem,
    };
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    #[test]
    fn rejects_invalid_rename_child_names() {
        assert!(validate_child_name("").is_err());
        assert!(validate_child_name("a\\b").is_err());
        assert!(validate_child_name("a/b").is_err());
        assert!(validate_child_name("bad:name").is_err());
        assert!(validate_child_name("bad*name").is_err());
        assert!(validate_child_name("bad\u{0001}name").is_err());
        assert!(validate_child_name("Name.").is_err());
        assert!(validate_child_name("Name ").is_err());
        assert!(validate_child_name("CON").is_err());
        assert!(validate_child_name("con.txt").is_err());
        assert!(validate_child_name("COM1").is_err());
        assert!(validate_child_name("LPT9.log").is_err());
        assert_eq!(validate_child_name("Notes.txt").unwrap(), "Notes.txt");
        assert_eq!(
            validate_child_name("Project Notes.txt").unwrap(),
            "Project Notes.txt"
        );
    }

    #[test]
    fn reads_folder_details_with_folders_first() {
        let root = test_dir("read-folder");
        fs::create_dir_all(root.join("Folder")).unwrap();
        fs::write(root.join("alpha.txt"), b"hello").unwrap();

        let page = read_stack_folder_page(root.to_str().unwrap(), 0, 10).unwrap();

        assert_eq!(page.total, 2);
        assert_eq!(page.items[0].kind, "folder");
        assert_eq!(page.items[1].name, "alpha.txt");
        assert!(page.warnings.is_empty());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn serializes_stack_item_icon_data_url_for_frontend() {
        let item = StackItem {
            path: r"C:\Items\app.exe".to_string(),
            name: "app.exe".to_string(),
            kind: "file".to_string(),
            type_label: "EXE File".to_string(),
            icon_data_url: Some("data:image/png;base64,icon".to_string()),
            size_bytes: Some(1),
            modified_at: None,
            is_hidden: false,
            is_readonly: false,
            is_system: false,
            is_symlink: false,
            is_reparse_point: false,
        };

        let serialized = serde_json::to_value(item).unwrap();

        assert_eq!(serialized["iconDataUrl"], "data:image/png;base64,icon");
    }

    #[test]
    fn normalizes_show_stack_popup_request_before_delivery() {
        let root = test_dir("show-request");
        fs::create_dir_all(&root).unwrap();

        let request = ShowStackPopupRequest {
            path: root.to_string_lossy().into_owned(),
            anchor_left: 12.0,
            anchor_width: 34.0,
            request_id: Some("open-1".to_string()),
        };
        let normalized = super::normalize_show_stack_popup_request(request).unwrap();

        assert_eq!(
            fs::canonicalize(&normalized.path).unwrap(),
            fs::canonicalize(&root).unwrap()
        );
        assert_eq!(normalized.anchor_left, 12.0);
        assert_eq!(normalized.anchor_width, 34.0);
        assert_eq!(normalized.request_id.as_deref(), Some("open-1"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn deserializes_legacy_show_stack_popup_request_without_request_id() {
        let request: ShowStackPopupRequest = serde_json::from_value(serde_json::json!({
            "path": r"C:\Pins\Docs",
            "anchorLeft": 12.0,
            "anchorWidth": 34.0
        }))
        .unwrap();

        assert_eq!(request.request_id, None);
    }

    #[test]
    fn paginates_large_stack_folders_without_truncating_metadata() {
        let root = test_dir("large-folder");
        fs::create_dir_all(&root).unwrap();
        for index in 0..505 {
            fs::write(root.join(format!("file-{index:03}.txt")), b"x").unwrap();
        }

        let first_page = read_stack_folder_page(root.to_str().unwrap(), 0, 500).unwrap();
        let second_page = read_stack_folder_page(root.to_str().unwrap(), 500, 500).unwrap();

        assert_eq!(first_page.total, 505);
        assert_eq!(first_page.items.len(), 500);
        assert!(first_page.has_more);
        assert_eq!(second_page.items.len(), 5);
        assert!(!second_page.has_more);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn preserves_original_symlink_paths_when_materializing_page_items() {
        let root = test_dir("symlink-page-items");
        let target = root.join("target-folder");
        let live_link = root.join("Folder Alias");
        let broken_link = root.join("Broken Alias");
        fs::create_dir_all(&target).unwrap();

        if create_dir_symlink(&target, &live_link).is_err()
            || create_dir_symlink(&root.join("missing-target"), &broken_link).is_err()
        {
            fs::remove_dir_all(root).ok();
            return;
        }

        let page = read_stack_folder_page(root.to_str().unwrap(), 0, 10).unwrap();
        let live_item = page
            .items
            .iter()
            .find(|item| item.name == "Folder Alias")
            .unwrap();
        let broken_item = page
            .items
            .iter()
            .find(|item| item.name == "Broken Alias")
            .unwrap();

        assert_eq!(PathBuf::from(&live_item.path), live_link);
        assert_eq!(live_item.name, "Folder Alias");
        assert_eq!(live_item.kind, "folder");
        assert!(live_item.is_symlink || live_item.is_reparse_point);

        assert_eq!(PathBuf::from(&broken_item.path), broken_link);
        assert_eq!(broken_item.name, "Broken Alias");
        assert!(broken_item.is_symlink || broken_item.is_reparse_point);
        assert!(page.warnings.is_empty());

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn builds_partial_listing_warnings_with_optional_paths() {
        let warning = stack_folder_warning(
            Some(PathBuf::from(r"C:\missing\child")),
            "denied".to_string(),
        );

        assert_eq!(warning.path.as_deref(), Some(r"C:\missing\child"));
        assert_eq!(warning.message, "denied");
    }

    #[test]
    fn chooses_copy_destination_when_name_exists() {
        let root = test_dir("copy-destination");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("plan.txt"), b"one").unwrap();
        let next = available_destination_path(&root, Path::new("plan.txt")).unwrap();

        assert!(next.ends_with("plan - Copy (1).txt"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn rejects_copying_folder_into_itself_or_descendant() {
        let root = test_dir("self-copy");
        let source = root.join("A");
        fs::create_dir_all(source.join("child")).unwrap();

        let result = copy_dir(&source, &source.join("A"));

        assert!(result.is_err());
        assert!(!source.join("A").exists());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn refuses_to_copy_symlink_directories_when_supported() {
        let root = test_dir("symlink-copy");
        let target = root.join("target");
        let link = root.join("link");
        let destination = root.join("destination");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("child.txt"), b"x").unwrap();

        if create_dir_symlink(&target, &link).is_err() {
            fs::remove_dir_all(root).ok();
            return;
        }

        let item = stack_item_from_path(link.clone()).unwrap();
        assert!(item.is_symlink || item.is_reparse_point);
        let result = super::copy_path(&link, &destination);

        assert!(result.is_err());
        assert!(!destination.exists());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn clipboard_mode_debug_labels_remain_stable() {
        assert_eq!(format!("{:?}", ClipboardMode::Copy), "Copy");
        assert_eq!(format!("{:?}", ClipboardMode::Cut), "Cut");
    }

    #[test]
    fn maps_preferred_drop_effect_to_clipboard_mode() {
        assert_eq!(clipboard_mode_from_drop_effect(0), ClipboardMode::Copy);
        assert_eq!(clipboard_mode_from_drop_effect(1), ClipboardMode::Copy);
        assert_eq!(clipboard_mode_from_drop_effect(2), ClipboardMode::Cut);
        assert_eq!(clipboard_mode_from_drop_effect(3), ClipboardMode::Cut);
    }

    #[test]
    fn move_fallback_copies_then_deletes_source() {
        let root = test_dir("move-fallback");
        let source = root.join("source.txt");
        let destination = root.join("destination.txt");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"moved").unwrap();

        move_path_with_rename(&source, &destination, |_source, _destination| {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "forced rename failure",
            ))
        })
        .unwrap();

        assert!(!source.exists());
        assert_eq!(fs::read(&destination).unwrap(), b"moved");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn paste_result_preserves_successes_and_reports_failures() {
        let root = test_dir("partial-paste");
        let source_dir = root.join("source");
        let destination = root.join("destination");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&destination).unwrap();
        let good = source_dir.join("good.txt");
        let missing = source_dir.join("missing.txt");
        fs::write(&good, b"ok").unwrap();

        let clipboard = StackClipboard {
            mode: ClipboardMode::Copy,
            paths: vec![good, missing.clone()],
        };
        let result = paste_clipboard_items(&clipboard, &destination);

        assert_eq!(result.pasted.len(), 1);
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].path, missing.to_string_lossy());
        assert!(destination.join("good.txt").exists());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn resolves_supported_shell_aliases_for_pinning() {
        let profile = Path::new(r"C:\Users\tester");
        assert_eq!(
            resolve_stack_alias_with_profile("shell:Profile", profile).unwrap(),
            PathBuf::from(r"C:\Users\tester")
        );
        assert_eq!(
            resolve_stack_alias_with_profile("shell:Desktop", profile).unwrap(),
            PathBuf::from(r"C:\Users\tester\Desktop")
        );
        assert_eq!(
            resolve_stack_alias_with_profile("shell:Personal", profile).unwrap(),
            PathBuf::from(r"C:\Users\tester\Documents")
        );
        assert_eq!(
            resolve_stack_alias_with_profile("shell:Downloads", profile).unwrap(),
            PathBuf::from(r"C:\Users\tester\Downloads")
        );
        assert!(resolve_stack_alias_with_profile("shell:Unknown", profile).is_none());
    }

    #[test]
    fn maps_windows_file_attribute_bits() {
        assert_eq!(stack_file_attributes_from_bits(0), (false, false, false));
        assert_eq!(stack_file_attributes_from_bits(0x2), (true, false, false));
        assert_eq!(
            stack_file_attributes_from_bits(0x1 | 0x4),
            (false, true, true)
        );
    }

    #[test]
    fn stack_item_reports_readonly_metadata() {
        let root = test_dir("readonly-metadata");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("locked.txt");
        fs::write(&file, b"locked").unwrap();
        let mut permissions = fs::metadata(&file).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&file, permissions).unwrap();

        let item = stack_item_from_path(file.clone()).unwrap();

        assert!(item.is_readonly);
        let mut permissions = fs::metadata(&file).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&file, permissions).ok();
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn reorders_pins_by_requested_paths_and_keeps_unspecified_tail() {
        let pins = vec![
            test_pin("One", r"C:\Pins\One"),
            test_pin("Two", r"C:\Pins\Two"),
            test_pin("Three", r"C:\Pins\Three"),
        ];

        let reordered = reorder_pins_by_paths(
            pins,
            &[r"C:\Pins\Three".to_string(), r"C:\Pins\One".to_string()],
        );

        assert_eq!(
            reordered
                .iter()
                .map(|pin| pin.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Three", "One", "Two"]
        );
    }

    #[test]
    fn backs_up_corrupt_pin_store_file() {
        let root = test_dir("corrupt-pins");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("stack-folders-v1.json");
        fs::write(&path, b"not json").unwrap();

        backup_corrupt_pin_store(&path).unwrap();

        assert!(!path.exists());
        let backups = fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("stack-folders-v1.json.corrupt-")
            })
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        fs::remove_dir_all(root).ok();
    }

    fn test_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "jasonshell-stack-popup-{name}-{}",
            std::process::id()
        ))
    }

    fn test_pin(name: &str, path: &str) -> PinnedStackFolder {
        PinnedStackFolder {
            id: path.to_string(),
            name: name.to_string(),
            path: path.to_string(),
        }
    }

    #[cfg(windows)]
    fn platform_path(path: &str) -> String {
        path.to_string()
    }

    #[cfg(not(windows))]
    fn platform_path(path: &str) -> String {
        path.replace('\\', "/")
    }

    #[cfg(unix)]
    fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_dir_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[test]
    fn normalize_file_uri_paths() {
        let file = test_dir("file-uri").join("example.txt");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"ok").unwrap();
        let canonical = fs::canonicalize(&file).unwrap();
        let uri = format!("file:///{}", canonical.to_string_lossy().replace('\\', "/"));
        let resolved = super::normalize_existing_path(&uri).unwrap();
        assert_eq!(fs::canonicalize(resolved).unwrap(), canonical);
        fs::remove_file(&file).ok();
    }

    #[test]
    fn normalizes_file_uri_candidate_forms() {
        assert_eq!(
            super::normalize_stack_path_candidate("file:///C:/Dev/My%20Repo"),
            platform_path(r"C:\Dev\My Repo")
        );
        assert_eq!(
            super::normalize_stack_path_candidate("file://localhost/C:/Dev/Repo"),
            platform_path(r"C:\Dev\Repo")
        );
        assert_eq!(
            super::normalize_stack_path_candidate("file://server/share/My%20Repo"),
            r"\\server\share\My Repo"
        );
    }

    #[test]
    fn strips_extended_windows_prefixes_from_stack_paths() {
        assert_eq!(
            super::stack_display_path_string(r"\\?\C:\Dev\Repo"),
            platform_path(r"C:\Dev\Repo")
        );
        assert_eq!(
            super::stack_display_path_string(r"\\?\UNC\server\share\Repo"),
            r"\\server\share\Repo"
        );
        assert_eq!(
            super::normalize_stack_path_candidate(r"\\?\C:\Dev\Repo"),
            platform_path(r"C:\Dev\Repo")
        );
    }

    #[test]
    fn matches_stale_pins_by_raw_normalized_path() {
        assert!(paths_match_for_unpin(
            r"C:\Offline\Stack",
            r"file:///C:/Offline/Stack"
        ));
        assert!(paths_match_for_unpin(
            r"\\server\share\Stack",
            r"file://server/share/Stack"
        ));
        assert!(!paths_match_for_unpin(
            r"C:\Offline\Stack",
            r"C:\Offline\Other"
        ));
    }

    #[test]
    fn creates_new_folder() {
        let root = test_dir("new-folder");
        fs::create_dir_all(&root).unwrap();
        let folder_name = "SubFolder";
        let item =
            super::new_stack_folder(root.to_str().unwrap().to_string(), folder_name.to_string())
                .unwrap();
        assert_eq!(item.name, folder_name);
        assert!(Path::new(&item.path).exists());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn deletes_file_and_folder() {
        let root = test_dir("delete-test");
        fs::create_dir_all(root.join("Folder")).unwrap();
        fs::write(root.join("File.txt"), b"x").unwrap();
        super::delete_stack_item(root.join("File.txt").to_str().unwrap().to_string()).unwrap();
        assert!(!root.join("File.txt").exists());
        super::delete_stack_item(root.join("Folder").to_str().unwrap().to_string()).unwrap();
        assert!(!root.join("Folder").exists());
        fs::remove_dir_all(root).ok();
    }
}
