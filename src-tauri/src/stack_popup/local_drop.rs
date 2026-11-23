use super::models::{StackDeleteResult, StackLocalDropResult, StackPasteFailure, StackUndoMoveResult};
use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
    fs::{self, File, OpenOptions},
    io::{self, Write},
    mem::{offset_of, size_of},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::{AsRawHandle, FromRawHandle}},
    path::{Component, Path, PathBuf},
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    time::{Duration, Instant},
};
use windows::{
    core::PWSTR,
    Wdk::Storage::FileSystem::{
        FileRenameInformation, NtSetInformationFile, FILE_OPEN_REPARSE_POINT,
        FILE_RENAME_INFORMATION, FILE_SYNCHRONOUS_IO_NONALERT,
    },
    Win32::{
        Foundation::{HANDLE, NTSTATUS, UNICODE_STRING},
        Storage::FileSystem::{
            GetFileInformationByHandle, GetFileInformationByHandleEx, GetFinalPathNameByHandleW, FileIdInfo,
            BY_HANDLE_FILE_INFORMATION, FILE_ID_INFO, DELETE, FILE_ATTRIBUTE_DIRECTORY,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
            FILE_SHARE_WRITE, SYNCHRONIZE, VOLUME_NAME_GUID,
        },
        System::IO::IO_STATUS_BLOCK,
    },
};

pub(super) const MAX_LOCAL_ITEMS: usize = 1024;
const RECEIPT_LIFETIME: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct FileIdentity {
    volume: u32,
    index: (u32, u32),
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct UndoIdentity(u64, [u8; 16]);

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

    pub(super) fn commit(self, destination: &Path) -> Result<(StackLocalDropResult, Vec<UndoItem>), String> {
        self.sources.commit(destination)
    }
}

#[derive(Default)]
pub(super) struct UndoState {
    busy: Arc<AtomicBool>,
    owner_epoch: u64,
    latest: Option<UndoReceipt>,
}

struct UndoReceipt {
    id: String,
    owner: isize,
    items: Vec<UndoItem>,
}

pub(super) struct MovePermit {
    busy: Arc<AtomicBool>,
    owner_epoch: u64,
}

impl Drop for MovePermit {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}

impl UndoState {
    pub(super) fn reserve(&self) -> Result<MovePermit, String> {
        self.busy.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| "Another Stack move or undo is in progress")?;
        Ok(MovePermit { busy: self.busy.clone(), owner_epoch: self.owner_epoch })
    }

    pub(super) fn retire_owner(&mut self) {
        self.owner_epoch = self.owner_epoch.wrapping_add(1);
        self.latest = None;
    }

    // Called only after at least one item actually moved, even for an untrackable batch.
    pub(super) fn replace(
        &mut self,
        permit: &MovePermit,
        owner: Option<isize>,
        receipt: Option<(String, Vec<UndoItem>)>,
    ) -> Option<String> {
        self.latest = None;
        let owner = owner.filter(|_| self.owner_epoch == permit.owner_epoch)?;
        let (id, items) = receipt?;
        self.latest = Some(UndoReceipt { id: id.clone(), owner, items });
        Some(id)
    }

    pub(super) fn take(&mut self, id: &str, owner: isize) -> Result<Vec<UndoItem>, String> {
        self.latest.take_if(|receipt| receipt.id == id && receipt.owner == owner)
            .map(|receipt| receipt.items)
            .ok_or_else(|| "Move is no longer available to undo".into())
    }
}

struct MoveDirectory {
    path: PathBuf,
    id: UndoIdentity,
    delete_chain: Option<Vec<UndoIdentity>>,
}

pub(super) struct UndoItem {
    original: Arc<MoveDirectory>,
    current: Arc<MoveDirectory>,
    original_leaf: OsString,
    current_leaf: OsString,
    id: UndoIdentity,
}

