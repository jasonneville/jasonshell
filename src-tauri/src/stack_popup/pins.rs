use crate::stack_popup::models::PinnedStackFolder;
use crate::stack_popup::paths::{normalize_existing_dir, paths_match_for_unpin, user_profile_dir};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::{AppHandle, Manager};

const PIN_STORE_FILE: &str = "stack-folders-v1.json";

pub(crate) fn load_pins_with_defaults(
    app_handle: &AppHandle,
) -> Result<Vec<PinnedStackFolder>, String> {
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

pub(crate) fn pin_folder(
    app_handle: &AppHandle,
    path: &str,
) -> Result<Vec<PinnedStackFolder>, String> {
    let folder = pinned_folder_from_path(path)?;
    let mut pins = load_pins_with_defaults(app_handle)?;
    if !pins
        .iter()
        .any(|pin| pin.path.eq_ignore_ascii_case(&folder.path))
    {
        pins.push(folder);
        save_pins(app_handle, &pins)?;
    }
    Ok(pins)
}

pub(crate) fn unpin_folder(
    app_handle: &AppHandle,
    path: &str,
) -> Result<Vec<PinnedStackFolder>, String> {
    let mut pins = load_pins_with_defaults(app_handle)?;
    pins.retain(|pin| !paths_match_for_unpin(&pin.path, path));
    save_pins(app_handle, &pins)?;
    Ok(pins)
}

pub(crate) fn reorder_pinned_folders(
    app_handle: &AppHandle,
    ordered_paths: &[String],
) -> Result<Vec<PinnedStackFolder>, String> {
    let pins = load_pins_with_defaults(app_handle)?;
    let pins = reorder_pins_by_paths(pins, ordered_paths);
    save_pins(app_handle, &pins)?;
    Ok(pins)
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

pub(crate) fn backup_corrupt_pin_store(path: &Path) -> Result<(), String> {
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

pub(crate) fn reorder_pins_by_paths(
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

pub(crate) fn pinned_folder_from_path(path: &str) -> Result<PinnedStackFolder, String> {
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
