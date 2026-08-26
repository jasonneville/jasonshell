use crate::contracts;
use tauri::WebviewWindow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StackCommandAuth {
    AllowedCallers {
        command: &'static str,
        callers: &'static [&'static str],
    },
    TerminalSessionTarget {
        command: &'static str,
        callers: &'static [&'static str],
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallerAuthError {
    Unauthorized {
        command: &'static str,
        caller: String,
    },
}

impl CallerAuthError {
    pub(crate) fn into_string(self) -> String {
        match self {
            Self::Unauthorized { command, caller } => {
                let _ = caller;
                format!("Unauthorized caller for command {command}")
            }
        }
    }
}

pub(crate) fn allowed_stack_command_callers(auth: StackCommandAuth) -> &'static [&'static str] {
    match auth {
        StackCommandAuth::AllowedCallers { callers, .. }
        | StackCommandAuth::TerminalSessionTarget { callers, .. } => callers,
    }
}

pub(crate) fn authorize_stack_command(
    window: &WebviewWindow,
    auth: StackCommandAuth,
) -> Result<(), CallerAuthError> {
    let caller = window.label().to_string();
    let command = match auth {
        StackCommandAuth::AllowedCallers { command, callers }
        | StackCommandAuth::TerminalSessionTarget { command, callers } => {
            if callers.iter().any(|label| *label == caller) {
                return Ok(());
            }
            command
        }
    };

    Err(CallerAuthError::Unauthorized { command, caller })
}

// terminal session target auth: caller must match stored session target; never trust request target alone.

pub(crate) const STACK_GUARDED_COMMANDS: &[StackCommandAuth] = &[
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::LIST_PINNED_STACK_FOLDERS,
        callers: &[
            contracts::surfaces::TOP_BAR,
            contracts::surfaces::STACK_POPUP,
        ],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::PIN_STACK_FOLDER,
        callers: &[
            contracts::surfaces::TOP_BAR,
            contracts::surfaces::STACK_POPUP,
            contracts::surfaces::SEARCH_PANEL,
        ],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::UNPIN_STACK_FOLDER,
        callers: &[
            contracts::surfaces::TOP_BAR,
            contracts::surfaces::STACK_POPUP,
        ],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::REORDER_PINNED_STACK_FOLDERS,
        callers: &[contracts::surfaces::TOP_BAR],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::SHOW_STACK_POPUP,
        callers: &[contracts::surfaces::TOP_BAR],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::HIDE_STACK_POPUP,
        callers: &[
            contracts::surfaces::TOP_BAR,
            contracts::surfaces::STACK_POPUP,
        ],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::BEGIN_STACK_POPUP_FOCUS_LOSS_HOLD,
        callers: &[
            contracts::surfaces::TOP_BAR,
            contracts::surfaces::STACK_POPUP,
        ],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::END_STACK_POPUP_FOCUS_LOSS_HOLD,
        callers: &[
            contracts::surfaces::TOP_BAR,
            contracts::surfaces::STACK_POPUP,
        ],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::GET_STACK_POPUP_REQUEST,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::RESIZE_STACK_POPUP,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::READ_STACK_FOLDER,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::SUGGEST_STACK_PATHS,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::RESOLVE_STACK_ITEM_ICONS,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::OPEN_STACK_ITEM,
        callers: &[
            contracts::surfaces::STACK_POPUP,
            contracts::surfaces::TERMINAL_PANEL,
        ],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::OPEN_STACK_ITEM_WITH_PICKER,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::LIST_STACK_OPEN_WITH_CANDIDATES,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::OPEN_STACK_ITEM_WITH_APP,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::RENAME_STACK_ITEM,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::COPY_STACK_ITEMS,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::PREPARE_STACK_FILE_DRAG,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::CUT_STACK_ITEMS,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::PASTE_STACK_ITEMS,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::DELETE_STACK_ITEM,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::NEW_STACK_FOLDER,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::NEW_STACK_TEXT_FILE,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::OPEN_STACK_TERMINAL_HERE,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::REVEAL_STACK_ITEM,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::EXTRACT_STACK_ARCHIVE,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::SHOW_STACK_ITEM_PROPERTIES,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::OPEN_STACK_FOLDER_IN_VSCODE,
        callers: &[
            contracts::surfaces::TOP_BAR,
            contracts::surfaces::STACK_POPUP,
        ],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::GET_STACK_GIT_STATUS,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::OPEN_STACK_GIT_REMOTE_URL,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_ADD_PATHS,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_UNSTAGE_PATHS,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_REVERT_PATHS,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_DIFF,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_STASHES,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_STASH,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_STASH_APPLY,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_STASH_POP,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_STASH_DROP,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_COMMIT,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_LOG,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_TREE,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_BRANCHES,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_FETCH,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_PULL,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_PUSH,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_CHECKOUT_BRANCH,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_CREATE_BRANCH,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STACK_GIT_DELETE_BRANCH,
        callers: &[contracts::surfaces::STACK_POPUP],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::START_PERSISTENT_TERMINAL,
        callers: &[contracts::surfaces::TERMINAL_PANEL],
    },
    StackCommandAuth::TerminalSessionTarget {
        command: contracts::commands::START_STACK_TERMINAL,
        callers: &[
            contracts::surfaces::TERMINAL_PANEL,
            contracts::surfaces::STACK_POPUP,
        ],
    },
    StackCommandAuth::TerminalSessionTarget {
        command: contracts::commands::READ_STACK_TERMINAL,
        callers: &[
            contracts::surfaces::STACK_POPUP,
            contracts::surfaces::TERMINAL_PANEL,
        ],
    },
    StackCommandAuth::TerminalSessionTarget {
        command: contracts::commands::WRITE_STACK_TERMINAL,
        callers: &[
            contracts::surfaces::STACK_POPUP,
            contracts::surfaces::TERMINAL_PANEL,
        ],
    },
    StackCommandAuth::TerminalSessionTarget {
        command: contracts::commands::RESIZE_STACK_TERMINAL,
        callers: &[
            contracts::surfaces::STACK_POPUP,
            contracts::surfaces::TERMINAL_PANEL,
        ],
    },
    StackCommandAuth::TerminalSessionTarget {
        command: contracts::commands::STOP_STACK_TERMINAL,
        callers: &[
            contracts::surfaces::STACK_POPUP,
            contracts::surfaces::TERMINAL_PANEL,
        ],
    },
    StackCommandAuth::TerminalSessionTarget {
        command: contracts::commands::POLL_STACK_TERMINAL_SESSION,
        callers: &[
            contracts::surfaces::STACK_POPUP,
            contracts::surfaces::TERMINAL_PANEL,
        ],
    },
    StackCommandAuth::TerminalSessionTarget {
        command: contracts::commands::LIST_STACK_TERMINALS,
        callers: &[
            contracts::surfaces::STACK_POPUP,
            contracts::surfaces::TERMINAL_PANEL,
        ],
    },
    StackCommandAuth::TerminalSessionTarget {
        command: contracts::commands::RENAME_STACK_TERMINAL,
        callers: &[
            contracts::surfaces::STACK_POPUP,
            contracts::surfaces::TERMINAL_PANEL,
        ],
    },
    StackCommandAuth::AllowedCallers {
        command: contracts::commands::STOP_TERMINAL_PANEL_SESSIONS,
        callers: &[contracts::surfaces::TERMINAL_PANEL],
    },
    StackCommandAuth::TerminalSessionTarget {
        command: contracts::commands::GET_STACK_TERMINAL_CWD,
        callers: &[
            contracts::surfaces::STACK_POPUP,
            contracts::surfaces::TERMINAL_PANEL,
        ],
    },
];
