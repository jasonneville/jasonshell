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
    fn creation_times_match(
        record_creation_time: Option<u64>,
        identity_creation_time: Option<u64>,
    ) -> bool {
        match (record_creation_time, identity_creation_time) {
            (Some(left), Some(right)) => left == right,
            _ => true,
        }
    }

    fn identities_match(record: &TaskbarAttentionKey, identity: &TaskbarAttentionIdentity) -> bool {
        record.root_owner_hwnd == identity.root_owner_hwnd
            && record.process_id == identity.process_id
            && Self::creation_times_match(record.creation_time, identity.creation_time)
    }

    fn visible_identity_matches(
        record: &TaskbarAttentionKey,
        identity: &TaskbarAttentionIdentity,
    ) -> bool {
        Self::identities_match(record, identity)
    }

    pub fn request(&mut self, identity: TaskbarAttentionIdentity) {
        self.records
            .entry(TaskbarAttentionKey::from(&identity))
            .and_modify(|record| record.attention_state = TaskbarWindowAttentionState::Requested)
            .or_insert(AttentionRecord {
                attention_state: TaskbarWindowAttentionState::Requested,
            });
    }

    pub fn clear(&mut self, identity: &TaskbarAttentionIdentity) {
        for (key, record) in &mut self.records {
            if Self::identities_match(key, identity) {
                record.attention_state = TaskbarWindowAttentionState::Idle;
            }
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
        self.records.retain(|key, _| {
            visible
                .iter()
                .any(|identity| Self::visible_identity_matches(key, identity))
        });
    }

    pub fn state_for(&self, identity: &TaskbarAttentionIdentity) -> TaskbarWindowAttentionState {
        self.records
            .get(&TaskbarAttentionKey::from(identity))
            .or_else(|| {
                if identity.creation_time.is_some() {
                    self.records
                        .iter()
                        .find(|(key, _)| {
                            key.root_owner_hwnd == identity.root_owner_hwnd
                                && key.process_id == identity.process_id
                                && key.creation_time.is_none()
                                && Self::identities_match(key, identity)
                        })
                        .map(|(_, record)| record)
                } else {
                    None
                }
            })
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

    #[test]
    fn reconcile_keeps_none_creation_time_request_when_visible_snapshot_gains_creation_time() {
        let requested = identity(7, 8, None);
        let visible = identity(7, 8, Some(99));
        let mut store = AttentionStore::default();
        store.request(requested.clone());

        store.reconcile(std::slice::from_ref(&visible));

        assert_eq!(
            store.state_for(&requested),
            TaskbarWindowAttentionState::Requested
        );
        assert_eq!(
            store.state_for(&visible),
            TaskbarWindowAttentionState::Requested
        );
    }

    #[test]
    fn unequal_known_creation_times_do_not_match() {
        let earlier = identity(9, 10, Some(1));
        let later = identity(9, 10, Some(2));
        let mut store = AttentionStore::default();
        store.request(earlier.clone());

        assert_eq!(store.state_for(&later), TaskbarWindowAttentionState::Idle);
        store.reconcile(std::slice::from_ref(&later));
        assert_eq!(store.state_for(&earlier), TaskbarWindowAttentionState::Idle);
    }

    #[test]
    fn clear_with_known_creation_time_clears_provisional_request() {
        let requested = identity(11, 12, None);
        let foreground = identity(11, 12, Some(123));
        let mut store = AttentionStore::default();
        store.request(requested.clone());

        store.clear(&foreground);

        assert_eq!(
            store.state_for(&requested),
            TaskbarWindowAttentionState::Idle
        );
        assert_eq!(
            store.state_for(&foreground),
            TaskbarWindowAttentionState::Idle
        );
    }
}
