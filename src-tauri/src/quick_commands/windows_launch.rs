//! Windows-only creation with a job assigned by the kernel before it returns.

use std::{
    ffi::{c_void, OsStr, OsString},
    io::{self, PipeReader, PipeWriter},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE, WAIT_OBJECT_0},
        System::{
            SystemInformation::{GetSystemDirectoryW, GetWindowsDirectoryW},
            Threading::{
                CreateProcessW, DeleteProcThreadAttributeList, GetCurrentProcess,
                GetExitCodeProcess, InitializeProcThreadAttributeList, ResumeThread,
                TerminateProcess, UpdateProcThreadAttribute, WaitForSingleObject, CREATE_NO_WINDOW,
                CREATE_SUSPENDED, EXTENDED_STARTUPINFO_PRESENT, INFINITE,
                LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_JOB_LIST,
                PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, STARTF_USESTDHANDLES, STARTUPINFOEXW,
                STARTUPINFOW,
            },
        },
    },
};

use super::QuickCommandSpawnPlan;

pub(super) struct Child {
    process: OwnedHandle,
    pub(super) id: u32,
    pub(super) stdin: PipeWriter,
    pub(super) stdout: PipeReader,
    pub(super) stderr: PipeReader,
}

impl Child {
    pub(super) fn process(&self) -> HANDLE {
        HANDLE(self.process.as_raw_handle())
    }
    pub(super) fn into_parts(self) -> (OwnedHandle, PipeWriter, PipeReader, PipeReader) {
        (self.process, self.stdin, self.stdout, self.stderr)
    }
}

pub(super) fn wait(process: &OwnedHandle) -> Result<i32, String> {
    let process = HANDLE(process.as_raw_handle());
    let mut code = 0;
    // SAFETY: the caller retains the actual process handle through this wait.
    if unsafe { WaitForSingleObject(process, INFINITE) } != WAIT_OBJECT_0 {
        return Err("Quick Command process wait failed".into());
    }
    unsafe { GetExitCodeProcess(process, &mut code) }
        .map_err(|_| "Quick Command exit status unavailable")?;
    Ok(code as i32)
}

pub(super) fn has_exited(process: &OwnedHandle) -> bool {
    // SAFETY: the retained process handle is live; this never blocks.
    (unsafe { WaitForSingleObject(HANDLE(process.as_raw_handle()), 0) }) == WAIT_OBJECT_0
}

pub(super) fn observe_job_members(job: HANDLE) -> Result<Vec<OwnedHandle>, String> {
    use windows::Win32::{
        Foundation::ERROR_MORE_DATA,
        System::{
            JobObjects::{IsProcessInJob, JobObjectBasicProcessIdList, QueryInformationJobObject},
            Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE},
        },
    };
    const UNAVAILABLE: &str = "Owned command process observation unavailable";
    // The Win32 list has two DWORDs followed by pointer-sized IDs.
    let header_words = (2 * size_of::<u32>()).div_ceil(size_of::<usize>());
    let mut capacity = 16;
    let (buffer, count) = loop {
        let mut buffer = vec![0usize; header_words + capacity];
        // SAFETY: the buffer is pointer-aligned and covers the passed byte size.
        let queried = unsafe {
            QueryInformationJobObject(
                Some(job),
                JobObjectBasicProcessIdList,
                buffer.as_mut_ptr().cast(),
                size_of_val(buffer.as_slice()) as u32,
                None,
            )
        };
        match queried {
            Ok(()) => {
                // SAFETY: the initialized buffer contains at least two DWORDs.
                let count = unsafe { buffer.as_ptr().cast::<u32>().add(1).read() } as usize;
                if count > capacity {
                    return Err(UNAVAILABLE.into());
                }
                break (buffer, count);
            }
            Err(error) if error.code() == ERROR_MORE_DATA.to_hresult() && capacity < 65536 => {
                capacity *= 2
            }
            Err(_) => return Err(UNAVAILABLE.into()),
        }
    };
    let mut observed = Vec::with_capacity(count);
    for &pid in &buffer[header_words..header_words + count] {
        let pid = u32::try_from(pid).map_err(|_| UNAVAILABLE)?;
        // PID is only a lookup key. Recheck ownership on the actual handle;
        // no PID-derived handle is ever used to terminate a process.
        let handle = match unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                false,
                pid,
            )
        } {
            Ok(handle) => unsafe { OwnedHandle::from_raw_handle(handle.0) },
            Err(_) => return Err(UNAVAILABLE.into()),
        };
        let mut member = windows::core::BOOL(0);
        // SAFETY: both handles are valid and member is a writable BOOL.
        unsafe { IsProcessInJob(HANDLE(handle.as_raw_handle()), Some(job), &mut member) }
            .map_err(|_| UNAVAILABLE)?;
        if !member.as_bool() {
            return Err(UNAVAILABLE.into());
        }
        observed.push(handle);
    }
    Ok(observed)
}

