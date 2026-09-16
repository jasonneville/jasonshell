//! Narrow Win32 FFI for P02. Pointers remain owned until overlapped completion.
//! Oplock refusal is observable evidence, never replaced with metadata polling.
use super::contract::ProtocolErrorCode;
use super::{failure, io_failure, Result};
use std::{
    ffi::c_void,
    fs::File,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

type Handle = *mut c_void;
pub(crate) const READ: u32 = 0x8000_0000;
pub(crate) const WRITE: u32 = 0x4000_0000;
pub(crate) const SHARE_READ: u32 = 1;
pub(crate) const SHARE_WRITE: u32 = 2;
pub(crate) const SHARE_DELETE: u32 = 4;
pub(crate) const OVERLAPPED: u32 = 0x4000_0000;
pub(crate) const OPEN_REPARSE: u32 = 0x0020_0000;
pub(crate) const BACKUP: u32 = 0x0200_0000;
const IO_PENDING: i32 = 997;

#[repr(C)]
#[derive(Default)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: Handle,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct FileInfo {
    pub attributes: u32,
    pub creation: [u32; 2],
    pub access: [u32; 2],
    pub write: [u32; 2],
    pub volume: u32,
    pub size_high: u32,
    pub size_low: u32,
    pub links: u32,
    pub id_high: u32,
    pub id_low: u32,
}
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct FileAttributeTagInfo {
    attributes: u32,
    reparse_tag: u32,
}
#[repr(C)]
#[derive(Default)]
struct OplockInput {
    version: u16,
    length: u16,
    level: u32,
    flags: u32,
}
#[repr(C)]
#[derive(Default)]
struct OplockOutput {
    version: u16,
    length: u16,
    original: u32,
    new_level: u32,
    flags: u32,
    access: u32,
    share: u16,
}
#[repr(C)]
struct SecurityAttributes {
    length: u32,
    descriptor: *mut c_void,
    inherit: i32,
}
#[repr(C)]
struct Blob {
    length: u32,
    data: *mut u8,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *const SecurityAttributes,
        disposition: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    fn GetFileInformationByHandle(file: Handle, info: *mut FileInfo) -> i32;
    fn GetFileInformationByHandleEx(file: Handle, class: i32, info: *mut c_void, size: u32) -> i32;
    fn GetFileType(file: Handle) -> u32;
    fn GetDriveTypeW(root: *const u16) -> u32;
    fn GetVolumeInformationByHandleW(
        file: Handle,
        name: *mut u16,
        name_size: u32,
        serial: *mut u32,
        component: *mut u32,
        flags: *mut u32,
        fs: *mut u16,
        fs_size: u32,
    ) -> i32;
    fn GetFinalPathNameByHandleW(file: Handle, path: *mut u16, length: u32, flags: u32) -> u32;
    fn ReadFile(
        file: Handle,
        buffer: *mut u8,
        size: u32,
        read: *mut u32,
        overlapped: *mut Overlapped,
    ) -> i32;
    fn CreateEventW(security: *const c_void, manual: i32, initial: i32, name: *const u16)
        -> Handle;
    fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;
    fn GetOverlappedResult(
        file: Handle,
        overlapped: *mut Overlapped,
        bytes: *mut u32,
        wait: i32,
    ) -> i32;
    fn CancelIoEx(file: Handle, overlapped: *mut Overlapped) -> i32;
    fn DeviceIoControl(
        file: Handle,
        code: u32,
        input: *const c_void,
        input_size: u32,
        output: *mut c_void,
        output_size: u32,
        bytes: *mut u32,
        overlapped: *mut Overlapped,
    ) -> i32;
    fn GetDiskFreeSpaceExW(
        path: *const u16,
        available: *mut u64,
        total: *mut u64,
        free: *mut u64,
    ) -> i32;
    fn CreateDirectoryW(path: *const u16, security: *const SecurityAttributes) -> i32;
    fn LocalFree(memory: *mut c_void) -> *mut c_void;
    fn GetCurrentProcess() -> Handle;
    fn CreateFileMappingW(
        file: Handle,
        security: *const c_void,
        protection: u32,
        high: u32,
        low: u32,
        name: *const u16,
    ) -> Handle;
    fn MapViewOfFile(
        mapping: Handle,
        access: u32,
        high: u32,
        low: u32,
        bytes: usize,
    ) -> *mut c_void;
    fn UnmapViewOfFile(base: *const c_void) -> i32;
    fn FlushViewOfFile(base: *const c_void, bytes: usize) -> i32;
}
#[link(name = "advapi32")]
extern "system" {
    fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        text: *const u16,
        revision: u32,
        descriptor: *mut *mut c_void,
        size: *mut u32,
    ) -> i32;
    fn OpenProcessToken(process: Handle, access: u32, token: *mut Handle) -> i32;
    fn GetTokenInformation(
        token: Handle,
        class: i32,
        info: *mut c_void,
        length: u32,
        required: *mut u32,
    ) -> i32;
    fn ConvertSidToStringSidW(sid: *const c_void, text: *mut *mut u16) -> i32;
    fn GetNamedSecurityInfoW(
        object: *const u16,
        object_type: u32,
        security_info: u32,
        owner: *mut *mut c_void,
        group: *mut *mut c_void,
        dacl: *mut *mut c_void,
        sacl: *mut *mut c_void,
        descriptor: *mut *mut c_void,
    ) -> u32;
    fn ConvertSecurityDescriptorToStringSecurityDescriptorW(
        descriptor: *const c_void,
        revision: u32,
        security_info: u32,
        text: *mut *mut u16,
        length: *mut u32,
    ) -> i32;
}
#[link(name = "crypt32")]
extern "system" {
    fn CryptProtectData(
        input: *const Blob,
        description: *const u16,
        entropy: *const Blob,
        reserved: *const c_void,
        prompt: *const c_void,
        flags: u32,
        output: *mut Blob,
    ) -> i32;
    fn CryptUnprotectData(
        input: *const Blob,
        description: *mut *mut u16,
        entropy: *const Blob,
        reserved: *const c_void,
        prompt: *const c_void,
        flags: u32,
        output: *mut Blob,
    ) -> i32;
}
#[link(name = "bcrypt")]
extern "system" {
    fn BCryptGenRandom(algorithm: Handle, buffer: *mut u8, length: u32, flags: u32) -> i32;
}

