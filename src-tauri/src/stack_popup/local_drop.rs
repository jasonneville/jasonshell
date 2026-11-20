use super::models::{StackLocalDropResult, StackPasteFailure};
use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::{File, OpenOptions},
    mem::{offset_of, size_of},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};
use windows::{
    Wdk::Storage::FileSystem::{FileRenameInformation, NtSetInformationFile, FILE_RENAME_INFORMATION},
    Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION, DELETE, FILE_ATTRIBUTE_DIRECTORY,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE,
        },
        System::IO::IO_STATUS_BLOCK,
    },
};

pub(super) const MAX_LOCAL_ITEMS: usize = 1024;
const RECEIPT_LIFETIME: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct FileIdentity(u32, u32, u32);

pub(super) struct Sources {
    parent: PathBuf,
    parent_id: FileIdentity,
    items: Vec<(PathBuf, FileIdentity)>,
}

pub(super) struct Receipt {
    request_id: String,
    owner: isize,
    issued: Instant,
    sources: Sources,
}

impl Receipt {
    pub(super) fn new(request_id: String, owner: isize, sources: Sources) -> Self {
        Self { request_id, owner, issued: Instant::now(), sources }
    }

    pub(super) fn matches(&self, request_id: &str, owner: isize) -> bool {
        self.request_id == request_id && self.owner == owner && self.issued.elapsed() <= RECEIPT_LIFETIME
    }

    pub(super) fn commit(self, destination: &Path) -> Result<StackLocalDropResult, String> {
        self.sources.commit(destination)
    }
}

fn handle(file: &File) -> HANDLE {
    HANDLE(file.as_raw_handle())
}

fn open_entry(path: &Path, delete: bool, pin: bool) -> Result<(File, FileIdentity, bool), String> {
    let sharing = if pin {
        FILE_SHARE_READ.0
    } else {
        (FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0
    };
    open_entry_with_access(path, delete, pin, sharing)
}

fn open_destination(path: &Path) -> Result<(File, FileIdentity, bool), String> {
    // The rename needs compatible destination write sharing. Deny delete sharing,
    // and bind the rename to this directory handle rather than resolving its path again.
    open_entry_with_access(path, false, true, (FILE_SHARE_READ | FILE_SHARE_WRITE).0)
}

fn open_entry_with_access(path: &Path, delete: bool, pin: bool, sharing: u32) -> Result<(File, FileIdentity, bool), String> {
    // Attribute-only opens do not participate in Windows share-access accounting.
    // A source's DELETE access participates; directory pins need list access.
    let access = FILE_READ_ATTRIBUTES.0 | match (delete, pin) {
        (true, _) => DELETE.0,
        (false, true) => FILE_LIST_DIRECTORY.0,
        (false, false) => 0,
    };
    let file = OpenOptions::new()
        .access_mode(access)
        .share_mode(sharing)
        .custom_flags((FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS).0)
        .open(path)
        .map_err(|error| format!("Local drop entry unavailable: {error}"))?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // The file owns a valid handle and `info` is a writable Win32 output record.
    unsafe { GetFileInformationByHandle(handle(&file), &mut info) }
        .map_err(|error| format!("Local drop identity unavailable: {error}"))?;
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 ||
        (info.nFileIndexHigh == 0 && info.nFileIndexLow == 0) {
        return Err("Local drop requires non-linked entries with filesystem identity".into());
    }
    Ok((file, FileIdentity(info.dwVolumeSerialNumber, info.nFileIndexHigh, info.nFileIndexLow),
        info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0))
}

fn plain_absolute_path(path: &Path) -> bool {
    path.is_absolute() && path.components().all(|part| match part {
        Component::Prefix(prefix) => matches!(prefix.kind(),
            std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_) |
            std::path::Prefix::UNC(_, _) | std::path::Prefix::VerbatimUNC(_, _)),
        Component::RootDir => true,
        Component::Normal(name) => !name.encode_wide().any(|unit| unit == 0 || unit == b':' as u16),
        _ => false,
    })
}

