//! Thread-affine native feasibility windows. No AppBar, taskbar, or shell-state mutation.
use super::geometry::PhysicalRect;
use std::{cell::Cell, marker::PhantomData, mem::size_of, rc::Rc, time::{Duration, Instant}};
use windows::{core::w, Win32::{Foundation::*, Graphics::{Dwm::DwmFlush, Gdi::*}, UI::{Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture}, WindowsAndMessaging::*}}};

#[link(name = "user32")]
unsafe extern "system" {
    fn SetThreadDpiAwarenessContext(context: isize) -> isize;
    fn GetDpiForWindow(hwnd: HWND) -> u32;
    fn GetThreadDpiAwarenessContext() -> isize;
    fn AreDpiAwarenessContextsEqual(first: isize, second: isize) -> windows_core::BOOL;
}

#[derive(Debug)]
pub struct DpiObservation { pub context: isize, pub per_monitor_v2: bool }

pub fn dpi_observation() -> DpiObservation {
    let context = unsafe { GetThreadDpiAwarenessContext() };
    DpiObservation { context, per_monitor_v2: unsafe { AreDpiAwarenessContextsEqual(context, -4) }.as_bool() }
}

/// Numeric identity and class only: never window titles, paths, or content.
pub fn foreground_observation() -> String {
    let hwnd = foreground();
    let mut pid = 0u32;
    let mut class = [0u16; 128];
    let mut rect = RECT::default();
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let length = GetClassNameW(hwnd, &mut class).max(0) as usize;
        let bounds = GetWindowRect(hwnd, &mut rect).ok().map(|_| physical(rect));
        format!("hwnd={:p}; pid={pid}; class={}; bounds={bounds:?}", hwnd.0, String::from_utf16_lossy(&class[..length]))
    }
}

/// Restore the previous thread DPI context on exit; no display configuration changes.
pub struct PhysicalDpi { previous: isize, _thread: PhantomData<Rc<()>> }
impl PhysicalDpi {
    pub fn enter() -> Result<Self, &'static str> {
        let previous = unsafe { SetThreadDpiAwarenessContext(-4) }; // PER_MONITOR_AWARE_V2
        if previous == 0 { return Err("per-monitor v2 dpi unavailable"); }
        Ok(Self { previous, _thread: PhantomData })
    }
}
impl Drop for PhysicalDpi { fn drop(&mut self) { unsafe { SetThreadDpiAwarenessContext(self.previous); } } }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Monitor { pub rect: PhysicalRect, pub work: PhysicalRect }

pub fn monitors() -> Result<Vec<Monitor>, &'static str> {
    struct Enumeration { monitors: Vec<Monitor>, failed: bool }
    unsafe extern "system" fn callback(handle: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> windows_core::BOOL {
        // SAFETY: synchronous enumeration receives the live stack-owned context.
        let context = unsafe { &mut *(data.0 as *mut Enumeration) };
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if context.monitors.len() >= 32 || !unsafe { GetMonitorInfoW(handle, &mut info) }.as_bool() {
            context.failed = true;
            return windows_core::BOOL(0);
        }
        context.monitors.push(Monitor { rect: physical(info.rcMonitor), work: physical(info.rcWork) });
        windows_core::BOOL(1)
    }
    let mut context = Enumeration { monitors: Vec::with_capacity(32), failed: false };
    if !unsafe { EnumDisplayMonitors(None, None, Some(callback), LPARAM((&mut context as *mut Enumeration) as isize)) }.as_bool()
        || context.failed || context.monitors.is_empty() { return Err("monitor enumeration failed"); }
    Ok(context.monitors)
}

fn physical(rect: RECT) -> PhysicalRect { PhysicalRect { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom } }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind { SyntheticTarget, Overlay, Preview }

struct State {
    alive: Cell<bool>,
    kind: Kind,
    variant: Cell<u8>,
    down: Cell<u32>, up: Cell<u32>, moves: Cell<u32>, lost_capture: Cell<u32>,
    last_x: Cell<i32>, last_y: Cell<i32>, escape: Cell<bool>,
}

#[derive(Debug)]
pub struct PointerObservation { pub down: u32, pub up: u32, pub moves: u32, pub lost_capture: u32, pub last: (i32, i32), pub escape: bool }

