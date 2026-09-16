//! Real Windows P04 `ReplaceFileW` experiment. No delete-first fallback.

use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use windows::{
    core::PCWSTR,
    Win32::Storage::FileSystem::{ReplaceFileW, REPLACE_FILE_FLAGS},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Identity {
    pub(crate) volume: u64,
    pub(crate) file_index: [u8; 16],
    pub(crate) links: u32,
    pub(crate) len: u64,
    pub(crate) modified: u64,
    pub(crate) created: u64,
    pub(crate) digest: super::hash::Digest,
}

#[derive(Debug)]
pub(crate) struct Publication {
    pub(crate) before: Identity,
    pub(crate) target: Identity,
    pub(crate) backup: Identity,
    pub(crate) staging: PathBuf,
    pub(crate) backup_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RetainedArtifacts {
    pub(crate) target: PathBuf,
    pub(crate) staging: PathBuf,
    pub(crate) backup: PathBuf,
}

#[derive(Debug)]
pub(crate) enum SaveError {
    Io(io::Error),
    Unsupported(io::Error),
    Conflict(io::Error),
    PublicationPossible {
        source: io::Error,
        artifacts: RetainedArtifacts,
    },
}

impl SaveError {
    pub(crate) fn state(&self) -> Option<super::save_failpoints::RestartClass> {
        match self {
            Self::PublicationPossible { .. } => {
                Some(super::save_failpoints::RestartClass::PublicationPossible)
            }
            _ => None,
        }
    }

    pub(crate) fn artifacts(&self) -> Option<&RetainedArtifacts> {
        match self {
            Self::PublicationPossible { artifacts, .. } => Some(artifacts),
            _ => None,
        }
    }
}

impl From<io::Error> for SaveError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

type SaveResult<T> = Result<T, SaveError>;

/// Revalidation detects known changes but is not an atomic compare-and-swap.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Revalidation {
    Unchanged,
    Changed,
    Ambiguous,
}

pub(crate) fn assess_revalidation(expected: &Identity, observed: &Identity) -> Revalidation {
    if expected == observed {
        Revalidation::Ambiguous
    } else if expected.volume != observed.volume
        || expected.file_index != observed.file_index
        || expected.len != observed.len
        || expected.modified != observed.modified
        || expected.digest != observed.digest
        || observed.links != 1
    {
        Revalidation::Changed
    } else {
        Revalidation::Ambiguous
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ConflictAssets {
    pub(crate) ours: bool,
    pub(crate) external: bool,
    pub(crate) backup: bool,
}

impl ConflictAssets {
    pub(crate) fn preserved(ours: bool, external: bool, backup: bool) -> Self {
        Self {
            ours,
            external,
            backup,
        }
    }
}

fn opened_file(path: &Path, share: u32) -> io::Result<std::fs::File> {
    super::native::open(path, super::native::READ, share, 0)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.message))
}

fn identity_from_file(file: &mut std::fs::File) -> io::Result<Identity> {
    let info = super::native::info(&file)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.message))?;
    let (volume, file_index) = super::native::file_id(&file)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.message))?;
    let mut hasher = super::hash::Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    file.seek(SeekFrom::Start(0))?;
    let identity = Identity {
        volume,
        file_index,
        links: info.links,
        len: ((info.size_high as u64) << 32) | info.size_low as u64,
        modified: ((info.write[1] as u64) << 32) | info.write[0] as u64,
        created: ((info.creation[1] as u64) << 32) | info.creation[0] as u64,
        digest: hasher.finalize(),
    };
    Ok(identity)
}

fn opened_identity(path: &Path, share: u32) -> io::Result<(std::fs::File, Identity)> {
    let mut file = opened_file(path, share)?;
    let identity = identity_from_file(&mut file)?;
    Ok((file, identity))
}

