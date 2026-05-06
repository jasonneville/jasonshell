mod clipboard;
mod file_ops;
mod icons;
mod items;
mod models;
mod native_drag;
mod open_with;
mod paging;
mod paths;
mod pins;
mod popup_window;

use crate::shell_paths;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackPathSuggestion {
    pub name: String,
    pub path: String,
}

pub use models::{
    PinnedStackFolder, ShowStackPopupRequest, StackFolderPage, StackItem,
    StackItemIconResolutionBatch, StackNativeDragPreparation, StackOpenWithCandidate,
    StackPasteResult, StackPopupLogicalSize, StackPopupRuntimeState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchiveKind {
    Zip,
    Rar,
}

impl ArchiveKind {
    pub(crate) fn from_path(path: &Path) -> Option<Self> {
        if !path.is_file() {
            return None;
        }
        match path.extension().and_then(|value| value.to_str()).map(str::to_ascii_lowercase).as_deref() {
            Some("zip") => Some(Self::Zip),
            Some("rar") => Some(Self::Rar),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ArchiveDestinationMode {
    Here,
    Folder,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ArchiveExtractor {
    Builtin,
    SevenZip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchiveExtractionPlan {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub destination_path: PathBuf,
    pub expected_created_folder: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StackItemPropertiesPlan {
    pub executable: PathBuf,
    pub args: Vec<String>,
}

pub(crate) fn build_stack_item_properties_plan(path: &Path) -> Result<StackItemPropertiesPlan, String> {
    if !path.exists() {
        return Err("Path unavailable".to_string());
    }
    Ok(StackItemPropertiesPlan {
        executable: PathBuf::from("powershell.exe"),
        args: vec![
            "-NoProfile".to_string(),
            "-Command".to_string(),
            "$item = Get-Item -LiteralPath $args[0]; $shell = New-Object -ComObject Shell.Application; $folder = $shell.Namespace($item.DirectoryName); $folder.ParseName($item.Name).InvokeVerb('properties')".to_string(),
            path.to_string_lossy().to_string(),
        ],
    })
}

pub(crate) fn seven_zip_discovery_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        candidates.push(PathBuf::from(program_files).join("7-Zip").join("7z.exe"));
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        candidates.push(PathBuf::from(program_files_x86).join("7-Zip").join("7z.exe"));
    }
    candidates.push(PathBuf::from("7z.exe"));
    candidates
}

fn find_seven_zip() -> Option<PathBuf> {
    seven_zip_discovery_candidates()
        .into_iter()
        .find(|candidate| candidate.file_name().and_then(|name| name.to_str()) == Some("7z.exe") && (candidate.is_relative() || candidate.exists()))
}

pub(crate) fn build_archive_extraction_plan(
    archive: &Path,
    destination_mode: ArchiveDestinationMode,
    extractor: ArchiveExtractor,
    seven_zip: Option<PathBuf>,
) -> Result<ArchiveExtractionPlan, String> {
    let kind = ArchiveKind::from_path(archive).ok_or_else(|| "Unsupported archive type".to_string())?;
    let parent = archive.parent().ok_or_else(|| "Archive parent folder unavailable".to_string())?;
    let stem = archive.file_stem().and_then(|value| value.to_str()).ok_or_else(|| "Archive name unavailable".to_string())?;
    let destination_path = match destination_mode {
        ArchiveDestinationMode::Here => parent.to_path_buf(),
        ArchiveDestinationMode::Folder => parent.join(stem),
    };
    if matches!(destination_mode, ArchiveDestinationMode::Folder) && destination_path.exists() {
        return Err("Extraction destination already exists".to_string());
    }

    if kind == ArchiveKind::Rar && extractor == ArchiveExtractor::Builtin {
        return Err("7-Zip is required to extract RAR archives".to_string());
    }

    if kind == ArchiveKind::Zip && extractor == ArchiveExtractor::Builtin {
        return Ok(ArchiveExtractionPlan {
            executable: PathBuf::from("powershell.exe"),
            args: vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "Expand-Archive -LiteralPath $args[0] -DestinationPath $args[1]".to_string(),
                archive.to_string_lossy().to_string(),
                destination_path.to_string_lossy().to_string(),
            ],
            destination_path,
            expected_created_folder: matches!(destination_mode, ArchiveDestinationMode::Folder).then(|| parent.join(stem)),
        });
    }

    let seven_zip = seven_zip.ok_or_else(|| "7-Zip is required to use 7-Zip extraction".to_string())?;
    Ok(ArchiveExtractionPlan {
        executable: seven_zip,
        args: vec![
            "x".to_string(),
            archive.to_string_lossy().to_string(),
            format!("-o{}", destination_path.to_string_lossy()),
            "-y".to_string(),
        ],
        destination_path,
        expected_created_folder: matches!(destination_mode, ArchiveDestinationMode::Folder).then(|| parent.join(stem)),
    })
}

#[cfg(test)]
pub(crate) use clipboard::{clipboard_mode_from_drop_effect, paste_clipboard_items};
#[cfg(test)]
pub(crate) use file_ops::{
    available_destination_path, copy_dir, copy_path, move_path_with_rename,
    next_new_text_document_path,
};
#[cfg(test)]
pub(crate) use icons::{
    resolve_stack_item_icons_batch, resolve_stack_item_icons_for_paths,
    resolve_stack_item_icons_for_paths_async,
};
#[cfg(test)]
pub(crate) use items::{stack_file_attributes_from_bits, stack_item_from_path};
#[cfg(test)]
pub(crate) use models::{ClipboardMode, StackClipboard};
#[cfg(test)]
pub(crate) use native_drag::native_drag_mechanism;
#[cfg(test)]
pub(crate) use open_with::open_with_candidates_for_extension_with_resolver;
#[cfg(test)]
pub(crate) use paging::{
    read_stack_folder_page, read_stack_folder_page_with_session, stack_folder_warning,
};
#[cfg(test)]
pub(crate) use paths::{
    normalize_existing_path, normalize_stack_path_candidate, paths_match_for_unpin,
    resolve_stack_alias_with_profile, stack_display_path_string, validate_child_name,
};
#[cfg(test)]
pub(crate) use pins::{backup_corrupt_pin_store, reorder_pins_by_paths};
#[cfg(test)]
pub(crate) use popup_window::normalize_show_stack_popup_request;

pub(crate) fn suppress_stack_popup_focus_loss(app_handle: &AppHandle) -> bool {
    popup_window::suppress_stack_popup_focus_loss(app_handle)
}

#[tauri::command]
pub fn list_pinned_stack_folders(app_handle: AppHandle) -> Result<Vec<PinnedStackFolder>, String> {
    pins::load_pins_with_defaults(&app_handle)
}

#[tauri::command]
pub fn pin_stack_folder(
    app_handle: AppHandle,
    path: String,
) -> Result<Vec<PinnedStackFolder>, String> {
    pins::pin_folder(&app_handle, &path)
}

#[tauri::command]
pub fn unpin_stack_folder(
    app_handle: AppHandle,
    path: String,
) -> Result<Vec<PinnedStackFolder>, String> {
    pins::unpin_folder(&app_handle, &path)
}

#[tauri::command]
pub fn reorder_pinned_stack_folders(
    app_handle: AppHandle,
    ordered_paths: Vec<String>,
) -> Result<Vec<PinnedStackFolder>, String> {
    pins::reorder_pinned_folders(&app_handle, &ordered_paths)
}

#[tauri::command]
pub fn show_stack_popup(
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    request: ShowStackPopupRequest,
) -> Result<(), String> {
    popup_window::show_stack_popup_window(app_handle, state, request)
}

#[tauri::command]
pub fn hide_stack_popup(app_handle: AppHandle) -> Result<(), String> {
    popup_window::hide_stack_popup_window(app_handle)
}

#[tauri::command]
pub fn get_stack_popup_request(
    state: State<'_, Mutex<StackPopupRuntimeState>>,
) -> Result<Option<ShowStackPopupRequest>, String> {
    Ok(popup_window::latest_stack_popup_request(state))
}

#[tauri::command]
pub fn begin_stack_popup_focus_loss_hold(
    state: State<'_, Mutex<StackPopupRuntimeState>>,
) -> Result<(), String> {
    popup_window::begin_stack_popup_focus_hold(&state);
    Ok(())
}

#[tauri::command]
pub fn end_stack_popup_focus_loss_hold(
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
) -> Result<(), String> {
    popup_window::end_stack_popup_focus_hold(&app_handle, &state);
    Ok(())
}

#[tauri::command]
pub fn resize_stack_popup(
    app_handle: AppHandle,
    width: f64,
    height: f64,
    persist: bool,
) -> Result<StackPopupLogicalSize, String> {
    popup_window::resize_stack_popup_window(app_handle, width, height, persist)
}

#[tauri::command]
pub fn read_stack_folder(
    path: String,
    offset: usize,
    limit: Option<usize>,
    session_id: Option<String>,
) -> Result<StackFolderPage, String> {
    let candidate = PathBuf::from(&path);
    let folder = if candidate.is_dir() {
        paths::normalize_existing_dir(&path)?
    } else {
        candidate.to_string_lossy().to_string()
    };
    paging::read_stack_folder_page_with_session(
        &folder,
        session_id.as_deref(),
        offset,
        limit.unwrap_or(paging::DEFAULT_PAGE_LIMIT),
    )
}

#[tauri::command]
pub fn suggest_stack_paths(
    parent_path: String,
    segment: String,
    limit: Option<usize>,
) -> Result<Vec<StackPathSuggestion>, String> {
    let parent = paths::normalize_existing_dir(&parent_path)
        .map_err(|error| format!("Folder unavailable: {error}"))?;
    let normalized_segment = segment.to_lowercase();
    let max = limit.unwrap_or(20).clamp(1, 50);
    let mut suggestions = Vec::new();
    for entry in std::fs::read_dir(&parent).map_err(|error| format!("Folder unavailable: {error}"))? {
        let entry = entry.map_err(|error| format!("Folder unavailable: {error}"))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !normalized_segment.is_empty() && !name.to_lowercase().starts_with(&normalized_segment) {
            continue;
        }
        suggestions.push(StackPathSuggestion {
            name,
            path: path.to_string_lossy().into_owned(),
        });
    }
    suggestions.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    suggestions.truncate(max);
    Ok(suggestions)
}

#[tauri::command]
pub async fn resolve_stack_item_icons(paths: Vec<String>) -> Result<StackItemIconResolutionBatch, String> {
    icons::resolve_stack_item_icons_for_paths_async(paths).await
}

#[tauri::command]
pub fn open_stack_item(path: String) -> Result<(), String> {
    shell_paths::open_shell_path(path)
}

#[tauri::command]
pub fn open_stack_item_with_picker(path: String) -> Result<(), String> {
    let path = paths::normalize_existing_path(&path)?;
    if Path::new(&path).is_dir() {
        return Err("Open with is only available for files".to_string());
    }
    shell_paths::open_shell_path_with_picker(path)
}

#[tauri::command]
pub fn list_stack_open_with_candidates(
    path: String,
) -> Result<Vec<StackOpenWithCandidate>, String> {
    let path = paths::normalize_existing_path(&path)?;
    if Path::new(&path).is_dir() {
        return Err("Open with is only available for files".to_string());
    }
    open_with::open_with_candidates_for_path(Path::new(&path))
}

#[tauri::command]
pub fn open_stack_item_with_app(path: String, app_id: String) -> Result<(), String> {
    let path = paths::normalize_existing_path(&path)?;
    if Path::new(&path).is_dir() {
        return Err("Open with is only available for files".to_string());
    }
    open_with::open_with_app(Path::new(&path), &app_id)
}

#[tauri::command]
pub fn rename_stack_item(path: String, new_name: String) -> Result<StackItem, String> {
    file_ops::rename_stack_item_path(path, new_name)
}

#[tauri::command]
pub fn copy_stack_items(
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    paths: Vec<String>,
) -> Result<(), String> {
    clipboard::set_stack_clipboard(&state, models::ClipboardMode::Copy, paths)
}

#[tauri::command]
pub fn prepare_stack_file_drag(paths: Vec<String>) -> Result<StackNativeDragPreparation, String> {
    native_drag::start_stack_file_drag(paths)
}

#[tauri::command]
pub fn cut_stack_items(
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    paths: Vec<String>,
) -> Result<(), String> {
    clipboard::set_stack_clipboard(&state, models::ClipboardMode::Cut, paths)
}

#[tauri::command]
pub fn paste_stack_items(
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    destination: String,
) -> Result<StackPasteResult, String> {
    clipboard::paste_stack_clipboard_items(&state, destination)
}

#[tauri::command]
pub fn delete_stack_item(
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    path: String,
) -> Result<(), String> {
    popup_window::begin_stack_popup_focus_hold(&state);
    let result = file_ops::delete_stack_item_path(path);
    popup_window::end_stack_popup_focus_hold(&app_handle, &state);
    result
}

#[tauri::command]
pub fn new_stack_folder(parent: String, name: String) -> Result<StackItem, String> {
    file_ops::new_stack_folder_path(parent, name)
}

#[tauri::command]
pub fn new_stack_text_file(parent: String) -> Result<StackItem, String> {
    file_ops::new_stack_text_file_path(parent)
}

#[tauri::command]
pub fn open_stack_terminal_here(path: String) -> Result<(), String> {
    file_ops::open_terminal_here_path(path)
}

#[tauri::command]
pub fn open_stack_folder_in_vscode(path: String) -> Result<(), String> {
    let path = paths::normalize_existing_dir(&path)?;
    crate::shell_paths::open_folder_in_vscode(path)
}

#[tauri::command]
pub fn reveal_stack_item(path: String) -> Result<(), String> {
    file_ops::reveal_stack_item_path(path)
}

#[tauri::command]
pub fn extract_stack_archive(
    archive_path: String,
    destination_mode: ArchiveDestinationMode,
    extractor: ArchiveExtractor,
) -> Result<(), String> {
    let archive = PathBuf::from(paths::normalize_existing_path(&archive_path)?);
    if !archive.is_absolute() {
        return Err("Archive path must be absolute".to_string());
    }
    if !archive.is_file() {
        return Err("Archive path must be a file".to_string());
    }
    let kind = ArchiveKind::from_path(&archive).ok_or_else(|| "Unsupported archive type".to_string())?;
    let seven_zip = if extractor == ArchiveExtractor::SevenZip || kind == ArchiveKind::Rar { find_seven_zip() } else { None };
    let plan = build_archive_extraction_plan(&archive, destination_mode, extractor, seven_zip)?;
    Command::new(&plan.executable)
        .args(&plan.args)
        .status()
        .map_err(|error| format!("Failed to extract archive: {error}"))
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!("Archive extraction failed with status {status}"))
            }
        })
}

