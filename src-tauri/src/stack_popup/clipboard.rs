use crate::stack_popup::file_ops::{
    available_destination_path, copy_path_with_journal, ensure_paste_destination_allowed,
    move_path_with_fallback_journal,
};
use crate::stack_popup::items::stack_item_from_path;
use crate::stack_popup::models::{
    ClipboardMode, StackClipboard, StackPasteFailure, StackPasteResult, StackPopupRuntimeState,
};
use crate::stack_popup::paths::{normalize_existing_dir, normalize_existing_path};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

#[cfg(target_os = "windows")]
struct ClipboardSession;

#[cfg(target_os = "windows")]
impl ClipboardSession {
    fn open() -> Result<Self, String> {
        use windows::Win32::System::DataExchange::OpenClipboard;

        unsafe { OpenClipboard(None).map_err(|error| format!("Failed to open clipboard: {error}"))? };
        Ok(Self)
    }
}

#[cfg(target_os = "windows")]
impl Drop for ClipboardSession {
    fn drop(&mut self) {
        use windows::Win32::System::DataExchange::CloseClipboard;

        unsafe {
            CloseClipboard().ok();
        }
    }
}

#[cfg(target_os = "windows")]
struct GlobalLockGuard {
    handle: windows::Win32::Foundation::HGLOBAL,
    ptr: *mut core::ffi::c_void,
}

#[cfg(target_os = "windows")]
impl GlobalLockGuard {
    fn lock(handle: windows::Win32::Foundation::HGLOBAL) -> Result<Self, String> {
        use windows::Win32::System::Memory::GlobalLock;

        let ptr = unsafe { GlobalLock(handle) };
        if ptr.is_null() {
            return Err("Failed to lock clipboard memory".to_string());
        }
        Ok(Self { handle, ptr })
    }

    fn as_mut_ptr(&self) -> *mut u8 {
        self.ptr.cast::<u8>()
    }
}

#[cfg(target_os = "windows")]
impl Drop for GlobalLockGuard {
    fn drop(&mut self) {
        use windows::Win32::System::Memory::GlobalUnlock;

        unsafe {
            GlobalUnlock(self.handle).ok();
        }
    }
}

#[cfg(target_os = "windows")]
struct OwnedGlobalMem {
    handle: windows::Win32::Foundation::HGLOBAL,
    owned: bool,
}

#[cfg(target_os = "windows")]
impl OwnedGlobalMem {
    fn allocate(bytes: usize) -> Result<Self, String> {
        use windows::Win32::System::Memory::{GlobalAlloc, GMEM_MOVEABLE};

        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) }
            .map_err(|error| format!("Failed to allocate clipboard memory: {error}"))?;
        Ok(Self { handle, owned: true })
    }

    fn handle(&self) -> windows::Win32::Foundation::HGLOBAL {
        self.handle
    }

    fn into_handle(mut self) -> windows::Win32::Foundation::HGLOBAL {
        self.owned = false;
        self.handle
    }

    fn disarm(&mut self) {
        self.owned = false;
    }
}

#[cfg(target_os = "windows")]
impl Drop for OwnedGlobalMem {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                #[link(name = "kernel32")]
                extern "system" {
                    fn GlobalFree(hMem: windows::Win32::Foundation::HGLOBAL)
                        -> windows::Win32::Foundation::HGLOBAL;
                }
                let _ = GlobalFree(self.handle);
            }
        }
    }
}

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

