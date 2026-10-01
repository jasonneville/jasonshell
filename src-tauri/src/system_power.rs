use serde::Deserialize;
use tauri::WebviewWindow;
use crate::stack_popup::{authorize_stack_command, CallerAuthError, StackCommandAuth};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemPowerActionRequest {
    action: SystemPowerAction,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum SystemPowerAction {
    Sleep,
    Restart,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PowerActionPlan {
    program: &'static str,
    args: Vec<&'static str>,
}

#[tauri::command]
pub fn trigger_system_power_action(window: WebviewWindow, request: SystemPowerActionRequest) -> Result<(), String> {
    authorize_stack_command(&window, StackCommandAuth::AllowedCallers {
        command: "trigger_system_power_action",
        callers: &[crate::contracts::surfaces::SETTINGS_PANEL],
    }).map_err(CallerAuthError::into_string)?;
    match request.action {
        SystemPowerAction::Sleep => trigger_sleep(),
        SystemPowerAction::Restart | SystemPowerAction::Shutdown => {
            let plan = power_action_plan(request.action);
            std::process::Command::new(plan.program)
                .args(plan.args)
                .spawn()
                .map(|_| ())
                .map_err(|error| format!("Failed to trigger system power action: {error}"))
        }
    }
}

fn power_action_plan(action: SystemPowerAction) -> PowerActionPlan {
    match action {
        SystemPowerAction::Sleep => PowerActionPlan {
            program: "",
            args: Vec::new(),
        },
        SystemPowerAction::Restart => PowerActionPlan {
            program: "shutdown.exe",
            args: vec!["/r", "/t", "0"],
        },
        SystemPowerAction::Shutdown => PowerActionPlan {
            program: "shutdown.exe",
            args: vec!["/s", "/t", "0"],
        },
    }
}

#[cfg(target_os = "windows")]
fn trigger_sleep() -> Result<(), String> {
    use windows::Win32::System::Power::SetSuspendState;

    // SAFETY: SetSuspendState takes value parameters only and does not retain pointers.
    if unsafe { SetSuspendState(false, false, false) } {
        Ok(())
    } else {
        Err("Failed to trigger sleep".to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn trigger_sleep() -> Result<(), String> {
    Err("Sleep is only available on Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::{power_action_plan, PowerActionPlan, SystemPowerAction, SystemPowerActionRequest};

    #[test]
    fn power_caller_policy_accepts_settings_only_and_redacts_denial() {
        use crate::stack_popup::{authorize_stack_command_caller, StackCommandAuth};
        let handler = include_str!("system_power.rs").split("pub fn trigger_system_power_action(").nth(1).unwrap().split("fn power_action_plan(").next().unwrap();
        assert!(handler.contains("callers: &[crate::contracts::surfaces::SETTINGS_PANEL]"), "handler must bind exactly settings panel policy");
        let auth = StackCommandAuth::AllowedCallers {
            command: "trigger_system_power_action",
            callers: &[crate::contracts::surfaces::SETTINGS_PANEL],
        };
        assert!(authorize_stack_command_caller(crate::contracts::surfaces::SETTINGS_PANEL, auth).is_ok());
        for denied in [crate::contracts::surfaces::TOP_BAR, crate::contracts::surfaces::COMMAND_PANEL, crate::contracts::surfaces::PROCESS_MANAGER] {
            let error = authorize_stack_command_caller(denied, auth).unwrap_err().into_string();
            assert_eq!(error, "Unauthorized caller for command trigger_system_power_action");
            assert!(!error.contains(denied), "denial must not disclose caller label");
        }
    }

    #[test]
    fn power_action_requires_caller_before_any_native_power_operation() {
        let source = include_str!("system_power.rs");
        let handler = source.split("pub fn trigger_system_power_action(").nth(1).unwrap().split("fn power_action_plan(").next().unwrap();
        assert!(handler.contains("WebviewWindow"), "power action needs a trusted invoking webview label");
        let guard = handler.find("authorize_stack_command(").expect("power action must check caller");
        let effect = handler.find("match request.action").unwrap();
        assert!(guard < effect, "deny before sleep or shutdown spawn");
    }

    #[test]
    fn deserializes_only_known_power_actions() {
        let sleep: SystemPowerActionRequest =
            serde_json::from_str(r#"{"action":"sleep"}"#).unwrap();
        assert_eq!(sleep.action, SystemPowerAction::Sleep);

        assert!(
            serde_json::from_str::<SystemPowerActionRequest>(r#"{"action":"hibernate"}"#).is_err()
        );
        assert!(serde_json::from_str::<SystemPowerActionRequest>(
            r#"{"action":"restart && calc"}"#
        )
        .is_err());
    }

    #[test]
    fn restart_and_shutdown_use_argument_vector_plans() {
        assert_eq!(
            power_action_plan(SystemPowerAction::Restart),
            PowerActionPlan {
                program: "shutdown.exe",
                args: vec!["/r", "/t", "0"]
            }
        );
        assert_eq!(
            power_action_plan(SystemPowerAction::Shutdown),
            PowerActionPlan {
                program: "shutdown.exe",
                args: vec!["/s", "/t", "0"]
            }
        );
    }
}