impl UndoItem {
    pub(super) fn before_clipboard_rename(source: &Path, target: &Path) -> Option<Self> {
        fn directory(path: &Path) -> Option<(File, Arc<MoveDirectory>)> {
            let (file, _, true) = open_destination(path).ok()? else { return None };
            let id = undo_identity(&file).ok()?;
            Some((file, Arc::new(MoveDirectory { path: path.to_owned(), id, delete_chain: None })))
        }
        if !plain_absolute_path(source) || !plain_absolute_path(target) {
            return None;
        }
        let (original_handle, original) = directory(source.parent()?)?;
        let (_current_handle, current) = directory(target.parent()?)?;
        let original_leaf = source.file_name()?;
        let (file, _, _) = open_child(&original_handle, original_leaf, false).ok()?;
        let id = undo_identity(&file).ok()?;
        if id.0 != original.id.0 || id.0 != current.id.0 ||
            !entry_has_single_link(&file).ok()? {
            return None;
        }
        Some(Self {
            original, current, id,
            original_leaf: original_leaf.to_owned(),
            current_leaf: target.file_name()?.to_owned(),
        })
    }

    pub(super) fn verify_clipboard_rename(self) -> Option<Self> {
        let _original = self.original.reopen().ok()?;
        let current = self.current.reopen().ok()?;
        let (file, _, _) = open_child(&current, &self.current_leaf, false).ok()?;
        (undo_identity(&file).ok()? == self.id && entry_has_single_link(&file).ok()?).then_some(self)
    }

    fn undo(&self) -> Result<(), String> {
        let original = self.original.repin_for_undo()?;
        let current = self.current.repin_for_undo()?;
        let original_handle = &original.last().ok_or("Original folder unavailable")?.handle;
        let current_handle = &current.last().ok_or("Current folder unavailable")?.handle;
        let (file, _, _) = open_child(current_handle, &self.current_leaf, true)?;
        if undo_identity(&file)? != self.id || !entry_has_single_link(&file)? {
            return Err("Moved item changed".into());
        }
        rename_without_replacement(&file, original_handle, &self.original_leaf)
    }
}

impl MoveDirectory {
    fn reopen(&self) -> Result<File, String> {
        let (file, _, directory) = open_destination(&self.path)?;
        if !directory || undo_identity(&file)? != self.id { return Err("Move folder changed".into()); }
        Ok(file)
    }

    fn repin_for_undo(&self) -> Result<Vec<PinnedDirectory>, String> {
        let Some(expected) = &self.delete_chain else {
            return Ok(vec![PinnedDirectory { handle: self.reopen()?, id: self.id }]);
        };
        let directories = pin_directory_chain(&self.path, false)?;
        if !directories.iter().map(|entry| entry.id).eq(expected.iter().copied()) {
            return Err("Recovery directory ancestry changed".into());
        }
        Ok(directories)
    }
}

pub(super) fn undo_moves(items: Vec<UndoItem>) -> StackUndoMoveResult {
    let mut result = StackUndoMoveResult { moved: 0, failures: Vec::new() };
    for item in items.into_iter().rev() {
        match item.undo() {
            Ok(()) => result.moved += 1,
            Err(message) => result.failures.push(StackPasteFailure {
                path: item.current.path.join(&item.current_leaf).to_string_lossy().into_owned(),
                message,
            }),
        }
    }
    result
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
    identify_entry(file)
}

fn identify_entry(file: File) -> Result<(File, FileIdentity, bool), String> {
    let (id, directory, _) = entry_identity(&file)?;
    Ok((file, id, directory))
}

fn entry_identity(file: &File) -> Result<(FileIdentity, bool, bool), String> {
    let info = entry_info(file)?;
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 ||
        (info.nFileIndexHigh == 0 && info.nFileIndexLow == 0) {
        return Err("Local drop requires non-linked entries with filesystem identity".into());
    }
    let directory = info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0 != 0;
    let id = FileIdentity {
        volume: info.dwVolumeSerialNumber,
        index: (info.nFileIndexHigh, info.nFileIndexLow),
    };
    Ok((id, directory, directory || info.nNumberOfLinks == 1))
}

