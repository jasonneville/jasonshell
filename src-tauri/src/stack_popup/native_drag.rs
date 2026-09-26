use crate::stack_popup::paths::resolve_stack_path_candidate;
use std::path::PathBuf;

pub(crate) struct NativeDragResult {
    pub(crate) status: &'static str,
    pub(crate) effect: &'static str,
    pub(crate) stage: Option<&'static str>,
    pub(crate) message: Option<String>,
}

impl NativeDragResult {
    pub(crate) fn new(
        status: &'static str,
        effect: &'static str,
        stage: Option<&'static str>,
        message: Option<String>,
    ) -> Self {
        Self {
            status,
            effect,
            stage,
            message,
        }
    }
}

pub(crate) fn normalize_drag_paths(paths: Vec<String>) -> Result<Vec<PathBuf>, String> {
    if paths.is_empty() {
        return Err("Select at least one stack item first".to_string());
    }

    paths
        .into_iter()
        .map(|path| {
            if path.trim().is_empty() || path.contains('\0') {
                return Err("Invalid drag item path".to_string());
            }
            let candidate = resolve_stack_path_candidate(&path);
            if !candidate.is_absolute() || !candidate.exists() {
                return Err("Drag item unavailable".to_string());
            }
            Ok(candidate)
        })
        .collect()
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
pub(crate) fn start_native_file_drag(
    paths: &[PathBuf],
    hwnd: windows::Win32::Foundation::HWND,
) -> NativeDragResult {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::Com::IDataObject;
    use windows::Win32::System::Ole::{IDropSource, DROPEFFECT_COPY};
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{
        ILCreateFromPathW, ILFindLastID, SHCreateDataObject, SHDoDragDrop,
    };

    let failed = |stage, message: &str| {
        NativeDragResult::new("failed", "none", Some(stage), Some(message.into()))
    };
    let parent = match paths.first().and_then(|path| path.parent()) {
        Some(parent) => parent,
        None => return failed("parent", "Drag item parent unavailable"),
    };
    if paths.iter().any(|path| path.parent() != Some(parent)) {
        return NativeDragResult::new(
            "unsupported",
            "none",
            Some("mixed-parent"),
            Some("Dragging items from different folders is not yet supported".into()),
        );
    }
    let _ole = match OleApartment::initialize() {
        Ok(ole) => ole,
        Err(_) => return failed("ole-init", "Failed to initialize native drag"),
    };
    let make_pidl = |path: &std::path::Path| {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: the NUL-terminated buffer remains valid during the call; ILFree owns the result.
        let pointer = unsafe { ILCreateFromPathW(PCWSTR(wide.as_ptr())) };
        (!pointer.is_null()).then_some(OwnedPidl(pointer))
    };
    let parent_pidl = match make_pidl(parent) {
        Some(value) => value,
        None => return failed("parent-pidl", "Failed to resolve drag parent in Shell"),
    };
    let mut items = Vec::with_capacity(paths.len());
    for path in paths {
        match make_pidl(path) {
            Some(value) => items.push(value),
            None => return failed("child-pidl", "Failed to resolve drag item in Shell"),
        }
    }
    // ILFindLastID points into each owned absolute PIDL. All owners outlive SHCreateDataObject.
    let children: Vec<*const ITEMIDLIST> = items
        .iter()
        .map(|item| unsafe { ILFindLastID(item.0) as *const ITEMIDLIST })
        .collect();
    let data_object: IDataObject = match unsafe {
        SHCreateDataObject(Some(parent_pidl.0), Some(&children), None::<&IDataObject>)
    } {
        Ok(value) => value,
        Err(_) => return failed("data-object", "Failed to create Shell drag data"),
    };
    // SAFETY: the UI thread reads the current physical primary button state directly before OLE.
    if unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } as u16 & 0x8000 == 0 {
        return NativeDragResult::new("cancelled", "none", Some("gesture-expired"), None);
    }
    // SAFETY: owner HWND belongs to popup on the UI thread; OLE and PIDLs stay alive through drag.
    match unsafe {
        SHDoDragDrop(
            Some(hwnd),
            &data_object,
            None::<&IDropSource>,
            DROPEFFECT_COPY,
        )
    } {
        Ok(effect) if effect == DROPEFFECT_COPY => {
            NativeDragResult::new("copied", "copy", None, None)
        }
        Ok(_) => NativeDragResult::new("cancelled", "none", None, None),
        Err(_) => failed("shell-drop", "Native Shell drag failed"),
    }
}

#[cfg(target_os = "windows")]
struct OwnedPidl(*mut windows::Win32::UI::Shell::Common::ITEMIDLIST);

#[cfg(target_os = "windows")]
impl Drop for OwnedPidl {
    fn drop(&mut self) {
        // SAFETY: ILCreateFromPathW allocated this PIDL; it is freed exactly once.
        unsafe { windows::Win32::UI::Shell::ILFree(Some(self.0)) }
    }
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
