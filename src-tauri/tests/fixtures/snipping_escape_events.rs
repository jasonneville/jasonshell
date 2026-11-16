// Included inside the actual runtime acceptance module. No copied router,
// boolean-only cancellation adapter, OS hook, injected global input, or sleep.
// Requested owner seam (equivalent private aliases acceptable):
// escape::EventRouter::new(Weak<SnipRuntime>, FnOnce cleanup) -> Arc<EventRouter>
// router.route(native_message: u32, virtual_key: u32) -> Option<PendingEscape>
// PendingEscape::deliver(self) -> bool; router.shutdown() is idempotent.
// The actual native callback MUST use this router and enqueue its opaque delivery.
// Generation is captured at keydown; the exact still-current token is resolved
// only for that generation at delivery. Renderer input never supplies authority.
// Listener shutdown/drop invokes its owned native-unsubscribe boundary exactly once
// and invalidates queued deliveries. Source must not own a strong Runtime cycle.
use crate::snipping::escape::EventRouter;
use std::sync::atomic::{AtomicUsize, Ordering as EscapeOrdering};
use windows::Win32::UI::WindowsAndMessaging::{WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP};

fn escape_event_router(f:&Fixture)->(Arc<EventRouter>,Arc<AtomicUsize>) {
    let cleaned=Arc::new(AtomicUsize::new(0));let cleanup=cleaned.clone();
    let router=EventRouter::new(Arc::downgrade(&f.runtime),move||{
        cleanup.fetch_add(1,EscapeOrdering::SeqCst);
    });
    (router,cleaned)
}
const ESCAPE_KEY:u32=0x1b;

#[test]
fn snip_runtime_escape_events_concurrent_cancel_arm_boundary_never_loses_escape() {
    // No production pause point exists inside cancellation admission. Queue both
    // real operations behind the actual mutex, then contend repeatedly. This is
    // bounded interleaving evidence, not deterministic replay of the old gap.
    for iteration in 0..64 {
        let f=Fixture::new();let t=f.accepted();
        let overlay=f.windows.live.lock().unwrap().values().next().unwrap().caller.clone();
        let (router,_)=escape_event_router(&f);
        let pending=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
        assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
        let (attempted,attempts)=mpsc::channel();
        let (armed_tx,armed_rx)=mpsc::channel();let (cancelled_tx,cancelled_rx)=mpsc::channel();
        let mut workers=None;
        crate::snipping::coordinator::snip_test_with_coordinator_lock(&f.runtime.coordinator,||{
            let runtime=f.runtime.clone();let arming=t.clone();let attempted_arm=attempted.clone();
            let arm=thread::spawn(move||{
                attempted_arm.send(()).unwrap();
                let _=armed_tx.send(runtime.coordinator.ready(overlay,arming,"m0".into()));
            });
            let cancel=thread::spawn(move||{
                attempted.send(()).unwrap();
                let _=cancelled_tx.send(pending.deliver());
            });
            workers=Some((arm,cancel));
            attempts.recv_timeout(Duration::from_secs(2)).unwrap();
            attempts.recv_timeout(Duration::from_secs(2)).unwrap();
        }); // Release the same state mutex to both actual production paths.
        let armed=armed_rx.recv_timeout(Duration::from_secs(2));
        let cancelled=cancelled_rx.recv_timeout(Duration::from_secs(2));
        let (arm,cancel)=workers.unwrap();
        // Join after bounded result observation; no artificial pin/arming state.
        if armed.is_ok() {arm.join().unwrap();}
        if cancelled.is_ok() {cancel.join().unwrap();}
        assert_eq!(cancelled.unwrap(),true,"Escape lost at cancel/arm boundary, iteration {iteration}");
        let armed=armed.unwrap();
        assert!(matches!(armed,Ok(true)|Err(SnipError::Stale)|Err(SnipError::Unauthorized)|Err(SnipError::Inactive)),
            "arming must finish or observe retirement, iteration {iteration}: {armed:?}");
        f.clean();assert_eq!(f.count("close"),1,"iteration {iteration}");
        assert_eq!(f.count(&format!("retire:{}:{}",t.generation,t.capture_id)),1);
        assert_eq!(f.count("restore"),0);
        assert_eq!(f.runtime.coordinator.observed_selection_generation(),0);
    }
}

