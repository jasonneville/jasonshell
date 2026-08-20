use std::path::Path;

#[tauri::command]
pub fn open_shell_path(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Shell path is empty".to_string());
    }

    let target = classify_shell_open_target(path)?;
    open_path(target.as_shell_value())
}

#[tauri::command]
pub fn launch_app_path(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Application path is empty".to_string());
    }

    let target = classify_app_launch_target(path)?;
    open_path(target)
}

#[derive(Debug)]
enum ShellOpenTarget<'a> {
    LocalPath(&'a str),
    WindowsSettings(&'a str),
}

impl<'a> ShellOpenTarget<'a> {
    fn as_shell_value(&self) -> &'a str {
        match self {
            ShellOpenTarget::LocalPath(path) | ShellOpenTarget::WindowsSettings(path) => path,
        }
    }
}

fn classify_shell_open_target(path: &str) -> Result<ShellOpenTarget<'_>, String> {
    let lower = path.to_ascii_lowercase();
    if lower.starts_with("ms-settings:") {
        return validate_ms_settings_uri(path).map(|_| ShellOpenTarget::WindowsSettings(path));
    }
    if lower.contains("://") || lower.starts_with("file:") || looks_like_protocol(path) {
        return Err("Shell path protocol is not allowed".to_string());
    }

    let local_path = Path::new(path);
    if !local_path.exists() {
        return Err(format!("Shell path does not exist: {path}"));
    }
    if is_executable_shell_path(local_path) {
        return Err("Shell path executable or script launch is not allowed".to_string());
    }
    Ok(ShellOpenTarget::LocalPath(path))
}

fn classify_app_launch_target(path: &str) -> Result<&str, String> {
    if path.contains("://")
        || path.to_ascii_lowercase().starts_with("file:")
        || looks_like_protocol(path)
    {
        return Err("Application path protocol is not allowed".to_string());
    }

    let local_path = Path::new(path);
    if !local_path.exists() {
        return Err(format!("Application path does not exist: {path}"));
    }
    if !is_audited_app_launch_path(local_path) {
        return Err("Application path extension is not allowed".to_string());
    }
    Ok(path)
}

fn validate_ms_settings_uri(path: &str) -> Result<(), String> {
    let Some((scheme, suffix)) = path.split_once(':') else {
        return Err("Windows Settings URI is invalid".to_string());
    };
    if !scheme.eq_ignore_ascii_case("ms-settings") {
        return Err("Windows Settings URI is invalid".to_string());
    }
    if suffix
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Ok(());
    }
    Err("Windows Settings URI is invalid".to_string())
}

fn looks_like_protocol(path: &str) -> bool {
    let Some((scheme, _)) = path.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && scheme.chars().all(|ch| {
            ch.is_ascii_alphabetic() || ch.is_ascii_digit() || matches!(ch, '+' | '-' | '.')
        })
        && !is_windows_drive_path(path)
}

fn is_windows_drive_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
        && bytes[0].is_ascii_alphabetic()
}

fn is_executable_shell_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "exe"
                    | "com"
                    | "bat"
                    | "cmd"
                    | "ps1"
                    | "psm1"
                    | "vbs"
                    | "js"
                    | "jse"
                    | "wsf"
                    | "msi"
                    | "msc"
                    | "scr"
                    | "lnk"
                    | "url"
                    | "cpl"
                    | "reg"
            )
        })
}

pub(crate) fn is_audited_app_launch_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "exe" | "lnk" | "appref-ms"
            )
        })
}

