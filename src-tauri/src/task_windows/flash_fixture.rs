use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{GetLastError, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FlashWindowEx,
    GetForegroundWindow, IsIconic, IsWindowVisible, PeekMessageW, RegisterClassW,
    SetForegroundWindow, ShowWindow, TranslateMessage, UnregisterClassW, CS_HREDRAW, CS_VREDRAW,
    CW_USEDEFAULT, FLASHWINFO, FLASHW_TIMERNOFG, FLASHW_TRAY, MSG, PM_REMOVE, SW_RESTORE,
    SW_SHOWMINNOACTIVE, SW_SHOWNOACTIVATE, WINDOW_EX_STYLE, WINDOW_STYLE, WM_DESTROY, WNDCLASSW,
    WS_OVERLAPPEDWINDOW,
};

static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy)]
pub struct FlashFixtureArgs {
    pub minimized: bool,
    pub flash_count: u32,
    pub interval_ms: u64,
    pub timeout_ms: u64,
}

pub fn handle_taskbar_flash_fixture_args() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().collect();
    if !args.iter().any(|a| a == "--taskbar-flash-fixture") {
        return Ok(false);
    }
    let parsed = parse_args(&args)?;
    run_fixture(parsed)?;
    std::process::exit(0);
}

pub(crate) fn parse_args(args: &[String]) -> Result<FlashFixtureArgs, String> {
    let mut minimized = false;
    let mut flash_count = None;
    let mut interval_ms = None;
    let mut timeout_ms = None;
    let mut saw_marker = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--taskbar-flash-fixture" => {
                if saw_marker {
                    return Err("duplicate --taskbar-flash-fixture".to_string());
                }
                saw_marker = true;
            }
            "--minimized" => {
                if minimized {
                    return Err("duplicate --minimized".to_string());
                }
                minimized = true;
            }
            "--flash-count" | "--interval-ms" | "--timeout-ms" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| format!("missing {}", args[i]))?;
                let parsed: u64 = value.parse().map_err(|_| format!("invalid {}", args[i]))?;
                match args[i].as_str() {
                    "--flash-count" if flash_count.is_none() && (1..=20).contains(&parsed) => {
                        flash_count = Some(parsed as u32)
                    }
                    "--interval-ms" if interval_ms.is_none() && (1..=10_000).contains(&parsed) => {
                        interval_ms = Some(parsed)
                    }
                    "--timeout-ms" if timeout_ms.is_none() && (1..=30_000).contains(&parsed) => {
                        timeout_ms = Some(parsed)
                    }
                    _ => return Err(format!("out of range {}", args[i])),
                }
                i += 1;
            }
            other if other.starts_with("--") => return Err(format!("unknown arg {other}")),
            _ => return Err("unexpected positional arg".to_string()),
        }
        i += 1;
    }

    if !saw_marker {
        return Err("missing --taskbar-flash-fixture".to_string());
    }

    Ok(FlashFixtureArgs {
        minimized,
        flash_count: flash_count.ok_or_else(|| "missing --flash-count".to_string())?,
        interval_ms: interval_ms.ok_or_else(|| "missing --interval-ms".to_string())?,
        timeout_ms: timeout_ms.ok_or_else(|| "missing --timeout-ms".to_string())?,
    })
}

