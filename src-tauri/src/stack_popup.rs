mod auth;
mod clipboard;
mod file_ops;
mod git_status;
mod icons;
mod items;
mod models;
mod native_drag;
mod open_with;
mod paging;
mod paths;
mod pins;
mod popup_window;
mod process_runner;
mod recovery_journal;
pub(crate) mod terminal;

use crate::shell_paths;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, State, WebviewWindow};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackPathSuggestion {
    pub name: String,
    pub path: String,
}

pub(crate) use auth::{
    allowed_stack_command_callers, authorize_stack_command, CallerAuthError, StackCommandAuth,
};
pub use models::{
    PinnedStackFolder, ShowStackPopupRequest, StackFolderPage, StackGitBranchRequest,
    StackGitBranches, StackGitCommitRequest, StackGitLog, StackGitLogRequest,
    StackGitOperationResult, StackGitStageRequest, StackGitStatus, StackGitTree,
    StackGitTreeRequest, StackItem, StackItemIconResolutionBatch, StackNativeDragPreparation,
    StackOpenWithCandidate, StackPasteResult, StackPopupLogicalSize, StackPopupRuntimeState,
};
pub use terminal::{
    StackTerminalPollResult, StackTerminalRenameRequest, StackTerminalResizeRequest,
    StackTerminalSessionSnapshot, StackTerminalStartRequest, StackTerminalStopRequest,
    StackTerminalWriteRequest,
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
        match path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
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
    pub path: PathBuf,
    pub verb: &'static str,
    pub invoke_id_list: bool,
    pub dialog_title_fragment: String,
}

pub(crate) fn build_stack_item_properties_plan(
    path: &Path,
) -> Result<StackItemPropertiesPlan, String> {
    if !path.exists() {
        return Err("Path unavailable".to_string());
    }
    Ok(StackItemPropertiesPlan {
        path: path.to_path_buf(),
        verb: "properties",
        invoke_id_list: true,
        dialog_title_fragment: stack_item_properties_title_fragment(path),
    })
}

fn stack_item_properties_title_fragment(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| format!("{name} Properties"))
        .unwrap_or_else(|| "Properties".to_string())
}

pub(crate) fn seven_zip_discovery_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        candidates.push(PathBuf::from(program_files).join("7-Zip").join("7z.exe"));
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        candidates.push(
            PathBuf::from(program_files_x86)
                .join("7-Zip")
                .join("7z.exe"),
        );
    }
    candidates
}

fn find_seven_zip() -> Option<PathBuf> {
    seven_zip_discovery_candidates()
        .into_iter()
        .find(|candidate| {
            candidate.file_name().and_then(|name| name.to_str()) == Some("7z.exe")
                && candidate.exists()
        })
}

pub(crate) fn build_archive_extraction_plan(
    archive: &Path,
    destination_mode: ArchiveDestinationMode,
    extractor: ArchiveExtractor,
    seven_zip: Option<PathBuf>,
) -> Result<ArchiveExtractionPlan, String> {
    let kind =
        ArchiveKind::from_path(archive).ok_or_else(|| "Unsupported archive type".to_string())?;
    let parent = archive
        .parent()
        .ok_or_else(|| "Archive parent folder unavailable".to_string())?;
    let stem = archive
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "Archive name unavailable".to_string())?;
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
            expected_created_folder: matches!(destination_mode, ArchiveDestinationMode::Folder)
                .then(|| parent.join(stem)),
        });
    }

    let seven_zip =
        seven_zip.ok_or_else(|| "7-Zip is required to use 7-Zip extraction".to_string())?;
    Ok(ArchiveExtractionPlan {
        executable: seven_zip,
        args: vec![
            "x".to_string(),
            archive.to_string_lossy().to_string(),
            format!("-o{}", destination_path.to_string_lossy()),
            "-y".to_string(),
        ],
        destination_path,
        expected_created_folder: matches!(destination_mode, ArchiveDestinationMode::Folder)
            .then(|| parent.join(stem)),
    })
}

#[cfg(test)]
pub(crate) use clipboard::{clipboard_mode_from_drop_effect, paste_clipboard_items};
#[cfg(test)]
pub(crate) use file_ops::{
    available_destination_path, copy_dir, copy_path, move_path_with_rename,
    next_new_text_document_path, windows_explorer_reveal_launch_plan,
    windows_explorer_reveal_select_arg, windows_explorer_reveal_show_mode,
    WindowsExplorerRevealShowMode,
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

pub(crate) fn restore_stack_popup_topmost(app_handle: &AppHandle) {
    #[cfg(target_os = "windows")]
    {
        if popup_window::suppress_stack_popup_topmost_restore(app_handle) {
            return;
        }
        if let Ok(hwnd) = stack_popup_owner_hwnd(app_handle) {
            let _ = set_stack_popup_topmost(hwnd, true);
        }
    }
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
pub async fn get_stack_git_status(
    window: WebviewWindow,
    path: String,
) -> Result<Option<StackGitStatus>, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::GET_STACK_GIT_STATUS,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_status_for_path_async(path).await
}

#[tauri::command]
pub fn open_stack_git_remote_url(window: WebviewWindow, url: String) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::OPEN_STACK_GIT_REMOTE_URL,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    open_stack_git_remote_url_native(&validate_stack_git_remote_url(&url)?)
}

