//! Production-used generic owner exercised with Drop probes, no HWND/DWM/shell.
#[path = "../src/task_gallery/resources.rs"]
mod resources;
use resources::{ResourceOwner, ResourceSet};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
struct Generation(&'static str, u64, u64);
struct Probe { id: &'static str, dropped: Arc<Mutex<Vec<&'static str>>> }
impl Drop for Probe { fn drop(&mut self) { self.dropped.lock().unwrap().push(self.id); } }
fn probe(id: &'static str, dropped: &Arc<Mutex<Vec<&'static str>>>) -> Probe {
    Probe { id, dropped: Arc::clone(dropped) }
}
fn set(id: &'static str, dropped: &Arc<Mutex<Vec<&'static str>>>) -> ResourceSet<Probe> {
    let mut result = ResourceSet::default(); result.replace("101".into(), probe(id, dropped)); result
}
fn drops(dropped: &Arc<Mutex<Vec<&'static str>>>) -> Vec<&'static str> { dropped.lock().unwrap().clone() }

#[test]
fn replacement_removal_and_visibility_omissions_transfer_without_early_drop() {
    let dropped = Arc::default(); let mut frames = ResourceSet::default();
    frames.replace("101".into(), probe("old", &dropped));
    let retired = frames.replace("101".into(), probe("new", &dropped));
    frames.replace("102".into(), probe("offscreen", &dropped));
    assert!(drops(&dropped).is_empty()); drop(retired); assert_eq!(drops(&dropped), ["old"]);
    let omitted = frames.retain_visible(["101"]);
    assert_eq!(frames.len(), 1); assert_eq!(omitted.len(), 1);
    assert_eq!(frames.get("101").unwrap().id, "new");
    assert_eq!(drops(&dropped), ["old"]); drop(omitted);
    assert_eq!(drops(&dropped), ["old", "offscreen"]);
    let removed = frames.remove("101"); assert!(frames.is_empty());
    assert_eq!(drops(&dropped), ["old", "offscreen"]); drop(removed);
    assert_eq!(drops(&dropped), ["old", "offscreen", "new"]);
    assert!(frames.remove("101").is_none());
}

#[test]
fn newer_generation_reuses_only_visible_resources_and_empty_completion_clears() {
    let dropped = Arc::default(); let mut owner = ResourceOwner::default();
    let a = Generation("A", 1, 1); let b = Generation("A", 1, 2);
    owner.advance(a.clone()); let first = owner.finish(&a, set("live", &dropped));
    assert!(first.accepted); drop(first.retired);
    owner.advance(b.clone()); assert!(owner.take_if_current(&a).is_none());
    let mut reused = owner.take_if_current(&b).unwrap();
    assert!(owner.is_empty()); assert_eq!(reused.get("101").unwrap().id, "live");
    let omitted = reused.retain_visible(std::iter::empty());
    assert!(reused.is_empty()); assert!(drops(&dropped).is_empty());
    let completion = owner.finish(&b, reused); assert!(completion.accepted);
    assert!(owner.is_empty()); drop(completion.retired); drop(omitted);
    assert_eq!(drops(&dropped), ["live"]);
}

#[test]
fn stale_completion_after_newer_commit_cannot_drop_or_replace_newer_registration() {
    for newer in [Generation("B", 1, 1), Generation("A", 2, 1), Generation("A", 1, 2)] {
        let dropped = Arc::default(); let owner = Mutex::new(ResourceOwner::default());
        let old = Generation("A", 1, 1);
        owner.lock().unwrap().advance(old.clone());
        let staged = set("stale", &dropped);
        let (current_retired, stale_retired) = {
            let mut guard = owner.lock().unwrap(); guard.advance(newer.clone());
            let current = guard.finish(&newer, set("current", &dropped)); assert!(current.accepted);
            let stale = guard.finish(&old, staged); assert!(!stale.accepted);
            assert_eq!(guard.get("101").unwrap().id, "current"); assert_eq!(guard.len(), 1);
            assert!(drops(&dropped).is_empty()); (current.retired, stale.retired)
        };
        // Returned handles are disposed only after runtime guard releases.
        assert!(owner.try_lock().is_ok()); drop(current_retired); drop(stale_retired);
        assert_eq!(drops(&dropped), ["stale"]);
        let retired = owner.lock().unwrap().clear(); assert!(owner.try_lock().is_ok()); drop(retired);
        assert_eq!(drops(&dropped), ["stale", "current"]);
    }
}

#[test]
fn cancellation_revokes_staged_completion_without_dropping_committed_inside_lock() {
    let dropped = Arc::default(); let owner = Mutex::new(ResourceOwner::default());
    let generation = Generation("A", 1, 1);
    { let mut guard = owner.lock().unwrap(); guard.advance(generation.clone());
      assert!(guard.finish(&generation, set("committed", &dropped)).accepted); }
    let candidate = set("staged", &dropped);
    let retired = { let mut guard = owner.lock().unwrap(); guard.cancel();
        assert!(!guard.is_current(&generation)); assert_eq!(guard.len(), 1);
        assert!(drops(&dropped).is_empty());
        let completion = guard.finish(&generation, candidate); assert!(!completion.accepted); completion.retired };
    assert!(owner.try_lock().is_ok()); drop(retired); assert_eq!(drops(&dropped), ["staged"]);
    let committed = owner.lock().unwrap().clear(); assert!(drops(&dropped).iter().all(|id| *id != "committed"));
    drop(committed); assert_eq!(drops(&dropped), ["staged", "committed"]);
}

#[test]
fn current_failure_can_take_cleanup_but_stale_failure_leaves_newer_handles_owned() {
    let dropped = Arc::default(); let mut owner = ResourceOwner::default();
    let old = Generation("A", 1, 1); let current = Generation("A", 1, 2);
    owner.advance(current.clone()); assert!(owner.finish(&current, set("new", &dropped)).accepted);
    assert!(owner.take_if_current(&old).is_none()); assert_eq!(owner.get("101").unwrap().id, "new");
    let failed = owner.take_if_current(&current).unwrap(); assert!(owner.is_empty());
    assert!(drops(&dropped).is_empty()); drop(failed); assert_eq!(drops(&dropped), ["new"]);
    assert!(owner.take_if_current(&current).unwrap().is_empty());
}

#[test]
fn host_clear_member_removal_and_repeated_cleanup_dispose_exactly_once() {
    let dropped = Arc::default(); let mut owner = ResourceOwner::default();
    let generation = Generation("A", 1, 1); owner.advance(generation.clone());
    let mut candidate = set("one", &dropped); candidate.replace("102".into(), probe("two", &dropped));
    assert!(owner.finish(&generation, candidate).accepted);
    let removed = owner.remove("101"); assert_eq!(owner.len(), 1); assert!(drops(&dropped).is_empty());
    drop(removed); let retired = owner.clear(); assert!(!owner.is_current(&generation));
    assert!(owner.is_empty()); assert_eq!(drops(&dropped), ["one"]); drop(retired);
    drop(owner.clear()); drop(owner.remove("102")); drop(owner);
    assert_eq!(drops(&dropped), ["one", "two"]);
}

#[test]
fn same_session_revision_advance_preserves_registration_identity_until_reuse() {
    let dropped = Arc::default();
    let mut owner = ResourceOwner::default();
    let initial = Generation("A", 1, 1);
    owner.advance(initial.clone());
    assert!(owner.finish(&initial, set("live", &dropped)).accepted);
    let identity = owner.get("101").unwrap() as *const Probe;
    let refresh = Generation("A", 2, 2);
    owner.advance(refresh.clone());
    assert_eq!(owner.get("101").unwrap() as *const Probe, identity);
    assert!(drops(&dropped).is_empty());
    assert!(owner.take_if_current(&initial).is_none());
    let reusable = owner.take_if_current(&refresh).unwrap();
    assert_eq!(reusable.get("101").unwrap().id, "live");
    assert!(owner.finish(&refresh, reusable).accepted);
    assert!(drops(&dropped).is_empty());
    drop(owner.clear());
    assert_eq!(drops(&dropped), ["live"]);
}
