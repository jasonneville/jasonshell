use serde::Deserialize;

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
pub fn trigger_system_power_action(request: SystemPowerActionRequest) -> Result<(), String> {
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
    fn deserializes_only_known_power_actions() {
        let sleep: SystemPowerActionRequest = serde_json::from_str(r#"{"action":"sleep"}"#).unwrap();
        assert_eq!(sleep.action, SystemPowerAction::Sleep);

        assert!(serde_json::from_str::<SystemPowerActionRequest>(r#"{"action":"hibernate"}"#).is_err());
        assert!(serde_json::from_str::<SystemPowerActionRequest>(r#"{"action":"restart && calc"}"#).is_err());
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
