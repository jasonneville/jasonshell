//! Visible Stack Browser subscriptions. Native roots are derived, never supplied
//! by the renderer; generation retirement precedes cancellation and slow joins.
//! The native worker uses ReadDirectoryChangesW, not interval scanning.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use tauri::{Emitter, EventTarget, Manager, WebviewWindow};

#[cfg(windows)]
mod native;
#[cfg(all(test, windows))]
use super::git_status::query_watch_summary;
#[cfg(windows)]
use native::ProductionWatch;

#[derive(Default)]
pub(super) struct WatchGeneration {
    next: u64,
    current: Option<u64>,
}

impl WatchGeneration {
    pub(super) fn reserve(&mut self) -> u64 {
        self.next = self
            .next
            .checked_add(1)
            .expect("watch generation exhausted");
        self.current = Some(self.next);
        self.next
    }
    pub(super) fn is_current(&self, token: u64) -> bool {
        self.current == Some(token)
    }
    pub(super) fn retire(&mut self, token: u64) -> bool {
        if !self.is_current(token) {
            return false;
        }
        self.current = None;
        true
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFolderWatch {
    watch_id: String,
    canonical_path: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FolderChanged {
    watch_id: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FolderWatchError {
    watch_id: String,
    message: String,
}

#[derive(Clone)]
struct WatchTarget {
    path: PathBuf,
    metadata: bool,
    recursive: bool,
}

fn watch_targets(path: &Path) -> Result<(PathBuf, Vec<WatchTarget>), String> {
    let folder =
        std::fs::canonicalize(path).map_err(|_| "Watched folder unavailable".to_string())?;
    if !folder.is_dir() {
        return Err("Watched path must be a directory".to_string());
    }
    let path = folder
        .to_str()
        .ok_or_else(|| "Watched path is not valid Unicode".to_string())?;
    let targets = if let Some((root, git_dir, common_dir)) =
        super::git_status::stack_git_watch_roots(path)?
    {
        let mut targets = vec![WatchTarget {
            path: root,
            metadata: false,
            recursive: true,
        }];
        for path in [git_dir, common_dir] {
            if !targets.iter().any(|target| target.path == path) {
                targets.push(WatchTarget {
                    path,
                    metadata: true,
                    recursive: true,
                });
            }
        }
        targets
    } else {
        vec![WatchTarget {
            path: folder.clone(),
            metadata: false,
            recursive: false,
        }]
    };
    Ok((folder, targets))
}

#[cfg(windows)]
struct ActiveWatch {
    token: u64,
    id: String,
    watch: ProductionWatch,
}

#[derive(Default)]
struct Registry {
    generation: WatchGeneration,
    starting: bool,
    retiring: bool,
    shutdown: bool,
    #[cfg(windows)]
    active: Option<ActiveWatch>,
}

#[derive(Default)]
pub(crate) struct FolderWatchRuntime {
    registry: Mutex<Registry>,
    changed: Condvar,
}

#[cfg(windows)]
struct StartAdmission {
    runtime: Arc<FolderWatchRuntime>,
    token: u64,
    installed: bool,
}

#[cfg(windows)]
impl StartAdmission {
    fn wait_for_retirement(&self) -> Result<(), String> {
        let mut registry = self
            .runtime
            .registry
            .lock()
            .map_err(|_| "Folder watcher unavailable".to_string())?;
        let (mut registry, _) = self
            .runtime
            .changed
            .wait_timeout_while(registry, Duration::from_secs(5), |registry| {
                registry.retiring
                    && !registry.shutdown
                    && registry.generation.is_current(self.token)
            })
            .map_err(|_| "Folder watcher unavailable".to_string())?;
        if registry.shutdown || !registry.generation.is_current(self.token) {
            return Err("Folder watch retired while waiting for cleanup".to_string());
        }
        if registry.retiring {
            return Err(
                "Folder watch cleanup did not complete; use Refresh or reopen to retry".to_string(),
            );
        }
        let old = registry.active.take();
        drop(registry);
        // Replacement joins run only on the caller's blocking worker, never
        // under the registry lock or on the window event thread.
        drop(old);
        let registry = self
            .runtime
            .registry
            .lock()
            .map_err(|_| "Folder watcher unavailable".to_string())?;
        if registry.shutdown || !registry.generation.is_current(self.token) {
            return Err("Folder watch retired before startup".to_string());
        }
        Ok(())
    }

    fn install(&mut self, watch: ProductionWatch, visible: bool) -> Result<(), String> {
        let mut registry = self
            .runtime
            .registry
            .lock()
            .map_err(|_| "Folder watcher unavailable".to_string())?;
        if !visible || registry.shutdown || !registry.generation.is_current(self.token) {
            drop(registry);
            drop(watch);
            return Err("Folder watch retired before startup completed".to_string());
        }
        registry.active = Some(ActiveWatch {
            token: self.token,
            id: watch_id(self.token),
            watch,
        });
        self.installed = true;
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for StartAdmission {
    fn drop(&mut self) {
        if let Ok(mut registry) = self.runtime.registry.lock() {
            registry.starting = false;
            if !self.installed {
                registry.generation.retire(self.token);
            }
        }
        self.runtime.changed.notify_all();
    }
}

#[cfg(windows)]
fn watch_id(token: u64) -> String {
    format!("stack-watch-{}-{token}", std::process::id())
}

#[cfg(windows)]
fn visible_instance(window: &WebviewWindow) -> bool {
    let Some(current) = window.app_handle().get_webview_window(window.label()) else {
        return false;
    };
    window
        .hwnd()
        .ok()
        .zip(current.hwnd().ok())
        .is_some_and(|(left, right)| left == right)
        && window.is_visible().unwrap_or(false)
}

#[cfg(windows)]
fn native_visible(hwnd: isize) -> bool {
    // Worker publication must not synchronously dispatch a visibility query to
    // the UI thread: exit/destroy can be waiting there for this worker to join.
    // Concrete instance continuity is guarded by the retired generation.
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(windows::Win32::Foundation::HWND(
            hwnd as *mut std::ffi::c_void,
        ))
        .as_bool()
    }
}

#[cfg(windows)]
impl FolderWatchRuntime {
    fn admit(self: &Arc<Self>) -> Result<StartAdmission, String> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| "Folder watcher unavailable".to_string())?;
        if registry.starting || registry.shutdown {
            return Err("Folder watch start already pending or shutting down".to_string());
        }
        registry.starting = true;
        let token = registry.generation.reserve();
        Ok(StartAdmission {
            runtime: self.clone(),
            token,
            installed: false,
        })
    }

    fn start_reserved(
        self: &Arc<Self>,
        window: &WebviewWindow,
        path: &str,
        mut admission: StartAdmission,
    ) -> Result<StackFolderWatch, String> {
        admission.wait_for_retirement()?;
        let token = admission.token;
        let (folder, targets) = watch_targets(Path::new(path))?;
        let hwnd = window
            .hwnd()
            .map_err(|_| "Stack Browser window unavailable".to_string())?
            .0 as isize;
        let id = watch_id(token);
        let weak = Arc::downgrade(self);
        let destination = window.clone();
        let event_id = id.clone();
        let on_change: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let Some(runtime) = weak.upgrade() else {
                return;
            };
            if !native_visible(hwnd) {
                return;
            }
            let Ok(registry) = runtime.registry.lock() else {
                return;
            };
            if !registry.generation.is_current(token) {
                return;
            }
            let _ = destination.emit_to(
                EventTarget::WebviewWindow {
                    label: destination.label().into(),
                },
                "stack-popup:folder-changed",
                FolderChanged {
                    watch_id: event_id.clone(),
                },
            );
        });
        let weak = Arc::downgrade(self);
        let destination = window.clone();
        let event_id = id.clone();
        let on_error: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let Some(runtime) = weak.upgrade() else {
                return;
            };
            let visible = native_visible(hwnd);
            let Ok(mut registry) = runtime.registry.lock() else {
                return;
            };
            if registry.generation.retire(token) && visible {
                let _ = destination.emit_to(
                    EventTarget::WebviewWindow {
                        label: destination.label().into(),
                    },
                    "stack-popup:folder-watch-error",
                    FolderWatchError {
                        watch_id: event_id.clone(),
                        message:
                            "Folder watching stopped; use Refresh or reopen the folder to retry"
                                .to_string(),
                    },
                );
            }
            runtime.changed.notify_all();
        });
        let watch = ProductionWatch::start(targets, on_change, on_error)?;
        let visible = visible_instance(window);
        admission.install(watch, visible)?;
        Ok(StackFolderWatch {
            watch_id: id,
            canonical_path: folder.to_string_lossy().into_owned(),
        })
    }

    fn take_retired(&self, id: Option<&str>, shutdown: bool) -> Option<ProductionWatch> {
        let mut registry = self.registry.lock().ok()?;
        registry.shutdown |= shutdown;
        if let Some(id) = id {
            if !registry
                .active
                .as_ref()
                .is_some_and(|active| active.id == id)
            {
                return None;
            }
        }
        let token = if id.is_some() {
            registry.active.as_ref().map(|active| active.token)
        } else {
            registry.generation.current
        };
        if let Some(token) = token {
            registry.generation.retire(token);
        }
        self.changed.notify_all();
        let active = registry.active.take()?;
        registry.retiring = true;
        active.watch.signal_stop();
        Some(active.watch)
    }

    fn finish_retirement(&self) {
        if let Ok(mut registry) = self.registry.lock() {
            registry.retiring = false;
        }
        self.changed.notify_all();
    }
}