fn open_child(parent: &File, leaf: &OsStr, delete: bool) -> Result<(File, FileIdentity, bool), String> {
    // The windows crate gates this NT routine behind Wdk_Foundation. Bind its
    // documented ABI locally to keep the existing Windows feature set unchanged.
    #[repr(C)]
    struct ObjectAttributes {
        length: u32,
        root_directory: HANDLE,
        object_name: *const UNICODE_STRING,
        attributes: u32,
        security_descriptor: *const std::ffi::c_void,
        security_quality_of_service: *const std::ffi::c_void,
    }
    #[link(name = "ntdll")]
    extern "system" {
        fn NtOpenFile(
            output: *mut HANDLE, access: u32, attributes: *const ObjectAttributes,
            io: *mut IO_STATUS_BLOCK, sharing: u32, options: u32,
        ) -> NTSTATUS;
    }

    let mut name = encode_leaf(leaf)?;
    let len = (name.len() * size_of::<u16>()) as u16;
    let name = UNICODE_STRING { Length: len, MaximumLength: len, Buffer: PWSTR(name.as_mut_ptr()) };
    let attributes = ObjectAttributes {
        length: size_of::<ObjectAttributes>() as u32,
        root_directory: handle(parent),
        object_name: &name,
        attributes: 0,
        security_descriptor: std::ptr::null(),
        security_quality_of_service: std::ptr::null(),
    };
    let mut output = HANDLE::default();
    let mut io = IO_STATUS_BLOCK::default();
    let access = (FILE_READ_ATTRIBUTES | SYNCHRONIZE).0 | if delete { DELETE.0 } else { 0 };
    let sharing = if delete { FILE_SHARE_READ } else { FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE };
    // SAFETY: The pinned parent, UTF-16 leaf, and C records live through this
    // synchronous call. Only a successful call transfers a handle to File.
    let status = unsafe {
        NtOpenFile(&mut output, access, &attributes, &mut io, sharing.0,
            (FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT).0)
    };
    if status.0 < 0 || output.is_invalid() {
        return Err(format!("Moved item unavailable (NTSTATUS 0x{:08X})", status.0 as u32));
    }
    // SAFETY: NtOpenFile produced a fresh owned file handle on success.
    identify_entry(unsafe { File::from_raw_handle(output.0) })
}

fn entry_info(file: &File) -> Result<BY_HANDLE_FILE_INFORMATION, String> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // The file owns a valid handle and `info` is a writable Win32 output record.
    unsafe { GetFileInformationByHandle(handle(file), &mut info) }
        .map_err(|error| format!("Local drop identity unavailable: {error}"))?;
    Ok(info)
}

fn entry_has_single_link(file: &File) -> Result<bool, String> {
    entry_identity(file).map(|(_, _, single_link)| single_link)
}