#[test]
fn snip_runtime_escape_mailbox_arming_busy_cannot_lose_selection_cancellation() {
    let f=Fixture::new();let t=f.accepted();
    let overlay=f.windows.live.lock().unwrap().values().next().unwrap().caller.clone();
    let (entered_tx,entered)=mpsc::channel();let (release,release_rx)=mpsc::channel();
    *f.windows.armed_gate.lock().unwrap()=Some((entered_tx,release_rx));
    let runtime=f.runtime.clone();let arming=t.clone();
    let ready=thread::spawn(move||runtime.coordinator.ready(overlay,arming,"m0".into()));
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    let (router,_)=escape_event_router(&f);let (wake,receive)=mpsc::sync_channel::<()>(1);
    // The actual arming path already showed the overlay, but is still pinned
    // inside emit_armed. This is visible Selecting, NOT pinned completion.
    let shown=f.count("show");let before=f.runtime.coordinator.inspect();
    let pending=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    router.enqueue(pending,&wake);receive.recv_timeout(Duration::from_secs(1)).unwrap();
    let first=router.drain_pending();
    // Always release/join before assertions, including the expected RED path.
    release.send(()).unwrap();let ready_result=ready.join().unwrap();
    assert_eq!(shown,1);assert_eq!(before.phase,Some(Phase::Selecting));
    assert_eq!(before.token,Some(t.clone()));
    assert!(matches!(ready_result,Ok(true)|Err(SnipError::Stale)),"arming may finish or observe cancellation: {ready_result:?}");
    // Arming finalization itself must honor the retained Escape. Do not offer
    // another drain/wake/keypress that could conceal a missing retry scheduler.
    let after=f.runtime.coordinator.inspect();
    assert!(after.phase.is_none(),"Escape during visible arming must cancel without another keypress or worker retry; first_drain={first}, observed_phase={:?}",after.phase);
    f.clean();assert_eq!(f.count("close"),1);
    assert_eq!(f.count(&format!("retire:{}:{}",t.generation,t.capture_id)),1);
    assert_eq!(f.count("restore"),0);assert!(!router.drain_pending());
}

#[test]
fn snip_runtime_escape_mailbox_completion_pin_still_wins() {
    let publication=Arc::new(Mutex::new(Vec::new()));
    let f=Fixture::with_clipboard(Arc::new(EscapeClipboard(publication.clone())));
    let t=f.accepted();let overlay=escape_overlay(&f,&t);
    f.runtime.coordinator.begin_selection(overlay.clone(),t.clone(),"m0".into()).unwrap();
    let (entered_tx,entered)=mpsc::channel();let (release,release_rx)=mpsc::channel();
    *f.windows.create_gate.lock().unwrap()=Some((entered_tx,release_rx));
    let runtime=f.runtime.clone();let completing=t.clone();
    let complete=thread::spawn(move||runtime.coordinator.complete(overlay,completing,"m0".into(),
        crate::snipping::geometry::LogicalRect {x:0.0,y:0.0,width:2.0,height:2.0}));
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    let (router,_)=escape_event_router(&f);let (wake,receive)=mpsc::sync_channel::<()>(1);
    // Completion may either suppress observation or reject deferred delivery.
    if let Some(pending)=router.route(WM_KEYDOWN,ESCAPE_KEY) {router.enqueue(pending,&wake);}
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    let _=receive.try_recv();let cancelled=router.drain_pending();
    release.send(()).unwrap();let result=complete.join().unwrap();
    assert!(!cancelled);assert_eq!(result,Ok(true));assert!(!router.drain_pending());
    assert_eq!(f.runtime.coordinator.inspect().phase,Some(Phase::Preview));
    assert_eq!(publication.lock().unwrap().as_slice(),["publication"]);
    let preview=f.windows.live.lock().unwrap().values().next().unwrap().caller.clone();
    f.runtime.coordinator.dismiss(preview,t).unwrap();f.clean();assert_eq!(f.count("close"),2);
}