pub(crate) fn classify_stack_item_open_route(path: &Path) -> StackItemOpenRoute {
    if is_audited_app_launch_path(path) {
        StackItemOpenRoute::AuditedApp
    } else {
        StackItemOpenRoute::ShellOpen
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StackItemOpenRoute {
    AuditedApp,
    ShellOpen,
}

#[tauri::command]
pub fn run_control_panel(args: Option<Vec<String>>) -> Result<(), String> {
    let args = args.unwrap_or_default();
    if !args.iter().all(|arg| is_safe_control_panel_arg(arg)) {
        return Err("Control Panel argument is invalid".to_string());
    }

    run_control_panel_command(&args)
}

pub fn open_folder_in_vscode(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Folder path is empty".to_string());
    }
    let folder = Path::new(path);
    if !folder.is_dir() {
        return Err(format!("Folder does not exist: {path}"));
    }

    let Some(vscode_executable) = resolve_vscode_executable() else {
        return Err(
            "Visual Studio Code was not found in standard install paths or PATH".to_string(),
        );
    };

    std::process::Command::new(&vscode_executable)
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to open folder in VS Code: {error}"))
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

const VSCODE_EXECUTABLE_CANDIDATES: &[&str] = &[
    r"%LocalAppData%\Programs\Microsoft VS Code\Code.exe",
    r"%ProgramFiles%\Microsoft VS Code\Code.exe",
    r"C:\Program Files\Microsoft VS Code\Code.exe",
    r"C:\Program Files (x86)\Microsoft VS Code\Code.exe",
];

pub(crate) fn resolve_vscode_executable() -> Option<std::path::PathBuf> {
    resolve_vscode_executable_with(resolve_executable_candidate)
}

fn resolve_vscode_executable_with<F>(resolver: F) -> Option<std::path::PathBuf>
where
    F: Fn(&str) -> Option<std::path::PathBuf>,
{
    VSCODE_EXECUTABLE_CANDIDATES
        .iter()
        .find_map(|candidate| resolver(candidate))
}

fn resolve_executable_candidate(candidate: &str) -> Option<std::path::PathBuf> {
    let expanded = expand_environment(candidate);
    let path = std::path::PathBuf::from(&expanded);
    if !path.is_absolute() {
        return None;
    }
    path.exists()
        .then(|| std::fs::canonicalize(&path).ok())
        .flatten()
}

fn expand_environment(candidate: &str) -> String {
    let mut expanded = candidate.to_string();
    for (name, value) in [
        ("ProgramFiles", std::env::var_os("ProgramFiles")),
        ("ProgramFiles(x86)", std::env::var_os("ProgramFiles(x86)")),
        ("LocalAppData", std::env::var_os("LocalAppData")),
    ] {
        if let Some(value) = value {
            expanded = expanded.replace(&format!("%{name}%"), value.to_string_lossy().as_ref());
        }
    }
    expanded
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
        return Err(format!(
            "ShellExecuteW failed for control.exe with code {code}"
        ));
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
    use super::{
        classify_app_launch_target, classify_shell_open_target, classify_stack_item_open_route,
        is_safe_control_panel_arg, resolve_vscode_executable_with, ShellOpenTarget,
        StackItemOpenRoute,
    };
    use std::path::PathBuf;

    #[test]
    fn control_panel_args_allow_applets_and_block_shell_metacharacters() {
        assert!(is_safe_control_panel_arg("Microsoft.Sound"));
        assert!(is_safe_control_panel_arg(
            "{26EE0668-A00A-44D7-9371-BEB064C98683}"
        ));
        assert!(!is_safe_control_panel_arg(""));
        assert!(!is_safe_control_panel_arg("&calc.exe"));
        assert!(!is_safe_control_panel_arg("Microsoft.Sound;calc.exe"));
    }

    #[test]
    fn vscode_resolver_uses_standard_candidate_order() {
        let resolved = resolve_vscode_executable_with(|candidate| match candidate {
            "code.cmd" => Some(PathBuf::from(r"C:\Tools\code.cmd")),
            _ => None,
        });

        assert_eq!(resolved, Some(PathBuf::from(r"C:\Tools\code.cmd")));
    }

    #[test]
    fn vscode_resolver_returns_none_when_missing() {
        assert_eq!(resolve_vscode_executable_with(|_| None), None);
    }

    #[test]
    fn shell_open_boundary_allows_existing_local_files_and_folders() {
        let root =
            std::env::temp_dir().join(format!("jasonshell-shell-open-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("notes.txt");
        std::fs::write(&file, b"hello").unwrap();

        assert!(matches!(
            classify_shell_open_target(&root.to_string_lossy()).unwrap(),
            ShellOpenTarget::LocalPath(_)
        ));
        assert!(matches!(
            classify_shell_open_target(&file.to_string_lossy()).unwrap(),
            ShellOpenTarget::LocalPath(_)
        ));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn shell_open_boundary_rejects_execution_and_protocol_inputs() {
        let root = std::env::temp_dir().join(format!(
            "jasonshell-shell-open-reject-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        for name in [
            "cmd.exe",
            "script.ps1",
            "batch.bat",
            "batch.cmd",
            "shortcut.lnk",
            "website.url",
        ] {
            let path = root.join(name);
            std::fs::write(&path, b"echo bad").unwrap();
            assert!(classify_shell_open_target(&path.to_string_lossy())
                .unwrap_err()
                .contains("not allowed"));
        }
        for path in [
            "http://example.com",
            "file:///C:/Temp/a.txt",
            "unknown-protocol:value",
        ] {
            assert!(classify_shell_open_target(path)
                .unwrap_err()
                .contains("protocol is not allowed"));
        }
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn shell_open_boundary_allows_ms_settings_only_as_vetted_protocol() {
        assert!(matches!(
            classify_shell_open_target("ms-settings:display").unwrap(),
            ShellOpenTarget::WindowsSettings(_)
        ));
        assert!(classify_shell_open_target("ms-settings:display&calc.exe").is_err());
        assert!(classify_shell_open_target("Ms-Settings:display&calc.exe").is_err());
    }

    #[test]
    fn audited_app_launch_boundary_accepts_executables_for_stack_browser_activation() {
        let root = std::env::temp_dir().join(format!(
            "jasonshell-app-launch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let app = root.join("tool.exe");
        std::fs::write(&app, b"app").unwrap();

        assert_eq!(
            classify_stack_item_open_route(&app),
            StackItemOpenRoute::AuditedApp
        );

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn audited_app_launch_boundary_allows_apps_without_weakening_generic_shell_open() {
        let root = std::env::temp_dir().join(format!(
            "jasonshell-app-launch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let app = root.join("code.exe");
        let shortcut = root.join("spotify.lnk");
        let text = root.join("notes.txt");
        std::fs::write(&app, b"app").unwrap();
        std::fs::write(&shortcut, b"shortcut").unwrap();
        std::fs::write(&text, b"text").unwrap();

        assert_eq!(
            classify_stack_item_open_route(&app),
            StackItemOpenRoute::AuditedApp
        );
        assert_eq!(
            classify_stack_item_open_route(&shortcut),
            StackItemOpenRoute::AuditedApp
        );
        assert_eq!(
            classify_stack_item_open_route(&root),
            StackItemOpenRoute::ShellOpen
        );
        assert_eq!(
            classify_stack_item_open_route(&text),
            StackItemOpenRoute::ShellOpen
        );
        assert!(classify_app_launch_target(&text.to_string_lossy()).is_err());
        assert!(classify_app_launch_target("http://example.com/app.exe").is_err());

        std::fs::remove_dir_all(root).ok();
    }
}
