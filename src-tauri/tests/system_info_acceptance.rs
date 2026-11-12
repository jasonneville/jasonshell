//! Pure production coordinator contract; no Tauri window, OS collector, or shell startup.
//! A missing module/API is intentional RED until native implementation lands.
#[path = "../src/process_manager/system_info.rs"]
mod system_info;
use system_info::{Coordinator, Lane, ProcessManagerTab as Tab};

fn active() -> Coordinator {
    let mut state = Coordinator::default();
    let opened = state.open(Some(42)).unwrap();
    state.activate(opened.open_generation, opened.tab_revision + 1, Tab::SystemInfo, true, true).unwrap();
    state
}

#[test]
fn openings_and_hide_retire_epochs_and_never_restore_old_tab_or_pid() {
    let mut s = active();
    let old = s.context();
    let work = s.admit(Lane::Live, old.open_generation, old.tab_revision, 0, true, true).unwrap();
    s.hide().unwrap();
    assert!(!s.context().visible);
    assert!(!s.complete(work, 1));
    let next = s.open(None).unwrap();
    assert!(next.open_generation > old.open_generation);
    assert!(next.tab_revision > old.tab_revision);
    assert_eq!(next.active_tab, Tab::Processes);
    assert_eq!(next.focus_pid, None);
    let repeated = s.open(Some(99)).unwrap();
    assert!(repeated.open_generation > next.open_generation);
    assert_eq!(repeated.focus_pid, Some(99));
}

#[test]
fn stale_equal_future_generation_and_unsafe_revision_rejected_without_mutation() {
    let mut s = active(); let c = s.context();
    for (generation, revision) in [(c.open_generation - 1,c.tab_revision + 1), (c.open_generation + 1,c.tab_revision + 1), (c.open_generation,c.tab_revision), (c.open_generation,c.tab_revision - 1), (c.open_generation,9_007_199_254_740_992)] {
        assert!(s.activate(generation,revision,Tab::Processes,true,true).is_err());
        assert_eq!(s.context(), c);
    }
    assert!(s.activate(c.open_generation,c.tab_revision+1,Tab::Processes,false,true).is_err());
    assert!(s.activate(c.open_generation,c.tab_revision+1,Tab::Processes,true,false).is_err());
    assert_eq!(s.context(), c);
}

#[test]
fn independent_lane_cadences_have_one_outstanding_and_no_queued_admission() {
    let mut s = active(); let c = s.context();
    let live = s.admit(Lane::Live,c.open_generation,c.tab_revision,0,true,true).unwrap();
    let storage = s.admit(Lane::Storage,c.open_generation,c.tab_revision,0,true,true).unwrap();
    let inventory = s.admit(Lane::Inventory,c.open_generation,c.tab_revision,0,true,true).unwrap();
    for lane in [Lane::Live,Lane::Storage,Lane::Inventory] { assert!(s.admit(lane,c.open_generation,c.tab_revision,100_000,true,true).is_none()); }
    assert!(s.complete(live,0)); assert!(s.complete(storage,0)); assert!(s.complete(inventory,0));
    assert!(s.admit(Lane::Live,c.open_generation,c.tab_revision,1_999,true,true).is_none());
    assert!(s.admit(Lane::Storage,c.open_generation,c.tab_revision,14_999,true,true).is_none());
    assert!(s.admit(Lane::Inventory,c.open_generation,c.tab_revision,59_999,true,true).is_none());
    assert!(s.admit(Lane::Live,c.open_generation,c.tab_revision,2_000,true,true).is_some());
    assert!(s.admit(Lane::Storage,c.open_generation,c.tab_revision,15_000,true,true).is_some());
    assert!(s.admit(Lane::Inventory,c.open_generation,c.tab_revision,60_000,true,true).is_some());
}

#[test]
fn retirement_keeps_old_worker_slot_until_completion_even_across_reopen() {
    let mut s = active(); let c = s.context();
    let old = s.admit(Lane::Storage,c.open_generation,c.tab_revision,0,true,true).unwrap();
    s.hide().unwrap(); let c = s.open(None).unwrap();
    let c = s.activate(c.open_generation,c.tab_revision+1,Tab::SystemInfo,true,true).unwrap();
    assert!(s.admit(Lane::Storage,c.open_generation,c.tab_revision,100_000,true,true).is_none());
    assert!(s.admit(Lane::Live,c.open_generation,c.tab_revision,100_000,true,true).is_some());
    assert!(!s.complete(old,100_000));
    assert!(s.admit(Lane::Storage,c.open_generation,c.tab_revision,100_000,true,true).is_some());
}