fn undo_identity(file: &File) -> Result<UndoIdentity, String> {
    let mut info = FILE_ID_INFO::default();
    // SAFETY: The live file owns its handle; the output buffer has the exact FileIdInfo layout and size.
    unsafe {
        GetFileInformationByHandleEx(handle(file), FileIdInfo, (&mut info as *mut FILE_ID_INFO).cast(), size_of::<FILE_ID_INFO>() as u32)
    }.map_err(|_| "Filesystem identity unavailable for move undo")?;
    if info.FileId.Identifier == [0; 16] { return Err("Filesystem identity unavailable for move undo".into()); }
    Ok(UndoIdentity(info.VolumeSerialNumber, info.FileId.Identifier))
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

    pub(super) fn commit(self, destination: &Path) -> Result<(StackLocalDropResult, Vec<UndoItem>), String> {
        if !plain_absolute_path(destination) || destination.parent() != Some(self.parent.as_path()) {
            return Err("Local drop destination must be a sibling folder".into());
        }
        let (_parent, parent_id, is_dir) = open_entry(&self.parent, false, true)?;
        if !is_dir || parent_id != self.parent_id { return Err("Local drop parent changed".into()); }
        let (target_handle, destination_id, is_dir) = open_destination(destination)?;
        if !is_dir || destination_id.volume != parent_id.volume || self.items.iter().any(|(_, id)| *id == destination_id) {
            return Err("Local drop requires a different folder on the same volume".into());
        }
        let original = undo_identity(&_parent).ok().map(|id| Arc::new(MoveDirectory { path: self.parent, id, delete_chain: None }));
        let current = undo_identity(&target_handle).ok().map(|id| Arc::new(MoveDirectory { path: destination.to_owned(), id, delete_chain: None }));
        let mut result = StackLocalDropResult { moved: 0, failures: Vec::new(), undo_move_id: None };
        let mut undo_items = Vec::with_capacity(self.items.len());
        for (path, expected) in self.items {
            let moved = (|| {
                let (file, actual, _) = open_entry(&path, true, true)?;
                if actual != expected { return Err("Local drop source changed".into()); }
                let name = path.file_name().ok_or("Local drop source has no name")?;
                let undo_id = undo_identity(&file).ok();
                rename_without_replacement(&file, &target_handle, name)?;
                let item = (|| {
                    let (original, current, id) = (original.as_ref()?, current.as_ref()?, undo_id?);
                    if id.0 != original.id.0 || id.0 != current.id.0 || undo_identity(&file).ok()? != id ||
                        !entry_has_single_link(&file).ok()? {
                        return None;
                    }
                    Some(UndoItem {
                        original: original.clone(), current: current.clone(),
                        original_leaf: name.to_owned(), current_leaf: name.to_owned(), id,
                    })
                })();
                Ok(item)
            })();
            match moved {
                Ok(item) => {
                    result.moved += 1;
                    undo_items.extend(item);
                }
                Err(message) => result.failures.push(StackPasteFailure { path: path.to_string_lossy().into_owned(), message }),
            }
        }
        Ok((result, undo_items))
    }
}

fn encode_leaf(leaf: &OsStr) -> Result<Vec<u16>, String> {
    let name: Vec<u16> = leaf.encode_wide().collect();
    if name.is_empty() || name.len() > 32_767 || leaf == "." || leaf == ".." ||
        name.iter().any(|&unit| matches!(unit, 0 | 0x2F | 0x3A | 0x5C)) {
        return Err("Local drop target must be a single file name".into());
    }
    Ok(name)
}

struct PinnedDirectory {
    handle: File,
    id: UndoIdentity,
}

fn pin_directory(path: &Path) -> Result<PinnedDirectory, String> {
    let (handle, _, directory) = open_destination(path)?;
    if !directory { return Err("Recovery path is not a directory".into()); }
    let id = undo_identity(&handle)?;
    Ok(PinnedDirectory { handle, id })
}

fn require_volume_root(root: &File) -> Result<(), String> {
    let mut name = [0u16; 64];
    // SAFETY: The handle and writable buffer live through the call. GUID form
    // exposes the pinned object's real volume-relative path, not a DOS alias.
    let len = unsafe { GetFinalPathNameByHandleW(handle(root), &mut name, VOLUME_NAME_GUID) } as usize;
    let canonical = (len > 0 && len < name.len()).then(|| String::from_utf16(&name[..len]).ok()).flatten();
    let at_root = canonical.as_deref().and_then(|path| path.strip_prefix(r"\\?\Volume{"))
        .and_then(|path| path.split_once('}'))
        .is_some_and(|(guid, suffix)| guid.len() == 36 && matches!(suffix, "" | "\\"));
    if !at_root { return Err("Recovery requires a real local volume root, not a drive alias".into()); }
    Ok(())
}

fn pin_directory_chain(path: &Path, create: bool) -> Result<Vec<PinnedDirectory>, String> {
    if !plain_absolute_path(path) || !matches!(path.components().next(), Some(Component::Prefix(prefix))
        if matches!(prefix.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_))) {
        return Err("Recoverable delete requires an absolute local drive path".into());
    }
    let directories: Vec<_> = path.ancestors().filter(|parent| parent.is_absolute()).collect();
    if directories.len() > 256 { return Err("Recovery path has too many directory levels".into()); }
    let mut pinned: Vec<PinnedDirectory> = Vec::with_capacity(directories.len());
    for directory in directories.into_iter().rev() {
        if create && !pinned.is_empty() {
            match fs::create_dir(directory) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(format!("Cannot create recovery directory: {error}")),
            }
        }
        let current = pin_directory(directory)?;
        if let Some(parent) = pinned.last() {
            verify_directory_child(parent, directory, &current)?;
        } else {
            require_volume_root(&current.handle)?;
        }
        pinned.push(current);
    }
    Ok(pinned)
}

