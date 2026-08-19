use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) const RECOVERY_JOURNAL_VERSION: u32 = 1;
pub(crate) const RECOVERY_JOURNAL_RETENTION_DAYS: u64 = 14;
pub(crate) const RECOVERY_JOURNAL_DIR_NAME: &str = "stack-browser-recovery";
const RECOVERY_JOURNAL_STALE_INTERRUPT_AFTER_MS: u64 = 24 * 60 * 60 * 1000;
static OP_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RecoveryJournalState {
    Planned,
    CopyStarted,
    CopiedVerified,
    DeleteStarted,
    SourceRemoved,
    Failed,
    Completed,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveryJournalEntry {
    pub version: u32,
    pub operation_id: String,
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
    pub state: RecoveryJournalState,
    pub mode: String,
    pub source_kind: String,
    pub source: String,
    pub destination: String,
    pub phase: String,
    pub source_path: String,
    pub destination_path: String,
    pub failure_kind: Option<String>,
    pub selected_collision_destination: String,
    pub source_file_length: Option<u64>,
    pub copied_file_bytes: Option<u64>,
    pub source_manifest: Option<RecoveryJournalManifest>,
    pub copied_manifest: Option<RecoveryJournalManifest>,
    pub delete_started_at_ms: Option<u64>,
    pub source_removed_at_ms: Option<u64>,
    pub failure_message: Option<String>,
    pub completed_at_ms: Option<u64>,
    pub interrupted_at_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RecoveryJournalOperationKind {
    Copy,
    Cut,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveryJournalManifest {
    pub file_count: u64,
    pub dir_count: u64,
    pub total_bytes: u64,
    pub skipped_unsupported_count: u64,
}

impl RecoveryJournalEntry {
    pub(crate) fn new(
        operation_id: String,
        selected_collision_destination: String,
        _sources: Vec<String>,
        now_epoch_ms: u64,
    ) -> Self {
        Self {
            version: RECOVERY_JOURNAL_VERSION,
            operation_id,
            created_at_epoch_ms: now_epoch_ms,
            updated_at_epoch_ms: now_epoch_ms,
            state: RecoveryJournalState::Planned,
            mode: "copy".to_string(),
            source_kind: "file".to_string(),
            source: String::new(),
            destination: String::new(),
            phase: "planned".to_string(),
            source_path: String::new(),
            destination_path: String::new(),
            failure_kind: None,
            selected_collision_destination,
            source_file_length: None,
            copied_file_bytes: None,
            source_manifest: None,
            copied_manifest: None,
            delete_started_at_ms: None,
            source_removed_at_ms: None,
            failure_message: None,
            completed_at_ms: None,
            interrupted_at_ms: None,
        }
    }

    pub(crate) fn mark_failed(&mut self, message: String, now_epoch_ms: u64) {
        self.failure_message = Some(message);
        self.failure_kind = Some("unknown".to_string());
        self.transition_to(RecoveryJournalState::Failed, now_epoch_ms);
    }

    pub(crate) fn transition_to(&mut self, state: RecoveryJournalState, now_epoch_ms: u64) {
        self.state = state;
        self.updated_at_epoch_ms = now_epoch_ms;
        self.phase = match self.state {
            RecoveryJournalState::Planned => "planned",
            RecoveryJournalState::CopyStarted => "copy-started",
            RecoveryJournalState::CopiedVerified => "copied-verified",
            RecoveryJournalState::DeleteStarted => "delete-started",
            RecoveryJournalState::SourceRemoved => "source-removed",
            RecoveryJournalState::Failed => "failed",
            RecoveryJournalState::Completed => "completed",
            RecoveryJournalState::Interrupted => "interrupted",
        }
        .to_string();
        match self.state {
            RecoveryJournalState::DeleteStarted => self.delete_started_at_ms = Some(now_epoch_ms),
            RecoveryJournalState::SourceRemoved => self.source_removed_at_ms = Some(now_epoch_ms),
            RecoveryJournalState::Completed => self.completed_at_ms = Some(now_epoch_ms),
            RecoveryJournalState::Interrupted => self.interrupted_at_ms = Some(now_epoch_ms),
            _ => {}
        }
    }

    pub(crate) fn mark_stale_running_as_interrupted(
        &mut self,
        now_epoch_ms: u64,
        stale_after_ms: u64,
    ) {
        if !self.is_terminal()
            && now_epoch_ms.saturating_sub(self.updated_at_epoch_ms) > stale_after_ms
        {
            self.transition_to(RecoveryJournalState::Interrupted, now_epoch_ms);
        }
    }

    pub(crate) fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            RecoveryJournalState::Failed
                | RecoveryJournalState::Completed
                | RecoveryJournalState::Interrupted
        )
    }
}

pub(crate) fn recovery_journal_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(RECOVERY_JOURNAL_DIR_NAME)
}

