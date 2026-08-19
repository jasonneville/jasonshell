use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::{actions::TaskWindowIdentity, TaskbarWindowAttentionState};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskbarAttentionIdentity {
    pub root_owner_hwnd: isize,
    pub process_id: u32,
    pub creation_time: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaskbarAttentionKey {
    pub root_owner_hwnd: isize,
    pub process_id: u32,
    pub creation_time: Option<u64>,
}

impl From<&TaskbarAttentionIdentity> for TaskbarAttentionKey {
    fn from(value: &TaskbarAttentionIdentity) -> Self {
        Self {
            root_owner_hwnd: value.root_owner_hwnd,
            process_id: value.process_id,
            creation_time: value.creation_time,
        }
    }
}

#[derive(Clone, Debug)]
struct AttentionRecord {
    attention_state: TaskbarWindowAttentionState,
}

#[derive(Clone, Debug)]
pub struct AttentionStore {
    records: HashMap<TaskbarAttentionKey, AttentionRecord>,
}

impl Default for AttentionStore {
    fn default() -> Self {
        Self {
            records: HashMap::new(),
        }
    }
}

impl AttentionStore {
    pub fn request(&mut self, identity: TaskbarAttentionIdentity) {
        self.records
            .entry(TaskbarAttentionKey::from(&identity))
            .and_modify(|record| record.attention_state = TaskbarWindowAttentionState::Requested)
            .or_insert(AttentionRecord {
                attention_state: TaskbarWindowAttentionState::Requested,
            });
    }

    pub fn clear(&mut self, identity: &TaskbarAttentionIdentity) {
        if let Some(record) = self.records.get_mut(&TaskbarAttentionKey::from(identity)) {
            record.attention_state = TaskbarWindowAttentionState::Idle;
        }
    }

    pub fn remove(&mut self, identity: &TaskbarAttentionIdentity) {
        self.records.remove(&TaskbarAttentionKey::from(identity));
    }

    pub fn remove_root_owner(&mut self, root_owner_hwnd: isize) {
        self.records
            .retain(|key, _| key.root_owner_hwnd != root_owner_hwnd);
    }

    pub fn reconcile(&mut self, visible: &[TaskbarAttentionIdentity]) {
        let visible: std::collections::HashSet<_> =
            visible.iter().map(TaskbarAttentionKey::from).collect();
        self.records.retain(|key, _| visible.contains(key));
    }

    pub fn state_for(&self, identity: &TaskbarAttentionIdentity) -> TaskbarWindowAttentionState {
        self.records
            .get(&TaskbarAttentionKey::from(identity))
            .map(|record| record.attention_state)
            .unwrap_or(TaskbarWindowAttentionState::Idle)
    }
}

static ATTENTION_STATE: OnceLock<Mutex<AttentionStore>> = OnceLock::new();

pub fn taskbar_attention_identity_from_window(
    root_owner_hwnd: isize,
    identity: Option<&TaskWindowIdentity>,
) -> TaskbarAttentionIdentity {
    TaskbarAttentionIdentity {
        root_owner_hwnd,
        process_id: identity.map(|value| value.process_id).unwrap_or_default(),
        creation_time: identity.map(|value| value.creation_time),
    }
}

pub fn record_taskbar_attention(identity: TaskbarAttentionIdentity, requested: bool) {
    let state = ATTENTION_STATE.get_or_init(|| Mutex::new(AttentionStore::default()));
    if let Ok(mut store) = state.lock() {
        if requested {
            store.request(identity);
        } else {
            store.clear(&identity);
        }
    }
}

pub fn clear_taskbar_attention_if_matches(identity: &TaskbarAttentionIdentity) {
    let state = ATTENTION_STATE.get_or_init(|| Mutex::new(AttentionStore::default()));
    if let Ok(mut store) = state.lock() {
        store.clear(identity);
    }
}

pub fn clear_taskbar_attention(identity: &TaskbarAttentionIdentity) {
    let state = ATTENTION_STATE.get_or_init(|| Mutex::new(AttentionStore::default()));
    if let Ok(mut store) = state.lock() {
        store.remove(identity);
    }
}

pub fn remove_root_owner_taskbar_attention(root_owner_hwnd: isize) {
    let state = ATTENTION_STATE.get_or_init(|| Mutex::new(AttentionStore::default()));
    if let Ok(mut store) = state.lock() {
        store.remove_root_owner(root_owner_hwnd);
    }
}

pub fn reconcile_taskbar_attention(visible: &[TaskbarAttentionIdentity]) {
    let state = ATTENTION_STATE.get_or_init(|| Mutex::new(AttentionStore::default()));
    if let Ok(mut store) = state.lock() {
        store.reconcile(visible);
    }
}

pub fn attention_state_for(identity: &TaskbarAttentionIdentity) -> TaskbarWindowAttentionState {
    ATTENTION_STATE
        .get_or_init(|| Mutex::new(AttentionStore::default()))
        .lock()
        .ok()
        .map(|store| store.state_for(identity))
        .unwrap_or(TaskbarWindowAttentionState::Idle)
}

pub fn taskbar_attention_diagnostics_snapshot() -> super::diagnostics::AttentionDiagnosticsSnapshot
{
    let tracked_count = ATTENTION_STATE
        .get_or_init(|| Mutex::new(AttentionStore::default()))
        .lock()
        .ok()
        .map(|store| store.records.len())
        .unwrap_or(0);
    super::diagnostics::AttentionDiagnosticsSnapshot { tracked_count }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(hwnd: isize, pid: u32, creation_time: Option<u64>) -> TaskbarAttentionIdentity {
        TaskbarAttentionIdentity {
            root_owner_hwnd: hwnd,
            process_id: pid,
            creation_time,
        }
    }

    #[test]
    fn request_sets_requested_and_is_idempotent() {
        let id = identity(5, 9, Some(99));
        let mut store = AttentionStore::default();
        store.request(id.clone());
        store.request(id.clone());
        assert_eq!(store.state_for(&id), TaskbarWindowAttentionState::Requested);
    }

    #[test]
    fn clear_removes_record() {
        let id = identity(6, 10, Some(100));
        let mut store = AttentionStore::default();
        store.request(id.clone());
        store.clear(&id);
        assert_eq!(store.state_for(&id), TaskbarWindowAttentionState::Idle);
    }

    #[test]
    fn reconcile_removes_stale_entries() {
        let keep = identity(1, 1, Some(1));
        let stale = identity(2, 2, Some(2));
        let mut store = AttentionStore::default();
        store.request(keep.clone());
        store.request(stale.clone());
        store.reconcile(std::slice::from_ref(&keep));
        assert_eq!(
            store.state_for(&keep),
            TaskbarWindowAttentionState::Requested
        );
        assert_eq!(store.state_for(&stale), TaskbarWindowAttentionState::Idle);
    }

    #[test]
    fn clear_only_matches_exact_identity() {
        let keep = identity(1, 1, Some(1));
        let other = identity(1, 2, Some(1));
        let mut store = AttentionStore::default();
        store.request(keep.clone());
        store.request(other.clone());
        store.clear(&keep);
        assert_eq!(store.state_for(&keep), TaskbarWindowAttentionState::Idle);
        assert_eq!(
            store.state_for(&other),
            TaskbarWindowAttentionState::Requested
        );
    }

    #[test]
    fn remove_root_owner_is_idempotent() {
        let keep = identity(1, 1, Some(1));
        let remove = identity(2, 2, Some(2));
        let mut store = AttentionStore::default();
        store.request(keep.clone());
        store.request(remove.clone());
        store.remove_root_owner(2);
        store.remove_root_owner(2);
        assert_eq!(
            store.state_for(&keep),
            TaskbarWindowAttentionState::Requested
        );
        assert_eq!(store.state_for(&remove), TaskbarWindowAttentionState::Idle);
    }
}