fn os_error() -> super::contract::ProtocolError {
    io_failure(std::io::Error::last_os_error())
}
pub(crate) fn wide(path: &Path) -> Result<Vec<u16>> {
    let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
    if value.len() > 32760 || value.contains(&0) {
        return Err(failure(
            ProtocolErrorCode::UnsupportedTarget,
            "invalid path",
        ));
    }
    value.push(0);
    Ok(value)
}
fn owned(handle: Handle) -> Result<OwnedHandle> {
    if handle.is_null() || handle as isize == -1 {
        return Err(os_error());
    }
    // SAFETY: Win32 returned a new owned handle; OwnedHandle closes exactly once.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}
pub(crate) fn open(path: &Path, access: u32, share: u32, flags: u32) -> Result<File> {
    let name = wide(path)?;
    // SAFETY: NUL-terminated path; no inheritance; OPEN_EXISTING cannot create/truncate.
    let handle = unsafe {
        CreateFileW(
            name.as_ptr(),
            access,
            share,
            ptr::null(),
            3,
            flags,
            ptr::null_mut(),
        )
    };
    Ok(File::from(owned(handle)?))
}
pub(crate) fn info(file: &File) -> Result<FileInfo> {
    let mut value = FileInfo::default();
    // SAFETY: correctly sized writable structure, live file handle.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut value) } == 0 {
        return Err(os_error());
    }
    Ok(value)
}
pub(crate) fn attribute_tag(file: &File) -> Result<(u32, u32)> {
    let mut value = FileAttributeTagInfo::default();
    // SAFETY: FILE_ATTRIBUTE_TAG_INFO = class 9 and exact C layout.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            9,
            &mut value as *mut _ as *mut c_void,
            std::mem::size_of::<FileAttributeTagInfo>() as u32,
        )
    } == 0
    {
        return Err(os_error());
    }
    Ok((value.attributes, value.reparse_tag))
}
pub(crate) fn file_id(file: &File) -> Result<(u64, [u8; 16])> {
    #[repr(C)]
    struct Id {
        volume: u64,
        id: [u8; 16],
    }
    let mut value = Id {
        volume: 0,
        id: [0; 16],
    };
    // SAFETY: FILE_ID_INFO = class 18 and exact C layout.
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            18,
            &mut value as *mut _ as *mut c_void,
            std::mem::size_of::<Id>() as u32,
        )
    } == 0
    {
        return Err(os_error());
    }
    Ok((value.volume, value.id))
}
pub(crate) fn regular_ntfs(file: &File, root: &Path) -> Result<()> {
    let root = wide(root)?;
    let mut fs = [0u16; 32];
    // SAFETY: fixed-size filesystem name output; null optional outputs are supported.
    let valid = unsafe {
        GetFileType(file.as_raw_handle()) == 1
            && GetDriveTypeW(root.as_ptr()) == 3
            && GetVolumeInformationByHandleW(
                file.as_raw_handle(),
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                fs.as_mut_ptr(),
                fs.len() as u32,
            ) != 0
    };
    if !valid || &fs[..5] != [b'N' as u16, b'T' as u16, b'F' as u16, b'S' as u16, 0] {
        return Err(failure(
            ProtocolErrorCode::UnsupportedTarget,
            "requires fixed local NTFS disk file",
        ));
    }
    Ok(())
}
pub(crate) fn final_path(file: &File) -> Result<Vec<u16>> {
    let mut path = vec![0u16; 32768];
    // SAFETY: bounded writable buffer; normalized DOS name.
    let count = unsafe {
        GetFinalPathNameByHandleW(
            file.as_raw_handle(),
            path.as_mut_ptr(),
            path.len() as u32,
            0,
        )
    } as usize;
    if count == 0 {
        return Err(os_error());
    }
    if count >= path.len() {
        return Err(failure(
            ProtocolErrorCode::ResourceLimit,
            "authoritative path exceeds bound",
        ));
    }
    path.truncate(count);
    Ok(path)
}
fn event() -> Result<OwnedHandle> {
    // SAFETY: unnamed manual-reset event, non-inheritable.
    owned(unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) })
}
pub(crate) fn read_at(
    file: &File,
    offset: u64,
    buffer: &mut [u8],
    cancel: &AtomicBool,
) -> Result<usize> {
    if buffer.len() > super::contract::IO_BLOCK_BYTES {
        return Err(failure(
            ProtocolErrorCode::ResourceLimit,
            "read exceeds I/O block",
        ));
    }
    if cancel.load(Ordering::Acquire) {
        return Err(failure(
            ProtocolErrorCode::Cancelled,
            "read cancelled before submission",
        ));
    }
    let event = event()?;
    let mut ov = Overlapped {
        offset: offset as u32,
        offset_high: (offset >> 32) as u32,
        event: event.as_raw_handle(),
        ..Default::default()
    };
    let mut bytes = 0;
    // SAFETY: buffer and OVERLAPPED live until GetOverlappedResult confirms completion.
    let started = unsafe {
        ReadFile(
            file.as_raw_handle(),
            buffer.as_mut_ptr(),
            buffer.len() as u32,
            &mut bytes,
            &mut ov,
        )
    };
    if started == 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(38) {
            return Ok(0);
        }
        if error.raw_os_error() != Some(IO_PENDING) {
            return Err(io_failure(error));
        }
        loop {
            // SAFETY: event belongs to this request.
            let waited = unsafe { WaitForSingleObject(event.as_raw_handle(), 5) };
            if waited == 0 {
                break;
            }
            if waited == u32::MAX || cancel.load(Ordering::Acquire) {
                // Cancellation is only requested here; completion below is authoritative.
                unsafe {
                    CancelIoEx(file.as_raw_handle(), &mut ov);
                }
                break;
            }
        }
        if unsafe { GetOverlappedResult(file.as_raw_handle(), &mut ov, &mut bytes, 1) } == 0 {
            return Err(os_error());
        }
    }
    if cancel.load(Ordering::Acquire) {
        return Err(failure(
            ProtocolErrorCode::Cancelled,
            "read completed after cancellation request",
        ));
    }
    Ok(bytes as usize)
}

