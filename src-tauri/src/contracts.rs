#![allow(dead_code)]

pub mod surfaces {
    pub const TOP_BAR: &str = "top-bar";
    pub const BOTTOM_BAR: &str = "bottom-bar";
    pub const TASK_PREVIEW: &str = "task-preview";
    pub const SEARCH_PANEL: &str = "search-panel";
    pub const STACK_POPUP: &str = "stack-popup";
    pub const PROCESS_MANAGER: &str = "process-manager";
    pub const CONTROL_PLANE: &str = "control-plane";
    pub const SETTINGS_PANEL: &str = "settings-panel";
    pub const TRAY_PANEL: &str = "tray-panel";
    pub const TERMINAL_PANEL: &str = "terminal-panel";
    pub const COMMAND_PANEL: &str = "command-panel";
    pub const AUDIO_PANEL: &str = "audio-panel";
    pub const CALENDAR_PANEL: &str = "calendar-panel";

    pub const ALL: &[&str] = &[
        TOP_BAR,
        BOTTOM_BAR,
        TASK_PREVIEW,
        SEARCH_PANEL,
        STACK_POPUP,
        PROCESS_MANAGER,
        CONTROL_PLANE,
        SETTINGS_PANEL,
        TRAY_PANEL,
        TERMINAL_PANEL,
        COMMAND_PANEL,
        AUDIO_PANEL,
        CALENDAR_PANEL,
    ];
}

