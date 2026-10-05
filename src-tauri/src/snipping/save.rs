//! Native-picker-only PNG publication. Never truncate the destination before commit.
//! A successful rename is publication, not a power-loss durability guarantee.
use std::{fs::{self, File, OpenOptions}, io::Write, path::{Component, Path, PathBuf}};

const MAX_PNG_BYTES: usize = 512 * 1024 * 1024;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveError { AccessDenied, InvalidImage, Failed }
impl SaveError {
    pub fn code(self) -> &'static str { match self {
        Self::AccessDenied => "save-access-denied", Self::InvalidImage | Self::Failed => "save-failed",
    } }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveFailpoint { CreateSibling, WriteSibling, FlushSibling, PublishSibling }

struct Sibling(PathBuf);
impl Drop for Sibling { fn drop(&mut self) { let _ = fs::remove_file(&self.0); } }

fn io_error(error: std::io::Error) -> SaveError {
    if error.kind() == std::io::ErrorKind::PermissionDenied { SaveError::AccessDenied } else { SaveError::Failed }
}

/// Destination is supplied exclusively by an overwrite-confirming native picker.
/// Reject relative paths, alternate data streams, reparse ancestors and nonfiles.
pub fn publish_png(destination: &Path, png: &[u8]) -> Result<(), SaveError> {
    publish(destination, png, None)
}
#[cfg(test)]
pub fn publish_png_with_failpoint(destination: &Path, png: &[u8], fail: SaveFailpoint) -> Result<(), SaveError> {
    publish(destination, png, Some(fail))
}

fn publish(destination: &Path, png: &[u8], fail: Option<SaveFailpoint>) -> Result<(), SaveError> {
    if png.len() < 33 || png.len() > MAX_PNG_BYTES || &png[..8] != b"\x89PNG\r\n\x1a\n" || &png[12..16] != b"IHDR" {
        return Err(SaveError::InvalidImage);
    }
    if !destination.is_absolute() || !destination.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("png")) {
        return Err(SaveError::AccessDenied);
    }
    for component in destination.components() {
        if let Component::Normal(name) = component {
            let text = name.to_string_lossy();
            if text.contains(':') || text.ends_with(['.', ' ']) || text.chars().any(|c| c.is_control()) {
                return Err(SaveError::AccessDenied);
            }
        } else if matches!(component, Component::ParentDir | Component::CurDir) { return Err(SaveError::AccessDenied); }
    }
    let parent = destination.parent().ok_or(SaveError::AccessDenied)?;
    let _ancestors = pin_ancestors(parent)?;
    validate_target(destination)?;
    if fail == Some(SaveFailpoint::CreateSibling) { return Err(SaveError::Failed); }
    let (sibling, mut file) = create_sibling(parent)?;
    if fail == Some(SaveFailpoint::WriteSibling) { return Err(SaveError::Failed); }
    file.write_all(png).map_err(io_error)?;
    if fail == Some(SaveFailpoint::FlushSibling) { return Err(SaveError::Failed); }
    file.sync_all().map_err(io_error)?;
    drop(file);
    // Ancestors remain pinned against rename/delete throughout validation and commit.
    validate_target(destination)?;
    if fail == Some(SaveFailpoint::PublishSibling) { return Err(SaveError::Failed); }
    atomic_publish(&sibling.0, destination)?;
    Ok(())
}

fn validate_target(path: &Path) -> Result<(), SaveError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() && !reparse(&metadata) => Ok(()),
        Ok(_) => Err(SaveError::AccessDenied),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}
#[cfg(windows)]
fn reparse(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn reparse(_: &fs::Metadata) -> bool { false }

fn pin_ancestors(parent: &Path) -> Result<Vec<File>, SaveError> {
    let mut pinned = Vec::new();
    // Pin from root downward so an unchecked intermediate directory cannot be
    // swapped while a descendant is opened through it.
    let ancestors: Vec<_> = parent.ancestors().collect();
    for path in ancestors.into_iter().rev() {
        #[cfg(windows)]
        let file = {
            use std::os::windows::fs::OpenOptionsExt;
            // No FILE_SHARE_DELETE: keep each real directory bound until publication.
            OpenOptions::new().access_mode(0).share_mode(3)
                .custom_flags(0x0200_0000 | 0x0020_0000).open(path).map_err(io_error)?
        };
        #[cfg(not(windows))]
        let file = File::open(path).map_err(io_error)?;
        let metadata = file.metadata().map_err(io_error)?;
        if !metadata.is_dir() || reparse(&metadata) || fs::symlink_metadata(path).map_err(io_error)?.file_type().is_symlink() {
            return Err(SaveError::AccessDenied);
        }
        pinned.push(file);
    }
    Ok(pinned)
}

fn create_sibling(parent: &Path) -> Result<(Sibling, File), SaveError> {
    for _ in 0..8 {
        let name = format!(".jasonshell-snip-{}.tmp", random_suffix()?);
        let path = parent.join(name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(windows)] {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(0).custom_flags(0x0020_0000);
        }
        match options.open(&path) {
            Ok(file) => return Ok((Sibling(path), file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_error(error)),
        }
    }
    Err(SaveError::Failed)
}

#[cfg(windows)]
fn random_suffix() -> Result<String, SaveError> {
    #[link(name = "bcrypt")]
    unsafe extern "system" { fn BCryptGenRandom(algorithm: *mut std::ffi::c_void, buffer: *mut u8, len: u32, flags: u32) -> i32; }
    let mut bytes = [0u8; 16];
    // System-preferred cryptographic provider; no caller-supplied RNG handle.
    if unsafe { BCryptGenRandom(std::ptr::null_mut(), bytes.as_mut_ptr(), 16, 2) } < 0 { return Err(SaveError::Failed); }
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}
#[cfg(not(windows))]
fn random_suffix() -> Result<String, SaveError> { Err(SaveError::Failed) }

#[cfg(windows)]
fn atomic_publish(source: &Path, destination: &Path) -> Result<(), SaveError> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" { fn MoveFileExW(source: *const u16, destination: *const u16, flags: u32) -> i32; }
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    // Same-volume atomic rename/replace, no copy fallback, write-through requested.
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 1 | 8) } == 0 {
        return Err(io_error(std::io::Error::last_os_error()));
    }
    Ok(())
}
#[cfg(not(windows))]
fn atomic_publish(_: &Path, _: &Path) -> Result<(), SaveError> { Err(SaveError::Failed) }