pub(super) fn resume(thread: &OwnedHandle) -> Result<(), String> {
    // SAFETY: the caller owns the primary thread of the contained process.
    if unsafe { ResumeThread(HANDLE(thread.as_raw_handle())) } != 1 {
        return Err("Quick Command process could not be resumed".into());
    }
    Ok(())
}

struct Attributes {
    list: LPPROC_THREAD_ATTRIBUTE_LIST,
    _storage: Vec<usize>,
}

impl Attributes {
    fn new(count: u32) -> windows::core::Result<Self> {
        let mut size = 0;
        // SAFETY: the documented sizing call is expected to fail, writing size.
        let _ = unsafe { InitializeProcThreadAttributeList(None, count, None, &mut size) };
        let mut storage = vec![0usize; size.div_ceil(size_of::<usize>())];
        let list = LPPROC_THREAD_ATTRIBUTE_LIST(storage.as_mut_ptr().cast());
        // SAFETY: pointer-aligned storage stays allocated for the list's lifetime.
        unsafe { InitializeProcThreadAttributeList(Some(list), count, None, &mut size) }?;
        Ok(Self {
            list,
            _storage: storage,
        })
    }
}

impl Drop for Attributes {
    fn drop(&mut self) {
        // SAFETY: this list was initialized and its allocation is still alive.
        unsafe { DeleteProcThreadAttributeList(self.list) };
    }
}

