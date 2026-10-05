// Include at actual runtime module scope under cfg(test), or via the standalone
// actual-source integration harness. No duplicated runtime/foreground state machine.
mod runtime_acceptance {
    use crate::snipping::{coordinator::*, foreground::{Intent, verification::Tracker},
        geometry::{CroppedImage, PhysicalRect}, native::PreparationBarrier,
        native_registry::NativeRegistry, runtime::{RuntimeEdges, SnipRuntime, verification as v}};
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
    struct Windows {live:Mutex<HashMap<u64,WindowRef>>,next:Mutex<u64>,trace:Trace}
    impl Windows {
        fn create(&self,t:&Token,m:&MonitorSnapshot,preview:bool)->Result<WindowRef,SnipError> {
            let mut next=self.next.lock().unwrap(); *next+=1;
            let w=WindowRef {caller:Caller {label:if preview {format!("snip-preview-{}",t.generation)} else {format!("snip-overlay-{}-{}",t.generation,m.id)},window_id:*next},monitor_id:Some(m.id.clone())};
            self.live.lock().unwrap().insert(*next,w.clone()); record(&self.trace,"create"); Ok(w)
        }
    }
    impl WindowPort for Windows {
        fn create_overlay(&self,t:&Token,m:&MonitorSnapshot)->Result<WindowRef,SnipError>{self.create(t,m,false)}
        fn create_preview(&self,t:&Token,m:&MonitorSnapshot,_:u32,_:u32)->Result<WindowRef,SnipError>{self.create(t,m,true)}
        fn emit_context(&self,_:&WindowRef,_:&Context)->Result<(),SnipError>{Ok(())}
        fn emit_armed(&self,_:&WindowRef,_:&Token)->Result<(),SnipError>{Ok(())}
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
            let trace=Arc::new(Mutex::new(Vec::new())); let (tx,prepared)=mpsc::channel();
            let mut tracker=Tracker::new(); tracker.foreground(10,Some((20,30)),false);
            let edges=Arc::new(Edges {trace:trace.clone(),tracker:Mutex::new(tracker),time:Mutex::new(Instant::now()),visible:Mutex::new(true),foreground:Mutex::new(99),prepared:tx,focus_change_on_valid:Mutex::new(false),fail_emit:Mutex::new(false),flush_effect:Mutex::new("none"),flush_gate:Mutex::new(None)});
            let windows=Arc::new(Windows {live:Mutex::new(HashMap::new()),next:Mutex::new(100),trace});
            let capture_fail=Arc::new(Mutex::new(false)); let (e,f)=(edges.clone(),capture_fail.clone());
            let runtime=v::construct(bar(),Arc::new(NativeRegistry::default()),edges.clone(),v::Ports {
                capture:Box::new(move |prepare,release|Arc::new(Capture {prepare,release,edges:e,fail:f})),
                windows:windows.clone(),clipboard:Arc::new(NoPublication),picker:Arc::new(NoPublication),publisher:Arc::new(NoPublication),clock:Arc::new(Clock(edges.clone()))});
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
            match event {"destroy"=>tracker.destroyed(10),"create"=>tracker.created(10),"unknown"=>tracker.foreground(0,None,false),"different"=>tracker.foreground(11,Some((20,30)),false),"same-hwnd-new-epoch"=>tracker.foreground(10,Some((20,30)),false),_=>tracker.lose_continuity()}
            assert!(!tracker.valid(old,Some((20,30))),"{event}: same HWND/PID cannot inherit native instance continuity");
        }
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
