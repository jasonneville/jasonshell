//! Process image lookup shared by task-window identity capture. The elevated
//! "task window helper" (which could terminate processes) was removed: Close is
//! request-only and never elevates or terminates.

#[cfg(target_os = "windows")]
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::CloseHandle;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

#[cfg(target_os = "windows")]
pub(super) fn process_image_path(pid: u32) -> Result<PathBuf, String> {
    let process_handle = unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            .map_err(|_| "process path unavailable".to_string())?
    };
    struct HandleGuard(windows::Win32::Foundation::HANDLE);
    impl Drop for HandleGuard {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let _guard = HandleGuard(process_handle);
    let mut buffer = vec![0u16; 1024];
    let mut size = buffer.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            process_handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(PathBuf::from(String::from_utf16_lossy(
        &buffer[..size as usize],
    )))
}
