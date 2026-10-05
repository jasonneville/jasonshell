//! Generic resource is a Drop spy, not a fake claim of native GDI cleanup.
#[path = "../src/snipping/session.rs"]
mod session;

use session::{Action, Caller, SessionManager};
use std::{cell::Cell, rc::Rc};

struct Resource(Rc<Cell<usize>>);
impl Drop for Resource {
    fn drop(&mut self) { self.0.set(self.0.get() + 1); }
}
fn caller(label: &str, window_id: u64) -> Caller {
    Caller { label: label.into(), window_id }
}
fn resource(drops: &Rc<Cell<usize>>) -> Resource { Resource(Rc::clone(drops)) }

#[test]
fn snipping_duplicate_start_preserves_active_and_releases_rejected_resource() {
    let drops = Rc::new(Cell::new(0));
    let mut manager = SessionManager::default();
    let generation = manager.begin(resource(&drops), caller("snip-overlay", 10)).unwrap();
    assert!(manager.begin(resource(&drops), caller("snip-overlay", 11)).is_err());
    assert_eq!(drops.get(), 1);
    assert!(manager.authorize(generation, &caller("snip-overlay", 10), Action::Select).is_ok());
    manager.cancel(generation).unwrap();
    assert_eq!(drops.get(), 2);
}

#[test]
fn snipping_cancel_and_stale_completion_cannot_touch_new_generation() {
    let drops = Rc::new(Cell::new(0));
    let mut manager = SessionManager::default();
    let old = manager.begin(resource(&drops), caller("snip-overlay", 10)).unwrap();
    manager.cancel(old).unwrap();
    assert_eq!(drops.get(), 1);
    let current = manager.begin(resource(&drops), caller("snip-overlay", 20)).unwrap();
    assert_ne!(current, old);
    assert!(manager.cancel(old).is_err());
    assert!(manager.show_preview(old, caller("snip-preview", 30)).is_err());
    assert!(manager.finish(old).is_err());
    assert_eq!(drops.get(), 1);
    assert!(manager.authorize(current, &caller("snip-overlay", 20), Action::Select).is_ok());
    drop(manager);
    assert_eq!(drops.get(), 2);
}

#[test]
fn snipping_preview_finish_releases_owned_image_exactly_once() {
    let drops = Rc::new(Cell::new(0));
    let mut manager = SessionManager::default();
    let generation = manager.begin(resource(&drops), caller("snip-overlay", 10)).unwrap();
    assert!(manager.finish(generation).is_err());
    assert_eq!(drops.get(), 0);
    manager.show_preview(generation, caller("snip-preview", 30)).unwrap();
    assert_eq!(drops.get(), 0);
    assert!(manager.show_preview(generation, caller("snip-preview", 31)).is_err());
    manager.finish(generation).unwrap();
    assert_eq!(drops.get(), 1);
    assert!(manager.finish(generation).is_err());
    drop(manager);
    assert_eq!(drops.get(), 1);
}

#[test]
fn snipping_preview_cancel_releases_image_and_all_bindings() {
    let drops = Rc::new(Cell::new(0));
    let mut manager = SessionManager::default();
    let generation = manager.begin(resource(&drops), caller("snip-overlay", 10)).unwrap();
    let preview = caller("snip-preview", 30);
    manager.show_preview(generation, preview.clone()).unwrap();
    manager.cancel(generation).unwrap();
    assert_eq!(drops.get(), 1);
    assert!(manager.authorize(generation, &preview, Action::Copy).is_err());
    assert!(manager.cancel(generation).is_err());
    drop(manager);
    assert_eq!(drops.get(), 1);
}

#[test]
fn snipping_authorization_requires_bound_concrete_window_generation_and_phase() {
    let drops = Rc::new(Cell::new(0));
    let mut manager = SessionManager::default();
    let overlay = caller("snip-overlay", 10);
    let preview = caller("snip-preview", 30);
    let generation = manager.begin(resource(&drops), overlay.clone()).unwrap();
    for stranger in [caller("snip-overlay", 11), caller("top-bar", 10), caller("unknown", 10), preview.clone()] {
        for action in [Action::Select, Action::Cancel, Action::Copy, Action::Save, Action::Close] {
            assert!(manager.authorize(generation, &stranger, action).is_err());
        }
    }
    assert!(manager.authorize(generation, &overlay, Action::Select).is_ok());
    assert!(manager.authorize(generation, &overlay, Action::Cancel).is_ok());
    for action in [Action::Copy, Action::Save, Action::Close] {
        assert!(manager.authorize(generation, &overlay, action).is_err());
    }
    manager.show_preview(generation, preview.clone()).unwrap();
    for action in [Action::Select, Action::Cancel, Action::Copy, Action::Save, Action::Close] {
        assert!(manager.authorize(generation, &overlay, action).is_err());
    }
    for action in [Action::Copy, Action::Save, Action::Close] {
        assert!(manager.authorize(generation, &preview, action).is_ok());
        assert!(manager.authorize(generation, &caller("snip-preview", 31), action).is_err());
    }
    assert!(manager.authorize(generation, &preview, Action::Select).is_err());
    assert!(manager.authorize(generation + 1, &preview, Action::Copy).is_err());
    manager.cancel(generation).unwrap();
    assert!(manager.authorize(generation, &preview, Action::Copy).is_err());
    assert_eq!(drops.get(), 1);
}
