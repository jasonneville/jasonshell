//! Bounded archive import. Only three fixed filenames are written; archive paths
//! are never extraction destinations. Immutable generations publish by rename.
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const FILES: [&str; 3] = [
    "encoder-model.int8.onnx",
    "decoder_joint-model.int8.onnx",
    "vocab.txt",
];
const INVALID: &str =
    "Invalid or unsafe model archive. Choose a complete Parakeet TDT int8 tar bundle.";
const STORAGE: &str = "Could not install model. Check free disk space and app data permissions.";
static NEXT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
pub struct InstallLimits {
    pub max_entries: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
}

impl Default for InstallLimits {
    fn default() -> Self {
        Self {
            max_entries: 4096,
            max_file_bytes: 1024 * 1024 * 1024,
            max_total_bytes: 2 * 1024 * 1024 * 1024,
        }
    }
}

struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn create_staging(app_data: &Path) -> Result<(Staging, PathBuf), String> {
    let root = app_data.join("speech-models");
    fs::create_dir_all(&root).map_err(|_| STORAGE)?;
    let previous = fs::read_dir(&root)
        .map_err(|_| STORAGE)?
        .filter_map(Result::ok)
        .filter_map(|e| {
            e.file_name()
                .to_str()?
                .strip_prefix("model-")?
                .split('-')
                .next()?
                .parse::<u128>()
                .ok()
        })
        .max()
        .unwrap_or(0);
    let clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| STORAGE)?
        .as_nanos()
        .max(previous.saturating_add(1));
    let id = format!("{:030}-{:08}", clock, NEXT.fetch_add(1, Ordering::Relaxed));
    let path = root.join(format!("staging-{id}"));
    // Acquire ownership before constructing the cleanup guard. A collision is not ours.
    fs::create_dir(&path).map_err(|_| STORAGE)?;
    Ok((Staging(path), root.join(format!("model-{id}"))))
}

fn publish<F>(staging: &Staging, installed: PathBuf, validate: F) -> Result<PathBuf, String>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    validate(&staging.0).map_err(|_| {
        "Model could not load. Choose the matching Parakeet TDT 0.6b v2 int8 ONNX bundle."
    })?;
    fs::rename(&staging.0, &installed).map_err(|_| STORAGE)?;
    Ok(installed)
}

fn is_reparse(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn open_source(path: &Path, directory: bool) -> Result<File, String> {
    let before = fs::symlink_metadata(path).map_err(|_| FOLDER_INVALID)?;
    if is_reparse(&before)
        || (if directory {
            !before.is_dir()
        } else {
            !before.is_file() || before.len() == 0
        })
    {
        return Err(FOLDER_INVALID.into());
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // OPEN_REPARSE_POINT validates the opened object, not only a prior path lookup.
        // Deny concurrent write/delete sharing while copying from these handles.
        options
            .share_mode(1)
            .custom_flags(0x00200000 | if directory { 0x02000000 } else { 0 });
    }
    let file = options.open(path).map_err(|_| FOLDER_INVALID)?;
    let opened = file.metadata().map_err(|_| FOLDER_INVALID)?;
    if is_reparse(&opened)
        || (if directory {
            !opened.is_dir()
        } else {
            !opened.is_file() || opened.len() == 0
        })
    {
        return Err(FOLDER_INVALID.into());
    }
    Ok(file)
}

const FOLDER_INVALID: &str = "Choose a folder containing all three regular, nonempty required model files, without links or junctions.";

/// Copies only the three root model files; extras are never traversed or opened.
/// Opened Windows root/file handles reject reparse points and deny write/delete sharing.
/// Ancestor path replacement is not claimed to be protected by these handle checks.
pub fn install_directory<F>(
    source: &Path,
    app_data: &Path,
    limits: InstallLimits,
    validate: F,
) -> Result<PathBuf, String>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    let _root_handle = open_source(source, true)?;
    let mut inputs = Vec::with_capacity(FILES.len());
    let mut total = 0u64;
    for name in FILES {
        let file = open_source(&source.join(name), false)?;
        let size = file.metadata().map_err(|_| FOLDER_INVALID)?.len();
        total = total.checked_add(size).ok_or(FOLDER_INVALID)?;
        if size > limits.max_file_bytes || total > limits.max_total_bytes {
            return Err("Model folder exceeds safe size limits.".into());
        }
        inputs.push((name, file, size));
    }
    let (staging, installed) = create_staging(app_data)?;
    let mut copied_total = 0u64;
    for (name, mut input, size) in inputs {
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(staging.0.join(name))
            .map_err(|_| STORAGE)?;
        let cap = limits
            .max_file_bytes
            .min(limits.max_total_bytes - copied_total);
        let copied = io::copy(
            &mut (&mut input).take(cap.checked_add(1).ok_or(FOLDER_INVALID)?),
            &mut output,
        )
        .map_err(|_| FOLDER_INVALID)?;
        if copied == 0
            || copied > cap
            || copied != size
            || input.metadata().map_err(|_| FOLDER_INVALID)?.len() != size
        {
            return Err(FOLDER_INVALID.into());
        }
        copied_total += copied;
        output.flush().map_err(|_| STORAGE)?;
        output.sync_all().map_err(|_| STORAGE)?;
    }
    publish(&staging, installed, validate)
}

fn regular_nonempty(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .ok()
        .is_some_and(|m| m.is_file() && !m.file_type().is_symlink() && m.len() > 0)
}

