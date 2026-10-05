//! Explicit opt-in, standalone synthetic native gate. Never launches the Tauri shell.
#[path = "../snipping/mod.rs"]
mod snipping;
#[cfg(windows)]
#[path = "../snipping/clipboard_driver.rs"]
mod clipboard_driver;

#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--internal-snip-clipboard-owner"] {
        if snipping::clipboard_process::helper_main().is_err() { std::process::exit(1); }
        return;
    }
    if args == ["--clipboard-test-protocol"] {
        println!("{{\"protocol\":\"snipping-clipboard-test-v1\",\"sideEffects\":false}}");
        return;
    }
    if args == ["--consent-synthetic-native", "--window-lifetime-check"] {
        if let Err(error) = window_lifetime_check() { eprintln!("window lifetime check failed: {error}"); std::process::exit(1); }
        return;
    }
    if args.iter().any(|arg| arg.starts_with("--clipboard-test") || arg == "--consent-synthetic-clipboard-test") {
        if let Err(error) = clipboard_driver::run(&args) { eprintln!("clipboard driver failed: {error}"); std::process::exit(1); }
        return;
    }
    if let Err(error) = run() { eprintln!("native snip gate FAILED: {error}"); std::process::exit(1); }
}

#[cfg(windows)]
fn window_lifetime_check() -> Result<(), &'static str> {
    use snipping::{geometry::PhysicalRect, windows::{Kind, NativeWindow, WindowClass}};
    use windows::Win32::UI::WindowsAndMessaging::{SendMessageW, WM_CLOSE};
    let class = WindowClass::register()?;
    let rect = PhysicalRect { left: 0, top: 0, right: 64, bottom: 64 };
    let original = NativeWindow::create(&class, rect, Kind::Preview)?;
    let handle = original.hwnd();
    // Hidden native HWND only: no activation, capture, clipboard or shell change.
    unsafe { SendMessageW(handle, WM_CLOSE, None, None); }
    if !original.hwnd().0.is_null() || original.bounds().is_ok() || original.show().is_ok()
        || original.hide().is_ok() || original.repaint().is_ok() || original.scale().is_ok()
        || original.exclude_from_capture().is_ok() || original.owns_pointer_capture() {
        return Err("destroyed window operation retained authority");
    }
    let replacement = NativeWindow::create(&class, rect, Kind::Preview)?;
    let reused = replacement.hwnd() == handle;
    drop(original);
    if replacement.bounds()? != rect { return Err("old wrapper drop damaged replacement"); }
    drop(replacement);
    println!("{{\"windowLifetime\":\"passed\",\"hiddenOnly\":true,\"handleReused\":{reused}}}");
    Ok(())
}
#[cfg(not(windows))]
fn main() { eprintln!("native snip gate requires Windows"); std::process::exit(1); }

