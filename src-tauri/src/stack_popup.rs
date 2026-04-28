mod clipboard;
mod file_ops;
mod items;
mod models;
mod paging;
mod paths;
mod pins;
mod popup_window;

use crate::shell_paths;
use std::path::Path;
use std::sync::Mutex;
use tauri::{AppHandle, State};

pub use models::{
    PinnedStackFolder, ShowStackPopupRequest, StackFolderPage, StackItem, StackPasteResult,
    StackPopupRuntimeState,
};

#[cfg(test)]
pub(crate) use clipboard::{clipboard_mode_from_drop_effect, paste_clipboard_items};
#[cfg(test)]
pub(crate) use file_ops::{available_destination_path, copy_dir, copy_path, move_path_with_rename};
#[cfg(test)]
pub(crate) use items::{stack_file_attributes_from_bits, stack_item_from_path};
#[cfg(test)]
pub(crate) use models::{ClipboardMode, StackClipboard};
#[cfg(test)]
pub(crate) use paging::{read_stack_folder_page, stack_folder_warning};
#[cfg(test)]
pub(crate) use paths::{
    normalize_existing_path, normalize_stack_path_candidate, paths_match_for_unpin,
    resolve_stack_alias_with_profile, stack_display_path_string, validate_child_name,
};
#[cfg(test)]
pub(crate) use pins::{backup_corrupt_pin_store, reorder_pins_by_paths};
#[cfg(test)]
pub(crate) use popup_window::normalize_show_stack_popup_request;

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
pub fn read_stack_folder(
    path: String,
    offset: usize,
    limit: Option<usize>,
) -> Result<StackFolderPage, String> {
    let folder = paths::normalize_existing_dir(&path)?;
    paging::read_stack_folder_page(&folder, offset, limit.unwrap_or(paging::DEFAULT_PAGE_LIMIT))
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
pub fn delete_stack_item(path: String) -> Result<(), String> {
    file_ops::delete_stack_item_path(path)
}

#[tauri::command]
pub fn new_stack_folder(parent: String, name: String) -> Result<StackItem, String> {
    file_ops::new_stack_folder_path(parent, name)
}

#[tauri::command]
pub fn reveal_stack_item(path: String) -> Result<(), String> {
    file_ops::reveal_stack_item_path(path)
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