pub(crate) async fn start(window: WebviewWindow, path: String) -> Result<StackFolderWatch, String> {
    #[cfg(windows)]
    {
        let runtime = window.state::<Arc<FolderWatchRuntime>>().inner().clone();
        if !visible_instance(&window) {
            return Err("Stack Browser must be visible to watch a folder".to_string());
        }
        // Admit before spawning: at most one pending start/blocking wait exists,
        // including rapid show/hide and concurrent renderer requests.
        let admission = runtime.admit()?;
        tauri::async_runtime::spawn_blocking(move || {
            runtime.start_reserved(&window, &path, admission)
        })
        .await
        .map_err(|error| format!("Failed to join folder watch start: {error}"))?
    }
    #[cfg(not(windows))]
    {
        let _ = (window, path);
        Err("Folder notifications require Windows".to_string())
    }
}

pub(crate) async fn stop(window: WebviewWindow, watch_id: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        let runtime = window.state::<Arc<FolderWatchRuntime>>().inner().clone();
        let watch = runtime.take_retired(Some(&watch_id), false);
        if let Some(watch) = watch {
            tauri::async_runtime::spawn_blocking(move || {
                drop(watch);
                runtime.finish_retirement();
            })
            .await
            .map_err(|error| format!("Failed to join folder watch stop: {error}"))?;
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (window, watch_id);
        Ok(())
    }
}