struct DeleteSource {
    path: PathBuf,
    ancestors: Vec<UndoIdentity>,
    id: UndoIdentity,
}

impl DeleteSource {
    fn inspect(path: &Path, storage: &[PinnedDirectory]) -> Result<Self, String> {
        if !plain_absolute_path(path) || path.to_str().is_none() {
            return Err("Original recovery path cannot be represented losslessly".into());
        }
        let staging = storage.last().ok_or("Recovery storage unavailable")?;
        let parent = path.parent().ok_or("Cannot delete a drive root")?;
        let leaf = path.file_name().ok_or("Cannot delete a drive root")?;
        let ancestors = pin_directory_chain(parent, false)?;
        let parent = ancestors.last().ok_or("Original folder unavailable")?;
        if parent.id.0 != staging.id.0 {
            return Err("Recovery storage is on a different filesystem".into());
        }
        if ancestors.iter().any(|ancestor| ancestor.id == staging.id) {
            return Err("Cannot delete recovery storage or its contents".into());
        }
        let (file, _, _) = open_child(&parent.handle, leaf, false)?;
        let id = undo_identity(&file)?;
        if id.0 != staging.id.0 || !entry_has_single_link(&file)? {
            return Err("Recoverable delete requires a same-volume, non-linked item".into());
        }
        if storage.iter().any(|directory| directory.id == id) {
            return Err("Cannot delete recovery storage or one of its parent folders".into());
        }
        // The supplied Win32 path and the exact leaf under its pinned parent must name the same object.
        let (absolute_entry, _, _) = open_entry(path, false, false)?;
        if undo_identity(&absolute_entry)? != id { return Err("Original item path changed".into()); }
        Ok(Self { path: path.to_owned(), ancestors: ancestors.iter().map(|entry| entry.id).collect(), id })
    }

    fn stage(self, batch: &Path, batch_handle: &PinnedDirectory, storage: &[PinnedDirectory], index: usize) -> Result<Option<UndoItem>, String> {
        let parent = self.path.parent().ok_or("Original folder unavailable")?;
        let leaf = self.path.file_name().ok_or("Original filename unavailable")?;
        let ancestors = pin_directory_chain(parent, false)?;
        if !ancestors.iter().map(|entry| entry.id).eq(self.ancestors.iter().copied()) {
            return Err("Original folder changed".into());
        }
        let original = ancestors.last().ok_or("Original folder unavailable")?;
        let (source, _, _) = open_child(&original.handle, leaf, true)?;
        if undo_identity(&source)? != self.id || !entry_has_single_link(&source)? {
            return Err("Original item changed".into());
        }

        let slot = batch.join(index.to_string());
        let slot_handle = create_recovery_directory(batch_handle, &slot)?;
        let payload = slot.join("payload");
        let target = create_recovery_directory(&slot_handle, &payload)?;
        if target.id.0 != self.id.0 { return Err("Recovery storage changed filesystems".into()); }
        write_delete_manifest(&slot, &self.path, leaf)?;
        rename_without_replacement(&source, &target.handle, leaf)?;
        // The file was safely staged even if identity verification subsequently fails.
        Ok((undo_identity(&source).ok() == Some(self.id)).then(|| UndoItem {
            original: Arc::new(MoveDirectory {
                path: parent.to_owned(), id: original.id, delete_chain: Some(self.ancestors),
            }),
            current: Arc::new(MoveDirectory {
                path: payload, id: target.id,
                delete_chain: Some(storage.iter().map(|directory| directory.id)
                    .chain([batch_handle.id, slot_handle.id, target.id]).collect()),
            }),
            original_leaf: leaf.to_owned(), current_leaf: leaf.to_owned(), id: self.id,
        }))
    }
}

