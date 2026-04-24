use crate::shell_paths;
use crate::shell_windows::{STACK_POPUP_LABEL, TOP_BAR_LABEL};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State};

const PIN_STORE_FILE: &str = "stack-folders-v1.json";
const DEFAULT_PAGE_LIMIT: usize = 80;
const STACK_POPUP_WIDTH_LOGICAL: f64 = 760.0;
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackItem {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub type_label: String,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<u64>,
    pub is_hidden: bool,
    pub is_readonly: bool,
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
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackPasteResult {
    pub pasted: Vec<StackItem>,
}

#[derive(Clone, Copy, Debug)]
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
pub fn pin_stack_folder(app_handle: AppHandle, path: String) -> Result<PinnedStackFolder, String> {
    let folder = pinned_folder_from_path(&path)?;
    let mut pins = load_pins_with_defaults(&app_handle)?;
    if !pins
        .iter()
        .any(|pin| pin.path.eq_ignore_ascii_case(&folder.path))
    {
        pins.push(folder.clone());
        save_pins(&app_handle, &pins)?;
    }
    Ok(folder)
}

#[tauri::command]
pub fn unpin_stack_folder(app_handle: AppHandle, path: String) -> Result<(), String> {
    let requested = normalize_existing_dir(&path)?;
    let mut pins = load_pins_with_defaults(&app_handle)?;
    pins.retain(|pin| !pin.path.eq_ignore_ascii_case(&requested));
    save_pins(&app_handle, &pins)
}

#[tauri::command]
pub fn show_stack_popup(
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    request: ShowStackPopupRequest,
) -> Result<(), String> {
    let path = normalize_existing_dir(&request.path)?;
    let request = ShowStackPopupRequest { path, ..request };
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
    let clipboard = state
        .lock()
        .expect("stack popup runtime state is poisoned")
        .clipboard
        .clone()
        .ok_or_else(|| "Stack clipboard is empty".to_string())?;
    let mut pasted = Vec::new();

    for source in &clipboard.paths {
        ensure_paste_destination_allowed(source, &destination)?;
        let target = available_destination_path(&destination, source)?;
        match clipboard.mode {
            ClipboardMode::Copy => copy_path(source, &target)?,
            ClipboardMode::Cut => fs::rename(source, &target)
                .map_err(|error| format!("Failed to move stack item: {error}"))?,
        }
        pasted.push(stack_item_from_path(target)?);
    }

    if matches!(clipboard.mode, ClipboardMode::Cut) {
        state
            .lock()
            .expect("stack popup runtime state is poisoned")
            .clipboard = None;
    }

    Ok(StackPasteResult { pasted })
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
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("Failed to parse stack folder pins: {error}"))
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
    fs::write(path, bytes).map_err(|error| format!("Failed to write stack folder pins: {error}"))
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
    let mut items = fs::read_dir(path)
        .map_err(|error| format!("Failed to read stack folder: {error}"))?
        .filter_map(Result::ok)
        .filter_map(|entry| stack_item_from_path(entry.path()).ok())
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    let folders_first = items
        .into_iter()
        .partition::<Vec<_>, _>(|item| item.kind == "folder");
    let all_items = folders_first
        .0
        .into_iter()
        .chain(folders_first.1)
        .collect::<Vec<_>>();
    let total = all_items.len();
    let limit = limit.max(1);
    let items = all_items
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    Ok(StackFolderPage {
        path: path.to_string(),
        has_more: offset + items.len() < total,
        items,
        limit,
        offset,
        total,
    })
}

fn stack_item_from_path(path: PathBuf) -> Result<StackItem, String> {
    let metadata =
        fs::metadata(&path).map_err(|error| format!("Failed to inspect stack item: {error}"))?;
    let is_dir = metadata.is_dir();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64);

    Ok(StackItem {
        path: path.to_string_lossy().into_owned(),
        type_label: type_label(&path, is_dir),
        size_bytes: (!is_dir).then_some(metadata.len()),
        modified_at,
        is_hidden: is_hidden_name(&name),
        is_readonly: metadata.permissions().readonly(),
        kind: if is_dir { "folder" } else { "file" }.to_string(),
        name,
    })
}