/// Returns the newest atomically published generation, never a partial staging directory.
pub fn resolve_installed_model(app_data: &Path) -> Option<PathBuf> {
    let root = app_data.join("speech-models");
    let newest = fs::read_dir(&root)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_owned();
            let suffix = name.strip_prefix("model-")?;
            if suffix.len() != 39 || !suffix.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
                return None;
            }
            Some((name, entry.path()))
        })
        .max_by(|a, b| a.0.cmp(&b.0))?
        .1;
    let metadata = fs::symlink_metadata(&newest).ok()?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || !FILES
            .iter()
            .all(|name| regular_nonempty(&newest.join(name)))
    {
        return None;
    }
    Some(newest)
}

fn safe_name(raw: &[u8], directory: bool) -> Result<String, String> {
    let name = std::str::from_utf8(raw).map_err(|_| INVALID)?;
    let name = if directory {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    if name.is_empty()
        || name.len() > 1024
        || name.contains('\\')
        || name.contains(':')
        || name.starts_with('/')
    {
        return Err(INVALID.into());
    }
    for part in name.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part
                .chars()
                .any(|c| c.is_control() || "<>\"|?*".contains(c))
        {
            return Err(INVALID.into());
        }
        let base = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&base.as_str())
            || base
                .strip_prefix("COM")
                .or_else(|| base.strip_prefix("LPT"))
                .is_some_and(|suffix| {
                    suffix.chars().count() == 1 && "123456789¹²³".contains(suffix)
                })
        {
            return Err(INVALID.into());
        }
    }
    Ok(name.to_owned())
}

/// Streams a tar/gzip bundle into app-owned staging, validates with the actual
/// caller's loader, then atomically publishes. Failed imports retain all old generations.
pub fn install_archive<F>(
    archive: &Path,
    app_data: &Path,
    limits: InstallLimits,
    validate: F,
) -> Result<PathBuf, String>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    let name = archive
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !(name.ends_with(".tar") || name.ends_with(".tar.gz") || name.ends_with(".tgz")) {
        return Err("Choose a .tar, .tar.gz, or .tgz model archive.".into());
    }
    let mut file = File::open(archive).map_err(|_| "Could not read selected archive.")?;
    let mut magic = [0; 2];
    file.read_exact(&mut magic).map_err(|_| INVALID)?;
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(0)).map_err(|_| INVALID)?;
    let compressed = magic == [0x1f, 0x8b];
    if (name.ends_with(".gz") || name.ends_with(".tgz")) && !compressed {
        return Err(INVALID.into());
    }
    let reader: Box<dyn Read> = if compressed {
        Box::new(flate2::read::MultiGzDecoder::new(BufReader::new(file)))
    } else {
        Box::new(BufReader::new(file))
    };
    // Also bound decompressed metadata/padding/trailing input, not just file payloads.
    let stream_limit = limits
        .max_total_bytes
        .checked_add((limits.max_entries as u64).saturating_mul(2048))
        .and_then(|n| n.checked_add(1024 * 1024))
        .ok_or(INVALID)?;
    let mut tar = tar::Archive::new(reader.take(stream_limit.checked_add(1).ok_or(INVALID)?));
    let (staging, installed) = create_staging(app_data)?;
    let mut seen = HashSet::new();
    let mut candidates: HashMap<String, HashSet<String>> = HashMap::new();
    let mut total = 0u64;
    for (index, entry) in tar.entries().map_err(|_| INVALID)?.raw(true).enumerate() {
        if index >= limits.max_entries {
            return Err("Model archive has too many entries.".into());
        }
        let mut entry = entry.map_err(|_| INVALID)?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            return Err(INVALID.into());
        }
        let name = safe_name(&entry.path_bytes(), kind.is_dir())?;
        if !seen.insert(name.to_ascii_lowercase()) {
            return Err(INVALID.into());
        }
        let size = entry.header().size().map_err(|_| INVALID)?;
        total = total.checked_add(size).ok_or(INVALID)?;
        if size > limits.max_file_bytes || total > limits.max_total_bytes {
            return Err("Model archive exceeds safe size limits.".into());
        }
        if kind.is_dir() {
            if size != 0 {
                return Err(INVALID.into());
            }
            continue;
        }
        let (parent, leaf) = name.rsplit_once('/').unwrap_or(("", &name));
        if FILES.contains(&leaf) {
            if size == 0 {
                return Err(INVALID.into());
            }
            candidates
                .entry(parent.to_owned())
                .or_default()
                .insert(leaf.to_owned());
            // Any second occurrence of a required basename is ambiguous even across folders.
            let destination = staging.0.join(leaf);
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .map_err(|_| INVALID)?;
            if io::copy(&mut entry, &mut output).map_err(|_| INVALID)? != size {
                return Err(INVALID.into());
            }
            output.flush().map_err(|_| STORAGE)?;
            output.sync_all().map_err(|_| STORAGE)?;
        } else if io::copy(&mut entry, &mut io::sink()).map_err(|_| INVALID)? != size {
            return Err(INVALID.into());
        }
    }
    // Read through gzip trailers: CRC/truncation errors must fail before loading/publication.
    let mut remainder = tar.into_inner();
    let mut trailing = [0u8; 8192];
    let mut trailing_bytes = 0u64;
    loop {
        let count = remainder.read(&mut trailing).map_err(|_| INVALID)?;
        if count == 0 {
            break;
        }
        trailing_bytes += count as u64;
        if trailing_bytes > 1024 * 1024 || trailing[..count].iter().any(|b| *b != 0) {
            return Err(INVALID.into());
        }
    }
    if remainder.limit() == 0 {
        return Err(INVALID.into());
    }
    if candidates.len() != 1
        || candidates
            .values()
            .next()
            .is_none_or(|files| files.len() != FILES.len())
    {
        return Err(INVALID.into());
    }
    publish(&staging, installed, validate)
}
