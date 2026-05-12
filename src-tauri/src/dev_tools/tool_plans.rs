use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceToolReference {
    pub id: Option<String>,
    pub name: Option<String>,
    pub root_path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandTemplate {
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolLaunchRequest {
    pub workspace: WorkspaceToolReference,
    pub file_path: Option<String>,
    pub file_line: Option<u32>,
    pub template: Option<CommandTemplate>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolLaunchPlan {
    pub executable: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub workspace_id: Option<String>,
    pub workspace_path: String,
    pub target_path: Option<String>,
    pub uses_shell: bool,
}

#[tauri::command]
pub fn build_terminal_launch_plan(request: ToolLaunchRequest) -> Result<ToolLaunchPlan, String> {
    let template = request
        .template
        .clone()
        .unwrap_or_else(default_terminal_template);
    build_launch_plan(&request, &template)
}

#[tauri::command]
pub fn build_editor_launch_plan(request: ToolLaunchRequest) -> Result<ToolLaunchPlan, String> {
    let template = request.template.clone().unwrap_or_else(|| {
        if request.file_path.is_some() {
            default_editor_file_template()
        } else {
            default_editor_workspace_template()
        }
    });
    build_launch_plan(&request, &template)
}

pub fn build_launch_plan(
    request: &ToolLaunchRequest,
    template: &CommandTemplate,
) -> Result<ToolLaunchPlan, String> {
    validate_executable(&template.executable)?;
    let workspace_path = normalize_workspace_path(&request.workspace.root_path)?;
    let target_path = request
        .file_path
        .as_deref()
        .map(|path| normalize_target_path(&workspace_path, path))
        .transpose()?;
    let context = TemplateContext {
        workspace_name: request.workspace.name.as_deref().unwrap_or(""),
        workspace_path: &workspace_path,
        file_path: target_path.as_deref().unwrap_or(&workspace_path),
        file_line: request.file_line.unwrap_or(1),
    };
    let cwd_template = template.cwd.as_deref().unwrap_or("{workspacePath}");
    let cwd = expand_template_part(cwd_template, &context)?;
    let args = template
        .args
        .iter()
        .map(|arg| expand_template_part(arg, &context))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ToolLaunchPlan {
        executable: template.executable.clone(),
        args,
        cwd,
        workspace_id: request.workspace.id.clone(),
        workspace_path,
        target_path,
        uses_shell: false,
    })
}

fn default_terminal_template() -> CommandTemplate {
    CommandTemplate {
        executable: "wt.exe".to_string(),
        args: vec!["-d".to_string(), "{workspacePath}".to_string()],
        cwd: Some("{workspacePath}".to_string()),
    }
}

fn default_editor_workspace_template() -> CommandTemplate {
    CommandTemplate {
        executable: "code".to_string(),
        args: vec!["{workspacePath}".to_string()],
        cwd: Some("{workspacePath}".to_string()),
    }
}

fn default_editor_file_template() -> CommandTemplate {
    CommandTemplate {
        executable: "code".to_string(),
        args: vec!["--goto".to_string(), "{filePath}:{fileLine}".to_string()],
        cwd: Some("{workspacePath}".to_string()),
    }
}

struct TemplateContext<'a> {
    workspace_name: &'a str,
    workspace_path: &'a str,
    file_path: &'a str,
    file_line: u32,
}

fn expand_template_part(part: &str, context: &TemplateContext<'_>) -> Result<String, String> {
    reject_control_chars(part, "command template")?;
    let mut expanded = part.to_string();
    for (token, value) in [
        ("{workspaceName}", context.workspace_name.to_string()),
        ("{workspacePath}", context.workspace_path.to_string()),
        ("{filePath}", context.file_path.to_string()),
        ("{fileLine}", context.file_line.to_string()),
    ] {
        expanded = expanded.replace(token, &value);
    }
    if expanded.contains('{') || expanded.contains('}') {
        return Err(format!("unsupported command template token in '{part}'"));
    }
    reject_control_chars(&expanded, "expanded command argument")?;
    Ok(expanded)
}

fn validate_executable(executable: &str) -> Result<(), String> {
    let trimmed = executable.trim();
    if trimmed.is_empty() {
        return Err("tool executable must not be empty".to_string());
    }
    if trimmed != executable {
        return Err("tool executable must not contain leading or trailing whitespace".to_string());
    }
    if executable.contains('{') || executable.contains('}') {
        return Err("tool executable must be a literal program path, not a template".to_string());
    }
    if executable.chars().any(|ch| {
        matches!(
            ch,
            '"' | '\'' | '&' | '|' | ';' | '<' | '>' | '\n' | '\r' | '\0'
        )
    }) {
        return Err("tool executable must not contain shell metacharacters".to_string());
    }
    Ok(())
}

fn reject_control_chars(value: &str, label: &str) -> Result<(), String> {
    if value.chars().any(|ch| matches!(ch, '\n' | '\r' | '\0')) {
        return Err(format!("{label} must not contain control characters"));
    }
    Ok(())
}

fn normalize_workspace_path(path: &str) -> Result<String, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("workspace path must not be empty".to_string());
    }
    reject_control_chars(path, "workspace path")?;
    reject_parent_components(Path::new(path), "workspace path")?;
    Ok(path.to_string())
}

