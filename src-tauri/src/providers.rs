use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

const MAX_PROVIDER_ID_LEN: usize = 64;
const DEFAULT_MAX_RESULTS: u16 = 25;
const DEFAULT_TIMEOUT_MS: u16 = 150;
const MAX_PROVIDER_RESULTS: u16 = 100;
const MAX_PROVIDER_TIMEOUT_MS: u16 = 500;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ProviderRegistryConfig {
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ProviderConfig {
    pub id: String,
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub budget: ProviderBudget,
    #[serde(default)]
    pub config: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderType {
    WorkspaceFiles,
    GitChanges,
    TaskHistory,
    Commands,
    Settings,
    Processes,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ProviderBudget {
    pub max_results: u16,
    pub timeout_ms: u16,
}

impl Default for ProviderBudget {
    fn default() -> Self {
        Self {
            max_results: DEFAULT_MAX_RESULTS,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRegistry {
    pub providers: Vec<ResolvedProvider>,
    pub total_max_results: u16,
    pub max_timeout_ms: u16,
    pub arbitrary_plugin_execution_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedProvider {
    pub id: String,
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    pub disabled: bool,
    pub budget: ProviderBudget,
}

#[tauri::command]
pub fn resolve_provider_registry(
    config: ProviderRegistryConfig,
) -> Result<ProviderRegistry, String> {
    resolve_registry(config)
}

pub fn resolve_registry(config: ProviderRegistryConfig) -> Result<ProviderRegistry, String> {
    let mut seen = HashSet::new();
    let mut providers = Vec::with_capacity(config.providers.len());
    let mut total_max_results = 0_u16;
    let mut max_timeout_ms = 0_u16;

    for provider in config.providers {
        let provider = normalize_provider(provider)?;
        if !seen.insert(provider.id.to_ascii_lowercase()) {
            return Err(format!("provider id must be unique: {}", provider.id));
        }
        if !provider.disabled {
            total_max_results = total_max_results.saturating_add(provider.budget.max_results);
            max_timeout_ms = max_timeout_ms.max(provider.budget.timeout_ms);
        }
        providers.push(ResolvedProvider {
            id: provider.id,
            provider_type: provider.provider_type,
            disabled: provider.disabled,
            budget: provider.budget,
        });
    }

    Ok(ProviderRegistry {
        providers,
        total_max_results,
        max_timeout_ms,
        arbitrary_plugin_execution_allowed: false,
    })
}

fn normalize_provider(mut provider: ProviderConfig) -> Result<ProviderConfig, String> {
    provider.id = provider.id.trim().to_string();
    validate_provider_id(&provider.id)?;
    validate_budget(&provider.budget)?;
    reject_secret_like_config(
        &provider.config,
        &["providers".to_string(), provider.id.clone()],
    )?;
    reject_executable_provider_config(&provider.config)?;
    Ok(provider)
}

fn validate_provider_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("provider id is required".to_string());
    }
    if id.len() > MAX_PROVIDER_ID_LEN
        || !id.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
    {
        return Err(
            "provider id must use only letters, numbers, dash, underscore, or dot".to_string(),
        );
    }
    Ok(())
}

fn validate_budget(budget: &ProviderBudget) -> Result<(), String> {
    if budget.max_results == 0 || budget.max_results > MAX_PROVIDER_RESULTS {
        return Err(format!(
            "provider maxResults must be between 1 and {MAX_PROVIDER_RESULTS}"
        ));
    }
    if budget.timeout_ms == 0 || budget.timeout_ms > MAX_PROVIDER_TIMEOUT_MS {
        return Err(format!(
            "provider timeoutMs must be between 1 and {MAX_PROVIDER_TIMEOUT_MS}"
        ));
    }
    Ok(())
}

fn reject_secret_like_config(value: &Value, path: &[String]) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let mut next_path = path.to_vec();
                next_path.push(key.clone());
                if is_secret_like_name(key) {
                    return Err(format!(
                        "provider config must not store secret-like key: {}",
                        next_path.join(".")
                    ));
                }
                reject_secret_like_config(child, &next_path)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let mut next_path = path.to_vec();
                next_path.push(index.to_string());
                reject_secret_like_config(child, &next_path)?;
            }
            Ok(())
        }
        Value::String(value) => {
            if looks_secret_like_value(value) {
                return Err(format!(
                    "provider config must not store secret-like value at {}",
                    path.join(".")
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn reject_executable_provider_config(value: &Value) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let normalized = normalize_scan(key);
                if matches!(
                    normalized.as_str(),
                    "executable" | "command" | "script" | "pluginpath" | "dllpath" | "entrypoint"
                ) {
                    return Err(format!(
                        "provider config must not declare executable/plugin loading key: {key}"
                    ));
                }
                reject_executable_provider_config(child)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            for item in items {
                reject_executable_provider_config(item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn is_secret_like_name(value: &str) -> bool {
    let value = normalize_scan(value);
    [
        "token",
        "secret",
        "password",
        "credential",
        "apikey",
        "authorization",
        "cookie",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}

fn normalize_scan(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn looks_secret_like_value(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("bearer ")
        || value.contains("ghp_")
        || value.contains("gho_")
        || value.contains("github_pat_")
        || value.contains("xoxb-")
        || value.contains("sk-")
        || value.contains("akia")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn provider(id: &str, provider_type: ProviderType) -> ProviderConfig {
        ProviderConfig {
            id: id.to_string(),
            provider_type,
            disabled: false,
            budget: ProviderBudget::default(),
            config: json!({ "scope": "active-workspace" }),
        }
    }

    #[test]
    fn resolves_config_driven_provider_contract_with_bounded_budgets() {
        let registry = resolve_registry(ProviderRegistryConfig {
            providers: vec![
                ProviderConfig {
                    budget: ProviderBudget {
                        max_results: 10,
                        timeout_ms: 75,
                    },
                    ..provider("workspace-files", ProviderType::WorkspaceFiles)
                },
                ProviderConfig {
                    disabled: true,
                    budget: ProviderBudget {
                        max_results: 100,
                        timeout_ms: 500,
                    },
                    ..provider("processes", ProviderType::Processes)
                },
            ],
        })
        .unwrap();

        assert_eq!(registry.providers.len(), 2);
        assert_eq!(registry.total_max_results, 10);
        assert_eq!(registry.max_timeout_ms, 75);
        assert!(!registry.arbitrary_plugin_execution_allowed);
    }

    #[test]
    fn rejects_duplicate_provider_ids_and_over_budget_providers() {
        let duplicate = resolve_registry(ProviderRegistryConfig {
            providers: vec![
                provider("commands", ProviderType::Commands),
                provider("Commands", ProviderType::Commands),
            ],
        })
        .unwrap_err();
        assert!(duplicate.contains("unique"));

        let over_budget = resolve_registry(ProviderRegistryConfig {
            providers: vec![ProviderConfig {
                budget: ProviderBudget {
                    max_results: MAX_PROVIDER_RESULTS + 1,
                    timeout_ms: 50,
                },
                ..provider("git", ProviderType::GitChanges)
            }],
        })
        .unwrap_err();
        assert!(over_budget.contains("maxResults"));
    }

    #[test]
    fn rejects_secret_like_provider_config_keys_and_values() {
        let key_error = resolve_registry(ProviderRegistryConfig {
            providers: vec![ProviderConfig {
                config: json!({ "apiToken": "abc" }),
                ..provider("settings", ProviderType::Settings)
            }],
        })
        .unwrap_err();
        assert!(key_error.contains("secret-like key"));

        let value_error = resolve_registry(ProviderRegistryConfig {
            providers: vec![ProviderConfig {
                config: json!({ "header": "Bearer abc.def" }),
                ..provider("settings", ProviderType::Settings)
            }],
        })
        .unwrap_err();
        assert!(value_error.contains("secret-like value"));
    }

    #[test]
    fn rejects_arbitrary_executable_or_plugin_provider_config() {
        let executable_error = resolve_registry(ProviderRegistryConfig {
            providers: vec![ProviderConfig {
                config: json!({ "executable": "cmd.exe" }),
                ..provider("external", ProviderType::Commands)
            }],
        })
        .unwrap_err();

        assert!(executable_error.contains("executable/plugin"));

        let payload = json!({
            "providers": [{
                "id": "bad",
                "type": "external-executable",
                "disabled": false,
                "budget": { "maxResults": 10, "timeoutMs": 50 }
            }]
        });
        let parse_error = serde_json::from_value::<ProviderRegistryConfig>(payload).unwrap_err();

        assert!(parse_error.to_string().contains("unknown variant"));
    }
}