pub mod commands {
    pub const LIST_PINNED_TASKBAR_APPS: &str = "list_pinned_taskbar_apps";
    pub const LAUNCH_PINNED_TASKBAR_APP: &str = "launch_pinned_taskbar_app";
    pub const LIST_OPEN_TASK_WINDOWS: &str = "list_open_task_windows";
    pub const LIST_TASKBAR_PROCESS_WINDOWS: &str = "list_taskbar_process_windows";
    pub const ACTIVATE_TASK_WINDOW: &str = "activate_task_window";
    pub const MAXIMIZE_TASK_WINDOW: &str = "maximize_task_window";
    pub const CLOSE_TASK_WINDOW: &str = "close_task_window";
    pub const SHOW_TASK_WINDOW_PREVIEW: &str = "show_task_window_preview";
    pub const HIDE_TASK_WINDOW_PREVIEW: &str = "hide_task_window_preview";
    pub const SHOW_TASK_WINDOW_CONTEXT_MENU: &str = "show_task_window_context_menu";
    pub const SHOW_LAUNCHER_CONTEXT_MENU: &str = "show_launcher_context_menu";
    pub const SHOW_TOP_BAR_PIN_CONTEXT_MENU: &str = "show_top_bar_pin_context_menu";
    pub const SHOW_SEARCH_PANEL: &str = "show_search_panel";
    pub const SEARCH_ENGINE: &str = "search_engine";
    pub const HIDE_SEARCH_PANEL: &str = "hide_search_panel";
    pub const PUBLISH_SEARCH_PANEL: &str = "publish_search_panel";
    pub const GET_SEARCH_PANEL_PAYLOAD: &str = "get_search_panel_payload";
    pub const SHOW_PROCESS_MANAGER: &str = "show_process_manager";
    pub const HIDE_PROCESS_MANAGER: &str = "hide_process_manager";
    pub const SHOW_CONTROL_PLANE: &str = "show_control_plane";
    pub const HIDE_CONTROL_PLANE: &str = "hide_control_plane";
    pub const SHOW_SETTINGS_PANEL: &str = "show_settings_panel";
    pub const HIDE_SETTINGS_PANEL: &str = "hide_settings_panel";
    pub const SHOW_TRAY_PANEL: &str = "show_tray_panel";
    pub const HIDE_TRAY_PANEL: &str = "hide_tray_panel";
    pub const SHOW_TERMINAL_PANEL: &str = "show_terminal_panel";
    pub const HIDE_TERMINAL_PANEL: &str = "hide_terminal_panel";
    pub const SHOW_COMMAND_PANEL: &str = "show_command_panel";
    pub const HIDE_COMMAND_PANEL: &str = "hide_command_panel";
    pub const SHOW_AUDIO_PANEL: &str = "show_audio_panel";
    pub const HIDE_AUDIO_PANEL: &str = "hide_audio_panel";
    pub const SHOW_CALENDAR_PANEL: &str = "show_calendar_panel";
    pub const HIDE_CALENDAR_PANEL: &str = "hide_calendar_panel";
    pub const LIST_PROCESSES: &str = "list_processes";
    pub const KILL_PROCESS: &str = "kill_process";
    pub const GET_AUDIO_STATE: &str = "get_audio_state";
    pub const LIST_AUDIO_DEVICES: &str = "list_audio_devices";
    pub const LIST_AUDIO_SESSIONS: &str = "list_audio_sessions";
    pub const SET_MASTER_VOLUME: &str = "set_master_volume";
    pub const SET_MASTER_VOLUME_PERCENT: &str = "set_master_volume_percent";
    pub const SET_MASTER_MUTE: &str = "set_master_mute";
    pub const SET_APP_VOLUME: &str = "set_app_volume";
    pub const SET_APP_SESSION_VOLUME_PERCENT: &str = "set_app_session_volume_percent";
    pub const SET_APP_SESSION_MUTE: &str = "set_app_session_mute";
    pub const SET_DEFAULT_AUDIO_DEVICE: &str = "set_default_audio_device";
    pub const SET_DEFAULT_AUDIO_INPUT_DEVICE: &str = "set_default_audio_input_device";
    pub const SET_DEFAULT_AUDIO_OUTPUT_DEVICE: &str = "set_default_audio_output_device";
    pub const LIST_SYSTEM_TRAY_ICONS: &str = "list_system_tray_icons";
    pub const INVOKE_SYSTEM_TRAY_ICON: &str = "invoke_system_tray_icon";
    pub const GET_SEARCH_PROVIDER_HEALTH: &str = "get_search_provider_health";
    pub const REQUEST_EVERYTHING_SETUP: &str = "request_everything_setup";
    pub const OPEN_SHELL_PATH: &str = "open_shell_path";
    pub const LAUNCH_APP_PATH: &str = "launch_app_path";
    pub const RUN_CONTROL_PANEL: &str = "run_control_panel";
    pub const RUN_QUICK_COMMAND: &str = "run_quick_command";
    pub const LIST_PINNED_STACK_FOLDERS: &str = "list_pinned_stack_folders";
    pub const PIN_STACK_FOLDER: &str = "pin_stack_folder";
    pub const UNPIN_STACK_FOLDER: &str = "unpin_stack_folder";
    pub const REORDER_PINNED_STACK_FOLDERS: &str = "reorder_pinned_stack_folders";
    pub const SHOW_STACK_POPUP: &str = "show_stack_popup";
    pub const HIDE_STACK_POPUP: &str = "hide_stack_popup";
    pub const GET_STACK_POPUP_REQUEST: &str = "get_stack_popup_request";
    pub const BEGIN_STACK_POPUP_FOCUS_LOSS_HOLD: &str = "begin_stack_popup_focus_loss_hold";
    pub const END_STACK_POPUP_FOCUS_LOSS_HOLD: &str = "end_stack_popup_focus_loss_hold";
    pub const RESIZE_STACK_POPUP: &str = "resize_stack_popup";
    pub const READ_STACK_FOLDER: &str = "read_stack_folder";
    pub const GET_STACK_GIT_STATUS: &str = "get_stack_git_status";
    pub const STACK_GIT_ADD_PATHS: &str = "stack_git_add_paths";
    pub const STACK_GIT_COMMIT: &str = "stack_git_commit";
    pub const STACK_GIT_LOG: &str = "stack_git_log";
    pub const STACK_GIT_TREE: &str = "stack_git_tree";
    pub const STACK_GIT_BRANCHES: &str = "stack_git_branches";
    pub const STACK_GIT_FETCH: &str = "stack_git_fetch";
    pub const STACK_GIT_PULL: &str = "stack_git_pull";
    pub const STACK_GIT_PUSH: &str = "stack_git_push";
    pub const STACK_GIT_CHECKOUT_BRANCH: &str = "stack_git_checkout_branch";
    pub const STACK_GIT_CREATE_BRANCH: &str = "stack_git_create_branch";
    pub const SUGGEST_STACK_PATHS: &str = "suggest_stack_paths";
    pub const RESOLVE_STACK_ITEM_ICONS: &str = "resolve_stack_item_icons";
    pub const OPEN_STACK_ITEM: &str = "open_stack_item";
    pub const OPEN_STACK_ITEM_WITH_PICKER: &str = "open_stack_item_with_picker";
    pub const LIST_STACK_OPEN_WITH_CANDIDATES: &str = "list_stack_open_with_candidates";
    pub const OPEN_STACK_ITEM_WITH_APP: &str = "open_stack_item_with_app";
    pub const RENAME_STACK_ITEM: &str = "rename_stack_item";
    pub const COPY_STACK_ITEMS: &str = "copy_stack_items";
    pub const PREPARE_STACK_FILE_DRAG: &str = "prepare_stack_file_drag";
    pub const CUT_STACK_ITEMS: &str = "cut_stack_items";
    pub const PASTE_STACK_ITEMS: &str = "paste_stack_items";
    pub const DELETE_STACK_ITEM: &str = "delete_stack_item";
    pub const NEW_STACK_FOLDER: &str = "new_stack_folder";
    pub const NEW_STACK_TEXT_FILE: &str = "new_stack_text_file";
    pub const OPEN_STACK_TERMINAL_HERE: &str = "open_stack_terminal_here";
    pub const START_PERSISTENT_TERMINAL: &str = "start_persistent_terminal";
    pub const START_STACK_TERMINAL: &str = "start_stack_terminal";
    pub const READ_STACK_TERMINAL: &str = "read_stack_terminal";
    pub const WRITE_STACK_TERMINAL: &str = "write_stack_terminal";
    pub const RESIZE_STACK_TERMINAL: &str = "resize_stack_terminal";
    pub const STOP_STACK_TERMINAL: &str = "stop_stack_terminal";
    pub const POLL_STACK_TERMINAL_SESSION: &str = "poll_stack_terminal_session";
    pub const GET_STACK_TERMINAL_CWD: &str = "get_stack_terminal_cwd";
    pub const REVEAL_STACK_ITEM: &str = "reveal_stack_item";
    pub const EXTRACT_STACK_ARCHIVE: &str = "extract_stack_archive";
    pub const SHOW_STACK_ITEM_PROPERTIES: &str = "show_stack_item_properties";
    pub const REPORT_SHELL_SURFACE_RUNTIME_METRICS: &str = "report_shell_surface_runtime_metrics";
    pub const LOAD_SHELL_SETTINGS: &str = "load_shell_settings";
    pub const SAVE_SHELL_SETTINGS: &str = "save_shell_settings";
    pub const LIST_WORKSPACES: &str = "list_workspaces";
    pub const CREATE_WORKSPACE: &str = "create_workspace";
    pub const UPDATE_WORKSPACE: &str = "update_workspace";
    pub const DELETE_WORKSPACE: &str = "delete_workspace";
    pub const ACTIVATE_WORKSPACE: &str = "activate_workspace";
    pub const RECORD_DIAGNOSTIC: &str = "record_diagnostic";
    pub const EXPORT_DIAGNOSTICS: &str = "export_diagnostics";
    pub const BUILD_TERMINAL_LAUNCH_PLAN: &str = "build_terminal_launch_plan";
    pub const BUILD_EDITOR_LAUNCH_PLAN: &str = "build_editor_launch_plan";
    pub const GET_WORKSPACE_GIT_STATUS: &str = "get_workspace_git_status";
    pub const SPAWN_WORKSPACE_TASK: &str = "spawn_workspace_task";
    pub const CANCEL_WORKSPACE_TASK: &str = "cancel_workspace_task";
    pub const LIST_WORKSPACE_TASK_HISTORY: &str = "list_workspace_task_history";
    pub const LIST_JASONSHELL_TASK_PROCESS_METADATA: &str = "list_jasonshell_task_process_metadata";
    pub const PARSE_AUTOMATION_CLI: &str = "parse_automation_cli";
    pub const VALIDATE_AUTOMATION_REQUEST: &str = "validate_automation_request";
    pub const GET_SINGLE_INSTANCE_FORWARDING_CONTRACT: &str =
        "get_single_instance_forwarding_contract";
    pub const RESOLVE_PROVIDER_REGISTRY: &str = "resolve_provider_registry";

