//! Synthetic-only native/frontend integration entry. Never enters production main.
//! stdin: one bounded JSON {"op":"start|select|preview|save|inspect|shutdown"} per line.
//! stdout: bounded synthetic metadata JSON. Requires explicit clipboard opt-in.
#![cfg(windows)]
#[path = "../src/snipping/mod.rs"] mod snipping;
use snipping::{coordinator::*, geometry::PhysicalRect, native::PreparationBarrier};
use std::{io::{BufRead, Read, Write}, path::PathBuf, process::{Child, Command, Stdio}, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
use tauri::{Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

fn report(value: serde_json::Value) { let mut out = std::io::stdout().lock(); let _ = writeln!(out, "{value}"); let _ = out.flush(); }
struct OwnedOutput(PathBuf);
impl Drop for OwnedOutput { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
struct ChildOwner(Arc<Mutex<Child>>);
impl Drop for ChildOwner { fn drop(&mut self) { if let Ok(mut child) = self.0.lock() { let _ = child.kill(); } } }
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
    report(serde_json::json!({"stage":"target-ready","left":rect.left,"top":rect.top}));
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
    if args != ["--synthetic-clipboard-opt-in"] { report(serde_json::json!({"error":"explicit-synthetic-clipboard-opt-in-required"})); std::process::exit(2); }
    if let Err(_) = journey() { report(serde_json::json!({"error":"synthetic-journey-failed"})); std::process::exit(1); }
}
fn journey() -> Result<(), Box<dyn std::error::Error>> {
    let approved = PathBuf::from(std::env::var("LOCALAPPDATA")?).join("Temp/opencode");
    if !approved.is_dir() { return Err("approved synthetic output root missing".into()); }
    let fixture_id = format!("snip-journey-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos());
    let directory = approved.join(&fixture_id); std::fs::create_dir(&directory)?;
    let owned_output = OwnedOutput(directory.clone());
    let mut child = Command::new(std::env::current_exe()?).arg("--synthetic-target").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let pid = child.id(); let stdout = child.stdout.take().ok_or("target stdout unavailable")?;
    let child = Arc::new(Mutex::new(child)); let _owned_child = ChildOwner(child.clone());
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || { let mut line = String::new(); let result = std::io::BufReader::new(stdout).take(4096).read_line(&mut line); let _ = send.send(result.ok().and_then(|_| serde_json::from_str::<serde_json::Value>(&line).ok())); });
    let target = receive.recv_timeout(Duration::from_secs(5))?.ok_or("synthetic target did not initialize")?;
    let left = i32::try_from(target["left"].as_i64().ok_or("target bounds unavailable")?)?;
    let top = i32::try_from(target["top"].as_i64().ok_or("target bounds unavailable")?)?;
    let rect = PhysicalRect { left, top, right: left.checked_add(640).ok_or("synthetic bounds overflow")?, bottom: top.checked_add(400).ok_or("synthetic bounds overflow")? };
    let observer = snipping::foreground::ForegroundObserver::start_scoped(pid).map_err(|_| "scoped observer unavailable")?;
    let destination = directory.join("synthetic.png");
    let profile = directory.join("webview");
    let fixture = fixture_id.clone();
    let app = tauri::Builder::default().plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![snipping::runtime::start_snip, snipping::runtime::get_snip_context, snipping::runtime::get_snip_image,
            snipping::runtime::snip_ready, snipping::runtime::snip_begin_selection, snipping::runtime::complete_snip, snipping::runtime::cancel_snip,
            snipping::runtime::copy_snip, snipping::runtime::save_snip, snipping::runtime::dismiss_snip, snipping::runtime::snip_bar_ready])
        .setup(move |app| {
            // Extra emit authority exists ONLY in this standalone process and only
            // for its concrete synthetic surfaces. Production ACL is unchanged.
            app.add_capability(r#"{"identifier":"synthetic-journey-report","windows":["top-bar","snip-overlay-*","snip-preview-*"],"permissions":["core:event:allow-emit"]}"#)?;
            app.listen("synthetic:journey-report", |event| {
                if event.payload().len() <= 2048 { if let Ok(value) = serde_json::from_str::<serde_json::Value>(event.payload()) { report(serde_json::json!({"renderer":value})); } }
            });
            let bar = WebviewWindowBuilder::new(app, "top-bar", WebviewUrl::App("index.html".into())).data_directory(profile.clone()).visible(false).focused(false).decorations(false).skip_taskbar(true).inner_size(640.0, 52.0).build()?;
            let runtime = snipping::runtime::SnipRuntime::install_native_ports(app.handle().clone(), bar.clone(), observer,
                Arc::new(Picker(destination)), Some(profile), Box::new(move |prepare, release| Arc::new(SyntheticCapture { prepare, release,
                    monitor: MonitorSnapshot { id:"m0".into(), rect, scale:1.0, identity:0x53594e5448455449 } }))).map_err(|_| "native runtime initialization failed")?;
            app.manage(runtime);
            // Native show without activation: the immutable external target stays
            // foreground until renderer/user interaction with our own surfaces.
            unsafe { let _ = ShowWindow(HWND(bar.hwnd()?.0), SW_SHOWNOACTIVATE); }
            report(serde_json::json!({"stage":"ready","fixtureId":fixture,"source":"owned-offscreen-dib","synthetic":true,"selection":{"x":64,"y":48,"width":126,"height":75}}));
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let input = std::io::stdin(); let mut input = input.lock();
                loop {
                    let mut bytes = Vec::new();
                    match input.by_ref().take(1025).read_until(b'\n', &mut bytes) { Ok(0) | Err(_) => { handle.exit(0); break; }, _ => {} }
                    if bytes.len() > 1024 || bytes.last() != Some(&b'\n') { report(serde_json::json!({"error":"invalid-protocol-line"})); handle.exit(2); break; }
                    let op = serde_json::from_slice::<serde_json::Value>(&bytes).ok().and_then(|value| value["op"].as_str().map(str::to_owned));
                    if let Some(op) = op { if let Err(_) = action(&handle, &op) { report(serde_json::json!({"error":"action-unavailable","op":op})); } if op == "shutdown" { break; } }
                    else { report(serde_json::json!({"error":"invalid-operation"})); }
                }
            });
            let watchdog = app.handle().clone(); std::thread::spawn(move || { std::thread::sleep(Duration::from_secs(90)); watchdog.exit(3); });
            Ok(())
        }).build(tauri::generate_context!())?;
    app.run(|app, event| { if matches!(event, tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. }) {
        if let Some(runtime) = app.try_state::<Arc<snipping::runtime::SnipRuntime>>() { runtime.shutdown(); }
    } });
    drop(owned_output);
    Ok(())
}