#[test]
fn hidden_unfocused_wrong_epoch_and_inactive_tab_never_admit_work() {
    let mut s = active(); let c = s.context();
    for (generation,revision,visible,focused) in [(c.open_generation,c.tab_revision,false,true),(c.open_generation,c.tab_revision,true,false),(c.open_generation+1,c.tab_revision,true,true),(c.open_generation,c.tab_revision+1,true,true)] {
        assert!(s.admit(Lane::Live,generation,revision,0,visible,focused).is_none());
    }
    let c = s.activate(c.open_generation,c.tab_revision+1,Tab::Processes,true,true).unwrap();
    assert!(s.admit(Lane::Live,c.open_generation,c.tab_revision,0,true,true).is_none());
}

#[test]
fn numeric_helpers_reject_counter_reset_nonfinite_and_precision_loss() {
    use system_info::{network_rate,memory_used_percent,gpu_busiest_engine,safe_number,valid_probe_root};
    assert_eq!(network_rate(100,300,2_000),Some(100.0));
    assert_eq!(network_rate(300,100,2_000),None);
    assert_eq!(network_rate(100,300,0),None);
    assert_eq!(network_rate(100,100,2_000),Some(0.0));
    assert_eq!(memory_used_percent(16_000,4_000),Some(75.0));
    assert_eq!(memory_used_percent(0,0),None);
    assert_eq!(memory_used_percent(100,101),None);
    assert_eq!(gpu_busiest_engine(&[40.0,70.0,30.0]),Some(70.0));
    assert_eq!(gpu_busiest_engine(&[]),None);
    assert_eq!(gpu_busiest_engine(&[f64::NAN]),None);
    assert_eq!(gpu_busiest_engine(&[f64::INFINITY]),None);
    assert_eq!(gpu_busiest_engine(&[-1.0]),None);
    assert_eq!(safe_number(9_007_199_254_740_991),Some(9_007_199_254_740_991));
    assert_eq!(safe_number(9_007_199_254_740_992),None);
    for root in ["C:\\","Z:\\"] { assert!(valid_probe_root(root)); }
    for root in ["C:","C:\\secret","\\\\server\\share","C:\\..\\","C:\\;whoami","c:\\",""] { assert!(!valid_probe_root(root),"{root}"); }
}

#[test]
fn unauthorized_label_never_inspects_instance_and_all_denials_are_sanitized() {
    use system_info::authorize_manager;
    for label in ["bottom-bar","top-bar","unknown","process-manager-other",""] {
        assert_eq!(authorize_manager(label,|| panic!("unauthorized caller reached instance lookup")),Err("unauthorized"));
    }
    assert_eq!(authorize_manager("process-manager",|| false),Err("unauthorized"));
    assert_eq!(authorize_manager("process-manager",|| true),Ok(()));
}

#[test]
fn probe_admission_two_global_one_per_root_survives_retire_and_reopen_until_reaped() {
    use system_info::ProbeBudget;
    let mut budget = ProbeBudget::default();
    for root in ["C:\\secret","\\\\server\\share","C:",""] { assert!(budget.reserve(root,1,2,0).is_none()); }
    let a = budget.reserve("C:\\",1,2,0).unwrap();
    assert!(budget.reserve("C:\\",1,2,0).is_none());
    let b = budget.reserve("D:\\",1,2,0).unwrap();
    assert!(budget.reserve("E:\\",1,2,0).is_none());
    assert!(budget.may_publish(&a,1,2));
    assert!(!budget.may_publish(&a,2,4));
    budget.retire(1,2);
    assert!(!budget.may_publish(&a,1,2));
    assert!(budget.reserve("E:\\",2,4,1).is_none());
    let killed = budget.kill_requests(1);
    assert_eq!(killed.len(),2); assert!(killed.contains(&a)); assert!(killed.contains(&b));
    assert!(budget.kill_requests(2).is_empty(),"kill requests exactly once");
    assert!(budget.reserve("E:\\",2,4,2).is_none(),"kill request is not reaping");
    assert!(budget.reaped(&a)); assert!(!budget.reaped(&a));
    let c = budget.reserve("E:\\",2,4,2).unwrap();
    assert!(budget.may_publish(&c,2,4));
    assert!(!budget.may_publish(&a,1,2));
    assert!(budget.reserve("F:\\",2,4,2).is_none());
    assert!(budget.reaped(&b)); assert!(budget.reaped(&c));
}