fn identity(path: &Path) -> io::Result<Identity> {
    opened_identity(
        path,
        super::native::SHARE_READ | super::native::SHARE_WRITE | super::native::SHARE_DELETE,
    )
    .map(|(_, identity)| identity)
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn validate_publication_target(target: &Path) -> io::Result<()> {
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "target has no file name"))?;
    if name.encode_wide().any(|unit| unit == b':' as u16) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "alternate data streams are unsupported",
        ));
    }
    let mut cursor = Some(target);
    while let Some(path) = cursor {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "reparse paths are unsupported",
            ));
        }
        cursor = path.parent().filter(|parent| parent != &path);
        if path.parent().is_none() {
            break;
        }
    }
    if !fs::metadata(target)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "target is not a regular file",
        ));
    }
    Ok(())
}

pub(crate) fn replace(
    target: &Path,
    frozen_revision: &[u8],
    expected: &Identity,
) -> SaveResult<Publication> {
    #[cfg(test)]
    if ACTUAL_CALLER_INSPECTION_FAULT.with(std::cell::Cell::get) {
        return replace_impl(
            target,
            frozen_revision,
            expected,
            || Ok(()),
            |_| Ok(Box::new(TestProtection(|| Ok(())))),
            |target, staging, backup| {
                ACTUAL_CALLER_REPLACE_COUNT.with(|count| count.set(count.get() + 1));
                replace_paths(target, staging, backup)
            },
            |_| {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "injected inspection fault",
                ))
            },
        );
    }
    replace_with_before_publish(target, frozen_revision, expected, || Ok(()))
}

#[cfg(test)]
thread_local! {
    static ACTUAL_CALLER_INSPECTION_FAULT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ACTUAL_CALLER_REPLACE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn inject_actual_caller_inspection_fault(enabled: bool) {
    ACTUAL_CALLER_INSPECTION_FAULT.with(|value| value.set(enabled));
    if enabled {
        ACTUAL_CALLER_REPLACE_COUNT.with(|count| count.set(0));
    }
}

#[cfg(test)]
pub(crate) fn actual_caller_replace_count() -> usize {
    ACTUAL_CALLER_REPLACE_COUNT.with(std::cell::Cell::get)
}

trait Protection {
    fn intact(&self) -> io::Result<()>;
}

impl Protection for super::native::Oplock {
    fn intact(&self) -> io::Result<()> {
        self.intact()
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.message))
    }
}

struct TestProtection<F>(F);
impl<F: Fn() -> io::Result<()>> Protection for TestProtection<F> {
    fn intact(&self) -> io::Result<()> {
        (self.0)()
    }
}

fn replace_with_before_publish(
    target: &Path,
    frozen_revision: &[u8],
    expected: &Identity,
    before_publish: impl FnOnce() -> io::Result<()>,
) -> SaveResult<Publication> {
    replace_impl(
        target,
        frozen_revision,
        expected,
        before_publish,
        |file| {
            super::native::Oplock::acquire(file)
                .map(|guard| Box::new(guard) as Box<dyn Protection>)
                .map_err(|error| {
                    SaveError::Unsupported(io::Error::new(
                        io::ErrorKind::Unsupported,
                        error.message,
                    ))
                })
        },
        replace_paths,
        identity,
    )
}

