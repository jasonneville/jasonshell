//! A one-way native exit request. Renderer liveness is not part of shutdown.

use std::{
    sync::{mpsc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Manager, State, WebviewWindow};

#[cfg(all(test, windows))]
#[path = "../tests/fixtures/graceful_exit_owned_e2e.rs"]
mod owned_e2e;

const DEADLINE: Duration = Duration::from_secs(3);
const UNAVAILABLE: &str = "shutdown authority unavailable";

#[derive(Default)]
pub(crate) struct ExitCoordinator {
    deadline: Mutex<Option<Instant>>,
}

impl ExitCoordinator {
    #[cfg(all(test, windows))]
    fn request(
        &self,
        restore: impl FnOnce() + Send + 'static,
        cleanup: impl FnOnce(Instant) + Send + 'static,
        hard_exit: impl FnOnce() + Send + 'static,
    ) -> Result<(), String> {
        self.request_checked(|| Ok(()), restore, cleanup, hard_exit)
    }

    fn request_checked(
        &self,
        revalidate: impl FnOnce() -> Result<(), String>,
        restore: impl FnOnce() + Send + 'static,
        cleanup: impl FnOnce(Instant) + Send + 'static,
        hard_exit: impl FnOnce() + Send + 'static,
    ) -> Result<(), String> {
        let mut accepted = self.deadline.lock().map_err(|_| UNAVAILABLE)?;
        if accepted.is_some() {
            return Ok(());
        }

        // Provision independent threads before accepting. Until the commit below,
        // a setup failure only drops senders; no effects are authorized.
        let watchdog = spawn_waiting("shell-exit-watchdog", move |deadline| {
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                thread::sleep(remaining);
            }
            hard_exit();
        })?;
        let restore = spawn_waiting("shell-exit-restore", move |_| restore())?;
        let cleanup = spawn_waiting("shell-exit-cleanup", cleanup)?;

        // The caller may have lost focus or been replaced during setup. A
        // rejection drops the senders without arming any of the waiting lanes.
        revalidate()?;
        let deadline = Instant::now() + DEADLINE;
        // The same lock gates tracking and resuming Quick Command roots. No
        // renderer acknowledgement or later cleanup result can reopen it.
        *accepted = Some(deadline);
        let _ = watchdog.send(deadline);
        let _ = restore.send(deadline);
        let _ = cleanup.send(deadline);
        Ok(())
    }

    fn while_running<T>(&self, admit: impl FnOnce() -> T) -> Result<T, String> {
        let accepted = self.deadline.lock().map_err(|_| UNAVAILABLE)?;
        if accepted.is_some() {
            return Err("JasonShell is exiting".into());
        }
        Ok(admit())
    }
}

fn spawn_waiting(
    name: &str,
    run: impl FnOnce(Instant) + Send + 'static,
) -> Result<mpsc::Sender<Instant>, String> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            if let Ok(deadline) = rx.recv() {
                run(deadline);
            }
        })
        .map_err(|_| UNAVAILABLE)?;
    Ok(tx)
}

/// The old draft fence is gone. Already-admitted native actions may finish;
/// this only rejects new actions after the one-way request is accepted.
pub struct Admission;

pub fn admit_mutation(app: &AppHandle) -> Result<Admission, String> {
    while_running(app, || Admission)
}

pub(crate) fn while_running<T>(app: &AppHandle, admit: impl FnOnce() -> T) -> Result<T, String> {
    app.try_state::<ExitCoordinator>()
        .ok_or(UNAVAILABLE)?
        .while_running(admit)
}

pub(crate) fn is_exit_accepted(app: &AppHandle) -> bool {
    app.try_state::<ExitCoordinator>()
        .is_some_and(|coordinator| {
            coordinator
                .deadline
                .lock()
                .is_ok_and(|deadline| deadline.is_some())
        })
}

// Still used locally by best-effort resource owners. These receipts never
// gate shell lifetime or reach the renderer.
pub(crate) struct DrainReceipt {
    pub finished: bool,
}

#[cfg(windows)]
fn cleanup_native_resources(app: AppHandle, deadline: Instant) {
    // Quick Commands has its own process-handle oracle and runs independently
    // of the other owners (and of the AppBar lane).
    for cleanup in [
        |_: &AppHandle| {
            let _ = crate::windows_key_hook::stop_and_drain_hotkeys();
        },
        |_: &AppHandle| {
            let _ = crate::task_windows::stop_and_drain_task_tracking();
        },
        |app: &AppHandle| {
            let _ = crate::speech_runtime::stop_and_drain_speech(app);
        },
        |app: &AppHandle| {
            let _ = app
                .state::<std::sync::Arc<crate::snipping::runtime::SnipRuntime>>()
                .stop_and_drain();
        },
        |app: &AppHandle| {
            let _ = crate::stack_popup::folder_watch::stop_and_drain(app);
        },
        |app: &AppHandle| {
            let _ = app
                .state::<crate::process_manager::SystemInfoRuntime>()
                .stop_and_drain();
        },
        |app: &AppHandle| stop_terminal(app, crate::shell_windows::TERMINAL_PANEL_LABEL),
        |app: &AppHandle| stop_terminal(app, crate::shell_windows::STACK_POPUP_LABEL),
    ] as [fn(&AppHandle); 8]
    {
        let app = app.clone();
        let _ = thread::Builder::new().spawn(move || cleanup(&app));
    }
    let _ = crate::quick_commands::drain_owned_quick_commands(deadline);
}

#[cfg(windows)]
fn stop_terminal(app: &AppHandle, label: &str) {
    let state = app.state::<Mutex<crate::stack_popup::StackPopupRuntimeState>>();
    let _ = crate::stack_popup::terminal::stop_terminal_sessions_for_target(app, &state, label);
}

#[cfg(windows)]
fn terminate_current_process() {
    use windows::Win32::System::Threading::{GetCurrentProcess, TerminateProcess};
    // TerminateProcess on this process bypasses UI callbacks and DLL detach,
    // which can deadlock if a stalled thread owns a lock needed by teardown.
    // SAFETY: the pseudo-handle identifies only this process and needs no close.
    let _ = unsafe { TerminateProcess(GetCurrentProcess(), 0) };
    std::process::abort(); // self-termination does not return on success
}

#[tauri::command]
pub(crate) fn request_shell_exit(
    window: WebviewWindow,
    coordinator: State<'_, ExitCoordinator>,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        let app = window.app_handle();
        use crate::windows_key_hook::authorize_settings_instance;
        let initial_caller = authorize_settings_instance(&window, app)?;
        let restore_app = app.clone();
        let cleanup_app = app.clone();
        coordinator.request_checked(
            || {
                if authorize_settings_instance(&window, app)? != initial_caller {
                    return Err("Unauthorized Settings instance".into());
                }
                Ok(())
            },
            move || crate::appbar::restore_on_shell_exit(&restore_app),
            move |deadline| cleanup_native_resources(cleanup_app, deadline),
            terminate_current_process,
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (window, coordinator);
        Err("shell exit is only supported on Windows".into())
    }
}