#[test]
fn probe_deadline_boundary_requests_kill_once_without_releasing_slot() {
    use system_info::ProbeBudget;
    let mut budget = ProbeBudget::default(); let token = budget.reserve("Z:\\",4,8,100).unwrap();
    assert!(budget.kill_requests(1_599).is_empty()); assert!(budget.may_publish(&token,4,8));
    assert_eq!(budget.kill_requests(1_600),vec![token.clone()]);
    assert!(!budget.may_publish(&token,4,8)); assert!(budget.kill_requests(10_000).is_empty());
    assert!(budget.reserve("Z:\\",5,10,10_000).is_none());
    assert!(budget.reaped(&token)); assert!(budget.reserve("Z:\\",5,10,10_000).is_some());
}

#[test]
fn retiring_another_epoch_never_kills_or_releases_current_probe() {
    use system_info::ProbeBudget;
    let mut budget = ProbeBudget::default(); let token = budget.reserve("C:\\",2,4,0).unwrap();
    budget.retire(1,2); assert!(budget.kill_requests(1).is_empty()); assert!(budget.may_publish(&token,2,4));
}

fn assert_safe_wire(result: Result<serde_json::Value, serde_json::Error>) {
    // Rejecting malformed wire data outright is also an acceptable fail-closed result.
    let Ok(value) = result else { return; };
    fn visit(value: &serde_json::Value) {
        match value {
            serde_json::Value::Number(number) => {
                let number = number.as_f64().expect("JSON numeric value");
                assert!(number.is_finite() && number.abs() <= 9_007_199_254_740_991.0,"unsafe JSON number {number}");
            }
            serde_json::Value::Array(values) => { for value in values { visit(value); } }
            serde_json::Value::Object(object) => {
                if object.get("state").and_then(|state|state.as_str()) == Some("ready") && object.contains_key("value") {
                    assert!(!object["value"].is_null(),"ready sample cannot silently become null");
                }
                for value in object.values() { visit(value); }
            }
            _ => {}
        }
    }
    visit(&value);
}

#[test]
fn warming_snapshot_is_versioned_truthful_and_privacy_allowlisted() {
    let s = active();
    let wire = serde_json::to_value(system_info::SystemInfoSnapshot::warming(&s.context(),1,123)).unwrap();
    assert_eq!(wire["schemaVersion"],1,"versioned cache contract");
    assert_eq!(wire["openGeneration"],s.context().open_generation);
    assert_eq!(wire["tabRevision"],s.context().tab_revision);
    assert_eq!(wire["live"]["cpu"]["state"],"warming_up");
    assert!(wire["live"]["cpu"]["value"].is_null());
    assert_eq!(wire["live"]["cpu"]["reason"],"no_baseline");
    for sensor in ["cpuTemperatureC","gpuTemperatureC"] {
        assert_eq!(wire["live"]["sensors"][sensor]["state"],"unavailable");
        assert_eq!(wire["live"]["sensors"][sensor]["reason"],"provider_not_verified");
        assert!(wire["live"]["sensors"][sensor]["value"].is_null());
    }
    let inventory = wire["inventory"].as_object().unwrap();
    let allowlist = ["state","sampledAtMs","cpuName","physicalCores","logicalProcessors","installedRamBytes","gpuNames","vramBytes","board","bios","windowsVersion"];
    assert_eq!(inventory.len(),allowlist.len());
    assert!(inventory.keys().all(|key|allowlist.contains(&key.as_str())),"unexpected inventory identifier");
    assert_safe_wire(Ok(wire));
}

