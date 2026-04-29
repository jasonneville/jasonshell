use super::everything::{
    EverythingProviderError, EverythingRawResult, EverythingResultKind, EverythingSdk,
    EverythingSearchRequest,
};
use crate::settings::EverythingSortMode;
use libloading::Library;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const REQUEST_FULL_PATH_AND_FILE_NAME: u32 = 0x0000_0004;
const REQUEST_RUN_COUNT: u32 = 0x0000_0400;
const BUFFER_CHARS: usize = 4096;
static EVERYTHING_SDK_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EverythingSdkDetection {
    pub dll_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub(crate) struct DynamicEverythingSdk {
    dll_path: PathBuf,
}

impl DynamicEverythingSdk {
    pub(crate) fn new(dll_path: PathBuf) -> Self {
        Self { dll_path }
    }
}

impl EverythingSdk for DynamicEverythingSdk {
    fn query(
        &mut self,
        request: &EverythingSearchRequest,
    ) -> Result<Vec<EverythingRawResult>, EverythingProviderError> {
        query_everything_dll(&self.dll_path, request)
    }

    fn reset(&mut self) {}
}

pub(crate) fn detect_system_sdk(installed_exe: Option<&Path>) -> EverythingSdkDetection {
    EverythingSdkDetection {
        dll_path: sdk_candidates(installed_exe)
            .into_iter()
            .find(|path| path.is_file()),
    }
}

pub(crate) fn sdk_candidates(installed_exe: Option<&Path>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(dir) = env::var_os("JASONSHELL_EVERYTHING_SDK_DIR").map(PathBuf::from) {
        candidates.push(dir.join("Everything64.dll"));
        candidates.push(dir.join("Everything.dll"));
    }

    if let Ok(current_dir) = env::current_dir() {
        push_repo_local_sdk_candidates(&mut candidates, &current_dir);
    }

    if let Some(exe) = installed_exe.and_then(Path::parent) {
        candidates.push(exe.join("Everything64.dll"));
        candidates.push(exe.join("Everything.dll"));
    }

    if let Some(program_files) = env::var_os("ProgramFiles").map(PathBuf::from) {
        candidates.push(program_files.join(r"Everything SDK\dll\Everything64.dll"));
        candidates.push(program_files.join(r"Everything\Everything64.dll"));
        candidates.push(program_files.join(r"Everything\Everything.dll"));
    }

    candidates
}

fn push_repo_local_sdk_candidates(candidates: &mut Vec<PathBuf>, current_dir: &Path) {
    for root in current_dir.ancestors().take(4) {
        let dll_dir = root.join(r".local\Everything-SDK\dll");
        candidates.push(dll_dir.join("Everything64.dll"));
        candidates.push(dll_dir.join("Everything.dll"));
    }
}

pub(crate) fn sdk_missing_message() -> &'static str {
    "Everything SDK DLL was not found in approved system SDK locations"
}

fn query_everything_dll(
    dll_path: &Path,
    request: &EverythingSearchRequest,
) -> Result<Vec<EverythingRawResult>, EverythingProviderError> {
    with_serialized_sdk_access(|| {
        // SAFETY: fixed Everything SDK symbols are called with documented signatures;
        // all string buffers are nul-terminated UTF-16 and reset is called before releasing the global SDK lock.
        unsafe {
            let library = Library::new(dll_path)
                .map_err(|error| EverythingProviderError::QueryFailed(error.to_string()))?;
            run_query_with_library(&library, request)
        }
    })
}

fn with_serialized_sdk_access<T>(
    operation: impl FnOnce() -> Result<T, EverythingProviderError>,
) -> Result<T, EverythingProviderError> {
    let _guard = EVERYTHING_SDK_LOCK.lock().map_err(|_| {
        EverythingProviderError::QueryFailed("Everything SDK lock failed".to_string())
    })?;
    operation()
}