fn validate_stack_git_remote_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.contains('\0') || trimmed.chars().any(char::is_whitespace) {
        return Err("Git remote URL is invalid".to_string());
    }

    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .ok_or_else(|| "Git remote URL must use http or https".to_string())?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') {
        return Err("Git remote URL must not include credentials".to_string());
    }

    Ok(trimmed.to_string())
}

#[cfg(target_os = "windows")]
fn open_stack_git_remote_url_native(url: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn to_wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    let url_wide = to_wide(OsStr::new(url));
    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            None,
            PCWSTR(url_wide.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as isize;
    if code <= 32 {
        return Err(format!(
            "ShellExecuteW failed to open git remote URL with code {code}"
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_stack_git_remote_url_native(_url: &str) -> Result<(), String> {
    Err("Opening git remote URLs is only supported on Windows".to_string())
}

#[tauri::command]
pub async fn stack_git_add_paths(
    window: WebviewWindow,
    request: StackGitStageRequest,
) -> Result<StackGitOperationResult, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STACK_GIT_ADD_PATHS,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_add_paths_async(request).await
}

#[tauri::command]
pub async fn stack_git_commit(
    window: WebviewWindow,
    request: StackGitCommitRequest,
) -> Result<StackGitOperationResult, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STACK_GIT_COMMIT,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_commit_async(request).await
}

#[tauri::command]
pub async fn stack_git_log(
    window: WebviewWindow,
    request: StackGitLogRequest,
) -> Result<StackGitLog, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STACK_GIT_LOG,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_log_async(request).await
}

#[tauri::command]
pub async fn stack_git_tree(
    window: WebviewWindow,
    request: StackGitTreeRequest,
) -> Result<StackGitTree, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STACK_GIT_TREE,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_tree_async(request).await
}

#[tauri::command]
pub async fn stack_git_branches(
    window: WebviewWindow,
    path: String,
) -> Result<StackGitBranches, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STACK_GIT_BRANCHES,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_branches_async(path).await
}

#[tauri::command]
pub async fn stack_git_fetch(
    window: WebviewWindow,
    folder_path: String,
) -> Result<StackGitOperationResult, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STACK_GIT_FETCH,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_fetch_async(folder_path).await
}

#[tauri::command]
pub async fn stack_git_pull(
    window: WebviewWindow,
    folder_path: String,
) -> Result<StackGitOperationResult, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STACK_GIT_PULL,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_pull_async(folder_path).await
}

#[tauri::command]
pub async fn stack_git_push(
    window: WebviewWindow,
    folder_path: String,
) -> Result<StackGitOperationResult, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STACK_GIT_PUSH,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_push_async(folder_path).await
}

#[tauri::command]
pub async fn stack_git_checkout_branch(
    window: WebviewWindow,
    request: StackGitBranchRequest,
) -> Result<StackGitOperationResult, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STACK_GIT_CHECKOUT_BRANCH,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_checkout_branch_async(request).await
}

#[tauri::command]
pub async fn stack_git_create_branch(
    window: WebviewWindow,
    request: StackGitBranchRequest,
) -> Result<StackGitOperationResult, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STACK_GIT_CREATE_BRANCH,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    git_status::stack_git_create_branch_async(request).await
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
    for entry in
        std::fs::read_dir(&parent).map_err(|error| format!("Folder unavailable: {error}"))?
    {
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
pub async fn resolve_stack_item_icons(
    paths: Vec<String>,
) -> Result<StackItemIconResolutionBatch, String> {
    icons::resolve_stack_item_icons_for_paths_async(paths).await
}

#[tauri::command]
pub fn open_stack_item(window: WebviewWindow, path: String) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::OPEN_STACK_ITEM,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    let path = paths::normalize_existing_path(&path)?;
    if shell_paths::classify_stack_item_open_route(Path::new(&path))
        == shell_paths::StackItemOpenRoute::AuditedApp
    {
        return shell_paths::launch_app_path(path);
    }
    shell_paths::open_shell_path(path)
}

#[tauri::command]
pub fn open_stack_item_with_picker(window: WebviewWindow, path: String) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::OPEN_STACK_ITEM_WITH_PICKER,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    let path = paths::normalize_existing_path(&path)?;
    if Path::new(&path).is_dir() {
        return Err("Open with is only available for files".to_string());
    }
    shell_paths::open_shell_path_with_picker(path)
}

#[tauri::command]
pub fn list_stack_open_with_candidates(
    window: WebviewWindow,
    path: String,
) -> Result<Vec<StackOpenWithCandidate>, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::LIST_STACK_OPEN_WITH_CANDIDATES,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    let path = paths::normalize_existing_path(&path)?;
    if Path::new(&path).is_dir() {
        return Err("Open with is only available for files".to_string());
    }
    open_with::open_with_candidates_for_path(Path::new(&path))
}

