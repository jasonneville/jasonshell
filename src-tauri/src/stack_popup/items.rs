use crate::stack_popup::models::StackItem;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub(crate) fn stack_item_from_path(path: PathBuf) -> Result<StackItem, String> {
    stack_item_from_path_with_icon_mode(path, true)
}

pub(crate) fn stack_item_metadata_from_path(path: PathBuf) -> Result<StackItem, String> {
    stack_item_from_path_with_icon_mode(path, false)
}

fn stack_item_from_path_with_icon_mode(
    path: PathBuf,
    include_icon_data: bool,
) -> Result<StackItem, String> {
    let link_metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("Failed to inspect stack item: {error}"))?;
    let is_symlink = link_metadata.file_type().is_symlink();
    let is_reparse_point = metadata_is_reparse_point(&link_metadata);
    let target_metadata = if is_symlink || is_reparse_point {
        fs::metadata(&path).ok()
    } else {
        None
    };
    let metadata = target_metadata.as_ref().unwrap_or(&link_metadata);
    let is_dir = metadata.is_dir();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    let modified_at = metadata_modified_epoch_millis(&link_metadata);

    Ok(StackItem {
        path: path.to_string_lossy().into_owned(),
        type_label: type_label(&path, is_dir, is_symlink, is_reparse_point),
        icon_data_url: include_icon_data
            .then(|| stack_item_icon_data_url(&path))
            .flatten(),
        size_bytes: (!is_dir).then_some(metadata.len()),
        modified_at,
        is_hidden: metadata_is_hidden(&link_metadata, &name),
        is_readonly: metadata.permissions().readonly(),
        is_system: metadata_is_system(&link_metadata),
        is_symlink,
        is_reparse_point,
        kind: if is_dir { "folder" } else { "file" }.to_string(),
        name,
    })
}

#[cfg(target_os = "windows")]
fn stack_item_icon_data_url(path: &Path) -> Option<String> {
    crate::task_windows::shell_file_icon_data_url(path).ok()
}

#[cfg(not(target_os = "windows"))]
fn stack_item_icon_data_url(_path: &Path) -> Option<String> {
    None
}

fn type_label(path: &Path, is_dir: bool, is_symlink: bool, is_reparse_point: bool) -> String {
    if is_symlink {
        return if is_dir {
            "Folder Symlink".to_string()
        } else {
            "File Symlink".to_string()
        };
    }
    if is_reparse_point {
        return if is_dir {
            "Reparse Folder".to_string()
        } else {
            "Reparse File".to_string()
        };
    }
    if is_dir {
        return "Folder".to_string();
    }
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| format!("{} File", extension.to_uppercase()))
        .unwrap_or_else(|| "File".to_string())
}

pub(crate) fn metadata_modified_epoch_millis(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
}

#[cfg(windows)]
pub(crate) fn metadata_is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT_BIT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT_BIT != 0
}

#[cfg(not(windows))]
pub(crate) fn metadata_is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.')
}

pub(crate) fn stack_file_attributes_from_bits(bits: u32) -> (bool, bool, bool) {
    const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x0000_0004;
    (
        bits & FILE_ATTRIBUTE_HIDDEN != 0,
        bits & FILE_ATTRIBUTE_READONLY != 0,
        bits & FILE_ATTRIBUTE_SYSTEM != 0,
    )
}

#[cfg(windows)]
fn metadata_file_attributes(metadata: &fs::Metadata) -> u32 {
    use std::os::windows::fs::MetadataExt;

    metadata.file_attributes()
}

#[cfg(windows)]
fn metadata_is_hidden(metadata: &fs::Metadata, name: &str) -> bool {
    stack_file_attributes_from_bits(metadata_file_attributes(metadata)).0 || is_hidden_name(name)
}

#[cfg(not(windows))]
fn metadata_is_hidden(_metadata: &fs::Metadata, name: &str) -> bool {
    is_hidden_name(name)
}

#[cfg(windows)]
fn metadata_is_system(metadata: &fs::Metadata) -> bool {
    stack_file_attributes_from_bits(metadata_file_attributes(metadata)).2
}

#[cfg(not(windows))]
fn metadata_is_system(_metadata: &fs::Metadata) -> bool {
    false
}
