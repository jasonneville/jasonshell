use super::parse_hwnd;

#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStringExt;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{CloseHandle, FILETIME, HWND, LPARAM, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{GetProcessTimes, OpenProcess, QueryFullProcessImageNameW, TerminateProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{IsWindow, PostMessageW, WM_CLOSE};

const HELPER_ARG_PREFIX: &str = "--task-window-helper";
const PATH_PREFIX: &str = "utf16hex:";

#[cfg(target_os = "windows")]
pub(super) fn encode_canonical_path(path: &Path) -> String {
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    let mut encoded = String::with_capacity(PATH_PREFIX.len() + wide.len() * 4);
    encoded.push_str(PATH_PREFIX);
    for unit in wide {
        use std::fmt::Write as _;
        let _ = write!(&mut encoded, "{unit:04x}");
    }
    encoded
}

#[cfg(target_os = "windows")]
pub(super) fn decode_canonical_path(value: &str) -> Result<PathBuf, String> {
    let hex = value
        .strip_prefix(PATH_PREFIX)
        .ok_or_else(|| "Invalid task-window-helper path descriptor".to_string())?;
    if hex.len() % 4 != 0 {
        return Err("Invalid task-window-helper path descriptor".to_string());
    }
    let mut wide = Vec::with_capacity(hex.len() / 4);
    for chunk in hex.as_bytes().chunks_exact(4) {
        let chunk = std::str::from_utf8(chunk).map_err(|_| "Invalid task-window-helper path descriptor".to_string())?;
        wide.push(u16::from_str_radix(chunk, 16).map_err(|_| "Invalid task-window-helper path descriptor".to_string())?);
    }
    Ok(PathBuf::from(OsString::from_wide(&wide)))
}

#[cfg(target_os = "windows")]
pub(super) fn handle_task_window_helper_args() -> Result<bool, String> {
    let mut args = std::env::args().skip(1);
    let Some(flag) = args.next() else { return Ok(false); };
    if flag != HELPER_ARG_PREFIX {
        return Ok(false);
    }

    let hwnd = parse_hwnd(&args.next().ok_or_else(|| helper_usage_error("hwnd"))?)?;
    let pid = parse_pid(&args.next().ok_or_else(|| helper_usage_error("pid"))?)?;
    let creation_time = parse_u64(&args.next().ok_or_else(|| helper_usage_error("creation_time"))?)?;
    let canonical_image_path = decode_canonical_path(&args.next().ok_or_else(|| helper_usage_error("image_path"))?)?;

    let exit_code = handle_task_window_helper_action(hwnd, pid, creation_time, canonical_image_path);
    std::process::exit(exit_code);
}

#[cfg(not(target_os = "windows"))]
pub(super) fn handle_task_window_helper_args() -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_os = "windows")]
fn handle_task_window_helper_action(
    hwnd: HWND,
    pid: u32,
    creation_time: u64,
    canonical_image_path: PathBuf,
) -> i32 {
    if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return 3;
    }
    if process_id(hwnd).ok() != Some(pid)
        || process_creation_time(pid).ok() != Some(creation_time)
        || process_image_path(pid).ok() != Some(canonical_image_path.clone())
    {
        return 3;
    }

    if let Err(_error) = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) } {
        return 5;
    }
    if wait_for_window_close(hwnd) {
        return 0;
    }

    let process_handle = match unsafe { OpenProcess(PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
        Ok(handle) => handle,
        Err(_) => return 6,
    };
    if !revalidate_task_window_descriptor(hwnd, pid, creation_time, &canonical_image_path) {
        unsafe { let _ = CloseHandle(process_handle); }
        return 3;
    }
    let result = unsafe { TerminateProcess(process_handle, 1) };
    unsafe { let _ = CloseHandle(process_handle); }
    if result.is_ok() { 0 } else { 6 }
}

#[cfg(target_os = "windows")]
pub(super) fn revalidate_task_window_descriptor(
    hwnd: HWND,
    pid: u32,
    creation_time: u64,
    canonical_image_path: &Path,
) -> bool {
    if !unsafe { IsWindow(Some(hwnd)).as_bool() } {
        return false;
    }
    process_id(hwnd).ok() == Some(pid)
        && process_creation_time(pid).ok() == Some(creation_time)
        && process_image_path(pid).ok().as_deref() == Some(canonical_image_path)
}

#[cfg(target_os = "windows")]
fn wait_for_window_close(hwnd: HWND) -> bool { for _ in 0..10 { if !unsafe { IsWindow(Some(hwnd)).as_bool() } { return true; } std::thread::sleep(std::time::Duration::from_millis(50)); } false }

#[cfg(target_os = "windows")]
fn process_id(hwnd: HWND) -> Result<u32, String> { let mut pid = 0; unsafe { let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(hwnd, Some(&mut pid)); } if pid == 0 { Err("Helper target rejected: pid unavailable".to_string()) } else { Ok(pid) } }

#[cfg(target_os = "windows")]
pub(super) fn process_image_path(pid: u32) -> Result<PathBuf, String> { let process_handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).map_err(|_| "process path unavailable".to_string())? }; struct HandleGuard(windows::Win32::Foundation::HANDLE); impl Drop for HandleGuard { fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } } } let _guard = HandleGuard(process_handle); let mut buffer = vec![0u16; 1024]; let mut size = buffer.len() as u32; unsafe { QueryFullProcessImageNameW(process_handle, PROCESS_NAME_WIN32, windows::core::PWSTR(buffer.as_mut_ptr()), &mut size).map_err(|e| e.to_string())?; } Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..size as usize]))) }