#[test]
fn nonfinite_ready_measurements_never_serialize_as_ready_null() {
    for malformed in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY] {
        let mut wire = system_info::SystemInfoSnapshot::warming(&active().context(),1,123);
        wire.live.cpu = system_info::Sample::ready(malformed,"fixture",Some("percent"),123);
        assert_safe_wire(serde_json::to_value(wire));
    }
}

#[test]
fn unsafe_integer_telemetry_never_crosses_json_precision_boundary() {
    let mut wire = system_info::SystemInfoSnapshot::warming(&active().context(),1,123);
    wire.live.memory.total_bytes = system_info::Sample::ready(9_007_199_254_740_992,"fixture",Some("bytes"),123);
    assert_safe_wire(serde_json::to_value(wire));
}

#[test]
fn safe_integer_boundary_survives_serialization_exactly() {
    let mut wire = system_info::SystemInfoSnapshot::warming(&active().context(),1,123);
    wire.live.memory.total_bytes = system_info::Sample::ready(9_007_199_254_740_991,"fixture",Some("bytes"),123);
    let value = serde_json::to_value(wire).unwrap();
    assert_eq!(value["live"]["memory"]["totalBytes"]["value"].as_u64(),Some(9_007_199_254_740_991));
    assert_safe_wire(Ok(value));
}

#[test]
fn unsafe_snapshot_revisions_and_timestamps_fail_closed_on_wire() {
    for (revision,timestamp) in [(9_007_199_254_740_992,123),(1,9_007_199_254_740_992)] {
        let wire = system_info::SystemInfoSnapshot::warming(&active().context(),revision,timestamp);
        assert_safe_wire(serde_json::to_value(wire));
    }
}

#[test]
fn disk_caller_quota_and_total_free_are_distinct_wire_values() {
    use system_info::{Sample, StorageRoot};
    let root = StorageRoot { root: "C:\\".into(), kind: "fixed".into(), filesystem: Sample::ready("NTFS".to_string(),"fixture",None,123),
        total_bytes: Sample::ready(1_000,"fixture",Some("bytes"),123),
        total_free_bytes: Sample::ready(700,"fixture",Some("bytes"),123),
        caller_available_bytes: Sample::ready(300,"fixture",Some("bytes"),123) };
    let wire = serde_json::to_value(root).unwrap();
    assert_eq!(wire["totalBytes"]["value"],1_000);
    assert_eq!(wire["totalFreeBytes"]["value"],700);
    assert_eq!(wire["callerAvailableBytes"]["value"],300);
    assert_ne!(wire["totalFreeBytes"],wire["callerAvailableBytes"]);
}

