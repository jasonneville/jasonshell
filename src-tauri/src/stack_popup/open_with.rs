use crate::stack_popup::models::StackOpenWithCandidate;
use std::path::Path;

#[cfg(windows)]
pub(crate) fn open_with_candidates_for_path(
    path: &Path,
) -> Result<Vec<StackOpenWithCandidate>, String> {
    let _apartment = native::Apartment::initialize()?;
    native::handlers(path).map(|handlers| {
        handlers
            .into_iter()
            .map(|(candidate, _)| candidate)
            .collect()
    })
}

#[cfg(windows)]
pub(crate) fn open_with_app(path: &Path, app_id: &str) -> Result<(), String> {
    let _apartment = native::Apartment::initialize()?;
    // Re-enumerate current registrations. Never interpret the renderer ID as a command.
    let (_, handler) = native::handlers(path)?
        .into_iter()
        .find(|(candidate, _)| candidate.id == app_id)
        .ok_or_else(|| "Selected Open With application is unavailable".to_string())?;
    native::invoke(&handler, path)
}

#[cfg(not(windows))]
pub(crate) fn open_with_candidates_for_path(
    _path: &Path,
) -> Result<Vec<StackOpenWithCandidate>, String> {
    Err("Windows associations are unavailable on this platform".into())
}

#[cfg(not(windows))]
pub(crate) fn open_with_app(_path: &Path, _app_id: &str) -> Result<(), String> {
    Err("Windows associations are unavailable on this platform".into())
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::collections::HashSet;
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::System::Com::{
        CoInitializeEx, CoTaskMemFree, CoUninitialize, IDataObject, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::Common::ITEMIDLIST;
    use windows::Win32::UI::Shell::{
        IAssocHandler, ILCreateFromPathW, ILFindLastID, ILFree, SHAssocEnumHandlers,
        SHCreateDataObject, ASSOC_FILTER_NONE,
    };

    pub(super) struct Apartment;
    impl Apartment {
        pub(super) fn initialize() -> Result<Self, String> {
            // S_OK and S_FALSE require balancing; changed apartment mode fails.
            unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
                .ok()
                .map_err(|error| format!("Failed to initialize Open With COM: {error}"))?;
            Ok(Self)
        }
    }
    impl Drop for Apartment {
        fn drop(&mut self) {
            // SAFETY: successful init on this worker; interfaces drop before guard.
            unsafe { CoUninitialize() }
        }
    }
    struct TaskString(PWSTR);
    impl Drop for TaskString {
        fn drop(&mut self) {
            // SAFETY: handler getters transfer a CoTaskMem allocation.
            unsafe { CoTaskMemFree(Some(self.0 .0.cast())) }
        }
    }
    impl TaskString {
        fn wide(&self) -> Result<&[u16], String> {
            if self.0.is_null() {
                return Err("Windows association returned an empty string".into());
            }
            // SAFETY: Shell returned a terminated string owned by this guard.
            Ok(unsafe { self.0.as_wide() })
        }
    }
    pub(super) fn handlers(
        path: &Path,
    ) -> Result<Vec<(StackOpenWithCandidate, IAssocHandler)>, String> {
        let Some(extension) = path.extension().filter(|extension| !extension.is_empty()) else {
            return Ok(Vec::new());
        };
        let dotted: Vec<u16> = std::iter::once(b'.' as u16)
            .chain(extension.encode_wide())
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: terminated buffer; interfaces stay in this worker apartment.
        let enumerator = unsafe { SHAssocEnumHandlers(PCWSTR(dotted.as_ptr()), ASSOC_FILTER_NONE) }
            .map_err(|error| format!("Failed to enumerate Windows associations: {error}"))?;
        let mut seen = HashSet::new();
        let mut handlers = Vec::new();
        loop {
            let mut slot = [None];
            let mut fetched = 0;
            unsafe { enumerator.Next(&mut slot, Some(&mut fetched)) }
                .map_err(|error| format!("Failed to enumerate Windows associations: {error}"))?;
            if fetched == 0 {
                break;
            }
            let handler = slot[0]
                .take()
                .ok_or_else(|| "Windows association handler is unavailable".to_string())?;
            let name = TaskString(unsafe { handler.GetName() }.map_err(|error| {
                format!("Failed to read Windows association identity: {error}")
            })?);
            let identity = name.wide()?;
            if identity.is_empty() {
                return Err("Windows association identity is empty".into());
            }
            // Lossless native UTF-16 identity, never display-label deduplication.
            let id = identity
                .iter()
                .map(|unit| format!("{unit:04x}"))
                .collect::<String>();
            if !seen.insert(id.clone()) {
                continue;
            }
            let label =
                TaskString(unsafe { handler.GetUIName() }.map_err(|error| {
                    format!("Failed to read Windows association label: {error}")
                })?);
            let label = String::from_utf16(label.wide()?)
                .map_err(|_| "Windows association label is invalid".to_string())?;
            if label.trim().is_empty() {
                return Err("Windows association label is empty".into());
            }
            handlers.push((
                StackOpenWithCandidate {
                    id,
                    label,
                    source: "windows-association".into(),
                },
                handler,
            ));
        }
        Ok(handlers)
    }
    struct OwnedPidl(*mut ITEMIDLIST);
    impl Drop for OwnedPidl {
        fn drop(&mut self) {
            // SAFETY: ILCreateFromPathW allocation, freed exactly once.
            unsafe { ILFree(Some(self.0)) }
        }
    }
    fn pidl(path: &Path) -> Result<OwnedPidl, String> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: terminated buffer is live during call.
        let pointer = unsafe { ILCreateFromPathW(PCWSTR(wide.as_ptr())) };
        if pointer.is_null() {
            return Err("Failed to resolve Open With file in Shell".into());
        }
        Ok(OwnedPidl(pointer))
    }
    pub(super) fn invoke(handler: &IAssocHandler, path: &Path) -> Result<(), String> {
        let parent = path
            .parent()
            .ok_or_else(|| "Open With file parent is unavailable".to_string())?;
        let parent = pidl(parent)?;
        let item = pidl(path)?;
        // SAFETY: borrowed child pointer and both owners outlive data and Invoke.
        let children = [unsafe { ILFindLastID(item.0) } as *const ITEMIDLIST];
        let data: IDataObject =
            unsafe { SHCreateDataObject(Some(parent.0), Some(&children), None::<&IDataObject>) }
                .map_err(|error| format!("Failed to create Open With Shell data: {error}"))?;
        unsafe { handler.Invoke(&data) }
            .map_err(|error| format!("Failed to invoke Windows association: {error}"))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::native;
    use std::path::Path;

    #[test]
    fn extensionless_names_return_empty_without_com_or_filesystem_work() {
        // No apartment, registered-handler enumeration, files, picker or launch.
        // Calling the native adapter directly proves its pre-COM empty branch.
        for path in [r"C:\fixture\README", r"C:\fixture\.gitignore", r"C:\fixture\trailing."] {
            assert!(native::handlers(Path::new(path)).unwrap().is_empty(), "{path}");
        }
    }
}
