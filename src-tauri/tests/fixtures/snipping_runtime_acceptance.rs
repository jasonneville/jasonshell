// Include at actual runtime module scope under cfg(test), or via the standalone
// actual-source integration harness. No duplicated runtime/foreground state machine.
pub(crate) mod runtime_acceptance {
    use crate::snipping::{coordinator::*, foreground::{Intent, verification::Tracker},
        geometry::{CroppedImage, PhysicalRect}, native::PreparationBarrier,
        native_registry::{NativeRegistry,FocusRegistration}, runtime::{RuntimeEdges, SnipRuntime, verification as v}};
    use std::{collections::HashMap, path::{Path,PathBuf}, sync::{Arc,Mutex,mpsc},
        thread, time::{Duration,Instant}};
    type Trace=Arc<Mutex<Vec<String>>>;
    fn record(trace:&Trace, event:&str) { trace.lock().unwrap().push(event.into()); }
    fn bar()->Caller { Caller { label:"top-bar".into(),window_id:1 } }
    struct Edges {
        trace:Trace, tracker:Mutex<Tracker>, time:Mutex<Instant>, visible:Mutex<bool>,
        foreground:Mutex<isize>, prepared:mpsc::Sender<Token>,
        focus_change_on_valid:Mutex<bool>,
        fail_emit:Mutex<bool>, flush_effect:Mutex<&'static str>,
        flush_gate:Mutex<Option<(mpsc::Sender<()>,mpsc::Receiver<()>)>>,
        registrations:Mutex<HashMap<isize,FocusRegistration>>,switch_on_registration:Mutex<bool>,
    }
    impl RuntimeEdges for Edges {
        fn intent(&self)->Result<Intent,SnipError> {
            record(&self.trace,"intent"); self.tracker.lock().unwrap().candidate().ok_or(SnipError::Inactive)
        }
        fn valid(&self,intent:Intent)->bool {
            let valid=self.tracker.lock().unwrap().valid(intent,Some((20,30)));
            if *self.focus_change_on_valid.lock().unwrap() {*self.foreground.lock().unwrap()=88;}
            valid
        }
        fn bar(&self)->Result<(),SnipError> { if *self.visible.lock().unwrap() {Ok(())} else {Err(SnipError::Unauthorized)} }
        fn emit(&self,event:&str,token:&Token)->Result<(),SnipError> {
            record(&self.trace,&format!("{event}:{}:{}",token.generation,token.capture_id));
            if event=="snip:prepare_bar" {
                self.prepared.send(token.clone()).unwrap();
                if *self.fail_emit.lock().unwrap() {return Err(SnipError::WindowFailed);}
            }
            Ok(())
        }
        fn flush(&self,_:Instant)->Result<(),SnipError> {
            record(&self.trace,"composition");
            let gate=self.flush_gate.lock().unwrap().take();
            if let Some((entered,release))=gate {
                entered.send(()).unwrap(); release.recv_timeout(Duration::from_secs(2)).map_err(|_|SnipError::CaptureFailed)?;
            }
            match *self.flush_effect.lock().unwrap() {
                "continuity"=>self.tracker.lock().unwrap().lose_continuity(),
                "foreground"=>self.tracker.lock().unwrap().foreground(11,Some((20,30)),false),
                "reused"=>{let mut tracker=self.tracker.lock().unwrap();tracker.created(10);tracker.foreground(10,Some((20,30)),false);},
                "bar"=>*self.visible.lock().unwrap()=false,
                "deadline"=>*self.time.lock().unwrap()+=Duration::from_secs(6),
                _=>{}
            }
            Ok(())
        }
        fn now(&self)->Instant { *self.time.lock().unwrap() }
        fn foreground(&self)->isize { *self.foreground.lock().unwrap() }
        fn restore(&self,_:Intent,owned:isize) { assert_eq!(owned,99); record(&self.trace,"restore"); }
        fn shutdown(&self) { record(&self.trace,"shutdown"); }
        fn feedback(&self,attempt:u64,code:Option<&str>) {record(&self.trace,&format!("feedback:{attempt}:{}",code.unwrap_or("started")));}
        fn focus_registration(&self,hwnd:isize)->Option<FocusRegistration> {
            let registration=self.registrations.lock().unwrap().get(&hwnd).cloned();
            if *self.switch_on_registration.lock().unwrap(){*self.foreground.lock().unwrap()=88;}
            registration
        }
    }
    struct Clock(Arc<Edges>);
    impl ClockPort for Clock { fn now(&self)->Instant { self.0.now() } }
    struct Capture { prepare:PreparationBarrier,release:Arc<dyn Fn()+Send+Sync>,edges:Arc<Edges>,fail:Arc<Mutex<bool>> }
    fn monitor()->MonitorSnapshot { MonitorSnapshot {id:"m0".into(),rect:PhysicalRect {left:-4,top:0,right:0,bottom:4},scale:1.0,identity:77} }
    impl CapturePort for Capture {
        fn snapshot(&self)->Result<Vec<MonitorSnapshot>,SnipError> {Ok(vec![monitor()])}
        fn freeze(&self,monitors:&[MonitorSnapshot])->Result<Vec<Frame>,SnipError> {
            struct Release(Arc<dyn Fn()+Send+Sync>);
            impl Drop for Release { fn drop(&mut self) { (self.0)(); } }
            let _release=Release(self.release.clone());
            (self.prepare)(self.edges.now()+Duration::from_secs(5))?;
            record(&self.edges.trace,"capture");
            if *self.fail.lock().unwrap() {return Err(SnipError::CaptureFailed);}
            Ok(monitors.iter().map(|m|Frame {monitor:m.clone(),rgba:vec![255;64]}).collect())
        }
    }
    struct Windows {live:Mutex<HashMap<u64,WindowRef>>,next:Mutex<u64>,trace:Trace,create_gate:Mutex<Option<(mpsc::Sender<()>,mpsc::Receiver<()>)>>,armed_gate:Mutex<Option<(mpsc::Sender<()>,mpsc::Receiver<()>)>>}
    impl Windows {
        fn create(&self,t:&Token,m:&MonitorSnapshot,preview:bool)->Result<WindowRef,SnipError> {
            if let Some((entered,release))=self.create_gate.lock().unwrap().take() {
                entered.send(()).unwrap();release.recv_timeout(Duration::from_secs(2)).expect("late-create fixture must release within bound");
            }
            let mut next=self.next.lock().unwrap(); *next+=1;
            let w=WindowRef {caller:Caller {label:if preview {format!("snip-preview-{}",t.generation)} else {format!("snip-overlay-{}-{}",t.generation,m.id)},window_id:*next},monitor_id:Some(m.id.clone())};
            self.live.lock().unwrap().insert(*next,w.clone()); record(&self.trace,"create"); Ok(w)
        }
    }
    impl WindowPort for Windows {
        fn retire_generation(&self,t:&Token){record(&self.trace,&format!("retire:{}:{}",t.generation,t.capture_id));}
        fn create_overlay(&self,t:&Token,m:&MonitorSnapshot)->Result<WindowRef,SnipError>{self.create(t,m,false)}
        fn create_preview(&self,t:&Token,m:&MonitorSnapshot,_:u32,_:u32)->Result<WindowRef,SnipError>{self.create(t,m,true)}
        fn emit_context(&self,_:&WindowRef,_:&Context)->Result<(),SnipError>{record(&self.trace,"context");Ok(())}
        fn emit_armed(&self,_:&WindowRef,_:&Token)->Result<(),SnipError>{
            if let Some((entered,release))=self.armed_gate.lock().unwrap().take() {
                entered.send(()).unwrap();
                release.recv_timeout(Duration::from_secs(3)).expect("arming fixture must release emit_armed");
            }
            record(&self.trace,"armed");Ok(())
        }
        fn show(&self,_:&WindowRef)->Result<(),SnipError>{record(&self.trace,"show");Ok(())}
        fn hide(&self,_:&WindowRef)->Result<(),SnipError>{record(&self.trace,"hide");Ok(())}
        fn close(&self,w:WindowRef){assert!(self.live.lock().unwrap().remove(&w.caller.window_id).is_some());record(&self.trace,"close");}
        fn still_same(&self,w:&WindowRef)->bool {
            (w.caller==bar() && w.monitor_id.is_none()) || self.live.lock().unwrap().get(&w.caller.window_id)
                .is_some_and(|live|live.caller==w.caller && w.monitor_id.as_ref().is_none_or(|id|live.monitor_id.as_ref()==Some(id)))
        }
    }
    struct NoPublication;
    impl ClipboardPort for NoPublication {fn publish(&self,_:&CroppedImage,_:usize)->ClipboardOutcome {panic!("runtime barrier cases must never publish clipboard")}}
    impl PickerPort for NoPublication {fn pick_png(&self)->Result<Option<PathBuf>,SaveCode>{panic!("runtime barrier cases must never open picker")}}
    impl PublisherPort for NoPublication {fn publish_png(&self,_:&Path,_:&[u8])->Result<(),SaveCode>{panic!("runtime barrier cases must never publish file")}}
    struct Fixture {runtime:Arc<SnipRuntime>,edges:Arc<Edges>,windows:Arc<Windows>,prepared:mpsc::Receiver<Token>,capture_fail:Arc<Mutex<bool>>}
    struct Work {result:mpsc::Receiver<Result<Token,SnipError>>,join:thread::JoinHandle<()>}
    impl Work {fn finish(self)->Result<Token,SnipError> {let result=self.result.recv_timeout(Duration::from_secs(2)).expect("runtime worker did not finish within bound"); self.join.join().unwrap(); result}}
    impl Fixture {
        fn new()->Self {
            Self::with_clipboard(Arc::new(NoPublication))
        }
        fn with_clipboard(clipboard:Arc<dyn ClipboardPort>)->Self {
            let trace=Arc::new(Mutex::new(Vec::new())); let (tx,prepared)=mpsc::channel();
            let mut tracker=Tracker::new(); tracker.foreground(10,Some((20,30)),false);
            let edges=Arc::new(Edges {trace:trace.clone(),tracker:Mutex::new(tracker),time:Mutex::new(Instant::now()),visible:Mutex::new(true),foreground:Mutex::new(99),prepared:tx,focus_change_on_valid:Mutex::new(false),fail_emit:Mutex::new(false),flush_effect:Mutex::new("none"),flush_gate:Mutex::new(None),registrations:Mutex::new(HashMap::new()),switch_on_registration:Mutex::new(false)});
            let windows=Arc::new(Windows {live:Mutex::new(HashMap::new()),next:Mutex::new(100),trace,create_gate:Mutex::new(None),armed_gate:Mutex::new(None)});
            let capture_fail=Arc::new(Mutex::new(false)); let (e,f)=(edges.clone(),capture_fail.clone());
            let runtime=v::construct(bar(),Arc::new(NativeRegistry::default()),edges.clone(),v::Ports {
                capture:Box::new(move |prepare,release|Arc::new(Capture {prepare,release,edges:e,fail:f})),
                windows:windows.clone(),clipboard,picker:Arc::new(NoPublication),publisher:Arc::new(NoPublication),clock:Arc::new(Clock(edges.clone()))});
            Self {runtime,edges,windows,prepared,capture_fail}
        }
        fn launch(&self)->Work {
            let runtime=self.runtime.clone(); let ticket=runtime.reserve_native_start().unwrap();
            record(&self.edges.trace,"worker-launch"); let (tx,result)=mpsc::channel();
            let join=thread::spawn(move||{let _=tx.send(runtime.run_native_start(ticket));}); Work {result,join}
        }
        fn pending(&self)->Token {let token=self.prepared.recv_timeout(Duration::from_secs(2)).unwrap();assert_eq!(v::pending(&self.runtime).unwrap().0,token);token}
        fn expire(&self) {*self.edges.time.lock().unwrap()+=Duration::from_secs(6);v::wake(&self.runtime);}
        fn count(&self,event:&str)->usize {self.edges.trace.lock().unwrap().iter().filter(|s|*s==event).count()}
        fn released(&self,t:&Token) {let event=format!("snip:release_bar:{}:{}",t.generation,t.capture_id);assert_eq!(self.count(&event),1);assert!(v::pending(&self.runtime).is_none());}
        fn clean(&self) {let s=self.runtime.coordinator.inspect();assert!(s.phase.is_none());assert_eq!(s.frame_bytes,0);assert!(self.windows.live.lock().unwrap().is_empty());}
        fn accepted(&self)->Token {let work=self.launch();let t=self.pending();v::ack(&self.runtime,bar(),t.clone()).unwrap();assert_eq!(work.finish(),Ok(t.clone()));t}
    }
    impl Drop for Fixture {fn drop(&mut self) {self.runtime.shutdown();}}