fn normalize_target_path(workspace_path: &str, file_path: &str) -> Result<String, String> {
    let file_path = file_path.trim();
    if file_path.is_empty() {
        return Err("target file path must not be empty".to_string());
    }
    reject_control_chars(file_path, "target file path")?;
    let path = PathBuf::from(file_path);
    reject_parent_components(&path, "target file path")?;
    let workspace = Path::new(workspace_path);
    if path.is_absolute() && !path_starts_with_case_insensitive(&path, workspace) {
        return Err("target file path must stay within the workspace".to_string());
    }
    Ok(file_path.to_string())
}

fn reject_parent_components(path: &Path, label: &str) -> Result<(), String> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!(
            "{label} must not contain parent-directory traversal"
        ));
    }
    Ok(())
}

fn path_starts_with_case_insensitive(path: &Path, base: &Path) -> bool {
    let path = path
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    let mut base = base
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    while base.ends_with('\\') {
        base.pop();
    }
    path == base || path.starts_with(&format!("{base}\\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(root_path: &str) -> WorkspaceToolReference {
        WorkspaceToolReference {
            id: Some("workspace-a".to_string()),
            name: Some("JasonShell".to_string()),
            root_path: root_path.to_string(),
        }
    }

    #[test]
    fn expands_terminal_plan_as_argv_without_shell() {
        let request = ToolLaunchRequest {
            workspace: workspace(r"C:\dev\jasonshell"),
            file_path: None,
            file_line: None,
            template: None,
        };

        let plan = build_terminal_launch_plan(request).unwrap();

        assert_eq!(plan.executable, "wt.exe");
        assert_eq!(plan.args, vec!["-d", r"C:\dev\jasonshell"]);
        assert_eq!(plan.cwd, r"C:\dev\jasonshell");
        assert!(!plan.uses_shell);
    }

    #[test]
    fn expands_editor_file_plan_as_single_goto_argument() {
        let request = ToolLaunchRequest {
            workspace: workspace(r"C:\dev\jasonshell"),
            file_path: Some(r"C:\dev\jasonshell\src\main.ts".to_string()),
            file_line: Some(27),
            template: None,
        };

        let plan = build_editor_launch_plan(request).unwrap();

        assert_eq!(plan.executable, "code");
        assert_eq!(
            plan.args,
            vec![
                "--goto".to_string(),
                r"C:\dev\jasonshell\src\main.ts:27".to_string()
            ]
        );
        assert!(!plan.uses_shell);
    }

    #[test]
    fn preserves_shell_metacharacters_in_paths_as_literal_args() {
        let request = ToolLaunchRequest {
            workspace: workspace(r"C:\dev\jasonshell & whoami"),
            file_path: None,
            file_line: None,
            template: Some(CommandTemplate {
                executable: "wt.exe".to_string(),
                args: vec!["-d".to_string(), "{workspacePath}".to_string()],
                cwd: Some("{workspacePath}".to_string()),
            }),
        };

        let plan = build_terminal_launch_plan(request).unwrap();

        assert_eq!(plan.args, vec!["-d", r"C:\dev\jasonshell & whoami"]);
        assert!(!plan.uses_shell);
    }

    #[test]
    fn rejects_template_executables_and_path_traversal() {
        let executable_error = build_terminal_launch_plan(ToolLaunchRequest {
            workspace: workspace(r"C:\dev\jasonshell"),
            file_path: None,
            file_line: None,
            template: Some(CommandTemplate {
                executable: "{workspacePath}\\tool.exe".to_string(),
                args: Vec::new(),
                cwd: None,
            }),
        })
        .unwrap_err();
        assert!(executable_error.contains("literal program path"));

        let traversal_error = build_editor_launch_plan(ToolLaunchRequest {
            workspace: workspace(r"C:\dev\jasonshell"),
            file_path: Some(r"C:\dev\jasonshell\..\secret.txt".to_string()),
            file_line: None,
            template: None,
        })
        .unwrap_err();
        assert!(traversal_error.contains("parent-directory traversal"));
    }
}
