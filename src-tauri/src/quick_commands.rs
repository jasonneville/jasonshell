use crate::settings::{
    self, validate_quick_command_args, validate_quick_command_entry, QuickCommandEntry,
    QuickCommandMode,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};
use tauri::AppHandle;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunQuickCommandRequest {
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuickCommandSpawnResult {
    pub process_id: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct QuickCommandSpawnPlan {
    executable: String,
    args: Vec<String>,
    cwd: Option<String>,
}

#[tauri::command]
pub fn run_quick_command(
    app_handle: AppHandle,
    request: RunQuickCommandRequest,
) -> Result<QuickCommandSpawnResult, String> {
    validate_run_request(&request)?;
    let settings = settings::load_shell_settings_for_app(&app_handle)?;
    let entry = resolve_quick_command_entry(&settings, &request.id)?;
    let plan = build_spawn_plan(&entry)?;
    let process_id = spawn_quick_command(&plan)?;
    Ok(QuickCommandSpawnResult { process_id })
}

fn validate_run_request(request: &RunQuickCommandRequest) -> Result<(), String> {
    if request.id.trim().is_empty() {
        return Err("quick command id must not be empty".to_string());
    }
    Ok(())
}

fn resolve_quick_command_entry(
    settings: &settings::ShellSettings,
    command_id: &str,
) -> Result<QuickCommandEntry, String> {
    let Some(entry) = settings
        .quick_commands
        .entries
        .iter()
        .find(|entry| entry.id == command_id)
    else {
        return Err(format!("quick command '{}' is not configured", command_id));
    };
    validate_quick_command_entry(entry)
}

fn build_spawn_plan(entry: &QuickCommandEntry) -> Result<QuickCommandSpawnPlan, String> {
    if let Some(cwd) = entry.cwd.as_deref() {
        if !Path::new(cwd).is_dir() {
            return Err(format!(
                "quick command '{}' cwd does not exist: {}",
                entry.id, cwd
            ));
        }
    }
    if !entry.args.is_empty() {
        let _ = validate_quick_command_args(&entry.args, &entry.id)?;
    }

    match entry.mode {
        QuickCommandMode::Direct => Ok(QuickCommandSpawnPlan {
            executable: entry.target_path.clone(),
            args: entry.args.clone(),
            cwd: entry.cwd.clone(),
        }),
        QuickCommandMode::PowershellFile => {
            if !Path::new(&entry.target_path).is_file() {
                return Err(format!(
                    "quick command '{}' powershell file does not exist: {}",
                    entry.id, entry.target_path
                ));
            }
            let mut args = vec![
                "-NoLogo".to_string(),
                "-NoProfile".to_string(),
                "-File".to_string(),
                entry.target_path.clone(),
            ];
            args.extend(entry.args.clone());
            Ok(QuickCommandSpawnPlan {
                executable: "pwsh.exe".to_string(),
                args,
                cwd: entry.cwd.clone(),
            })
        }
        QuickCommandMode::CmdFile => {
            if !Path::new(&entry.target_path).is_file() {
                return Err(format!(
                    "quick command '{}' cmd script does not exist: {}",
                    entry.id, entry.target_path
                ));
            }
            let mut args = vec!["/C".to_string(), entry.target_path.clone()];
            args.extend(entry.args.clone());
            Ok(QuickCommandSpawnPlan {
                executable: "cmd.exe".to_string(),
                args,
                cwd: entry.cwd.clone(),
            })
        }
    }
}

fn spawn_quick_command(plan: &QuickCommandSpawnPlan) -> Result<u32, String> {
    let mut command = Command::new(&plan.executable);
    command
        .args(&plan.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(cwd) = &plan.cwd {
        command.current_dir(cwd);
    }
    command
        .spawn()
        .map(|child| child.id())
        .map_err(|error| format!("failed to run quick command '{}': {error}", plan.executable))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn entry(mode: QuickCommandMode, target_path: String, args: Vec<&str>) -> QuickCommandEntry {
        QuickCommandEntry {
            id: "quick".to_string(),
            label: "Quick".to_string(),
            mode,
            target_path,
            args: args.into_iter().map(|value| value.to_string()).collect(),
            cwd: None,
        }
    }

    #[test]
    fn validates_run_request_requires_id() {
        let error =
            validate_run_request(&RunQuickCommandRequest { id: "".to_string() }).unwrap_err();
        assert!(error.contains("must not be empty"));
    }

    #[test]
    fn builds_direct_mode_spawn_plan_without_shell_wrapping() {
        let plan = build_spawn_plan(&entry(
            QuickCommandMode::Direct,
            "git.exe".to_string(),
            vec!["status", "--short"],
        ))
        .unwrap();
        assert_eq!(plan.executable, "git.exe");
        assert_eq!(plan.args, vec!["status", "--short"]);
    }

    #[test]
    fn builds_powershell_file_spawn_plan_with_explicit_file_mode() {
        let path = env::temp_dir().join("jasonshell-quick-command-test.ps1");
        fs::write(&path, "Write-Output 'ok'").unwrap();
        let plan = build_spawn_plan(&entry(
            QuickCommandMode::PowershellFile,
            path.to_string_lossy().to_string(),
            vec!["arg-a"],
        ))
        .unwrap();
        assert_eq!(plan.executable, "pwsh.exe");
        assert_eq!(
            plan.args[0..4],
            [
                "-NoLogo",
                "-NoProfile",
                "-File",
                path.to_string_lossy().as_ref()
            ]
        );
        assert_eq!(plan.args[4], "arg-a");
    }

    #[test]
    fn builds_cmd_file_spawn_plan_with_c_flag() {
        let path = env::temp_dir().join("jasonshell-quick-command-test.cmd");
        fs::write(&path, "@echo off").unwrap();
        let plan = build_spawn_plan(&entry(
            QuickCommandMode::CmdFile,
            path.to_string_lossy().to_string(),
            vec!["arg-a", "arg-b"],
        ))
        .unwrap();
        assert_eq!(plan.executable, "cmd.exe");
        assert_eq!(plan.args[0], "/C");
        assert_eq!(plan.args[1], path.to_string_lossy().as_ref());
        assert_eq!(plan.args[2], "arg-a");
        assert_eq!(plan.args[3], "arg-b");
    }

    #[test]
    fn rejects_secret_like_args_from_spawn_plan() {
        let error = build_spawn_plan(&entry(
            QuickCommandMode::Direct,
            "git.exe".to_string(),
            vec!["--token", "abc"],
        ))
        .unwrap_err();
        assert!(error.contains("secret-like"));
    }
}
