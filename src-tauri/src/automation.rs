use serde::{Deserialize, Serialize};

pub const LOCAL_AUTOMATION_OPT_IN_FLAG: &str = "--allow-local-automation";
pub const AUTHENTICATED_FLAG: &str = "--authenticated";
pub const USER_PRESENT_FLAG: &str = "--user-present";
pub const CONFIRM_FLAG: &str = "--confirm";
const FORWARDING_STATUS: &str = "planned-not-wired";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationCliParseRequest {
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct AutomationRequest {
    pub source: AutomationSource,
    pub action: AutomationAction,
    pub boundary: AutomationSecurityBoundary,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AutomationSource {
    LocalCli,
    SingleInstanceForward,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct AutomationAction {
    pub kind: AutomationActionKind,
    pub target: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AutomationActionKind {
    Help,
    ShowSearch,
    ListProviders,
    ActivateWorkspace,
    DeleteWorkspace,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct AutomationSecurityBoundary {
    pub local_automation_enabled: bool,
    pub authenticated: bool,
    pub user_present: bool,
    pub destructive_confirmation: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutomationValidation {
    pub accepted: bool,
    pub action: AutomationActionKind,
    pub security_level: AutomationSecurityLevel,
    pub forwarding_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AutomationSecurityLevel {
    ReadOnly,
    Mutating,
    Destructive,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SingleInstanceForwardingContract {
    pub status: String,
    pub transport: String,
    pub accepts_argv_only: bool,
    pub requires_local_opt_in: bool,
    pub requires_authenticated_destructive_actions: bool,
    pub executes_forwarded_payloads: bool,
    pub arbitrary_plugin_execution_allowed: bool,
}

#[tauri::command]
pub fn parse_automation_cli(
    request: AutomationCliParseRequest,
) -> Result<AutomationRequest, String> {
    parse_cli_args(&request.args)
}

#[tauri::command]
pub fn validate_automation_request(
    request: AutomationRequest,
) -> Result<AutomationValidation, String> {
    validate_request(&request)
}

#[tauri::command]
pub fn get_single_instance_forwarding_contract() -> SingleInstanceForwardingContract {
    forwarding_contract()
}

pub fn parse_cli_args(args: &[String]) -> Result<AutomationRequest, String> {
    if args.is_empty() {
        return Err("CLI command is required".to_string());
    }

    let mut local_automation_enabled = false;
    let mut authenticated = false;
    let mut user_present = false;
    let mut destructive_confirmation = None;
    let mut positional = Vec::new();
    let mut index = 0;

    while index < args.len() {
        let arg = args[index].trim();
        match arg {
            LOCAL_AUTOMATION_OPT_IN_FLAG => local_automation_enabled = true,
            AUTHENTICATED_FLAG => authenticated = true,
            USER_PRESENT_FLAG => user_present = true,
            CONFIRM_FLAG => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err("--confirm requires a value".to_string());
                };
                destructive_confirmation = Some(value.trim().to_string());
            }
            "--help" | "-h" => {
                return Ok(AutomationRequest {
                    source: AutomationSource::LocalCli,
                    action: AutomationAction {
                        kind: AutomationActionKind::Help,
                        target: None,
                    },
                    boundary: AutomationSecurityBoundary {
                        local_automation_enabled,
                        authenticated,
                        user_present: true,
                        destructive_confirmation,
                    },
                });
            }
            value if value.starts_with('-') => {
                return Err(format!("unsupported CLI flag: {value}"));
            }
            value => positional.push(value.to_string()),
        }
        index += 1;
    }

    let action = parse_positional_action(&positional)?;
    Ok(AutomationRequest {
        source: AutomationSource::LocalCli,
        action,
        boundary: AutomationSecurityBoundary {
            local_automation_enabled,
            authenticated,
            user_present,
            destructive_confirmation,
        },
    })
}

pub fn validate_request(request: &AutomationRequest) -> Result<AutomationValidation, String> {
    if !request.boundary.local_automation_enabled {
        return Err("local automation must be explicitly enabled for this request".to_string());
    }
    validate_action_target(&request.action)?;
    let security_level = security_level(&request.action.kind);
    match security_level {
        AutomationSecurityLevel::ReadOnly => {}
        AutomationSecurityLevel::Mutating => {
            if !request.boundary.authenticated && !request.boundary.user_present {
                return Err(
                    "unauthenticated mutating automation requires an explicit user-present boundary"
                        .to_string(),
                );
            }
        }
        AutomationSecurityLevel::Destructive => {
            if !request.boundary.authenticated || !request.boundary.user_present {
                return Err(
                    "destructive automation requires authenticated and user-present boundaries"
                        .to_string(),
                );
            }
            let expected = destructive_confirmation_phrase(&request.action)?;
            if request.boundary.destructive_confirmation.as_deref() != Some(expected.as_str()) {
                return Err(format!(
                    "destructive automation requires confirmation phrase: {expected}"
                ));
            }
        }
    }

    Ok(AutomationValidation {
        accepted: true,
        action: request.action.kind.clone(),
        security_level,
        forwarding_status: FORWARDING_STATUS.to_string(),
    })
}

pub fn forwarding_contract() -> SingleInstanceForwardingContract {
    SingleInstanceForwardingContract {
        status: FORWARDING_STATUS.to_string(),
        transport: "local-single-instance-forwarding-plan".to_string(),
        accepts_argv_only: true,
        requires_local_opt_in: true,
        requires_authenticated_destructive_actions: true,
        executes_forwarded_payloads: false,
        arbitrary_plugin_execution_allowed: false,
    }
}

fn parse_positional_action(positional: &[String]) -> Result<AutomationAction, String> {
    match positional {
        [command] if command == "search" => Ok(action(AutomationActionKind::ShowSearch, None)),
        [command] if command == "providers" => {
            Ok(action(AutomationActionKind::ListProviders, None))
        }
        [command, subcommand] if command == "providers" && subcommand == "list" => {
            Ok(action(AutomationActionKind::ListProviders, None))
        }
        [command, subcommand, id] if command == "workspace" && subcommand == "activate" => {
            Ok(action(AutomationActionKind::ActivateWorkspace, Some(id)))
        }
        [command, subcommand, id] if command == "workspace" && subcommand == "delete" => {
            Ok(action(AutomationActionKind::DeleteWorkspace, Some(id)))
        }
        _ => Err("unsupported CLI command".to_string()),
    }
}

fn action(kind: AutomationActionKind, target: Option<&String>) -> AutomationAction {
    AutomationAction {
        kind,
        target: target.cloned(),
    }
}

fn validate_action_target(action: &AutomationAction) -> Result<(), String> {
    match action.kind {
        AutomationActionKind::Help
        | AutomationActionKind::ShowSearch
        | AutomationActionKind::ListProviders => {
            if action.target.is_some() {
                return Err("read-only automation action must not include a target".to_string());
            }
        }
        AutomationActionKind::ActivateWorkspace | AutomationActionKind::DeleteWorkspace => {
            let Some(target) = action.target.as_deref() else {
                return Err("workspace automation action requires a target id".to_string());
            };
            validate_id("workspace target id", target)?;
        }
    }
    Ok(())
}

fn security_level(kind: &AutomationActionKind) -> AutomationSecurityLevel {
    match kind {
        AutomationActionKind::Help
        | AutomationActionKind::ShowSearch
        | AutomationActionKind::ListProviders => AutomationSecurityLevel::ReadOnly,
        AutomationActionKind::ActivateWorkspace => AutomationSecurityLevel::Mutating,
        AutomationActionKind::DeleteWorkspace => AutomationSecurityLevel::Destructive,
    }
}

fn destructive_confirmation_phrase(action: &AutomationAction) -> Result<String, String> {
    let target = action
        .target
        .as_deref()
        .ok_or_else(|| "destructive automation requires a target id".to_string())?;
    Ok(format!("delete-workspace:{target}"))
}

fn validate_id(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} is required"));
    }
    if value.len() > 64
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(format!(
            "{label} must use only letters, numbers, dash, underscore, or dot"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn cli_parser_requires_explicit_local_automation_opt_in() {
        let request = parse_cli_args(&args(&["providers", "list"])).unwrap();

        let error = validate_request(&request).unwrap_err();

        assert!(error.contains("explicitly enabled"));
    }

    #[test]
    fn cli_parser_accepts_read_only_provider_listing_with_opt_in() {
        let request =
            parse_cli_args(&args(&["providers", "list", LOCAL_AUTOMATION_OPT_IN_FLAG])).unwrap();

        let validation = validate_request(&request).unwrap();

        assert_eq!(validation.security_level, AutomationSecurityLevel::ReadOnly);
        assert_eq!(validation.action, AutomationActionKind::ListProviders);
    }

    #[test]
    fn unauthenticated_mutation_requires_user_present_boundary() {
        let request = parse_cli_args(&args(&[
            "workspace",
            "activate",
            "main",
            LOCAL_AUTOMATION_OPT_IN_FLAG,
        ]))
        .unwrap();

        let error = validate_request(&request).unwrap_err();

        assert!(error.contains("user-present"));
    }

    #[test]
    fn destructive_action_requires_auth_user_presence_and_confirmation() {
        let request = parse_cli_args(&args(&[
            "workspace",
            "delete",
            "main",
            LOCAL_AUTOMATION_OPT_IN_FLAG,
            USER_PRESENT_FLAG,
            AUTHENTICATED_FLAG,
        ]))
        .unwrap();

        let error = validate_request(&request).unwrap_err();

        assert!(error.contains("delete-workspace:main"));

        let confirmed = parse_cli_args(&args(&[
            "workspace",
            "delete",
            "main",
            LOCAL_AUTOMATION_OPT_IN_FLAG,
            USER_PRESENT_FLAG,
            AUTHENTICATED_FLAG,
            CONFIRM_FLAG,
            "delete-workspace:main",
        ]))
        .unwrap();
        let validation = validate_request(&confirmed).unwrap();

        assert_eq!(
            validation.security_level,
            AutomationSecurityLevel::Destructive
        );
    }

    #[test]
    fn cli_payload_cannot_request_arbitrary_executable_execution() {
        let payload = serde_json::json!({
            "source": "localCli",
            "action": { "kind": "listProviders", "executable": "cmd.exe" },
            "boundary": {
                "localAutomationEnabled": true,
                "authenticated": false,
                "userPresent": true,
                "destructiveConfirmation": null
            }
        });

        let error = serde_json::from_value::<AutomationRequest>(payload).unwrap_err();

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn forwarding_contract_is_plan_only_and_never_executes_payloads() {
        let contract = forwarding_contract();

        assert_eq!(contract.status, FORWARDING_STATUS);
        assert!(contract.accepts_argv_only);
        assert!(contract.requires_local_opt_in);
        assert!(!contract.executes_forwarded_payloads);
        assert!(!contract.arbitrary_plugin_execution_allowed);
    }
}