pub(crate) fn recovery_journal_path(dir: &Path, operation_id: &str) -> PathBuf {
    dir.join(format!("recovery-{operation_id}.json"))
}

pub(crate) fn emergency_recovery_journal_disable() -> bool {
    std::env::var_os("JASONSHELL_RECOVERY_JOURNAL_DISABLE").is_some()
}

pub(crate) fn new_recovery_operation_id() -> String {
    let counter = OP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}-{counter:x}-{:x}", std::process::id())
}

pub(crate) fn write_recovery_journal_atomic(
    path: &Path,
    journal: &RecoveryJournalEntry,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Journal parent unavailable".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to prepare recovery journal directory: {error}"))?;
    let temp_path = unique_recovery_journal_temp_path(
        parent,
        path.file_stem()
            .and_then(|v| v.to_str())
            .unwrap_or("recovery-journal"),
    );
    let payload = serde_json::to_vec_pretty(journal)
        .map_err(|error| format!("Failed to serialize recovery journal: {error}"))?;

    {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|error| format!("Failed to create recovery journal temp file: {error}"))?;
        file.write_all(&payload)
            .map_err(|error| format!("Failed to write recovery journal temp file: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Failed to flush recovery journal temp file: {error}"))?;
    }

    replace_file_atomic(&temp_path, path)?;
    sync_parent_dir(parent);
    Ok(())
}

pub(crate) fn cleanup_recovery_journals(
    dir: &Path,
    now_epoch_ms: u64,
    emergency_disable: bool,
) -> Result<Vec<PathBuf>, String> {
    if emergency_disable {
        return Ok(Vec::new());
    }
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let retention_ms =
        Duration::from_secs(RECOVERY_JOURNAL_RETENTION_DAYS * 24 * 60 * 60).as_millis() as u64;
    let mut removed = Vec::new();
    for entry in fs::read_dir(dir)
        .map_err(|error| format!("Failed to read recovery journal directory: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("Failed to read recovery journal entry: {error}"))?;
        let path = entry.path();
        if !is_terminal_journal_file(&path) || is_reparse_or_symlink(&path)? {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let modified_ms = metadata
            .modified()
            .ok()
            .and_then(epoch_ms)
            .unwrap_or(now_epoch_ms);
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let mut journal: RecoveryJournalEntry = match serde_json::from_str(&raw) {
            Ok(journal) => journal,
            Err(_) => continue,
        };
        if journal.is_terminal() {
            if now_epoch_ms.saturating_sub(modified_ms) >= retention_ms {
                fs::remove_file(&path).map_err(|error| {
                    format!("Failed to remove recovery journal artifact: {error}")
                })?;
                removed.push(path);
            }
            continue;
        }
        journal.mark_stale_running_as_interrupted(
            now_epoch_ms,
            RECOVERY_JOURNAL_STALE_INTERRUPT_AFTER_MS,
        );
        write_recovery_journal_atomic(&path, &journal)?;
    }
    Ok(removed)
}

fn is_reparse_or_symlink(path: &Path) -> Result<bool, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("Failed to inspect recovery journal artifact: {error}"))?;
    if metadata.file_type().is_symlink() {
        return Ok(true);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt;
        use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        return Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0);
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}

fn is_terminal_journal_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|v| v.to_str()).unwrap_or("");
    name.starts_with("recovery-") && name.ends_with(".json")
}

fn unique_recovery_journal_temp_path(dir: &Path, stem: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    dir.join(format!(".{stem}.{nonce}.json.tmp"))
}

fn epoch_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

fn sync_parent_dir(parent: &Path) {
    if let Ok(dir) = fs::File::open(parent) {
        let _ = dir.sync_all();
    }
}