fn action(app: &tauri::AppHandle, op: &str) -> Result<(), &'static str> {
    let runtime = app.state::<Arc<snipping::runtime::SnipRuntime>>();
    let snapshot = runtime.coordinator.inspect();
    if op == "shutdown" { app.exit(0); return Ok(()); }
    if op == "inspect" {
        let preview = snapshot.token.as_ref().and_then(|token| app.get_webview_window(&format!("snip-preview-{}", token.generation)));
        let foreground_is_preview = preview.as_ref().and_then(|window| window.hwnd().ok()).is_some_and(|hwnd| unsafe { GetForegroundWindow() }.0 == hwnd.0);
        let preview_visible = preview.as_ref().and_then(|window| window.hwnd().ok()).is_some_and(|hwnd| unsafe { IsWindowVisible(HWND(hwnd.0)) }.as_bool());
        report(serde_json::json!({"stage":"inspect","phase":format!("{:?}",snapshot.phase),"frameBytes":snapshot.frame_bytes,"imageBytes":snapshot.image_bytes,
            "windowCount":snapshot.window_ids.len(),"foregroundIsPreview":foreground_is_preview,"previewVisible":preview_visible,"publicationAttempts":runtime.publication_attempts(),
            "publication":snapshot.last_publication.map(|publication| publication.outcome.metadata())}));
        return Ok(());
    }
    let generation = snapshot.token.as_ref().map(|token| token.generation.as_str()).unwrap_or("0");
    let (label, body) = match op {
        "start" => ("top-bar".into(), "await wait(()=>document.querySelector('.top-bar')); const token=await invoke('start_snip'); report({op:'start',...token});"),
        "select" => (format!("snip-overlay-{generation}-m0"), "await wait(()=>document.querySelector('.snip-overlay .hint')); const c=await invoke('get_snip_context'); const a={generation:c.generation,captureId:c.captureId,monitorId:c.monitorId}; const r=await invoke('snip_begin_selection',a); if(!r.accepted)throw Error('not-accepted'); await invoke('complete_snip',{...a,rect:{x:64,y:48,width:126,height:75}}); report({op:'select',...a});"),
        "preview" => (format!("snip-preview-{generation}"), "await wait(()=>{const i=document.querySelector('.preview-frame img');return i&&i.naturalWidth===126&&getComputedStyle(i).visibility==='visible'}); const c=await invoke('get_snip_context'); report({op:'preview',width:c.width,height:c.height});"),
        "save" => (format!("snip-preview-{generation}"), "await wait(()=>{const b=document.querySelector('button[aria-label=Save]');return b&&!b.disabled});document.querySelector('button[aria-label=Save]').click();await wait(()=>document.querySelector('.preview-status')?.textContent==='Saved');report({op:'save',status:'saved'});"),
        "dismiss" => (format!("snip-preview-{generation}"), "const c=await invoke('get_snip_context');await invoke('dismiss_snip',{generation:c.generation,captureId:c.captureId});report({op:'dismiss'});"),
        // Negative native IPC check: same mounted preview has no start authority.
        "deny-start" => (format!("snip-preview-{generation}"), "try{await invoke('start_snip');report({op:'deny-start',denied:false})}catch{report({op:'deny-start',denied:true})}"),
        _ => return Err("unknown operation"),
    };
    let window = app.get_webview_window(&label).ok_or("owned surface unavailable")?;
    let script = format!(r#"(async()=>{{const invoke=window.__TAURI_INTERNALS__.invoke;const report=payload=>invoke('plugin:event|emit',{{event:'synthetic:journey-report',payload}});const wait=async check=>{{const end=performance.now()+8000;while(!check()){{if(performance.now()>end)throw Error('timeout');await new Promise(r=>setTimeout(r,16))}}}};try{{{body}}}catch{{report({{op:'{op}',error:'journey-action-failed'}})}}}})();"#);
    window.eval(&script).map_err(|_| "owned webview evaluation failed")
}