pub(crate) struct Oplock {
    file: File,
    event: OwnedHandle,
    overlapped: Box<std::cell::UnsafeCell<Overlapped>>,
    output: Box<std::cell::UnsafeCell<OplockOutput>>,
}
// SAFETY: buffers are heap-stable and exclusively written by the kernel. Rust
// never reads them while pending; shared calls only query the event. Drop owns
// the final reference, cancels and drains before freeing either UnsafeCell.
unsafe impl Send for Oplock {}
unsafe impl Sync for Oplock {}
impl Oplock {
    pub(crate) fn acquire(file: &File) -> Result<Self> {
        Self::request(file, 3)
    }
    pub(crate) fn acquire_read(file: &File) -> Result<Self> {
        // Share denial owns write/delete exclusion. A Read break is advisory and
        // needs no acknowledgement, so a conflicting synchronous open cannot
        // deadlock against this same process's guard.
        Self::request(file, 1)
    }
    fn request(file: &File, level: u32) -> Result<Self> {
        let file = file.try_clone().map_err(io_failure)?;
        let event = event()?;
        let overlapped = Box::new(std::cell::UnsafeCell::new(Overlapped {
            event: event.as_raw_handle(),
            ..Default::default()
        }));
        let output = Box::new(std::cell::UnsafeCell::new(OplockOutput::default()));
        let input = OplockInput {
            version: 1,
            length: 12,
            level,
            flags: 1,
        };
        // SAFETY: request buffers heap-stable until Drop cancels AND drains this request.
        let result = unsafe {
            DeviceIoControl(
                file.as_raw_handle(),
                0x0009_0240,
                &input as *const _ as *const c_void,
                12,
                output.get() as *mut c_void,
                std::mem::size_of::<OplockOutput>() as u32,
                ptr::null_mut(),
                overlapped.get(),
            )
        };
        let error = std::io::Error::last_os_error();
        if result != 0 || error.raw_os_error() != Some(IO_PENDING) {
            return Err(failure(
                ProtocolErrorCode::SharingViolation,
                format!(
                    "oplock level {level} not granted (os={:?}); immutable-source gate refused",
                    error.raw_os_error()
                ),
            ));
        }
        Ok(Self {
            file,
            event,
            overlapped,
            output,
        })
    }
    pub(crate) fn intact(&self) -> Result<()> {
        // SAFETY: event is live throughout this object.
        if unsafe { WaitForSingleObject(self.event.as_raw_handle(), 0) } != 258 {
            return Err(failure(
                ProtocolErrorCode::SourceChanged,
                "source oplock broke; no immutable handoff",
            ));
        }
        Ok(())
    }
}
impl Drop for Oplock {
    fn drop(&mut self) {
        let mut bytes = 0;
        // SAFETY: never release pending kernel output/OVERLAPPED memory before completion.
        unsafe {
            CancelIoEx(self.file.as_raw_handle(), self.overlapped.get());
            GetOverlappedResult(
                self.file.as_raw_handle(),
                self.overlapped.get(),
                &mut bytes,
                1,
            );
        }
        let _ = &self.output;
    }
}