#[tauri::command]
pub fn open_stack_item_with_app(
    window: WebviewWindow,
    path: String,
    app_id: String,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::OPEN_STACK_ITEM_WITH_APP,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    let path = paths::normalize_existing_path(&path)?;
    if Path::new(&path).is_dir() {
        return Err("Open with is only available for files".to_string());
    }
    open_with::open_with_app(Path::new(&path), &app_id)
}

#[tauri::command]
pub fn rename_stack_item(
    window: WebviewWindow,
    path: String,
    new_name: String,
) -> Result<StackItem, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::RENAME_STACK_ITEM,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    file_ops::rename_stack_item_path(path, new_name)
}

#[tauri::command]
pub fn copy_stack_items(
    window: WebviewWindow,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    paths: Vec<String>,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::COPY_STACK_ITEMS,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    clipboard::set_stack_clipboard(&state, models::ClipboardMode::Copy, paths)
}

#[tauri::command]
pub fn prepare_stack_file_drag(
    window: WebviewWindow,
    paths: Vec<String>,
) -> Result<StackNativeDragPreparation, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::PREPARE_STACK_FILE_DRAG,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    native_drag::start_stack_file_drag(paths)
}

#[tauri::command]
pub fn cut_stack_items(
    window: WebviewWindow,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    paths: Vec<String>,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::CUT_STACK_ITEMS,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    clipboard::set_stack_clipboard(&state, models::ClipboardMode::Cut, paths)
}

#[tauri::command]
pub async fn paste_stack_items(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    destination: String,
) -> Result<StackPasteResult, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::PASTE_STACK_ITEMS,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    clipboard::paste_stack_clipboard_items_async(&app_handle, &state, destination).await
}

#[tauri::command]
pub async fn delete_stack_item(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    path: String,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::DELETE_STACK_ITEM,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    popup_window::begin_stack_popup_focus_hold(&state);
    let result = file_ops::delete_stack_item_path_async(path).await;
    popup_window::end_stack_popup_focus_hold(&app_handle, &state);
    result
}

#[tauri::command]
pub fn new_stack_folder(
    window: WebviewWindow,
    parent: String,
    name: String,
) -> Result<StackItem, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::NEW_STACK_FOLDER,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    file_ops::new_stack_folder_path(parent, name)
}

#[tauri::command]
pub fn new_stack_text_file(window: WebviewWindow, parent: String) -> Result<StackItem, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::NEW_STACK_TEXT_FILE,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    file_ops::new_stack_text_file_path(parent)
}

#[tauri::command]
pub fn open_stack_terminal_here(window: WebviewWindow, path: String) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::OPEN_STACK_TERMINAL_HERE,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    file_ops::open_terminal_here_path(path)
}

#[tauri::command]
pub async fn start_persistent_terminal(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
) -> Result<StackTerminalSessionSnapshot, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::START_PERSISTENT_TERMINAL,
            callers: &[crate::shell_windows::TERMINAL_PANEL_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    let folder_path = std::env::var("USERPROFILE")
        .ok()
        .filter(|path| Path::new(path).is_dir())
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|path| path.to_string_lossy().into_owned())
        })
        .ok_or_else(|| "No startup directory is available for the terminal".to_string())?;
    terminal::start_stack_terminal_session(
        &app_handle,
        &state,
        window.label(),
        StackTerminalStartRequest {
            folder_path,
            profile: None,
            target_label: Some(crate::shell_windows::TERMINAL_PANEL_LABEL.to_string()),
        },
    )
    .await
}

