use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default)]
pub struct StackPopupRuntimeState {
    pub(crate) latest_request: Option<ShowStackPopupRequest>,
    pub(crate) clipboard: Option<StackClipboard>,
    pub(crate) focus_loss_hold_count: usize,
    pub(crate) restore_focus_after_hold: bool,
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
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackGitStatus {
    pub repository_root: String,
    pub branch: String,
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
