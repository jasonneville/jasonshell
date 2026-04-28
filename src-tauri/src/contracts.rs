#![allow(dead_code)]

pub mod surfaces {
    pub const TOP_BAR: &str = "top-bar";
    pub const BOTTOM_BAR: &str = "bottom-bar";
    pub const TASK_PREVIEW: &str = "task-preview";
    pub const SEARCH_PANEL: &str = "search-panel";
    pub const STACK_POPUP: &str = "stack-popup";
    pub const PROCESS_MANAGER: &str = "process-manager";
    pub const CONTROL_PLANE: &str = "control-plane";

    pub const ALL: &[&str] = &[
        TOP_BAR,
        BOTTOM_BAR,
        TASK_PREVIEW,
        SEARCH_PANEL,
        STACK_POPUP,
        PROCESS_MANAGER,
        CONTROL_PLANE,
    ];
}

pub mod commands {
    pub const LIST_PINNED_TASKBAR_APPS: &str = "list_pinned_taskbar_apps";
    pub const LAUNCH_PINNED_TASKBAR_APP: &str = "launch_pinned_taskbar_app";
    pub const LIST_OPEN_TASK_WINDOWS: &str = "list_open_task_windows";
    pub const ACTIVATE_TASK_WINDOW: &str = "activate_task_window";
    pub const MAXIMIZE_TASK_WINDOW: &str = "maximize_task_window";
    pub const SHOW_TASK_WINDOW_PREVIEW: &str = "show_task_window_preview";
    pub const HIDE_TASK_WINDOW_PREVIEW: &str = "hide_task_window_preview";
    pub const SHOW_TASK_WINDOW_CONTEXT_MENU: &str = "show_task_window_context_menu";
    pub const SHOW_LAUNCHER_CONTEXT_MENU: &str = "show_launcher_context_menu";
    pub const SHOW_TOP_BAR_PIN_CONTEXT_MENU: &str = "show_top_bar_pin_context_menu";
    pub const SHOW_SEARCH_PANEL: &str = "show_search_panel";
    pub const HIDE_SEARCH_PANEL: &str = "hide_search_panel";
    pub const PUBLISH_SEARCH_PANEL: &str = "publish_search_panel";
    pub const GET_SEARCH_PANEL_PAYLOAD: &str = "get_search_panel_payload";
    pub const SHOW_PROCESS_MANAGER: &str = "show_process_manager";
    pub const HIDE_PROCESS_MANAGER: &str = "hide_process_manager";
    pub const SHOW_CONTROL_PLANE: &str = "show_control_plane";
    pub const HIDE_CONTROL_PLANE: &str = "hide_control_plane";
    pub const LIST_PROCESSES: &str = "list_processes";
    pub const KILL_PROCESS: &str = "kill_process";
    pub const SEARCH_SYSTEM: &str = "search_system";
    pub const OPEN_SHELL_PATH: &str = "open_shell_path";
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
    pub const OPEN_STACK_ITEM: &str = "open_stack_item";
    pub const OPEN_STACK_ITEM_WITH_PICKER: &str = "open_stack_item_with_picker";
    pub const RENAME_STACK_ITEM: &str = "rename_stack_item";
    pub const COPY_STACK_ITEMS: &str = "copy_stack_items";
    pub const CUT_STACK_ITEMS: &str = "cut_stack_items";
    pub const PASTE_STACK_ITEMS: &str = "paste_stack_items";
    pub const DELETE_STACK_ITEM: &str = "delete_stack_item";
    pub const NEW_STACK_FOLDER: &str = "new_stack_folder";
    pub const REVEAL_STACK_ITEM: &str = "reveal_stack_item";
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
        ACTIVATE_TASK_WINDOW,
        MAXIMIZE_TASK_WINDOW,
        SHOW_TASK_WINDOW_PREVIEW,
        HIDE_TASK_WINDOW_PREVIEW,
        SHOW_TASK_WINDOW_CONTEXT_MENU,
        SHOW_LAUNCHER_CONTEXT_MENU,
        SHOW_TOP_BAR_PIN_CONTEXT_MENU,
        SHOW_SEARCH_PANEL,
        HIDE_SEARCH_PANEL,
        PUBLISH_SEARCH_PANEL,
        GET_SEARCH_PANEL_PAYLOAD,
        SHOW_PROCESS_MANAGER,
        HIDE_PROCESS_MANAGER,
        SHOW_CONTROL_PLANE,
        HIDE_CONTROL_PLANE,
        LIST_PROCESSES,
        KILL_PROCESS,
        SEARCH_SYSTEM,
        OPEN_SHELL_PATH,
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
        OPEN_STACK_ITEM,
        OPEN_STACK_ITEM_WITH_PICKER,
        RENAME_STACK_ITEM,
        COPY_STACK_ITEMS,
        CUT_STACK_ITEMS,
        PASTE_STACK_ITEMS,
        DELETE_STACK_ITEM,
        NEW_STACK_FOLDER,
        REVEAL_STACK_ITEM,
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
    pub const PROCESS_MANAGER_CLOSED: &str = "process-manager:closed";
    pub const SEARCH_PANEL_INTERACTION: &str = "search-panel:interaction";
    pub const SEARCH_PANEL_CLOSED: &str = "search-panel:closed";
    pub const SEARCH_INDEX_REFRESHED: &str = "search-index:refreshed";
    pub const STACK_PINS_UPDATED: &str = "stack-pins:updated";
    pub const TASK_STARTED: &str = "task:started";
    pub const TASK_OUTPUT: &str = "task:output";
    pub const TASK_COMPLETED: &str = "task:completed";

    pub const ALL: &[&str] = &[
        PROCESS_MANAGER_CLOSED,
        SEARCH_PANEL_INTERACTION,
        SEARCH_PANEL_CLOSED,
        SEARCH_INDEX_REFRESHED,
        STACK_PINS_UPDATED,
        TASK_STARTED,
        TASK_OUTPUT,
        TASK_COMPLETED,
    ];
}

#[cfg(test)]
mod tests {
    use super::{commands, events, surfaces};
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
            ]
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
    }

    #[test]
    fn core_event_contracts_are_stable() {
        assert_eq!(
            events::ALL,
            &[
                "process-manager:closed",
                "search-panel:interaction",
                "search-panel:closed",
                "search-index:refreshed",
                "stack-pins:updated",
                "task:started",
                "task:output",
                "task:completed",
            ]
        );
    }
}