#[cfg(target_os = "windows")]
fn process_creation_time(pid: u32) -> Result<u64, String> { let process_handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).map_err(|_| "process time unavailable".to_string())? }; struct HandleGuard(windows::Win32::Foundation::HANDLE); impl Drop for HandleGuard { fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } } } let _guard = HandleGuard(process_handle); let mut creation = FILETIME::default(); let mut exit = FILETIME::default(); let mut kernel = FILETIME::default(); let mut user = FILETIME::default(); unsafe { GetProcessTimes(process_handle, &mut creation, &mut exit, &mut kernel, &mut user).map_err(|e| e.to_string())?; } Ok(((creation.dwHighDateTime as u64) << 32) | creation.dwLowDateTime as u64) }

fn helper_usage_error(field: &str) -> String { format!("Invalid task-window-helper args: missing {field}") }
fn parse_pid(value: &str) -> Result<u32, String> { value.parse().map_err(|e| format!("Invalid pid: {e}")) }
fn parse_u64(value: &str) -> Result<u64, String> { value.parse().map_err(|e| format!("Invalid creation time: {e}")) }

#[cfg(target_os = "windows")]
pub(super) fn helper_exit_code_for_shell_execute_result(status: u32) -> Result<(), String> {
    match status {
        0 => Ok(()),
        code if is_uac_canceled_shell_execute_code(code) => Err("UAC canceled".to_string()),
        code => Err(format!("Task window helper failed with code {code}")),
    }
}

pub(super) fn is_access_denied_win32_code(status: u32) -> bool {
    if status == 5 {
        return true;
    }
    status == 0x8007_0005
}

pub(super) fn is_uac_canceled_shell_execute_code(status: u32) -> bool {
    if status == 1223 {
        return true;
    }
    let facility = (status >> 16) & 0x1fff;
    let code = status & 0xffff;
    facility == 7 && code == 1223
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_path_roundtrip_handles_spaces_and_unicode() {
        let paths = [
            PathBuf::from(r"C:\Program Files\FIFA Mod Manager\app.exe"),
            PathBuf::from(r"C:\测试\привет\app.exe"),
        ];
        for path in paths {
            let encoded = encode_canonical_path(&path);
            let decoded = decode_canonical_path(&encoded).expect("roundtrip");
            assert_eq!(decoded, path);
        }
    }

    #[test]
    fn helper_failure_codes_are_stable() {
        let codes = [0, 3, 5, 6, 1223];
        assert_eq!(codes, [0, 3, 5, 6, 1223]);
    }

    #[test]
    fn access_denied_escalation_predicate_is_only_access_denied() {
        fn should_escalate(code: u32) -> bool { is_access_denied_win32_code(code) }
        assert!(should_escalate(5));
        assert!(should_escalate(0x8007_0005));
        assert!(!should_escalate(0));
        assert!(!should_escalate(2));
        assert!(!should_escalate(0x8007_0002));
        assert!(!should_escalate(0x4007_0005));
        assert!(!should_escalate(0xC007_0005));
        assert!(!should_escalate(1223));
    }

    #[test]
    fn helper_exit_code_mapping_handles_uac_cancel_exactly() {
        assert!(helper_exit_code_for_shell_execute_result(0).is_ok());
        assert_eq!(helper_exit_code_for_shell_execute_result(1223).unwrap_err(), "UAC canceled");
        assert_eq!(helper_exit_code_for_shell_execute_result(0x8007_04C7).unwrap_err(), "UAC canceled");
        assert!(helper_exit_code_for_shell_execute_result(5).unwrap_err().contains("code 5"));
        assert_ne!(helper_exit_code_for_shell_execute_result(0x8007_0005).unwrap_err(), "UAC canceled");
    }

    #[test]
    fn helper_argv_decode_rejects_bad_descriptors() {
        assert!(decode_canonical_path("bad").is_err());
        assert!(decode_canonical_path("utf16hex:123").is_err());
        assert!(decode_canonical_path("utf16hex:zzzz").is_err());
    }

    #[test]
    fn descriptor_validation_requires_exact_hwnd_pid_time_and_path_match() {
        let captured = PathBuf::from(r"C:\good.exe");
        let current = PathBuf::from(r"C:\bad.exe");
        assert!(matches_descriptor_for_test(1, 2, 3, &captured, 1, 2, 3, &captured));
        assert!(!matches_descriptor_for_test(1, 2, 3, &captured, 1, 2, 3, &current));
        assert!(!matches_descriptor_for_test(1, 2, 3, &captured, 9, 2, 3, &captured));
    }

    fn matches_descriptor_for_test(
        hwnd: isize,
        pid: u32,
        creation_time: u64,
        canonical_image_path: &PathBuf,
        current_hwnd: isize,
        current_pid: u32,
        current_creation_time: u64,
        current_canonical_image_path: &PathBuf,
    ) -> bool {
        hwnd == current_hwnd
            && pid == current_pid
            && creation_time == current_creation_time
            && canonical_image_path == current_canonical_image_path
    }
}