#[test]
fn snip_runtime_escape_mailbox_full_wake_queue_preserves_latest_generation() {
    let f=Fixture::new();let old=f.accepted();let overlay=escape_overlay(&f,&old);
    let (router,_)=escape_event_router(&f);
    let (wake,receive)=mpsc::sync_channel::<()>(1);
    let stale=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    let late_stale=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    router.enqueue(stale,&wake);
    assert!(matches!(wake.try_send(()),Err(mpsc::TrySendError::Full(()))),"stale wake must fill the actual capacity-one channel");
    f.runtime.coordinator.cancel(overlay,old).unwrap();f.clean();
    let current=f.accepted();escape_overlay(&f,&current);
    let pending=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    // No receiver runs until BOTH generations enqueue. Current work must live
    // in the production mailbox even though its wake try_send cannot succeed.
    router.enqueue(pending,&wake);
    assert!(matches!(wake.try_send(()),Err(mpsc::TrySendError::Full(()))));
    router.enqueue(late_stale,&wake); // Out-of-order old arrival cannot overwrite new.
    assert_eq!(f.runtime.coordinator.inspect().token,Some(current));
    receive.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(receive.try_recv(),Err(mpsc::TryRecvError::Empty),"one wake, not one payload per press");
    assert!(router.drain_pending(),"full wake queue must not discard latest-generation cancellation");
    f.clean();assert_eq!(f.count("close"),2);
    assert!(!router.drain_pending(),"coalesced work drains once");
}

#[test]
fn snip_runtime_escape_mailbox_delayed_old_generation_cannot_cancel_replacement() {
    let f=Fixture::new();let old=f.accepted();let overlay=escape_overlay(&f,&old);
    let (router,_)=escape_event_router(&f);let (wake,receive)=mpsc::sync_channel::<()>(1);
    let pending=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    f.runtime.coordinator.cancel(overlay,old).unwrap();f.clean();
    let current=f.accepted();let overlay=escape_overlay(&f,&current);
    // Enqueue itself is delayed across replacement, not merely final delivery.
    router.enqueue(pending,&wake);receive.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(!router.drain_pending());
    assert_eq!(f.runtime.coordinator.inspect().token,Some(current.clone()));
    assert_eq!(f.runtime.coordinator.inspect().phase,Some(Phase::Selecting));
    assert_eq!(f.count("close"),1);assert!(!router.drain_pending());
    f.runtime.coordinator.cancel(overlay,current).unwrap();f.clean();
}

#[test]
fn snip_runtime_escape_mailbox_shutdown_and_foreign_router_cannot_deliver() {
    let f=Fixture::new();let t=f.accepted();let overlay=escape_overlay(&f,&t);
    let (router,cleaned)=escape_event_router(&f);let (wake,receive)=mpsc::sync_channel::<()>(1);
    let (foreign,_)=escape_event_router(&f);
    let wrong_owner=foreign.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(foreign.route(WM_KEYUP,ESCAPE_KEY).is_none());
    router.enqueue(wrong_owner,&wake);
    assert_eq!(receive.try_recv(),Err(mpsc::TryRecvError::Empty));
    assert!(!router.drain_pending(),"same-generation foreign payload cannot acquire router authority");
    let queued=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    let late=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    router.enqueue(queued,&wake);router.shutdown();
    receive.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(!router.drain_pending(),"shutdown invalidates already-mailboxed cancellation");
    router.enqueue(late,&wake);
    assert_eq!(receive.try_recv(),Err(mpsc::TryRecvError::Empty),"stopped enqueue cannot issue another wake");
    assert!(!router.drain_pending());
    assert_eq!(f.runtime.coordinator.inspect().token,Some(t.clone()));assert_eq!(f.count("close"),0);
    drop(router);assert_eq!(cleaned.load(EscapeOrdering::SeqCst),1);
    f.runtime.coordinator.cancel(overlay,t).unwrap();f.clean();
}

