use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CopyIcon, DestroyIcon, EnumChildWindows, FindWindowExW, FindWindowW, GetClassNameW,
    GetWindowThreadProcessId, PostMessageW, SendMessageTimeoutW, SetForegroundWindow, HICON,
    SMTO_ABORTIFHUNG, SMTO_ERRORONEXIT, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

const TB_BUTTONCOUNT: u32 = 0x0418;
const TB_GETBUTTON: u32 = 0x0417;
const TB_GETITEMRECT: u32 = 0x041D;
const TBSTATE_HIDDEN: u8 = 0x08;
const SEND_TIMEOUT_MS: u32 = 50;
const MAX_TRAY_METADATA_BYTES: usize = 64;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemTrayIconSnapshot {
    pub id: String,
    pub command_id: i32,
    pub index: i32,
    pub label: String,
    pub icon_data_url: String,
    pub has_native_icon: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvokeSystemTrayIconRequest {
    pub id: String,
    pub button: SystemTrayMouseButton,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SystemTrayMouseButton {
    Left,
    Right,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct ToolbarButton {
    i_bitmap: i32,
    id_command: i32,
    fs_state: u8,
    fs_style: u8,
    b_reserved: [u8; 6],
    dw_data: usize,
    i_string: isize,
}

#[derive(Clone, Copy)]
struct TrayButtonRef {
    toolbar: HWND,
    index: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrayToolbarCandidate {
    hwnd: HWND,
    source: ToolbarDiscoverySource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolbarDiscoverySource {
    TrayNotifyArea,
    OverflowWindow,
}

impl ToolbarDiscoverySource {
    fn id_segment(self) -> &'static str {
        match self {
            Self::TrayNotifyArea => "tray-notify",
            Self::OverflowWindow => "overflow",
        }
    }

    fn from_id_segment(value: &str) -> Option<Self> {
        match value {
            "tray-notify" => Some(Self::TrayNotifyArea),
            "overflow" => Some(Self::OverflowWindow),
            _ => None,
        }
    }
}

pub fn list_system_tray_icons() -> Result<Vec<SystemTrayIconSnapshot>, String> {
    let candidates = tray_toolbar_candidates();
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let mut first_error = None;
    let snapshots = candidates
        .into_iter()
        .filter_map(|candidate| match list_toolbar_icons(candidate) {
            Ok(icons) => Some(icons),
            Err(error) => {
                first_error.get_or_insert(error);
                None
            }
        })
        .collect::<Vec<_>>();
    let merged = merge_toolbar_snapshots(snapshots);
    if merged.is_empty() {
        if let Some(error) = first_error {
            return Err(error);
        }
    }
    Ok(merged)
}

pub fn invoke_system_tray_icon(request: InvokeSystemTrayIconRequest) -> Result<(), String> {
    let parsed = parse_snapshot_id(&request.id)?;
    let Some(toolbar) = tray_toolbar_candidates()
        .into_iter()
        .find(|candidate| {
            candidate.source == parsed.source && candidate.hwnd.0 as isize == parsed.toolbar_hwnd
        })
        .map(|candidate| candidate.hwnd)
    else {
        return Err(
            "Explorer notification-area toolbar changed; refresh tray icons and retry".to_string(),
        );
    };
    let button = find_button_by_command(toolbar, parsed.command_id).ok_or_else(|| {
        "Tray icon is no longer visible in Explorer's notification area".to_string()
    })?;
    click_toolbar_button(button, request.button)
}

fn list_toolbar_icons(
    candidate: TrayToolbarCandidate,
) -> Result<Vec<SystemTrayIconSnapshot>, String> {
    let toolbar = candidate.hwnd;
    let process = RemoteProcess::for_window(toolbar)?;
    let count = send_toolbar_message(toolbar, TB_BUTTONCOUNT, WPARAM(0), LPARAM(0))? as i32;
    if count <= 0 {
        return Ok(Vec::new());
    }

    let mut icons = Vec::new();
    for index in 0..count.min(64) {
        let Some(button) = read_toolbar_button(toolbar, &process, index)? else {
            continue;
        };
        if normalize_tray_button(button).is_none() {
            continue;
        }
        let native_icon = tray_native_icon_data_url(&process, button);
        let (icon_data_url, has_native_icon) = native_icon
            .map(|data_url| (data_url, true))
            .unwrap_or_else(|| (tray_icon_placeholder_data_url(), false));
        icons.push(SystemTrayIconSnapshot {
            id: snapshot_id(candidate.source, toolbar, button.id_command),
            command_id: button.id_command,
            index,
            label: format!("Notification area icon {}", icons.len() + 1),
            icon_data_url,
            has_native_icon,
        });
    }

    Ok(icons)
}

fn find_button_by_command(toolbar: HWND, command_id: i32) -> Option<TrayButtonRef> {
    let process = RemoteProcess::for_window(toolbar).ok()?;
    let count = send_toolbar_message(toolbar, TB_BUTTONCOUNT, WPARAM(0), LPARAM(0)).ok()? as i32;
    for index in 0..count.min(64) {
        let Some(button) = read_toolbar_button(toolbar, &process, index).ok().flatten() else {
            continue;
        };
        let Some(button) = normalize_tray_button(button) else {
            continue;
        };
        if button.id_command == command_id {
            return Some(TrayButtonRef { toolbar, index });
        }
    }
    None
}

fn click_toolbar_button(
    button: TrayButtonRef,
    mouse_button: SystemTrayMouseButton,
) -> Result<(), String> {
    let process = RemoteProcess::for_window(button.toolbar)?;
    let remote_rect = RemoteAllocation::new(&process, size_of::<RECT>())?;
    send_toolbar_message(
        button.toolbar,
        TB_GETITEMRECT,
        WPARAM(button.index as usize),
        LPARAM(remote_rect.ptr as isize),
    )?;
    let rect: RECT = process.read_struct(remote_rect.ptr)?;
    let x = (rect.left + rect.right) / 2;
    let y = (rect.top + rect.bottom) / 2;
    let lparam = LPARAM(((y as u32) << 16 | (x as u32 & 0xffff)) as isize);
    let (down, up) = match mouse_button {
        SystemTrayMouseButton::Left => (WM_LBUTTONDOWN, WM_LBUTTONUP),
        SystemTrayMouseButton::Right => (WM_RBUTTONDOWN, WM_RBUTTONUP),
    };

    unsafe {
        // SAFETY: The toolbar HWND was discovered from Explorer immediately before dispatch.
        // Focusing it and posting mouse messages at a button-local point mirrors user input without
        // mutating Explorer settings or taking ownership of notification-area registration.
        let _ = SetForegroundWindow(button.toolbar);
        PostMessageW(Some(button.toolbar), down, WPARAM(0), lparam)
            .map_err(|error| format!("Failed to post tray mouse-down: {error}"))?;
        PostMessageW(Some(button.toolbar), up, WPARAM(0), lparam)
            .map_err(|error| format!("Failed to post tray mouse-up: {error}"))?;
    }
    Ok(())
}

fn tray_icon_placeholder_data_url() -> String {
    // Keep the relay snapshot safe by exposing a placeholder when bounded native icon extraction
    // cannot obtain a copied icon handle for this tray item.
    crate::task_windows::empty_icon_data_url()
}

fn tray_native_icon_data_url(process: &RemoteProcess, button: ToolbarButton) -> Option<String> {
    let metadata = read_tray_metadata(process, button.dw_data).ok()?;
    for icon_handle in metadata.icon_handles() {
        let Some(copied_icon) = copy_foreign_icon(icon_handle) else {
            continue;
        };
        let data_url = crate::task_windows::hicon_data_url(copied_icon).ok();
        destroy_local_icon(copied_icon);
        if data_url.is_some() {
            return data_url;
        }
    }
    None
}

fn read_tray_metadata(
    process: &RemoteProcess,
    metadata_ptr: usize,
) -> Result<TrayMetadata, String> {
    if metadata_ptr == 0 {
        return Err("Explorer tray metadata pointer was null".to_string());
    }
    let bytes = process.read_bytes(metadata_ptr as *const c_void, tray_metadata_read_bytes())?;
    TrayMetadata::parse(&bytes)
}

fn copy_foreign_icon(icon: HICON) -> Option<HICON> {
    if icon.0.is_null() {
        return None;
    }
    let copied = unsafe {
        // SAFETY: The handle value came from Explorer tray metadata read as data, not by
        // dereferencing Explorer memory locally. USER icon handles are copied via CopyIcon; failure
        // is treated as per-icon absence and no foreign handle is retained.
        CopyIcon(icon).ok()?
    };
    if copied.0.is_null() {
        None
    } else {
        Some(copied)
    }
}

fn destroy_local_icon(icon: HICON) {
    if icon.0.is_null() {
        return;
    }
    unsafe {
        // SAFETY: `icon` is the local duplicate returned by CopyIcon in this process; destroying it
        // does not affect Explorer's original tray icon handle.
        let _ = DestroyIcon(icon);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TrayMetadata {
    icon_values: Vec<usize>,
}

impl TrayMetadata {
    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_TRAY_METADATA_BYTES {
            return Err("Explorer tray metadata read exceeded bounded parser size".to_string());
        }
        let icon_values = tray_icon_candidate_offsets()
            .iter()
            .filter_map(|offset| read_usize_at(bytes, *offset))
            .filter(|value| *value != 0)
            .collect::<Vec<_>>();

        if icon_values.is_empty() {
            return Err(
                "Explorer tray metadata did not contain an icon handle candidate".to_string(),
            );
        }

        Ok(Self { icon_values })
    }

    fn icon_handles(&self) -> impl Iterator<Item = HICON> + '_ {
        self.icon_values
            .iter()
            .copied()
            .map(|value| HICON(value as *mut _))
    }
}

fn tray_icon_candidate_offsets() -> &'static [usize] {
    // Explorer stores notification-item metadata behind TBBUTTON::dwData. The layout is private,
    // but the known Win7-Win11 shape places hIcon after HWND/uID/callback/state fields. Keep this
    // as a tiny compatibility shim: bounded read, candidate offsets only, CopyIcon validation, and
    // graceful fallback when no candidate is valid on a Windows build.
    #[cfg(target_pointer_width = "64")]
    {
        &[24, 32, 40]
    }
    #[cfg(target_pointer_width = "32")]
    {
        &[20, 24, 28]
    }
}

fn tray_metadata_read_bytes() -> usize {
    tray_icon_candidate_offsets()
        .iter()
        .copied()
        .max()
        .and_then(|offset| offset.checked_add(size_of::<usize>()))
        .unwrap_or(MAX_TRAY_METADATA_BYTES)
        .min(MAX_TRAY_METADATA_BYTES)
}

fn read_usize_at(bytes: &[u8], offset: usize) -> Option<usize> {
    let width = size_of::<usize>();
    let end = offset.checked_add(width)?;
    let raw = bytes.get(offset..end)?;
    let mut value = 0usize;
    for (index, byte) in raw.iter().enumerate() {
        value |= (*byte as usize) << (index * 8);
    }
    Some(value)
}

fn read_toolbar_button(
    toolbar: HWND,
    process: &RemoteProcess,
    index: i32,
) -> Result<Option<ToolbarButton>, String> {
    let remote_button = RemoteAllocation::new(process, size_of::<ToolbarButton>())?;
    let ok = send_toolbar_message(
        toolbar,
        TB_GETBUTTON,
        WPARAM(index as usize),
        LPARAM(remote_button.ptr as isize),
    )?;
    if ok == 0 {
        return Ok(None);
    }
    Ok(Some(process.read_struct(remote_button.ptr)?))
}

fn normalize_tray_button(button: ToolbarButton) -> Option<ToolbarButton> {
    if button.id_command <= 0 || button.fs_state & TBSTATE_HIDDEN != 0 {
        return None;
    }
    Some(button)
}

fn tray_toolbar_candidates() -> Vec<TrayToolbarCandidate> {
    let mut candidates = Vec::new();

    for shell_class in ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"] {
        for shell in top_level_windows_by_class(shell_class) {
            if let Some(tray_notify) = find_child_chain(shell, &["TrayNotifyWnd"]) {
                push_toolbar_descendants(
                    &mut candidates,
                    tray_notify,
                    ToolbarDiscoverySource::TrayNotifyArea,
                );
            }
        }
    }

    for overflow in top_level_windows_by_class("NotifyIconOverflowWindow") {
        push_toolbar_descendants(
            &mut candidates,
            overflow,
            ToolbarDiscoverySource::OverflowWindow,
        );
    }

    candidates
}

fn top_level_windows_by_class(class_name: &str) -> Vec<HWND> {
    let mut windows = Vec::new();
    let class_name = wide(class_name);
    let mut after = HWND(std::ptr::null_mut());
    loop {
        let next = unsafe {
            // SAFETY: FindWindowExW traverses top-level windows using a static class-name buffer
            // that remains alive for the loop; null title means any title.
            FindWindowExW(None, Some(after), class_name.as_pcwstr(), PCWSTR::null()).ok()
        };
        let Some(hwnd) = next else {
            break;
        };
        if hwnd.0.is_null() {
            break;
        }
        windows.push(hwnd);
        after = hwnd;
    }

    if windows.is_empty() {
        let fallback = unsafe {
            // SAFETY: FindWindowW only reads the top-level window tree using the same live class buffer.
            FindWindowW(class_name.as_pcwstr(), PCWSTR::null()).ok()
        };
        if let Some(hwnd) = fallback.filter(|hwnd| !hwnd.0.is_null()) {
            windows.push(hwnd);
        }
    }

    windows
}

fn find_child_chain(mut parent: HWND, classes: &[&str]) -> Option<HWND> {
    for class_name in classes {
        parent = unsafe {
            // SAFETY: FindWindowExW traverses child windows for static class names; null title means any title.
            FindWindowExW(
                Some(parent),
                None,
                wide(class_name).as_pcwstr(),
                PCWSTR::null(),
            )
            .ok()?
        };
        if parent.0.is_null() {
            return None;
        }
    }
    Some(parent)
}

fn push_toolbar_descendants(
    candidates: &mut Vec<TrayToolbarCandidate>,
    root: HWND,
    source: ToolbarDiscoverySource,
) {
    for hwnd in descendant_windows(root) {
        if window_class_name(hwnd).as_deref() != Some("ToolbarWindow32") {
            continue;
        }
        if candidates.iter().any(|candidate| candidate.hwnd == hwnd) {
            continue;
        }
        candidates.push(TrayToolbarCandidate { hwnd, source });
    }
}

fn descendant_windows(root: HWND) -> Vec<HWND> {
    let mut windows = Vec::new();
    unsafe {
        // SAFETY: EnumChildWindows synchronously enumerates descendants of an HWND owned by Explorer.
        // `windows` lives for the duration of the call, and the callback only pushes HWND values into it.
        let _ = EnumChildWindows(
            Some(root),
            Some(enum_child_window),
            LPARAM((&mut windows as *mut Vec<HWND>) as isize),
        );
    }
    windows
}

unsafe extern "system" fn enum_child_window(hwnd: HWND, lparam: LPARAM) -> windows::core::BOOL {
    let windows = unsafe {
        // SAFETY: lparam is the exact Vec<HWND> pointer supplied by descendant_windows and remains
        // valid until EnumChildWindows returns.
        &mut *(lparam.0 as *mut Vec<HWND>)
    };
    windows.push(hwnd);
    true.into()
}

fn window_class_name(hwnd: HWND) -> Option<String> {
    let mut buffer = [0_u16; 128];
    let length = unsafe {
        // SAFETY: buffer is a valid writable UTF-16 buffer and hwnd is only queried for its class name.
        GetClassNameW(hwnd, &mut buffer)
    };
    if length == 0 {
        None
    } else {
        Some(String::from_utf16_lossy(&buffer[..length as usize]))
    }
}

fn send_toolbar_message(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Result<usize, String> {
    let mut result = 0usize;
    let status = unsafe {
        // SAFETY: SendMessageTimeoutW is used with Explorer-owned HWNDs and a short timeout so a hung
        // Explorer toolbar cannot hang JasonShell's UI command indefinitely.
        SendMessageTimeoutW(
            hwnd,
            message,
            wparam,
            lparam,
            SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
            SEND_TIMEOUT_MS,
            Some(&mut result),
        )
    };
    if status.0 == 0 {
        Err(format!(
            "Explorer tray toolbar did not answer message 0x{message:x}"
        ))
    } else {
        Ok(result)
    }
}

fn merge_toolbar_snapshots(
    snapshots_by_toolbar: Vec<Vec<SystemTrayIconSnapshot>>,
) -> Vec<SystemTrayIconSnapshot> {
    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::new();
    for snapshot in snapshots_by_toolbar.into_iter().flatten() {
        if seen.insert(snapshot.id.clone()) {
            merged.push(snapshot);
        }
    }
    merged
}

fn snapshot_id(source: ToolbarDiscoverySource, toolbar: HWND, command_id: i32) -> String {
    format!(
        "{}:{}:{command_id}",
        source.id_segment(),
        toolbar.0 as isize
    )
}

struct ParsedSnapshotId {
    source: ToolbarDiscoverySource,
    toolbar_hwnd: isize,
    command_id: i32,
}

fn parse_snapshot_id(id: &str) -> Result<ParsedSnapshotId, String> {
    let parts = id.split(':').collect::<Vec<_>>();
    let [source, toolbar, command] = parts.as_slice() else {
        return Err("Invalid tray icon id".to_string());
    };
    Ok(ParsedSnapshotId {
        source: ToolbarDiscoverySource::from_id_segment(source)
            .ok_or_else(|| "Invalid tray toolbar source".to_string())?,
        toolbar_hwnd: toolbar
            .parse::<isize>()
            .map_err(|error| format!("Invalid tray toolbar handle: {error}"))?,
        command_id: command
            .parse::<i32>()
            .map_err(|error| format!("Invalid tray command id: {error}"))?,
    })
}

struct RemoteProcess {
    handle: HANDLE,
}

impl RemoteProcess {
    fn for_window(hwnd: HWND) -> Result<Self, String> {
        let mut process_id = 0u32;
        unsafe {
            // SAFETY: hwnd is an Explorer toolbar handle; the function only writes a process id to the local out-param.
            let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        }
        if process_id == 0 {
            return Err("Explorer tray toolbar process id was unavailable".to_string());
        }
        let handle = unsafe {
            // SAFETY: Opening Explorer with VM read/write/operation rights is required for documented toolbar
            // cross-process message buffers; the handle is closed in Drop.
            OpenProcess(
                PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
                false,
                process_id,
            )
        }
        .map_err(|error| format!("Failed to open Explorer tray process: {error}"))?;
        Ok(Self { handle })
    }

    fn read_struct<T: Copy>(&self, remote_ptr: *mut c_void) -> Result<T, String> {
        // SAFETY: `value` is immediately overwritten by `ReadProcessMemory`; `T` is only used
        // with plain-old-data Win32 structs in this module and the completed byte count is checked.
        let mut value: T = unsafe { zeroed() };
        let mut bytes_read = 0usize;
        unsafe {
            // SAFETY: remote_ptr points to a buffer allocated in this process handle by RemoteAllocation.
            // T is plain toolbar/RECT data and bytes_read is validated against size_of::<T>().
            ReadProcessMemory(
                self.handle,
                remote_ptr,
                (&mut value as *mut T).cast(),
                size_of::<T>(),
                Some(&mut bytes_read),
            )
            .map_err(|error| format!("Failed to read Explorer tray memory: {error}"))?;
        }
        if bytes_read != size_of::<T>() {
            return Err("Explorer tray memory read was incomplete".to_string());
        }
        Ok(value)
    }

    fn read_bytes(&self, remote_ptr: *const c_void, bytes: usize) -> Result<Vec<u8>, String> {
        if remote_ptr.is_null() || bytes == 0 || bytes > MAX_TRAY_METADATA_BYTES {
            return Err("Explorer tray metadata read size was invalid".to_string());
        }
        let mut buffer = vec![0_u8; bytes];
        let mut bytes_read = 0usize;
        unsafe {
            // SAFETY: remote_ptr is an address read from Explorer toolbar metadata, never
            // dereferenced locally. ReadProcessMemory copies at most MAX_TRAY_METADATA_BYTES into a
            // local Vec, and the exact byte count is validated before parsing.
            ReadProcessMemory(
                self.handle,
                remote_ptr,
                buffer.as_mut_ptr().cast(),
                bytes,
                Some(&mut bytes_read),
            )
            .map_err(|error| format!("Failed to read Explorer tray metadata: {error}"))?;
        }
        if bytes_read != bytes {
            return Err("Explorer tray metadata read was incomplete".to_string());
        }
        Ok(buffer)
    }
}

impl Drop for RemoteProcess {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: handle was returned by OpenProcess and is owned by this RAII wrapper.
            let _ = CloseHandle(self.handle);
        }
    }
}

struct RemoteAllocation<'a> {
    process: &'a RemoteProcess,
    ptr: *mut c_void,
}