#[tauri::command]
pub async fn start_stack_terminal(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    request: StackTerminalStartRequest,
) -> Result<StackTerminalSessionSnapshot, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::TerminalSessionTarget {
            command: crate::contracts::commands::START_STACK_TERMINAL,
            callers: &[
                crate::shell_windows::TERMINAL_PANEL_LABEL,
                crate::shell_windows::STACK_POPUP_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    terminal::start_stack_terminal_session(&app_handle, &state, window.label(), request).await
}

#[tauri::command]
pub fn read_stack_terminal(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
) -> Result<terminal::StackTerminalReadResult, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::TerminalSessionTarget {
            command: crate::contracts::commands::READ_STACK_TERMINAL,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    terminal::read_stack_terminal(&app_handle, &state, window.label(), session_id)
}

#[tauri::command]
pub async fn write_stack_terminal(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
    input: String,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::TerminalSessionTarget {
            command: crate::contracts::commands::WRITE_STACK_TERMINAL,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    terminal::write_stack_terminal(
        &app_handle,
        &state,
        window.label(),
        StackTerminalWriteRequest { session_id, input },
    )
    .await
}

#[tauri::command]
pub fn resize_stack_terminal(
    window: WebviewWindow,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
    cols: u16,
    rows: u16,
    pixel_width: Option<u16>,
    pixel_height: Option<u16>,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::TerminalSessionTarget {
            command: crate::contracts::commands::RESIZE_STACK_TERMINAL,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    terminal::resize_stack_terminal_session(
        &state,
        window.label(),
        StackTerminalResizeRequest {
            session_id,
            cols,
            rows,
            pixel_width,
            pixel_height,
        },
    )
}

#[tauri::command]
pub fn stop_stack_terminal(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::TerminalSessionTarget {
            command: crate::contracts::commands::STOP_STACK_TERMINAL,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    terminal::stop_stack_terminal(
        &app_handle,
        &state,
        window.label(),
        StackTerminalStopRequest { session_id },
    )
}

#[tauri::command]
pub fn poll_stack_terminal_session(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
) -> Result<StackTerminalPollResult, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::TerminalSessionTarget {
            command: crate::contracts::commands::POLL_STACK_TERMINAL_SESSION,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    terminal::poll_stack_terminal_session(&app_handle, &state, window.label(), session_id)
}

#[tauri::command]
pub fn list_stack_terminals(
    window: WebviewWindow,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    target_label: Option<String>,
) -> Result<Vec<StackTerminalSessionSnapshot>, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::TerminalSessionTarget {
            command: crate::contracts::commands::LIST_STACK_TERMINALS,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    terminal::list_stack_terminals(&state, window.label(), target_label)
}

#[tauri::command]
pub fn rename_stack_terminal(
    window: WebviewWindow,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
    title: String,
) -> Result<StackTerminalSessionSnapshot, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::TerminalSessionTarget {
            command: crate::contracts::commands::RENAME_STACK_TERMINAL,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    terminal::rename_stack_terminal(
        &state,
        window.label(),
        StackTerminalRenameRequest { session_id, title },
    )
}

#[tauri::command]
pub fn stop_terminal_panel_sessions(
    window: WebviewWindow,
    app_handle: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::STOP_TERMINAL_PANEL_SESSIONS,
            callers: &[crate::shell_windows::TERMINAL_PANEL_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    terminal::stop_terminal_sessions_for_target(
        &app_handle,
        &state,
        crate::shell_windows::TERMINAL_PANEL_LABEL,
    )
}

#[tauri::command]
pub fn get_stack_terminal_cwd(
    window: WebviewWindow,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    session_id: String,
) -> Result<StackTerminalSessionSnapshot, String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::TerminalSessionTarget {
            command: crate::contracts::commands::GET_STACK_TERMINAL_CWD,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    terminal::get_stack_terminal_cwd(&state, window.label(), session_id)
}

#[tauri::command]
pub fn open_stack_folder_in_vscode(window: WebviewWindow, path: String) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::OPEN_STACK_FOLDER_IN_VSCODE,
            callers: &[
                crate::shell_windows::TOP_BAR_LABEL,
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    let path = paths::normalize_existing_dir(&path)?;
    crate::shell_paths::open_folder_in_vscode(path)
}

#[tauri::command]
pub fn reveal_stack_item(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    path: String,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::REVEAL_STACK_ITEM,
            callers: &[
                crate::shell_windows::STACK_POPUP_LABEL,
                crate::shell_windows::TERMINAL_PANEL_LABEL,
            ],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    let path = paths::normalize_existing_path(&path)?;
    #[cfg(not(target_os = "windows"))]
    let _ = (&app, &state);
    #[cfg(target_os = "windows")]
    let demoted_hwnd = {
        popup_window::suppress_next_stack_popup_focus_loss(&state);
        popup_window::suppress_next_stack_popup_topmost_restore(&state);
        match demote_stack_popup_for_external_foreground(&app) {
            Ok(hwnd) => hwnd,
            Err(error) => {
                popup_window::clear_stack_popup_focus_loss_suppression(&state);
                popup_window::clear_stack_popup_topmost_restore_suppression(&state);
                return Err(error);
            }
        }
    };

    match file_ops::reveal_stack_item_path(path) {
        Ok(()) => Ok(()),
        Err(error) => {
            #[cfg(target_os = "windows")]
            {
                popup_window::clear_stack_popup_focus_loss_suppression(&state);
                popup_window::clear_stack_popup_topmost_restore_suppression(&state);
                if let Some(hwnd) = demoted_hwnd {
                    let _ = set_stack_popup_topmost(hwnd, true);
                }
            }
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn extract_stack_archive(
    window: WebviewWindow,
    archive_path: String,
    destination_mode: ArchiveDestinationMode,
    extractor: ArchiveExtractor,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::EXTRACT_STACK_ARCHIVE,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    let archive = PathBuf::from(paths::normalize_existing_path(&archive_path)?);
    if !archive.is_absolute() {
        return Err("Archive path must be absolute".to_string());
    }
    if !archive.is_file() {
        return Err("Archive path must be a file".to_string());
    }
    let kind =
        ArchiveKind::from_path(&archive).ok_or_else(|| "Unsupported archive type".to_string())?;
    let seven_zip = if extractor == ArchiveExtractor::SevenZip || kind == ArchiveKind::Rar {
        find_seven_zip()
    } else {
        None
    };
    let plan = build_archive_extraction_plan(&archive, destination_mode, extractor, seven_zip)?;
    tauri::async_runtime::spawn_blocking(move || run_archive_extraction_plan(plan))
        .await
        .map_err(|error| format!("Failed to join archive extraction task: {error}"))?
}

fn run_archive_extraction_plan(plan: ArchiveExtractionPlan) -> Result<(), String> {
    let timeout = archive_extraction_timeout();
    let spec = process_runner::ProcessRunSpec {
        program: plan.executable.to_string_lossy().to_string(),
        args: plan.args,
        cwd: plan.destination_path.parent().map(Path::to_path_buf),
        envs: vec![],
        stdin: None,
        timeout,
        stdout_cap: 64 * 1024,
        stderr_cap: 64 * 1024,
        poll_interval: Duration::from_millis(50),
        kill_tree: true,
    };
    match process_runner::run_process(spec) {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Archive extraction failed with status {}",
                    output.status
                ))
            }
        }
        Err(process_runner::ProcessRunError::Spawn(error)) => {
            Err(format!("Failed to extract archive: {error}"))
        }
        Err(process_runner::ProcessRunError::Timeout { .. }) => Err(format!(
            "Failed to extract archive: timed out after {}s",
            timeout.as_secs()
        )),
        Err(process_runner::ProcessRunError::NonZero { status, .. }) => Err(format!(
            "Archive extraction failed with status {}",
            status.map_or_else(|| "unknown".to_string(), |code| code.to_string())
        )),
        Err(process_runner::ProcessRunError::CleanupIncomplete { reason, .. }) => {
            Err(format!("Failed to extract archive: {reason}"))
        }
    }
}

fn archive_extraction_timeout() -> Duration {
    const DEFAULT_SECONDS: u64 = 600;
    const MIN_SECONDS: u64 = 30;
    const MAX_SECONDS: u64 = 3600;
    let seconds = std::env::var("JASONSHELL_ARCHIVE_EXTRACTION_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SECONDS)
        .clamp(MIN_SECONDS, MAX_SECONDS);
    Duration::from_secs(seconds)
}

#[tauri::command]
pub fn show_stack_item_properties(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Mutex<StackPopupRuntimeState>>,
    path: String,
) -> Result<(), String> {
    authorize_stack_command(
        &window,
        StackCommandAuth::AllowedCallers {
            command: crate::contracts::commands::SHOW_STACK_ITEM_PROPERTIES,
            callers: &[crate::shell_windows::STACK_POPUP_LABEL],
        },
    )
    .map_err(CallerAuthError::into_string)?;
    let path = PathBuf::from(paths::normalize_existing_path(&path)?);
    let plan = build_stack_item_properties_plan(&path)?;
    popup_window::suppress_next_stack_popup_focus_loss(&state);
    popup_window::suppress_next_stack_popup_topmost_restore(&state);
    match show_stack_item_properties_native(&app, &plan) {
        Ok(()) => Ok(()),
        Err(error) => {
            popup_window::clear_stack_popup_focus_loss_suppression(&state);
            popup_window::clear_stack_popup_topmost_restore_suppression(&state);
            Err(error)
        }
    }
}

#[cfg(target_os = "windows")]
fn show_stack_item_properties_native(
    app: &AppHandle,
    plan: &StackItemPropertiesPlan,
) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_INVOKEIDLIST, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn to_wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    let owner_hwnd = stack_popup_owner_hwnd(app)?;
    set_stack_popup_topmost(owner_hwnd, false)?;
    let path_wide = to_wide(plan.path.as_os_str());
    let verb_wide = to_wide(OsStr::new(plan.verb));
    let mut execute_info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: if plan.invoke_id_list {
            SEE_MASK_INVOKEIDLIST
        } else {
            0
        },
        hwnd: owner_hwnd,
        lpVerb: PCWSTR(verb_wide.as_ptr()),
        lpFile: PCWSTR(path_wide.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    match unsafe { ShellExecuteExW(&mut execute_info) } {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = set_stack_popup_topmost(owner_hwnd, true);
            Err(format!(
                "ShellExecuteExW failed to show properties for {}: {error}",
                plan.path.display()
            ))
        }
    }
}

#[cfg(target_os = "windows")]
fn demote_stack_popup_for_external_foreground(
    app: &AppHandle,
) -> Result<Option<windows::Win32::Foundation::HWND>, String> {
    use tauri::Manager;

    if app
        .get_webview_window(crate::shell_windows::STACK_POPUP_LABEL)
        .is_none()
    {
        return Ok(None);
    }

    let hwnd = stack_popup_owner_hwnd(app)?;
    set_stack_popup_topmost(hwnd, false)?;
    Ok(Some(hwnd))
}

#[cfg(target_os = "windows")]
fn stack_popup_owner_hwnd(app: &AppHandle) -> Result<windows::Win32::Foundation::HWND, String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use tauri::Manager;
    use windows::Win32::Foundation::HWND;

    let window = app
        .get_webview_window(crate::shell_windows::STACK_POPUP_LABEL)
        .ok_or_else(|| "Stack popup window is unavailable".to_string())?;
    let handle = window
        .window_handle()
        .map_err(|error| format!("Failed to get stack popup window handle: {error}"))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut _)),
        other => Err(format!("Unsupported stack popup window handle: {other:?}")),
    }
}