fn cached_state() -> system_info::RuntimeInner {
    let mut state=system_info::RuntimeInner::default();let c=state.coordinator.open(None).unwrap();
    state.coordinator.activate(c.open_generation,c.tab_revision+1,Tab::SystemInfo,true,true).unwrap();state.warm(123).unwrap();state
}
#[test]
fn actual_runtime_cache_reads_are_detached_monotonic_and_never_admit_collectors_or_probes() {
    let mut state=cached_state();let c=state.coordinator.context();let before=serde_json::to_value(state.cache.as_ref().unwrap()).unwrap();
    let live=state.coordinator.admit(Lane::Live,c.open_generation,c.tab_revision,0,true,true).unwrap();
    let probe=state.probes.reserve("C:\\",c.open_generation,c.tab_revision,0).unwrap();
    for _ in 0..1000 {
        let mut clone=state.read_snapshot(c.open_generation,c.tab_revision,None,true,true).unwrap().unwrap();
        assert_eq!(clone.snapshot_revision,1);clone.live.cpu=system_info::Sample::ready(99.0,"mutated detached clone",Some("percent"),123);
        assert!(state.read_snapshot(c.open_generation,c.tab_revision,Some(0),true,true).unwrap().is_some());
        assert!(state.read_snapshot(c.open_generation,c.tab_revision,Some(1),true,true).unwrap().is_none());
        assert!(state.read_snapshot(c.open_generation,c.tab_revision,Some(999),true,true).unwrap().is_none());
    }
    assert_eq!(state.revision,1);assert_eq!(serde_json::to_value(state.cache.as_ref().unwrap()).unwrap(),before);
    assert!(state.coordinator.admit(Lane::Live,c.open_generation,c.tab_revision,100_000,true,true).is_none(),"reader must not release running collector");
    assert!(state.coordinator.admit(Lane::Storage,c.open_generation,c.tab_revision,0,true,true).is_some(),"reads did not enqueue inactive lane");
    assert!(state.coordinator.admit(Lane::Inventory,c.open_generation,c.tab_revision,0,true,true).is_some());
    assert!(state.probes.reserve("C:\\",c.open_generation,c.tab_revision,0).is_none());
    assert!(state.probes.reserve("D:\\",c.open_generation,c.tab_revision,0).is_some(),"reader did not consume spare probe slot");
    assert!(state.probes.may_publish(&probe,c.open_generation,c.tab_revision));assert!(state.coordinator.complete(live,100_000));
    assert!(state.coordinator.admit(Lane::Live,c.open_generation,c.tab_revision,101_999,true,true).is_none());
    assert!(state.coordinator.admit(Lane::Live,c.open_generation,c.tab_revision,102_000,true,true).is_some(),"only actual completion controls next deadline");
}
#[test]
fn cache_read_finishes_while_actual_lane_token_is_held_by_pending_collector() {
    use std::sync::{Arc,Mutex,mpsc};use std::time::Duration;
    let mut state=cached_state();let c=state.coordinator.context();
    let token=state.coordinator.admit(Lane::Live,c.open_generation,c.tab_revision,0,true,true).unwrap();
    let shared=Arc::new(Mutex::new(state));let reader_state=shared.clone();let (done_tx,done_rx)=mpsc::channel();
    // Collector is deliberately unresolved. The production cache method has no
    // collector reference/callback and must complete without that lane token.
    let reader=std::thread::spawn(move|| {let state=reader_state.lock().unwrap();let read=state.read_snapshot(c.open_generation,c.tab_revision,None,true,true).unwrap();done_tx.send(read.is_some()).unwrap();});
    assert!(done_rx.recv_timeout(Duration::from_secs(2)).expect("cache read blocked on outstanding collection"));reader.join().unwrap();
    let mut state=shared.lock().unwrap();assert!(state.coordinator.admit(Lane::Live,c.open_generation,c.tab_revision,100_000,true,true).is_none());assert!(state.coordinator.complete(token,0));
}
#[test]
fn runtime_cache_rejects_hidden_unfocused_stale_unsafe_and_retired_epochs_before_reading_data() {
    let mut state=cached_state();let c=state.coordinator.context();
    for (g,r,visible,focused) in [(c.open_generation,c.tab_revision,false,true),(c.open_generation,c.tab_revision,true,false),(c.open_generation+1,c.tab_revision,true,true),(c.open_generation,c.tab_revision+1,true,true)] {
        assert!(state.read_snapshot(g,r,None,visible,focused).is_err());
    }
    assert!(state.read_snapshot(c.open_generation,c.tab_revision,Some(9_007_199_254_740_992),true,true).is_err());
    let next=state.coordinator.activate(c.open_generation,c.tab_revision+1,Tab::Processes,true,true).unwrap();
    assert!(state.read_snapshot(next.open_generation,next.tab_revision,None,true,true).is_err());
    state.invalidate();state.coordinator.hide().unwrap();let next=state.coordinator.open(None).unwrap();
    let next=state.coordinator.activate(next.open_generation,next.tab_revision+1,Tab::SystemInfo,true,true).unwrap();state.warm(234).unwrap();
    assert!(state.read_snapshot(c.open_generation,c.tab_revision,None,true,true).is_err());
    let cache=state.read_snapshot(next.open_generation,next.tab_revision,None,true,true).unwrap().unwrap();
    assert_eq!(cache.open_generation,next.open_generation);assert_eq!(cache.tab_revision,next.tab_revision);assert_eq!(cache.snapshot_revision,2);
    assert!(matches!(cache.live.cpu.state,system_info::SampleState::WarmingUp));assert!(cache.live.cpu.value.is_none());
}
#[test]
fn runtime_cache_revision_exhaustion_fails_closed_and_clears_old_data() {
    let mut state=cached_state();state.revision=9_007_199_254_740_991;assert!(state.warm(234).is_err());assert!(state.cache.is_none());assert!(!state.coordinator.context().visible);
}