#[cfg(target_os = "windows")]
fn replace_file_atomic(temp_path: &Path, target_path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let temp: Vec<u16> = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let target: Vec<u16> = target_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        MoveFileExW(
            PCWSTR(temp.as_ptr()),
            PCWSTR(target.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(|_| "Failed to replace recovery journal atomically".to_string())
    }
}

#[cfg(not(target_os = "windows"))]
fn replace_file_atomic(temp_path: &Path, target_path: &Path) -> Result<(), String> {
    fs::rename(temp_path, target_path)
        .map_err(|error| format!("Failed to replace recovery journal atomically: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn journal_transitions_and_interrupts_stale_running() {
        let mut journal =
            RecoveryJournalEntry::new("op1".into(), "dest".into(), vec!["src".into()], 1000);
        journal.transition_to(RecoveryJournalState::CopyStarted, 1500);
        journal.transition_to(RecoveryJournalState::CopiedVerified, 1750);
        journal.transition_to(RecoveryJournalState::DeleteStarted, 2000);
        journal.mark_stale_running_as_interrupted(
            24 * 60 * 60 * 1000 + 3001,
            RECOVERY_JOURNAL_STALE_INTERRUPT_AFTER_MS,
        );
        assert_eq!(journal.state, RecoveryJournalState::Interrupted);
    }

    #[test]
    fn journal_schema_includes_required_fields() {
        let journal = RecoveryJournalEntry::new("op2".into(), "dest".into(), vec![], 42);
        let json = serde_json::to_string(&journal).unwrap();
        for field in [
            "mode",
            "sourceKind",
            "source",
            "destination",
            "selectedCollisionDestination",
            "sourceFileLength",
            "copiedFileBytes",
            "sourceManifest",
            "copiedManifest",
            "deleteStartedAtMs",
            "sourceRemovedAtMs",
            "completedAtMs",
            "interruptedAtMs",
        ] {
            assert!(json.contains(field), "missing {field}");
        }
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let root = std::env::temp_dir().join(format!("jasonshell-journal-{}", unique_suffix()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("recovery-op.json");
        fs::write(&path, b"old").unwrap();
        let journal = RecoveryJournalEntry::new("op".into(), "dest".into(), vec![], now_ms());
        write_recovery_journal_atomic(&path, &journal).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"version\": 1"));
    }

    #[test]
    fn cleanup_skips_unknown_artifacts_and_respects_emergency_disable() {
        let root =
            std::env::temp_dir().join(format!("jasonshell-journal-cleanup-{}", unique_suffix()));
        fs::create_dir_all(&root).unwrap();
        let known = root.join("recovery-old.json");
        let unknown = root.join("notes.txt");
        let mut journal = RecoveryJournalEntry::new(
            "op".into(),
            "dest".into(),
            vec![],
            now_ms() - 15 * 24 * 60 * 60 * 1000,
        );
        journal.transition_to(
            RecoveryJournalState::Completed,
            now_ms() - 15 * 24 * 60 * 60 * 1000,
        );
        fs::write(&known, serde_json::to_vec(&journal).unwrap()).unwrap();
        fs::write(&unknown, b"keep").unwrap();
        let removed =
            cleanup_recovery_journals(&root, now_ms() + 15 * 24 * 60 * 60 * 1000, false).unwrap();
        assert!(removed.iter().any(|path| path == &known));
        assert!(unknown.exists());
        fs::write(&known, serde_json::to_vec(&journal).unwrap()).unwrap();
        let removed_disabled =
            cleanup_recovery_journals(&root, now_ms() + 15 * 24 * 60 * 60 * 1000, true).unwrap();
        assert!(removed_disabled.is_empty());
        assert!(known.exists());
    }

    #[test]
    fn unique_temp_path_stays_in_dir_and_uses_unique_name() {
        let root = PathBuf::from(r"C:\temp\journal");
        let first = unique_recovery_journal_temp_path(&root, "recovery-op");
        let second = unique_recovery_journal_temp_path(&root, "recovery-op");
        assert_eq!(first.parent(), Some(root.as_path()));
        assert_eq!(second.parent(), Some(root.as_path()));
        assert_ne!(first, second);
        assert!(first
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(".json.tmp"));
    }

    #[test]
    fn operation_id_is_uniqueish() {
        let first = new_recovery_operation_id();
        let second = new_recovery_operation_id();
        assert_ne!(first, second);
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    fn unique_suffix() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }
}
