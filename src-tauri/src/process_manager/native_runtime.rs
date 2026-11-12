//! Short-lock cache and native-owned independent lane scheduling.
use super::system_info::*;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

pub use super::system_info::RuntimeInner;
pub struct SystemInfoRuntime {
    pub inner: Arc<Mutex<RuntimeInner>>,
    started: AtomicBool,
    pub origin: Instant,
}
impl Default for SystemInfoRuntime {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeInner {
                coordinator: Coordinator::default(),
                cache: None,
                revision: 0,
                probes: ProbeBudget::default(),
            })),
            started: AtomicBool::new(false),
            origin: Instant::now(),
        }
    }
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(9_007_199_254_740_991) as u64)
        .unwrap_or(0)
}
impl SystemInfoRuntime {
    pub fn start(&self, app: &AppHandle) {
        if self.started.swap(true, Ordering::AcqRel) {
            return;
        }
        let app = app.clone();
        let origin = self.origin;
        let inner = self.inner.clone();
        std::thread::spawn(move || {
            let live_collector = Arc::new(Mutex::new(
                super::windows_telemetry::LiveCollector::default(),
            ));
            loop {
                std::thread::sleep(Duration::from_millis(50));
                let Some(window) = app.get_webview_window("process-manager") else {
                    break;
                };
                let visible = window.is_visible().unwrap_or(false);
                let focused = window.is_focused().unwrap_or(false);
                let ticks = origin.elapsed().as_millis().min(u64::MAX as u128) as u64;
                let mut tokens = Vec::with_capacity(3);
                if let Ok(mut state) = inner.lock() {
                    let context = state.coordinator.context();
                    if context.visible && (!visible || !focused) {
                        state.invalidate();
                        let _ = state.coordinator.hide();
                    }
                    let context = state.coordinator.context();
                    for lane in [Lane::Live, Lane::Storage, Lane::Inventory] {
                        if let Some(token) = state.coordinator.admit(
                            lane,
                            context.open_generation,
                            context.tab_revision,
                            ticks,
                            visible,
                            focused,
                        ) {
                            tokens.push(token);
                        }
                    }
                }
                for token in tokens {
                    let inner = inner.clone();
                    let collector = live_collector.clone();
                    let app = app.clone();
                    std::thread::spawn(move || {
                        let result = match token.lane {
                            Lane::Live => collector.lock().ok().map(|mut collector| {
                                super::windows_telemetry::Collected::Live(
                                    collector.collect(token.generation, token.revision),
                                )
                            }),
                            Lane::Storage => Some(super::windows_telemetry::Collected::Storage(
                                super::volume_probe::collect(&inner, origin, token),
                            )),
                            Lane::Inventory => {
                                Some(super::windows_telemetry::Collected::Inventory(
                                    super::windows_telemetry::inventory(),
                                ))
                            }
                        };
                        let active =
                            app.get_webview_window("process-manager")
                                .is_some_and(|window| {
                                    window.is_visible().unwrap_or(false)
                                        && window.is_focused().unwrap_or(false)
                                });
                        if let Ok(mut state) = inner.lock() {
                            if !active && state.coordinator.context().visible {
                                state.invalidate();
                                let _ = state.coordinator.hide();
                            }
                            if !state
                                .coordinator
                                .complete(token, origin.elapsed().as_millis() as u64)
                            {
                                return;
                            }
                            let Some(result) = result else {
                                return;
                            };
                            let Some(revision) =
                                state.revision.checked_add(1).and_then(safe_number)
                            else {
                                state.invalidate();
                                let _ = state.coordinator.hide();
                                return;
                            };
                            state.revision = revision;
                            if let Some(cache) = state.cache.as_mut() {
                                match result {
                                    super::windows_telemetry::Collected::Live(live) => {
                                        cache.live = live
                                    }
                                    super::windows_telemetry::Collected::Storage(storage) => {
                                        cache.storage = storage
                                    }
                                    super::windows_telemetry::Collected::Inventory(inventory) => {
                                        cache.inventory = inventory
                                    }
                                }
                                cache.snapshot_revision = revision;
                                cache.captured_at_ms = now();
                            }
                        }
                    });
                }
            }
        });
    }
}