pub(super) fn create_suspended(
    plan: &QuickCommandSpawnPlan,
    job: HANDLE,
    holder_job: HANDLE,
) -> Result<(Child, OwnedHandle), String> {
    let exe = resolve_executable(&plan.executable)?;
    let (app, mut args) = launch_arguments(&exe, &plan.args)?;
    let cwd = plan
        .cwd
        .as_deref()
        .map(|s| wide(OsStr::new(s)))
        .transpose()?;
    let pipes = || -> io::Result<_> { Ok((io::pipe()?, io::pipe()?, io::pipe()?)) };
    let ((child_stdin, stdin), (stdout, child_stdout), (stderr, child_stderr)) =
        pipes().map_err(|_| "Quick Command pipes unavailable")?;
    // Never publish inheritable copies in JasonShell's own handle table. The
    // designated parent is a job-owned, never-resumed handle-only process.
    let holder = PrivateHandleTable::create(holder_job)?;
    let mut parent = [holder.handle()];
    let mut handles = [
        holder.inheritable(&child_stdin)?,
        holder.inheritable(&child_stdout)?,
        holder.inheritable(&child_stderr)?,
    ];
    let mut jobs = [job];
    let attrs = Attributes::new(3).map_err(|_| "Quick Command process attributes unavailable")?;
    // SAFETY: the typed arrays (and their owned handles) live until after the
    // attribute list is deleted. Only standard streams may be inherited.
    unsafe {
        for (key, values) in [
            (PROC_THREAD_ATTRIBUTE_HANDLE_LIST, handles.as_mut_slice()),
            (PROC_THREAD_ATTRIBUTE_JOB_LIST, jobs.as_mut_slice()),
            (PROC_THREAD_ATTRIBUTE_PARENT_PROCESS, parent.as_mut_slice()),
        ] {
            UpdateProcThreadAttribute(
                attrs.list,
                0,
                key as usize,
                Some(values.as_mut_ptr().cast::<c_void>()),
                size_of_val(values),
                None,
                None,
            )
            .map_err(|_| "Quick Command requires Windows process creation with job assignment")?;
        }
    }
    let startup = STARTUPINFOEXW {
        StartupInfo: STARTUPINFOW {
            cb: size_of::<STARTUPINFOEXW>() as u32,
            dwFlags: STARTF_USESTDHANDLES,
            hStdInput: handles[0],
            hStdOutput: handles[1],
            hStdError: handles[2],
            ..Default::default()
        },
        lpAttributeList: attrs.list,
    };
    let mut info = PROCESS_INFORMATION::default();
    #[cfg(test)]
    if let Some(callback) = super::BEFORE_CONTAINED_CREATE.with(std::cell::RefCell::take) {
        callback();
    }
    // SAFETY: both UTF-16 strings and the writable command-line buffer are
    // NUL-terminated; startup's first field is the Win32 base structure. No
    // unsupported-host or nested-job error is retried without the attributes.
    // Console images use the explicit pipes rather than console startup state
    // associated with the never-resumed designated parent.
    unsafe {
        CreateProcessW(
            PCWSTR(app.as_ptr()),
            Some(PWSTR(args.as_mut_ptr())),
            None,
            None,
            true,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_NO_WINDOW,
            None,
            cwd.as_ref()
                .map_or(PCWSTR::null(), |cwd| PCWSTR(cwd.as_ptr())),
            &startup.StartupInfo,
            &mut info,
        )
    }
    .map_err(|error| format!("Failed to create contained Quick Command: {error}"))?;
    // SAFETY: a successful CreateProcessW gives us ownership of both handles.
    let process = unsafe { OwnedHandle::from_raw_handle(info.hProcess.0) };
    let thread = unsafe { OwnedHandle::from_raw_handle(info.hThread.0) };
    Ok((
        Child {
            process,
            id: info.dwProcessId,
            stdin,
            stdout,
            stderr,
        },
        thread,
    ))
}

struct PrivateHandleTable(OwnedHandle);

impl PrivateHandleTable {
    fn handle(&self) -> HANDLE {
        HANDLE(self.0.as_raw_handle())
    }

    fn create(job: HANDLE) -> Result<Self, String> {
        let image =
            std::env::current_exe().map_err(|_| "Quick Command handle-table image unavailable")?;
        let image = wide(image.as_os_str())?;
        let mut jobs = [job];
        let attrs =
            Attributes::new(1).map_err(|_| "Quick Command handle-table attributes unavailable")?;
        // SAFETY: jobs stays live until after the attribute list is deleted.
        unsafe {
            UpdateProcThreadAttribute(
                attrs.list,
                0,
                PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                Some(jobs.as_mut_ptr().cast()),
                size_of_val(&jobs),
                None,
                None,
            )
        }
        .map_err(|_| "Quick Command handle-table job assignment unavailable")?;
        let startup = STARTUPINFOEXW {
            StartupInfo: STARTUPINFOW {
                cb: size_of::<STARTUPINFOEXW>() as u32,
                ..Default::default()
            },
            lpAttributeList: attrs.list,
        };
        let mut info = PROCESS_INFORMATION::default();
        // SAFETY: image is NUL terminated. The trusted image has no inherited
        // handles, starts suspended inside its tracked job, and is never resumed.
        unsafe {
            CreateProcessW(
                PCWSTR(image.as_ptr()),
                None,
                None,
                None,
                false,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_NO_WINDOW,
                None,
                PCWSTR::null(),
                &startup.StartupInfo,
                &mut info,
            )
        }
        .map_err(|error| format!("Failed to create Quick Command handle table: {error}"))?;
        // SAFETY: successful creation transfers both handles; immediately drop
        // the primary thread handle so this code cannot resume the holder.
        drop(unsafe { OwnedHandle::from_raw_handle(info.hThread.0) });
        Ok(Self(unsafe {
            OwnedHandle::from_raw_handle(info.hProcess.0)
        }))
    }

