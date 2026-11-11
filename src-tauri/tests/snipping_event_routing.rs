//! Real hidden WebView2 JS event transport + actual NativeWindowPort emission.
//! No production main/handlers, AppBars, hooks, capture, clipboard or foreground changes.
#![cfg(windows)]
#[path = "../src/snipping/mod.rs"] mod snipping;
use snipping::{coordinator::*, native::NativeWindowPort, native_registry::NativeRegistry};
use std::{collections::BTreeMap, path::PathBuf, process::{Command, Stdio}, sync::{Arc, Mutex, mpsc, atomic::{AtomicBool, Ordering}}, thread, time::{Duration, Instant}};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, webview::PageLoadEvent};
const NAME: &str = "native_snip_events_are_delivered_only_to_registered_destination";
const LABELS: [&str; 2] = ["snip-overlay-7-m0", "snip-overlay-7-m1"];
struct OwnedDirectory(PathBuf);
impl Drop for OwnedDirectory { fn drop(&mut self) { for _ in 0..20 {
    if std::fs::remove_dir_all(&self.0).is_ok() || !self.0.exists() { return; }
    thread::sleep(Duration::from_millis(100));
}}}
fn parent() {
    let id = format!("snip-events-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    let path = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode").join(&id);
    std::fs::create_dir(&path).unwrap(); let directory = OwnedDirectory(path.clone());
    std::fs::write(path.join("parent-owner"), id.as_bytes()).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", NAME, "--test-threads=1", "--nocapture"])
        .env("JASONSHELL_SNIP_EVENTS_CHILD_ID", &id).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(50); let mut timed_out = false;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline { timed_out = true; child.kill().expect("terminate exact owned fixture child"); break; }
        thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    drop(directory); assert!(!path.exists(), "exact parent-owned WebView profile cleanup after child exit");
    eprintln!("event fixture child exited; exact parent-owned profile cleanup verified");
    assert!(output.stdout.len() + output.stderr.len() < 32768, "bounded synthetic metadata");
    eprintln!("{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(!timed_out, "event transport fixture timed out");
    assert!(output.status.success(), "actual native event isolation regression failed");
    assert!(String::from_utf8_lossy(&output.stdout).contains("EVENT_BARRIERS_COMPLETED=8"), "no zero-stage success");
}
fn token() -> Token { Token { generation: "7".into(), capture_id: "77777777777777777777777777777777".into() } }
fn context(monitor: &str) -> Context {
    Context { token: token(), phase: Phase::Selecting, monitor_id: monitor.into(), width: 2, height: 2, scale_factor: 1.0, origin_x: 0, origin_y: 0 }
}
type Reports = BTreeMap<(usize, String), serde_json::Value>;
fn drive(port: NativeWindowPort, registry: Arc<NativeRegistry>, windows: Vec<tauri::WebviewWindow>, receiver: mpsc::Receiver<(String, serde_json::Value)>) -> Result<Reports, &'static str> {
    let mut ready = BTreeMap::new();
    while ready.len() < 2 {
        let (label, value) = receiver.recv_timeout(Duration::from_secs(15)).map_err(|_| "listener readiness deadline")?;
        if value["stage"] != "ready" || value["bridge"] != true { return Err("actual JS listeners did not initialize"); }
        if ready.insert(label, ()).is_some() { return Err("duplicate listener readiness"); }
    }
    // Fresh registry initialization deadline begins only after both JS listeners
    // exist; no production instance/token/monitor authorization is bypassed.
    let mut references = Vec::new();
    for (i, window) in windows.iter().enumerate() {
        let entry = registry.reserve(LABELS[i].into(), Some(token()), Some(format!("m{i}")), false).map_err(|_| "reserve failed")?;
        entry.bind(window.clone()).map_err(|_| "bind failed")?;
        references.push(entry.reference.clone());
    }
    let mut reports = BTreeMap::new();
    for phase in 0..4 {
        let destination = phase % 2;
        if phase < 2 { port.emit_context(&references[destination], &context(&format!("m{destination}"))).map_err(|_| "production context emission failed")?; }
        else { port.emit_armed(&references[destination], &token()).map_err(|_| "production armed emission failed")?; }
        // Production event delivery and WebviewWindow::eval queue JS evaluation
        // on the same native per-WebView dispatcher. Callbacks record synchronously.
        // An independent per-window eval marker drains earlier callbacks without
        // assuming event routing itself works. All 8 reports are mandatory.
        for window in &windows { window.eval(&format!("window.__snipEventDrain({phase})")).map_err(|_| "drain marker evaluation failed")?; }
        let mut received = 0;
        while received < 2 {
            let (label, value) = receiver.recv_timeout(Duration::from_secs(5)).map_err(|_| "event drain deadline")?;
            if value["stage"] != "drain" || value["phase"].as_u64() != Some(phase as u64) { return Err("out-of-order drain report"); }
            if reports.insert((phase, label), value).is_some() { return Err("duplicate drain report"); }
            received += 1;
        }
    }
    Ok(reports)
}
#[test]
#[ignore = "requires JASONSHELL_SNIP_EVENTS_TESTS=1; isolated hidden native WebViews only"]
fn native_snip_events_are_delivered_only_to_registered_destination() {
    assert_eq!(std::env::var("JASONSHELL_SNIP_EVENTS_TESTS").as_deref(), Ok("1"));
    let Ok(id) = std::env::var("JASONSHELL_SNIP_EVENTS_CHILD_ID") else { parent(); return; };
    let parts: Vec<_> = id.strip_prefix("snip-events-").unwrap().split('-').collect();
    assert!(parts.len() == 2 && parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())));
    let profile = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Temp/opencode").join(&id);
    assert!(!std::fs::symlink_metadata(&profile).unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read(profile.join("parent-owner")).unwrap(), id.as_bytes());
    let result = Arc::new(Mutex::new(None)); let results = result.clone();
    let done = Arc::new(AtomicBool::new(false)); let completed = done.clone();
    let app = tauri::Builder::default().any_thread()
        .register_uri_scheme_protocol("snip-events", |_, _| tauri::http::Response::builder().header("Content-Type", "text/html")
            .body(b"<!doctype html><html><head><title>owned event fixture</title></head><body></body></html>".to_vec()).unwrap())
        .setup(move |app| {
            let (send, receiver) = mpsc::channel(); let mut windows = Vec::new();
            for label in LABELS {
                let report = send.clone();
                // Official getCurrentWebviewWindow().listen target, installed
                // webviewWindow.js130-132. Production helper registration is
                // separately asserted behaviorally in snipping-product.test.ts.
                // Actual JS plugin transport, not a fake event bus/filter.
                let script = r#"(async()=>{try{const b=window.__TAURI_INTERNALS__;const events=[];
                    const listen=(event,callback)=>b.invoke('plugin:event|listen',{event,target:{kind:'WebviewWindow',label:__LABEL__},handler:b.transformCallback(callback)});
                    await listen('snip:context',e=>events.push({event:'context',payload:e.payload}));
                    await listen('snip:armed',e=>events.push({event:'armed',payload:e.payload}));
                    window.__snipEventDrain=phase=>{document.title='event-report:'+JSON.stringify({stage:'drain',phase,events:events.splice(0)});};
                    document.title='event-report:'+JSON.stringify({stage:'ready',bridge:typeof b.invoke==='function'});
                }catch{document.title='event-report:'+JSON.stringify({stage:'listener-error'});}})();"#.replace("__LABEL__", &serde_json::to_string(label)?);
                let window = WebviewWindowBuilder::new(app, label, WebviewUrl::CustomProtocol("snip-events://localhost/".parse()?))
                    .visible(false).focused(false).skip_taskbar(true).inner_size(32.0, 32.0).data_directory(profile.join(label))
                    .on_page_load(move |window, payload| { if payload.event() == PageLoadEvent::Finished { let _ = window.eval(&script); } })
                    .on_document_title_changed(move |window, title| {
                        if let Some(value) = title.strip_prefix("event-report:").filter(|v| v.len() <= 8192).and_then(|v| serde_json::from_str(v).ok()) {
                            let _ = report.send((window.label().to_owned(), value));
                        }
                    }).build()?;
                windows.push(window);
            }
            let registry = Arc::new(NativeRegistry::default());
            let port = NativeWindowPort::new(app.handle().clone(), registry.clone());
            let handle = app.handle().clone(); let watchdog = handle.clone(); let watchdog_done = completed.clone();
            thread::spawn(move || {
                let outcome = drive(port, registry, windows, receiver);
                *results.lock().unwrap() = Some(outcome);
                completed.store(true, Ordering::Release); handle.exit(0);
            });
            thread::spawn(move || {
                let end = Instant::now() + Duration::from_secs(35);
                while !watchdog_done.load(Ordering::Acquire) && Instant::now() < end { thread::sleep(Duration::from_millis(25)); }
                if !watchdog_done.load(Ordering::Acquire) { watchdog.exit(3); }
            });
            Ok(())
        }).build(tauri::generate_context!()).unwrap();
    let code = app.run_return(|_, _| {}); done.store(true, Ordering::Release);
    assert_eq!(code, 0, "native fixture exit/watchdog");
    let reports = result.lock().unwrap().take().expect("actual transport stages executed").expect("native transport setup/drain completed");
    assert_eq!(reports.len(), 8); println!("EVENT_BARRIERS_COMPLETED=8");
    let mut failures = Vec::new();
    for phase in 0..4 {
        for (i, label) in LABELS.iter().enumerate() {
            let events = reports[&(phase, label.to_string())]["events"].as_array().unwrap();
            println!("phase={phase}; receiver=m{i}; events={}", serde_json::to_string(events).unwrap());
            if i == phase % 2 {
                if events.len() != 1 { failures.push(format!("phase{phase} destination m{i} count={}", events.len())); continue; }
                assert_eq!(events[0]["event"], if phase < 2 { "context" } else { "armed" });
                assert_eq!(events[0]["payload"]["generation"], "7");
                assert_eq!(events[0]["payload"]["captureId"], token().capture_id);
                if phase < 2 { assert_eq!(events[0]["payload"]["monitorId"], format!("m{i}")); }
            } else if !events.is_empty() { failures.push(format!("phase{phase} non-destination m{i} received {} event(s)", events.len())); }
        }
    }
    assert!(failures.is_empty(), "actual cross-WebView delivery: {failures:?}");
}
