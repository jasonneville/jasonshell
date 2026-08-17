use crate::stack_popup::models::StackOpenWithCandidate;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OpenWithCandidateSpec {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
    pub(crate) executables: &'static [&'static str],
    pub(crate) source: &'static str,
}

const NOTEPAD_SPEC: OpenWithCandidateSpec = OpenWithCandidateSpec {
    id: "notepad",
    label: "Notepad",
    executables: &["notepad.exe"],
    source: "windows",
};
const NOTEPAD_PLUS_PLUS_SPEC: OpenWithCandidateSpec = OpenWithCandidateSpec {
    id: "notepad-plus-plus",
    label: "Notepad++",
    executables: &[
        r"%ProgramFiles%\Notepad++\notepad++.exe",
        r"%ProgramFiles(x86)%\Notepad++\notepad++.exe",
        "notepad++.exe",
    ],
    source: "common-editor",
};
const VSCODE_SPEC: OpenWithCandidateSpec = OpenWithCandidateSpec {
    id: "vscode",
    label: "Visual Studio Code",
    executables: &[
        r"%LocalAppData%\Programs\Microsoft VS Code\Code.exe",
        r"%ProgramFiles%\Microsoft VS Code\Code.exe",
        r"C:\Program Files\Microsoft VS Code\Code.exe",
        r"C:\Program Files (x86)\Microsoft VS Code\Code.exe",
    ],
    source: "common-editor",
};
const PAINT_SPEC: OpenWithCandidateSpec = OpenWithCandidateSpec {
    id: "paint",
    label: "Paint",
    executables: &["mspaint.exe"],
    source: "windows",
};

const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "log", "md", "markdown", "rst", "csv", "tsv", "json", "jsonc", "xml", "yaml", "yml",
    "toml", "ini", "cfg", "conf", "env", "ps1", "psm1", "bat", "cmd", "sh", "bash", "rs", "js",
    "jsx", "ts", "tsx", "svelte", "css", "scss", "html", "htm", "py", "go", "java", "cs", "cpp",
    "c", "h", "hpp", "sql", "lock",
];
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"];

pub(crate) fn open_with_candidates_for_path(
    path: &Path,
) -> Result<Vec<StackOpenWithCandidate>, String> {
    let extension = extension_lower(path);
    open_with_candidates_for_extension_with_resolver(extension.as_deref(), resolve_executable)
}

pub(crate) fn open_with_app(path: &Path, app_id: &str) -> Result<(), String> {
    let candidates = open_with_candidates_for_path(path)?;
    let candidate = candidates
        .into_iter()
        .find(|candidate| candidate.id == app_id)
        .ok_or_else(|| "Selected Open With application is unavailable".to_string())?;

    Command::new(&candidate.executable)
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to launch {}: {error}", candidate.label))
}

pub(crate) fn open_with_candidates_for_extension_with_resolver<F>(
    extension: Option<&str>,
    resolver: F,
) -> Result<Vec<StackOpenWithCandidate>, String>
where
    F: Fn(&str) -> Option<PathBuf>,
{
    let specs = candidate_specs_for_extension(extension);
    let mut seen_ids = HashSet::new();
    let mut candidates = Vec::new();

    for spec in specs {
        if !seen_ids.insert(spec.id) {
            continue;
        }
        let Some(executable) = spec.executables.iter().find_map(|path| resolver(path)) else {
            continue;
        };
        candidates.push(StackOpenWithCandidate {
            id: spec.id.to_string(),
            label: spec.label.to_string(),
            executable: executable.to_string_lossy().into_owned(),
            source: spec.source.to_string(),
        });
    }

    Ok(candidates)
}

pub(crate) fn candidate_specs_for_extension(extension: Option<&str>) -> Vec<OpenWithCandidateSpec> {
    let extension = extension.unwrap_or_default().trim_start_matches('.');
    if IMAGE_EXTENSIONS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(extension))
    {
        return vec![PAINT_SPEC, VSCODE_SPEC];
    }

    if TEXT_EXTENSIONS
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(extension))
    {
        return vec![NOTEPAD_SPEC, NOTEPAD_PLUS_PLUS_SPEC, VSCODE_SPEC];
    }

    vec![NOTEPAD_SPEC, VSCODE_SPEC]
}

fn extension_lower(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}

fn resolve_executable(candidate: &str) -> Option<PathBuf> {
    let expanded = expand_environment(candidate);
    let path = PathBuf::from(&expanded);
    if path.is_absolute() {
        return path.exists().then(|| std::fs::canonicalize(&path).ok()).flatten();
    }

    windows_dir_candidate(&expanded)
}

fn windows_dir_candidate(executable: &str) -> Option<PathBuf> {
    let system_root = std::env::var_os("SystemRoot")?;
    let path = PathBuf::from(system_root).join("System32").join(executable);
    path.exists().then(|| std::fs::canonicalize(&path).ok()).flatten()
}

fn expand_environment(candidate: &str) -> String {
    let mut expanded = candidate.to_string();
    for (name, value) in [
        ("ProgramFiles", std::env::var_os("ProgramFiles")),
        ("ProgramFiles(x86)", std::env::var_os("ProgramFiles(x86)")),
        ("LocalAppData", std::env::var_os("LocalAppData")),
    ] {
        if let Some(value) = value {
            expanded = expanded.replace(&format!("%{name}%"), value.to_string_lossy().as_ref());
        }
    }
    expanded
}
