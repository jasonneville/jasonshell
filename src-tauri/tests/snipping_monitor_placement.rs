//! Opt-in real Wry/WebView2 + production NativeWindowPort placement regression.
//! No production main, AppBars, hooks, foreground observer, capture or clipboard.
//! Standalone renderer barriers require --features tauri/custom-protocol so the
//! unchanged App URL loads packaged dist instead of an absent localhost dev server.
#![cfg(windows)]
#[path = "../src/snipping/mod.rs"]
mod snipping;
use snipping::{
    coordinator::*,
    native::{NativeCapturePort, NativeWindowPort},
    native_registry::NativeRegistry,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use tauri::{webview::PageLoadEvent, Listener, Manager};
use windows::Win32::{
    Foundation::*,
    Graphics::{
        Dwm::{DwmFlush, DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS},
        Gdi::{ClientToScreen, MonitorFromWindow, MONITOR_DEFAULTTOPRIMARY},
    },
    UI::WindowsAndMessaging::*,
};

#[link(name = "user32")]
unsafe extern "system" {
    fn GetDpiForWindow(hwnd: HWND) -> u32;
}

type Bounds = (i32, i32, i32, i32);
fn rectangle(rect: RECT) -> Bounds {
    (
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
    )
}

#[derive(Debug)]
struct Metrics {
    hwnd: usize,
    epoch: u64,
    outer: Bounds,
    client: Bounds,
    dwm: Bounds,
    dpi: u32,
    style: usize,
    exstyle: usize,
    visible: bool,
    affinity: u32,
    foreground: usize,
}

fn metrics(
    registry: &NativeRegistry,
    reference: &WindowRef,
    expected_hwnd: usize,
) -> Result<Metrics, String> {
    let _dpi = snipping::windows::PhysicalDpi::enter().map_err(str::to_owned)?;
    if !snipping::windows::dpi_observation().per_monitor_v2 {
        return Err("measurement is not PMv2".into());
    }
    let window = registry
        .entry(reference)
        .and_then(|entry| entry.window())
        .map_err(|e| format!("identity={e:?}"))?;
    let hwnd = HWND(window.hwnd().map_err(|_| "missing HWND")?.0);
    if hwnd.0 as usize != expected_hwnd {
        return Err("correlated HWND changed".into());
    }
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    if pid != std::process::id() {
        return Err("HWND PID changed".into());
    }
    let (mut outer, mut client, mut dwm) = (RECT::default(), RECT::default(), RECT::default());
    let mut origin = POINT::default();
    let mut affinity = 0;
    unsafe {
        GetWindowRect(hwnd, &mut outer).map_err(|_| "GetWindowRect failed")?;
        GetClientRect(hwnd, &mut client).map_err(|_| "GetClientRect failed")?;
        if !ClientToScreen(hwnd, &mut origin).as_bool() {
            return Err("ClientToScreen failed".into());
        }
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut dwm as *mut RECT).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
        .map_err(|_| "DWM frame bounds failed")?;
        GetWindowDisplayAffinity(hwnd, &mut affinity)
            .map_err(|_| "capture affinity unavailable")?;
        Ok(Metrics {
            hwnd: hwnd.0 as usize,
            epoch: reference.caller.window_id,
            outer: rectangle(outer),
            client: (
                origin.x,
                origin.y,
                client.right - client.left,
                client.bottom - client.top,
            ),
            dwm: rectangle(dwm),
            dpi: GetDpiForWindow(hwnd),
            style: GetWindowLongPtrW(hwnd, GWL_STYLE) as usize,
            exstyle: GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as usize,
            visible: IsWindowVisible(hwnd).as_bool(),
            affinity,
            foreground: GetForegroundWindow().0 as usize,
        })
    }
}

fn main_drain(app: &tauri::AppHandle) -> Result<(), String> {
    let (send, receive) = mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = send.send(());
    })
    .map_err(|_| "main drain dispatch failed")?;
    receive
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "main drain deadline".into())
}