pub(crate) async fn paste_stack_clipboard_items_async(
    app_handle: &AppHandle,
    state: &State<'_, Mutex<StackPopupRuntimeState>>,
    destination: String,
) -> Result<StackPasteResult, String> {
    let destination = PathBuf::from(normalize_existing_dir(&destination)?);
    let emergency_disable = crate::stack_popup::recovery_journal::emergency_recovery_journal_disable();
    let journal_dir = if emergency_disable { None } else { Some(recovery_journal_dir(app_handle)?) };
    let used_internal_clipboard = state
        .lock()
        .expect("stack popup runtime state is poisoned")
        .clipboard
        .is_some();
    let clipboard = clipboard_for_paste(state)?;
    let mode = clipboard.mode;
    let result = tauri::async_runtime::spawn_blocking(move || {
        paste_clipboard_items(&clipboard, &destination, journal_dir.as_deref())
    })
    .await
    .map_err(|error| format!("Failed to join stack paste task: {error}"))?;

    if matches!(mode, ClipboardMode::Cut) {
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
    journal_dir: Option<&Path>,
) -> StackPasteResult {
    let mut pasted = Vec::new();
    let mut failures = Vec::new();

    for source in &clipboard.paths {
        match paste_one_clipboard_item(clipboard.mode, source, destination, journal_dir) {
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
    journal_dir: Option<&Path>,
) -> Result<crate::stack_popup::models::StackItem, String> {
    ensure_paste_destination_allowed(source, destination)?;
    let target = available_destination_path(destination, source)?;
    match mode {
        ClipboardMode::Copy => copy_path_with_journal(source, &target, journal_dir.as_deref())?,
        ClipboardMode::Cut => move_path_with_fallback_journal(source, &target, journal_dir.as_deref())?,
    }
    stack_item_from_path(target)
}

fn recovery_journal_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    app_handle
        .path()
        .app_local_data_dir()
        .map(|dir| crate::stack_popup::recovery_journal::recovery_journal_dir(dir.as_path()))
        .map_err(|error| format!("failed to resolve recovery journal directory: {error}"))
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
    let _session = ClipboardSession::open()?;

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
            let memory = GlobalLockGuard::lock(HGLOBAL(effect_handle.0));
            if memory.is_err() {
                ClipboardMode::Copy
            } else {
                let memory = memory.unwrap();
                let effect = unsafe { *(memory.as_mut_ptr() as *mut u32) };
                clipboard_mode_from_drop_effect(effect)
            }
        } else {
            ClipboardMode::Copy
        };

        Ok(Some(StackClipboard { mode, paths }))
    })();

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
    let mut hdrop_owner = OwnedGlobalMem::allocate(dropfiles_size + paths_size)?;
    let hdrop = unsafe {
        let memory = GlobalLockGuard::lock(hdrop_owner.handle())?;
        let dropfiles = memory.as_mut_ptr() as *mut DROPFILES;
        *dropfiles = DROPFILES {
            pFiles: dropfiles_size as u32,
            pt: POINT { x: 0, y: 0 },
            fNC: false.into(),
            fWide: true.into(),
        };
        std::ptr::copy_nonoverlapping(
            encoded_paths.as_ptr(),
            memory.as_mut_ptr().add(dropfiles_size) as *mut u16,
            encoded_paths.len(),
        );
        hdrop_owner.handle()
    };

    // SAFETY: Allocates a movable global memory block for one u32 drop-effect
    // value, writes the initialized value while locked, then unlocks before
    // transferring ownership to the clipboard.
    let mut effect_owner = OwnedGlobalMem::allocate(size_of::<u32>())?;
    let effect_handle = unsafe {
        let memory = GlobalLockGuard::lock(effect_owner.handle())?;
        let effect = match mode {
            ClipboardMode::Copy => 1u32,
            ClipboardMode::Cut => 2u32,
        };
        *(memory.as_mut_ptr().cast::<u32>()) = effect;
        effect_owner.handle()
    };

    let format_name = to_wide("Preferred DropEffect");
    // SAFETY: Opens the process clipboard, publishes ownership of the allocated
    // HGLOBAL handles with SetClipboardData, then closes the clipboard before return.
    unsafe {
        let _session = ClipboardSession::open()?;
        EmptyClipboard().map_err(|error| format!("Failed to empty clipboard: {error}"))?;
        let effect_format = RegisterClipboardFormatW(PCWSTR(format_name.as_ptr()));
        if effect_format != 0 {
            SetClipboardData(effect_format, Some(HANDLE(effect_handle.0)))
                .map_err(|error| format!("Failed to set clipboard drop effect: {error}"))?;
            effect_owner.disarm();
        }
        match SetClipboardData(CF_HDROP.0 as u32, Some(HANDLE(hdrop.0))) {
            Ok(_) => {
                let _ = hdrop_owner.into_handle();
                let _ = effect_owner.into_handle();
            }
            Err(error) => {
                let cleanup_error = EmptyClipboard()
                    .err()
                    .map(|cleanup| format!("Failed to empty clipboard after file clipboard publish failure: {cleanup}"));
                return Err(cleanup_error
                    .map(|cleanup| format!("Failed to set file clipboard data: {error}; {cleanup}"))
                    .unwrap_or_else(|| format!("Failed to set file clipboard data: {error}")));
            }
        }
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