#[cfg(windows)]
fn run() -> Result<(), String> {
    use snipping::{capture::freeze_monitors, geometry::*, windows::*};
    use std::time::Duration;
    if !std::env::args().any(|arg| arg == "--consent-synthetic-native") {
        return Err("requires --consent-synthetic-native; temporary full-monitor synthetic windows; no shell launch, saving, or upload".into());
    }
    let _dpi = PhysicalDpi::enter()?;
    let baseline = monitors()?;
    let baseline_dpi = dpi_observation();
    println!("baseline: {baseline:?}; dpi={baseline_dpi:?}; {}", foreground_observation());
    if baseline.iter().all(|monitor| monitor.work == monitor.rect) {
        println!("LIMITATION baseline has no reserved work area; this run cannot establish preservation of reserved AppBars");
    }
    let class = WindowClass::register()?;
    let original_foreground = foreground();
    println!("standalone synthetic gate; monitors={}; foreground={:p}; clipboard unchanged", baseline.len(), original_foreground.0);
    let mut targets = Vec::new();
    for monitor in &baseline {
        let target = NativeWindow::create(&class, monitor.rect, Kind::SyntheticTarget)?;
        println!("monitor={:?}; work={:?}; scale={}", monitor.rect, monitor.work, target.scale()?);
        target.show()?;
        targets.push(target);
    }
    pump(Duration::from_millis(150));
    println!("targets-visible: {:?}; dpi={:?}; {}", monitors()?, dpi_observation(), foreground_observation());
    let rects: Vec<_> = baseline.iter().map(|monitor| monitor.rect).collect();
    let frozen = freeze_monitors(&rects)?;
    for image in &frozen {
        let (width, _) = image.rect.dimensions().map_err(|e| format!("{e:?}"))?;
        // Compare all pixels privately. Only operation status/dimensions reach logs.
        for (index, pixel) in image.pixels.chunks_exact(4).enumerate() {
            if pixel != synthetic_pixel(index % width as usize, index / width as usize, 0) {
                return Err("synthetic full-monitor pixel oracle mismatch; no image logged or published".into());
            }
        }
    }
    println!("PASS full-monitor physical capture and synthetic grid oracle on every enumerated monitor");
    for target in &targets { target.change_synthetic_target()?; }
    let source = &frozen[0];
    let crop_rect = PhysicalRect { left: source.rect.left + 16, top: source.rect.top + 16,
        right: source.rect.left + 144, bottom: source.rect.top + 112 };
    let crop = crop_rgba(source.rect, crop_rect, &source.pixels, MAX_FROZEN_BYTES).map_err(|e| format!("crop: {e:?}"))?;
    for (index, pixel) in crop.pixels.chunks_exact(4).enumerate() {
        if pixel != synthetic_pixel(16 + index % 128, 16 + index / 128, 0) { return Err("frozen crop oracle mismatch".into()); }
    }
    println!("PASS frozen crop remains original grid after target changes; 128x96");
    drop(frozen);
    let monitor = baseline[0];
    let preview_rect = PhysicalRect { left: monitor.rect.left + 160, top: monitor.rect.top + 160,
        right: monitor.rect.left + 416, bottom: monitor.rect.top + 352 };
    let preview = NativeWindow::create(&class, preview_rect, Kind::Preview)?;
    preview.exclude_from_capture()?;
    let before = foreground();
    preview.show()?;
    pump(Duration::from_millis(80));
    if foreground() != before { return Err("preview show stole foreground focus".into()); }
    println!("PASS preview show leaves foreground HWND unchanged");
    let excluded = freeze_monitors(&rects)?;
    for image in &excluded {
        if !image.pixels.chunks_exact(4).all(|pixel| pixel == synthetic_pixel(0, 0, 1)) {
            return Err("previous preview exclusion synthetic oracle mismatch".into());
        }
    }
    drop(excluded);
    println!("PASS prior visible preview excluded from full-monitor capture");
    preview.hide()?;
    let overlay = NativeWindow::create(&class, monitor.rect, Kind::Overlay)?;
    overlay.exclude_from_capture()?;
    overlay.show()?;
    println!("overlay-visible: {:?}; dpi={:?}; {}", monitors()?, dpi_observation(), foreground_observation());
    if overlay.bounds()? != monitor.rect { return Err("overlay not full monitor".into()); }
    println!("PASS overlay full physical monitor bounds; native pointer click/drag requires --interactive");
    if std::env::args().any(|arg| arg == "--interactive") {
        println!("MANUAL: 12 seconds: click-drag overlay across its monitor boundary, release, then Escape");
        pump(Duration::from_secs(12));
        println!("overlay observations: {:?}; capture_held={}", overlay.pointer_observation(), overlay.owns_pointer_capture());
        overlay.hide()?;
        preview.show()?;
        let before = foreground();
        println!("MANUAL: 8 seconds: click green preview; foreground must stay unchanged");
        pump(Duration::from_secs(8));
        let observation = preview.pointer_observation();
        if observation.down == 0 || observation.up == 0 || foreground() != before { return Err("interactive preview click/focus gate not established".into()); }
        println!("PASS real preview click delivered down/up without foreground change");
    } else { println!("UNTESTED native pointer boundary and actual clickable-preview usability (interactive consent/session required)"); }
    if std::env::args().any(|arg| arg == "--replace-clipboard-synthetic") {
        let mut publisher = snipping::clipboard_process::ProcessOwner::spawn(&std::env::current_exe().map_err(|_| "helper executable unavailable")?)?;
        let retained = crop.pixels.capacity();
        let outcome = publisher.publish(crop, retained, Default::default());
        println!("synthetic clipboard outcome={:?}; independent reader still required", outcome);
        if !matches!(outcome, snipping::clipboard_process::Outcome::Committed { durable: true }) { return Err("clipboard publication unknown or durability not established".into()); }
    }
    drop(overlay);
    drop(preview);
    drop(targets);
    let final_monitors = monitors()?;
    let final_dpi = dpi_observation();
    println!("final-immediate: {final_monitors:?}; dpi={final_dpi:?}; {}", foreground_observation());
    // Diagnose external shell settling without concealing the immediate invariant failure.
    if final_monitors != baseline {
        for delay in [100, 250, 500, 1000] {
            pump(Duration::from_millis(delay));
            println!("post-cleanup-after-additional-{delay}ms: {:?}; dpi={:?}; {}", monitors()?, dpi_observation(), foreground_observation());
        }
        return Err("monitor/work-area baseline changed (immediate comparison retained; bounded settling observations are diagnostic only)".into());
    }
    if !baseline_dpi.per_monitor_v2 || !final_dpi.per_monitor_v2 || baseline_dpi.context != final_dpi.context {
        return Err("dpi context changed during native gate".into());
    }
    println!("PASS windows cleaned; work-area unchanged; no shell/AppBar/taskbar mutation");
    println!("UNTESTED shell/AppBar integration, actual invoke guards, external paste consumer, resource repeat/failpoints; native gate NOT complete");
    Ok(())
}
