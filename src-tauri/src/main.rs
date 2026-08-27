#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

mod audio;
mod audio_panel;
mod automation;
mod calendar_panel;
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
mod quick_launch_panel;
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
mod terminal_panel;
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
use std::time::Duration;
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
    match launchers::handle_launch_pinned_taskbar_helper_args() {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
    match task_windows::handle_task_window_helper_args() {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
    match task_windows::handle_taskbar_flash_fixture_args() {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }

    let builder = tauri::Builder::default()
        .manage(shell_runtime_state())
        .manage(task_preview_state())
        .manage(search_panel_state())
        .manage(stack_popup_state())
        .manage(diagnostics::diagnostics_state())
        .invoke_handler(tauri::generate_handler![
            launchers::list_pinned_taskbar_apps,
            launchers::launch_pinned_taskbar_app,
            task_windows::list_open_task_windows,
            task_windows::get_taskbar_runtime_diagnostics,
            task_windows::request_taskbar_windows_refresh,
            task_windows::list_taskbar_process_windows,
            task_windows::activate_task_window,
            task_windows::maximize_task_window,
            task_windows::close_task_window,
            task_preview::show_task_window_preview,
            task_preview::hide_task_window_preview,
            taskbar_menu::show_task_window_context_menu,
            taskbar_menu::show_launcher_context_menu,
            taskbar_menu::show_top_bar_pin_context_menu,
            search_panel::show_search_panel,
            search_panel::show_centered_search_panel,
            search_panel::resize_search_panel,
            search_panel::hide_search_panel,
            search_panel::publish_search_panel,
            search_panel::get_search_panel_payload,
            appbar::resize_shell_bar,
            settings::save_shell_bar_height,
            settings::save_shell_bar_lock,
            process_manager::show_process_manager,
            process_manager::hide_process_manager,
            quick_launch_panel::show_quick_launch_panel,
            quick_launch_panel::hide_quick_launch_panel_on_focus_loss,
            quick_launch_panel::hide_quick_launch_panel,
            quick_launch_panel::select_quick_launch_panel,
            quick_launch_panel::run_quick_launch_panel_as_admin,
            quick_launch_panel::show_quick_launch_panel_context_menu,
            control_plane::show_control_plane,
            control_plane::hide_control_plane,
            settings_panel::show_settings_panel,
            settings_panel::hide_settings_panel,
            system_power::trigger_system_power_action,
            tray_panel::show_tray_panel,
            tray_panel::hide_tray_panel,
            terminal_panel::show_terminal_panel,
            terminal_panel::hide_terminal_panel,
            command_panel::show_command_panel,
            command_panel::hide_command_panel,
            command_panel::save_command_panel_size,
            audio_panel::show_audio_panel,
            audio_panel::hide_audio_panel,
            calendar_panel::show_calendar_panel,
            calendar_panel::hide_calendar_panel,
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
            shell_paths::launch_app_path,
            shell_paths::run_control_panel,
            quick_commands::run_quick_command,
            quick_commands::stop_quick_command,
            quick_commands::list_quick_command_history,
            quick_commands::send_quick_command_input,
            quick_commands::save_quick_commands_settings,
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
            stack_popup::get_stack_git_status,
            stack_popup::open_stack_git_remote_url,
            stack_popup::stack_git_add_paths,
            stack_popup::stack_git_unstage_paths,
            stack_popup::stack_git_revert_paths,
            stack_popup::stack_git_diff,
            stack_popup::stack_git_stashes,
            stack_popup::stack_git_stash,
            stack_popup::stack_git_stash_apply,
            stack_popup::stack_git_stash_pop,
            stack_popup::stack_git_stash_drop,
            stack_popup::stack_git_commit,
            stack_popup::stack_git_commit_files,
            stack_popup::stack_git_commit_file_diff,
            stack_popup::stack_git_stash_files,
            stack_popup::stack_git_stash_file_diff,
            stack_popup::stack_git_log,
            stack_popup::stack_git_tree,
            stack_popup::stack_git_branches,
            stack_popup::stack_git_fetch,
            stack_popup::stack_git_pull,
            stack_popup::stack_git_push,
            stack_popup::stack_git_checkout_branch,
            stack_popup::stack_git_create_branch,
            stack_popup::stack_git_delete_branch,
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
            stack_popup::start_persistent_terminal,
            stack_popup::start_stack_terminal,
            stack_popup::read_stack_terminal,
            stack_popup::write_stack_terminal,
            stack_popup::resize_stack_terminal,
            stack_popup::stop_stack_terminal,
            stack_popup::poll_stack_terminal_session,
            stack_popup::list_stack_terminals,
            stack_popup::rename_stack_terminal,
            stack_popup::stop_terminal_panel_sessions,
            stack_popup::get_stack_terminal_cwd,
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
                && matches!(event, WindowEvent::Focused(true))
            {
                stack_popup::restore_stack_popup_topmost(window.app_handle());
                return;
            }

            if window.label() == shell_windows::STACK_POPUP_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                if stack_popup::suppress_stack_popup_focus_loss(window.app_handle()) {
                    return;
                }
                if window
                    .app_handle()
                    .get_webview_window(shell_windows::TOP_BAR_LABEL)
                    .and_then(|top_bar| top_bar.is_focused().ok())
                    .unwrap_or(false)
                {
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
                let _ = search_panel::emit_search_panel_closed_to_top_bar(window.app_handle());
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

            if window.label() == shell_windows::QUICK_LAUNCH_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                let _ = quick_launch_panel::hide_quick_launch_panel_on_focus_loss(
                    window.clone(),
                    window.app_handle().clone(),
                );
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
                let _ = audio_panel::emit_audio_panel_closed(window.app_handle());
                let _ = window.hide();
                return;
            }

            if window.label() == shell_windows::CALENDAR_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                let _ = calendar_panel::emit_calendar_panel_closed(window.app_handle());
                let _ = window.hide();
                return;
            }

            if window.label() == shell_windows::TRAY_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                if tray_panel::take_tray_panel_focus_loss_suppression() {
                    // Keep the tray visible under Explorer-owned native icon menus. The user can
                    // dismiss it through the top-bar tray toggle or another mutually exclusive popup.
                    return;
                }
                let _ = window.app_handle().emit_to(
                    shell_windows::TOP_BAR_LABEL,
                    tray_panel::TRAY_PANEL_CLOSED_EVENT,
                    (),
                );
                let _ = window.hide();
                return;
            }

            if window.label() == shell_windows::TERMINAL_PANEL_LABEL {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = terminal_panel::hide_terminal_panel(window.app_handle().clone());
                    return;
                }
            }

            if window.label() == shell_windows::COMMAND_PANEL_LABEL
                && matches!(event, WindowEvent::Focused(false))
            {
                let focus_loss_nonce = command_panel::invalidate_command_panel_focus_loss_nonce();
                let app_handle = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = tauri::async_runtime::spawn_blocking(move || {
                        std::thread::sleep(Duration::from_millis(150));
                    })
                    .await;
                    if !command_panel::command_panel_focus_loss_nonce_is_current(focus_loss_nonce) {
                        return;
                    }
                    let Some(panel) =
                        app_handle.get_webview_window(shell_windows::COMMAND_PANEL_LABEL)
                    else {
                        return;
                    };
                    if !panel.is_visible().ok().unwrap_or(false)
                        || panel.is_focused().ok().unwrap_or(false)
                        || panel.is_maximized().ok().unwrap_or(false)
                        || panel.is_minimized().ok().unwrap_or(false)
                    {
                        return;
                    }
                    if !command_panel::command_panel_focus_loss_nonce_is_current(focus_loss_nonce) {
                        return;
                    }
                    let _ = app_handle.emit_to(
                        shell_windows::TOP_BAR_LABEL,
                        command_panel::COMMAND_PANEL_CLOSED_EVENT,
                        (),
                    );
                    let _ = panel.hide();
                });
                return;
            }

            if window.label() == shell_windows::COMMAND_PANEL_LABEL {
                if let WindowEvent::Resized(size) = event {
                    let _ = command_panel::save_command_panel_size_for_app(
                        &window.app_handle().clone(),
                        size.width,
                        size.height,
                    );
                }
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
            windows_key_hook::install_windows_key_hook(app.handle().clone())
                .map_err(|error| format!("search hotkey hook is required: {error}"))?;

            #[cfg(target_os = "windows")]
            {
                appbar::activate_shell_surfaces(app, &windows)?;
                task_windows::start_notification_tracking();
                task_windows::start_taskbar_snapshot_pipeline(app.handle());
                if let Err(error) = task_windows::start_taskbar_hooks(app.handle().clone()) {
                    eprintln!("native hook start failed: {error}");
                }
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
            if let Some(state) =
                app_handle.try_state::<Mutex<stack_popup::StackPopupRuntimeState>>()
            {
                if let Err(error) = stack_popup::terminal::stop_terminal_sessions_for_target(
                    app_handle,
                    &state,
                    shell_windows::TERMINAL_PANEL_LABEL,
                ) {
                    eprintln!("terminal cleanup failed: {error}");
                }
            }
            if let Err(error) = appbar::cleanup_shell_surfaces(app_handle) {
                eprintln!("cleanup failed: {error}");
            }
            task_windows::stop_taskbar_hooks();
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
