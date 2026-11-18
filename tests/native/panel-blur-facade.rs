//! Controlled platform ports around actual production Rust source, not a copied blur reducer.
#![allow(dead_code, unused_imports)]
use std::{sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}}, time::Duration, future::Future, task::{Context, Poll, Wake, Waker}};
use serde::Serialize;
/* ACTUAL_SYSTEM_INFO_MODULE */
use system_info::{RuntimeInner, ProcessManagerContext, ProcessManagerTab, Lane};
const PROCESS_MANAGER_LABEL: &str = "process-manager";
const PROCESS_MANAGER_CLOSED_EVENT: &str = "process-manager:closed";
mod shell_windows { pub const PROCESS_MANAGER_LABEL: &str = "process-manager"; pub const SETTINGS_PANEL_LABEL: &str = "settings-panel"; }
// This fixture exercises the Process Manager blur branch only. Settings exit
// retention is covered by the dedicated actual coordinator/window fixture.
mod graceful_exit { pub fn holds_settings(_: &super::Window) -> bool { false } }
enum WindowEvent { Focused(bool), Destroyed }
struct SystemInfoRuntime { inner: Mutex<RuntimeInner> }
type Callback = Box<dyn FnOnce() + Send>;
#[derive(Clone)]
struct Window { shared: Arc<Shared> }
struct Shared {
    focused: Mutex<bool>, visible: Mutex<bool>, queue: Mutex<Vec<Callback>>,
    on_main: Mutex<bool>, runtime: SystemInfoRuntime, hides: AtomicU64,
}
impl Window {
    fn label(&self) -> &str { PROCESS_MANAGER_LABEL }
    fn app_handle(&self) -> &Self { self }
    fn state<T>(&self) -> &SystemInfoRuntime { &self.shared.runtime }
    fn try_state<T>(&self) -> Option<Arc<snipping::runtime::SnipRuntime>> { None }
    fn hwnd(&self) -> Result<Hwnd, ()> { Ok(Hwnd(1)) }
    fn get_webview_window(&self, _: &str) -> Option<Self> { Some(self.clone()) }
    fn emit<T>(&self, _: &str, _: T) -> Result<(), ()> { Ok(()) }
    fn is_visible(&self) -> Result<bool, ()> { Ok(*self.shared.visible.lock().unwrap()) }
    fn is_focused(&self) -> Result<bool, ()> { Ok(*self.shared.focused.lock().unwrap()) }
    fn run_on_main_thread(&self, callback: impl FnOnce() + Send + 'static) -> Result<(), ()> {
        self.shared.queue.lock().unwrap().push(Box::new(callback)); Ok(())
    }
    fn hide(&self) -> Result<(), ()> {
        if *self.shared.on_main.lock().unwrap() {
            *self.shared.visible.lock().unwrap() = false;
            self.shared.hides.fetch_add(1, Ordering::SeqCst);
        } else {
            // Tao/Tauri's off-thread window effects are delivered later by the UI loop.
            let panel = self.clone(); self.run_on_main_thread(move || { let _ = panel.hide(); })?;
        }
        Ok(())
    }
    fn deliver(&self) {
        let callbacks = std::mem::take(&mut *self.shared.queue.lock().unwrap());
        *self.shared.on_main.lock().unwrap() = true;
        for callback in callbacks { callback(); }
        *self.shared.on_main.lock().unwrap() = false;
    }
}
struct Hwnd(isize);
type AppHandle = Window;
mod snipping { pub mod runtime { pub struct SnipRuntime; impl SnipRuntime { pub fn holds_focus_loss(&self, _: isize) -> bool { false } } } }
fn block_on<F: Future>(future: F) -> F::Output {
    struct Noop; impl Wake for Noop { fn wake(self: Arc<Self>) {} }
    let waker = Waker::from(Arc::new(Noop)); let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    match future.as_mut().poll(&mut context) { Poll::Ready(value) => value, Poll::Pending => panic!("facade futures must be ready") }
}
mod tauri { pub mod async_runtime {
    pub fn spawn<F: std::future::Future>(future: F) { super::super::block_on(future); }
    pub async fn spawn_blocking<F: FnOnce() -> R, R>(work: F) -> Result<R, ()> { Ok(work()) }
} }
mod process_manager {
    use super::*;
    /* ACTUAL_RETIRE_FUNCTION */
    /* ACTUAL_SUSPEND_FUNCTION */
}
fn now() -> u64 { 123 }
fn scheduler_tick(window: &Window) -> Vec<system_info::WorkToken> {
    let inner = &window.shared.runtime.inner;
    let visible = window.is_visible().unwrap(); let focused = window.is_focused().unwrap();
    let ticks = 100; let mut tokens = Vec::new();
    loop {
        /* ACTUAL_SCHEDULER_TICK */
        break;
    }
    tokens
}
fn collector_inactive(window: &Window) {
    let mut state = window.shared.runtime.inner.lock().unwrap();
    let active = false;
    /* ACTUAL_COLLECTOR_INACTIVE */
}
fn dispatch(window: &Window, event: WindowEvent, revision: &'static AtomicU64, expected: u64) {
    let panel_blur_revision = Some(revision); let blur_revision = Some(expected);
    /* ACTUAL_DEFERRED_BRANCH */
}
fn fixture() -> (Window, &'static AtomicU64) {
    let mut runtime = RuntimeInner::default();
    let opened = runtime.coordinator.open(Some(101)).unwrap();
    runtime.coordinator.activate(opened.open_generation, opened.tab_revision + 1, ProcessManagerTab::SystemInfo, true, true).unwrap();
    (Window { shared: Arc::new(Shared {
        focused: Mutex::new(false), visible: Mutex::new(true), queue: Mutex::new(Vec::new()),
        on_main: Mutex::new(false), runtime: SystemInfoRuntime { inner: Mutex::new(runtime) }, hides: AtomicU64::new(0),
    }) }, Box::leak(Box::new(AtomicU64::new(1))))
}
#[test]
fn ordinary_unfocused_callback_hides() {
    let (window, revision) = fixture(); dispatch(&window, WindowEvent::Focused(false), revision, 1);
    assert!(window.shared.runtime.inner.lock().unwrap().coordinator.context().visible, "blur alone must retain context");
    assert_eq!(window.shared.queue.lock().unwrap().len(), 1, "exactly one guarded UI callback");
    window.deliver(); assert!(!window.is_visible().unwrap()); assert_eq!(window.shared.hides.load(Ordering::SeqCst), 1);
    assert!(!window.shared.runtime.inner.lock().unwrap().coordinator.context().visible, "actual hide must retire context");
}
#[test]
fn refocus_after_queue_before_delivery_cannot_hide() {
    let (window, revision) = fixture(); dispatch(&window, WindowEvent::Focused(false), revision, 1);
    assert!(!window.shared.queue.lock().unwrap().is_empty(), "must exercise queued UI delivery");
    *window.shared.focused.lock().unwrap() = true; revision.fetch_add(1, Ordering::SeqCst);
    window.deliver(); assert!(window.is_visible().unwrap(), "stale queued blur hid a refocused visible panel");
    assert_eq!(window.shared.hides.load(Ordering::SeqCst), 0);
}
#[test]
fn replacement_revision_after_queue_cannot_hide_reopened_panel() {
    let (window, revision) = fixture(); dispatch(&window, WindowEvent::Focused(false), revision, 1);
    revision.fetch_add(2, Ordering::SeqCst);
    window.shared.runtime.inner.lock().unwrap().coordinator.open(None).unwrap();
    window.deliver(); assert!(window.is_visible().unwrap(), "stale queued blur hid a reopened generation");
}
#[test]
fn pm_blur_refocus_retains_working_context_and_rejects_stale_work() {
    let (window, revision) = fixture();
    let old = window.shared.runtime.inner.lock().unwrap().coordinator.context();
    let work = window.shared.runtime.inner.lock().unwrap().coordinator.admit(Lane::Live, old.open_generation, old.tab_revision, 0, true, true).unwrap();
    dispatch(&window, WindowEvent::Focused(false), revision, 1);
    *window.shared.focused.lock().unwrap() = true; revision.fetch_add(1, Ordering::SeqCst);
    dispatch(&window, WindowEvent::Focused(true), revision, 2);
    let mut runtime = window.shared.runtime.inner.lock().unwrap();
    assert!(!runtime.coordinator.complete(work, 1), "blur must reject pre-blur work");
    let retained = runtime.coordinator.context();
    assert!(retained.visible, "refocused visually retained panel has a closed coordinator");
    assert_eq!(retained, old, "short blur must preserve the renderer's context/tab/PID");
    let processes = runtime.coordinator.activate(retained.open_generation, retained.tab_revision + 1, ProcessManagerTab::Processes, true, true).expect("refocus must restore working tab IPC");
    let info = runtime.coordinator.activate(processes.open_generation, processes.tab_revision + 1, ProcessManagerTab::SystemInfo, true, true).unwrap();
    assert!(runtime.coordinator.admit(Lane::Live, info.open_generation, info.tab_revision, 10, true, true).is_some(), "fresh System Info refresh must work");
}
#[test]
fn actual_scheduler_refocus_rewarms_same_context_and_admits_new_tokens() {
    let (window, revision) = fixture();
    *window.shared.focused.lock().unwrap() = true;
    let old_context = window.shared.runtime.inner.lock().unwrap().coordinator.context();
    let before = scheduler_tick(&window); assert_eq!(before.len(), 3);
    assert!(window.shared.runtime.inner.lock().unwrap().cache.is_some());
    *window.shared.focused.lock().unwrap() = false;
    dispatch(&window, WindowEvent::Focused(false), revision, 1);
    assert!(window.shared.runtime.inner.lock().unwrap().cache.is_none());
    assert!(scheduler_tick(&window).is_empty(), "unfocused scheduler must admit no work");
    *window.shared.focused.lock().unwrap() = true; revision.fetch_add(1, Ordering::SeqCst);
    let resumed = scheduler_tick(&window); assert_eq!(resumed.len(), 3);
    let mut runtime = window.shared.runtime.inner.lock().unwrap();
    assert_eq!(runtime.coordinator.context(), old_context);
    let cache = runtime.read_snapshot(old_context.open_generation, old_context.tab_revision, None, true, true).unwrap();
    assert!(cache.is_some(), "actual resumed scheduler must warm readable cache");
    for token in before { assert!(!runtime.coordinator.complete(token, 100), "pre-blur worker must remain rejected after resume"); }
    for token in resumed { assert!(runtime.coordinator.complete(token, 100), "stale completion must not clear resumed work"); }
    drop(runtime); window.deliver();
    assert!(window.is_visible().unwrap(), "refocused resumed panel must survive delayed callback");
}
#[test]
fn actual_collector_inactive_path_suspends_without_silently_closing_context() {
    let (window, _) = fixture();
    *window.shared.focused.lock().unwrap() = true;
    let context = window.shared.runtime.inner.lock().unwrap().coordinator.context();
    let old = scheduler_tick(&window); assert_eq!(old.len(), 3);
    collector_inactive(&window);
    let mut runtime = window.shared.runtime.inner.lock().unwrap();
    assert_eq!(runtime.coordinator.context(), context, "collector focus loss must not close retained renderer context");
    assert!(runtime.cache.is_none());
    for token in old { assert!(!runtime.coordinator.complete(token, 100)); }
    drop(runtime);
    assert_eq!(scheduler_tick(&window).len(), 3, "collector suspension must allow resumed refresh");
}