fn replace_impl(
    target: &Path,
    frozen_revision: &[u8],
    expected: &Identity,
    before_publish: impl FnOnce() -> io::Result<()>,
    acquire: impl FnOnce(&std::fs::File) -> SaveResult<Box<dyn Protection>>,
    publish: impl FnOnce(&Path, &Path, &Path) -> io::Result<()>,
    inspect: impl Fn(&Path) -> io::Result<Identity>,
) -> SaveResult<Publication> {
    validate_publication_target(target)?;
    let before = identity(target)?;
    let parent = target
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "target has no parent"))?;
    let nonce = super::native::random::<8>()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "system RNG failed"))?;
    let token: String = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
    let staging = parent.join(format!(".jasonshell-stage-{token}"));
    let publication_path = parent.join(format!(".jasonshell-publish-{token}"));
    let backup_path = parent.join(format!(".jasonshell-backup-{token}"));
    let mut stage = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)?;
    stage.write_all(frozen_revision)?;
    stage.sync_all()?;
    drop(stage);
    let mut publication = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&publication_path)?;
    publication.write_all(frozen_revision)?;
    publication.sync_all()?;
    drop(publication);
    if &before != expected || before.links != 1 {
        return Err(SaveError::Conflict(io::Error::new(
            io::ErrorKind::Other,
            "source changed or hard-linked",
        )));
    }
    let mut guard_file = opened_file(
        target,
        super::native::SHARE_READ | super::native::SHARE_WRITE | super::native::SHARE_DELETE,
    )?;
    let protection = acquire(&guard_file)?;
    let final_identity = identity_from_file(&mut guard_file)?;
    protection.intact().map_err(SaveError::Conflict)?;
    if final_identity != before {
        return Err(SaveError::Conflict(io::Error::new(
            io::ErrorKind::Other,
            "source changed before publication",
        )));
    }
    if identity(&staging)?.volume != before.volume {
        return Err(SaveError::Io(io::Error::new(
            io::ErrorKind::Other,
            "staging is not same-volume",
        )));
    }
    before_publish()?;
    protection.intact().map_err(SaveError::Conflict)?;
    publish(target, &publication_path, &backup_path)?;
    drop(protection);
    drop(guard_file);
    let artifacts = RetainedArtifacts {
        target: target.to_path_buf(),
        staging: staging.clone(),
        backup: backup_path.clone(),
    };
    let target_identity = inspect(target).map_err(|source| SaveError::PublicationPossible {
        source,
        artifacts: artifacts.clone(),
    })?;
    let backup_identity = inspect(&backup_path)
        .map_err(|source| SaveError::PublicationPossible { source, artifacts })?;
    Ok(Publication {
        before,
        target: target_identity,
        backup: backup_identity,
        staging,
        backup_path,
    })
}

fn replace_paths(target: &Path, staging: &Path, backup: &Path) -> io::Result<()> {
    let target_w = wide(target);
    let staging_w = wide(staging);
    let backup_w = wide(backup);
    // SAFETY: NUL-terminated buffers live through call; paths are distinct, same-volume files.
    unsafe {
        ReplaceFileW(
            PCWSTR(target_w.as_ptr()),
            PCWSTR(staging_w.as_ptr()),
            PCWSTR(backup_w.as_ptr()),
            REPLACE_FILE_FLAGS(0),
            None,
            None,
        )
    }
    .map_err(|error| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("ReplaceFileW failed: {error}"),
        )
    })
}

#[cfg(test)]
pub(crate) fn replace_with_test_hook(
    target: &Path,
    frozen_revision: &[u8],
    expected: &Identity,
    before_publish: impl FnOnce() -> io::Result<()>,
) -> SaveResult<Publication> {
    replace_with_before_publish(target, frozen_revision, expected, before_publish)
}

#[cfg(test)]
pub(crate) fn replace_with_test_guard<F, P, I>(
    target: &Path,
    frozen_revision: &[u8],
    expected: &Identity,
    before_publish: F,
    intact: impl Fn() -> io::Result<()> + 'static,
    publish: P,
    inspect: I,
) -> SaveResult<Publication>
where
    F: FnOnce() -> io::Result<()>,
    P: FnOnce(&Path, &Path, &Path) -> io::Result<()>,
    I: Fn(&Path) -> io::Result<Identity>,
{
    replace_impl(
        target,
        frozen_revision,
        expected,
        before_publish,
        |_| Ok(Box::new(TestProtection(intact))),
        publish,
        inspect,
    )
}

#[cfg(test)]
pub(crate) fn replace_paths_for_test(
    target: &Path,
    staging: &Path,
    backup: &Path,
) -> io::Result<()> {
    replace_paths(target, staging, backup)
}

pub(crate) fn inspect(path: &Path) -> io::Result<Identity> {
    identity(path)
}

pub(crate) fn save_as(target: &Path, frozen_revision: &[u8]) -> io::Result<Identity> {
    if target
        .file_name()
        .is_some_and(|name| name.encode_wide().any(|unit| unit == b':' as u16))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "alternate data streams are unsupported",
        ));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    file.write_all(frozen_revision)?;
    file.sync_all()?;
    drop(file);
    identity(target)
}