#[test]
fn snip_runtime_escape_events_held_coordinator_mutex_does_not_block_or_drop_observation() {
    let f=Fixture::new();let t=f.accepted();escape_overlay(&f,&t);
    let (router,_)=escape_event_router(&f);
    let runtime=f.runtime.clone();
    let (locked_tx,locked)=mpsc::channel();let (release,release_rx)=mpsc::channel();
    let holder=thread::spawn(move||{
        crate::snipping::coordinator::snip_test_with_coordinator_lock(&runtime.coordinator,||{
            locked_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(3)).expect("contention fixture must release coordinator lock");
        });
    });
    locked.recv_timeout(Duration::from_secs(2)).unwrap();
    let (observed_tx,observed_rx)=mpsc::channel();let callback_owner=router.clone();
    let callback=thread::spawn(move||{
        let pending=callback_owner.route(WM_KEYDOWN,ESCAPE_KEY);
        assert!(callback_owner.route(WM_KEYUP,ESCAPE_KEY).is_none());
        observed_tx.send(pending).unwrap();
    });
    // Prove return WHILE the actual coordinator mutex remains held. Always
    // release/join before asserting so a blocking regression fails, not hangs.
    let observed=observed_rx.recv_timeout(Duration::from_secs(1));
    release.send(()).unwrap();holder.join().unwrap();callback.join().unwrap();
    let pending=observed.expect("native callback must not wait for coordinator mutex")
        .expect("lock contention must not discard the Selecting generation");
    assert_eq!(f.runtime.coordinator.inspect().token,Some(t));
    assert!(pending.deliver(),"deferred delivery cancels after coordinator lock release");
    f.clean();assert_eq!(f.count("close"),1);assert_eq!(f.count("restore"),0);
}

#[test]
fn snip_runtime_escape_events_short_press_release_delivers_without_maintenance_tick() {
    for dragging in [false,true] {
        let f=Fixture::new();let t=f.accepted();let overlay=escape_overlay(&f,&t);
        if dragging {f.runtime.coordinator.begin_selection(overlay,t.clone(),"m0".into()).unwrap();}
        *f.edges.foreground.lock().unwrap()=88; // No helper focus/DOM keyboard delivery.
        let (router,cleaned)=escape_event_router(&f);let time=f.edges.now();
        let pending=router.route(WM_KEYDOWN,ESCAPE_KEY).expect("native keydown must enqueue a token-bound cancellation");
        assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
        assert_eq!(f.edges.now(),time,"press and release fit wholly between maintenance ticks");
        assert_eq!(f.runtime.coordinator.inspect().phase,Some(Phase::Selecting));
        // Deliver AFTER release, with no clock advance, maintenance wake, or key
        // state sampling. A brief tap is durable work, not a currently-held bit.
        assert!(pending.deliver());f.clean();assert_eq!(f.count("close"),1);
        assert_eq!(f.count("restore"),0);assert_eq!(cleaned.load(EscapeOrdering::SeqCst),0);
        drop(router);assert_eq!(cleaned.load(EscapeOrdering::SeqCst),1);
    }
}

