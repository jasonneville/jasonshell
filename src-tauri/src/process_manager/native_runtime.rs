//! Short-lock cache and native-owned independent lane scheduling.
#[cfg(all(test, windows))]
#[path = "../../tests/fixtures/graceful_exit_system_probe.rs"]
mod shutdown_test_probe;
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
    stop: Arc<AtomicBool>,
    scheduler: Mutex<Option<std::thread::JoinHandle<()>>>,
    workers: Arc<Mutex<Vec<std::thread::JoinHandle<()>>>>,
    worker_failed: Arc<AtomicBool>,
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
            stop: Arc::new(AtomicBool::new(false)),
            scheduler: Mutex::new(None),
            workers: Arc::new(Mutex::new(Vec::new())),
            worker_failed: Arc::new(AtomicBool::new(false)),
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
        if self.stop.load(Ordering::Acquire) { return; }
        let Ok(mut scheduler) = self.scheduler.lock() else { return; };
        // Serialize the last admission check with drain's scheduler inspection.
        if self.stop.load(Ordering::Acquire) { return; }
        if self.started.swap(true, Ordering::AcqRel) {
            return;
        }
        let app = app.clone();
        let origin = self.origin;
        let inner = self.inner.clone();
        let stop = self.stop.clone();
        let workers = self.workers.clone();
        let worker_failed = self.worker_failed.clone();
        *scheduler = Some(std::thread::spawn(move || {
            let live_collector = Arc::new(Mutex::new(
                super::windows_telemetry::LiveCollector::default(),
            ));
            loop {
                std::thread::sleep(Duration::from_millis(50));
                if stop.load(Ordering::Acquire) { break; }
                {
                    let mut workers = workers.lock().unwrap_or_else(|poison| poison.into_inner());
                    let mut index = 0;
                    while index < workers.len() {
                        if workers[index].is_finished() {
                            if workers.swap_remove(index).join().is_err() { worker_failed.store(true, Ordering::Release); }
                        } else { index += 1; }
                    }
                }
                let Some(window) = app.get_webview_window("process-manager") else {
                    break;
                };
                let visible = window.is_visible().unwrap_or(false);
                let focused = window.is_focused().unwrap_or(false);
                let ticks = origin.elapsed().as_millis().min(u64::MAX as u128) as u64;
                let mut tokens = Vec::with_capacity(3);
                // Shared native admission excludes new collection/probe work
                // atomically with the shutdown freeze. Existing work retains
                // its slot through actual worker completion.
                if let Ok(mut state) = inner.lock() {
                    let context = state.coordinator.context();
                    if context.visible && (!visible || !focused) {
                        state.suspend();
                    }
                    let context = state.coordinator.context();
                    // Refocus can retain the same mounted tab after a short blur.
                    // Resume its cache without publishing a new open/tab context.
                    if context.visible && visible && focused
                        && context.active_tab == ProcessManagerTab::SystemInfo
                        && state.cache.is_none()
                        && state.warm(now()).is_err()
                    {
                        continue;
                    }
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
                    let Ok(admission) = crate::graceful_exit::admit_mutation(&app) else {
                        if let Ok(mut state) = inner.lock() { state.suspend(); }
                        continue;
                    };
                    let worker = std::thread::spawn(move || {
                        let _admission = admission;
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
                                state.suspend();
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
                    // A poisoned ownership mutex is recoverable for retention,
                    // not permission to drop a still-running JoinHandle.
                    workers.lock().unwrap_or_else(|poison| poison.into_inner()).push(worker);
                }
            }
        }));
    }

    /// Stops only JasonShell telemetry/helper workers, never selected process rows.
    /// Timeouts retain every unfinished handle; retry joins the same workers.
    pub fn stop_and_drain(&self) -> Result<crate::graceful_exit::DrainReceipt, String> {
        self.stop.store(true, Ordering::Release);
        {
            let mut state = self.inner.lock().map_err(|_| "System Info state unavailable")?;
            state.suspend();
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let scheduler_done = {
                let mut scheduler = self.scheduler.lock().map_err(|_| "System Info scheduler unavailable")?;
                if scheduler.as_ref().is_some_and(|worker| worker.is_finished()) {
                    if scheduler.take().ok_or("System Info scheduler unavailable")?.join().is_err() {
                        self.worker_failed.store(true, Ordering::Release);
                    }
                }
                scheduler.is_none()
            };
            if scheduler_done {
                let mut workers = self.workers.lock().map_err(|_| "System Info workers unavailable")?;
                let mut index = 0;
                while index < workers.len() {
                    if workers[index].is_finished() {
                        if workers.swap_remove(index).join().is_err() { self.worker_failed.store(true, Ordering::Release); }
                    } else { index += 1; }
                }
                if workers.is_empty() {
                    if self.worker_failed.load(Ordering::Acquire) { return Err("System Info worker completion could not be verified".into()); }
                    return Ok(crate::graceful_exit::DrainReceipt { finished: true });
                }
            }
            if Instant::now() >= deadline { return Err("System Info helpers still draining; retry retained workers".into()); }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
