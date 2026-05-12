use crate::stack_popup::items::{metadata_is_reparse_point, stack_item_from_path};
use crate::stack_popup::models::StackItem;
use crate::stack_popup::paths::{
    normalize_existing_dir, normalize_existing_path, validate_child_name,
};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn rename_stack_item_path(path: String, new_name: String) -> Result<StackItem, String> {
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

pub(crate) async fn delete_stack_item_path_async(path: String) -> Result<(), String> {
    let target = PathBuf::from(normalize_existing_path(&path)?);
    tauri::async_runtime::spawn_blocking(move || delete_path(&target))
        .await
        .map_err(|error| format!("Failed to join stack delete task: {error}"))?
}

pub(crate) fn new_stack_folder_path(parent: String, name: String) -> Result<StackItem, String> {
    let parent = PathBuf::from(normalize_existing_dir(&parent)?);
    let name = validate_child_name(&name)?;
    let destination = parent.join(name);
    if destination.exists() {
        return Err("A file or folder with that name already exists".to_string());
    }
    fs::create_dir(&destination).map_err(|error| format!("Failed to create folder: {error}"))?;
    stack_item_from_path(destination)
}

pub(crate) fn new_stack_text_file_path(parent: String) -> Result<StackItem, String> {
    let parent = PathBuf::from(normalize_existing_dir(&parent)?);
    let destination = next_new_text_document_path(&parent)?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
        .map_err(|error| format!("Failed to create text file: {error}"))?;
    stack_item_from_path(destination)
}

pub(crate) fn open_terminal_here_path(path: String) -> Result<(), String> {
    let directory = PathBuf::from(normalize_existing_dir(&path)?);
    launch_terminal_at_dir(&directory)
}

pub(crate) fn next_new_text_document_path(parent: &Path) -> Result<PathBuf, String> {
    let first = parent.join("New Text Document.txt");
    if !first.exists() {
        return Ok(first);
    }

    for index in 2..1000 {
        let candidate = parent.join(format!("New Text Document ({index}).txt"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err("Could not choose a New Text Document name".to_string())
}

#[cfg(target_os = "windows")]
fn launch_terminal_at_dir(directory: &Path) -> Result<(), String> {
    use std::process::Command;

    if Command::new("wt.exe")
        .arg("-d")
        .arg(directory)
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    if Command::new("powershell.exe")
        .arg("-NoExit")
        .arg("-NoLogo")
        .arg("-Command")
        .arg("Set-Location -LiteralPath $args[0]")
        .arg(directory)
        .spawn()
        .is_ok()
    {
        return Ok(());
    }

    Command::new("cmd.exe")
        .arg("/K")
        .arg("cd")
        .arg("/d")
        .arg(directory)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to open terminal here: {error}"))
}

#[cfg(not(target_os = "windows"))]
fn launch_terminal_at_dir(_directory: &Path) -> Result<(), String> {
    Err("Open Terminal Here is only available on Windows".to_string())
}

pub(crate) fn reveal_stack_item_path(path: String) -> Result<(), String> {
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
        crate::shell_paths::open_shell_path(path)
    }
}

pub(crate) fn available_destination_path(
    destination: &Path,
    source: &Path,
) -> Result<PathBuf, String> {
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

pub(crate) fn copy_path(source: &Path, destination: &Path) -> Result<(), String> {
    let mut visited = HashSet::new();
    copy_path_inner(source, destination, &mut visited)
}

pub(crate) fn move_path_with_fallback(source: &Path, destination: &Path) -> Result<(), String> {
    move_path_with_rename(source, destination, |source, destination| {
        fs::rename(source, destination)
    })
}

pub(crate) fn move_path_with_rename<F>(
    source: &Path,
    destination: &Path,
    rename: F,
) -> Result<(), String>
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
pub(crate) fn copy_dir(source: &Path, destination: &Path) -> Result<(), String> {
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

pub(crate) fn ensure_paste_destination_allowed(
    source: &Path,
    destination: &Path,
) -> Result<(), String> {
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