    // Production owner's event router, not a second test keyboard state machine.
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snipping_escape_events.rs"));
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snipping_escape_native.rs"));

    // Acceptance seam requested from production owner: the SAME native worker
    // must call native_escape(captured_token, escape_high_bit_is_down). No DOM
    // keydown, active WebView, renderer-supplied caller, or fake runtime here.
    // False means no cancellation (including stale/inactive/nonselecting input).
    fn escape_overlay(f:&Fixture,t:&Token)->Caller {
        let caller=f.windows.live.lock().unwrap().values().find(|w|w.caller.label.starts_with("snip-overlay-")).unwrap().caller.clone();
        assert!(f.runtime.coordinator.ready(caller.clone(),t.clone(),"m0".into()).unwrap());
        caller
    }
    // Test-only bridge for cross-subsystem acceptance. Both phases use the
    // production Runtime/Coordinator/EventRouter; no OS listener is installed.
    pub(crate) fn exercise_escape_hotkey_coexistence(mut check: impl FnMut(&Arc<crate::snipping::escape::EventRouter>, &Arc<SnipRuntime>, bool)) {
        let f=Fixture::new();
        let (router,_)=escape_event_router(&f);
        check(&router,&f.runtime,false);
        let token=f.accepted();escape_overlay(&f,&token);
        check(&router,&f.runtime,true);
        f.clean();router.shutdown();
    }
    #[test]
    fn snip_runtime_escape_before_and_during_drag_ignores_webview_focus() {
        for native in [true,false] { for dragging in [false,true] {
            let f=Fixture::new();
            let t=if native {f.accepted()} else {
                let runtime=f.runtime.clone();let work=thread::spawn(move||v::start(&runtime,bar()));
                let token=f.pending();v::ack(&f.runtime,bar(),token.clone()).unwrap();
                assert_eq!(work.join().unwrap(),Ok(token.clone()));token
            };
            let overlay=escape_overlay(&f,&t);
            if dragging {assert!(f.runtime.coordinator.begin_selection(overlay,t.clone(),"m0".into()).unwrap());}
            // External foreground, no focus registration or DOM event delivery.
            *f.edges.foreground.lock().unwrap()=88;
            assert!(!f.runtime.native_escape(&t,false),"key-up/unpressed sample must not cancel");
            assert_eq!(f.runtime.coordinator.inspect().phase,Some(Phase::Selecting));
            assert!(f.runtime.native_escape(&t,true),"Escape must cancel before pointer-down AND during selection for both start routes");
            f.clean();assert_eq!(f.count("close"),1);assert_eq!(f.count("restore"),0);
            assert_eq!(f.count(&format!("retire:{}:{}",t.generation,t.capture_id)),1);
            assert!(!f.runtime.native_escape(&t,true),"duplicate Escape is a no-op");
            assert_eq!(f.count("close"),1);f.released(&t);
        }}
    }
    #[test]
    fn snip_runtime_escape_stale_generation_and_capture_id_cannot_cancel_retry() {
        let f=Fixture::new();let old=f.accepted();escape_overlay(&f,&old);
        assert!(f.runtime.native_escape(&old,true));f.clean();
        let current=f.accepted();escape_overlay(&f,&current);
        for stale in [old,Token {generation:current.generation.clone(),capture_id:"00000000000000000000000000000000".into()}] {
            assert!(!f.runtime.native_escape(&stale,true));
            assert_eq!(f.runtime.coordinator.inspect().token,Some(current.clone()));
            assert_eq!(f.runtime.coordinator.inspect().phase,Some(Phase::Selecting));
            assert_eq!(f.count("close"),1);
        }
        assert!(f.runtime.native_escape(&current,true));f.clean();assert_eq!(f.count("close"),2);
    }
    #[test]
    fn snip_runtime_escape_preparing_and_shutdown_are_unowned() {
        let f=Fixture::new();let work=f.launch();let t=f.pending();
        assert_eq!(f.runtime.coordinator.inspect().phase,Some(Phase::Preparing));
        assert!(!f.runtime.native_escape(&t,true),"native selection key ownership must not start during preparation");
        assert_eq!(f.count("capture"),0);f.expire();
        assert_eq!(work.finish(),Err(SnipError::ReadinessTimeout));f.released(&t);f.clean();
        f.runtime.shutdown();assert!(!f.runtime.native_escape(&t,true));
        assert_eq!(f.count("create"),0);assert_eq!(f.count("restore"),0);
        // A stopped runtime must reject even a still-selecting synthetic port;
        // an already-absent generation alone cannot establish the stop guard.
        let f=Fixture::new();let t=f.accepted();let overlay=escape_overlay(&f,&t);
        f.runtime.shutdown();assert!(!f.runtime.native_escape(&t,true));
        assert_eq!(f.runtime.coordinator.inspect().phase,Some(Phase::Selecting));
        assert_eq!(f.count("close"),0);
        f.runtime.coordinator.cancel(overlay,t).unwrap();f.clean();
    }
    struct EscapeClipboard(Trace);
    impl ClipboardPort for EscapeClipboard {
        fn publish(&self,_:&CroppedImage,_:usize)->ClipboardOutcome {
            record(&self.0,"publication");ClipboardOutcome::Rejected("synthetic-only".into())
        }
    }
    #[test]
    fn snip_runtime_escape_does_not_dismiss_preview_or_republish() {
        let publication=Arc::new(Mutex::new(Vec::new()));
        let f=Fixture::with_clipboard(Arc::new(EscapeClipboard(publication.clone())));
        let t=f.accepted();let overlay=escape_overlay(&f,&t);
        f.runtime.coordinator.begin_selection(overlay.clone(),t.clone(),"m0".into()).unwrap();
        f.runtime.coordinator.complete(overlay,t.clone(),"m0".into(),crate::snipping::geometry::LogicalRect {x:0.0,y:0.0,width:2.0,height:2.0}).unwrap();
        let before=f.runtime.coordinator.inspect();assert_eq!(before.phase,Some(Phase::Preview));
        for down in [false,true,true] {assert!(!f.runtime.native_escape(&t,down));}
        let after=f.runtime.coordinator.inspect();
        assert_eq!(after.phase,before.phase);assert_eq!(after.token,before.token);
        assert_eq!(after.window_ids,before.window_ids);assert_eq!(after.image_bytes,before.image_bytes);
        assert_eq!(publication.lock().unwrap().as_slice(),["publication"]);
        // Cancelling a replacement restores the retained original preview,
        // instead of dismissing it or relabelling its pixels/publication.
        let replacement=f.accepted();escape_overlay(&f,&replacement);
        assert!(f.runtime.native_escape(&replacement,true));
        let restored=f.runtime.coordinator.inspect();
        assert_eq!(restored.phase,Some(Phase::Preview));assert_eq!(restored.token,Some(t.clone()));
        assert_eq!(restored.window_ids,before.window_ids);assert_eq!(restored.image_bytes,before.image_bytes);
        assert!(!f.runtime.native_escape(&replacement,true));assert!(!f.runtime.native_escape(&t,true));
        assert_eq!(publication.lock().unwrap().as_slice(),["publication"]);
        let preview=f.windows.live.lock().unwrap().values().next().unwrap().caller.clone();
        f.runtime.coordinator.dismiss(preview,t).unwrap();f.clean();
        assert_eq!(f.count("close"),3,"both overlays and explicit-dismiss preview each close once");
    }
    #[test]
    fn snip_runtime_escape_cannot_interrupt_pinned_completion() {
        let publication=Arc::new(Mutex::new(Vec::new()));
        let f=Fixture::with_clipboard(Arc::new(EscapeClipboard(publication.clone())));
        let t=f.accepted();let overlay=escape_overlay(&f,&t);
        f.runtime.coordinator.begin_selection(overlay.clone(),t.clone(),"m0".into()).unwrap();
        let (entered_tx,entered)=mpsc::channel();let (release,release_rx)=mpsc::channel();
        *f.windows.create_gate.lock().unwrap()=Some((entered_tx,release_rx));
        let runtime=f.runtime.clone();let completing=t.clone();
        let worker=thread::spawn(move||runtime.coordinator.complete(overlay,completing,"m0".into(),
            crate::snipping::geometry::LogicalRect {x:0.0,y:0.0,width:2.0,height:2.0}));
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        let cancelled=f.runtime.native_escape(&t,true);
        // Release before asserting, so a failed assertion cannot strand worker.
        release.send(()).unwrap();let result=worker.join().unwrap();
        assert!(!cancelled,"completion already owns the mutation pin");assert_eq!(result,Ok(true));
        assert_eq!(f.runtime.coordinator.inspect().phase,Some(Phase::Preview));
        assert_eq!(publication.lock().unwrap().as_slice(),["publication"]);
        assert!(!f.runtime.native_escape(&t,true));
        let preview=f.windows.live.lock().unwrap().values().next().unwrap().caller.clone();
        f.runtime.coordinator.dismiss(preview,t).unwrap();f.clean();assert_eq!(f.count("close"),2);
    }

