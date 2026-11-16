//! Test-only OS/COM facade. The callback itself is inserted verbatim from vendor.
#![allow(non_snake_case, dead_code)]
use std::sync::Mutex;
#[derive(Clone, Copy, Default)] struct HWND(usize);
#[derive(Clone, Copy)] struct WPARAM(usize);
#[derive(Clone, Copy)] struct LPARAM(isize);
#[derive(Clone, Copy)] struct LRESULT(isize);
struct RECT { left: i32, top: i32, right: i32, bottom: i32 }
struct PhysicalSize<T> { width: T, height: T }
type Result<T> = std::result::Result<T, ()>;
type Callback = unsafe extern "system" fn(HWND,u32,WPARAM,LPARAM,usize,usize)->LRESULT;
const WM_NULL:u32=0; const WM_SIZE:u32=5; const WM_SETFOCUS:u32=7;
const WM_MOVE:u32=3; const WM_MOVING:u32=0x216; const WM_ENTERSIZEMOVE:u32=0x231;
const WM_DESTROY:u32=2; const PARENT_DESTROY_MESSAGE:u32=0x465;
const PARENT_SUBCLASS_ID:u32=0x464; const SIZE_MINIMIZED:u32=1;
const SWP_ASYNCWINDOWPOS:u32=0x4000; const SWP_NOACTIVATE:u32=0x10; const SWP_NOZORDER:u32=4;
const COREWEBVIEW2_MOVE_FOCUS_REASON_PROGRAMMATIC:u32=0;
struct State { handler:Option<(Callback,usize)>, trace:Vec<&'static str>, releases:usize, releasing:bool, reentrant:usize }
static STATE:Mutex<State>=Mutex::new(State{handler:None,trace:Vec::new(),releases:0,releasing:false,reentrant:0});
fn record(event:&'static str){ STATE.lock().unwrap().trace.push(event); }

// Dispatch reentry with WM_NULL: it exercises the ACTUAL callback's default arm
// without dereferencing an object currently inside Drop (which would itself be
// undefined behavior). Callback authority is the oracle, not an assumed crash.
fn dispatch(message:u32) {
    let handler={let mut state=STATE.lock().unwrap();let handler=state.handler;
        if handler.is_some() && state.releasing {state.reentrant+=1;}
        handler};
    if let Some((callback,data))=handler {unsafe{callback(HWND(0),message,WPARAM(0),LPARAM(0),PARENT_SUBCLASS_ID as usize,data);}}
}
struct ICoreWebView2Controller;
impl Drop for ICoreWebView2Controller {
    fn drop(&mut self){
        {let mut state=STATE.lock().unwrap();state.trace.push("release");state.releases+=1;state.releasing=true;}
        dispatch(WM_NULL);
        STATE.lock().unwrap().releasing=false;
    }
}
impl ICoreWebView2Controller {
    unsafe fn SetBounds(&self,_:RECT)->Result<()> {record("controller_call");Ok(())}
    unsafe fn ParentWindow(&self,_:&mut HWND)->Result<()> {record("controller_call");Err(())}
    unsafe fn MoveFocus(&self,_:u32)->Result<()> {record("controller_call");Ok(())}
    unsafe fn NotifyParentWindowPositionChanged(&self)->Result<()> {record("controller_call");Ok(())}
}
unsafe fn DefSubclassProc(_:HWND,_:u32,_:WPARAM,_:LPARAM)->LRESULT{record("default");LRESULT(91)}
unsafe fn SetWindowPos(_:HWND,_:Option<HWND>,_:i32,_:i32,_:i32,_:i32,_:u32)->Result<()>{record("position");Ok(())}
unsafe fn SetWindowSubclass(_:HWND,callback:Option<Callback>,_:usize,data:usize)->Result<()>{
    let mut state=STATE.lock().unwrap();state.trace.push("set");state.handler=callback.map(|callback|(callback,data));Ok(())
}
unsafe fn RemoveWindowSubclass(_:HWND,_:Option<Callback>,_:usize)->Result<()>{
    let mut state=STATE.lock().unwrap();state.trace.push("remove");state.handler=None;Ok(())
}
struct InnerWebView;
impl InnerWebView {
    fn parent_bounds(_:HWND)->Result<PhysicalSize<i32>>{record("bounds");Ok(PhysicalSize{width:10,height:10})}
    /* ACTUAL_CALLBACK */
}
fn assert_retirement(message:u32){
    let data=Box::into_raw(Box::new(ICoreWebView2Controller)) as usize;
    *STATE.lock().unwrap()=State{handler:Some((InnerWebView::parent_subclass_proc,data)),trace:Vec::new(),releases:0,releasing:false,reentrant:0};
    dispatch(message);
    let (releases,reentrant,removed,trace)={let state=STATE.lock().unwrap();
        (state.releases,state.reentrant,state.handler.is_none(),state.trace.clone())};
    assert_eq!(releases,1,"owned controller must release once");
    assert_eq!(reentrant,0,"COM release synchronously reentered still-authorized ACTUAL callback: {trace:?}");
    assert!(removed,"retirement must remove callback, not reset its data to null");
    assert!(trace.iter().position(|event|*event=="remove").unwrap()<trace.iter().position(|event|*event=="release").unwrap());
    // The facade dispatcher must no longer deliver these messages to callback
    // code. We do NOT directly invoke a retired raw callback with invalid data.
    for msg in [WM_SIZE,WM_SETFOCUS,WM_ENTERSIZEMOVE,WM_MOVE,WM_MOVING,WM_DESTROY,PARENT_DESTROY_MESSAGE]{dispatch(msg);}
    let state=STATE.lock().unwrap();assert_eq!(state.releases,1);
    assert!(!state.trace.iter().any(|event|matches!(*event,"controller_call"|"bounds"|"position")));
}
#[test] fn actual_callback_wm_destroy_removes_authority_before_release_reentry(){assert_retirement(WM_DESTROY);}
#[test] fn actual_callback_private_destroy_removes_authority_before_release_reentry(){assert_retirement(PARENT_DESTROY_MESSAGE);}
