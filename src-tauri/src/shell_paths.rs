#[tauri::command]
pub fn open_shell_path(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Shell path is empty".to_string());
    }

    open_path(path)
}

#[tauri::command]
pub fn run_control_panel(args: Option<Vec<String>>) -> Result<(), String> {
    let args = args.unwrap_or_default();
    if !args.iter().all(|arg| is_safe_control_panel_arg(arg)) {
        return Err("Control Panel argument is invalid".to_string());
    }

    run_control_panel_command(&args)
}

pub fn open_shell_path_with_picker(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Shell path is empty".to_string());
    }

    open_with_picker(path)
}

fn is_safe_control_panel_arg(arg: &str) -> bool {
    !arg.is_empty()
        && arg
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '{' | '}' | ',' | '-'))
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
fn run_control_panel_command(args: &[String]) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let executable_wide = to_wide("control.exe");
    let parameters = args.join(" ");
    let parameters_wide = to_wide(&parameters);
    let parameter_ptr = if parameters.is_empty() {
        PCWSTR::null()
    } else {
        PCWSTR(parameters_wide.as_ptr())
    };
    // SAFETY: executable and parameter buffers are NUL-terminated UTF-16 and
    // live for the duration of the call. ShellExecuteW does not retain them.
    let result = unsafe {
        ShellExecuteW(
            Some(HWND::default()),
            None,
            PCWSTR(executable_wide.as_ptr()),
            parameter_ptr,
            None,
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as isize;
    if code <= 32 {
        return Err(format!("ShellExecuteW failed for control.exe with code {code}"));
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
fn run_control_panel_command(args: &[String]) -> Result<(), String> {
    let mut command = std::process::Command::new("control.exe");
    command.args(args);
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to open Control Panel: {error}"))
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

#[cfg(test)]
mod tests {
    use super::is_safe_control_panel_arg;

    #[test]
    fn control_panel_args_allow_applets_and_block_shell_metacharacters() {
        assert!(is_safe_control_panel_arg("Microsoft.Sound"));
        assert!(is_safe_control_panel_arg("{26EE0668-A00A-44D7-9371-BEB064C98683}"));
        assert!(!is_safe_control_panel_arg(""));
        assert!(!is_safe_control_panel_arg("&calc.exe"));
        assert!(!is_safe_control_panel_arg("Microsoft.Sound;calc.exe"));
    }
}
