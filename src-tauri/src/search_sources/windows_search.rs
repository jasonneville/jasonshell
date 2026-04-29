#![allow(dead_code)]

use super::SystemSearchResult;

const MAX_WINDOWS_RESULTS: usize = 100;
const WINDOWS_SEARCH_COLUMNS: [&str; 5] = [
    "System.ItemUrl",
    "System.ItemPathDisplay",
    "System.FileName",
    "System.ItemTypeText",
    "System.KindText",
];

pub(crate) enum ProviderSearchOutcome {
    Results(Vec<SystemSearchResult>),
    Fallback { reason: String },
}

pub(crate) fn search_windows(query: &str, limit: usize) -> ProviderSearchOutcome {
    imp::search_windows(query, limit)
}

fn bounded_limit(limit: usize) -> i32 {
    result_limit(limit) as i32
}

fn result_limit(limit: usize) -> usize {
    limit.clamp(1, MAX_WINDOWS_RESULTS)
}

fn windows_search_select_columns() -> &'static str {
    "System.ItemUrl,System.ItemPathDisplay,System.FileName,System.ItemTypeText,System.KindText"
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct WindowsSearchRow {
    item_url: Option<String>,
    item_path_display: Option<String>,
    file_name: Option<String>,
    item_type_text: Option<String>,
    kind_text: Option<String>,
}

fn map_windows_search_row(row: WindowsSearchRow, rank_index: usize) -> Option<SystemSearchResult> {
    let path = row
        .item_path_display
        .as_deref()
        .and_then(non_empty)
        .map(str::to_string)
        .or_else(|| row.item_url.as_deref().and_then(file_url_to_path))?;
    let kind = infer_result_kind(
        &path,
        row.item_type_text.as_deref(),
        row.kind_text.as_deref(),
    );
    let title = title_for_result(row.file_name.as_deref(), &path, &kind);
    let subtitle = subtitle_for_result(&kind, &path);
    let priority = base_priority(&kind) - rank_index.min(25) as i32;
    let terms = [
        title.as_str(),
        subtitle.as_str(),
        path.as_str(),
        row.item_type_text.as_deref().unwrap_or_default(),
        row.kind_text.as_deref().unwrap_or_default(),
        "windows search systemindex local filesystem installed program",
    ]
    .join(" ");

    Some(SystemSearchResult {
        id: format!("system:{kind}:{path}"),
        provider_id: Some("windowsSearch".to_string()),
        kind: kind.clone(),
        title,
        subtitle,
        terms,
        priority,
        path: path.clone(),
        record_key: Some(format!(
            "{}:{}",
            kind,
            path.trim().replace('/', r"\").to_lowercase()
        )),
        run_count: None,
        top_most: None,
    })
}

fn infer_result_kind(path: &str, item_type_text: Option<&str>, kind_text: Option<&str>) -> String {
    let path_lower = path.to_lowercase();
    let text = format!(
        "{} {}",
        item_type_text.unwrap_or_default().to_lowercase(),
        kind_text.unwrap_or_default().to_lowercase()
    );

    if text.contains("folder") || text.contains("directory") {
        return "folder".to_string();
    }

    if text.contains("program") || text.contains("application") || has_app_extension(&path_lower) {
        return "app".to_string();
    }

    "file".to_string()
}

fn title_for_result(file_name: Option<&str>, path: &str, kind: &str) -> String {
    let raw = file_name
        .and_then(non_empty)
        .unwrap_or_else(|| last_path_segment(path));
    if kind == "folder" {
        return raw.to_string();
    }
    strip_extension(raw).to_string()
}

fn subtitle_for_result(kind: &str, path: &str) -> String {
    let label = match kind {
        "app" => "Installed app",
        "folder" => "Folder",
        _ => "File",
    };

    parent_path(path)
        .map(|parent| format!("{label} - {parent}"))
        .unwrap_or_else(|| label.to_string())
}

fn base_priority(kind: &str) -> i32 {
    match kind {
        "app" => 118,
        "folder" => 86,
        _ => 80,
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn file_url_to_path(value: &str) -> Option<String> {
    let value = non_empty(value)?;
    let lower = value.to_lowercase();
    let path = lower
        .strip_prefix("file:///")
        .map(|_| &value[8..])
        .or_else(|| lower.strip_prefix("file://").map(|_| &value[7..]))?;
    Some(percent_decode(path).replace('/', "\\"))
}

fn percent_decode(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&value[index + 1..index + 3], 16) {
                output.push(hex as char);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index] as char);
        index += 1;
    }
    output
}

