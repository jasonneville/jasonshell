//! Actual-source temporary diagnostic acceptance. No hook/app/input injection,
//! no global opt-in creation. Each test owns only its approved-temp subdirectory.
//! API/event aliases may be aligned with owner implementation without weakening
//! privacy, bounded storage, default-off or hot-path allocation assertions.
#[path = "../src/snipping/diagnostics.rs"]
mod diagnostics;
use diagnostics::{Diagnostics,Event};
use std::{alloc::{GlobalAlloc,Layout,System},cell::Cell,path::PathBuf,
    sync::{Arc,atomic::{AtomicUsize,Ordering}},thread,time::{Duration,Instant}};

struct TrackingAllocator;
thread_local! {static TRACK:Cell<bool>=const {Cell::new(false)};}
static ALLOCATIONS:AtomicUsize=AtomicUsize::new(0);
fn count_allocation(){if TRACK.try_with(Cell::get).unwrap_or(false){ALLOCATIONS.fetch_add(1,Ordering::Relaxed);}}
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self,l:Layout)->*mut u8 {count_allocation();unsafe {System.alloc(l)}}
    unsafe fn alloc_zeroed(&self,l:Layout)->*mut u8 {count_allocation();unsafe {System.alloc_zeroed(l)}}
    unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8 {count_allocation();unsafe {System.realloc(p,l,n)}}
    unsafe fn dealloc(&self,p:*mut u8,l:Layout){unsafe {System.dealloc(p,l)}}
}
#[global_allocator] static ALLOCATOR:TrackingAllocator=TrackingAllocator;
static NEXT:AtomicUsize=AtomicUsize::new(0);
struct OwnedDirectory(PathBuf);
impl OwnedDirectory {
    fn new()->Self {
        let approved=PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode");
        assert!(!std::fs::symlink_metadata(&approved).unwrap().file_type().is_symlink());
        let path=approved.join(format!("snip-diagnostic-test-{}-{}-{}",std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::create_dir(&path).unwrap();Self(path)
    }
    fn marker(&self)->PathBuf {self.0.join("snip-escape-diagnostics.enable")}
    fn output(&self)->PathBuf {self.0.join(format!("snip-escape-diagnostics-{}.json",std::process::id()))}
    fn enable(&self)->Arc<Diagnostics> {
        // Isolated TEST subdirectory only. Leader owns the real startup sentinel.
        std::fs::write(self.marker(),b"synthetic-test-only").unwrap();
        Diagnostics::start(&self.0).expect("explicit owned marker enables temporary diagnostics")
    }
}
impl Drop for OwnedDirectory {fn drop(&mut self){for _ in 0..60 {
    if std::fs::remove_dir_all(&self.0).is_ok()||!self.0.exists(){return;}
    thread::sleep(Duration::from_millis(20));
}}}
const FIELDS:&[&str]=&[
    "schema","process_id","enabled","observed_generation","escape_down","escape_up","escape_repeat",
    "hook_registered","hook_install_failed","pump_ticks","hook_exit","worker_exit","shutdown",
    "route_admitted","route_no_generation","enqueued","drained","delivery_accepted","delivery_rejected",
    "begin","arm_begin","arm_end","cancel_accepted","cancel_rejected","cancel_deferred","phase","busy","armed","deferred",
    "callback_escape_down","callback_escape_up","callback_tls_missing","callback_owner_missing",
];
fn snapshot(d:&Diagnostics)->serde_json::Value {serde_json::to_value(d.snapshot()).unwrap()}
fn file_snapshot(root:&OwnedDirectory,predicate:impl Fn(&serde_json::Value)->bool)->serde_json::Value {
    let deadline=Instant::now()+Duration::from_secs(3);
    while Instant::now()<deadline {
        if let Ok(metadata)=std::fs::symlink_metadata(root.output()) {
            assert!(metadata.is_file()&&!metadata.file_type().is_symlink());assert!(metadata.len()<=4096);
            if let Ok(bytes)=std::fs::read(root.output()) {if let Ok(value)=serde_json::from_slice(&bytes) {
                if predicate(&value){return value;}
            }}
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("deferred diagnostic writer did not publish bounded expected counters");
}

#[test]
fn diagnostic_startup_defaults_off_and_marker_is_checked_once() {
    let root=OwnedDirectory::new();
    assert!(Diagnostics::start(&root.0).is_none());
    thread::sleep(Duration::from_millis(300));assert!(!root.output().exists(),"disabled startup creates no output");
    let d=root.enable();std::fs::remove_file(root.marker()).unwrap();
    // No Selecting generation is involved. Escape edges remain observable even
    // outside selection, and removing the marker does not reconfigure live state.
    d.record(Event::EscapeDown);d.record(Event::EscapeUp);
    let file=file_snapshot(&root,|v|v["escape_down"]==1&&v["escape_up"]==1);
    assert_eq!(file["enabled"],true);assert_eq!(file["process_id"],std::process::id());
    d.shutdown();d.shutdown();drop(d);
    assert!(Diagnostics::start(&root.0).is_none(),"new startup without marker stays disabled");
}

#[test]
fn diagnostic_snapshot_is_allowlisted_metadata_not_a_keyboard_or_text_log() {
    let root=OwnedDirectory::new();let d=root.enable();
    // Event API accepts no raw virtual-key value, typed text, captureId, HWND,
    // window title or renderer-controlled payload. Actual callback must filter
    // vkCode==Escape BEFORE constructing these typed diagnostic events.
    d.record(Event::EscapeDown);d.record(Event::EscapeUp);d.record(Event::EscapeRepeat);
    d.record(Event::ObservedGeneration(9));
    for event in [Event::HookRegistered,Event::PumpTick,Event::HookExit,Event::WorkerExit,
        Event::RouteAdmitted,Event::RouteNoGeneration,Event::Enqueued,Event::Drained,
        Event::DeliveryAccepted,Event::DeliveryRejected,Event::Begin,Event::ArmBegin,
        Event::ArmEnd,Event::CancelAccepted,Event::CancelRejected] {d.record(event);}
    d.record(Event::ObservedGeneration(9));
    let value=snapshot(&d);let object=value.as_object().unwrap();
    // These are pre-route callback observations. Manual typed route-event
    // injection above must not counterfeit native callback/TLS/owner evidence.
    for field in ["callback_escape_down","callback_escape_up","callback_tls_missing","callback_owner_missing"] {
        assert_eq!(value[field],0,"unobserved native callback counter must start at zero: {field}");
    }
    for required in FIELDS {assert!(object.contains_key(*required),"required diagnostic distinction missing: {required}");}
    for (name,value) in object {
        assert!(FIELDS.contains(&name.as_str()),"unexpected diagnostic field {name}");
        if matches!(name.as_str(),"enabled"|"busy"|"armed"|"deferred") {assert!(value.is_boolean());}
        else {assert!(value.as_u64().is_some(),"numeric/enums only: {name}");}
    }
    assert_eq!(value["escape_down"],1);assert_eq!(value["escape_up"],1);assert_eq!(value["escape_repeat"],1);
    assert_eq!(value["observed_generation"],9);
    let file=file_snapshot(&root,|v|v["escape_up"]==1&&v["delivery_accepted"]==1);
    assert_eq!(file.as_object().unwrap().len(),FIELDS.len(),"file must not append raw event/key payloads");
    d.shutdown();drop(d);
}

#[test]
fn diagnostic_record_and_snapshot_allocate_nothing_and_remain_fixed_size() {
    fn require_copy<T:Copy>(_:T){}
    let root=OwnedDirectory::new();let d=root.enable();
    TRACK.with(|flag|flag.set(false));let start=ALLOCATIONS.load(Ordering::Relaxed);
    TRACK.with(|flag|flag.set(true));
    for generation in 1..=100_000 {
        d.record(Event::EscapeDown);d.record(Event::EscapeUp);
        d.record(Event::ObservedGeneration(generation));d.record(Event::Enqueued);
        d.record(Event::Drained);d.record(Event::DeliveryAccepted);
        std::hint::black_box(d.snapshot());
    }
    TRACK.with(|flag|flag.set(false));
    assert_eq!(ALLOCATIONS.load(Ordering::Relaxed)-start,0,"hot-path record/snapshot must not allocate/reallocate");
    require_copy(d.snapshot());assert!(std::mem::size_of_val(&*d)<=1024,"fixed atomics, not a growing event history");
    assert!(serde_json::to_vec(&d.snapshot()).unwrap().len()<=4096);
    let value=snapshot(&d);assert_eq!(value["escape_down"],100_000);assert_eq!(value["escape_up"],100_000);
    assert_eq!(value["observed_generation"],100_000);
    // Filesystem work belongs solely to the weak-owned deferred writer. Its
    // records do not delay shutdown/drop; actual call-graph no-lock/no-I/O review
    // remains mandatory in addition to this allocation measurement.
    let began=Instant::now();d.shutdown();d.shutdown();drop(d);
    assert!(began.elapsed()<Duration::from_millis(250),"shutdown/drop must not join a filesystem writer");
}

#[test]
fn diagnostic_deferred_arming_result_and_latest_flags_remain_distinguishable() {
    let root=OwnedDirectory::new();let d=root.enable();
    d.record(Event::Begin);d.record(Event::ArmBegin);d.record(Event::CancelDeferred);
    // Phase is presentation-only numeric/enum metadata, never cancellation
    // authority. Equivalent owner enum aliases may replace this field spelling.
    d.record(Event::State {phase:2,busy:true,armed:false,deferred:true});
    let deferred=snapshot(&d);
    assert_eq!(deferred["phase"],2);assert_eq!(deferred["busy"],true);
    assert_eq!(deferred["armed"],false);assert_eq!(deferred["deferred"],true);
    assert_eq!(deferred["cancel_deferred"],1);assert_eq!(deferred["cancel_accepted"],0);
    d.record(Event::CancelAccepted);
    d.record(Event::State {phase:0,busy:false,armed:false,deferred:false});
    let completed=snapshot(&d);
    assert_eq!(completed["phase"],0);assert_eq!(completed["busy"],false);assert_eq!(completed["deferred"],false);
    assert_eq!(completed["cancel_deferred"],1);assert_eq!(completed["cancel_accepted"],1);
    d.record(Event::CancelRejected);assert_eq!(snapshot(&d)["cancel_rejected"],1);
    d.shutdown();drop(d);
}
