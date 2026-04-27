#[tauri::command]
pub fn open_shell_path(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Shell path is empty".to_string());
    }

    open_path(path)
}

pub fn open_shell_path_with_picker(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Shell path is empty".to_string());
    }

    open_with_picker(path)
}

#[cfg(target_os = "windows")]
fn open_path(path: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let path_wide = to_wide(path);
    // SAFETY: `path_wide` is a NUL-terminated UTF-16 buffer that lives for the
    // duration of the call, all optional pointer parameters are either null or
    // valid constants, and ShellExecuteW does not retain those pointers.
    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            None,
            PCWSTR(path_wide.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as isize;
    if code <= 32 {
        return Err(format!("ShellExecuteW failed for {path} with code {code}"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_with_picker(path: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let verb_wide = to_wide("openas");
    let path_wide = to_wide(path);
    // SAFETY: `verb_wide` and `path_wide` are NUL-terminated UTF-16 buffers
    // that remain alive for the duration of the call. The HWND is intentionally
    // null so Windows owns the Open With picker UI, and ShellExecuteW does not
    // retain the passed pointers after it returns.
    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            PCWSTR(verb_wide.as_ptr()),
            PCWSTR(path_wide.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as isize;
    if code <= 32 {
        return Err(format!(
            "ShellExecuteW Open With failed for {path} with code {code}"
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_path(path: &str) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to open shell path: {error}"))
}

#[cfg(not(target_os = "windows"))]
fn open_with_picker(_path: &str) -> Result<(), String> {
    Err("Open with is only available on Windows".to_string())
}

#[cfg(target_os = "windows")]
fn to_wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    value
        .as_ref()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
