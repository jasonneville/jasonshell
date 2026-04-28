use crate::stack_popup::file_ops::{
    available_destination_path, copy_path, ensure_paste_destination_allowed,
    move_path_with_fallback,
};
use crate::stack_popup::items::stack_item_from_path;
use crate::stack_popup::models::{
    ClipboardMode, StackClipboard, StackPasteFailure, StackPasteResult, StackPopupRuntimeState,
};
use crate::stack_popup::paths::{normalize_existing_dir, normalize_existing_path};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

pub(crate) fn set_stack_clipboard(
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

pub(crate) fn paste_stack_clipboard_items(
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    destination: String,
) -> Result<StackPasteResult, String> {
    let destination = PathBuf::from(normalize_existing_dir(&destination)?);
    let used_internal_clipboard = state
        .lock()
        .expect("stack popup runtime state is poisoned")
        .clipboard
        .is_some();
    let clipboard = clipboard_for_paste(state)?;
    let result = paste_clipboard_items(&clipboard, &destination);

    if matches!(clipboard.mode, ClipboardMode::Cut) {
        update_cut_clipboard_after_paste(state, used_internal_clipboard, &result);
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

pub(crate) fn paste_clipboard_items(
    clipboard: &StackClipboard,
    destination: &Path,
) -> StackPasteResult {
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
) -> Result<crate::stack_popup::models::StackItem, String> {
    ensure_paste_destination_allowed(source, destination)?;
    let target = available_destination_path(destination, source)?;
    match mode {
        ClipboardMode::Copy => copy_path(source, &target)?,
        ClipboardMode::Cut => move_path_with_fallback(source, &target)?,
    }
    stack_item_from_path(target)
}

pub(crate) fn clipboard_mode_from_drop_effect(effect: u32) -> ClipboardMode {
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