pub(crate) fn random<const N: usize>() -> Result<[u8; N]> {
    let mut bytes = [0; N];
    // SAFETY: system preferred RNG, exactly N writable bytes.
    if unsafe { BCryptGenRandom(ptr::null_mut(), bytes.as_mut_ptr(), N as u32, 2) } < 0 {
        return Err(failure(ProtocolErrorCode::IoFailure, "system RNG failed"));
    }
    Ok(bytes)
}
pub(crate) fn free_space(path: &Path) -> Result<u64> {
    let path = wide(path)?;
    let mut free = 0;
    if unsafe { GetDiskFreeSpaceExW(path.as_ptr(), &mut free, ptr::null_mut(), ptr::null_mut()) }
        == 0
    {
        return Err(os_error());
    }
    Ok(free)
}
pub(crate) fn private_directory(path: &Path) -> Result<()> {
    let sid_text = current_user_sid()?;
    let sddl: Vec<u16> = format!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;{sid_text})")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(os_error());
    }
    let name = wide(path)?;
    let security = SecurityAttributes {
        length: std::mem::size_of::<SecurityAttributes>() as u32,
        descriptor,
        inherit: 0,
    };
    let created = unsafe { CreateDirectoryW(name.as_ptr(), &security) };
    let error = std::io::Error::last_os_error();
    unsafe {
        LocalFree(descriptor);
    }
    if created == 0 {
        return Err(io_failure(error));
    }
    Ok(())
}