impl Sources {
    pub(super) fn snapshot_listed_paths(paths: &[String]) -> Option<Self> {
        if paths.is_empty() || paths.len() > MAX_LOCAL_ITEMS {
            return None;
        }
        // Listing paths come from DirEntry::path, not from a typed path field.
        // Preserve every character: trimming can select a different sibling.
        let paths: Vec<_> = paths.iter().map(PathBuf::from).collect();
        Self::snapshot(&paths)
    }

    pub(super) fn snapshot(paths: &[PathBuf]) -> Option<Self> {
        if paths.is_empty() || paths.len() > MAX_LOCAL_ITEMS { return None; }
        let parent = paths.first()?.parent()?;
        if paths.iter().any(|path| !plain_absolute_path(path) || path.parent() != Some(parent)) {
            return None;
        }
        let (_, parent_id, true) = open_entry(parent, false, false).ok()? else { return None };
        let mut seen = HashSet::with_capacity(paths.len());
        let mut items = Vec::with_capacity(paths.len());
        for path in paths {
            let (_, id, _) = open_entry(path, false, false).ok()?;
            if !seen.insert(id) { return None; }
            items.push((path.clone(), id));
        }
        Some(Self { parent: parent.to_owned(), parent_id, items })
    }

    fn commit(self, destination: &Path) -> Result<StackLocalDropResult, String> {
        if !plain_absolute_path(destination) || destination.parent() != Some(self.parent.as_path()) {
            return Err("Local drop destination must be a sibling folder".into());
        }
        let (_parent, parent_id, is_dir) = open_entry(&self.parent, false, true)?;
        if !is_dir || parent_id != self.parent_id { return Err("Local drop parent changed".into()); }
        let (target_handle, destination_id, is_dir) = open_destination(destination)?;
        if !is_dir || destination_id.0 != parent_id.0 || self.items.iter().any(|(_, id)| *id == destination_id) {
            return Err("Local drop requires a different folder on the same volume".into());
        }
        let mut result = StackLocalDropResult { moved: 0, failures: Vec::new() };
        for (path, expected) in self.items {
            let moved = (|| {
                let (file, actual, _) = open_entry(&path, true, true)?;
                if actual != expected { return Err("Local drop source changed".into()); }
                let name = path.file_name().ok_or("Local drop source has no name")?;
                rename_without_replacement(&file, &target_handle, name)
            })();
            match moved {
                Ok(()) => result.moved += 1,
                Err(message) => result.failures.push(StackPasteFailure { path: path.to_string_lossy().into_owned(), message }),
            }
        }
        Ok(result)
    }
}

fn rename_without_replacement(file: &File, destination: &File, leaf: &OsStr) -> Result<(), String> {
    let name: Vec<u16> = leaf.encode_wide().collect();
    if name.is_empty() || name.len() > 32_767 || leaf == "." || leaf == ".." ||
        name.iter().any(|&unit| matches!(unit, 0 | 0x2F | 0x3A | 0x5C)) {
        return Err("Local drop target must be a single file name".into());
    }
    let name_offset = offset_of!(FILE_RENAME_INFORMATION, FileName);
    let bytes = (name_offset + name.len() * size_of::<u16>()).max(size_of::<FILE_RENAME_INFORMATION>());
    let mut buffer = vec![0usize; bytes.div_ceil(size_of::<usize>())];
    let info = buffer.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
    let mut io = IO_STATUS_BLOCK::default();
    // WDK FileRenameInformation resolves a relative FileName under RootDirectory.
    // The source handle is synchronous, both files own their handles, and the aligned
    // buffer contains the complete sized UTF-16 tail. ReplaceIfExists stays zero.
    let status = unsafe {
        std::ptr::addr_of_mut!((*info).RootDirectory).write(handle(destination));
        std::ptr::addr_of_mut!((*info).FileNameLength).write((name.len() * size_of::<u16>()) as u32);
        std::ptr::copy_nonoverlapping(name.as_ptr(), buffer.as_mut_ptr().cast::<u8>().add(name_offset).cast(), name.len());
        NtSetInformationFile(handle(file), &mut io, buffer.as_ptr().cast(), bytes as u32, FileRenameInformation)
    };
    if status.0 != 0 { return Err(format!("Move unavailable (NTSTATUS 0x{:08X})", status.0 as u32)); }
    Ok(())
}
