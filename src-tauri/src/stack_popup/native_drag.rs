use crate::stack_popup::models::StackNativeDragPreparation;
use crate::stack_popup::paths::normalize_existing_path;
use std::path::PathBuf;

pub(crate) fn start_stack_file_drag(
    paths: Vec<String>,
) -> Result<StackNativeDragPreparation, String> {
    let resolved = normalize_drag_paths(paths)?;

    #[cfg(target_os = "windows")]
    start_native_file_drag(&resolved)?;

    #[cfg(not(target_os = "windows"))]
    return Err("Native Explorer file drag is only available on Windows".to_string());

    Ok(StackNativeDragPreparation {
        paths: resolved
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
        effect: "copy".to_string(),
        mechanism: native_drag_mechanism().to_string(),
    })
}

pub(crate) fn normalize_drag_paths(paths: Vec<String>) -> Result<Vec<PathBuf>, String> {
    if paths.is_empty() {
        return Err("Select at least one stack item first".to_string());
    }

    paths
        .iter()
        .map(|path| normalize_existing_path(path).map(PathBuf::from))
        .collect::<Result<Vec<_>, _>>()
}

pub(crate) fn native_drag_mechanism() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "ole-do-drag-drop"
    }

    #[cfg(not(target_os = "windows"))]
    {
        "unsupported"
    }
}

#[cfg(target_os = "windows")]
fn start_native_file_drag(paths: &[PathBuf]) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::Com::IDataObject;
    use windows::Win32::System::Ole::{IDropSource, DROPEFFECT_COPY};
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{ILCreateFromPathW, SHCreateDataObject, SHDoDragDrop};

    let _ole = OleApartment::initialize()?;
    let mut pidls = Vec::<*const ITEMIDLIST>::with_capacity(paths.len());

    for path in paths {
        let encoded = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let pidl = unsafe { ILCreateFromPathW(PCWSTR(encoded.as_ptr())) };
        if pidl.is_null() {
            free_pidls(&pidls);
            return Err(format!(
                "Failed to create a shell drag item for {}",
                path.to_string_lossy()
            ));
        }
        pidls.push(pidl.cast_const());
    }

    let drag_result = (|| unsafe {
        let data_object: IDataObject = SHCreateDataObject(None, Some(&pidls), None::<&IDataObject>)
            .map_err(|error| format!("Failed to create shell drag data: {error}"))?;

        SHDoDragDrop(None, &data_object, None::<&IDropSource>, DROPEFFECT_COPY)
            .map_err(|error| format!("Native Explorer drag failed: {error}"))?;
        Ok(())
    })();

    free_pidls(&pidls);
    drag_result
}

#[cfg(target_os = "windows")]
struct OleApartment;

#[cfg(target_os = "windows")]
impl OleApartment {
    fn initialize() -> Result<Self, String> {
        use windows::Win32::System::Ole::OleInitialize;

        unsafe {
            OleInitialize(None)
                .map_err(|error| format!("Failed to initialize OLE drag: {error}"))?;
        }
        Ok(Self)
    }
}

#[cfg(target_os = "windows")]
impl Drop for OleApartment {
    fn drop(&mut self) {
        use windows::Win32::System::Ole::OleUninitialize;

        unsafe {
            OleUninitialize();
        }
    }
}

#[cfg(target_os = "windows")]
fn free_pidls(pidls: &[*const windows::Win32::UI::Shell::Common::ITEMIDLIST]) {
    use windows::Win32::UI::Shell::ILFree;

    for pidl in pidls {
        unsafe {
            ILFree(Some(*pidl));
        }
    }
}