pub(crate) fn current_user_sid() -> Result<String> {
    let mut raw_token = ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), 8, &mut raw_token) } == 0 {
        return Err(os_error());
    }
    let token = owned(raw_token)?;
    // TOKEN_USER contains a pointer into this aligned, bounded native buffer.
    let mut user = [0usize; 256];
    let mut required = 0;
    if unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            1,
            user.as_mut_ptr() as *mut c_void,
            std::mem::size_of_val(&user) as u32,
            &mut required,
        )
    } == 0
    {
        return Err(os_error());
    }
    let mut sid = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(user[0] as *const c_void, &mut sid) } == 0 {
        return Err(os_error());
    }
    let sid_text = unsafe {
        let mut length = 0;
        while length < 256 && *sid.add(length) != 0 {
            length += 1;
        }
        let text = String::from_utf16(std::slice::from_raw_parts(sid, length));
        LocalFree(sid as *mut c_void);
        text
    }
    .map_err(|_| failure(ProtocolErrorCode::IoFailure, "invalid current-user SID"))?;
    Ok(sid_text)
}

/// Test-only recovery ACL policy: owner=current user; protected DACL; full-control
/// directory ACEs only. Explicit allows are limited to user and SYSTEM. Inherited
/// user/SYSTEM/Builtin Administrators ACEs are tolerated; every other allow fails.
pub(crate) fn validate_private_recovery_sddl(sddl: &str, user_sid: &str) -> Result<()> {
    let owner = sddl
        .strip_prefix("O:")
        .and_then(|rest| rest.split_once("G:").map(|(owner, _)| owner));
    if owner != Some(user_sid) || !sddl.contains("D:P") {
        return Err(failure(
            ProtocolErrorCode::IoFailure,
            "recovery ACL policy mismatch",
        ));
    }

    let mut user_allow = false;
    let dacl = sddl
        .split_once("D:P")
        .map(|(_, dacl)| dacl)
        .unwrap_or_default();
    for body in dacl
        .split('(')
        .skip(1)
        .filter_map(|ace| ace.split_once(')').map(|v| v.0))
    {
        let fields: Vec<_> = body.split(';').collect();
        if fields.len() != 6 {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "recovery ACL policy mismatch",
            ));
        }
        if fields[0] == "D" {
            continue;
        }
        if fields[0] != "A" || fields[2] != "FA" {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "recovery ACL policy mismatch",
            ));
        }
        let inherited = fields[1].contains("ID");
        let flags_ok = fields[1] == "OICI" || fields[1] == "OICIID";
        let principal_ok =
            fields[5] == user_sid || fields[5] == "SY" || (inherited && fields[5] == "BA");
        if !flags_ok || !principal_ok {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "recovery ACL policy mismatch",
            ));
        }
        user_allow |= fields[5] == user_sid;
    }
    if !user_allow {
        return Err(failure(
            ProtocolErrorCode::IoFailure,
            "recovery ACL policy mismatch",
        ));
    }
    Ok(())
}

