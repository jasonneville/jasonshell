//! REAL native registry and composition boundary functions; no Tauri app, HWND,
//! observer/hook, compositor child, native clipboard or desktop capture is created.
#![cfg(windows)]
#[path = "../src/snipping/geometry.rs"] mod geometry;
#[path = "../src/snipping/session.rs"] mod session;
#[path = "../src/snipping/clipboard.rs"] mod clipboard;
#[path = "../src/snipping/clipboard_process.rs"] mod clipboard_process;
#[path = "../src/snipping/save.rs"] mod save;
#[path = "../src/snipping/coordinator.rs"] mod coordinator;
#[path = "../src/snipping/native_registry.rs"] mod native_registry;
#[path = "../src/snipping/composition.rs"] mod composition;
use coordinator::*;
use native_registry::NativeRegistry;
use std::{sync::{Arc,mpsc},thread,time::{Duration,Instant}};
fn token() -> Token { Token { generation:"7".into(),capture_id:"77777777777777777777777777777777".into() } }
fn context(monitor:&str) -> Context { Context { token:token(),phase:Phase::Selecting,monitor_id:monitor.into(),width:2,height:2,scale_factor:1.25,origin_x:-1000,origin_y:0 } }
#[test]
fn actual_registry_exact_instance_monitor_and_generation_bindings_reject_forgery() {
    let registry=NativeRegistry::default();
    let entry=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    assert!(registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).is_err());
    let mut forged=entry.reference.clone(); forged.caller.window_id+=1;
    assert!(matches!(registry.entry(&forged),Err(SnipError::Unauthorized)));
    forged=entry.reference.clone(); forged.monitor_id=Some("m1".into());
    assert!(matches!(registry.entry(&forged),Err(SnipError::Unauthorized)));
    assert_eq!(entry.commit(&context("m1")),Err(SnipError::Unauthorized));
    let mut stale=context("m0"); stale.token.generation="6".into();
    assert_eq!(entry.commit(&stale),Err(SnipError::Unauthorized));
    entry.commit(&context("m0")).unwrap();
    assert_eq!(registry.context(entry.reference.caller.clone()).unwrap().monitor_id,"m0");
}
#[test]
fn actual_registry_reused_label_cannot_inherit_old_epoch_or_buffered_context() {
    let registry=NativeRegistry::default();
    let old=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    old.commit(&context("m0")).unwrap(); old.invalidate();
    let newer=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    assert_ne!(old.reference.caller.window_id,newer.reference.caller.window_id);
    assert!(matches!(registry.entry(&old.reference),Err(SnipError::Unauthorized)));
    assert_eq!(old.commit(&context("m0")),Err(SnipError::Stale));
    // Actual HWND binding/resolution is deliberately not invoked by this OS-free
    // registry test. Native window epoch proof is a separate required affordance.
}
#[test]
fn actual_registry_context_wait_does_not_hold_registry_lock_and_commit_buffers_result() {
    let registry=Arc::new(NativeRegistry::default());
    let entry=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    let (tx,rx)=mpsc::channel(); let (r,c)=(registry.clone(),entry.reference.caller.clone());
    let worker=thread::spawn(move|| { tx.send(r.context(c)).unwrap(); });
    let before=Instant::now();
    registry.reserve("snip-overlay-7-m1".into(),Some(token()),Some("m1".into()),false).unwrap();
    assert!(before.elapsed()<Duration::from_millis(100));
    entry.commit(&context("m0")).unwrap();
    assert_eq!(rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap().token,token()); worker.join().unwrap();
    assert_eq!(registry.context(entry.reference.caller.clone()).unwrap().token,token());
}
#[test]
fn actual_registry_invalidation_wakes_waiter() {
    let registry=Arc::new(NativeRegistry::default());
    let entry=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    let (tx,rx)=mpsc::channel(); let (r,c)=(registry.clone(),entry.reference.caller.clone());
    let worker=thread::spawn(move|| { tx.send(r.context(c)).unwrap(); });
    entry.invalidate();
    assert!(matches!(rx.recv_timeout(Duration::from_secs(1)).unwrap(),Err(SnipError::Inactive))); worker.join().unwrap();
}
#[test]
fn actual_registry_uninitialized_context_has_bounded_five_second_timeout() {
    let registry=NativeRegistry::default();
    let entry=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    let before=Instant::now();
    assert!(matches!(registry.context(entry.reference.caller.clone()),Err(SnipError::ReadinessTimeout)));
    assert!(before.elapsed()>=Duration::from_millis(4900)); assert!(before.elapsed()<Duration::from_secs(7));
}
#[test]
fn actual_registry_pending_generation_retirement_blocks_late_construction_but_not_newer_retry() {
    let registry=NativeRegistry::default();
    let old=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    old.admit_construction_at(Instant::now()).unwrap();
    assert!(registry.retire_pending_generation(&token()).is_empty(),"unbound pending generation has no HWND effects");
    assert!(old.admit_construction_at(Instant::now()).is_err());
    assert_eq!(old.commit(&context("m0")),Err(SnipError::Stale));
    assert!(matches!(registry.reserve("late-old".into(),Some(token()),Some("m0".into()),false),Err(SnipError::Stale)));
    let mut newer_token=token();newer_token.generation="8".into();
    let newer=registry.reserve("snip-overlay-8-m0".into(),Some(newer_token.clone()),Some("m0".into()),false).unwrap();
    newer.admit_construction_at(Instant::now()).unwrap();
    assert!(registry.retire_pending_generation(&token()).is_empty(),"stale repeated cleanup owns no newer entry");
    newer.admit_construction_at(Instant::now()).unwrap();
    let mut newer_context=context("m0");newer_context.token=newer_token.clone();
    newer.commit(&newer_context).unwrap();
    assert_eq!(registry.context(newer.reference.caller.clone()).unwrap().token,newer_token);
    assert!(old.admit_construction_at(Instant::now()).is_err());
}
#[test]
fn actual_registry_expired_pending_construction_refuses_without_native_effects() {
    let registry=NativeRegistry::default();
    let entry=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    assert_eq!(entry.admit_construction_at(Instant::now()+Duration::from_secs(6)),Err(SnipError::ReadinessTimeout));
    assert!(entry.retire().is_none());
    assert!(entry.admit_construction_at(Instant::now()).is_err());
}
#[test]
fn actual_registry_retire_wakes_waiter_and_rejects_late_context_publication() {
    let registry=Arc::new(NativeRegistry::default());
    let entry=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    let (tx,rx)=mpsc::channel();let (r,c)=(registry.clone(),entry.reference.caller.clone());
    let worker=thread::spawn(move||{tx.send(r.context(c)).unwrap();});
    assert!(entry.retire().is_none(),"unbound entry owns no native window");
    assert!(matches!(rx.recv_timeout(Duration::from_secs(1)).unwrap(),Err(SnipError::Inactive)));
    worker.join().unwrap();
    assert_eq!(entry.commit(&context("m0")),Err(SnipError::Stale));
    assert!(matches!(entry.window(),Err(SnipError::Unauthorized)));
    assert!(entry.retire().is_none(),"retirement must be idempotent");
    let newer=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    assert_ne!(newer.reference.caller,entry.reference.caller);
    assert!(matches!(registry.entry(&entry.reference),Err(SnipError::Unauthorized)));
    newer.commit(&context("m0")).unwrap();
    assert_eq!(registry.context(newer.reference.caller.clone()).unwrap().token,token());
}
#[test]
fn actual_registry_unbound_registration_never_grants_foreground_or_helper_authority() {
    let registry=NativeRegistry::default();
    let normal=registry.reserve("process-manager".into(),None,None,false).unwrap();
    let helper=registry.reserve("snip-overlay-7-m0".into(),Some(token()),Some("m0".into()),false).unwrap();
    registry.bind_origin_generation(token()).unwrap();
    for hwnd in [0,99,200] {
        assert!(registry.foreground_identity(hwnd).is_none());
        assert!(registry.helper_token(hwnd).is_none());
        assert!(!registry.owns_foreground(helper.reference.caller.window_id,hwnd));
    }
    assert!(normal.retire().is_none());assert!(helper.retire().is_none());
    registry.close_snips();
    assert!(matches!(registry.reserve("retry".into(),None,None,false),Err(SnipError::Inactive)));
}
#[test]
fn actual_composition_expired_deadline_refuses_before_child_or_compositor_effect() {
    let before=Instant::now();
    assert_eq!(composition::flush(before-Duration::from_millis(1)),Err(SnipError::ReadinessTimeout));
    assert!(before.elapsed()<Duration::from_millis(100));
}
