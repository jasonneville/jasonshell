mod launchers;
mod layout;
mod process_manager;
mod search_panel;
mod search_sources;
mod shell_paths;
mod shell_windows;
mod stack_popup;
mod task_preview;
mod task_windows;
mod taskbar_menu;

#[cfg(target_os = "windows")]
mod appbar;
#[cfg(target_os = "windows")]
mod explorer;

use std::sync::Mutex;
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

#[cfg(target_os = "windows")]
use appbar::ShellRuntimeState;

fn main() {
    let builder = tauri::Builder::default()
        .manage(shell_runtime_state())
        .manage(task_preview_state())
        .manage(search_panel_state())
        .manage(stack_popup_state())
        .manage(search_sources::search_index_state())
        .invoke_handler(tauri::generate_handler![
            launchers::list_pinned_taskbar_apps,
            launchers::launch_pinned_taskbar_app,
            task_windows::list_open_task_windows,
            task_windows::activate_task_window,
            task_windows::maximize_task_window,
            task_preview::show_task_window_preview,
            task_preview::hide_task_window_preview,
            taskbar_menu::show_task_window_context_menu,
            taskbar_menu::show_launcher_context_menu,
            taskbar_menu::show_top_bar_pin_context_menu,
            search_panel::show_search_panel,
            search_panel::hide_search_panel,
            search_panel::publish_search_panel,
            search_panel::get_search_panel_payload,
            process_manager::show_process_manager,
            process_manager::hide_process_manager,
            process_manager::list_processes,
            process_manager::kill_process,
            search_sources::search_system,
            shell_paths::open_shell_path,
            stack_popup::list_pinned_stack_folders,
            stack_popup::pin_stack_folder,
            stack_popup::unpin_stack_folder,
            stack_popup::reorder_pinned_stack_folders,
            stack_popup::show_stack_popup,
            stack_popup::hide_stack_popup,
            stack_popup::get_stack_popup_request,
            stack_popup::read_stack_folder,
            stack_popup::open_stack_item,
            stack_popup::open_stack_item_with_picker,
            stack_popup::rename_stack_item,
            stack_popup::copy_stack_items,
            stack_popup::cut_stack_items,
            stack_popup::paste_stack_items,
            stack_popup::delete_stack_item,
            stack_popup::new_stack_folder,
            stack_popup::reveal_stack_item,
            report_shell_surface_runtime_metrics
        ])
        .on_menu_event(|app_handle, event| {
            taskbar_menu::handle_taskbar_menu_event(app_handle, event);
        })
        .on_window_event(|window, event| {
            if window.label() == shell_windows::STACK_POPUP_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                let _ = window.hide();
                return;
            }

            if window.label() == shell_windows::PROCESS_MANAGER_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                let _ = window.emit(process_manager::PROCESS_MANAGER_CLOSED_EVENT, ());
                let _ = window.hide();
                return;
            }

            if matches!(
                window.label(),
                shell_windows::TOP_BAR_LABEL | shell_windows::BOTTOM_BAR_LABEL
            ) && matches!(
                event,
                WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed
            ) {
                #[cfg(target_os = "windows")]
                if let Err(error) = appbar::cleanup_shell_surfaces(&window.app_handle()) {
                    eprintln!("cleanup failed during window close: {error}");
                }

                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            let windows = shell_windows::create_shell_windows(app)?;
            search_sources::warm_search_index(app.handle().clone());

            #[cfg(target_os = "windows")]
            {
                appbar::activate_shell_surfaces(app, &windows)?;
            }

            #[cfg(not(target_os = "windows"))]
            {
                windows.top.show()?;
                windows.bottom.show()?;
            }

            Ok(())
        });

    let app = builder
        .build(tauri::generate_context!())
        .expect("failed to build JasonShell prototype");

    app.run(|app_handle, event| {
        #[cfg(target_os = "windows")]
        if matches!(event, RunEvent::Exit | RunEvent::ExitRequested { .. }) {
            if let Err(error) = appbar::cleanup_shell_surfaces(app_handle) {
                eprintln!("cleanup failed: {error}");
            }
        }
    });
}

#[cfg(target_os = "windows")]
fn shell_runtime_state() -> Mutex<ShellRuntimeState> {
    Mutex::new(ShellRuntimeState::default())
}

#[cfg(not(target_os = "windows"))]
fn shell_runtime_state() -> Mutex<()> {
    Mutex::new(())
}

fn task_preview_state() -> Mutex<task_preview::TaskPreviewRuntimeState> {
    Mutex::new(task_preview::TaskPreviewRuntimeState::default())
}

fn search_panel_state() -> Mutex<search_panel::SearchPanelRuntimeState> {
    Mutex::new(search_panel::SearchPanelRuntimeState::default())
}

fn stack_popup_state() -> Mutex<stack_popup::StackPopupRuntimeState> {
    Mutex::new(stack_popup::StackPopupRuntimeState::default())
}

#[cfg(target_os = "windows")]
#[tauri::command]
fn report_shell_surface_runtime_metrics(
    app_handle: tauri::AppHandle,
    metrics: appbar::FrontendSurfaceMetrics,
) -> Result<appbar::ShellSurfaceRuntimeMetrics, String> {
    appbar::capture_shell_surface_runtime_metrics(&app_handle, metrics)
        .map_err(|error| error.to_string())
}

#[cfg(not(target_os = "windows"))]
#[tauri::command]
fn report_shell_surface_runtime_metrics(
    _app_handle: tauri::AppHandle,
    _metrics: (),
) -> Result<(), String> {
    Err("Shell surface runtime metrics are only available on Windows".to_string())
}