fn renderer_drain(
    receiver: &mpsc::Receiver<(String, usize, u32)>,
    windows: &BTreeMap<String, usize>,
    phase: u32,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut seen = BTreeSet::new();
    while seen.len() != windows.len() {
        let (label, hwnd, actual_phase) = receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| format!("renderer phase{phase} deadline"))?;
        if actual_phase != phase || windows.get(&label) != Some(&hwnd) || !seen.insert(label) {
            return Err(format!(
                "renderer phase{phase} identity/order/duplicate failure"
            ));
        }
    }
    println!(
        "PLACEMENT_RENDERER_BARRIER phase={phase} windows={}; doubleAnimationFrame={}",
        seen.len(),
        phase > 0
    );
    Ok(())
}

fn check_sample(
    stage: &str,
    monitor: &MonitorSnapshot,
    sample: &Metrics,
    foreground: usize,
    visible: bool,
    failures: &mut Vec<String>,
) {
    let expected = (
        monitor.rect.left,
        monitor.rect.top,
        monitor.rect.right - monitor.rect.left,
        monitor.rect.bottom - monitor.rect.top,
    );
    println!("PLACEMENT_METRICS monitor={} stage={stage} hwnd={} epoch={} pmv2=true expected={expected:?} outer={:?} clientScreen={:?} dwm={:?} dpi={} style={:#x} exstyle={:#x} visible={} excluded={} foregroundUnchanged={}",
        monitor.id, sample.hwnd, sample.epoch, sample.outer, sample.client, sample.dwm, sample.dpi, sample.style, sample.exstyle, sample.visible,
        sample.affinity == WDA_EXCLUDEFROMCAPTURE.0, sample.foreground == foreground);
    if sample.outer != expected {
        failures.push(format!(
            "{} {stage}: exact outer mismatch expected={expected:?} actual={:?}",
            monitor.id, sample.outer
        ));
    }
    if sample.affinity != WDA_EXCLUDEFROMCAPTURE.0 {
        failures.push(format!("{} {stage}: exclusion changed", monitor.id));
    }
    if sample.visible != visible {
        failures.push(format!("{} {stage}: visibility mismatch", monitor.id));
    }
    if sample.foreground != foreground {
        failures.push(format!("{} {stage}: foreground changed", monitor.id));
    }
    if sample.dpi == 0 {
        failures.push(format!("{} {stage}: invalid DPI", monitor.id));
    }
    // Source contract: SnipOverlaySurface fixed100vw/100vh image object-fit:fill,
    // pointer client-minus-root -> snipSelection -> geometry logical*monitor.scale
    // plus monitor origin. Full-monitor mapping therefore requires exact CLIENT
    // screen coverage; exact outer/DWM rectangles are distinct, insufficient facts.
    if visible && sample.client != expected {
        failures.push(format!(
            "{} {stage}: client coverage mismatch expected={expected:?} actual={:?}",
            monitor.id, sample.client
        ));
    }
}

const NAME: &str =
    "secondary_monitor_overlay_preserves_physical_bounds_exclusion_and_nonactivation";
const SINGLE_NAME: &str =
    "primary_monitor_visible_overlay_preserves_full_client_coverage_and_nonactivation";

#[derive(Clone, Copy, Debug)]
enum PlacementProfile {
    LegacyDual,
    SinglePrimary,
}
impl PlacementProfile {
    fn name(self) -> &'static str {
        match self {
            Self::LegacyDual => NAME,
            Self::SinglePrimary => SINGLE_NAME,
        }
    }
}