/// Retire immediately on real hide/destroy, but never join a worker on the UI thread.
pub(crate) fn retire(app: &tauri::AppHandle) {
    #[cfg(windows)]
    if let Some(state) = app.try_state::<Arc<FolderWatchRuntime>>() {
        let runtime = state.inner().clone();
        if let Some(watch) = runtime.take_retired(None, false) {
            tauri::async_runtime::spawn_blocking(move || {
                drop(watch);
                runtime.finish_retirement();
            });
        }
    }
}

pub(crate) fn shutdown(app: &tauri::AppHandle) {
    #[cfg(windows)]
    if let Some(runtime) = app.try_state::<Arc<FolderWatchRuntime>>() {
        let runtime = runtime.inner().clone();
        if let Some(watch) = runtime.take_retired(None, true) {
            // Exit must wake admission waits immediately, not synchronously join
            // a potentially slow driver cancellation on the UI/exit thread.
            tauri::async_runtime::spawn_blocking(move || {
                drop(watch);
                runtime.finish_retirement();
            });
        }
    }
}

#[cfg(all(test, windows))]
struct TestFolderWatch {
    watch: ProductionWatch,
}
#[cfg(all(test, windows))]
impl TestFolderWatch {
    fn start(path: &Path, on_change: Arc<dyn Fn() + Send + Sync>) -> Result<Self, String> {
        let (_, targets) = watch_targets(path)?;
        Ok(Self {
            watch: ProductionWatch::start(targets, on_change, Arc::new(|| {}))?,
        })
    }
    fn stop(&mut self) {
        self.watch.stop();
    }
    fn targets(&self) -> Vec<PathBuf> {
        self.watch.targets()
    }
}