#[cfg(target_os = "windows")]
fn set_stack_popup_topmost(
    hwnd: windows::Win32::Foundation::HWND,
    topmost: bool,
) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };

    let insert_after = if topmost {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    unsafe {
        SetWindowPos(
            hwnd,
            Some(insert_after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .map_err(|error| format!("Failed to update stack popup z-order: {error}"))
}

#[cfg(not(target_os = "windows"))]
fn show_stack_item_properties_native(
    _app: &AppHandle,
    _plan: &StackItemPropertiesPlan,
) -> Result<(), String> {
    Err("Stack item properties are only supported on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        available_destination_path, backup_corrupt_pin_store, clipboard_mode_from_drop_effect,
        copy_dir, move_path_with_rename, native_drag_mechanism, next_new_text_document_path,
        open_with_candidates_for_extension_with_resolver, paste_clipboard_items,
        paths_match_for_unpin, read_stack_folder_page, read_stack_folder_page_with_session,
        reorder_pins_by_paths, resolve_stack_alias_with_profile, resolve_stack_item_icons_batch,
        resolve_stack_item_icons_for_paths, resolve_stack_item_icons_for_paths_async,
        stack_file_attributes_from_bits, stack_folder_warning, stack_item_from_path,
        validate_child_name, validate_stack_git_remote_url, windows_explorer_reveal_launch_plan,
        windows_explorer_reveal_select_arg, windows_explorer_reveal_show_mode, ClipboardMode,
        PinnedStackFolder, ShowStackPopupRequest, StackClipboard, StackItem,
        WindowsExplorerRevealShowMode,
    };
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    #[test]
    fn stack_git_remote_url_validation_allows_only_safe_browser_urls() {
        assert_eq!(
            validate_stack_git_remote_url("https://github.com/acme/repo").as_deref(),
            Ok("https://github.com/acme/repo")
        );
        assert_eq!(
            validate_stack_git_remote_url(" http://gitlab.com/acme/repo ").as_deref(),
            Ok("http://gitlab.com/acme/repo")
        );
        assert!(validate_stack_git_remote_url("git@github.com:acme/repo").is_err());
        assert!(validate_stack_git_remote_url("file:///C:/repo").is_err());
        assert!(validate_stack_git_remote_url("javascript:alert(1)").is_err());
        assert!(validate_stack_git_remote_url("https://user:token@github.com/acme/repo").is_err());
        assert!(validate_stack_git_remote_url("https://github.com/acme/re po").is_err());
        assert!(validate_stack_git_remote_url("https://github.com/acme/repo\0bad").is_err());
    }

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

        let suggestions =
            super::suggest_stack_paths(root.to_string_lossy().into_owned(), "".into(), Some(1))
                .unwrap();

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

        let error =
            super::suggest_stack_paths(file.to_string_lossy().into_owned(), "".into(), Some(20))
                .unwrap_err();

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
        let journal_dir = root.join("journal");
        fs::create_dir_all(&source_dir).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&journal_dir).unwrap();
        let good = source_dir.join("good.txt");
        let missing = source_dir.join("missing.txt");
        fs::write(&good, b"ok").unwrap();

        let clipboard = StackClipboard {
            mode: ClipboardMode::Copy,
            paths: vec![good, missing.clone()],
        };
        let result = paste_clipboard_items(&clipboard, &destination, Some(&journal_dir));

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
        let item = super::file_ops::new_stack_folder_path(
            root.to_str().unwrap().to_string(),
            folder_name.to_string(),
        )
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
        tauri::async_runtime::block_on(super::file_ops::delete_stack_item_path_async(
            root.join("File.txt").to_str().unwrap().to_string(),
        ))
        .unwrap();
        assert!(!root.join("File.txt").exists());
        tauri::async_runtime::block_on(super::file_ops::delete_stack_item_path_async(
            root.join("Folder").to_str().unwrap().to_string(),
        ))
        .unwrap();
        assert!(!root.join("Folder").exists());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn async_delete_deletes_nested_folder_and_preserves_missing_error() {
        let root = test_dir("async-delete-test");
        fs::create_dir_all(root.join("Folder").join("Nested")).unwrap();
        fs::write(root.join("Folder").join("Nested").join("File.txt"), b"x").unwrap();

        tauri::async_runtime::block_on(super::file_ops::delete_stack_item_path_async(
            root.join("Folder").to_string_lossy().to_string(),
        ))
        .unwrap();

        assert!(!root.join("Folder").exists());
        let missing =
            tauri::async_runtime::block_on(super::file_ops::delete_stack_item_path_async(
                root.join("Missing").to_string_lossy().to_string(),
            ))
            .unwrap_err();
        assert!(missing.contains("Failed to resolve stack path"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn archive_extraction_runner_preserves_process_error_contract() {
        let root = test_dir("archive-runner-error");
        fs::create_dir_all(&root).unwrap();
        let plan = super::ArchiveExtractionPlan {
            executable: root.join("missing-extractor.exe"),
            args: Vec::new(),
            destination_path: root.clone(),
            expected_created_folder: None,
        };

        let error = super::run_archive_extraction_plan(plan).unwrap_err();

        assert!(error.starts_with("Failed to extract archive:"));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn archive_extraction_timeout_is_clamped_and_source_avoids_raw_status() {
        std::env::set_var("JASONSHELL_ARCHIVE_EXTRACTION_TIMEOUT_SECS", "5");
        assert_eq!(super::archive_extraction_timeout(), Duration::from_secs(30));
        std::env::set_var("JASONSHELL_ARCHIVE_EXTRACTION_TIMEOUT_SECS", "7200");
        assert_eq!(
            super::archive_extraction_timeout(),
            Duration::from_secs(3600)
        );
        std::env::remove_var("JASONSHELL_ARCHIVE_EXTRACTION_TIMEOUT_SECS");

        let stack_source = include_str!("stack_popup.rs");
        let archive_body = stack_source
            .split("pub async fn extract_stack_archive(")
            .nth(1)
            .and_then(|value| value.split("fn run_archive_extraction_plan").next())
            .expect("archive extraction body present");
        assert!(archive_body.contains(
            "tauri::async_runtime::spawn_blocking(move || run_archive_extraction_plan(plan))"
        ));
        assert!(!archive_body.contains(".status()"));

        let runner_body = stack_source
            .split("fn run_archive_extraction_plan(plan: ArchiveExtractionPlan) -> Result<(), String> {")
            .nth(1)
            .and_then(|value| value.split("#[cfg(test)]").next())
            .expect("archive runner body present");
        assert!(runner_body.contains("process_runner::ProcessRunSpec"));
        assert!(!runner_body.contains("Command::new"));
    }

    #[test]
    fn stack_long_running_file_op_commands_use_spawn_blocking_boundaries() {
        let stack_source = include_str!("stack_popup.rs");
        let file_ops_source = include_str!("stack_popup/file_ops.rs");
        let clipboard_source = include_str!("stack_popup/clipboard.rs");

        assert!(stack_source.contains("pub async fn paste_stack_items("));
        assert!(stack_source
            .contains("clipboard::paste_stack_clipboard_items_async(&state, destination).await"));
        assert!(stack_source.contains("pub async fn delete_stack_item("));
        assert!(stack_source.contains("file_ops::delete_stack_item_path_async(path).await"));
        assert!(stack_source.contains("pub async fn extract_stack_archive("));
        assert!(stack_source.contains(
            "tauri::async_runtime::spawn_blocking(move || run_archive_extraction_plan(plan))"
        ));

        assert!(file_ops_source.contains("pub(crate) async fn delete_stack_item_path_async("));
        assert!(file_ops_source
            .contains("tauri::async_runtime::spawn_blocking(move || delete_path(&target))"));
        assert!(clipboard_source.contains("pub(crate) async fn paste_stack_clipboard_items_async("));
        assert!(clipboard_source.contains(
            "tauri::async_runtime::spawn_blocking(move || {\r\n        paste_clipboard_items(&clipboard, &destination, journal_dir.as_deref())\r\n    })"
        ) || clipboard_source.contains(
            "tauri::async_runtime::spawn_blocking(move || {\n        paste_clipboard_items(&clipboard, &destination, journal_dir.as_deref())\n    })"
        ));

        let paste_body = clipboard_source
            .split("pub(crate) async fn paste_stack_clipboard_items_async(")
            .nth(1)
            .and_then(|value| value.split("fn update_cut_clipboard_after_paste").next())
            .expect("paste body present");
        assert!(paste_body.contains("recovery_journal_dir(app_handle)?"));
        assert!(paste_body.contains("journal_dir.as_deref()"));
    }

    #[test]
    fn stack_phase1_matrix_commands_are_guarded_in_source() {
        let stack_source = include_str!("stack_popup.rs");
        for command in [
            "OPEN_STACK_ITEM",
            "COPY_STACK_ITEMS",
            "CUT_STACK_ITEMS",
            "PASTE_STACK_ITEMS",
            "DELETE_STACK_ITEM",
            "NEW_STACK_FOLDER",
            "NEW_STACK_TEXT_FILE",
            "REVEAL_STACK_ITEM",
            "OPEN_STACK_FOLDER_IN_VSCODE",
        ] {
            assert!(stack_source.contains(command), "missing {command}");
            assert!(
                stack_source.contains("authorize_stack_command"),
                "missing guard for {command}"
            );
        }
    }

    #[test]
    fn stack_popup_setwindowpos_uses_noactivate_for_z_order_changes() {
        let stack_source = include_str!("stack_popup.rs");

        assert!(stack_source.contains("SWP_NOACTIVATE"));
        assert!(stack_source.contains("SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE"));
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

        assert_eq!(
            super::ArchiveKind::from_path(&root.join("bundle.zip")),
            Some(super::ArchiveKind::Zip)
        );
        assert_eq!(
            super::ArchiveKind::from_path(&root.join("BUNDLE.ZIP")),
            Some(super::ArchiveKind::Zip)
        );
        assert_eq!(
            super::ArchiveKind::from_path(&root.join("bundle.rar")),
            Some(super::ArchiveKind::Rar)
        );
        assert_eq!(super::ArchiveKind::from_path(&root.join("bundle.7z")), None);
        assert_eq!(super::ArchiveKind::from_path(&root.join("bundle")), None);
        assert_eq!(
            super::ArchiveKind::from_path(&root.join("folder.zip")),
            None
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn archive_7zip_candidates_include_program_files_and_path_fallback() {
        let candidates = super::seven_zip_discovery_candidates();
        assert!(candidates
            .iter()
            .any(|candidate| candidate.ends_with(r"7-Zip\7z.exe")));
        assert!(!candidates
            .iter()
            .any(|candidate| candidate == Path::new("7z.exe")));
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
        )
        .unwrap();

        assert_eq!(plan.executable, root.join("Tools Dir").join("7z.exe"));
        assert!(plan
            .args
            .iter()
            .any(|arg| arg == &archive.to_string_lossy().to_string()));
        assert!(plan
            .args
            .iter()
            .any(|arg| arg == &format!("-o{}", root.join("release build").to_string_lossy())));
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
        )
        .unwrap();
        assert_eq!(builtin.executable, Path::new("powershell.exe"));
        assert!(builtin
            .args
            .iter()
            .any(|arg| arg.contains("Expand-Archive")));

        let seven_zip_path = root.join("7z.exe");
        let seven_zip = super::build_archive_extraction_plan(
            &archive,
            super::ArchiveDestinationMode::Folder,
            super::ArchiveExtractor::SevenZip,
            Some(seven_zip_path.clone()),
        )
        .unwrap();
        assert_eq!(seven_zip.executable, seven_zip_path);
        assert!(seven_zip.args.iter().any(|arg| arg == "x"));

        let missing_seven_zip = super::build_archive_extraction_plan(
            &archive,
            super::ArchiveDestinationMode::Folder,
            super::ArchiveExtractor::SevenZip,
            None,
        )
        .unwrap_err();
        assert_eq!(
            missing_seven_zip,
            "7-Zip is required to use 7-Zip extraction"
        );

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
        assert_eq!(
            page.items
                .iter()
                .map(|item| (&item.name, &item.kind))
                .collect::<Vec<_>>(),
            vec![
                (&"docs".to_string(), &"folder".to_string()),
                (&"app.exe".to_string(), &"file".to_string())
            ]
        );
        assert!(page
            .items
            .iter()
            .any(|item| item.path.ends_with(r"bundle.zip\docs")));

        let nested_path = format!(r"{}\docs", archive.to_string_lossy());
        let nested = super::read_stack_folder_page(&nested_path, 0, 20).unwrap();
        assert_eq!(
            nested
                .items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["readme.md"]
        );

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

        let page =
            super::read_stack_folder(archive.to_string_lossy().to_string(), 0, Some(20), None)
                .unwrap();
        assert_eq!(
            page.items
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            vec!["app.exe"]
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn stack_item_properties_plan_validates_existing_paths() {
        let root = test_dir("properties-plan");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("server file.ts");
        fs::write(&file, b"ts").unwrap();

        let plan = super::build_stack_item_properties_plan(&file).unwrap();
        assert_eq!(plan.path, file);
        assert_eq!(plan.verb, "properties");
        assert!(plan.invoke_id_list);

        assert!(super::build_stack_item_properties_plan(&root.join("missing.ts")).is_err());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn windows_explorer_reveal_select_arg_requests_new_window_and_keeps_select_path_together() {
        let path = PathBuf::from(r"C:\Users\Jason\server file.ts");

        assert_eq!(
            windows_explorer_reveal_select_arg(&path),
            r"/n,/select,C:\Users\Jason\server file.ts"
        );
    }

    #[test]
    fn windows_explorer_reveal_show_mode_maximizes_only_hidden_directories() {
        assert_eq!(
            windows_explorer_reveal_show_mode(true, true),
            WindowsExplorerRevealShowMode::Maximized
        );
        assert_eq!(
            windows_explorer_reveal_show_mode(true, false),
            WindowsExplorerRevealShowMode::Restored
        );
        assert_eq!(
            windows_explorer_reveal_show_mode(false, true),
            WindowsExplorerRevealShowMode::Restored
        );
    }

    #[test]
    fn windows_explorer_reveal_launch_plan_preserves_fixed_executable_and_single_parameter() {
        let root = test_dir("reveal-launch-plan");
        fs::create_dir_all(&root).unwrap();
        let file = root.join("server file.ts");
        fs::write(&file, b"ts").unwrap();

        let plan = windows_explorer_reveal_launch_plan(&file).unwrap();

        assert_eq!(plan.executable, "explorer.exe");
        assert_eq!(
            plan.parameters,
            format!("/n,/select,{}", file.to_string_lossy())
        );
        assert_eq!(plan.show_mode, WindowsExplorerRevealShowMode::Restored);
        fs::remove_dir_all(root).ok();
    }
}