fn verify_directory_child(parent: &PinnedDirectory, path: &Path, expected: &PinnedDirectory) -> Result<(), String> {
    let name = path.file_name().ok_or("Missing recovery directory name")?;
    let (child, _, directory) = open_child(&parent.handle, name, false)?;
    if !directory || undo_identity(&child)? != expected.id { return Err("Recovery directory changed".into()); }
    Ok(())
}

fn create_recovery_directory(parent: &PinnedDirectory, path: &Path) -> Result<PinnedDirectory, String> {
    fs::create_dir(path).map_err(|error| format!("Cannot create unique recovery directory: {error}"))?;
    let directory = pin_directory(path)?;
    verify_directory_child(parent, path, &directory)?;
    Ok(directory)
}

fn write_delete_manifest(slot: &Path, original: &Path, leaf: &OsStr) -> Result<(), String> {
    let original = original.to_str().ok_or("Original path cannot be represented losslessly")?;
    let leaf = leaf.to_str().ok_or("Original filename cannot be represented losslessly")?;
    let manifest = serde_json::to_vec(&serde_json::json!({
        "version": 1, "originalPath": original, "payloadName": leaf,
    })).map_err(|error| format!("Cannot encode recovery manifest: {error}"))?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(slot.join("manifest.json"))
        .map_err(|error| format!("Cannot create recovery manifest: {error}"))?;
    file.write_all(&manifest).and_then(|()| file.sync_all())
        .map_err(|error| format!("Cannot persist recovery manifest: {error}"))
}

/// Stage recoverable deletes without deleting any data or following linked entries.
pub(super) fn stage_deleted_items(paths: &[PathBuf], staging_root: &Path) -> Result<(StackDeleteResult, Vec<UndoItem>), String> {
    if paths.is_empty() || paths.len() > MAX_LOCAL_ITEMS {
        return Err("Invalid recoverable-delete selection".into());
    }
    let storage = pin_directory_chain(staging_root, true)?;
    let candidates: Vec<_> = paths.iter().map(|path| DeleteSource::inspect(path, &storage)).collect();
    let overlapping: Vec<_> = candidates.iter().enumerate().map(|(index, candidate)| {
        let Ok(candidate) = candidate else { return false };
        candidates.iter().enumerate().any(|(other_index, other)| other_index != index &&
            other.as_ref().is_ok_and(|other| other.id == candidate.id ||
                candidate.ancestors.contains(&other.id) || other.ancestors.contains(&candidate.id)))
    }).collect();

    let batch = if candidates.iter().zip(&overlapping).any(|(item, overlap)| item.is_ok() && !overlap) {
        // SAFETY: CoCreateGuid has no caller-supplied pointers.
        let guid = unsafe { windows::Win32::System::Com::CoCreateGuid() }
            .map_err(|error| format!("Cannot allocate recovery batch: {error}"))?;
        let path = staging_root.join(format!("{guid:?}"));
        let handle = create_recovery_directory(storage.last().ok_or("Recovery storage unavailable")?, &path)?;
        Some((path, handle))
    } else { None };

    let mut result = StackDeleteResult { deleted: 0, failures: Vec::new(), undo_move_id: None };
    let mut undo = Vec::with_capacity(paths.len());
    for (index, (path, candidate)) in paths.iter().zip(candidates).enumerate() {
        let staged = candidate.and_then(|item| {
            if overlapping[index] { return Err("Recovery selection has overlapping items".into()); }
            let (batch_path, batch_handle) = batch.as_ref().ok_or("Recovery batch unavailable")?;
            item.stage(batch_path, batch_handle, &storage, index)
        });
        match staged {
            Ok(item) => { result.deleted += 1; undo.extend(item); }
            Err(message) => result.failures.push(StackPasteFailure { path: path.to_string_lossy().into_owned(), message }),
        }
    }
    Ok((result, undo))
}

fn rename_without_replacement(file: &File, destination: &File, leaf: &OsStr) -> Result<(), String> {
    let name = encode_leaf(leaf)?;
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