    pub const ALL: &[&str] = &[
        LIST_PINNED_TASKBAR_APPS,
        LAUNCH_PINNED_TASKBAR_APP,
        LIST_OPEN_TASK_WINDOWS,
        LIST_TASKBAR_PROCESS_WINDOWS,
        ACTIVATE_TASK_WINDOW,
        MAXIMIZE_TASK_WINDOW,
        CLOSE_TASK_WINDOW,
        SHOW_TASK_WINDOW_PREVIEW,
        HIDE_TASK_WINDOW_PREVIEW,
        SHOW_TASK_WINDOW_CONTEXT_MENU,
        SHOW_LAUNCHER_CONTEXT_MENU,
        SHOW_TOP_BAR_PIN_CONTEXT_MENU,
        SHOW_SEARCH_PANEL,
        SEARCH_ENGINE,
        HIDE_SEARCH_PANEL,
        PUBLISH_SEARCH_PANEL,
        GET_SEARCH_PANEL_PAYLOAD,
        SHOW_PROCESS_MANAGER,
        HIDE_PROCESS_MANAGER,
        SHOW_CONTROL_PLANE,
        HIDE_CONTROL_PLANE,
        SHOW_SETTINGS_PANEL,
        HIDE_SETTINGS_PANEL,
        SHOW_TRAY_PANEL,
        HIDE_TRAY_PANEL,
        SHOW_TERMINAL_PANEL,
        HIDE_TERMINAL_PANEL,
        SHOW_COMMAND_PANEL,
        HIDE_COMMAND_PANEL,
        SHOW_AUDIO_PANEL,
        HIDE_AUDIO_PANEL,
        SHOW_CALENDAR_PANEL,
        HIDE_CALENDAR_PANEL,
        LIST_PROCESSES,
        KILL_PROCESS,
        GET_AUDIO_STATE,
        LIST_AUDIO_DEVICES,
        LIST_AUDIO_SESSIONS,
        SET_MASTER_VOLUME,
        SET_MASTER_VOLUME_PERCENT,
        SET_MASTER_MUTE,
        SET_APP_VOLUME,
        SET_APP_SESSION_VOLUME_PERCENT,
        SET_APP_SESSION_MUTE,
        SET_DEFAULT_AUDIO_DEVICE,
        SET_DEFAULT_AUDIO_INPUT_DEVICE,
        SET_DEFAULT_AUDIO_OUTPUT_DEVICE,
        LIST_SYSTEM_TRAY_ICONS,
        INVOKE_SYSTEM_TRAY_ICON,
        GET_SEARCH_PROVIDER_HEALTH,
        REQUEST_EVERYTHING_SETUP,
        OPEN_SHELL_PATH,
        LAUNCH_APP_PATH,
        RUN_CONTROL_PANEL,
        RUN_QUICK_COMMAND,
        LIST_PINNED_STACK_FOLDERS,
        PIN_STACK_FOLDER,
        UNPIN_STACK_FOLDER,
        REORDER_PINNED_STACK_FOLDERS,
        SHOW_STACK_POPUP,
        HIDE_STACK_POPUP,
        GET_STACK_POPUP_REQUEST,
        BEGIN_STACK_POPUP_FOCUS_LOSS_HOLD,
        END_STACK_POPUP_FOCUS_LOSS_HOLD,
        RESIZE_STACK_POPUP,
        READ_STACK_FOLDER,
        GET_STACK_GIT_STATUS,
        STACK_GIT_ADD_PATHS,
        STACK_GIT_COMMIT,
        STACK_GIT_LOG,
        STACK_GIT_TREE,
        STACK_GIT_BRANCHES,
        STACK_GIT_FETCH,
        STACK_GIT_PULL,
        STACK_GIT_PUSH,
        STACK_GIT_CHECKOUT_BRANCH,
        STACK_GIT_CREATE_BRANCH,
        SUGGEST_STACK_PATHS,
        RESOLVE_STACK_ITEM_ICONS,
        OPEN_STACK_ITEM,
        OPEN_STACK_ITEM_WITH_PICKER,
        LIST_STACK_OPEN_WITH_CANDIDATES,
        OPEN_STACK_ITEM_WITH_APP,
        RENAME_STACK_ITEM,
        COPY_STACK_ITEMS,
        PREPARE_STACK_FILE_DRAG,
        CUT_STACK_ITEMS,
        PASTE_STACK_ITEMS,
        DELETE_STACK_ITEM,
        NEW_STACK_FOLDER,
        NEW_STACK_TEXT_FILE,
        OPEN_STACK_TERMINAL_HERE,
        START_PERSISTENT_TERMINAL,
        START_STACK_TERMINAL,
        READ_STACK_TERMINAL,
        WRITE_STACK_TERMINAL,
        RESIZE_STACK_TERMINAL,
        STOP_STACK_TERMINAL,
        POLL_STACK_TERMINAL_SESSION,
        GET_STACK_TERMINAL_CWD,
        REVEAL_STACK_ITEM,
        EXTRACT_STACK_ARCHIVE,
        SHOW_STACK_ITEM_PROPERTIES,
        REPORT_SHELL_SURFACE_RUNTIME_METRICS,
        LOAD_SHELL_SETTINGS,
        SAVE_SHELL_SETTINGS,
        LIST_WORKSPACES,
        CREATE_WORKSPACE,
        UPDATE_WORKSPACE,
        DELETE_WORKSPACE,
        ACTIVATE_WORKSPACE,
        RECORD_DIAGNOSTIC,
        EXPORT_DIAGNOSTICS,
        BUILD_TERMINAL_LAUNCH_PLAN,
        BUILD_EDITOR_LAUNCH_PLAN,
        GET_WORKSPACE_GIT_STATUS,
        SPAWN_WORKSPACE_TASK,
        CANCEL_WORKSPACE_TASK,
        LIST_WORKSPACE_TASK_HISTORY,
        LIST_JASONSHELL_TASK_PROCESS_METADATA,
        PARSE_AUTOMATION_CLI,
        VALIDATE_AUTOMATION_REQUEST,
        GET_SINGLE_INSTANCE_FORWARDING_CONTRACT,
        RESOLVE_PROVIDER_REGISTRY,
    ];
}

