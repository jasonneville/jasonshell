use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default)]
pub struct StackPopupRuntimeState {
    pub(crate) latest_request: Option<ShowStackPopupRequest>,
    pub(crate) clipboard: Option<StackClipboard>,
    pub(crate) focus_loss_hold_count: usize,
    pub(crate) focus_loss_suppression_expires_at_ms: Option<u64>,
    pub(crate) topmost_restore_suppression_expires_at_ms: Option<u64>,
    pub(crate) restore_focus_after_hold: bool,
    pub(crate) terminal_sessions: super::terminal::StackTerminalRegistry,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinnedStackFolder {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShowStackPopupRequest {
    pub path: String,
    pub anchor_left: f64,
    pub anchor_width: f64,
    #[serde(default)]
    pub request_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StackPopupLogicalSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackItem {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub type_label: String,
    pub icon_data_url: Option<String>,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<u64>,
    pub is_hidden: bool,
    pub is_readonly: bool,
    pub is_system: bool,
    pub is_symlink: bool,
    pub is_reparse_point: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFolderWarning {
    pub path: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFolderPage {
    pub path: String,
    pub sort_column: String,
    pub sort_direction: String,
    pub items: Vec<StackItem>,
    pub offset: usize,
    pub limit: usize,
    pub total: usize,
    pub has_more: bool,
    pub warnings: Vec<StackFolderWarning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<StackFolderPageDiagnostics>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StackGitFileStatusKind {
    Modified,
    Added,
    Deleted,
    Untracked,
    Conflict,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitFileStatus {
    pub path: String,
    pub relative_path: String,
    pub status: StackGitFileStatusKind,
    pub staged: bool,
    pub unstaged: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStatus {
    pub repository_root: String,
    pub branch: String,
    pub remote_repository_url: Option<String>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub modified: usize,
    pub added: usize,
    pub deleted: usize,
    pub untracked: usize,
    pub conflicts: usize,
    pub entries: Vec<StackGitFileStatus>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStageRequest {
    pub folder_path: String,
    pub paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitCommitRequest {
    pub folder_path: String,
    pub message: String,
    pub paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitOperationResult {
    pub repository_root: String,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitLogRequest {
    pub folder_path: String,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitLogEntry {
    pub commit_hash: String,
    pub short_hash: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_at: String,
    pub subject: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitLog {
    pub repository_root: String,
    pub entries: Vec<StackGitLogEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitCommitFilesRequest {
    pub folder_path: String,
    pub commit_hash: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitCommitFile {
    pub path: String,
    pub relative_path: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitCommitFiles {
    pub repository_root: String,
    pub commit_hash: String,
    pub files: Vec<StackGitCommitFile>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitCommitFileDiffRequest {
    pub folder_path: String,
    pub commit_hash: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitCommitFileDiff {
    pub repository_root: String,
    pub commit_hash: String,
    pub path: String,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStashFilesRequest {
    pub folder_path: String,
    pub stash_ref: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStashFile {
    pub path: String,
    pub relative_path: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStashFiles {
    pub repository_root: String,
    pub stash_ref: String,
    pub files: Vec<StackGitStashFile>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStashFileDiffRequest {
    pub folder_path: String,
    pub stash_ref: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStashFileDiff {
    pub repository_root: String,
    pub stash_ref: String,
    pub path: String,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitTreeRequest {
    pub folder_path: String,
    pub treeish: Option<String>,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitTreeEntry {
    pub mode: String,
    pub kind: String,
    pub object_hash: String,
    pub size_bytes: Option<u64>,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitTree {
    pub repository_root: String,
    pub treeish: String,
    pub entries: Vec<StackGitTreeEntry>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitBranch {
    pub name: String,
    pub ref_name: String,
    pub current: bool,
    pub remote: bool,
    pub checked_out_elsewhere: bool,
    pub checked_out_elsewhere_path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitBranches {
    pub repository_root: String,
    pub current_branch: Option<String>,
    pub branches: Vec<StackGitBranch>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitBranchRequest {
    pub folder_path: String,
    pub branch_name: String,
    pub checkout: Option<bool>,
    pub source_branch: Option<String>,
    pub force: Option<bool>,
    pub remove_worktree: Option<bool>,
    pub worktree_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitDiffRequest {
    pub folder_path: String,
    pub path: String,
    pub staged: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitRevertRequest {
    pub folder_path: String,
    pub paths: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitDiff {
    pub repository_root: String,
    pub path: String,
    pub staged: bool,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStashRequest {
    pub folder_path: String,
    pub message: Option<String>,
    pub include_untracked: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStashEntry {
    pub stash_ref: String,
    #[serde(rename = "ref")]
    pub ref_: String,
    pub index: usize,
    pub branch: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStashes {
    pub repository_root: String,
    pub entries: Vec<StackGitStashEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStashRefRequest {
    pub folder_path: String,
    pub stash_ref: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFolderPageDiagnostics {
    pub folder_open_duration_ms: u128,
    pub page_duration_ms: u128,
    pub page_item_count: usize,
    pub icon_resolution_count: usize,
    pub icon_resolution_duration_ms: u128,
    pub icon_cache_hits: usize,
    pub icon_cache_misses: usize,
    pub icon_fallback_count: usize,
    pub payload_item_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackItemIconResolution {
    pub path: String,
    pub icon_data_url: Option<String>,
    pub cache_hit: bool,
    pub resolution_duration_ms: u128,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackItemIconResolutionBatch {
    pub items: Vec<StackItemIconResolution>,
    pub requested_count: usize,
    pub resolved_count: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub truncated: bool,
    pub max_batch_size: usize,
    pub total_duration_ms: u128,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackPasteResult {
    pub pasted: Vec<StackItem>,
    pub failures: Vec<StackPasteFailure>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackOpenWithCandidate {
    pub id: String,
    pub label: String,
    pub executable: String,
    pub source: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackNativeDragPreparation {
    pub paths: Vec<String>,
    pub effect: String,
    pub mechanism: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackPasteFailure {
    pub path: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClipboardMode {
    Copy,
    Cut,
}

#[derive(Clone, Debug)]
pub(crate) struct StackClipboard {
    pub(crate) mode: ClipboardMode,
    pub(crate) paths: Vec<PathBuf>,
}