    #[test]
    fn snip_runtime_forged_ack_has_zero_composition_and_capture() {
        let f=Fixture::new();
        for caller in [Caller {label:"settings-panel".into(),window_id:1},Caller {label:"top-bar".into(),window_id:2}] {
            assert_eq!(v::start(&f.runtime,caller),Err(SnipError::Unauthorized));
        }
        assert_eq!(f.count("intent"),0);
        let work=f.launch();let t=f.pending();
        for caller in [Caller {label:"settings-panel".into(),window_id:1},Caller {label:"top-bar".into(),window_id:2}] {
            assert_eq!(v::ack(&f.runtime,caller,t.clone()),Err(SnipError::Unauthorized));
        }
        for changed in [Token {generation:"999".into(),capture_id:t.capture_id.clone()},Token {generation:t.generation.clone(),capture_id:"00000000000000000000000000000000".into()}] {
            assert_eq!(v::ack(&f.runtime,bar(),changed),Err(SnipError::Stale));
        }
        assert!(!v::pending(&f.runtime).unwrap().3);assert_eq!(f.count("composition"),0);assert_eq!(f.count("capture"),0);
        f.expire();assert_eq!(work.finish(),Err(SnipError::ReadinessTimeout));f.released(&t);f.clean();
    }
    #[test]
    fn snip_runtime_exact_ack_once_orders_composition_capture_release() {
        let f=Fixture::new();let (tx,entered)=mpsc::channel();let (release,rx)=mpsc::channel();*f.edges.flush_gate.lock().unwrap()=Some((tx,rx));
        let work=f.launch();let t=f.pending();let ack=v::ack(&f.runtime,bar(),t.clone()).unwrap();assert_eq!(ack["accepted"],true);
        assert_eq!(ack["generation"],t.generation);assert_eq!(ack["captureId"],t.capture_id);
        entered.recv_timeout(Duration::from_secs(2)).unwrap();assert_eq!(v::ack(&f.runtime,bar(),t.clone()),Err(SnipError::Stale));
        assert_eq!(f.count("capture"),0);release.send(()).unwrap();assert_eq!(work.finish(),Ok(t.clone()));f.released(&t);
        assert_eq!(f.count("composition"),1);assert_eq!(f.count("capture"),1);
        let trace=f.edges.trace.lock().unwrap();let pos=|s:&str|trace.iter().position(|e|e==s).unwrap();
        assert!(pos("intent")<pos("worker-launch"));assert!(pos("composition")<pos("capture"));assert!(pos("capture")<pos("create"));
        drop(trace);assert_eq!(v::ack(&f.runtime,bar(),t.clone()),Err(SnipError::Stale));
        let overlay=f.windows.live.lock().unwrap().values().next().unwrap().caller.clone();
        f.runtime.coordinator.cancel(overlay,t.clone()).unwrap();f.clean();
        let work=f.launch();let newer=f.pending();assert_ne!(newer,t);
        assert_eq!(v::ack(&f.runtime,bar(),t),Err(SnipError::Stale));
        assert!(!v::pending(&f.runtime).unwrap().3);f.expire();
        assert_eq!(work.finish(),Err(SnipError::ReadinessTimeout));f.released(&newer);f.clean();
        assert_eq!(f.count("composition"),1);assert_eq!(f.count("capture"),1);
    }
    #[test]
    fn snip_runtime_deadline_rejects_late_ack_and_releases_matching_token() {
        let f=Fixture::new();let work=f.launch();let t=f.pending();let deadline=v::pending(&f.runtime).unwrap().2;
        assert_eq!(deadline.duration_since(f.edges.now()),Duration::from_secs(5));
        *f.edges.time.lock().unwrap()=deadline;assert_eq!(v::ack(&f.runtime,bar(),t.clone()),Err(SnipError::Stale));v::wake(&f.runtime);
        assert_eq!(work.finish(),Err(SnipError::ReadinessTimeout));f.released(&t);f.clean();assert_eq!(f.count("composition"),0);assert_eq!(f.count("capture"),0);
    }
    #[test]
    fn snip_runtime_native_ticket_snapshots_before_worker_and_rejects_late_foreground() {
        let f=Fixture::new();let ticket=f.runtime.reserve_native_start().unwrap();assert_eq!(f.count("intent"),1);
        assert!(matches!(f.runtime.reserve_native_start(),Err(SnipError::Busy)));
        f.edges.tracker.lock().unwrap().foreground(11,Some((20,30)),false);
        assert_eq!(f.runtime.run_native_start(ticket),Err(SnipError::Inactive));assert_eq!(f.count("intent"),1);
        assert_eq!(f.count("composition"),0);assert_eq!(f.count("capture"),0);f.clean();
        let ticket=f.runtime.reserve_native_start().unwrap();drop(ticket);assert_eq!(f.count("intent"),2);
    }
    #[test]
    fn snip_runtime_ticket_drop_cross_runtime_and_expiry_release_reservation() {
        let f=Fixture::new();let ticket=f.runtime.reserve_native_start().unwrap();drop(ticket);
        let other=Fixture::new();let ticket=f.runtime.reserve_native_start().unwrap();assert_eq!(other.runtime.run_native_start(ticket),Err(SnipError::Unauthorized));
        let ticket=f.runtime.reserve_native_start().unwrap();f.expire();assert_eq!(f.runtime.run_native_start(ticket),Err(SnipError::ReadinessTimeout));
        let ticket=f.runtime.reserve_native_start().unwrap();drop(ticket);f.clean();assert_eq!(f.count("capture"),0);
        *f.edges.visible.lock().unwrap()=false;assert!(matches!(f.runtime.reserve_native_start(),Err(SnipError::Unauthorized)));
        *f.edges.visible.lock().unwrap()=true;drop(f.runtime.reserve_native_start().unwrap());
        let ticket=f.runtime.reserve_native_start().unwrap();f.runtime.shutdown();
        assert_eq!(f.runtime.run_native_start(ticket),Err(SnipError::Inactive));f.clean();
    }
    #[test]
    fn snip_runtime_composition_continuity_loss_prevents_freeze() {
        for effect in ["continuity","foreground","reused"] {
            let f=Fixture::new();*f.edges.flush_effect.lock().unwrap()=effect;
            let work=f.launch();let t=f.pending();v::ack(&f.runtime,bar(),t.clone()).unwrap();
            assert_eq!(work.finish(),Err(SnipError::Inactive));assert_eq!(f.count("composition"),1);assert_eq!(f.count("capture"),0);f.released(&t);f.clean();
        }
    }
    #[test]
    fn snip_runtime_postcomposition_bar_loss_and_deadline_prevent_freeze() {
        for (effect,error) in [("bar",SnipError::Unauthorized),("deadline",SnipError::ReadinessTimeout)] {
            let f=Fixture::new();*f.edges.flush_effect.lock().unwrap()=effect;let work=f.launch();let t=f.pending();v::ack(&f.runtime,bar(),t.clone()).unwrap();
            assert_eq!(work.finish(),Err(error));assert_eq!(f.count("composition"),1);assert_eq!(f.count("capture"),0);f.released(&t);f.clean();
        }
    }
    #[test]
    fn snip_runtime_prepare_emit_failure_and_shutdown_unwind() {
        for fail_emit in [true,false] {
            let f=Fixture::new();*f.edges.fail_emit.lock().unwrap()=fail_emit;let work=f.launch();
            let t=f.prepared.recv_timeout(Duration::from_secs(2)).unwrap();
            if !fail_emit {f.runtime.shutdown();}
            let result=work.finish();assert!(matches!(result,Err(SnipError::WindowFailed)|Err(SnipError::Inactive)|Err(SnipError::Stale)));
            f.released(&t);f.clean();assert_eq!(f.count("composition"),0);assert_eq!(f.count("capture"),0);
            if !fail_emit {assert!(matches!(f.runtime.reserve_native_start(),Err(SnipError::Unauthorized)));}
        }
    }
    #[test]
    fn snip_runtime_restore_requires_generation_capture_focus_and_continuity() {
        let f=Fixture::new();let t=f.accepted();
        for wrong in [Token {generation:"999".into(),capture_id:t.capture_id.clone()},Token {generation:t.generation.clone(),capture_id:"00000000000000000000000000000000".into()}] {v::restore(&f.runtime,wrong,99);}
        *f.edges.foreground.lock().unwrap()=88;v::restore(&f.runtime,t.clone(),99);assert_eq!(f.count("restore"),0);
        *f.edges.foreground.lock().unwrap()=99;v::restore(&f.runtime,t.clone(),99);assert_eq!(f.count("restore"),1);
        *f.edges.focus_change_on_valid.lock().unwrap()=true;v::restore(&f.runtime,t.clone(),99);assert_eq!(f.count("restore"),1,"user focus change during validation must win");
        *f.edges.focus_change_on_valid.lock().unwrap()=false;*f.edges.foreground.lock().unwrap()=99;
        f.edges.tracker.lock().unwrap().lose_continuity();v::restore(&f.runtime,t,99);assert_eq!(f.count("restore"),1);
    }
    #[test]
    fn snip_runtime_foreground_tracker_reuse_and_identity_fail_closed() {
        for event in ["destroy","create","unknown","different","same-hwnd-new-epoch","continuity"] {
            let mut tracker=Tracker::new();tracker.foreground(10,Some((20,30)),false);let old=tracker.candidate().unwrap();
            tracker.destroyed(42);tracker.created(42);assert!(tracker.valid(old,Some((20,30))));
            tracker.foreground(99,None,true);assert!(tracker.valid(old,Some((20,30))),"proven own activation preserves external intent");
            assert!(!tracker.valid(old,Some((21,30))));assert!(!tracker.valid(old,Some((20,31))));assert!(!tracker.valid(old,None));
            match event {"destroy"=>tracker.destroyed(10),"create"=>tracker.created(10),"unknown"=>tracker.foreground(0,None,false),"different"=>tracker.foreground(11,Some((20,30)),false),"same-hwnd-new-epoch"=>{tracker.foreground(11,Some((20,30)),false);tracker.foreground(10,Some((20,30)),false);},_=>tracker.lose_continuity()}
            assert!(!tracker.valid(old,Some((20,30))),"{event}: same HWND/PID cannot inherit native instance continuity");
        }
    }
    #[test]
    fn snip_runtime_own_origin_without_external_candidate_can_reserve() {
        let f=Fixture::new();
        // Native registry has resolved a concrete normal-surface instance. No
        // external application was observed first; legacy own=true is NOT authority.
        let mut tracker=Tracker::new();tracker.registered_foreground(200,20,30,400);
        *f.edges.tracker.lock().unwrap()=tracker;
        let ticket=f.runtime.reserve_native_start();
        assert!(ticket.is_ok(),"registered same-process origin must not require a previously observed external window; got {:?}",ticket.err());
        assert_eq!(f.count("capture"),0);
        f.clean();
    }
    #[test]
    fn snip_runtime_own_origin_must_not_inherit_previous_external_target() {
        let mut tracker=Tracker::new();tracker.foreground(10,Some((20,30)),false);
        let external=tracker.candidate().unwrap();
        // A normal registered panel becomes foreground BEFORE hotkey reservation,
        // not a snipping helper activated during an already-reserved operation.
        tracker.registered_foreground(200,20,30,400);
        assert_ne!(tracker.candidate(),Some(external),"Process Manager/Stack Browser origin must replace, not inherit, prior external restore authority");
        assert!(tracker.candidate().is_some(),"native registered identity must remain available for exact origin restoration");
    }
    #[test]
    fn snip_runtime_registered_instance_replacement_and_user_switch_invalidate_origin() {
        for event in ["instance","hwnd","pid","thread","destroy","create","unknown-own","switch-back","continuity"] {
            let mut tracker=Tracker::new();tracker.registered_foreground(200,20,30,400);
            let old=tracker.candidate().unwrap();
            tracker.registered_foreground(200,20,30,400);
            assert_eq!(tracker.candidate(),Some(old),"observer query refresh must not retire unchanged identity");
            tracker.foreground(99,None,true);assert!(tracker.valid(old,Some((20,30))),"exact helper transition preserves origin");
            match event {
                "instance"=>tracker.registered_foreground(200,20,30,401),
                "hwnd"=>tracker.registered_foreground(201,20,30,400),
                "pid"=>tracker.registered_foreground(200,21,30,400),
                "thread"=>tracker.registered_foreground(200,20,31,400),
                "destroy"=>tracker.destroyed(200),"create"=>tracker.created(200),
                "unknown-own"=>tracker.foreground(201,None,false),
                "switch-back"=>{tracker.registered_foreground(201,20,30,401);tracker.registered_foreground(200,20,30,400);},
                _=>tracker.lose_continuity(),
            }
            assert!(!tracker.valid(old,Some((20,30))),"{event}: old restore authority must retire");
        }
    }
    #[test]
    fn snip_runtime_registered_origin_restores_only_without_user_switch_or_replacement() {
        for event in ["none","user-switch","instance"] {
            let f=Fixture::new();f.edges.tracker.lock().unwrap().registered_foreground(200,20,30,400);
            let token=f.accepted();
            match event {
                "user-switch"=>f.edges.tracker.lock().unwrap().registered_foreground(201,20,30,401),
                "instance"=>f.edges.tracker.lock().unwrap().registered_foreground(200,20,30,401),
                _=>{},
            }
            v::restore(&f.runtime,token,99);
            assert_eq!(f.count("restore"),usize::from(event=="none"),"{event}");
        }
    }
    #[test]
    fn snip_runtime_hotkey_feedback_reports_bounded_failure_and_retry_without_repeat_noise() {
        let f=Fixture::new();let ticket=f.runtime.reserve_native_start().unwrap();
        assert!(matches!(f.runtime.reserve_native_start(),Err(SnipError::Busy)));
        assert_eq!(f.count("feedback:1:started"),1);assert_eq!(f.count("feedback:2:started"),0);
        f.expire();assert_eq!(f.runtime.run_native_start(ticket),Err(SnipError::ReadinessTimeout));
        assert_eq!(f.count("feedback:1:readiness-timeout"),1);
        f.runtime.hotkey_failure(1,SnipError::ReadinessTimeout);
        f.runtime.hotkey_failure(1,SnipError::CaptureFailed);
        assert_eq!(f.count("feedback:1:readiness-timeout"),1);
        assert_eq!(f.count("feedback:1:capture-failed"),0,"same-attempt failures announce only once");
        {let trace=f.edges.trace.lock().unwrap();
         assert!(trace.iter().position(|event|event=="intent").unwrap()<trace.iter().position(|event|event=="feedback:1:started").unwrap(),"origin acquisition must precede feedback");}
        let ticket=f.runtime.reserve_native_start().unwrap();assert_eq!(f.count("feedback:2:started"),1);drop(ticket);
        // Delayed native failure publication from a retired attempt must not
        // overwrite retry state, even when the old worker reports a real error.
        f.runtime.hotkey_failure(1,SnipError::CaptureFailed);
        assert_eq!(f.count("feedback:1:capture-failed"),0);
        f.edges.tracker.lock().unwrap().foreground(0,None,false);
        assert!(matches!(f.runtime.reserve_native_start(),Err(SnipError::Inactive)));
        assert_eq!(f.count("feedback:3:started"),1);assert_eq!(f.count("feedback:3:inactive"),1);
        assert_eq!(f.count("capture"),0);f.clean();
    }
    #[test]
    fn snip_runtime_native_prepare_topbar_switch_revokes_external_capture_and_restore() {
        let f=Fixture::new();
        {let mut tracker=f.edges.tracker.lock().unwrap();tracker.set_bar(1);tracker.admit(true,10);}
        let work=f.launch();let token=f.pending();
        f.edges.tracker.lock().unwrap().surface_foreground(1,20,30,1);
        v::ack(&f.runtime,bar(),token.clone()).unwrap();
        assert!(matches!(work.finish(),Err(SnipError::Stale)|Err(SnipError::Inactive)));
        v::restore(&f.runtime,token.clone(),99);
        assert_eq!(f.count("capture"),0);assert_eq!(f.count("restore"),0);f.released(&token);f.clean();
    }
    #[test]
    fn snip_runtime_renderer_topbar_admission_preserves_preclick_external_origin() {
        let f=Fixture::new();
        {let mut tracker=f.edges.tracker.lock().unwrap();tracker.set_bar(1);tracker.surface_foreground(1,20,30,1);tracker.admit(false,1);}
        // Renderer start shares the production preparation path without invoking
        // native admission. The callback seam above models pre-click retention.
        let runtime=f.runtime.clone();let (tx,result)=mpsc::channel();
        let join=thread::spawn(move||{tx.send(v::start(&runtime,bar())).unwrap();});let work=Work{result,join};
        let token=f.pending();v::ack(&f.runtime,bar(),token.clone()).unwrap();assert_eq!(work.finish(),Ok(token.clone()));
        v::restore(&f.runtime,token,99);assert_eq!(f.count("capture"),1);assert_eq!(f.count("restore"),1);
    }
    #[test]
    fn snip_runtime_registered_panel_hold_tracks_helper_generation_phase_and_user_switch() {
        for source in [200,201] { // Process Manager and Stack Browser resolved native identities.
            let f=Fixture::new();
            let normal=FocusRegistration{hwnd:source,pid:20,thread:30,instance:source as u64,token:None};
            f.edges.registrations.lock().unwrap().insert(source,normal.clone());
            let work=f.launch();let token=f.pending();
            let helper=FocusRegistration{hwnd:99,pid:20,thread:31,instance:999,token:Some(token.clone())};
            f.edges.registrations.lock().unwrap().insert(99,helper.clone());
            assert!(f.runtime.holds_focus_loss(source),"matching helper preserves source during Preparing");
            for phase in [None,Some(Phase::Preview)] {
                assert!(!super::focus_loss_held(source,99,Some(&normal),Some(&helper),Some(&token),phase),"completed/absent generation releases passive hold");
            }
            let mut changed=helper.clone();changed.pid=21;
            assert!(!super::focus_loss_held(source,99,Some(&normal),Some(&changed),Some(&token),Some(Phase::Selecting)));
            changed=helper.clone();changed.instance=0;
            assert!(!super::focus_loss_held(source,99,Some(&normal),Some(&changed),Some(&token),Some(Phase::Selecting)));
            let mut stale=helper.clone();stale.token.as_mut().unwrap().capture_id="00000000000000000000000000000000".into();
            f.edges.registrations.lock().unwrap().insert(99,stale);assert!(!f.runtime.holds_focus_loss(source));
            f.edges.registrations.lock().unwrap().insert(99,helper);
            v::ack(&f.runtime,bar(),token.clone()).unwrap();assert_eq!(work.finish(),Ok(token.clone()));
            assert!(f.runtime.holds_focus_loss(source),"hold persists through Selecting");
            *f.edges.foreground.lock().unwrap()=88;assert!(!f.runtime.holds_focus_loss(source),"user switch wins");
            *f.edges.foreground.lock().unwrap()=99;*f.edges.switch_on_registration.lock().unwrap()=true;
            assert!(!f.runtime.holds_focus_loss(source),"switch during native adapter resolution wins");
            *f.edges.switch_on_registration.lock().unwrap()=false;*f.edges.foreground.lock().unwrap()=99;
            f.edges.registrations.lock().unwrap().remove(&source);assert!(!f.runtime.holds_focus_loss(source),"retired/hidden native adapter result grants no hold");
            let overlay=f.windows.live.lock().unwrap().values().next().unwrap().caller.clone();
            f.runtime.coordinator.cancel(overlay,token).unwrap();assert!(!f.runtime.holds_focus_loss(source));
            assert_eq!(f.count("show"),0);assert_eq!(f.count("restore"),0);f.clean();
        }
    }
    #[test]
    fn snip_runtime_expiry_during_create_closes_late_result_without_context_or_arming_then_retries() {
        let f=Fixture::new();let (entered_tx,entered)=mpsc::channel();let (release,release_rx)=mpsc::channel();
        *f.windows.create_gate.lock().unwrap()=Some((entered_tx,release_rx));
        let work=f.launch();let old=f.pending();v::ack(&f.runtime,bar(),old.clone()).unwrap();
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        f.expire();f.runtime.coordinator.expire();f.clean();
        assert_eq!(f.count(&format!("retire:{}:{}",old.generation,old.capture_id)),1,"pending generation must retire before create returns");
        assert_eq!(f.count("context"),0);assert_eq!(f.count("armed"),0);assert_eq!(f.count("show"),0);
        release.send(()).unwrap();assert_eq!(work.finish(),Err(SnipError::Stale));
        assert_eq!(f.count("create"),1);assert_eq!(f.count("close"),1);f.clean();
        assert_eq!(f.count("context"),0);assert_eq!(f.count("armed"),0);assert_eq!(f.count("show"),0);
        // Runtime repeat suppression permits retry only once the prior worker
        // returns. This proves late-return cleanup, not native Entry::bind effects.
        let work=f.launch();let current=f.pending();assert_ne!(old,current);
        assert_eq!(v::ack(&f.runtime,bar(),old),Err(SnipError::Stale));
        v::ack(&f.runtime,bar(),current.clone()).unwrap();assert_eq!(work.finish(),Ok(current.clone()));
        assert_eq!(f.count("context"),1);assert_eq!(f.count("close"),1);
        let overlay=f.windows.live.lock().unwrap().values().next().unwrap().caller.clone();
        f.runtime.coordinator.cancel(overlay,current).unwrap();f.clean();
    }
    #[test]
    fn snip_runtime_unregistered_source_never_holds_focus_loss() {
        let f=Fixture::new();assert!(!f.runtime.holds_focus_loss(200));
        let work=f.launch();let token=f.pending();
        assert!(!f.runtime.holds_focus_loss(200),"preparation alone cannot authorize an unknown source");
        v::ack(&f.runtime,bar(),token.clone()).unwrap();assert_eq!(work.finish(),Ok(token));
        for hwnd in [0,99,200] {assert!(!f.runtime.holds_focus_loss(hwnd),"fake WindowPort handles are not native registrations");}
        f.runtime.shutdown();assert!(!f.runtime.holds_focus_loss(200));
        assert_eq!(f.count("restore"),0);
    }
    #[test]
    fn snip_runtime_renderer_start_does_not_emit_hotkey_feedback() {
        let f=Fixture::new();*f.edges.visible.lock().unwrap()=false;
        assert_eq!(v::start(&f.runtime,bar()),Err(SnipError::Unauthorized));
        assert!(!f.edges.trace.lock().unwrap().iter().any(|event|event.starts_with("feedback:")));
        f.clean();
    }
    #[test]
    fn snip_runtime_failed_capture_retry_rejects_old_ack_and_preserves_new_generation() {
        let f=Fixture::new();*f.capture_fail.lock().unwrap()=true;
        let work=f.launch();let old=f.pending();v::ack(&f.runtime,bar(),old.clone()).unwrap();
        assert_eq!(work.finish(),Err(SnipError::CaptureFailed));f.released(&old);f.clean();
        *f.capture_fail.lock().unwrap()=false;
        let work=f.launch();let current=f.pending();assert_ne!(current,old);
        assert_eq!(v::ack(&f.runtime,bar(),old),Err(SnipError::Stale));
        assert!(!v::pending(&f.runtime).unwrap().3);assert_eq!(f.count("capture"),1);
        v::ack(&f.runtime,bar(),current.clone()).unwrap();assert_eq!(work.finish(),Ok(current.clone()));
        f.released(&current);assert_eq!(f.count("capture"),2);
        let overlay=f.windows.live.lock().unwrap().values().next().unwrap().caller.clone();
        f.runtime.coordinator.cancel(overlay,current).unwrap();f.clean();
    }
    #[test]
    fn snip_runtime_capture_failure_and_drop_leave_no_owned_resources() {
        let f=Fixture::new();*f.capture_fail.lock().unwrap()=true;let work=f.launch();let t=f.pending();v::ack(&f.runtime,bar(),t.clone()).unwrap();
        assert_eq!(work.finish(),Err(SnipError::CaptureFailed));f.released(&t);f.clean();drop(f.runtime.reserve_native_start().unwrap());
        let edges=f.edges.clone();let windows=f.windows.clone();let weak=Arc::downgrade(&f.runtime);drop(f);
        assert!(weak.upgrade().is_none());assert!(windows.live.lock().unwrap().is_empty());assert_eq!(edges.trace.lock().unwrap().iter().filter(|s|*s=="shutdown").count(),1);
        let f=Fixture::new();f.accepted();assert_eq!(f.windows.live.lock().unwrap().len(),1);
        let windows=f.windows.clone();let weak=Arc::downgrade(&f.runtime);drop(f);
        assert!(weak.upgrade().is_none());assert!(windows.live.lock().unwrap().is_empty());
        assert_eq!(windows.trace.lock().unwrap().iter().filter(|s|*s=="close").count(),1);
    }
}