pub mod events {
    pub const AUDIO_PANEL_OPEN: &str = "audio-panel:open";
    pub const AUDIO_PANEL_CLOSED: &str = "audio-panel:closed";
    pub const CALENDAR_PANEL_OPEN: &str = "calendar-panel:open";
    pub const CALENDAR_PANEL_CLOSED: &str = "calendar-panel:closed";
    pub const COMMAND_PANEL_CLOSED: &str = "command-panel:closed";
    pub const TERMINAL_PANEL_OPEN: &str = "terminal-panel:open";
    pub const TERMINAL_PANEL_CLOSED: &str = "terminal-panel:closed";
    pub const PROCESS_MANAGER_OPEN: &str = "process-manager:open";
    pub const PROCESS_MANAGER_CLOSED: &str = "process-manager:closed";
    pub const SEARCH_TOGGLE_CENTERED: &str = "search:toggle-centered";
    pub const SEARCH_ENGINE_PROGRESS: &str = "search-engine:progress";
    pub const SEARCH_INDEX_REFRESHED: &str = "search-index:refreshed";
    pub const SEARCH_PANEL_ACTIVATE: &str = "search-panel:activate";
    pub const SEARCH_PANEL_CLOSED: &str = "search-panel:closed";
    pub const SEARCH_PANEL_EXPAND_GROUP: &str = "search-panel:expand-group";
    pub const SEARCH_PANEL_INTERACTION: &str = "search-panel:interaction";
    pub const SEARCH_PANEL_KEY: &str = "search-panel:key";
    pub const SEARCH_PANEL_PIN_FOLDER: &str = "search-panel:pin-folder";
    pub const SEARCH_PANEL_QUERY: &str = "search-panel:query";
    pub const SEARCH_PANEL_SELECT: &str = "search-panel:select";
    pub const SEARCH_PANEL_UPDATE: &str = "search-panel:update";
    pub const STACK_POPUP_OPEN: &str = "stack-popup:open";
    pub const STACK_TERMINAL_CLOSED: &str = "stack-terminal:closed";
    pub const STACK_TERMINAL_CWD: &str = "stack-terminal:cwd";
    pub const STACK_TERMINAL_OUTPUT: &str = "stack-terminal:output";
    pub const STACK_PINS_UPDATED: &str = "stack-pins:updated";
    pub const TASK_PREVIEW_HIDE: &str = "task-preview:hide";
    pub const TASK_PREVIEW_HOVER_ENTER: &str = "task-preview:hover-enter";
    pub const TASK_PREVIEW_UPDATE: &str = "task-preview:update";
    pub const TASK_COMPLETED: &str = "task:completed";
    pub const TASK_OUTPUT: &str = "task:output";
    pub const TASK_STARTED: &str = "task:started";
    pub const TASKBAR_REFRESH_LAUNCHERS: &str = "taskbar:refresh-launchers";
    pub const TASKBAR_REFRESH_WINDOWS: &str = "taskbar:refresh-windows";
    pub const TOP_BAR_PIN_MENU_ACTION: &str = "top-bar:pin-menu-action";
    pub const TRAY_PANEL_OPEN: &str = "tray-panel:open";
    pub const TRAY_PANEL_CLOSED: &str = "tray-panel:closed";

