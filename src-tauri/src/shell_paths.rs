#[tauri::command]
pub fn open_shell_path(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Shell path is empty".to_string());
    }

    open_path(path)
}

#[cfg(target_os = "windows")]
fn open_path(path: &str) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let path_wide = to_wide(path);
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

#[cfg(not(target_os = "windows"))]
fn open_path(path: &str) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to open shell path: {error}"))
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
