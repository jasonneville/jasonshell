//! Synthetic-only native/frontend integration entry. Never enters production main.
//! stdin: one bounded JSON {"op":"start|select|preview|save|inspect|shutdown"} per line.
//! stdout: bounded synthetic metadata JSON. Clipboard journey and cancellation-only
//! mode have separate explicit opt-ins; the latter omits publication handlers.
#![cfg(windows)]
#[path = "../src/snipping/mod.rs"] mod snipping;
use snipping::{coordinator::*, geometry::PhysicalRect, native::PreparationBarrier};
use std::{io::{BufRead, Read, Write}, path::PathBuf, process::{Child, Command, Stdio}, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
use tauri::{Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::{WindowsAndMessaging::*,Input::KeyboardAndMouse::{SendInput,GetAsyncKeyState,INPUT,INPUT_0,INPUT_KEYBOARD,INPUT_MOUSE,KEYBDINPUT,MOUSEINPUT,MOUSEEVENTF_LEFTDOWN,MOUSEEVENTF_LEFTUP,KEYEVENTF_KEYUP,VK_ESCAPE,VK_LBUTTON}}};

fn report(value: serde_json::Value) { let mut out = std::io::stdout().lock(); let _ = writeln!(out, "{value}"); let _ = out.flush(); }
struct OwnedOutput(PathBuf);
impl Drop for OwnedOutput { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
struct ChildOwner(Arc<Mutex<Child>>);
impl Drop for ChildOwner { fn drop(&mut self) { if let Ok(mut child) = self.0.lock() { let _ = child.kill(); let _=child.wait(); } } }
struct EscapeOnly { target_pid:u32, target_hwnd:isize }
fn escape_input(up:bool)->u32 {
    let event=INPUT {r#type:INPUT_KEYBOARD,Anonymous:INPUT_0 {ki:KEYBDINPUT {wVk:VK_ESCAPE,
        dwFlags:if up {KEYEVENTF_KEYUP}else{Default::default()},..Default::default()}}};
    unsafe {SendInput(&[event],std::mem::size_of::<INPUT>() as i32)}
}
struct ReleaseEscape(bool);
impl Drop for ReleaseEscape {fn drop(&mut self){if self.0 {let _=escape_input(true);}}}
fn mouse_input(up:bool)->u32 {
    let event=INPUT {r#type:INPUT_MOUSE,Anonymous:INPUT_0 {mi:MOUSEINPUT {dwFlags:if up {MOUSEEVENTF_LEFTUP}else{MOUSEEVENTF_LEFTDOWN},..Default::default()}}};
    unsafe {SendInput(&[event],std::mem::size_of::<INPUT>() as i32)}
}
struct ReleaseMouse(bool);
impl Drop for ReleaseMouse {fn drop(&mut self){if self.0 {let _=mouse_input(true);}}}
include!("fixtures/snip_held_pointer.rs");
fn focus_target(app:&tauri::AppHandle)->Result<HWND,&'static str> {
    let owned=app.try_state::<EscapeOnly>().ok_or("owned focus requires cancellation-only mode")?;
    let hwnd=HWND(owned.target_hwnd as *mut _);let mut pid=0;
    if unsafe {GetWindowThreadProcessId(hwnd,Some(&mut pid))}==0||pid!=owned.target_pid||!unsafe {IsWindow(Some(hwnd))}.as_bool(){return Err("owned target identity lost");}
    let deadline=Instant::now()+Duration::from_secs(2);
    while unsafe {GetForegroundWindow()}!=hwnd&&Instant::now()<deadline {unsafe {let _=SetForegroundWindow(hwnd);}std::thread::sleep(Duration::from_millis(20));}
    if unsafe {GetForegroundWindow()}!=hwnd{return Err("owned target not foreground; no injection");}Ok(hwnd)
}
fn topbar_pointer(app:&tauri::AppHandle,request:&serde_json::Value)->Result<(),&'static str> {
    if app.try_state::<EscapeOnly>().is_none(){return Err("owned pointer requires cancellation-only mode");}
    let bar=app.get_webview_window("top-bar").ok_or("owned bar absent")?;
    let runtime=app.state::<Arc<snipping::runtime::SnipRuntime>>();
    runtime.registry.resolve(&bar).map_err(|_|"owned bar identity lost")?;
    let hwnd=HWND(bar.hwnd().map_err(|_|"owned bar HWND absent")?.0);
    let x=request["clientX"].as_f64().filter(|v|v.is_finite()).ok_or("invalid button X")?;
    let y=request["clientY"].as_f64().filter(|v|v.is_finite()).ok_or("invalid button Y")?;
    let scale=request["dpr"].as_f64().filter(|v|v.is_finite()&&*v>0.0&&*v<=8.0).ok_or("invalid button scale")?;
    let mut client=RECT::default();unsafe {GetClientRect(hwnd,&mut client)}.map_err(|_|"owned bar bounds absent")?;
    let mut point=POINT {x:(x*scale).round() as i32,y:(y*scale).round() as i32};
    if point.x<0||point.y<0||point.x>=client.right||point.y>=client.bottom{return Err("button outside owned client");}
    if !unsafe {ClientToScreen(hwnd,&mut point)}.as_bool(){return Err("owned button screen mapping failed");}
    if unsafe {GetAsyncKeyState(i32::from(VK_LBUTTON.0))}<0{return Err("human mouse button already held");}
    let deadline=Instant::now()+Duration::from_secs(2);
    while unsafe {GetForegroundWindow()}!=hwnd&&Instant::now()<deadline {unsafe {let _=SetForegroundWindow(hwnd);}std::thread::sleep(Duration::from_millis(20));}
    if unsafe {GetForegroundWindow()}!=hwnd{return Err("owned topbar not foreground; no mouse input");}
    unsafe {SetCursorPos(point.x,point.y)}.map_err(|_|"owned button positioning failed")?;
    if unsafe {GetAncestor(WindowFromPoint(point),GA_ROOT)}!=hwnd{return Err("button occluded by nonowned window; no click");}
    let mut release=ReleaseMouse(false);
    if mouse_input(false)!=1{return Err("owned pointer down rejected");}release.0=true;
    std::thread::sleep(Duration::from_millis(20));
    if mouse_input(true)!=1{return Err("owned pointer up rejected");}release.0=false;
    report(serde_json::json!({"renderer":{"op":"topbar-pointer","foregroundIsTopbar":unsafe {GetForegroundWindow()}==hwnd}}));Ok(())
}
struct Picker(PathBuf);
impl PickerPort for Picker { fn pick_png(&self) -> Result<Option<PathBuf>, SaveCode> { Ok(Some(self.0.clone())) } }

