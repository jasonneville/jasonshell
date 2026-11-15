// Test-owned real held-pointer support. No DOM dispatch/click substitutes, no
// completion handler, and no input permitted outside the exact registered helper.
struct HeldPointer {down:AtomicBool}
impl HeldPointer {
    fn release(&self){if self.down.swap(false,Ordering::AcqRel){let _=mouse_input(true);}}
}
impl Drop for HeldPointer {fn drop(&mut self){self.release();}}
fn begin_owned_drag(app:&tauri::AppHandle,request:&serde_json::Value)->Result<(),&'static str> {
    if app.try_state::<EscapeOnly>().is_none(){return Err("owned drag requires cancellation-only mode");}
    let held=app.state::<HeldPointer>();
    if held.down.load(Ordering::Acquire)||unsafe {GetAsyncKeyState(i32::from(VK_LBUTTON.0))}<0{return Err("mouse already held; no drag input");}
    let runtime=app.state::<Arc<snipping::runtime::SnipRuntime>>();
    let snapshot=runtime.coordinator.inspect();
    if snapshot.phase!=Some(Phase::Selecting){return Err("owned drag requires Selecting");}
    let token=snapshot.token.ok_or("owned drag token absent")?;
    let window=app.get_webview_window(&format!("snip-overlay-{}-m0",token.generation)).ok_or("owned overlay absent")?;
    runtime.registry.resolve(&window).map_err(|_|"owned overlay identity lost")?;
    let hwnd=HWND(window.hwnd().map_err(|_|"owned overlay HWND absent")?.0);
    let x=request["clientX"].as_f64().filter(|v|v.is_finite()).ok_or("invalid drag X")?;
    let y=request["clientY"].as_f64().filter(|v|v.is_finite()).ok_or("invalid drag Y")?;
    let scale=request["dpr"].as_f64().filter(|v|v.is_finite()&&*v>0.0&&*v<=8.0).ok_or("invalid drag scale")?;
    let mut client=RECT::default();unsafe {GetClientRect(hwnd,&mut client)}.map_err(|_|"owned overlay bounds absent")?;
    let mut start=POINT {x:(x*scale).round() as i32,y:(y*scale).round() as i32};
    let mut end=POINT {x:start.x+(30.0*scale).round() as i32,y:start.y+(20.0*scale).round() as i32};
    for point in [&start,&end] {
        if point.x<0||point.y<0||point.x>=client.right||point.y>=client.bottom{return Err("drag outside owned client");}
    }
    if !unsafe {ClientToScreen(hwnd,&mut start)}.as_bool()||!unsafe {ClientToScreen(hwnd,&mut end)}.as_bool(){return Err("owned drag mapping failed");}
    unsafe {SetCursorPos(start.x,start.y)}.map_err(|_|"owned drag position failed")?;
    if unsafe {GetAncestor(WindowFromPoint(start),GA_ROOT)}!=hwnd{return Err("owned drag occluded; no input");}
    let mut release=ReleaseMouse(false);
    if mouse_input(false)!=1{return Err("owned drag down rejected");}release.0=true;
    std::thread::sleep(Duration::from_millis(30));
    if unsafe {GetForegroundWindow()}!=hwnd{return Err("actual drag did not focus owned helper");}
    unsafe {SetCursorPos(end.x,end.y)}.map_err(|_|"owned drag move failed")?;
    held.down.store(true,Ordering::Release);release.0=false;
    report(serde_json::json!({"renderer":{"op":"overlay-pointer","foregroundIsOverlay":true,"buttonHeld":true}}));Ok(())
}