/// Registration and HWND are destroyed on their creating thread. Not Send or Sync.
pub struct WindowClass { _thread: PhantomData<Rc<()>> }
impl WindowClass {
    pub fn register() -> Result<Self, &'static str> {
        let class = WNDCLASSW { lpfnWndProc: Some(procedure), lpszClassName: w!("JasonShellSnipFeasibility"), ..Default::default() };
        if unsafe { RegisterClassW(&class) } == 0 { return Err("window class registration failed"); }
        Ok(Self { _thread: PhantomData })
    }
}
impl Drop for WindowClass { fn drop(&mut self) { unsafe { let _ = UnregisterClassW(w!("JasonShellSnipFeasibility"), None); } } }

pub struct NativeWindow<'a> { hwnd: HWND, state: Box<State>, _class: &'a WindowClass, _thread: PhantomData<Rc<()>> }
impl<'a> NativeWindow<'a> {
    pub fn create(class: &'a WindowClass, rect: PhysicalRect, kind: Kind) -> Result<Self, &'static str> {
        let (width, height) = rect.dimensions().map_err(|_| "invalid window bounds")?;
        let state = Box::new(State { alive: Cell::new(true), kind, variant: Cell::new(0), down: Cell::new(0), up: Cell::new(0),
            moves: Cell::new(0), lost_capture: Cell::new(0), last_x: Cell::new(0), last_y: Cell::new(0), escape: Cell::new(false) });
        let style = WS_EX_TOOLWINDOW | WS_EX_TOPMOST | if kind == Kind::Preview { WS_EX_NOACTIVATE } else { WINDOW_EX_STYLE(0) };
        let hwnd = unsafe { CreateWindowExW(style, w!("JasonShellSnipFeasibility"), w!("Opt-in synthetic snip feasibility"), WS_POPUP,
            rect.left, rect.top, width as i32, height as i32, None, None, None, None) }.map_err(|_| "native window creation failed")?;
        let window = Self { hwnd, state, _class: class, _thread: PhantomData };
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*window.state as *const State) as isize); }
        if window.bounds()? != rect { return Err("native physical bounds mismatch"); }
        Ok(window)
    }
    fn owned(&self) -> Result<HWND, &'static str> {
        if !self.state.alive.get() { return Err("native window destroyed"); }
        let expected = (&*self.state as *const State) as isize;
        if unsafe { GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) } != expected { return Err("native window ownership lost"); }
        Ok(self.hwnd)
    }
    pub fn hwnd(&self) -> HWND { self.owned().unwrap_or_default() }
    pub fn scale(&self) -> Result<f64, &'static str> {
        self.owned()?;
        let dpi = unsafe { GetDpiForWindow(self.hwnd) };
        if dpi == 0 { Err("window dpi unavailable") } else { Ok(f64::from(dpi) / 96.0) }
    }
    pub fn bounds(&self) -> Result<PhysicalRect, &'static str> {
        self.owned()?;
        let mut rect = RECT::default();
        unsafe { GetWindowRect(self.hwnd, &mut rect) }.map_err(|_| "window bounds unavailable")?;
        Ok(physical(rect))
    }
    /// Exclusion is required, never best-effort. Must be applied before window visibility.
    pub fn exclude_from_capture(&self) -> Result<(), &'static str> {
        self.owned()?;
        unsafe { SetWindowDisplayAffinity(self.hwnd, WDA_EXCLUDEFROMCAPTURE) }.map_err(|_| "capture exclusion unavailable")
    }
    pub fn show(&self) -> Result<(), &'static str> {
        self.owned()?;
        unsafe { let _ = ShowWindow(self.hwnd, if self.state.kind == Kind::Preview { SW_SHOWNOACTIVATE } else { SW_SHOW }); }
        self.repaint()?;
        flush()
    }
    pub fn hide(&self) -> Result<(), &'static str> {
        self.owned()?;
        unsafe { let _ = ShowWindow(self.hwnd, SW_HIDE); }
        flush()
    }
    pub fn repaint(&self) -> Result<(), &'static str> {
        self.owned()?;
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
            if !UpdateWindow(self.hwnd).as_bool() { return Err("window repaint failed"); }
        }
        Ok(())
    }
    pub fn change_synthetic_target(&self) -> Result<(), &'static str> {
        self.state.variant.set(1);
        self.repaint()?;
        flush()
    }
    pub fn pointer_observation(&self) -> PointerObservation {
        PointerObservation { down: self.state.down.get(), up: self.state.up.get(), moves: self.state.moves.get(),
            lost_capture: self.state.lost_capture.get(), last: (self.state.last_x.get(), self.state.last_y.get()), escape: self.state.escape.get() }
    }
    pub fn owns_pointer_capture(&self) -> bool { self.owned().is_ok() && unsafe { GetCapture() == self.hwnd } }
}
impl Drop for NativeWindow<'_> {
    fn drop(&mut self) {
        if self.owned().is_err() { return; }
        unsafe {
            if GetCapture() == self.hwnd { let _ = ReleaseCapture(); }
            // State stays live until DestroyWindow completes its synchronous callbacks.
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

pub fn foreground() -> HWND { unsafe { GetForegroundWindow() } }
pub fn flush() -> Result<(), &'static str> { unsafe { DwmFlush() }.map_err(|_| "composition flush failed") }

/// Bounded pump for the standalone harness, not normal shell startup.
pub fn pump(duration: Duration) {
    let end = Instant::now() + duration;
    loop {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            unsafe { let _ = TranslateMessage(&message); DispatchMessageW(&message); }
        }
        if Instant::now() >= end { break; }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Independent synthetic grid oracle, never derived from captured desktop pixels.
pub fn synthetic_pixel(x: usize, y: usize, variant: u8) -> [u8; 4] {
    if variant != 0 { return [211, 19, 137, 255]; }
    [((x / 64) % 251) as u8, ((y / 64) % 251) as u8, 93, 255]
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let pointer = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const State;
    if pointer.is_null() { return unsafe { DefWindowProcW(hwnd, message, wp, lp) }; }
    // SAFETY: userdata points to the stable Box retained until DestroyWindow returns.
    let state = unsafe { &*pointer };
    match message {
        WM_NCDESTROY => {
            state.alive.set(false);
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0); DefWindowProcW(hwnd, message, wp, lp) }
        }
        WM_MOUSEACTIVATE if state.kind == Kind::Preview => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONDOWN => {
            state.down.set(state.down.get().saturating_add(1));
            if state.kind == Kind::Overlay { unsafe { SetCapture(hwnd); } }
            LRESULT(0)
        }
        WM_MOUSEMOVE | WM_LBUTTONUP => {
            state.last_x.set(lp.0 as i16 as i32);
            state.last_y.set((lp.0 >> 16) as i16 as i32);
            if message == WM_MOUSEMOVE { state.moves.set(state.moves.get().saturating_add(1)); }
            else {
                state.up.set(state.up.get().saturating_add(1));
                if unsafe { GetCapture() } == hwnd { unsafe { let _ = ReleaseCapture(); } }
            }
            LRESULT(0)
        }
        WM_CAPTURECHANGED => { state.lost_capture.set(state.lost_capture.get().saturating_add(1)); LRESULT(0) }
        WM_KEYDOWN if wp.0 == 27 => {
            state.escape.set(true);
            if unsafe { GetCapture() } == hwnd { unsafe { let _ = ReleaseCapture(); } }
            LRESULT(0)
        }
        WM_PAINT => {
            unsafe { paint(hwnd, state); }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wp, lp) },
    }
}