fn security_read_error() -> super::contract::ProtocolError {
    failure(
        ProtocolErrorCode::IoFailure,
        "security descriptor read failed",
    )
}

#[cfg(test)]
pub(crate) fn security_read_error_for_test(_status: u32) -> super::contract::ProtocolError {
    security_read_error()
}

/// Reads owner/group/DACL through Win32. Failures never disclose the input path.
pub(crate) fn security_sddl(path: &Path) -> Result<String> {
    const SE_FILE_OBJECT: u32 = 1;
    const OWNER_GROUP_DACL: u32 = 0x1 | 0x2 | 0x4;
    let name = wide(path)?;
    let mut descriptor = ptr::null_mut();
    let status = unsafe {
        GetNamedSecurityInfoW(
            name.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_GROUP_DACL,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if status != 0 {
        return Err(security_read_error());
    }
    let mut text = ptr::null_mut();
    let mut length = 0;
    let converted = unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            1,
            OWNER_GROUP_DACL,
            &mut text,
            &mut length,
        )
    };
    if converted == 0 {
        let error = std::io::Error::last_os_error();
        unsafe { LocalFree(descriptor) };
        return Err(failure(
            ProtocolErrorCode::IoFailure,
            format!(
                "security descriptor conversion failed (os={:?})",
                error.raw_os_error()
            ),
        ));
    }
    let units = unsafe { std::slice::from_raw_parts(text, length as usize) };
    let value = String::from_utf16(units.strip_suffix(&[0]).unwrap_or(units));
    unsafe {
        LocalFree(text as *mut c_void);
        LocalFree(descriptor);
    }
    value.map_err(|_| {
        failure(
            ProtocolErrorCode::IoFailure,
            "security descriptor encoding invalid",
        )
    })
}