    fn inheritable(&self, source: &impl AsRawHandle) -> Result<HANDLE, String> {
        let mut handle = HANDLE::default();
        // SAFETY: both handle tables and the source handle are alive. The
        // returned value is remote; only the holder/target may use it.
        unsafe {
            DuplicateHandle(
                GetCurrentProcess(),
                HANDLE(source.as_raw_handle()),
                self.handle(),
                &mut handle,
                0,
                true,
                DUPLICATE_SAME_ACCESS,
            )
        }
        .map_err(|_| "Quick Command stream inheritance unavailable")?;
        Ok(handle)
    }
}

impl Drop for PrivateHandleTable {
    fn drop(&mut self) {
        // SAFETY: terminate only the never-resumed holder, not its job, which
        // Windows may also have inherited into the real Quick Command.
        let _ = unsafe { TerminateProcess(self.handle(), 0) };
    }
}

fn resolve_executable(program: &str) -> Result<PathBuf, String> {
    let program = Path::new(program);
    let mut filename = program.to_owned();
    if program.extension().is_none() {
        filename.set_extension("exe");
    }
    if filename.is_absolute() {
        return Ok(filename);
    }
    if filename.components().count() != 1 {
        return Err("Invalid Quick Command executable".into());
    }
    let app = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_owned));
    // Like std::process::Command, never implicitly search the working directory.
    let system = read_system_path(|buffer| unsafe { GetSystemDirectoryW(Some(buffer)) });
    let windows = read_system_path(|buffer| unsafe { GetWindowsDirectoryW(Some(buffer)) });
    let path = std::env::var_os("PATH").unwrap_or_default();
    app.into_iter()
        .chain(system)
        .chain(windows)
        .chain(std::env::split_paths(&path))
        .filter(|dir| dir.is_absolute())
        .map(|dir| dir.join(&filename))
        .find(|path| path.is_file())
        .ok_or_else(|| "Quick Command executable not found".into())
}

fn read_system_path(read: impl FnOnce(&mut [u16]) -> u32) -> Option<PathBuf> {
    let mut buffer = [0; 260];
    let written = read(&mut buffer) as usize;
    if written == 0 || written >= buffer.len() {
        return None;
    }
    Some(PathBuf::from(OsString::from_wide(&buffer[..written])))
}

fn launch_arguments(exe: &Path, args: &[String]) -> Result<(Vec<u16>, Vec<u16>), String> {
    let batch = exe
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("bat") || extension.eq_ignore_ascii_case("cmd")
        });
    if batch {
        let cmd = read_system_path(|buffer| unsafe { GetSystemDirectoryW(Some(buffer)) })
            .ok_or("System command processor unavailable")?
            .join("cmd.exe");
        // Match Rust std's Windows batch wrapper: trusted system cmd, no
        // AutoRun or delayed expansion, and the extra outer quote for /c.
        let mut line: Vec<u16> = "cmd.exe /e:ON /v:OFF /d /c \"\"".encode_utf16().collect();
        let path = batch_path(exe)?;
        if path.iter().any(|&c| matches!(c, 10 | 13 | 34)) {
            return Err("Invalid Quick Command batch path".into());
        }
        for c in path.into_iter().take_while(|&c| c != 0) {
            append_batch_unit(&mut line, c);
        }
        line.push(34);
        for arg in args {
            line.push(32);
            append_batch_arg(&mut line, arg)?;
        }
        line.extend([34, 0]);
        return Ok((wide(cmd.as_os_str())?, line));
    }
    let mut line = Vec::new();
    append_arg(&mut line, exe.as_os_str())?;
    for arg in args {
        line.push(32);
        append_arg(&mut line, OsStr::new(arg))?;
    }
    line.push(0);
    Ok((wide(exe.as_os_str())?, line))
}