    pub const ALL: &[&str] = &[
        AUDIO_PANEL_OPEN,
        AUDIO_PANEL_CLOSED,
        CALENDAR_PANEL_OPEN,
        CALENDAR_PANEL_CLOSED,
        COMMAND_PANEL_CLOSED,
        TERMINAL_PANEL_OPEN,
        TERMINAL_PANEL_CLOSED,
        PROCESS_MANAGER_OPEN,
        PROCESS_MANAGER_CLOSED,
        SEARCH_TOGGLE_CENTERED,
        SEARCH_ENGINE_PROGRESS,
        SEARCH_INDEX_REFRESHED,
        SEARCH_PANEL_ACTIVATE,
        SEARCH_PANEL_CLOSED,
        SEARCH_PANEL_EXPAND_GROUP,
        SEARCH_PANEL_INTERACTION,
        SEARCH_PANEL_KEY,
        SEARCH_PANEL_PIN_FOLDER,
        SEARCH_PANEL_QUERY,
        SEARCH_PANEL_SELECT,
        SEARCH_PANEL_UPDATE,
        STACK_POPUP_OPEN,
        STACK_TERMINAL_CLOSED,
        STACK_TERMINAL_CWD,
        STACK_TERMINAL_OUTPUT,
        STACK_PINS_UPDATED,
        TASK_PREVIEW_HIDE,
        TASK_PREVIEW_HOVER_ENTER,
        TASK_PREVIEW_UPDATE,
        TASK_COMPLETED,
        TASK_OUTPUT,
        TASK_STARTED,
        TASKBAR_REFRESH_LAUNCHERS,
        TASKBAR_REFRESH_WINDOWS,
        TOP_BAR_PIN_MENU_ACTION,
        TRAY_PANEL_CLOSED,
        TRAY_PANEL_OPEN,
    ];
}