unsafe fn run_query_with_library(
    library: &Library,
    request: &EverythingSearchRequest,
) -> Result<Vec<EverythingRawResult>, EverythingProviderError> {
    type SetSearch = unsafe extern "system" fn(*const u16) -> u32;
    type SetBool = unsafe extern "system" fn(i32);
    type SetU32 = unsafe extern "system" fn(u32);
    type Query = unsafe extern "system" fn(i32) -> i32;
    type GetU32 = unsafe extern "system" fn() -> u32;
    type ResultBool = unsafe extern "system" fn(u32) -> i32;
    type GetPath = unsafe extern "system" fn(u32, *mut u16, u32);
    type GetResultRunCount = unsafe extern "system" fn(u32) -> u32;
    type GetHighlighted = unsafe extern "system" fn(u32) -> *const u16;
    type Reset = unsafe extern "system" fn();

    let set_search =
        unsafe { library.get::<SetSearch>(b"Everything_SetSearchW\0") }.map_err(symbol_error)?;
    let set_match_path =
        unsafe { library.get::<SetBool>(b"Everything_SetMatchPath\0") }.map_err(symbol_error)?;
    let set_max = unsafe { library.get::<SetU32>(b"Everything_SetMax\0") }.map_err(symbol_error)?;
    let set_offset =
        unsafe { library.get::<SetU32>(b"Everything_SetOffset\0") }.map_err(symbol_error)?;
    let set_sort =
        unsafe { library.get::<SetU32>(b"Everything_SetSort\0") }.map_err(symbol_error)?;
    let set_request_flags =
        unsafe { library.get::<SetU32>(b"Everything_SetRequestFlags\0") }.map_err(symbol_error)?;
    let query = unsafe { library.get::<Query>(b"Everything_QueryW\0") }.map_err(symbol_error)?;
    let get_last_error =
        unsafe { library.get::<GetU32>(b"Everything_GetLastError\0") }.map_err(symbol_error)?;
    let get_num_results =
        unsafe { library.get::<GetU32>(b"Everything_GetNumResults\0") }.map_err(symbol_error)?;
    let is_folder = unsafe { library.get::<ResultBool>(b"Everything_IsFolderResult\0") }
        .map_err(symbol_error)?;
    let is_file =
        unsafe { library.get::<ResultBool>(b"Everything_IsFileResult\0") }.map_err(symbol_error)?;
    let get_path = unsafe { library.get::<GetPath>(b"Everything_GetResultFullPathNameW\0") }
        .map_err(symbol_error)?;
    let get_run_count =
        unsafe { library.get::<GetResultRunCount>(b"Everything_GetResultRunCount\0") }
            .map_err(symbol_error)?;
    let get_highlighted =
        unsafe { library.get::<GetHighlighted>(b"Everything_GetResultHighlightedFileNameW\0") }
            .map_err(symbol_error)?;
    let reset = unsafe { library.get::<Reset>(b"Everything_Reset\0") }.map_err(symbol_error)?;

    let result = (|| {
        let query_text = wide(&request.query);
        unsafe {
            set_search(query_text.as_ptr());
            set_offset(0);
            set_max(request.max_results as u32);
            set_sort(sort_mode(request.sort));
            set_match_path(i32::from(request.full_path_search));
            let flags = if request.sort == EverythingSortMode::RunCountDesc {
                REQUEST_FULL_PATH_AND_FILE_NAME | REQUEST_RUN_COUNT
            } else {
                REQUEST_FULL_PATH_AND_FILE_NAME
            };
            set_request_flags(flags);

            if query(1) == 0 {
                return Err(map_last_error(get_last_error()));
            }

            let count = get_num_results().min(request.max_results as u32);
            let mut results = Vec::with_capacity(count as usize);
            for index in 0..count {
                let mut buffer = vec![0u16; BUFFER_CHARS];
                get_path(index, buffer.as_mut_ptr(), BUFFER_CHARS as u32);
                let full_path = PathBuf::from(utf16_nul_terminated(&buffer));
                let kind = if is_folder(index) != 0 {
                    EverythingResultKind::Folder
                } else if is_file(index) != 0 {
                    EverythingResultKind::File
                } else {
                    EverythingResultKind::Volume
                };
                let highlighted = string_from_wide_ptr(get_highlighted(index));
                results.push(EverythingRawResult {
                    full_path,
                    kind,
                    run_count: get_run_count(index),
                    highlighted_file_name: highlighted,
                });
            }
            Ok(results)
        }
    })();

    unsafe {
        reset();
    }
    result
}

fn symbol_error(error: libloading::Error) -> EverythingProviderError {
    EverythingProviderError::QueryFailed(format!("Everything SDK symbol missing: {error}"))
}

fn map_last_error(code: u32) -> EverythingProviderError {
    match code {
        0 => EverythingProviderError::QueryFailed("Everything query returned false".to_string()),
        1 => EverythingProviderError::QueryFailed("Everything SDK memory error".to_string()),
        2 => EverythingProviderError::IpcUnavailable,
        6 => EverythingProviderError::QueryFailed("Everything SDK invalid call".to_string()),
        _ => EverythingProviderError::QueryFailed(format!("Everything SDK error {code}")),
    }
}

fn sort_mode(sort: EverythingSortMode) -> u32 {
    match sort {
        EverythingSortMode::NameAsc => 1,
        EverythingSortMode::PathAsc => 3,
        EverythingSortMode::DateModifiedDesc => 14,
        EverythingSortMode::RunCountDesc => 20,
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn utf16_nul_terminated(buffer: &[u16]) -> String {
    let len = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

unsafe fn string_from_wide_ptr(ptr: *const u16) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0usize;
    while unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    Some(String::from_utf16_lossy(unsafe {
        std::slice::from_raw_parts(ptr, len)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn sdk_candidates_include_installed_everything_directory_without_recursive_scan() {
        let candidates = sdk_candidates(Some(Path::new(
            r"C:\Program Files\Everything\Everything.exe",
        )));

        assert!(candidates
            .iter()
            .any(|path| path.ends_with(r"Everything\Everything64.dll")));
        assert!(candidates
            .iter()
            .any(|path| path.ends_with(r"Everything\Everything.dll")));
    }

    #[test]
    fn sdk_candidates_include_repo_local_everything_sdk_without_env_dependency() {
        let candidates = sdk_candidates(None);

        assert!(candidates
            .iter()
            .any(|path| path.ends_with(r".local\Everything-SDK\dll\Everything64.dll")));
    }

    #[test]
    fn maps_everything_sort_modes_to_sdk_constants() {
        assert_eq!(sort_mode(EverythingSortMode::NameAsc), 1);
        assert_eq!(sort_mode(EverythingSortMode::PathAsc), 3);
        assert_eq!(sort_mode(EverythingSortMode::DateModifiedDesc), 14);
        assert_eq!(sort_mode(EverythingSortMode::RunCountDesc), 20);
    }

    #[test]
    fn sdk_access_lock_serializes_overlapping_queries() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(4));
        let mut handles = Vec::new();

        for _ in 0..4 {
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                barrier.wait();
                with_serialized_sdk_access(|| {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_active.fetch_max(current, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(5));
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                })
                .unwrap();
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }
}