/// Offscreen GDI source and destination only: neither DC comes from GetDC(None)
/// or a window. DIB contents are initialized entirely from synthetic coordinates.
struct Dc(HDC);
impl Drop for Dc { fn drop(&mut self) { unsafe { let _ = DeleteDC(self.0); } } }
struct Dib(HBITMAP);
impl Drop for Dib { fn drop(&mut self) { unsafe { let _ = DeleteObject(self.0.into()); } } }
struct Selected { dc: HDC, previous: HGDIOBJ }
impl Drop for Selected { fn drop(&mut self) { unsafe { let _ = SelectObject(self.dc, self.previous); } } }
fn dib_frame() -> Result<Vec<u8>, SnipError> {
    const WIDTH: usize = 640; const HEIGHT: usize = 400; const LEN: usize = WIDTH * HEIGHT * 4;
    unsafe {
        let source = Dc(CreateCompatibleDC(None)); let destination = Dc(CreateCompatibleDC(None));
        if source.0.0.is_null() || destination.0.0.is_null() { return Err(SnipError::CaptureFailed); }
        let info = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: WIDTH as i32, biHeight: -(HEIGHT as i32), biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() }, ..Default::default() };
        let mut input = std::ptr::null_mut(); let mut output = std::ptr::null_mut();
        let input_bitmap = Dib(CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut input, None, 0).map_err(|_| SnipError::CaptureFailed)?);
        let output_bitmap = Dib(CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut output, None, 0).map_err(|_| SnipError::CaptureFailed)?);
        if input.is_null() || output.is_null() { return Err(SnipError::CaptureFailed); }
        let bytes = std::slice::from_raw_parts_mut(input.cast::<u8>(), LEN);
        for y in 0..HEIGHT { for x in 0..WIDTH {
            let p = (y * WIDTH + x) * 4;
            // Independent runner oracle: RGBA = [x % 251, y % 251, 93, 255].
            bytes[p..p+4].copy_from_slice(&[93, (y % 251) as u8, (x % 251) as u8, 255]);
        } }
        std::slice::from_raw_parts_mut(output.cast::<u8>(), LEN).fill(0);
        let old_source = SelectObject(source.0, input_bitmap.0.into());
        if old_source.0.is_null() || old_source.0 as isize == -1 { return Err(SnipError::CaptureFailed); }
        let _source_selection = Selected { dc: source.0, previous: old_source };
        let old_destination = SelectObject(destination.0, output_bitmap.0.into());
        if old_destination.0.is_null() || old_destination.0 as isize == -1 { return Err(SnipError::CaptureFailed); }
        let _destination_selection = Selected { dc: destination.0, previous: old_destination };
        BitBlt(destination.0, 0, 0, WIDTH as i32, HEIGHT as i32, Some(source.0), 0, 0, SRCCOPY).map_err(|_| SnipError::CaptureFailed)?;
        if !GdiFlush().as_bool() { return Err(SnipError::CaptureFailed); }
        let mut rgba = Vec::new(); rgba.try_reserve_exact(LEN).map_err(|_| SnipError::CaptureFailed)?;
        for p in std::slice::from_raw_parts(output.cast::<u8>(), LEN).chunks_exact(4) { rgba.extend_from_slice(&[p[2], p[1], p[0], 255]); }
        Ok(rgba)
    }
}
struct SyntheticCapture { prepare: PreparationBarrier, release: Arc<dyn Fn() + Send + Sync>, monitor: MonitorSnapshot }
impl CapturePort for SyntheticCapture {
    fn snapshot(&self) -> Result<Vec<MonitorSnapshot>, SnipError> { Ok(vec![self.monitor.clone()]) }
    fn freeze(&self, monitors: &[MonitorSnapshot]) -> Result<Vec<Frame>, SnipError> {
        struct Release(Arc<dyn Fn() + Send + Sync>); impl Drop for Release { fn drop(&mut self) { (self.0)(); } }
        let _release = Release(self.release.clone());
        if monitors != [self.monitor.clone()] { return Err(SnipError::TopologyChanged); }
        (self.prepare)(Instant::now() + Duration::from_secs(5))?;
        let rgba = dib_frame()?;
        report(serde_json::json!({"stage":"synthetic-dib-frozen","width":640,"height":400,"source":"owned-offscreen-dib"}));
        Ok(vec![Frame { monitor: self.monitor.clone(), rgba }])
    }
}
fn target() -> Result<(), &'static str> {
    let _dpi = snipping::windows::PhysicalDpi::enter()?;
    let class = snipping::windows::WindowClass::register()?;
    let monitor = snipping::windows::monitors()?.into_iter().next().ok_or("no monitor")?;
    if monitor.work.right - monitor.work.left < 640 || monitor.work.bottom - monitor.work.top < 400 { return Err("synthetic fixture does not fit"); }
    let rect = PhysicalRect { left: monitor.work.left, top: monitor.work.top, right: monitor.work.left + 640, bottom: monitor.work.top + 400 };
    let window = snipping::windows::NativeWindow::create(&class, rect, snipping::windows::Kind::SyntheticTarget)?;
    window.show()?;
    unsafe { let _ = SetForegroundWindow(window.hwnd()); }
    report(serde_json::json!({"stage":"target-ready","left":rect.left,"top":rect.top,"workWidth":monitor.work.right-monitor.work.left,"scale":window.scale()?,"hwnd":(window.hwnd().0 as isize).to_string()}));
    let stopped = Arc::new(AtomicBool::new(false)); let eof = stopped.clone();
    std::thread::spawn(move || { let mut line = String::new(); let _ = std::io::stdin().read_line(&mut line); eof.store(true, Ordering::Release); });
    let deadline = Instant::now() + Duration::from_secs(120);
    while !stopped.load(Ordering::Acquire) && Instant::now() < deadline { snipping::windows::pump(Duration::from_millis(25)); }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--internal-snip-clipboard-owner"] { std::process::exit(if snipping::clipboard_process::helper_main().is_ok() { 0 } else { 1 }); }
    if args == ["--internal-snip-composition-flush"] { std::process::exit(if snipping::composition::helper() { 0 } else { 1 }); }
    if args == ["--synthetic-target"] { std::process::exit(if target().is_ok() { 0 } else { 1 }); }
    let escape_only=args==["--synthetic-escape-only"];
    if !escape_only && args != ["--synthetic-clipboard-opt-in"] { report(serde_json::json!({"error":"explicit-synthetic-mode-required"})); std::process::exit(2); }
    if let Err(_) = journey(escape_only) { report(serde_json::json!({"error":"synthetic-journey-failed"})); std::process::exit(1); }
}
fn journey(escape_only:bool) -> Result<(), Box<dyn std::error::Error>> {
    let approved = PathBuf::from(std::env::var("LOCALAPPDATA")?).join("Temp/opencode");
    if !approved.is_dir() { return Err("approved synthetic output root missing".into()); }
    // Parent allocates/guards this exact directory before spawn, so pre-READY
    // crashes cannot leave an undiscoverable WebView profile behind.
    let fixture_id = std::env::var("JASONSHELL_SNIP_JOURNEY_FIXTURE_ID")?;
    let parts: Vec<_> = fixture_id.strip_prefix("snip-journey-").ok_or("invalid fixture ID")?.split('-').collect();
    if parts.len() != 2 || parts.iter().any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit())) { return Err("invalid fixture ID".into()); }
    let directory = approved.join(&fixture_id);
    if std::fs::symlink_metadata(&directory)?.file_type().is_symlink()
        || std::fs::read(directory.join("parent-owner"))? != fixture_id.as_bytes() { return Err("unowned fixture directory".into()); }
    let owned_output = OwnedOutput(directory.clone());
    let mut child = Command::new(std::env::current_exe()?).arg("--synthetic-target").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let pid = child.id(); let stdout = child.stdout.take().ok_or("target stdout unavailable")?;
    let child = Arc::new(Mutex::new(child)); let _owned_child = ChildOwner(child.clone());
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || { let mut line = String::new(); let result = std::io::BufReader::new(stdout).take(4096).read_line(&mut line); let _ = send.send(result.ok().and_then(|_| serde_json::from_str::<serde_json::Value>(&line).ok())); });
    let target = receive.recv_timeout(Duration::from_secs(5))?.ok_or("synthetic target did not initialize")?;
    let left = i32::try_from(target["left"].as_i64().ok_or("target bounds unavailable")?)?;
    let top = i32::try_from(target["top"].as_i64().ok_or("target bounds unavailable")?)?;
    let target_hwnd=target["hwnd"].as_str().ok_or("target HWND unavailable")?.parse::<isize>()?;
    let bar_width=if escape_only {(target["workWidth"].as_f64().ok_or("target work width absent")?/target["scale"].as_f64().ok_or("target scale absent")?).min(1200.0)}else{640.0};
    let rect = PhysicalRect { left, top, right: left.checked_add(640).ok_or("synthetic bounds overflow")?, bottom: top.checked_add(400).ok_or("synthetic bounds overflow")? };
    let observer = snipping::foreground::ForegroundObserver::start_scoped(pid).map_err(|_| "scoped observer unavailable")?;
    let destination = directory.join("synthetic.png");
    let profile = directory.join("webview");
    let fixture = fixture_id.clone();
    let builder=tauri::Builder::default()
        // Match production: no raw DeviceEvent consumers; unregister Tao raw
        // input so focused WebViews do not bypass the passive Escape hook.
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .plugin(tauri_plugin_dialog::init());
    // Cancellation-only mode has NO completion/copy/save command handler. Even
    // an accidental real pointer-up cannot reach automatic clipboard publication.
    // Native clipboard helper may initialize, but receives no publish request.
    let builder=if escape_only {builder.invoke_handler(tauri::generate_handler![
        snipping::runtime::start_snip,snipping::runtime::get_snip_context,snipping::runtime::get_snip_image,
        snipping::runtime::snip_ready,snipping::runtime::snip_begin_selection,snipping::runtime::cancel_snip,snipping::runtime::snip_bar_ready])
    } else {builder.invoke_handler(tauri::generate_handler![snipping::runtime::start_snip, snipping::runtime::get_snip_context, snipping::runtime::get_snip_image,
            snipping::runtime::snip_ready, snipping::runtime::snip_begin_selection, snipping::runtime::complete_snip, snipping::runtime::cancel_snip,
            snipping::runtime::copy_snip, snipping::runtime::save_snip, snipping::runtime::dismiss_snip, snipping::runtime::snip_bar_ready])};
    let app = builder
        .setup(move |app| {
            if escape_only {app.manage(EscapeOnly {target_pid:pid,target_hwnd});app.manage(HeldPointer {down:AtomicBool::new(false)});}
            // Extra emit authority exists ONLY in this standalone process and only
            // for its concrete synthetic surfaces. Production ACL is unchanged.
            app.add_capability(r#"{"identifier":"synthetic-journey-report","windows":["top-bar","snip-overlay-*","snip-preview-*"],"permissions":["core:event:allow-emit"]}"#)?;
            app.listen("synthetic:journey-report", |event| {
                if event.payload().len() <= 2048 { if let Ok(value) = serde_json::from_str::<serde_json::Value>(event.payload()) { report(serde_json::json!({"renderer":value})); } }
            });
            let bar = WebviewWindowBuilder::new(app, "top-bar", WebviewUrl::App("index.html".into())).data_directory(profile.clone()).visible(false).focused(false).decorations(false).skip_taskbar(true).always_on_top(escape_only).inner_size(bar_width, 52.0).build()?;
            if escape_only {unsafe {SetWindowPos(HWND(bar.hwnd()?.0),Some(HWND_TOPMOST),rect.left,rect.top,0,0,SWP_NOSIZE|SWP_NOACTIVATE)}?;}
            let runtime = snipping::runtime::SnipRuntime::install_native_ports(app.handle().clone(), bar.clone(), observer,
                Arc::new(Picker(destination)), Some(profile), Box::new(move |prepare, release| Arc::new(SyntheticCapture { prepare, release,
                    monitor: MonitorSnapshot { id:"m0".into(), rect, scale:1.0, identity:0x53594e5448455449 } }))).map_err(|_| "native runtime initialization failed")?;
            app.manage(runtime);
            // Native show without activation: the immutable external target stays
            // foreground until renderer/user interaction with our own surfaces.
            unsafe { let _ = ShowWindow(HWND(bar.hwnd()?.0), SW_SHOWNOACTIVATE); }
            report(serde_json::json!({"stage":"ready","fixtureId":fixture,"processId":std::process::id(),"source":"owned-offscreen-dib","synthetic":true,"escapeOnly":escape_only,"publicationHandlersRegistered":!escape_only,"selection":{"x":64,"y":48,"width":126,"height":75}}));
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let input = std::io::stdin(); let mut input = input.lock();
                loop {
                    let mut bytes = Vec::new();
                    match input.by_ref().take(1025).read_until(b'\n', &mut bytes) { Ok(0) | Err(_) => { handle.exit(0); break; }, _ => {} }
                    if bytes.len() > 1024 || bytes.last() != Some(&b'\n') { report(serde_json::json!({"error":"invalid-protocol-line"})); handle.exit(2); break; }
                    let request=serde_json::from_slice::<serde_json::Value>(&bytes).ok();
                    let op=request.as_ref().and_then(|value|value["op"].as_str());
                    if let Some(op) = op { let result=if op=="topbar-pointer" {topbar_pointer(&handle,request.as_ref().unwrap())}else if op=="overlay-pointer" {begin_owned_drag(&handle,request.as_ref().unwrap())}else{action(&handle,op)}; if let Err(reason) = result { report(serde_json::json!({"error":"action-unavailable","op":op,"code":reason})); } if op == "shutdown" { break; } }
                    else { report(serde_json::json!({"error":"invalid-operation"})); }
                }
            });
            let watchdog = app.handle().clone(); std::thread::spawn(move || { std::thread::sleep(Duration::from_secs(90)); watchdog.exit(3); });
            Ok(())
        }).build(tauri::generate_context!())?;
    app.run(|app, event| { if matches!(event, tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }) {
        if let Some(held)=app.try_state::<HeldPointer>() {held.release();}
        if let Some(runtime) = app.try_state::<Arc<snipping::runtime::SnipRuntime>>() { runtime.shutdown(); }
    } });
    drop(owned_output);
    Ok(())
}

