use crate::stack_popup::items::stack_item_from_path;
use crate::stack_popup::models::{StackFolderPage, StackFolderWarning};
use std::fs;
use std::path::PathBuf;

pub(crate) const DEFAULT_PAGE_LIMIT: usize = 80;

pub(crate) fn read_stack_folder_page(
    path: &str,
    offset: usize,
    limit: usize,
) -> Result<StackFolderPage, String> {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for entry in
        fs::read_dir(path).map_err(|error| format!("Failed to read stack folder: {error}"))?
    {
        match entry {
            Ok(entry) => match stack_folder_entry_summary(entry) {
                Ok(summary) => entries.push(summary),
                Err((path, message)) => warnings.push(stack_folder_warning(path, message)),
            },
            Err(error) => warnings.push(stack_folder_warning(
                None,
                format!("Failed to read stack folder entry: {error}"),
            )),
        }
    }
    entries.sort_by(|a, b| {
        folder_sort_rank(a.is_dir)
            .cmp(&folder_sort_rank(b.is_dir))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let total = entries.len();
    let limit = limit.max(1);
    let page_entries = entries
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    let page_len = page_entries.len();
    let mut items = Vec::with_capacity(page_len);
    for entry in page_entries {
        match stack_item_from_path(entry.path.clone()) {
            Ok(item) => items.push(item),
            Err(message) => warnings.push(stack_folder_warning(Some(entry.path), message)),
        }
    }

    Ok(StackFolderPage {
        path: path.to_string(),
        has_more: offset + page_len < total,
        items,
        limit: page_len,
        offset,
        total,
        warnings,
    })
}

#[derive(Debug)]
struct StackFolderEntrySummary {
    path: PathBuf,
    name: String,
    is_dir: bool,
}

fn stack_folder_entry_summary(
    entry: fs::DirEntry,
) -> Result<StackFolderEntrySummary, (Option<PathBuf>, String)> {
    let path = entry.path();
    let file_type = entry.file_type().map_err(|error| {
        (
            Some(path.clone()),
            format!("Failed to inspect stack item: {error}"),
        )
    })?;
    let is_dir = if file_type.is_dir() {
        true
    } else if file_type.is_symlink() {
        fs::metadata(&path)
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false)
    } else {
        false
    };
    let name = entry
        .file_name()
        .to_str()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    Ok(StackFolderEntrySummary { path, name, is_dir })
}

fn folder_sort_rank(is_dir: bool) -> u8 {
    if is_dir {
        0
    } else {
        1
    }
}

pub(crate) fn stack_folder_warning(path: Option<PathBuf>, message: String) -> StackFolderWarning {
    StackFolderWarning {
        path: path.map(|path| path.to_string_lossy().into_owned()),
        message,
    }
}
