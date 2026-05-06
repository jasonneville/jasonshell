#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

mod audio;
mod audio_panel;
mod automation;
mod command_panel;
mod contracts;
mod control_plane;
mod dev_tools;
mod diagnostics;
mod launchers;
mod layout;
mod process_manager;
mod providers;
mod quick_commands;
mod quick_icons;
mod search;
mod search_panel;
mod search_sources;
mod settings;
mod settings_panel;
mod shell_paths;
mod shell_windows;
mod stack_popup;
mod system_power;
mod task_preview;
mod task_windows;
mod taskbar_menu;
mod tray_panel;
mod windows_key_hook;
mod workspaces;

#[cfg(target_os = "windows")]
mod system_tray;

#[cfg(target_os = "windows")]
mod appbar;
#[cfg(target_os = "windows")]
mod explorer;

use std::sync::Mutex;
#[cfg(target_os = "windows")]
use tauri::{Emitter, Manager, RunEvent, WindowEvent};

#[cfg(target_os = "windows")]
use appbar::ShellRuntimeState;

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!(
        "JasonShell must be run as a native Windows build. Running it inside WSL builds a \
Linux/WSLg app, so Win32 AppBar edge binding, Windows task-window discovery, shell icons, and \
Windows Search are unavailable. Run `npm run tauri dev` from Windows PowerShell or Windows \
Terminal with Windows Node.js/Rust installed."
    );
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
fn main() {
    let builder = tauri::Builder::default()
        .manage(shell_runtime_state())
        .manage(task_preview_state())
        .manage(search_panel_state())
        .manage(stack_popup_state())
        .manage(diagnostics::diagnostics_state())
        .invoke_handler(tauri::generate_handler![
            launchers::list_pinned_taskbar_apps,
            launchers::launch_pinned_taskbar_app,
            quick_icons::list_quick_icons,
            quick_icons::pin_task_window_quick_icon,
            quick_icons::unpin_quick_icon,
            quick_icons::launch_quick_icon,
            task_windows::list_open_task_windows,
            task_windows::list_taskbar_process_windows,
            task_windows::activate_task_window,
            task_windows::maximize_task_window,
            task_windows::close_task_window,
            task_preview::show_task_window_preview,
            task_preview::hide_task_window_preview,
            taskbar_menu::show_task_window_context_menu,
            taskbar_menu::show_launcher_context_menu,
            taskbar_menu::show_quick_icon_context_menu,
            taskbar_menu::show_top_bar_pin_context_menu,
            search_panel::show_search_panel,
            search_panel::show_centered_search_panel,
            search_panel::resize_search_panel,
            search_panel::hide_search_panel,
            search_panel::publish_search_panel,
            search_panel::get_search_panel_payload,
            process_manager::show_process_manager,
            process_manager::hide_process_manager,
            control_plane::show_control_plane,
            control_plane::hide_control_plane,
            settings_panel::show_settings_panel,
            settings_panel::hide_settings_panel,
            system_power::trigger_system_power_action,
            tray_panel::show_tray_panel,
            tray_panel::hide_tray_panel,
            command_panel::show_command_panel,
            command_panel::hide_command_panel,
            audio_panel::show_audio_panel,
            audio_panel::hide_audio_panel,
            process_manager::list_processes,
            process_manager::kill_process,
            audio::get_audio_state,
            audio::list_audio_devices,
            audio::list_audio_sessions,
            audio::set_master_volume,
            audio::set_master_volume_percent,
            audio::set_master_mute,
            audio::set_app_volume,
            audio::set_app_session_volume_percent,
            audio::set_app_session_mute,
            audio::set_default_audio_device,
            audio::set_default_audio_input_device,
            audio::set_default_audio_output_device,
            system_tray::list_system_tray_icons,
            system_tray::invoke_system_tray_icon,
            search::search_engine,
            search_sources::get_search_provider_health,
            search_sources::request_everything_setup,
            shell_paths::open_shell_path,
            shell_paths::run_control_panel,
            quick_commands::run_quick_command,
            stack_popup::list_pinned_stack_folders,
            stack_popup::pin_stack_folder,
            stack_popup::unpin_stack_folder,
            stack_popup::reorder_pinned_stack_folders,
            stack_popup::show_stack_popup,
            stack_popup::hide_stack_popup,
            stack_popup::get_stack_popup_request,
            stack_popup::begin_stack_popup_focus_loss_hold,
            stack_popup::end_stack_popup_focus_loss_hold,
            stack_popup::resize_stack_popup,
            stack_popup::read_stack_folder,
            stack_popup::suggest_stack_paths,
            stack_popup::resolve_stack_item_icons,
            stack_popup::open_stack_item,
            stack_popup::open_stack_item_with_picker,
            stack_popup::list_stack_open_with_candidates,
            stack_popup::open_stack_item_with_app,
            stack_popup::rename_stack_item,
            stack_popup::copy_stack_items,
            stack_popup::prepare_stack_file_drag,
            stack_popup::cut_stack_items,
            stack_popup::paste_stack_items,
            stack_popup::delete_stack_item,
            stack_popup::new_stack_folder,
            stack_popup::new_stack_text_file,
            stack_popup::open_stack_terminal_here,
            stack_popup::reveal_stack_item,
            stack_popup::extract_stack_archive,
            stack_popup::show_stack_item_properties,
            stack_popup::open_stack_folder_in_vscode,
            automation::parse_automation_cli,
            automation::validate_automation_request,
            automation::get_single_instance_forwarding_contract,
            providers::resolve_provider_registry,
            settings::load_shell_settings,
            settings::save_shell_settings,
            workspaces::list_workspaces,
            workspaces::create_workspace,
            workspaces::update_workspace,
            workspaces::delete_workspace,
            workspaces::activate_workspace,
            diagnostics::record_diagnostic,
            diagnostics::export_diagnostics,
            dev_tools::tool_plans::build_terminal_launch_plan,
            dev_tools::tool_plans::build_editor_launch_plan,
            dev_tools::git_status::get_workspace_git_status,
            dev_tools::task_runner::spawn_workspace_task,
            dev_tools::task_runner::cancel_workspace_task,
            dev_tools::task_runner::list_workspace_task_history,
            dev_tools::task_runner::list_jasonshell_task_process_metadata,
            report_shell_surface_runtime_metrics
        ])
        .on_menu_event(|app_handle, event| {
            taskbar_menu::handle_taskbar_menu_event(app_handle, event);
        })
        .on_window_event(|window, event| {
            if window.label() == shell_windows::STACK_POPUP_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                if stack_popup::suppress_stack_popup_focus_loss(window.app_handle()) {
                    return;
                }
                let _ = window.hide();
                return;
            }

            if window.label() == shell_windows::SEARCH_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(true))
            {
                let _ = window.app_handle().emit_to(
                    shell_windows::TOP_BAR_LABEL,
                    search_panel::SEARCH_PANEL_INTERACTION_EVENT,
                    (),
                );
                return;
            }

            if window.label() == shell_windows::SEARCH_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                let _ = window.emit(search_panel::SEARCH_PANEL_CLOSED_EVENT, ());
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

            if window.label() == shell_windows::SETTINGS_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                let _ = window.hide();
                return;
            }

            if window.label() == shell_windows::AUDIO_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                let _ = window.app_handle().emit_to(
                    shell_windows::TOP_BAR_LABEL,
                    audio_panel::AUDIO_PANEL_CLOSED_EVENT,
                    (),
                );
                let _ = window.hide();
                return;
            }

            if window.label() == shell_windows::TRAY_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                let _ = window.app_handle().emit_to(
                    shell_windows::TOP_BAR_LABEL,
                    tray_panel::TRAY_PANEL_CLOSED_EVENT,
                    (),
                );
                let _ = window.hide();
                return;
            }

            if window.label() == shell_windows::COMMAND_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                let _ = window.app_handle().emit_to(
                    shell_windows::TOP_BAR_LABEL,
                    command_panel::COMMAND_PANEL_CLOSED_EVENT,
                    (),
                );
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
            search::providers::apps::initialize_app_index_cache(app.handle());
            search::providers::apps::warm_app_index_async();
            windows_key_hook::install_windows_key_hook(app.handle().clone()).map_err(|error| {
                format!("Windows-key hook is required to suppress the Start Menu: {error}")
            })?;

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
            windows_key_hook::uninstall_windows_key_hook();
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