fn action(app: &tauri::AppHandle, op: &str) -> Result<(), &'static str> {
    if app.try_state::<EscapeOnly>().is_some() && !["shutdown","diagnostic-stop","mouse-release","inspect","bar-ready","bar-button-point","bar-keyboard-focus","overlay-drag-point","overlay-keyboard-focus","focus-target","start-native","start","begin-only","overlay-ready","deny-publication","cancel","escape-pulse","escape-repeat","escape-focused"].contains(&op) {return Err("operation forbidden in cancellation-only mode");}
    let runtime = app.state::<Arc<snipping::runtime::SnipRuntime>>();
    let snapshot = runtime.coordinator.inspect();
    if op=="mouse-release" {if let Some(held)=app.try_state::<HeldPointer>() {held.release();}report(serde_json::json!({"renderer":{"op":op}}));return Ok(());}
    if op == "shutdown" {if let Some(held)=app.try_state::<HeldPointer>() {held.release();}app.exit(0);return Ok(());}
    if op=="focus-target" {focus_target(app)?;report(serde_json::json!({"renderer":{"op":op,"foregroundIsTarget":true}}));return Ok(());}
    if op == "diagnostic-stop" {
        if app.try_state::<EscapeOnly>().is_none() {return Err("diagnostic stop requires cancellation-only mode");}
        // Test-owned terminal barrier: stop production runtime/listener while
        // keeping this isolated process alive for its deferred final snapshot.
        // No production shutdown path waits on diagnostic filesystem work.
        runtime.shutdown();report(serde_json::json!({"renderer":{"op":"diagnostic-stop"}}));return Ok(());
    }
    if op == "escape-pulse" || op == "escape-repeat" || op=="escape-focused" {
        let (hwnd,focus_scope)=if op=="escape-focused" {
            if app.try_state::<EscapeOnly>().is_none(){return Err("focused pulse requires cancellation-only mode");}
            let bar=app.get_webview_window("top-bar").ok_or("owned bar absent")?;
            runtime.registry.resolve(&bar).map_err(|_|"owned foreground bar identity lost")?;
            let bar_hwnd=HWND(bar.hwnd().map_err(|_|"owned bar HWND absent")?.0);
            let foreground=unsafe {GetForegroundWindow()};
            if foreground==bar_hwnd {(bar_hwnd,"top-bar")} else {
                let token=snapshot.token.as_ref().ok_or("owned focused selection absent")?;
                let overlay=app.get_webview_window(&format!("snip-overlay-{}-m0",token.generation)).ok_or("owned focused overlay absent")?;
                runtime.registry.resolve(&overlay).map_err(|_|"owned focused overlay identity lost")?;
                let hwnd=HWND(overlay.hwnd().map_err(|_|"owned focused overlay HWND absent")?.0);
                if foreground!=hwnd {return Err("owned app focus was not retained; no refocus/injection");}(hwnd,"overlay")
            }
        }else{(focus_target(app)?,"external-target")};
        // Do not take ownership of a human-held Escape. Release-state sampling
        // here is test safety/diagnostics only, never production cancellation.
        if unsafe {GetAsyncKeyState(i32::from(VK_ESCAPE.0))}<0 {return Err("Escape already held; no injection");}
        let mut release=ReleaseEscape(false);
        if escape_input(false)!=1 {return Err("owned target Escape down rejected");}release.0=true;
        std::thread::sleep(Duration::from_millis(20));
        let mut repeat_count=0;
        if op=="escape-repeat" {for _ in 0..3 {
            if unsafe {GetForegroundWindow()}!=hwnd {return Err("owned target lost focus; no repeat injection");}
            if escape_input(false)!=1 {return Err("owned target Escape repeat rejected");}
            repeat_count+=1;std::thread::sleep(Duration::from_millis(20));
        }}
        // In repeat cycles, make lifecycle ordering explicit: keep Escape held
        // until real helper WebViews have been destroyed, THEN inject key-up.
        // A failing cancellation still reaches key-up within this fixed bound.
        let helpers_left=||app.webview_windows().keys().filter(|label|label.starts_with("snip-")).count();
        if op=="escape-repeat" {
            let close_deadline=Instant::now()+Duration::from_secs(1);
            while helpers_left()!=0 && Instant::now()<close_deadline {std::thread::sleep(Duration::from_millis(5));}
        }
        let helpers_before_release=helpers_left();
        let phase_before_release=format!("{:?}",runtime.coordinator.inspect().phase);
        if escape_input(true)!=1 {return Err("owned target Escape up rejected");}release.0=false;
        let release_deadline=Instant::now()+Duration::from_millis(250);
        while unsafe {GetAsyncKeyState(i32::from(VK_ESCAPE.0))}<0 && Instant::now()<release_deadline {std::thread::sleep(Duration::from_millis(1));}
        let released=unsafe {GetAsyncKeyState(i32::from(VK_ESCAPE.0))}>=0;
        report(serde_json::json!({"renderer":{"op":op,"focusScope":focus_scope,"pulseMs":20*(1+repeat_count),"repeatCount":repeat_count,"nativeHelpersBeforeRelease":helpers_before_release,"phaseBeforeRelease":phase_before_release,"released":released,"foregroundUnchanged":unsafe {GetForegroundWindow()}==hwnd}}));
        return Ok(());
    }
    // Test-owned direct native dispatcher entry; NOT a real WM_HOTKEY test.
    if op == "start-native" {
        let ticket=runtime.reserve_native_start().map_err(|error| {report(serde_json::json!({"error":"native-reservation-failed","op":op,"code":format!("{error:?}")}));"native reservation failed"})?;
        let token=runtime.run_native_start(ticket).map_err(|error| {report(serde_json::json!({"error":"native-start-failed","op":op,"code":format!("{error:?}")}));"native start failed"})?;
        report(serde_json::json!({"renderer":{"op":"start-native","generation":token.generation,"captureId":token.capture_id}}));
        return Ok(());
    }
    if op == "inspect" {
        let preview = snapshot.token.as_ref().and_then(|token| app.get_webview_window(&format!("snip-preview-{}", token.generation)));
        let foreground_is_preview = preview.as_ref().and_then(|window| window.hwnd().ok()).is_some_and(|hwnd| unsafe { GetForegroundWindow() }.0 == hwnd.0);
        let foreground_is_snip = app.webview_windows().into_iter().filter(|(label,_)|label.starts_with("snip-")).any(|(_,window)|window.hwnd().ok().is_some_and(|hwnd|unsafe { GetForegroundWindow() }.0 == hwnd.0));
        let preview_visible = preview.as_ref().and_then(|window| window.hwnd().ok()).is_some_and(|hwnd| unsafe { IsWindowVisible(HWND(hwnd.0)) }.as_bool());
        let helpers=app.webview_windows().into_iter().filter(|(label,_)|label.starts_with("snip-")).collect::<Vec<_>>();
        let visible_helpers=helpers.iter().filter(|(_,window)|window.hwnd().ok().is_some_and(|hwnd|unsafe {IsWindowVisible(HWND(hwnd.0))}.as_bool())).count();
        let foreground_is_topbar=app.get_webview_window("top-bar").and_then(|w|w.hwnd().ok()).is_some_and(|hwnd|unsafe {GetForegroundWindow()}.0==hwnd.0);
        report(serde_json::json!({"stage":"inspect","processId":std::process::id(),"phase":format!("{:?}",snapshot.phase),"frameBytes":snapshot.frame_bytes,"imageBytes":snapshot.image_bytes,
            "windowCount":snapshot.window_ids.len(),"nativeHelperCount":helpers.len(),"visibleHelperCount":visible_helpers,"readinessCount":snapshot.readiness_count,"foregroundIsTopbar":foreground_is_topbar,"foregroundIsPreview":foreground_is_preview,"foregroundIsSnip":foreground_is_snip,"previewVisible":preview_visible,"publicationAttempts":runtime.publication_attempts(),
            "publication":snapshot.last_publication.map(|publication| publication.outcome.metadata())}));
        return Ok(());
    }
    let generation = snapshot.token.as_ref().map(|token| token.generation.as_str()).unwrap_or("0");
    let (label, body) = match op {
        "bar-ready" => ("top-bar".into(), "await wait(()=>document.querySelector('.top-bar'));await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));report({op:'bar-ready'});"),
        "bar-keyboard-focus" => ("top-bar".into(), "report({op:'bar-keyboard-focus',documentFocused:document.hasFocus()});"),
        "overlay-drag-point" => (format!("snip-overlay-{generation}-m0"), "const r=document.querySelector('.snip-overlay');r.addEventListener('pointerdown',e=>{window.__syntheticPointerId=e.pointerId;report({op:'overlay-ui-pointer',trusted:e.isTrusted})},{once:true});const b=r.getBoundingClientRect();report({op:'overlay-drag-point',clientX:b.width*0.3,clientY:b.height*0.4,dpr:devicePixelRatio});"),
        "overlay-keyboard-focus" => (format!("snip-overlay-{generation}-m0"), "await wait(()=>{const s=document.querySelector('.selection');return s&&s.getBoundingClientRect().width>1&&s.getBoundingClientRect().height>1});const r=document.querySelector('.snip-overlay');report({op:'overlay-keyboard-focus',documentFocused:document.hasFocus(),pointerCaptured:r.hasPointerCapture(window.__syntheticPointerId)});"),
        "bar-button-point" => ("top-bar".into(), "await wait(()=>{const b=document.querySelector('button[aria-label=\"Capture screen region\"]');return b&&!b.disabled});const b=document.querySelector('button[aria-label=\"Capture screen region\"]');b.addEventListener('click',e=>report({op:'topbar-ui-click',trusted:e.isTrusted,documentFocused:document.hasFocus()}),{once:true});const r=b.getBoundingClientRect();report({op:'bar-button-point',clientX:r.x+r.width/2,clientY:r.y+r.height/2,dpr:devicePixelRatio});"),
        "start" => ("top-bar".into(), "await wait(()=>document.querySelector('.top-bar')); const token=await invoke('start_snip'); report({op:'start',...token});"),
        "select" => (format!("snip-overlay-{generation}-m0"), "await wait(()=>document.querySelector('.snip-overlay .hint')); const c=await invoke('get_snip_context'); const a={generation:c.generation,captureId:c.captureId,monitorId:c.monitorId}; const r=await invoke('snip_begin_selection',a); if(!r.accepted)throw Error('not-accepted'); await invoke('complete_snip',{...a,rect:{x:64,y:48,width:126,height:75}}); report({op:'select',...a});"),
        "begin-only" => (format!("snip-overlay-{generation}-m0"), "await wait(()=>document.querySelector('.snip-overlay .hint')); const c=await invoke('get_snip_context'); const a={generation:c.generation,captureId:c.captureId,monitorId:c.monitorId}; const r=await invoke('snip_begin_selection',a); if(!r.accepted)throw Error('not-accepted'); report({op:'begin-only',...a});"),
        "overlay-ready" => (format!("snip-overlay-{generation}-m0"), "await wait(()=>{const i=document.querySelector('.snip-overlay img');return document.querySelector('.snip-overlay .hint')&&i&&i.naturalWidth===640&&i.naturalHeight===400});const c=await invoke('get_snip_context');report({op:'overlay-ready',...c});"),
        "deny-publication" => (format!("snip-overlay-{generation}-m0"), "const c=await invoke('get_snip_context');let denied=0;for(const cmd of ['complete_snip','copy_snip','save_snip']){try{await invoke(cmd,{generation:c.generation,captureId:c.captureId,monitorId:c.monitorId,rect:{x:0,y:0,width:2,height:2}})}catch{denied++}}report({op:'deny-publication',denied});"),
        "cancel" => (format!("snip-overlay-{generation}-m0"), "const c=await invoke('get_snip_context');await invoke('cancel_snip',{generation:c.generation,captureId:c.captureId});report({op:'cancel'});"),
        "preview" => (format!("snip-preview-{generation}"), "await wait(()=>{const i=document.querySelector('.preview-frame img');return i&&i.naturalWidth===126&&getComputedStyle(i).visibility==='visible'}); const c=await invoke('get_snip_context'); report({op:'preview',width:c.width,height:c.height});"),
        "save" => (format!("snip-preview-{generation}"), "await wait(()=>{const b=document.querySelector('button[aria-label=Save]');return b&&!b.disabled});document.querySelector('button[aria-label=Save]').click();await wait(()=>document.querySelector('.preview-status')?.textContent==='Saved');report({op:'save',status:'saved'});"),
        "dismiss" => (format!("snip-preview-{generation}"), "const c=await invoke('get_snip_context');await invoke('dismiss_snip',{generation:c.generation,captureId:c.captureId});report({op:'dismiss'});"),
        // Negative native IPC check: same mounted preview has no start authority.
        "deny-start" => (format!("snip-preview-{generation}"), "try{await invoke('start_snip');report({op:'deny-start',denied:false})}catch{report({op:'deny-start',denied:true})}"),
        _ => return Err("unknown operation"),
    };
    let window = app.get_webview_window(&label).ok_or("owned surface unavailable")?;
    // Only allowlisted error codes leave the renderer. Never dump OS/profile
    // paths or arbitrary exception messages to diagnose a synthetic failure.
    let script = format!(r#"(async()=>{{const invoke=window.__TAURI_INTERNALS__.invoke;const report=payload=>invoke('plugin:event|emit',{{event:'synthetic:journey-report',payload}});const wait=async check=>{{const end=performance.now()+8000;while(!check()){{if(performance.now()>end)throw Error('timeout');await new Promise(r=>setTimeout(r,16))}}}};try{{{body}}}catch(error){{const value=typeof error==='string'?error:error?.message;const known=['inactive','unauthorized','stale','busy','invalid-selection','topology-changed','capture-failed','readiness-timeout','window-failed','image-failed','timeout','not-accepted'];const code=known.includes(value)?value:typeof value==='string'&&/not allowed|permission|forbidden/i.test(value)?'acl-denied':'unclassified';report({{op:'{op}',error:'journey-action-failed',code}})}}}})();"#);
    window.eval(&script).map_err(|_| "owned webview evaluation failed")
}