#[cfg(test)]
mod tests {
    use super::{commands, events, surfaces};
    use crate::shell_windows;
    use std::collections::HashSet;

    #[test]
    fn shell_surface_contract_contains_current_windows() {
        assert_eq!(
            surfaces::ALL,
            &[
                "top-bar",
                "bottom-bar",
                "task-preview",
                "search-panel",
                "stack-popup",
                "process-manager",
                "control-plane",
                "settings-panel",
                "tray-panel",
                "terminal-panel",
                "command-panel",
                "audio-panel",
                "calendar-panel",
            ]
        );
    }

    #[test]
    fn shell_surface_contract_includes_all_shipped_shell_windows() {
        let contracted_labels = surfaces::ALL.iter().copied().collect::<HashSet<_>>();
        let missing_labels = shell_windows::ALL_LABELS
            .iter()
            .copied()
            .filter(|label| !contracted_labels.contains(label))
            .collect::<Vec<_>>();

        assert!(
            missing_labels.is_empty(),
            "contracts::surfaces::ALL is missing shipped shell window labels: {:?}",
            missing_labels
        );
    }

    #[test]
    fn new_command_contracts_are_unique_and_stable() {
        let unique = commands::ALL.iter().copied().collect::<HashSet<_>>();
        assert_eq!(unique.len(), commands::ALL.len());
        assert!(unique.contains("load_shell_settings"));
        assert!(unique.contains("save_shell_settings"));
        assert!(unique.contains("list_workspaces"));
        assert!(unique.contains("activate_workspace"));
        assert!(unique.contains("show_stack_popup"));
        assert!(unique.contains("begin_stack_popup_focus_loss_hold"));
        assert!(unique.contains("end_stack_popup_focus_loss_hold"));
        assert!(unique.contains("resize_stack_popup"));
        assert!(unique.contains("read_stack_folder"));
        assert!(unique.contains("get_stack_git_status"));
        assert!(unique.contains("stack_git_add_paths"));
        assert!(unique.contains("stack_git_commit"));
        assert!(unique.contains("stack_git_log"));
        assert!(unique.contains("stack_git_tree"));
        assert!(unique.contains("stack_git_branches"));
        assert!(unique.contains("stack_git_fetch"));
        assert!(unique.contains("stack_git_pull"));
        assert!(unique.contains("stack_git_push"));
        assert!(unique.contains("stack_git_checkout_branch"));
        assert!(unique.contains("stack_git_create_branch"));
        assert!(unique.contains("list_stack_open_with_candidates"));
        assert!(unique.contains("open_stack_item_with_app"));
        assert!(unique.contains("prepare_stack_file_drag"));
        assert!(unique.contains("new_stack_text_file"));
        assert!(unique.contains("open_stack_terminal_here"));
        assert!(unique.contains("start_stack_terminal"));
        assert!(unique.contains("read_stack_terminal"));
        assert!(unique.contains("write_stack_terminal"));
        assert!(unique.contains("resize_stack_terminal"));
        assert!(unique.contains("stop_stack_terminal"));
        assert!(unique.contains("poll_stack_terminal_session"));
        assert!(unique.contains("get_stack_terminal_cwd"));
        assert!(unique.contains("record_diagnostic"));
        assert!(unique.contains("export_diagnostics"));
        assert!(unique.contains("build_terminal_launch_plan"));
        assert!(unique.contains("get_workspace_git_status"));
        assert!(unique.contains("spawn_workspace_task"));
        assert!(unique.contains("list_jasonshell_task_process_metadata"));
        assert!(unique.contains("parse_automation_cli"));
        assert!(unique.contains("validate_automation_request"));
        assert!(unique.contains("get_single_instance_forwarding_contract"));
        assert!(unique.contains("resolve_provider_registry"));
        assert!(unique.contains("show_control_plane"));
        assert!(unique.contains("hide_control_plane"));
        assert!(unique.contains("show_settings_panel"));
        assert!(unique.contains("hide_settings_panel"));
        assert!(unique.contains("show_tray_panel"));
        assert!(unique.contains("hide_tray_panel"));
        assert!(unique.contains("show_command_panel"));
        assert!(unique.contains("hide_command_panel"));
        assert!(unique.contains("show_audio_panel"));
        assert!(unique.contains("hide_audio_panel"));
        assert!(unique.contains("show_calendar_panel"));
        assert!(unique.contains("hide_calendar_panel"));
        assert!(unique.contains("get_audio_state"));
        assert!(unique.contains("list_audio_devices"));
        assert!(unique.contains("list_audio_sessions"));
        assert!(unique.contains("set_master_volume"));
        assert!(unique.contains("set_master_volume_percent"));
        assert!(unique.contains("set_app_volume"));
        assert!(unique.contains("set_app_session_volume_percent"));
        assert!(unique.contains("set_default_audio_device"));
        assert!(unique.contains("set_default_audio_input_device"));
        assert!(unique.contains("set_default_audio_output_device"));
        assert!(unique.contains("list_system_tray_icons"));
        assert!(unique.contains("invoke_system_tray_icon"));
        assert!(unique.contains("get_search_provider_health"));
        assert!(unique.contains("request_everything_setup"));
        assert!(unique.contains("run_quick_command"));
    }

    #[test]
    fn core_event_contracts_are_stable() {
        assert_eq!(
            events::ALL,
            &[
                "audio-panel:open",
                "audio-panel:closed",
                "calendar-panel:open",
                "calendar-panel:closed",
                "command-panel:closed",
                "terminal-panel:open",
                "terminal-panel:closed",
                "process-manager:open",
                "process-manager:closed",
                "search:toggle-centered",
                "search-engine:progress",
                "search-index:refreshed",
                "search-panel:activate",
                "search-panel:closed",
                "search-panel:expand-group",
                "search-panel:interaction",
                "search-panel:key",
                "search-panel:pin-folder",
                "search-panel:query",
                "search-panel:select",
                "search-panel:update",
                "stack-popup:open",
                "stack-terminal:closed",
                "stack-terminal:cwd",
                "stack-terminal:output",
                "stack-pins:updated",
                "task-preview:hide",
                "task-preview:hover-enter",
                "task-preview:update",
                "task:completed",
                "task:output",
                "task:started",
                "taskbar:refresh-launchers",
                "taskbar:refresh-windows",
                "top-bar:pin-menu-action",
                "tray-panel:closed",
                "tray-panel:open",
            ]
        );
    }
}