#[test]
fn snip_runtime_escape_events_delayed_delivery_cannot_cancel_replacement() {
    let f=Fixture::new();let old=f.accepted();let overlay=escape_overlay(&f,&old);
    let (router,_)=escape_event_router(&f);
    let pending=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    f.runtime.coordinator.cancel(overlay,old).unwrap();f.clean();
    let current=f.accepted();escape_overlay(&f,&current);
    assert!(!pending.deliver(),"queued old event must not resolve a fresh token at dispatch time");
    assert_eq!(f.runtime.coordinator.inspect().token,Some(current.clone()));
    assert_eq!(f.runtime.coordinator.inspect().phase,Some(Phase::Selecting));
    assert_eq!(f.count("close"),1);
    let fresh=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    assert!(fresh.deliver());f.clean();assert_eq!(f.count("close"),2);
}

#[test]
fn snip_runtime_escape_events_repeat_keyup_and_other_keys_are_ignored() {
    let f=Fixture::new();let t=f.accepted();escape_overlay(&f,&t);
    let (router,_)=escape_event_router(&f);
    for message in [WM_KEYDOWN,WM_KEYUP,WM_SYSKEYDOWN,WM_SYSKEYUP] {
        assert!(router.route(message,0x41).is_none(),"ordinary keys remain unowned");
    }
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    let first=router.route(WM_SYSKEYDOWN,ESCAPE_KEY).unwrap();
    for message in [WM_SYSKEYDOWN,WM_KEYDOWN] {
        assert!(router.route(message,ESCAPE_KEY).is_none(),"held-key repeats must not enqueue additional cancellation");
    }
    assert!(router.route(WM_SYSKEYUP,ESCAPE_KEY).is_none());
    assert!(first.deliver());f.clean();assert_eq!(f.count("close"),1);
    // A physical release permits a genuinely new keydown, not permanent latch.
    let newer=f.accepted();escape_overlay(&f,&newer);
    let next=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    assert!(next.deliver());f.clean();assert_eq!(f.count("close"),2);
}

#[test]
fn snip_runtime_escape_events_shutdown_drop_and_runtime_retirement_release_listener() {
    let f=Fixture::new();let t=f.accepted();let overlay=escape_overlay(&f,&t);
    let (router,cleaned)=escape_event_router(&f);
    let pending=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    router.shutdown();router.shutdown();
    assert_eq!(cleaned.load(EscapeOrdering::SeqCst),1);
    assert!(!pending.deliver(),"unsubscribe must invalidate already queued deliveries");
    assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    assert!(router.route(WM_KEYDOWN,ESCAPE_KEY).is_none());
    assert_eq!(f.runtime.coordinator.inspect().token,Some(t.clone()));
    assert_eq!(f.count("close"),0);drop(router);
    assert_eq!(cleaned.load(EscapeOrdering::SeqCst),1,"Drop cannot unsubscribe twice");
    f.runtime.coordinator.cancel(overlay,t).unwrap();f.clean();

    let f=Fixture::new();let t=f.accepted();escape_overlay(&f,&t);
    let (router,cleaned)=escape_event_router(&f);
    let pending=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();
    let weak=Arc::downgrade(&f.runtime);drop(f);
    assert!(weak.upgrade().is_none(),"listener and queued event must not retain runtime");
    assert!(!pending.deliver());assert!(router.route(WM_KEYUP,ESCAPE_KEY).is_none());
    assert!(router.route(WM_KEYDOWN,ESCAPE_KEY).is_none());
    drop(router);assert_eq!(cleaned.load(EscapeOrdering::SeqCst),1);

    // Drop itself invalidates queued work, not merely the explicit stop path.
    let f=Fixture::new();let t=f.accepted();let overlay=escape_overlay(&f,&t);
    let (router,cleaned)=escape_event_router(&f);
    let pending=router.route(WM_KEYDOWN,ESCAPE_KEY).unwrap();drop(router);
    assert_eq!(cleaned.load(EscapeOrdering::SeqCst),1);assert!(!pending.deliver());
    f.runtime.coordinator.cancel(overlay,t).unwrap();f.clean();
}