fn run_fixture(args: FlashFixtureArgs) -> Result<(), String> {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Err("fixture already running".to_string());
    }
    let _running_lease = RunningLease;
    let result = (|| -> Result<(), String> {
        let mut guard = FixtureGuard::new(args.minimized)?;
        emit_event("ready", guard.hwnd, args.minimized, None)?;
        if current_foreground() == guard.hwnd {
            return Err("fixture already foreground".to_string());
        }

        let deadline = std::time::Instant::now() + Duration::from_millis(args.timeout_ms);
        for _ in 0..args.flash_count {
            pump_messages();
            let mut info = FLASHWINFO {
                cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
                hwnd: guard.hwnd,
                dwFlags: FLASHW_TRAY | FLASHW_TIMERNOFG,
                uCount: 1,
                dwTimeout: 0,
            };
            let was_active_caption = unsafe { FlashWindowEx(&mut info) }.as_bool();
            emit_event(
                "request",
                guard.hwnd,
                args.minimized,
                Some(if was_active_caption { "true" } else { "false" }),
            )?;
            sleep_until(deadline, args.interval_ms)?;
        }

        unsafe {
            let _ = ShowWindow(
                guard.hwnd,
                if args.minimized {
                    SW_SHOWMINNOACTIVE
                } else {
                    SW_SHOWNOACTIVATE
                },
            );
        }
        wait_for_window_state(guard.hwnd, true, args.minimized, deadline)?;
        unsafe {
            let _ = ShowWindow(guard.hwnd, SW_RESTORE);
        };
        wait_for_window_state(guard.hwnd, true, false, deadline)?;
        if !unsafe { SetForegroundWindow(guard.hwnd) }.as_bool() {
            return Err(win_err("SetForegroundWindow"));
        }
        wait_for_foreground(guard.hwnd, deadline)?;
        emit_event("focus", guard.hwnd, args.minimized, None)?;
        guard.restore_prior_foreground();
        guard.mark_success();
        Ok(())
    })();
    result
}

struct RunningLease;

impl Drop for RunningLease {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::SeqCst);
    }
}

struct FixtureGuard {
    hwnd: HWND,
    class_name: Vec<u16>,
    prior_foreground: HWND,
    class_registered: bool,
    cleanup_status: &'static str,
}

impl FixtureGuard {
    fn new(minimized: bool) -> Result<Self, String> {
        let class_name = wide(&format!(
            "TaskbarFlashFixtureWindowClass-{}",
            std::process::id()
        ));
        let hmodule: HMODULE =
            unsafe { GetModuleHandleW(PCWSTR::null()) }.map_err(|_| win_err("GetModuleHandleW"))?;
        let wc = WNDCLASSW {
            hCursor: Default::default(),
            hInstance: HINSTANCE(hmodule.0),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            lpfnWndProc: Some(window_proc),
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };
        let atom = unsafe { RegisterClassW(&wc) };
        if atom == 0 {
            return Err(win_err_with_last_error("RegisterClassW"));
        }
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR(class_name.as_ptr()),
                WINDOW_STYLE(WS_OVERLAPPEDWINDOW.0),
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                320,
                240,
                None,
                None,
                Some(HINSTANCE(hmodule.0)),
                None,
            )
        };
        let hwnd = hwnd.map_err(|_| win_err("CreateWindowExW"))?;
        unsafe {
            let _ = ShowWindow(
                hwnd,
                if minimized {
                    SW_SHOWMINNOACTIVE
                } else {
                    SW_SHOWNOACTIVATE
                },
            );
        }
        wait_for_window_state(
            hwnd,
            true,
            minimized,
            std::time::Instant::now() + Duration::from_millis(500),
        )?;
        Ok(Self {
            hwnd,
            class_name,
            prior_foreground: current_foreground(),
            class_registered: true,
            cleanup_status: "error",
        })
    }

    fn restore_prior_foreground(&self) {
        if self.prior_foreground.0 != std::ptr::null_mut() {
            let _ = unsafe { SetForegroundWindow(self.prior_foreground) };
        }
    }

    fn mark_success(&mut self) {
        self.cleanup_status = "success";
    }
}