fn batch_path(exe: &Path) -> Result<Vec<u16>, String> {
    use windows::Win32::Storage::FileSystem::GetFullPathNameW;

    let original = wide(exe.as_os_str())?;
    const MAX_PATH: usize = 260;
    // As in Rust std, remove a verbatim prefix only when the ordinary path
    // round-trips through Win32 normalization without changing its meaning.
    if original.len() > MAX_PATH {
        return Ok(original);
    }
    let candidate = match original.as_slice() {
        [92, 92, 63, 92, drive, 58, 92, ..]
            if u8::try_from(*drive).is_ok_and(|d| d.is_ascii_alphabetic()) =>
        {
            original[4..].to_vec()
        }
        [92, 92, 63, 92, 85, 78, 67, 92, ..] => {
            let mut unc = vec![92, 92];
            unc.extend_from_slice(&original[8..]);
            unc
        }
        _ => return Ok(original),
    };
    let mut normalized = [0; MAX_PATH];
    // SAFETY: candidate is NUL terminated and normalized is writable.
    let len = unsafe { GetFullPathNameW(PCWSTR(candidate.as_ptr()), Some(&mut normalized), None) }
        as usize;
    if len > 0 && len < normalized.len() && normalized[..len] == candidate[..candidate.len() - 1] {
        Ok(candidate)
    } else {
        Ok(original)
    }
}

// Adapted from Rust std's sys/args/windows.rs append_bat_arg
// (Rust project contributors, MIT OR Apache-2.0). cmd.exe does not use CRT
// quoting rules. Keep metacharacters quoted and suppress %VAR% expansion.
fn append_batch_arg(out: &mut Vec<u16>, arg: &str) -> Result<(), String> {
    if arg.contains(['\0', '\r', '\n']) {
        return Err("Invalid Quick Command batch argument".into());
    }
    let quote = arg.is_empty()
        || arg.ends_with('\\')
        || arg.chars().any(|c| {
            c.is_control()
                || (c.is_ascii() && !(c.is_ascii_alphanumeric() || r"#$*+-./:?@\_".contains(c)))
        });
    if quote {
        out.push(34);
    }
    let mut slashes = 0;
    for c in arg.encode_utf16() {
        if c == 92 {
            slashes += 1;
        } else {
            if c == 34 {
                out.extend(std::iter::repeat_n(92, slashes));
                out.push(34);
            }
            slashes = 0;
        }
        append_batch_unit(out, c);
    }
    if quote {
        out.extend(std::iter::repeat_n(92, slashes));
        out.push(34);
    }
    Ok(())
}

fn append_batch_unit(out: &mut Vec<u16>, unit: u16) {
    if unit == 37 {
        // The cmd substring expression consumes the percent without expanding
        // the following text as an environment variable.
        out.extend("%%cd:~,".encode_utf16());
    }
    out.push(unit);
}

fn wide(value: &OsStr) -> Result<Vec<u16>, String> {
    let value = value.encode_wide().collect::<Vec<_>>();
    if value.contains(&0) {
        return Err("Invalid Quick Command process argument".into());
    }
    Ok(value.into_iter().chain([0]).collect())
}

// Microsoft CRT argv quoting: quote every argument, doubling backslashes
// before a quote or the trailing delimiter. No shell interpolation.
fn append_arg(out: &mut Vec<u16>, value: &OsStr) -> Result<(), String> {
    const QUOTE: u16 = b'"' as u16;
    const SLASH: u16 = b'\\' as u16;
    let mut slashes = 0;
    out.push(QUOTE);
    for c in value.encode_wide() {
        match c {
            0 => return Err("Invalid Quick Command process argument".into()),
            SLASH => slashes += 1,
            _ => {
                out.extend(std::iter::repeat_n(
                    SLASH,
                    if c == QUOTE { 2 * slashes + 1 } else { slashes },
                ));
                out.push(c);
                slashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n(SLASH, 2 * slashes));
    out.push(QUOTE);
    Ok(())
}