fn check_profile_sample(
    profile: PlacementProfile,
    stage: &str,
    monitor: &MonitorSnapshot,
    sample: &Metrics,
    foreground: usize,
    visible: bool,
    failures: &mut Vec<String>,
) {
    // Legacy checks remain exactly check_sample's existing contract.
    check_sample(stage, monitor, sample, foreground, visible, failures);
    if matches!(profile, PlacementProfile::SinglePrimary) {
        let expected_dpi = (monitor.scale * 96.0).round() as u32;
        if sample.dpi != expected_dpi {
            failures.push(format!(
                "{} {stage}: DPI mismatch expected={expected_dpi} actual={}",
                monitor.id, sample.dpi
            ));
        }
    }
}
struct OwnedDirectory(PathBuf);
impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        for _ in 0..20 {
            if std::fs::remove_dir_all(&self.0).is_ok() || !self.0.exists() {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn parent(profile: PlacementProfile) {
    let id = format!(
        "snip-placement-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let path = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
        .join("Temp/opencode")
        .join(&id);
    std::fs::create_dir(&path).unwrap();
    let directory = OwnedDirectory(path.clone());
    std::fs::write(path.join("parent-owner"), id.as_bytes()).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            profile.name(),
            "--test-threads=1",
            "--nocapture",
        ])
        .env("JASONSHELL_SNIP_PLACEMENT_CHILD_ID", &id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(50);
    let mut timed_out = false;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            timed_out = true;
            child.kill().expect("terminate exact owned placement child");
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    drop(directory);
    assert!(
        !path.exists(),
        "parent-owned placement profile must be removed after native child exits"
    );
    eprintln!("placement child exited; exact parent-owned profile cleanup verified");
    assert!(
        output.stdout.len() + output.stderr.len() < 32768,
        "bounded synthetic placement diagnostics"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("{stdout}\n{stderr}");
    assert!(!timed_out, "native placement child timed out");
    assert!(
        output.status.success(),
        "actual production NativeWindowPort regression failed"
    );
    assert!(
        stdout.contains("PLACEMENT_EXECUTED=")
            && stdout.contains("PLACEMENT_RENDERER_BARRIER phase=2"),
        "must execute requested real monitor visible lifecycle stages, not skip"
    );
}

#[test]
#[ignore = "explicit JASONSHELL_SNIP_PLACEMENT_TESTS=1; real monitor WebView placement, no capture"]
fn secondary_monitor_overlay_preserves_physical_bounds_exclusion_and_nonactivation() {
    run_profile(PlacementProfile::LegacyDual);
}

#[test]
#[ignore = "explicit JASONSHELL_SNIP_PLACEMENT_TESTS=1; primary monitor visible WebView/client regression, no capture"]
fn primary_monitor_visible_overlay_preserves_full_client_coverage_and_nonactivation() {
    run_profile(PlacementProfile::SinglePrimary);
}

fn run_profile(placement: PlacementProfile) {
    assert_eq!(
        std::env::var("JASONSHELL_SNIP_PLACEMENT_TESTS").as_deref(),
        Ok("1")
    );
    if matches!(placement, PlacementProfile::SinglePrimary) {
        assert!(!tauri::is_dev(), "standalone visible renderer fixture requires --features tauri/custom-protocol; never launch a dev server as a fallback");
    }
    let Ok(id) = std::env::var("JASONSHELL_SNIP_PLACEMENT_CHILD_ID") else {
        parent(placement);
        return;
    };
    let parts: Vec<_> = id
        .strip_prefix("snip-placement-")
        .unwrap()
        .split('-')
        .collect();
    assert!(
        parts.len() == 2
            && parts
                .iter()
                .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    );
    let profile = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap())
        .join("Temp/opencode")
        .join(&id);
    assert!(!std::fs::symlink_metadata(&profile)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        std::fs::read(profile.join("parent-owner")).unwrap(),
        id.as_bytes()
    );
    // Only enumerate trusted physical monitor metadata; the capture barrier is a
    // panic tripwire and freeze() is never called by this fixture.
    let mut monitors =
        NativeCapturePort::new(Arc::new(|_| panic!("placement fixture must never capture")))
            .snapshot()
            .unwrap();
    println!("PLACEMENT_MONITORS count={}", monitors.len());
    for monitor in &monitors {
        println!(
            "PLACEMENT_MONITOR id={} scale={} rect={:?}",
            monitor.id, monitor.scale, monitor.rect
        );
    }
    if matches!(placement, PlacementProfile::LegacyDual) {
        assert!(
            monitors.len() >= 2,
            "requires two real monitors; unavailable hardware is not success"
        );
    } else {
        assert!(
            !monitors.is_empty(),
            "primary profile requires real monitor hardware"
        );
        // A null window with DEFAULTTOPRIMARY resolves the actual primary HMONITOR,
        // independently of ordering, screen dimensions or the measured HWND.
        let primary = unsafe { MonitorFromWindow(HWND::default(), MONITOR_DEFAULTTOPRIMARY) };
        assert!(!primary.0.is_null(), "primary HMONITOR unavailable");
        monitors.retain(|monitor| monitor.identity == primary.0 as usize as u64);
        assert_eq!(
            monitors.len(),
            1,
            "exactly one trusted primary snapshot required"
        );
    }
    println!(
        "PLACEMENT_PROFILE={placement:?} exercisedMonitors={}",
        monitors.len()
    );
    let expected_count = monitors.len();
    let failures = Arc::new(Mutex::new(Vec::<String>::new()));
    let observed = Arc::new(Mutex::new(BTreeMap::<String, (i32, i32, i32, i32)>::new()));
    let events = observed.clone();
    let finished = Arc::new(AtomicBool::new(false));
    let done = finished.clone();
    let results = failures.clone();
    let (report_send, report_receive) = mpsc::channel();
    let app = tauri::Builder::default().any_thread()
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished && webview.label().starts_with("snip-overlay-7-") {
                // Dedicated fixture-only markers. No production handler calls,
                // DOM content inspection, capture, input or state-changing window API.
                let Ok(hwnd) = webview.window().hwnd() else { return; };
                let script = r#"(()=>{const report=phase=>window.__TAURI_INTERNALS__.invoke('plugin:event|emit',{event:'placement-frame',payload:[__LABEL__,__HWND__,phase]});
                    window.__placementFrame=phase=>requestAnimationFrame(()=>requestAnimationFrame(()=>report(phase)));
                    report(0);})()"#.replace("__LABEL__", &serde_json::to_string(webview.label()).unwrap()).replace("__HWND__", &(hwnd.0 as usize).to_string());
                let _ = webview.eval(script);
            }
        })
        // NativeWindowPort still loads its unchanged App URL. No production
        // handlers are registered: frontend startup cannot execute shell actions.
        .on_window_event(move |window, event| {
            if window.label().starts_with("snip-overlay-") && matches!(event, tauri::WindowEvent::Resized(_) | tauri::WindowEvent::ScaleFactorChanged { .. }) {
                if let Ok(handle) = window.hwnd() {
                    let Ok(_dpi) = snipping::windows::PhysicalDpi::enter() else { return; };
                    let mut rect = RECT::default();
                    if unsafe { GetWindowRect(HWND(handle.0), &mut rect) }.is_ok() {
                        events.lock().unwrap().insert(window.label().into(), (rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top));
                    }
                }
            }
        })
        .setup(move |app| {
            // Fixture-local scalar acknowledgements only. Production app ACL does
            // not contain a placement_report command. Grant only the existing
            // core event emitter to these isolated test labels, never a production
            // handler/permission or a compiled source seam.
            app.add_capability(r#"{"identifier":"placement-drain-fixture-only","windows":["snip-overlay-7-*"],"permissions":["core:event:allow-emit"]}"#)?;
            app.listen("placement-frame", move |event| {
                if let Ok((label, hwnd, phase)) = serde_json::from_str::<(String, usize, u32)>(event.payload()) {
                    if label.starts_with("snip-overlay-7-") && phase <= 2 { let _ = report_send.send((label, hwnd, phase)); }
                }
            });
            let registry = Arc::new(NativeRegistry::default());
            let port = NativeWindowPort::new(app.handle().clone(), registry.clone()).with_data_directory(Some(profile));
            let handle = app.handle().clone();
            let watchdog = handle.clone();
            let watchdog_done = done.clone();
            thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(35);
                while !watchdog_done.load(Ordering::Acquire) && Instant::now() < deadline { thread::sleep(Duration::from_millis(25)); }
                if !watchdog_done.load(Ordering::Acquire) { watchdog.exit(3); }
            });
            thread::spawn(move || {
                let token = Token { generation: "7".into(), capture_id: "77777777777777777777777777777777".into() };
                let mut live = Vec::new();
                let before = unsafe { GetForegroundWindow() }.0 as usize;
                let mut owned_windows = BTreeMap::new();
                let (destroy_send, destroy_receive) = mpsc::channel();
                for monitor in &monitors {
                    let expected = (monitor.rect.left, monitor.rect.top, monitor.rect.right - monitor.rect.left, monitor.rect.bottom - monitor.rect.top);
                    match port.create_overlay(&token, monitor) {
                        Err(error) => results.lock().unwrap().push(format!("{} scale={} expected={expected:?}; create_overlay={error:?}", monitor.id, monitor.scale)),
                        Ok(reference) => {
                            let check = (|| -> Result<(), String> {
                                let window = registry.entry(&reference).map_err(|e| format!("registry={e:?}"))?.window().map_err(|e| format!("window={e:?}"))?;
                                let hwnd = HWND(window.hwnd().map_err(|_| "missing HWND")?.0);
                                let destroyed = destroy_send.clone();
                                let identity = hwnd.0 as usize;
                                window.on_window_event(move |event| {
                                    if matches!(event, tauri::WindowEvent::Destroyed) { let _ = destroyed.send(identity); }
                                });
                                let sample = metrics(&registry, &reference, hwnd.0 as usize)?;
                                check_profile_sample(placement, "hidden-created", monitor, &sample, before, false, &mut results.lock().unwrap());
                                owned_windows.insert(reference.caller.label.clone(), hwnd.0 as usize);
                                Ok(())
                            })();
                            if let Err(error) = check { results.lock().unwrap().push(format!("{}: {error}", monitor.id)); }
                            live.push(reference);
                        }
                    }
                }
                let lifecycle = (|| -> Result<(), String> {
                    if live.len() != expected_count || owned_windows.len() != expected_count { return Err("not all monitor overlays created".into()); }
                    renderer_drain(&report_receive, &owned_windows, 0)?;
                    main_drain(&handle)?;
                    for (monitor, reference) in monitors.iter().zip(&live) {
                        let sample = metrics(&registry, reference, owned_windows[&reference.caller.label])?;
                        check_profile_sample(placement, "hidden-renderer-drained", monitor, &sample, before, false, &mut results.lock().unwrap());
                    }
                    for reference in &live { port.show(reference).map_err(|e| format!("production show={e:?}"))?; }
                    for phase in 1..=2 {
                        for reference in &live {
                            let window = registry.entry(reference).and_then(|entry| entry.window()).map_err(|_| "visible identity failed")?;
                            window.eval(format!("window.__placementFrame({phase})")).map_err(|_| "frame drain eval failed")?;
                        }
                        renderer_drain(&report_receive, &owned_windows, phase)?;
                        main_drain(&handle)?;
                        unsafe { DwmFlush() }.map_err(|_| "visible compositor barrier failed")?;
                        main_drain(&handle)?;
                        println!("PLACEMENT_NATIVE_BARRIER phase={phase} mainDrains=2 dwmFlush=true");
                        for (monitor, reference) in monitors.iter().zip(&live) {
                            let sample = metrics(&registry, reference, owned_windows[&reference.caller.label])?;
                            check_profile_sample(placement, if phase == 1 { "visible-frame1" } else { "visible-frame2" }, monitor, &sample, before, true, &mut results.lock().unwrap());
                        }
                    }
                    Ok(())
                })();
                if let Err(error) = lifecycle { results.lock().unwrap().push(error); }
                // Queue exact owned cleanup before requesting normal event-loop exit.
                for reference in live { port.close(reference); }
                // destroy() queues another native message; a main-thread marker
                // alone cannot prove HWND destruction. Await exact native events.
                let mut destroyed = BTreeSet::new();
                let destruction_deadline = Instant::now() + Duration::from_secs(5);
                while destroyed.len() < owned_windows.len() {
                    match destroy_receive.recv_timeout(destruction_deadline.saturating_duration_since(Instant::now())) {
                        Ok(hwnd) if owned_windows.values().any(|expected| *expected == hwnd) && destroyed.insert(hwnd) => {},
                        _ => { results.lock().unwrap().push("owned native destruction acknowledgement failed".into()); break; }
                    }
                }
                if let Err(error) = main_drain(&handle) { results.lock().unwrap().push(error); }
                for hwnd in owned_windows.values() {
                    if unsafe { IsWindow(Some(HWND(*hwnd as *mut _))) }.as_bool() { results.lock().unwrap().push("owned HWND survived cleanup drain".into()); }
                }
                println!("PLACEMENT_CLEANUP ownedWindowsRemaining={}", owned_windows.values().filter(|hwnd| unsafe { IsWindow(Some(HWND(**hwnd as *mut _))) }.as_bool()).count());
                done.store(true, Ordering::Release);
                handle.exit(0);
            });
            Ok(())
        }).build(tauri::generate_context!()).unwrap();
    let code = app.run_return(|_, _| {});
    finished.store(true, Ordering::Release);
    assert_eq!(code, 0, "native placement watchdog/exit failure");
    println!("PLACEMENT_EXECUTED={expected_count}");
    let failures = failures.lock().unwrap();
    assert!(
        failures.is_empty(),
        "production placement failures={failures:?}; last native resize-event bounds={:?}",
        observed.lock().unwrap()
    );
}