impl Drop for FixtureGuard {
    fn drop(&mut self) {
        let visible = unsafe { IsWindowVisible(self.hwnd) }.as_bool();
        let iconic = unsafe { IsIconic(self.hwnd) }.as_bool();
        let _ = unsafe { DestroyWindow(self.hwnd) };
        if let Ok(hmodule) = unsafe { GetModuleHandleW(PCWSTR::null()) } {
            if self.class_registered {
                let _ = unsafe {
                    UnregisterClassW(PCWSTR(self.class_name.as_ptr()), Some(HINSTANCE(hmodule.0)))
                };
            }
        }
        let _ = emit_cleanup(self.cleanup_status, self.hwnd, visible, iconic);
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn emit_event(
    event: &str,
    hwnd: HWND,
    minimized: bool,
    status: Option<&str>,
) -> Result<(), String> {
    let mut out = io::stdout().lock();
    if let Some(status) = status {
        writeln!(out, "{{\"event\":\"{}\",\"timestamp_ms\":{},\"hwnd\":{},\"pid\":{},\"minimized\":{},\"status\":\"{}\"}}", event, now_ms(), hwnd.0 as usize, std::process::id(), minimized, status).map_err(|e| e.to_string())?;
    } else {
        writeln!(
            out,
            "{{\"event\":\"{}\",\"timestamp_ms\":{},\"hwnd\":{},\"pid\":{},\"minimized\":{}}}",
            event,
            now_ms(),
            hwnd.0 as usize,
            std::process::id(),
            minimized
        )
        .map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

fn emit_cleanup(status: &str, hwnd: HWND, visible: bool, iconic: bool) -> Result<(), String> {
    let mut out = io::stdout().lock();
    writeln!(out, "{{\"event\":\"cleanup\",\"timestamp_ms\":{},\"hwnd\":{},\"pid\":{},\"visible\":{},\"iconic\":{},\"status\":\"{}\"}}", now_ms(), hwnd.0 as usize, std::process::id(), visible, iconic, status).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}

fn current_foreground() -> HWND {
    unsafe { GetForegroundWindow() }
}

fn pump_messages() {
    let mut msg = MSG::default();
    while unsafe { PeekMessageW(&mut msg, Some(HWND(std::ptr::null_mut())), 0, 0, PM_REMOVE) }
        .as_bool()
    {
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn wait_for_foreground(hwnd: HWND, deadline: std::time::Instant) -> Result<(), String> {
    while std::time::Instant::now() <= deadline {
        pump_messages();
        if current_foreground() == hwnd {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("foreground wait timeout".to_string())
}

fn wait_for_window_state(
    hwnd: HWND,
    visible: bool,
    iconic: bool,
    deadline: std::time::Instant,
) -> Result<(), String> {
    while std::time::Instant::now() <= deadline {
        pump_messages();
        let cur_visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
        let cur_iconic = unsafe { IsIconic(hwnd) }.as_bool();
        if cur_visible == visible && cur_iconic == iconic {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err(format!(
        "window state wait timeout: visible={} iconic={}",
        visible, iconic
    ))
}

fn sleep_until(deadline: std::time::Instant, interval_ms: u64) -> Result<(), String> {
    let target = std::cmp::min(
        deadline,
        std::time::Instant::now() + Duration::from_millis(interval_ms),
    );
    while std::time::Instant::now() < target {
        pump_messages();
        std::thread::sleep(Duration::from_millis(5));
    }
    if std::time::Instant::now() > deadline {
        return Err("scenario deadline exceeded".to_string());
    }
    Ok(())
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn win_err(api: &str) -> String {
    format!("{} failed", api)
}

fn win_err_with_last_error(api: &str) -> String {
    let err = unsafe { GetLastError() };
    format!("{} failed: {}", api, err.0)
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fixture_args_with_bounds() {
        let args = vec![
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--minimized".into(),
            "--flash-count".into(),
            "3".into(),
            "--interval-ms".into(),
            "25".into(),
            "--timeout-ms".into(),
            "250".into(),
        ];
        let parsed = parse_args(&args).unwrap();
        assert!(parsed.minimized);
        assert_eq!(parsed.flash_count, 3);
        assert_eq!(parsed.interval_ms, 25);
        assert_eq!(parsed.timeout_ms, 250);
    }

    #[test]
    fn rejects_duplicate_flags_marker_required_boundaries_and_bad_args() {
        let base = [
            "app",
            "--taskbar-flash-fixture",
            "--flash-count",
            "1",
            "--interval-ms",
            "1",
            "--timeout-ms",
            "1",
        ];
        assert_eq!(
            parse_args(&base.iter().map(|s| s.to_string()).collect::<Vec<_>>())
                .unwrap()
                .flash_count,
            1
        );
        assert!(parse_args(&["app".into()]).is_err());
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--taskbar-flash-fixture".into(),
            "--flash-count".into(),
            "1".into(),
            "--interval-ms".into(),
            "1".into(),
            "--timeout-ms".into(),
            "1".into()
        ])
        .is_err());
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--minimized".into(),
            "--minimized".into(),
            "--flash-count".into(),
            "1".into(),
            "--interval-ms".into(),
            "1".into(),
            "--timeout-ms".into(),
            "1".into()
        ])
        .is_err());
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--flash-count".into(),
            "1".into(),
            "--flash-count".into(),
            "2".into(),
            "--interval-ms".into(),
            "1".into(),
            "--timeout-ms".into(),
            "1".into()
        ])
        .is_err());
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--interval-ms".into(),
            "1".into(),
            "--interval-ms".into(),
            "2".into(),
            "--flash-count".into(),
            "1".into(),
            "--timeout-ms".into(),
            "1".into()
        ])
        .is_err());
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--timeout-ms".into(),
            "1".into(),
            "--timeout-ms".into(),
            "2".into(),
            "--flash-count".into(),
            "1".into(),
            "--interval-ms".into(),
            "1".into()
        ])
        .is_err());
        for value in ["0", "21"] {
            assert!(parse_args(&[
                "app".into(),
                "--taskbar-flash-fixture".into(),
                "--flash-count".into(),
                value.into(),
                "--interval-ms".into(),
                "1".into(),
                "--timeout-ms".into(),
                "1".into()
            ])
            .is_err());
        }
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--flash-count".into(),
            "20".into(),
            "--interval-ms".into(),
            "1".into(),
            "--timeout-ms".into(),
            "1".into()
        ])
        .is_ok());
        for value in ["0", "10001"] {
            assert!(parse_args(&[
                "app".into(),
                "--taskbar-flash-fixture".into(),
                "--flash-count".into(),
                "1".into(),
                "--interval-ms".into(),
                value.into(),
                "--timeout-ms".into(),
                "1".into()
            ])
            .is_err());
        }
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--flash-count".into(),
            "1".into(),
            "--interval-ms".into(),
            "10000".into(),
            "--timeout-ms".into(),
            "1".into()
        ])
        .is_ok());
        for value in ["0", "30001"] {
            assert!(parse_args(&[
                "app".into(),
                "--taskbar-flash-fixture".into(),
                "--flash-count".into(),
                "1".into(),
                "--interval-ms".into(),
                "1".into(),
                "--timeout-ms".into(),
                value.into()
            ])
            .is_err());
        }
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--flash-count".into(),
            "1".into(),
            "--interval-ms".into(),
            "1".into(),
            "--timeout-ms".into(),
            "30000".into()
        ])
        .is_ok());
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--flash-count".into(),
            "x".into(),
            "--interval-ms".into(),
            "1".into(),
            "--timeout-ms".into(),
            "1".into()
        ])
        .is_err());
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--flash-count".into(),
            "1".into(),
            "--interval-ms".into(),
            "x".into(),
            "--timeout-ms".into(),
            "1".into()
        ])
        .is_err());
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--flash-count".into(),
            "1".into(),
            "--interval-ms".into(),
            "1".into(),
            "--timeout-ms".into(),
            "x".into()
        ])
        .is_err());
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "--unknown".into(),
            "1".into(),
            "--flash-count".into(),
            "1".into(),
            "--interval-ms".into(),
            "1".into(),
            "--timeout-ms".into(),
            "1".into()
        ])
        .is_err());
        assert!(parse_args(&[
            "app".into(),
            "--taskbar-flash-fixture".into(),
            "positional".into(),
            "--flash-count".into(),
            "1".into(),
            "--interval-ms".into(),
            "1".into(),
            "--timeout-ms".into(),
            "1".into()
        ])
        .is_err());
    }
}