#[tauri::command]
pub fn show_stack_item_properties(path: String) -> Result<(), String> {
    let path = PathBuf::from(paths::normalize_existing_path(&path)?);
    let plan = build_stack_item_properties_plan(&path)?;
    Command::new(&plan.executable)
        .args(&plan.args)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to show properties: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{
        available_destination_path, backup_corrupt_pin_store, clipboard_mode_from_drop_effect,
        copy_dir, move_path_with_rename, native_drag_mechanism, next_new_text_document_path,
        open_with_candidates_for_extension_with_resolver, paste_clipboard_items,
        paths_match_for_unpin, read_stack_folder_page, read_stack_folder_page_with_session,
        reorder_pins_by_paths, resolve_stack_item_icons_batch, resolve_stack_item_icons_for_paths,
        resolve_stack_item_icons_for_paths_async, resolve_stack_alias_with_profile,
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
    fn chooses_next_new_text_document_name_without_overwrite() {
        let root = test_dir("new-text-document");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("New Text Document.txt"), b"one").unwrap();
        fs::write(root.join("New Text Document (2).txt"), b"two").unwrap();

        let next = next_new_text_document_path(&root).unwrap();

        assert_eq!(next.file_name().unwrap(), "New Text Document (3).txt");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn suggests_developer_open_with_apps_for_text_extensions() {
        let candidates =
            open_with_candidates_for_extension_with_resolver(Some("txt"), |candidate| {
                match candidate {
                    "notepad.exe" => Some(PathBuf::from(r"C:\Windows\System32\notepad.exe")),
                    r"%ProgramFiles%\Notepad++\notepad++.exe" => {
                        Some(PathBuf::from(r"C:\Program Files\Notepad++\notepad++.exe"))
                    }
                    r"%LocalAppData%\Programs\Microsoft VS Code\Code.exe" => Some(PathBuf::from(
                        r"C:\Users\dev\AppData\Local\Programs\Microsoft VS Code\Code.exe",
                    )),
                    _ => None,
                }
            })
            .unwrap();

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Notepad", "Notepad++", "Visual Studio Code"]
        );
    }

    #[test]
    fn stack_file_drag_uses_native_drag_mechanism() {
        #[cfg(windows)]
        assert_eq!(native_drag_mechanism(), "ole-do-drag-drop");

        #[cfg(not(windows))]
        assert_eq!(native_drag_mechanism(), "unsupported");
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
    fn deserializes_stack_item_without_icon_payload() {
        let parsed: StackItem = serde_json::from_value(serde_json::json!({
            "path": r"C:\Items\notes.txt",
            "name": "notes.txt",
            "kind": "file",
            "typeLabel": "TXT File",
            "sizeBytes": 1,
            "modifiedAt": 1700000000000u64,
            "isHidden": false,
            "isReadonly": false,
            "isSystem": false,
            "isSymlink": false,
            "isReparsePoint": false
        }))
        .unwrap();

        assert_eq!(parsed.icon_data_url, None);
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

        let first_page =
            read_stack_folder_page_with_session(root.to_str().unwrap(), None, 0, 500).unwrap();
        let session_id = first_page.session_id.clone().unwrap();
        let second_page = read_stack_folder_page_with_session(
            root.to_str().unwrap(),
            Some(&session_id),
            500,
            500,
        )
        .unwrap();

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
    fn suggests_stack_paths_with_directories_only_sorted_and_bounded() {
        let root = test_dir("path-suggestions");
        fs::create_dir_all(root.join("zulu")).unwrap();
        fs::create_dir_all(root.join("Alpha")).unwrap();
        fs::write(root.join("aardvark.txt"), b"x").unwrap();

        let suggestions = super::suggest_stack_paths(root.to_string_lossy().into_owned(), "".into(), Some(1)).unwrap();

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].name, "Alpha");
        assert!(suggestions[0].path.ends_with("Alpha"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn suggests_stack_paths_returns_structured_errors_for_invalid_parent() {
        let root = test_dir("path-suggestions-file-parent");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("file.txt");
        fs::write(&file, b"x").unwrap();

        let error = super::suggest_stack_paths(file.to_string_lossy().into_owned(), "".into(), Some(20)).unwrap_err();

        assert!(error.contains("Folder unavailable"));
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
        super::file_ops::delete_stack_item_path(
            root.join("File.txt").to_str().unwrap().to_string(),
        )
        .unwrap();
        assert!(!root.join("File.txt").exists());
        super::file_ops::delete_stack_item_path(root.join("Folder").to_str().unwrap().to_string())
            .unwrap();
        assert!(!root.join("Folder").exists());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn stack_icon_resolution_cache_reuses_cached_path_icons() {
        let root = test_dir("icon-cache");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("alpha.txt");
        fs::write(&file, b"x").unwrap();
        let path = file.to_string_lossy().to_string();

        let first = resolve_stack_item_icons_for_paths(vec![path.clone()]).unwrap();
        let second = resolve_stack_item_icons_for_paths(vec![path.clone()]).unwrap();

        assert_eq!(first.items.len(), 1);
        assert_eq!(second.items.len(), 1);
        assert!(!first.items[0].cache_hit);
        assert!(second.items[0].cache_hit);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn stack_icon_resolution_returns_none_for_missing_paths_without_failing_batch() {
        let missing = test_dir("missing-icon")
            .join("does-not-exist.bin")
            .to_string_lossy()
            .to_string();

        let batch = resolve_stack_item_icons_for_paths(vec![missing.clone()]).unwrap();
        assert_eq!(batch.items.len(), 1);
        assert_eq!(batch.items[0].path, missing);
        assert_eq!(batch.items[0].icon_data_url, None);
    }

    #[test]
    fn stack_icon_resolution_batch_limit_stays_bounded() {
        let roots = (0..200usize)
            .map(|index| format!(r"C:\temp\path-{index:03}.txt"))
            .collect::<Vec<_>>();
        let bounded = resolve_stack_item_icons_batch(roots, 24);

        assert!(bounded.len() <= 24);
    }

    #[test]
    fn async_stack_icon_resolution_preserves_batch_contract() {
        let roots = (0..64usize)
            .map(|index| format!(r"C:\temp\async-path-{index:03}.txt"))
            .collect::<Vec<_>>();

        let batch = tauri::async_runtime::block_on(resolve_stack_item_icons_for_paths_async(roots))
            .unwrap();

        assert_eq!(batch.requested_count, 64);
        assert_eq!(batch.max_batch_size, 24);
        assert!(batch.truncated);
        assert!(batch.items.len() <= 24);
        assert_eq!(batch.resolved_count, batch.items.len());
        assert_eq!(batch.cache_hits + batch.cache_misses, batch.items.len());
    }

    #[test]
    fn archive_kind_from_path_accepts_zip_rar_files_only() {
        let root = test_dir("archive-kind");
        fs::create_dir_all(root.join("folder.zip")).unwrap();
        fs::write(root.join("bundle.zip"), b"zip").unwrap();
        fs::write(root.join("BUNDLE.ZIP"), b"zip").unwrap();
        fs::write(root.join("bundle.rar"), b"rar").unwrap();
        fs::write(root.join("bundle.7z"), b"7z").unwrap();
        fs::write(root.join("bundle"), b"none").unwrap();

        assert_eq!(super::ArchiveKind::from_path(&root.join("bundle.zip")), Some(super::ArchiveKind::Zip));
        assert_eq!(super::ArchiveKind::from_path(&root.join("BUNDLE.ZIP")), Some(super::ArchiveKind::Zip));
        assert_eq!(super::ArchiveKind::from_path(&root.join("bundle.rar")), Some(super::ArchiveKind::Rar));
        assert_eq!(super::ArchiveKind::from_path(&root.join("bundle.7z")), None);
        assert_eq!(super::ArchiveKind::from_path(&root.join("bundle")), None);
        assert_eq!(super::ArchiveKind::from_path(&root.join("folder.zip")), None);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn archive_7zip_candidates_include_program_files_and_path_fallback() {
        let candidates = super::seven_zip_discovery_candidates();
        assert!(candidates.iter().any(|candidate| candidate.ends_with(r"7-Zip\7z.exe")));
        assert!(candidates.iter().any(|candidate| candidate == Path::new("7z.exe")));
    }

    #[test]
    fn archive_extraction_plan_vectorizes_paths_with_spaces() {
        let root = test_dir("archive-plan spaces");
        fs::create_dir_all(&root).unwrap();
        let archive = root.join("release build.rar");
        fs::write(&archive, b"rar").unwrap();

        let plan = super::build_archive_extraction_plan(
            &archive,
            super::ArchiveDestinationMode::Folder,
            super::ArchiveExtractor::SevenZip,
            Some(root.join("Tools Dir").join("7z.exe")),
        ).unwrap();

        assert_eq!(plan.executable, root.join("Tools Dir").join("7z.exe"));
        assert!(plan.args.iter().any(|arg| arg == &archive.to_string_lossy().to_string()));
        assert!(plan.args.iter().any(|arg| arg == &format!("-o{}", root.join("release build").to_string_lossy())));
        assert!(!plan.args.join(" ").contains('"'));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn archive_extraction_plan_keeps_builtin_and_7zip_zip_modes_distinct() {
        let root = test_dir("archive-plan-modes");
        fs::create_dir_all(&root).unwrap();
        let archive = root.join("bundle.zip");
        fs::write(&archive, b"zip").unwrap();

        let builtin = super::build_archive_extraction_plan(
            &archive,
            super::ArchiveDestinationMode::Folder,
            super::ArchiveExtractor::Builtin,
            None,
        ).unwrap();
        assert_eq!(builtin.executable, Path::new("powershell.exe"));
        assert!(builtin.args.iter().any(|arg| arg.contains("Expand-Archive")));

        let seven_zip_path = root.join("7z.exe");
        let seven_zip = super::build_archive_extraction_plan(
            &archive,
            super::ArchiveDestinationMode::Folder,
            super::ArchiveExtractor::SevenZip,
            Some(seven_zip_path.clone()),
        ).unwrap();
        assert_eq!(seven_zip.executable, seven_zip_path);
        assert!(seven_zip.args.iter().any(|arg| arg == "x"));

        let missing_seven_zip = super::build_archive_extraction_plan(
            &archive,
            super::ArchiveDestinationMode::Folder,
            super::ArchiveExtractor::SevenZip,
            None,
        ).unwrap_err();
        assert_eq!(missing_seven_zip, "7-Zip is required to use 7-Zip extraction");

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn read_stack_folder_page_lists_zip_contents_as_stack_rows() {
        let root = test_dir("zip-browser");
        fs::create_dir_all(&root).unwrap();
        let archive = root.join("bundle.zip");
        let file = fs::File::create(&archive).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        writer.add_directory("docs/", options).unwrap();
        writer.start_file("docs/readme.md", options).unwrap();
        use std::io::Write;
        writer.write_all(b"hello").unwrap();
        writer.start_file("app.exe", options).unwrap();
        writer.write_all(b"exe").unwrap();
        writer.finish().unwrap();

        let page = super::read_stack_folder_page(archive.to_str().unwrap(), 0, 20).unwrap();
        assert_eq!(page.path, archive.to_string_lossy());
        assert_eq!(page.items.iter().map(|item| (&item.name, &item.kind)).collect::<Vec<_>>(), vec![(&"docs".to_string(), &"folder".to_string()), (&"app.exe".to_string(), &"file".to_string())]);
        assert!(page.items.iter().any(|item| item.path.ends_with(r"bundle.zip\docs")));

        let nested_path = format!(r"{}\docs", archive.to_string_lossy());
        let nested = super::read_stack_folder_page(&nested_path, 0, 20).unwrap();
        assert_eq!(nested.items.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(), vec!["readme.md"]);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn read_stack_folder_command_accepts_zip_paths() {
        let root = test_dir("zip-browser-command");
        fs::create_dir_all(&root).unwrap();
        let archive = root.join("bundle.zip");
        let file = fs::File::create(&archive).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("app.exe", options).unwrap();
        use std::io::Write;
        writer.write_all(b"exe").unwrap();
        writer.finish().unwrap();

        let page = super::read_stack_folder(archive.to_string_lossy().to_string(), 0, Some(20), None).unwrap();
        assert_eq!(page.items.iter().map(|item| item.name.as_str()).collect::<Vec<_>>(), vec!["app.exe"]);

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn stack_item_properties_plan_validates_existing_paths() {
        let root = test_dir("properties-plan");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("server file.ts");
        fs::write(&file, b"ts").unwrap();

        let plan = super::build_stack_item_properties_plan(&file).unwrap();
        assert_eq!(plan.executable, Path::new("powershell.exe"));
        assert!(plan.args.iter().any(|arg| arg.contains("InvokeVerb('properties')")));
        assert!(plan.args.iter().any(|arg| arg == &file.to_string_lossy().to_string()));
        assert!(!plan.args.join(" ").contains('"'));

        assert!(super::build_stack_item_properties_plan(&root.join("missing.ts")).is_err());
        fs::remove_dir_all(root).ok();
    }
}