// Thin headless seam: the production admission, wait, installation, retirement,
// cancellation and join boundaries are shared; only HWND visibility is supplied
// as true by tests. Every installed worker is the real Win32 ProductionWatch.
#[cfg(all(test, windows))]
struct TestWatchRuntime {
    runtime: Arc<FolderWatchRuntime>,
}
#[cfg(all(test, windows))]
struct TestWatchStart {
    admission: StartAdmission,
}
#[cfg(all(test, windows))]
struct TestWatchRetirement {
    runtime: Arc<FolderWatchRuntime>,
    watch: Option<ProductionWatch>,
}

#[cfg(all(test, windows))]
impl TestWatchRuntime {
    fn new() -> Self {
        Self {
            runtime: Arc::new(FolderWatchRuntime::default()),
        }
    }
    fn reserve_start(&self) -> Result<TestWatchStart, String> {
        self.runtime
            .admit()
            .map(|admission| TestWatchStart { admission })
    }
    fn start(&self, path: &Path, on_change: Arc<dyn Fn() + Send + Sync>) -> Result<u64, String> {
        self.reserve_start()?.start(path, on_change)
    }
    fn retire(&self) -> Option<TestWatchRetirement> {
        self.runtime
            .take_retired(None, false)
            .map(|watch| TestWatchRetirement {
                runtime: self.runtime.clone(),
                watch: Some(watch),
            })
    }
    fn shutdown(&self) -> Option<TestWatchRetirement> {
        self.runtime
            .take_retired(None, true)
            .map(|watch| TestWatchRetirement {
                runtime: self.runtime.clone(),
                watch: Some(watch),
            })
    }
    fn is_current(&self, token: u64) -> bool {
        self.runtime
            .registry
            .lock()
            .is_ok_and(|registry| registry.generation.is_current(token))
    }
}

#[cfg(all(test, windows))]
impl TestWatchStart {
    fn start(mut self, path: &Path, on_change: Arc<dyn Fn() + Send + Sync>) -> Result<u64, String> {
        self.admission.wait_for_retirement()?;
        let (_, targets) = watch_targets(path)?;
        let token = self.admission.token;
        let weak = Arc::downgrade(&self.admission.runtime);
        let guarded_change = Arc::new(move || {
            if let Some(runtime) = weak.upgrade() {
                if let Ok(registry) = runtime.registry.lock() {
                    if registry.generation.is_current(token) {
                        on_change();
                    }
                }
            }
        });
        let watch = ProductionWatch::start(targets, guarded_change, Arc::new(|| {}))?;
        self.admission.install(watch, true)?;
        Ok(token)
    }
}

#[cfg(all(test, windows))]
impl Drop for TestWatchRetirement {
    fn drop(&mut self) {
        if let Some(watch) = self.watch.take() {
            drop(watch);
            self.runtime.finish_retirement();
        }
    }
}

#[cfg(all(test, windows))]
mod acceptance {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/native/stack_folder_watch.rs"
    ));
}