impl<'a> RemoteAllocation<'a> {
    fn new(process: &'a RemoteProcess, bytes: usize) -> Result<Self, String> {
        let ptr = unsafe {
            // SAFETY: Allocates a small read/write buffer in Explorer for toolbar messages. The pointer
            // is released in Drop with MEM_RELEASE.
            VirtualAllocEx(
                process.handle,
                None,
                bytes,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        if ptr.is_null() {
            return Err("Failed to allocate Explorer tray message buffer".to_string());
        }
        Ok(Self { process, ptr })
    }
}

impl Drop for RemoteAllocation<'_> {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: ptr was returned by VirtualAllocEx for this process; size must be 0 with MEM_RELEASE.
            let _ = VirtualFreeEx(self.process.handle, self.ptr, 0, MEM_RELEASE);
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
    WideString(value.encode_utf16().chain(std::iter::once(0)).collect())
}

#[cfg(test)]
#[derive(Clone, Debug)]
struct WindowTreeNode {
    class_name: &'static str,
    hwnd: HWND,
    _visible: bool,
    children: Vec<WindowTreeNode>,
}

#[cfg(test)]
impl WindowTreeNode {
    fn new(class_name: &'static str, hwnd: isize, visible: bool, children: Vec<Self>) -> Self {
        Self {
            class_name,
            hwnd: HWND(hwnd as *mut _),
            _visible: visible,
            children,
        }
    }
}

#[cfg(test)]
fn collect_toolbar_candidates_from_tree(root: &WindowTreeNode) -> Vec<TrayToolbarCandidate> {
    let mut candidates = Vec::new();
    collect_tree_sources(root, &mut candidates);
    candidates
}

#[cfg(test)]
fn collect_tree_sources(node: &WindowTreeNode, candidates: &mut Vec<TrayToolbarCandidate>) {
    match node.class_name {
        "TrayNotifyWnd" => {
            push_tree_toolbar_descendants(candidates, node, ToolbarDiscoverySource::TrayNotifyArea)
        }
        "NotifyIconOverflowWindow" => {
            push_tree_toolbar_descendants(candidates, node, ToolbarDiscoverySource::OverflowWindow)
        }
        _ => {}
    }

    for child in &node.children {
        collect_tree_sources(child, candidates);
    }
}

#[cfg(test)]
fn push_tree_toolbar_descendants(
    candidates: &mut Vec<TrayToolbarCandidate>,
    node: &WindowTreeNode,
    source: ToolbarDiscoverySource,
) {
    for child in &node.children {
        if child.class_name == "ToolbarWindow32" {
            candidates.push(TrayToolbarCandidate {
                hwnd: child.hwnd,
                source,
            });
        }
        push_tree_toolbar_descendants(candidates, child, source);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        collect_toolbar_candidates_from_tree, merge_toolbar_snapshots, normalize_tray_button,
        parse_snapshot_id, read_usize_at, snapshot_id, tray_icon_candidate_offsets,
        tray_icon_placeholder_data_url, tray_metadata_read_bytes, SystemTrayIconSnapshot,
        ToolbarButton, ToolbarDiscoverySource, TrayMetadata, WindowTreeNode,
        MAX_TRAY_METADATA_BYTES, TBSTATE_HIDDEN,
    };
    use windows::Win32::Foundation::HWND;

    #[test]
    fn snapshot_id_round_trips_toolbar_and_command() {
        let id = snapshot_id(
            ToolbarDiscoverySource::TrayNotifyArea,
            HWND(1234isize as *mut _),
            44,
        );
        let parsed = parse_snapshot_id(&id).expect("valid id");
        assert_eq!(parsed.source, ToolbarDiscoverySource::TrayNotifyArea);
        assert_eq!(parsed.toolbar_hwnd, 1234);
        assert_eq!(parsed.command_id, 44);
    }

    #[test]
    fn invalid_snapshot_ids_are_rejected_before_relay() {
        assert!(parse_snapshot_id("not-a-snapshot-id").is_err());
        assert!(parse_snapshot_id("abc:44").is_err());
        assert!(parse_snapshot_id("1234:not-a-command").is_err());
    }

    #[test]
    fn tray_discovery_collects_visible_and_overflow_toolbar_candidates() {
        let tree = WindowTreeNode::new(
            "Desktop",
            1,
            true,
            vec![
                WindowTreeNode::new(
                    "Shell_TrayWnd",
                    10,
                    true,
                    vec![WindowTreeNode::new(
                        "TrayNotifyWnd",
                        20,
                        true,
                        vec![
                            WindowTreeNode::new(
                                "SysPager",
                                30,
                                true,
                                vec![WindowTreeNode::new("ToolbarWindow32", 40, true, vec![])],
                            ),
                            WindowTreeNode::new("ToolbarWindow32", 41, true, vec![]),
                            WindowTreeNode::new("ToolbarWindow32", 42, false, vec![]),
                        ],
                    )],
                ),
                WindowTreeNode::new(
                    "NotifyIconOverflowWindow",
                    50,
                    true,
                    vec![WindowTreeNode::new("ToolbarWindow32", 60, true, vec![])],
                ),
            ],
        );

        let candidates = collect_toolbar_candidates_from_tree(&tree);
        let handles = candidates
            .iter()
            .map(|candidate| candidate.hwnd.0 as isize)
            .collect::<Vec<_>>();

        assert_eq!(handles, vec![40, 41, 42, 60]);
        assert_eq!(candidates[0].source, ToolbarDiscoverySource::TrayNotifyArea);
        assert_eq!(candidates[1].source, ToolbarDiscoverySource::TrayNotifyArea);
        assert_eq!(candidates[2].source, ToolbarDiscoverySource::TrayNotifyArea);
        assert_eq!(candidates[3].source, ToolbarDiscoverySource::OverflowWindow);
    }

    #[test]
    fn tray_discovery_does_not_drop_toolbar_source_for_synthetic_visibility() {
        let tree = WindowTreeNode::new(
            "Desktop",
            1,
            true,
            vec![WindowTreeNode::new(
                "Shell_TrayWnd",
                10,
                false,
                vec![WindowTreeNode::new(
                    "TrayNotifyWnd",
                    20,
                    false,
                    vec![WindowTreeNode::new(
                        "SysPager",
                        30,
                        false,
                        vec![WindowTreeNode::new("ToolbarWindow32", 40, false, vec![])],
                    )],
                )],
            )],
        );

        let candidates = collect_toolbar_candidates_from_tree(&tree);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].hwnd.0 as isize, 40);
        assert_eq!(candidates[0].source, ToolbarDiscoverySource::TrayNotifyArea);
    }

    #[test]
    fn tray_discovery_includes_secondary_taskbar_sources() {
        let tree = WindowTreeNode::new(
            "Desktop",
            1,
            true,
            vec![
                WindowTreeNode::new(
                    "Shell_TrayWnd",
                    10,
                    true,
                    vec![WindowTreeNode::new(
                        "TrayNotifyWnd",
                        20,
                        true,
                        vec![WindowTreeNode::new("ToolbarWindow32", 40, true, vec![])],
                    )],
                ),
                WindowTreeNode::new(
                    "Shell_SecondaryTrayWnd",
                    11,
                    true,
                    vec![WindowTreeNode::new(
                        "TrayNotifyWnd",
                        21,
                        true,
                        vec![WindowTreeNode::new("ToolbarWindow32", 41, true, vec![])],
                    )],
                ),
            ],
        );

        let candidates = collect_toolbar_candidates_from_tree(&tree);
        let handles = candidates
            .iter()
            .map(|candidate| candidate.hwnd.0 as isize)
            .collect::<Vec<_>>();

        assert_eq!(handles, vec![40, 41]);
    }

    #[test]
    #[ignore = "live desktop diagnostic; depends on Explorer tray state"]
    fn live_tray_snapshot_is_not_empty_when_explorer_exposes_toolbar_buttons() {
        let candidates = super::tray_toolbar_candidates();
        if candidates.is_empty() {
            return;
        }

        let snapshots = super::list_system_tray_icons().expect("live tray snapshot succeeds");
        assert!(
            !snapshots.is_empty(),
            "Explorer exposed {} tray toolbar source(s), but JasonShell returned no tray snapshots",
            candidates.len()
        );
    }

    #[test]
    fn tray_snapshot_ids_include_source_and_toolbar_identity() {
        let visible_id = snapshot_id(
            ToolbarDiscoverySource::TrayNotifyArea,
            HWND(100isize as *mut _),
            44,
        );
        let overflow_id = snapshot_id(
            ToolbarDiscoverySource::OverflowWindow,
            HWND(100isize as *mut _),
            44,
        );

        assert_ne!(visible_id, overflow_id);

        let parsed = parse_snapshot_id(&overflow_id).expect("overflow id parses");
        assert_eq!(parsed.source, ToolbarDiscoverySource::OverflowWindow);
        assert_eq!(parsed.toolbar_hwnd, 100);
        assert_eq!(parsed.command_id, 44);
    }

    #[test]
    fn tray_snapshot_merge_preserves_distinct_sources_with_similar_labels() {
        let visible = SystemTrayIconSnapshot {
            id: snapshot_id(
                ToolbarDiscoverySource::TrayNotifyArea,
                HWND(100isize as *mut _),
                7,
            ),
            command_id: 7,
            index: 0,
            label: "Steam".to_string(),
            icon_data_url: tray_icon_placeholder_data_url(),
            has_native_icon: false,
        };
        let overflow = SystemTrayIconSnapshot {
            id: snapshot_id(
                ToolbarDiscoverySource::OverflowWindow,
                HWND(200isize as *mut _),
                7,
            ),
            command_id: 7,
            index: 0,
            label: "Steam".to_string(),
            icon_data_url: tray_icon_placeholder_data_url(),
            has_native_icon: false,
        };

        let merged = merge_toolbar_snapshots(vec![vec![visible.clone()], vec![overflow.clone()]]);
        assert_eq!(merged, vec![visible, overflow]);
    }

    #[test]
    fn tray_snapshots_serialize_absent_native_icons_explicitly() {
        let snapshot = SystemTrayIconSnapshot {
            id: snapshot_id(
                ToolbarDiscoverySource::TrayNotifyArea,
                HWND(1234isize as *mut _),
                44,
            ),
            command_id: 44,
            index: 0,
            label: "Notification area icon 1".to_string(),
            icon_data_url: tray_icon_placeholder_data_url(),
            has_native_icon: false,
        };
        let serialized = serde_json::to_value(snapshot).expect("snapshot serializes");
        assert_eq!(serialized["iconDataUrl"], tray_icon_placeholder_data_url());
        assert_eq!(serialized["hasNativeIcon"], false);
    }

    #[test]
    fn tray_snapshots_serialize_present_native_icons_explicitly() {
        let snapshot = SystemTrayIconSnapshot {
            id: snapshot_id(
                ToolbarDiscoverySource::TrayNotifyArea,
                HWND(1234isize as *mut _),
                44,
            ),
            command_id: 44,
            index: 0,
            label: "Notification area icon 1".to_string(),
            icon_data_url: "data:image/png;base64,native".to_string(),
            has_native_icon: true,
        };
        let serialized = serde_json::to_value(snapshot).expect("snapshot serializes");
        assert_eq!(serialized["iconDataUrl"], "data:image/png;base64,native");
        assert_eq!(serialized["hasNativeIcon"], true);
    }

    #[test]
    fn tray_button_normalization_excludes_hidden_or_invalid_commands() {
        let visible = ToolbarButton {
            id_command: 42,
            ..ToolbarButton::default()
        };
        assert_eq!(normalize_tray_button(visible).unwrap().id_command, 42);

        let hidden = ToolbarButton {
            id_command: 42,
            fs_state: TBSTATE_HIDDEN,
            ..ToolbarButton::default()
        };
        assert!(normalize_tray_button(hidden).is_none());

        let invalid = ToolbarButton {
            id_command: 0,
            ..ToolbarButton::default()
        };
        assert!(normalize_tray_button(invalid).is_none());
    }

    #[test]
    fn tray_metadata_parser_extracts_bounded_icon_candidates() {
        let mut bytes = vec![0_u8; MAX_TRAY_METADATA_BYTES];
        let expected = 0x1234_5678usize;
        let offset = tray_icon_candidate_offsets()[0];
        write_usize_at(&mut bytes, offset, expected);

        let metadata = TrayMetadata::parse(&bytes).expect("metadata parses");
        assert_eq!(metadata.icon_values[0], expected);
        assert_eq!(read_usize_at(&bytes, offset), Some(expected));
    }

    #[test]
    fn tray_metadata_read_size_covers_candidates_without_unbounded_reads() {
        let max_candidate_end = tray_icon_candidate_offsets()
            .iter()
            .map(|offset| offset + std::mem::size_of::<usize>())
            .max()
            .expect("candidate offsets are present");

        assert!(tray_metadata_read_bytes() >= max_candidate_end);
        assert!(tray_metadata_read_bytes() <= MAX_TRAY_METADATA_BYTES);
    }

    #[test]
    fn tray_metadata_parser_rejects_empty_or_oversized_input() {
        let empty = vec![0_u8; MAX_TRAY_METADATA_BYTES];
        assert!(TrayMetadata::parse(&empty).is_err());

        let oversized = vec![1_u8; MAX_TRAY_METADATA_BYTES + 1];
        assert!(TrayMetadata::parse(&oversized).is_err());
    }

    fn write_usize_at(bytes: &mut [u8], offset: usize, value: usize) {
        for index in 0..std::mem::size_of::<usize>() {
            bytes[offset + index] = ((value >> (index * 8)) & 0xff) as u8;
        }
    }
}
