use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use super::{attention, native_hooks, windows};

const MAX_UNRESOLVED_SAMPLES: usize = 8;
const MAX_APP_ID_CACHE: usize = 128;
const MAX_UNRESOLVED_IDENTS: usize = 128;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToastListenerStatus {
    #[default]
    Starting,
    Allowed,
    Denied,
    Unavailable,
    Error,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskbarRuntimeDiagnostics {
    pub toast_listener_status: ToastListenerStatus,
    pub status_updated_at_ms: u64,
    pub snapshot: TaskbarRuntimeDiagnosticsSnapshot,
    pub package_identity_status: PackageIdentityStatus,
    pub current_process_package_identity: PackageIdentitySummary,
    pub explorer_taskbar_diagnostics: ExplorerTaskbarDiagnosticsSnapshot,
    pub last_success: Option<RedactedDiagnosticEvent>,
    pub last_failure: Option<RedactedDiagnosticEvent>,
    pub counters: TaskbarRuntimeCounters,
    pub known_app_id_count: usize,
    pub unresolved_app_id_count: usize,
    pub unresolved_sample: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskbarRuntimeDiagnosticsSnapshot {
    pub native_hooks: NativeHooksDiagnosticsSnapshot,
    pub snapshot_pipeline: SnapshotPipelineDiagnosticsSnapshot,
    pub attention: AttentionDiagnosticsSnapshot,
    pub toast: ToastDiagnosticsSnapshot,
    pub explorer: ExplorerTaskbarDiagnosticsSnapshot,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeHooksDiagnosticsSnapshot {
    pub health: NativeHooksHealthSnapshot,
    pub last_signal: Option<NativeHooksSignalSnapshot>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeHooksHealthSnapshot {
    pub shell_hook: String,
    pub win_event: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeHooksSignalSnapshot {
    pub signal: String,
    pub timestamp_ms: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotPipelineDiagnosticsSnapshot {
    pub sequence: u64,
    pub refresh_reason: String,
    pub refreshed_at_ms: u64,
    pub latency_ms: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionDiagnosticsSnapshot {
    pub tracked_count: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToastDiagnosticsSnapshot {
    pub status: ToastListenerStatus,
    pub last_poll_at_ms: u64,
    pub unresolved_count: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageIdentityStatus {
    pub available: bool,
    pub checked: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageIdentitySummary {
    pub available: bool,
    pub summary: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactedDiagnosticEvent {
    pub kind: String,
    pub message: String,
    pub timestamp_ms: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskbarRuntimeCounters {
    pub listener_start_attempts: u64,
    pub listener_poll_attempts: u64,
    pub listener_poll_successes: u64,
    pub listener_poll_failures: u64,
    pub denied_requests: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerTaskbarDiagnosticsSnapshot {
    pub tracked: u64,
    pub hidden: u64,
    pub recreation_failures: u64,
    pub hide_failures: u64,
    pub last_error: Option<String>,
}

#[derive(Default)]
struct DiagnosticsState {
    diagnostics: TaskbarRuntimeDiagnostics,
    known_app_ids: HashMap<String, Instant>,
    unresolved_app_ids: HashSet<String>,
    unresolved_order: VecDeque<String>,
    unresolved_overflow_count: usize,
}

static STATE: OnceLock<Mutex<DiagnosticsState>> = OnceLock::new();

fn state() -> &'static Mutex<DiagnosticsState> {
    STATE.get_or_init(|| Mutex::new(DiagnosticsState::default()))
}

pub(super) fn note_listener_start() {
    initialize_package_identity_diagnostics();
    let Ok(mut state) = state().lock() else {
        return;
    };
    state.diagnostics.counters.listener_start_attempts = state
        .diagnostics
        .counters
        .listener_start_attempts
        .saturating_add(1);
    set_listener_status(&mut state.diagnostics, ToastListenerStatus::Starting, true);
}

pub(super) fn initialize_package_identity_diagnostics() {
    if let Ok(mut state) = state().lock() {
        if state.diagnostics.package_identity_status.checked {
            return;
        }
        probe_current_process_package_identity(&mut state.diagnostics);
    }
}

pub(super) fn note_listener_status(status: ToastListenerStatus) {
    if let Ok(mut state) = state().lock() {
        set_listener_status(
            &mut state.diagnostics,
            status,
            status != ToastListenerStatus::Error,
        );
    }
}

pub(super) fn note_listener_poll_success() {
    if let Ok(mut s) = state().lock() {
        s.diagnostics.counters.listener_poll_attempts = s
            .diagnostics
            .counters
            .listener_poll_attempts
            .saturating_add(1);
        s.diagnostics.counters.listener_poll_successes = s
            .diagnostics
            .counters
            .listener_poll_successes
            .saturating_add(1);
        s.diagnostics.last_success = Some(RedactedDiagnosticEvent {
            kind: "poll".to_string(),
            message: "toast listener poll ok".to_string(),
            timestamp_ms: now_ms(),
        });
        set_listener_status(&mut s.diagnostics, ToastListenerStatus::Allowed, true);
    }
}

pub(super) fn note_listener_poll_failure(message: impl Into<String>) {
    if let Ok(mut s) = state().lock() {
        s.diagnostics.counters.listener_poll_attempts = s
            .diagnostics
            .counters
            .listener_poll_attempts
            .saturating_add(1);
        s.diagnostics.counters.listener_poll_failures = s
            .diagnostics
            .counters
            .listener_poll_failures
            .saturating_add(1);
        s.diagnostics.last_failure = Some(RedactedDiagnosticEvent {
            kind: "poll".to_string(),
            message: redact_text(&message.into()),
            timestamp_ms: now_ms(),
        });
        set_listener_status(&mut s.diagnostics, ToastListenerStatus::Error, false);
    }
}

pub(super) fn note_denied_request() {
    if let Ok(mut s) = state().lock() {
        s.diagnostics.counters.denied_requests =
            s.diagnostics.counters.denied_requests.saturating_add(1);
        s.diagnostics.status_updated_at_ms = now_ms();
        s.diagnostics.toast_listener_status = ToastListenerStatus::Denied;
        s.diagnostics.last_failure = Some(RedactedDiagnosticEvent {
            kind: "listener".to_string(),
            message: "toast listener access denied".to_string(),
            timestamp_ms: s.diagnostics.status_updated_at_ms,
        });
    }
}

pub(super) fn note_package_identity(
    available: bool,
    package_full_name: Option<String>,
    error: Option<String>,
) {
    if let Ok(mut s) = state().lock() {
        s.diagnostics.package_identity_status = PackageIdentityStatus {
            available,
            checked: true,
            error: error.map(|e| redact_text(&e)),
        };
        s.diagnostics.current_process_package_identity = PackageIdentitySummary {
            available,
            summary: package_full_name.map(|v| redact_package_name(&v)),
        };
    }
}

pub(super) fn note_resolved_app_id(app_id: &str) {
    if let Ok(mut s) = state().lock() {
        let redacted = redact_app_id(app_id);
        s.known_app_ids.insert(redacted.clone(), Instant::now());
        if s.unresolved_app_ids.remove(&redacted) {
            s.unresolved_order.retain(|id| id != &redacted);
        } else if s.unresolved_overflow_count > 0 {
            s.unresolved_overflow_count = s.unresolved_overflow_count.saturating_sub(1);
        }
        trim_known(&mut s);
        s.diagnostics.known_app_id_count = s.known_app_ids.len();
        sync_unresolved_counts(&mut s);
    }
}

pub(super) fn note_unresolved_app_id(app_id: &str) {
    if let Ok(mut s) = state().lock() {
        let redacted = redact_app_id(app_id);
        if s.known_app_ids.contains_key(&redacted) || s.unresolved_app_ids.contains(&redacted) {
            return;
        }
        if s.unresolved_app_ids.len() < MAX_UNRESOLVED_IDENTS {
            s.unresolved_app_ids.insert(redacted.clone());
            if s.unresolved_order.len() < MAX_UNRESOLVED_SAMPLES {
                s.unresolved_order.push_back(redacted);
            }
        } else {
            s.unresolved_overflow_count = s.unresolved_overflow_count.saturating_add(1);
        }
        s.diagnostics.known_app_id_count = s.known_app_ids.len();
        sync_unresolved_counts(&mut s);
    }
}

pub(super) fn taskbar_runtime_diagnostics() -> TaskbarRuntimeDiagnostics {
    let mut diagnostics = state()
        .lock()
        .map(|s| s.diagnostics.clone())
        .unwrap_or_default();
    diagnostics.snapshot = TaskbarRuntimeDiagnosticsSnapshot {
        native_hooks: native_hooks::native_hooks_diagnostics_snapshot(),
        snapshot_pipeline: windows::taskbar_snapshot_diagnostics_snapshot(),
        attention: attention::taskbar_attention_diagnostics_snapshot(),
        toast: ToastDiagnosticsSnapshot {
            status: diagnostics.toast_listener_status,
            last_poll_at_ms: diagnostics
                .last_success
                .as_ref()
                .filter(|event| event.kind == "poll")
                .map(|event| event.timestamp_ms)
                .unwrap_or(diagnostics.status_updated_at_ms),
            unresolved_count: diagnostics.unresolved_app_id_count,
        },
        explorer: diagnostics.explorer_taskbar_diagnostics.clone(),
    };
    diagnostics
}

pub(super) fn note_explorer_taskbar_reconcile(
    tracked: u64,
    hidden: u64,
    recreations: u64,
    hide_failures: u64,
    last_error: Option<&str>,
) {
    let Ok(mut state) = state().lock() else {
        return;
    };
    let diagnostics = &mut state.diagnostics.explorer_taskbar_diagnostics;
    diagnostics.tracked = tracked;
    diagnostics.hidden = hidden;
    diagnostics.recreation_failures = diagnostics.recreation_failures.saturating_add(recreations);
    diagnostics.hide_failures = diagnostics.hide_failures.saturating_add(hide_failures);
    diagnostics.last_error = last_error.map(redact_text);
}

fn trim_known(state: &mut DiagnosticsState) {
    while state.known_app_ids.len() > MAX_APP_ID_CACHE {
        if let Some(oldest) = state
            .known_app_ids
            .iter()
            .min_by_key(|(_, at)| *at)
            .map(|(k, _)| k.clone())
        {
            state.known_app_ids.remove(&oldest);
        } else {
            break;
        }
    }
}

fn sync_unresolved_counts(state: &mut DiagnosticsState) {
    state.diagnostics.unresolved_app_id_count = state
        .unresolved_app_ids
        .len()
        .saturating_add(state.unresolved_overflow_count);
    state.diagnostics.unresolved_sample = state
        .unresolved_order
        .iter()
        .take(MAX_UNRESOLVED_SAMPLES)
        .cloned()
        .collect();
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn probe_current_process_package_identity(diagnostics: &mut TaskbarRuntimeDiagnostics) {
    #[cfg(target_os = "windows")]
    {
        diagnostics.package_identity_status = PackageIdentityStatus {
            available: false,
            checked: true,
            error: None,
        };
        diagnostics.current_process_package_identity = PackageIdentitySummary::default();
    }
}

fn redact_text(input: &str) -> String {
    redact_paths_and_profile(input).chars().take(180).collect()
}
fn redact_package_name(input: &str) -> String {
    redact_paths_and_profile(input)
        .split('!')
        .next()
        .unwrap_or(input)
        .chars()
        .take(180)
        .collect()
}
fn redact_app_id(input: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("app#{:016x}", hasher.finish())
}

fn redact_paths_and_profile(input: &str) -> String {
    let normalized = input.replace('\\', "/");
    let profile = std::env::var("USERPROFILE")
        .ok()
        .filter(|p| !p.is_empty())
        .map(|p| p.replace('\\', "/"));
    redact_path_like(&normalized, profile.as_deref())
}

fn redact_path_like(input: &str, profile: Option<&str>) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some((len, repl)) = match_path_token(&input[i..], profile) {
            out.push_str(repl);
            i += len;
        } else {
            out.push(input[i..].chars().next().unwrap());
            i += input[i..].chars().next().unwrap().len_utf8();
        }
    }
    out
}

fn match_path_token<'a>(s: &'a str, profile: Option<&'a str>) -> Option<(usize, &'static str)> {
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("file:///") {
        return Some((path_token_len(s), "<path>"));
    }
    if s.starts_with("//") {
        return Some((path_token_len(s), "<path>"));
    }
    let b = s.as_bytes();
    if b.len() >= 3 && b[1] == b':' && (b[2] == b'/' || b[2] == b'\\') && b[0].is_ascii_alphabetic()
    {
        return Some((path_token_len(s), "<path>"));
    }
    if let Some(profile) = profile {
        if !profile.is_empty() && s.starts_with(profile) {
            return Some((profile.len(), "<user>"));
        }
    }
    None
}

fn path_token_len(input: &str) -> usize {
    input
        .char_indices()
        .find(|(_, ch)| ch.is_whitespace() || matches!(ch, '"' | '\'' | ',' | ';' | ')' | ']'))
        .map(|(index, _)| index)
        .unwrap_or(input.len())
}

fn set_listener_status(
    diagnostics: &mut TaskbarRuntimeDiagnostics,
    status: ToastListenerStatus,
    clear_failure: bool,
) {
    diagnostics.toast_listener_status = status;
    diagnostics.status_updated_at_ms = now_ms();
    if clear_failure {
        diagnostics.last_failure = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_complete_path_tokens() {
        let value = redact_paths_and_profile(
            "failed C:\\Users\\Alice\\Secret\\x.txt and file:///C:/Users/Alice/App/data.db",
        );
        assert_eq!(value, "failed <path> and <path>");
        assert!(!value.contains("Alice"));
        assert!(!value.contains("Secret"));
    }

    #[test]
    fn unresolved_count_is_independent_from_sample_capacity() {
        let mut state = DiagnosticsState::default();
        for index in 0..9 {
            let identity = format!("app#{index}");
            state.unresolved_app_ids.insert(identity.clone());
            if state.unresolved_order.len() < MAX_UNRESOLVED_SAMPLES {
                state.unresolved_order.push_back(identity);
            }
        }
        sync_unresolved_counts(&mut state);
        assert_eq!(state.diagnostics.unresolved_app_id_count, 9);
        assert_eq!(state.diagnostics.unresolved_sample.len(), 8);
    }
}