pub(crate) fn set_write_time(file: &File, value: u64) -> Result<()> {
    #[link(name = "kernel32")]
    extern "system" {
        fn SetFileTime(
            file: Handle,
            creation: *const u64,
            access: *const u64,
            write: *const u64,
        ) -> i32;
    }
    if unsafe { SetFileTime(file.as_raw_handle(), ptr::null(), ptr::null(), &value) } == 0 {
        return Err(os_error());
    }
    Ok(())
}
pub(crate) fn protect_key(key: &[u8], unprotect: bool) -> Result<Vec<u8>> {
    if key.len() > 4096 {
        return Err(failure(
            ProtocolErrorCode::ResourceLimit,
            "protected key exceeds bound",
        ));
    }
    let input = Blob {
        length: key.len() as u32,
        data: key.as_ptr() as *mut u8,
    };
    let mut output = Blob {
        length: 0,
        data: ptr::null_mut(),
    };
    // SAFETY: DPAPI copies input, allocates output using LocalAlloc; no UI allowed.
    let ok = unsafe {
        if unprotect {
            CryptUnprotectData(
                &input,
                ptr::null_mut(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                1,
                &mut output,
            )
        } else {
            CryptProtectData(
                &input,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                ptr::null(),
                1,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err(os_error());
    }
    let bytes = unsafe {
        let data = std::slice::from_raw_parts(output.data, output.length as usize).to_vec();
        std::ptr::write_bytes(output.data, 0, output.length as usize);
        LocalFree(output.data as *mut c_void);
        data
    };
    Ok(bytes)
}

/// Synthetic-fixture-only mapped writer. Closing the originating File does not close this view.
pub(crate) struct WritableMap {
    base: *mut c_void,
    length: usize,
    _mapping: OwnedHandle,
}
impl WritableMap {
    pub(crate) fn create(file: &File, length: usize) -> Result<Self> {
        if length == 0 || length > 65536 {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "mapped experiment bound",
            ));
        }
        let mapping = owned(unsafe {
            CreateFileMappingW(file.as_raw_handle(), ptr::null(), 4, 0, 0, ptr::null())
        })?;
        let base = unsafe { MapViewOfFile(mapping.as_raw_handle(), 2, 0, 0, length) };
        if base.is_null() {
            return Err(os_error());
        }
        Ok(Self {
            base,
            length,
            _mapping: mapping,
        })
    }
    pub(crate) fn write(&mut self, offset: usize, byte: u8) -> Result<()> {
        if offset >= self.length {
            return Err(failure(
                ProtocolErrorCode::InvalidTextBoundary,
                "mapped experiment offset",
            ));
        }
        unsafe {
            std::ptr::write_volatile((self.base as *mut u8).add(offset), byte);
        }
        if unsafe { FlushViewOfFile(self.base, self.length) } == 0 {
            return Err(os_error());
        }
        Ok(())
    }
}
impl Drop for WritableMap {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.base);
        }
    }
}

#[repr(C)]
#[derive(Default)]
struct ProcessMemory {
    size: u32,
    faults: u32,
    peak_working: usize,
    working: usize,
    peak_paged_pool: usize,
    paged_pool: usize,
    peak_nonpaged_pool: usize,
    nonpaged_pool: usize,
    pagefile: usize,
    peak_pagefile: usize,
    private_bytes: usize,
}
#[repr(C)]
#[derive(Default)]
struct IoCounters {
    reads: u64,
    writes: u64,
    other: u64,
    read_bytes: u64,
    write_bytes: u64,
    other_bytes: u64,
}
#[link(name = "kernel32")]
extern "system" {
    fn K32GetProcessMemoryInfo(process: Handle, counters: *mut ProcessMemory, size: u32) -> i32;
    fn GetProcessIoCounters(process: Handle, counters: *mut IoCounters) -> i32;
    fn GetProcessHandleCount(process: Handle, count: *mut u32) -> i32;
    fn GetCurrentProcessId() -> u32;
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProcessSample {
    process_id: u32,
    private_bytes: u64,
    working_set: u64,
    peak_working_set: u64,
    peak_pagefile: u64,
    handles: u32,
    read_bytes: String,
    write_bytes: String,
}
pub(crate) fn process_sample() -> Result<ProcessSample> {
    let mut memory = ProcessMemory {
        size: std::mem::size_of::<ProcessMemory>() as u32,
        ..Default::default()
    };
    let mut io = IoCounters::default();
    let mut handles = 0;
    // SAFETY: correctly sized native structs and current-process pseudo handle.
    unsafe {
        if K32GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut memory,
            std::mem::size_of::<ProcessMemory>() as u32,
        ) == 0
            || GetProcessIoCounters(GetCurrentProcess(), &mut io) == 0
            || GetProcessHandleCount(GetCurrentProcess(), &mut handles) == 0
        {
            return Err(os_error());
        }
        Ok(ProcessSample {
            process_id: GetCurrentProcessId(),
            private_bytes: memory.private_bytes as u64,
            working_set: memory.working as u64,
            peak_working_set: memory.peak_working as u64,
            peak_pagefile: memory.peak_pagefile as u64,
            handles,
            read_bytes: io.read_bytes.to_string(),
            write_bytes: io.write_bytes.to_string(),
        })
    }
}