fn has_app_extension(path: &str) -> bool {
    [".lnk", ".exe", ".appref-ms", ".url"]
        .iter()
        .any(|extension| path.ends_with(extension))
}

fn last_path_segment(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

fn parent_path(path: &str) -> Option<&str> {
    path.rfind(['\\', '/'])
        .and_then(|index| (index > 0).then_some(&path[..index]))
}

fn strip_extension(value: &str) -> &str {
    value
        .rfind('.')
        .and_then(|index| (index > 0).then_some(&value[..index]))
        .unwrap_or(value)
}

#[cfg(target_os = "windows")]
mod imp {
    use super::{
        bounded_limit, map_windows_search_row, result_limit, windows_search_select_columns,
        ProviderSearchOutcome, WindowsSearchRow, WINDOWS_SEARCH_COLUMNS,
    };
    use std::collections::HashSet;
    use std::ffi::{c_void, OsStr};
    use std::mem::{offset_of, size_of};
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;
    use std::slice;
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};
    use windows::core::{w, IUnknown, Interface, PCWSTR, PWSTR};
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
        CLSCTX_LOCAL_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
    };
    use windows::Win32::System::Search::{
        CSearchManager, IAccessor, ICommandText, IDBCreateCommand, IDBCreateSession,
        IDataInitialize, IRowset, ISearchManager, DBACCESSOR_ROWDATA, DBBINDING, DBBINDSTATUS_OK,
        DBMEMOWNER_CLIENTOWNED, DBPART_LENGTH, DBPART_STATUS, DBPART_VALUE, DBSTATUS_S_OK,
        DBTYPE_WSTR, HACCESSOR, MSDAINITIALIZE,
    };

    const COLUMN_CHARS: usize = 520;
    const PROVIDER_RETRY_AFTER: Duration = Duration::from_secs(30);
    const SEARCH_COLLATOR_INIT: &str =
        "Provider=Search.CollatorDSO.1;Extended Properties='Application=Windows';";

    static PROVIDER_FAILURE: OnceLock<Mutex<Option<CachedFailure>>> = OnceLock::new();

    struct CachedFailure {
        reason: String,
        failed_at: Instant,
    }

    #[repr(C)]
    #[derive(Clone)]
    struct BoundColumn {
        length: usize,
        status: u32,
        value: [u16; COLUMN_CHARS],
    }

    impl Default for BoundColumn {
        fn default() -> Self {
            Self {
                length: 0,
                status: 0,
                value: [0; COLUMN_CHARS],
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Default)]
    struct RowBuffer {
        columns: [BoundColumn; WINDOWS_SEARCH_COLUMNS.len()],
    }

    pub(super) fn search_windows(query: &str, limit: usize) -> ProviderSearchOutcome {
        if query.trim().len() < 2 {
            return ProviderSearchOutcome::Fallback {
                reason: "Windows Search query is too short".to_string(),
            };
        }

        if let Some(reason) = cached_provider_failure() {
            return ProviderSearchOutcome::Fallback { reason };
        }

        match search_windows_results(query, result_limit(limit)) {
            Ok(results) if results.is_empty() => ProviderSearchOutcome::Fallback {
                reason: "Windows Search returned no usable rows".to_string(),
            },
            Ok(results) => {
                clear_provider_failure();
                ProviderSearchOutcome::Results(results)
            }
            Err(reason) => {
                remember_provider_failure(reason.clone());
                ProviderSearchOutcome::Fallback { reason }
            }
        }
    }

    fn search_windows_results(
        query: &str,
        limit: usize,
    ) -> Result<Vec<super::SystemSearchResult>, String> {
        let _apartment = ComApartment::initialize()?;
        let sql = generate_sql_from_windows_search(query, limit)?;
        let rowset = execute_windows_search_sql(&sql)?;
        collect_rowset_results(&rowset, limit)
    }

    fn generate_sql_from_windows_search(query: &str, limit: usize) -> Result<String, String> {
        let manager: ISearchManager =
            unsafe { CoCreateInstance(&CSearchManager, None, CLSCTX_LOCAL_SERVER) }
                .map_err(|error| format!("Windows Search manager unavailable: {error}"))?;
        let catalog = unsafe { manager.GetCatalog(w!("SystemIndex")) }
            .map_err(|error| format!("Windows Search SystemIndex catalog unavailable: {error}"))?;
        let helper = unsafe { catalog.GetQueryHelper() }
            .map_err(|error| format!("Windows Search query helper unavailable: {error}"))?;

        unsafe {
            helper
                .SetQuerySelectColumns(wide(windows_search_select_columns()).as_pcwstr())
                .map_err(|error| format!("Windows Search select columns rejected: {error}"))?;
            helper
                .SetQueryMaxResults(bounded_limit(limit))
                .map_err(|error| format!("Windows Search result limit rejected: {error}"))?;
        }

        let query = wide(query);
        let sql = unsafe {
            helper
                .GenerateSQLFromUserQuery(query.as_pcwstr())
                .map_err(|error| format!("Windows Search SQL generation failed: {error}"))?
        };
        Ok(unsafe { take_pwstr(sql) })
    }

    fn execute_windows_search_sql(sql: &str) -> Result<IRowset, String> {
        let data_initialize: IDataInitialize =
            unsafe { CoCreateInstance(&MSDAINITIALIZE, None, CLSCTX_INPROC_SERVER) }.map_err(
                |error| format!("Windows Search OLE DB initializer unavailable: {error}"),
            )?;
        let init = wide(SEARCH_COLLATOR_INIT);
        let mut data_source: Option<IUnknown> = None;
        unsafe {
            data_initialize
                .GetDataSource(
                    None::<&IUnknown>,
                    CLSCTX_INPROC_SERVER.0,
                    init.as_pcwstr(),
                    &IDBCreateSession::IID,
                    &mut data_source,
                )
                .map_err(|error| {
                    format!("Windows Search OLE DB data source unavailable: {error}")
                })?;
        }

        let data_source: IDBCreateSession = data_source
            .ok_or_else(|| "Windows Search OLE DB data source was not returned".to_string())?
            .cast()
            .map_err(|error| format!("Windows Search OLE DB session binding failed: {error}"))?;
        let session: IDBCreateCommand = unsafe {
            data_source
                .CreateSession(None::<&IUnknown>, &IDBCreateCommand::IID)
                .map_err(|error| format!("Windows Search OLE DB session unavailable: {error}"))?
                .cast()
                .map_err(|error| format!("Windows Search OLE DB command binding failed: {error}"))?
        };
        let command: ICommandText = unsafe {
            session
                .CreateCommand(None::<&IUnknown>, &ICommandText::IID)
                .map_err(|error| format!("Windows Search OLE DB command unavailable: {error}"))?
                .cast()
                .map_err(|error| {
                    format!("Windows Search OLE DB command text binding failed: {error}")
                })?
        };
        let sql = wide(sql);
        unsafe {
            command
                .SetCommandText(ptr::null(), sql.as_pcwstr())
                .map_err(|error| format!("Windows Search SQL command was rejected: {error}"))?;
        }

        let mut rowset: Option<IUnknown> = None;
        unsafe {
            command
                .Execute(
                    None::<&IUnknown>,
                    &IRowset::IID,
                    None,
                    None,
                    Some(&mut rowset),
                )
                .map_err(|error| format!("Windows Search SQL execution failed: {error}"))?;
        }
        rowset
            .ok_or_else(|| "Windows Search SQL returned no rowset".to_string())?
            .cast()
            .map_err(|error| format!("Windows Search rowset binding failed: {error}"))
    }

    fn collect_rowset_results(
        rowset: &IRowset,
        limit: usize,
    ) -> Result<Vec<super::SystemSearchResult>, String> {
        let accessor = RowAccessor::create(rowset)?;
        let mut results = Vec::new();
        let mut seen = HashSet::new();

        while results.len() < limit {
            let Some(handles) = RowHandles::next(rowset, (limit - results.len()).min(8))? else {
                break;
            };

            for handle in handles.as_slice() {
                let mut buffer = RowBuffer::default();
                let loaded = unsafe {
                    rowset.GetData(
                        *handle,
                        accessor.handle(),
                        (&mut buffer as *mut RowBuffer).cast::<c_void>(),
                    )
                }
                .is_ok();
                if !loaded {
                    continue;
                }

                if let Some(result) = row_from_buffer(&buffer)
                    .and_then(|row| map_windows_search_row(row, results.len()))
                {
                    let key = result.id.to_lowercase();
                    if seen.insert(key) {
                        results.push(result);
                    }
                }
            }
        }

        Ok(results)
    }

    fn row_bindings() -> [DBBINDING; WINDOWS_SEARCH_COLUMNS.len()] {
        std::array::from_fn(|index| {
            let column_offset = offset_of!(RowBuffer, columns) + index * size_of::<BoundColumn>();
            let mut binding = DBBINDING::default();
            binding.iOrdinal = index + 1;
            binding.obLength = column_offset + offset_of!(BoundColumn, length);
            binding.obStatus = column_offset + offset_of!(BoundColumn, status);
            binding.obValue = column_offset + offset_of!(BoundColumn, value);
            binding.dwPart = (DBPART_VALUE.0 | DBPART_LENGTH.0 | DBPART_STATUS.0) as u32;
            binding.dwMemOwner = DBMEMOWNER_CLIENTOWNED.0 as u32;
            binding.cbMaxLen = COLUMN_CHARS * size_of::<u16>();
            binding.wType = DBTYPE_WSTR.0 as u16;
            binding
        })
    }

    fn row_from_buffer(buffer: &RowBuffer) -> Option<WindowsSearchRow> {
        let values = buffer
            .columns
            .iter()
            .map(column_text)
            .collect::<Vec<Option<String>>>();
        Some(WindowsSearchRow {
            item_url: values.first()?.clone(),
            item_path_display: values.get(1)?.clone(),
            file_name: values.get(2)?.clone(),
            item_type_text: values.get(3)?.clone(),
            kind_text: values.get(4)?.clone(),
        })
    }

    fn column_text(column: &BoundColumn) -> Option<String> {
        if column.status != DBSTATUS_S_OK.0 as u32 || column.length == 0 {
            return None;
        }

        let char_count = (column.length / size_of::<u16>()).min(COLUMN_CHARS);
        let text = String::from_utf16_lossy(&column.value[..char_count])
            .trim_matches(char::from(0))
            .trim()
            .to_string();
        (!text.is_empty()).then_some(text)
    }

    struct RowAccessor {
        accessor: IAccessor,
        handle: HACCESSOR,
    }

    impl RowAccessor {
        fn create(rowset: &IRowset) -> Result<Self, String> {
            let accessor: IAccessor = rowset
                .cast()
                .map_err(|error| format!("Windows Search row accessor binding failed: {error}"))?;
            let bindings = row_bindings();
            let mut handle = HACCESSOR::default();
            let mut statuses = [0u32; WINDOWS_SEARCH_COLUMNS.len()];
            unsafe {
                accessor
                    .CreateAccessor(
                        DBACCESSOR_ROWDATA.0 as u32,
                        bindings.len(),
                        bindings.as_ptr(),
                        size_of::<RowBuffer>(),
                        &mut handle,
                        Some(statuses.as_mut_ptr()),
                    )
                    .map_err(|error| {
                        format!("Windows Search row accessor creation failed: {error}")
                    })?;
            }

            if statuses
                .iter()
                .any(|status| *status != DBBINDSTATUS_OK.0 as u32)
            {
                unsafe {
                    let _ = accessor.ReleaseAccessor(handle, None);
                }
                return Err(format!(
                    "Windows Search row accessor rejected bindings: {statuses:?}"
                ));
            }

            Ok(Self { accessor, handle })
        }

        fn handle(&self) -> HACCESSOR {
            self.handle
        }
    }

    impl Drop for RowAccessor {
        fn drop(&mut self) {
            if !self.handle.is_invalid() {
                unsafe {
                    let _ = self.accessor.ReleaseAccessor(self.handle, None);
                }
            }
        }
    }

    struct RowHandles<'a> {
        rowset: &'a IRowset,
        ptr: *mut usize,
        count: usize,
    }

    impl<'a> RowHandles<'a> {
        fn next(rowset: &'a IRowset, count: usize) -> Result<Option<Self>, String> {
            let mut obtained = 0usize;
            let mut row_ptrs = vec![ptr::null_mut(); count.max(1)];
            unsafe {
                rowset
                    .GetNextRows(0, 0, &mut obtained, &mut row_ptrs)
                    .map_err(|error| format!("Windows Search row retrieval failed: {error}"))?;
            }

            if obtained == 0 || row_ptrs[0].is_null() {
                return Ok(None);
            }

            Ok(Some(Self {
                rowset,
                ptr: row_ptrs[0].cast::<usize>(),
                count: obtained,
            }))
        }

        fn as_slice(&self) -> &[usize] {
            unsafe { slice::from_raw_parts(self.ptr, self.count) }
        }
    }

    impl Drop for RowHandles<'_> {
        fn drop(&mut self) {
            unsafe {
                let _ = self.rowset.ReleaseRows(
                    self.count,
                    self.ptr,
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                CoTaskMemFree(Some(self.ptr.cast()));
            }
        }
    }

    fn cached_provider_failure() -> Option<String> {
        let guard = PROVIDER_FAILURE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .ok()?;
        let failure = guard.as_ref()?;
        (failure.failed_at.elapsed() < PROVIDER_RETRY_AFTER).then(|| failure.reason.clone())
    }

    fn remember_provider_failure(reason: String) {
        if let Ok(mut guard) = PROVIDER_FAILURE.get_or_init(|| Mutex::new(None)).lock() {
            *guard = Some(CachedFailure {
                reason,
                failed_at: Instant::now(),
            });
        }
    }

    fn clear_provider_failure() {
        if let Ok(mut guard) = PROVIDER_FAILURE.get_or_init(|| Mutex::new(None)).lock() {
            *guard = None;
        }
    }

    struct ComApartment {
        should_uninitialize: bool,
    }

    impl ComApartment {
        fn initialize() -> Result<Self, String> {
            let hr =
                unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
            if hr.is_ok() {
                return Ok(Self {
                    should_uninitialize: true,
                });
            }
            if hr == RPC_E_CHANGED_MODE {
                return Ok(Self {
                    should_uninitialize: false,
                });
            }
            Err(format!(
                "Failed to initialize COM for Windows Search: {hr:?}"
            ))
        }
    }

    impl Drop for ComApartment {
        fn drop(&mut self) {
            if self.should_uninitialize {
                unsafe {
                    CoUninitialize();
                }
            }
        }
    }

    struct WideString(Vec<u16>);

    impl WideString {
        fn as_pcwstr(&self) -> PCWSTR {
            PCWSTR(self.0.as_ptr())
        }
    }

    fn wide(value: &str) -> WideString {
        WideString(
            OsStr::new(value)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect(),
        )
    }

    unsafe fn take_pwstr(value: PWSTR) -> String {
        if value.is_null() {
            return String::new();
        }

        let mut len = 0;
        while unsafe { *value.0.add(len) } != 0 {
            len += 1;
        }
        let text = unsafe { String::from_utf16_lossy(std::slice::from_raw_parts(value.0, len)) };
        unsafe {
            CoTaskMemFree(Some(value.0.cast()));
        }
        text
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::ProviderSearchOutcome;

    pub(super) fn search_windows(_query: &str, _limit: usize) -> ProviderSearchOutcome {
        ProviderSearchOutcome::Fallback {
            reason: "Windows Search is only available on Windows".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_search_limit_is_bounded_for_query_helper() {
        assert_eq!(bounded_limit(0), 1);
        assert_eq!(bounded_limit(40), 40);
        assert_eq!(bounded_limit(500), 100);
    }

    #[test]
    fn windows_search_result_limit_is_bounded_for_row_retrieval() {
        assert_eq!(result_limit(0), 1);
        assert_eq!(result_limit(40), 40);
        assert_eq!(result_limit(500), 100);
    }

    #[test]
    fn windows_search_provider_requests_displayable_columns() {
        let columns = windows_search_select_columns();

        assert!(columns.contains("System.ItemUrl"));
        assert!(columns.contains("System.ItemPathDisplay"));
        assert!(columns.contains("System.FileName"));
        assert!(columns.contains("System.KindText"));
    }

    #[test]
    fn maps_windows_search_file_row_to_system_result() {
        let result = map_windows_search_row(
            WindowsSearchRow {
                item_path_display: Some(r"C:\Users\me\Documents\Budget.xlsx".to_string()),
                file_name: Some("Budget.xlsx".to_string()),
                item_type_text: Some("Microsoft Excel Worksheet".to_string()),
                kind_text: Some("Document".to_string()),
                ..WindowsSearchRow::default()
            },
            0,
        )
        .unwrap();

        assert_eq!(result.kind, "file");
        assert_eq!(result.title, "Budget");
        assert_eq!(result.subtitle, r"File - C:\Users\me\Documents");
        assert_eq!(result.id, r"system:file:C:\Users\me\Documents\Budget.xlsx");
    }

    #[test]
    fn maps_windows_search_folder_row_to_system_result() {
        let result = map_windows_search_row(
            WindowsSearchRow {
                item_path_display: Some(r"C:\Users\me\Downloads".to_string()),
                file_name: Some("Downloads".to_string()),
                item_type_text: Some("File folder".to_string()),
                kind_text: Some("Folder".to_string()),
                ..WindowsSearchRow::default()
            },
            2,
        )
        .unwrap();

        assert_eq!(result.kind, "folder");
        assert_eq!(result.title, "Downloads");
        assert_eq!(result.subtitle, r"Folder - C:\Users\me");
        assert_eq!(result.priority, 84);
    }

    #[test]
    fn maps_windows_search_program_row_to_app_result() {
        let result = map_windows_search_row(
            WindowsSearchRow {
                item_path_display: Some(
                    r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Notepad.lnk".to_string(),
                ),
                file_name: Some("Notepad.lnk".to_string()),
                item_type_text: Some("Shortcut".to_string()),
                kind_text: Some("Program".to_string()),
                ..WindowsSearchRow::default()
            },
            1,
        )
        .unwrap();

        assert_eq!(result.kind, "app");
        assert_eq!(result.title, "Notepad");
        assert_eq!(
            result.subtitle,
            r"Installed app - C:\ProgramData\Microsoft\Windows\Start Menu\Programs"
        );
    }

    #[test]
    fn maps_file_url_when_display_path_is_missing() {
        let result = map_windows_search_row(
            WindowsSearchRow {
                item_url: Some("file:///C:/Users/me/My%20File.txt".to_string()),
                kind_text: Some("Document".to_string()),
                ..WindowsSearchRow::default()
            },
            0,
        )
        .unwrap();

        assert_eq!(result.path, r"C:\Users\me\My File.txt");
        assert_eq!(result.title, "My File");
    }
}