fn type_label(path: &Path, is_dir: bool) -> String {
    if is_dir {
        return "Folder".to_string();
    }
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| format!("{} File", extension.to_uppercase()))
        .unwrap_or_else(|| "File".to_string())
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

    // Support file:// URIs often sent by drag/drop from external sources.
    let candidate = if let Some(stripped) = trimmed.strip_prefix("file://") {
        // Remove leading slashes or optional host components (file:///C:/... or file://localhost/C:/...)
        let mut s = stripped;
        while s.starts_with('/') {
            s = &s[1..];
        }
        // On Windows, convert forward slashes to backslashes.
        #[cfg(windows)]
        let s = s.replace('/', "\\");
        #[cfg(not(windows))]
        let s = s.to_string();
        s
    } else {
        trimmed.to_string()
    };

    // Strip common extended-path artifacts produced when constructing file:// URIs from
    // Windows canonical paths (which can include the "\\?\\" prefix). After
    // converting slashes, this can manifest as a leading "?/" or "?\\". Remove it.
    let candidate = if candidate.starts_with("?\\") || candidate.starts_with("?/") {
        candidate[2..].to_string()
    } else {
        candidate
    };

    let pathbuf = resolve_stack_alias_path(&candidate).unwrap_or_else(|| PathBuf::from(candidate.clone()));
    fs::canonicalize(&pathbuf)
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| format!("Failed to resolve stack path: {error}"))
}

#[tauri::command]
pub fn delete_stack_item(path: String) -> Result<(), String> {
    let target = PathBuf::from(normalize_existing_path(&path)?);
    if target.is_dir() {
        fs::remove_dir_all(&target)
            .map_err(|error| format!("Failed to delete folder: {error}"))?;
    } else {
        fs::remove_file(&target).map_err(|error| format!("Failed to delete file: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn new_stack_folder(parent: String, name: String) -> Result<StackItem, String> {
    let parent = PathBuf::from(normalize_existing_dir(&parent)?);
    let name = validate_child_name(&name)?;
    let destination = parent.join(name);
    if destination.exists() {
        return Err("A file or folder with that name already exists".to_string());
    }
    fs::create_dir(&destination)
        .map_err(|error| format!("Failed to create folder: {error}"))?;
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
    let name = name.trim();
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if name.contains('\\') || name.contains('/') {
        return Err("Name cannot contain path separators".to_string());
    }
    Ok(name)
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
    ensure_paste_destination_allowed(source, destination)?;
    if source.is_dir() {
        copy_dir(source, destination)
    } else {
        fs::copy(source, destination)
            .map(|_| ())
            .map_err(|error| format!("Failed to copy stack item: {error}"))
    }
}

fn copy_dir(source: &Path, destination: &Path) -> Result<(), String> {
    ensure_paste_destination_allowed(source, destination)?;
    fs::create_dir_all(destination)
        .map_err(|error| format!("Failed to create pasted folder: {error}"))?;
    for entry in fs::read_dir(source).map_err(|error| format!("Failed to copy folder: {error}"))? {
        let entry = entry.map_err(|error| format!("Failed to copy folder entry: {error}"))?;
        copy_path(&entry.path(), &destination.join(entry.file_name()))?;
    }
    Ok(())
}

fn ensure_paste_destination_allowed(source: &Path, destination: &Path) -> Result<(), String> {
    if source.is_dir() && destination.starts_with(source) {
        return Err("Cannot paste a folder into itself or one of its subfolders".to_string());
    }
    Ok(())
}

fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.')
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

    let effect_handle = unsafe {
        let handle = GlobalAlloc(GMEM_MOVEABLE, size_of::<u32>())
            .map_err(|error| format!("Failed to allocate clipboard effect memory: {error}"))?;
        let memory = GlobalLock(handle);
        if memory.is_null() {
            return Err("Failed to lock clipboard effect memory".to_string());
        }
        let effect = match mode {
            ClipboardMode::Copy => 5u32,
            ClipboardMode::Cut => 2u32,
        };
        *(memory.cast::<u32>()) = effect;
        GlobalUnlock(handle).ok();
        handle
    };

    let format_name = to_wide("Preferred DropEffect");
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
        available_destination_path, copy_dir, read_stack_folder_page, resolve_stack_alias_with_profile,
        validate_child_name, ClipboardMode,
    };
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn rejects_invalid_rename_child_names() {
        assert!(validate_child_name("").is_err());
        assert!(validate_child_name("a\\b").is_err());
        assert_eq!(validate_child_name("Notes.txt").unwrap(), "Notes.txt");
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
        fs::remove_dir_all(root).ok();
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
    fn clipboard_mode_debug_labels_remain_stable() {
        assert_eq!(format!("{:?}", ClipboardMode::Copy), "Copy");
        assert_eq!(format!("{:?}", ClipboardMode::Cut), "Cut");
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

    fn test_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "jasonshell-stack-popup-{name}-{}",
            std::process::id()
        ))
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
    fn creates_new_folder() {
        let root = test_dir("new-folder");
        fs::create_dir_all(&root).unwrap();
        let folder_name = "SubFolder";
        let item = super::new_stack_folder(root.to_str().unwrap().to_string(), folder_name.to_string()).unwrap();
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