unsafe fn paint(hwnd: HWND, state: &State) {
    // BeginPaint/EndPaint are paired even if brush creation fails. Brushes are transient.
    let mut ps = PAINTSTRUCT::default();
    let dc = unsafe { BeginPaint(hwnd, &mut ps) };
    let mut client = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut client) }.is_ok() {
        let step = if state.kind == Kind::SyntheticTarget { 64 } else { client.right.max(client.bottom).max(1) };
        for y in (0..client.bottom).step_by(step as usize) {
            for x in (0..client.right).step_by(step as usize) {
                let rgba = if state.kind == Kind::SyntheticTarget { synthetic_pixel(x as usize, y as usize, state.variant.get()) }
                    else if state.kind == Kind::Preview { [40, 220, 70, 255] } else { [240, 80, 20, 255] };
                let brush = unsafe { CreateSolidBrush(COLORREF(u32::from(rgba[0]) | (u32::from(rgba[1]) << 8) | (u32::from(rgba[2]) << 16))) };
                if !brush.0.is_null() {
                    let rect = RECT { left: x, top: y, right: (x + step).min(client.right), bottom: (y + step).min(client.bottom) };
                    unsafe { FillRect(dc, &rect, brush); let _ = DeleteObject(brush.into()); }
                }
            }
        }
    }
    unsafe { let _ = EndPaint(hwnd, &ps); }
}
