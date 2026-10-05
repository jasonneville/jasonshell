//! Actual production coordinator, synthetic boundary adapters only. No desktop/shell effects.
//! Acceptance groups 1–6: tests/snipping-product-handoff.md:95–112.
#![cfg(windows)]
#[path = "../src/snipping/mod.rs"]
mod snipping;
// Actual sibling save.rs is included by production mod.rs (save::SaveError is the
// coordinator's public SaveCode); do not substitute a test-only error module.

use snipping::{
    coordinator::*,
    geometry::{CroppedImage, LogicalRect, PhysicalRect},
};
use std::{
    collections::HashMap,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

type Trace = Arc<Mutex<Vec<String>>>;
fn record(trace: &Trace, value: impl Into<String>) {
    trace.lock().unwrap().push(value.into());
}
fn owner() -> Caller {
    Caller {
        label: "top-bar".into(),
        window_id: 1,
    }
}
fn rect() -> LogicalRect {
    LogicalRect {
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: 60.0,
    }
}
fn monitors() -> Vec<MonitorSnapshot> {
    vec![
        MonitorSnapshot {
            id: "m0".into(),
            rect: PhysicalRect {
                left: -1000,
                top: 0,
                right: -750,
                bottom: 150,
            },
            scale: 1.25,
            identity: 11,
        },
        MonitorSnapshot {
            id: "m1".into(),
            rect: PhysicalRect {
                left: 0,
                top: -200,
                right: 300,
                bottom: 0,
            },
            scale: 2.0,
            identity: 22,
        },
    ]
}
fn pixels(monitor: &MonitorSnapshot) -> Vec<u8> {
    let (w, h) = monitor.rect.dimensions().unwrap();
    (0..h)
        .flat_map(|y| (0..w).flat_map(move |x| [x as u8, y as u8, 71, 255]))
        .collect()
}

// Resources live in the native-registry boundary, not in a fake coordinator.
struct Resource {
    window: WindowRef,
    visible: bool,
    trace: Trace,
}
impl Drop for Resource {
    fn drop(&mut self) {
        record(
            &self.trace,
            format!("drop:{}", self.window.caller.window_id),
        );
    }
}
struct Windows {
    trace: Trace,
    next: Mutex<u64>,
    live: Mutex<HashMap<u64, Resource>>,
    contexts: Mutex<HashMap<u64, Context>>,
    fail: Mutex<Option<String>>,
    context_gate: Mutex<Option<Gate>>,
}
impl Windows {
    fn new(trace: Trace) -> Self {
        let resource = Resource {
            window: WindowRef {
                caller: owner(),
                monitor_id: None,
            },
            visible: true,
            trace: trace.clone(),
        };
        Self {
            trace,
            next: Mutex::new(100),
            live: Mutex::new(HashMap::from([(1, resource)])),
            contexts: Mutex::new(HashMap::new()),
            fail: Mutex::new(None),
            context_gate: Mutex::new(None),
        }
    }
    fn effect(&self, stage: &str) -> Result<(), SnipError> {
        record(&self.trace, stage);
        let mut fail = self.fail.lock().unwrap();
        if fail.as_deref() == Some(stage) {
            fail.take();
            Err(SnipError::WindowFailed)
        } else {
            Ok(())
        }
    }
    fn create(
        &self,
        token: &Token,
        monitor: &MonitorSnapshot,
        preview: bool,
    ) -> Result<WindowRef, SnipError> {
        self.effect(if preview {
            "preview"
        } else if monitor.id == "m0" {
            "overlay:m0"
        } else {
            "overlay:m1"
        })?;
        let mut next = self.next.lock().unwrap();
        *next += 1;
        let window = WindowRef {
            caller: Caller {
                label: if preview {
                    format!("snip-preview-{}", token.generation)
                } else {
                    format!("snip-overlay-{}-{}", token.generation, monitor.id)
                },
                window_id: *next,
            },
            monitor_id: Some(monitor.id.clone()),
        };
        self.live.lock().unwrap().insert(
            *next,
            Resource {
                window: window.clone(),
                visible: false,
                trace: self.trace.clone(),
            },
        );
        Ok(window)
    }
    fn overlay(&self, token: &Token, monitor: &str) -> Caller {
        self.live
            .lock()
            .unwrap()
            .values()
            .find(|r| {
                r.window.caller.label == format!("snip-overlay-{}-{monitor}", token.generation)
            })
            .unwrap()
            .window
            .caller
            .clone()
    }
    fn preview(&self, token: &Token) -> Caller {
        self.live
            .lock()
            .unwrap()
            .values()
            .find(|r| r.window.caller.label == format!("snip-preview-{}", token.generation))
            .unwrap()
            .window
            .caller
            .clone()
    }
    fn visible(&self, id: u64) -> bool {
        self.live
            .lock()
            .unwrap()
            .get(&id)
            .is_some_and(|r| r.visible)
    }
    fn count(&self) -> usize {
        self.live.lock().unwrap().len() - 1
    }
}
impl WindowPort for Windows {
    fn create_overlay(&self, t: &Token, m: &MonitorSnapshot) -> Result<WindowRef, SnipError> {
        self.create(t, m, false)
    }
    fn create_preview(
        &self,
        t: &Token,
        m: &MonitorSnapshot,
        _: u32,
        _: u32,
    ) -> Result<WindowRef, SnipError> {
        self.create(t, m, true)
    }
    fn emit_context(&self, w: &WindowRef, c: &Context) -> Result<(), SnipError> {
        self.effect("context")?;
        self.effect(if w.caller.label.starts_with("snip-preview-") {
            "context:preview"
        } else {
            "context:overlay"
        })?;
        self.contexts
            .lock()
            .unwrap()
            .insert(w.caller.window_id, c.clone());
        let gate = self.context_gate.lock().unwrap().take();
        if let Some(gate) = gate {
            gate.wait();
        }
        Ok(())
    }
    fn emit_armed(&self, _: &WindowRef, _: &Token) -> Result<(), SnipError> {
        self.effect("armed")
    }
    fn show(&self, w: &WindowRef) -> Result<(), SnipError> {
        self.effect("show")?;
        self.effect(if w.caller.label.starts_with("snip-preview-") {
            "show:preview"
        } else if w.monitor_id.as_deref() == Some("m1") {
            "show:m1"
        } else {
            "show:m0"
        })?;
        self.live
            .lock()
            .unwrap()
            .get_mut(&w.caller.window_id)
            .unwrap()
            .visible = true;
        record(&self.trace, format!("shown:{}", w.caller.window_id));
        Ok(())
    }
    fn hide(&self, w: &WindowRef) -> Result<(), SnipError> {
        self.effect("hide")?;
        self.live
            .lock()
            .unwrap()
            .get_mut(&w.caller.window_id)
            .unwrap()
            .visible = false;
        record(&self.trace, format!("hidden:{}", w.caller.window_id));
        Ok(())
    }
    fn close(&self, w: WindowRef) {
        record(&self.trace, format!("close:{}", w.caller.window_id));
        let mut live = self.live.lock().unwrap();
        if live.get(&w.caller.window_id).is_some_and(|r| r.window == w) {
            live.remove(&w.caller.window_id);
        }
    }
    fn still_same(&self, w: &WindowRef) -> bool {
        self.live
            .lock()
            .unwrap()
            .get(&w.caller.window_id)
            .is_some_and(|r| r.window.caller == w.caller)
    }
}

// Bounded blocking at injected boundaries proves the core releases its own lock.
struct Gate {
    entered: mpsc::SyncSender<()>,
    release: mpsc::Receiver<()>,
}
impl Gate {
    fn wait(self) {
        self.entered.send(()).unwrap();
        self.release
            .recv_timeout(Duration::from_secs(3))
            .expect("fixture gate not released");
    }
}
fn gate() -> (Gate, mpsc::Receiver<()>, mpsc::SyncSender<()>) {
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    (
        Gate {
            entered: entered_tx,
            release: release_rx,
        },
        entered_rx,
        release_tx,
    )
}
struct Capture {
    trace: Trace,
    topology: Mutex<Vec<MonitorSnapshot>>,
    fail: Mutex<bool>,
    gate: Mutex<Option<Gate>>,
}
impl CapturePort for Capture {
    fn snapshot(&self) -> Result<Vec<MonitorSnapshot>, SnipError> {
        record(&self.trace, "snapshot");
        Ok(self.topology.lock().unwrap().clone())
    }
    fn freeze(&self, monitors: &[MonitorSnapshot]) -> Result<Vec<Frame>, SnipError> {
        record(&self.trace, "freeze:begin");
        let gate = self.gate.lock().unwrap().take();
        if let Some(gate) = gate {
            gate.wait();
        }
        if *self.fail.lock().unwrap() {
            return Err(SnipError::CaptureFailed);
        }
        let frames = monitors
            .iter()
            .map(|m| Frame {
                monitor: m.clone(),
                rgba: pixels(m),
            })
            .collect();
        record(&self.trace, "freeze:all");
        Ok(frames)
    }
}
struct Clock(Mutex<Instant>);
impl Clock {
    fn advance(&self, d: Duration) {
        *self.0.lock().unwrap() += d;
    }
}
impl ClockPort for Clock {
    fn now(&self) -> Instant {
        *self.0.lock().unwrap()
    }
}
struct Clipboard {
    trace: Trace,
    outcome: Mutex<ClipboardOutcome>,
    gate: Mutex<Option<Gate>>,
    crops: Mutex<Vec<(u32, u32, usize)>>,
}
impl ClipboardPort for Clipboard {
    fn publish(&self, crop: &CroppedImage, retained: usize) -> ClipboardOutcome {
        record(&self.trace, "clipboard");
        assert!(retained >= crop.pixels.len());
        self.crops
            .lock()
            .unwrap()
            .push((crop.width, crop.height, retained));
        let gate = self.gate.lock().unwrap().take();
        if let Some(gate) = gate {
            gate.wait();
        }
        self.outcome.lock().unwrap().clone()
    }
}
struct Picker {
    trace: Trace,
    result: Mutex<Result<Option<PathBuf>, SaveCode>>,
    gate: Mutex<Option<Gate>>,
}
impl PickerPort for Picker {
    fn pick_png(&self) -> Result<Option<PathBuf>, SaveCode> {
        record(&self.trace, "picker");
        let gate = self.gate.lock().unwrap().take();
        if let Some(gate) = gate {
            gate.wait();
        }
        self.result.lock().unwrap().clone()
    }
}
struct Publisher {
    trace: Trace,
    images: Mutex<Vec<Vec<u8>>>,
    fail: Mutex<bool>,
}
impl PublisherPort for Publisher {
    fn publish_png(&self, _: &Path, png: &[u8]) -> Result<(), SaveCode> {
        record(&self.trace, "file");
        if *self.fail.lock().unwrap() {
            return Err(SaveCode::Failed);
        }
        self.images.lock().unwrap().push(png.to_vec());
        Ok(())
    }
}
struct Fixture {
    core: Arc<Coordinator>,
    trace: Trace,
    windows: Arc<Windows>,
    capture: Arc<Capture>,
    clock: Arc<Clock>,
    clipboard: Arc<Clipboard>,
    picker: Arc<Picker>,
    publisher: Arc<Publisher>,
}
impl Fixture {
    fn new() -> Self {
        let trace = Arc::new(Mutex::new(Vec::new()));
        let windows = Arc::new(Windows::new(trace.clone()));
        let capture = Arc::new(Capture {
            trace: trace.clone(),
            topology: Mutex::new(monitors()),
            fail: Mutex::new(false),
            gate: Mutex::new(None),
        });
        let clock = Arc::new(Clock(Mutex::new(Instant::now())));
        let clipboard = Arc::new(Clipboard {
            trace: trace.clone(),
            outcome: Mutex::new(ClipboardOutcome::Committed { durable: true }),
            gate: Mutex::new(None),
            crops: Mutex::new(Vec::new()),
        });
        let picker = Arc::new(Picker {
            trace: trace.clone(),
            result: Mutex::new(Ok(None)),
            gate: Mutex::new(None),
        });
        let publisher = Arc::new(Publisher {
            trace: trace.clone(),
            images: Mutex::new(Vec::new()),
            fail: Mutex::new(false),
        });
        let core = Arc::new(Coordinator::new(
            capture.clone(),
            windows.clone(),
            clipboard.clone(),
            picker.clone(),
            publisher.clone(),
            clock.clone(),
        ));
        Self {
            core,
            trace,
            windows,
            capture,
            clock,
            clipboard,
            picker,
            publisher,
        }
    }
    fn selecting(&self) -> (Token, Caller, Caller) {
        let t = self.core.start(owner()).unwrap();
        let a = self.windows.overlay(&t, "m0");
        let b = self.windows.overlay(&t, "m1");
        assert!(!self.core.ready(a.clone(), t.clone(), "m0".into()).unwrap());
        assert!(self.core.ready(b.clone(), t.clone(), "m1".into()).unwrap());
        (t, a, b)
    }
    fn preview(&self) -> (Token, Caller) {
        let (t, a, _) = self.selecting();
        assert!(self
            .core
            .begin_selection(a.clone(), t.clone(), "m0".into())
            .unwrap());
        assert!(self
            .core
            .complete(a, t.clone(), "m0".into(), rect())
            .unwrap());
        let p = self.windows.preview(&t);
        (t, p)
    }
    fn trace(&self) -> Vec<String> {
        self.trace.lock().unwrap().clone()
    }
    fn clean(&self) {
        let s = self.core.inspect();
        assert_eq!(s.phase, None);
        assert_eq!(s.frame_bytes, 0);
        assert_eq!(s.image_bytes, 0);
        assert_eq!(self.windows.count(), 0);
    }
}
fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut r = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut pixels = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut pixels).unwrap();
    pixels.truncate(info.buffer_size());
    (info.width, info.height, pixels)
}

#[test]
fn group1_concrete_owner_and_tokens_authorize_before_effects_and_own_context_only() {
    let f = Fixture::new();
    for caller in [
        Caller {
            label: "settings".into(),
            window_id: 1,
        },
        Caller {
            label: "top-bar".into(),
            window_id: 2,
        },
    ] {
        assert_eq!(f.core.start(caller), Err(SnipError::Unauthorized));
        assert!(f.trace().is_empty());
    }
    let t = f.core.start(owner()).unwrap();
    let a = f.windows.overlay(&t, "m0");
    let b = f.windows.overlay(&t, "m1");
    let c = f.core.context(a.clone()).unwrap();
    assert_eq!(c.token, t);
    assert_eq!(c.monitor_id, "m0");
    assert_eq!((c.origin_x, c.origin_y, c.scale_factor), (-1000, 0, 1.25));
    assert_eq!(f.core.context(b.clone()).unwrap().monitor_id, "m1");
    let before = f.trace();
    let reused = Caller {
        window_id: a.window_id + 999,
        ..a.clone()
    };
    assert!(matches!(
        f.core.context(reused.clone()),
        Err(SnipError::Unauthorized)
    ));
    assert_eq!(
        f.core.image(reused, t.clone(), "m0".into()),
        Err(SnipError::Unauthorized)
    );
    for generation in ["0", "01", "18446744073709551616", "-1", "2"] {
        assert_eq!(
            f.core.image(
                a.clone(),
                Token {
                    generation: generation.into(),
                    ..t.clone()
                },
                "m0".into()
            ),
            Err(SnipError::Stale)
        );
    }
    for capture_id in ["0".repeat(32), "é".repeat(65), "a".repeat(100)] {
        assert_eq!(
            f.core.image(
                a.clone(),
                Token {
                    capture_id,
                    ..t.clone()
                },
                "m0".into()
            ),
            Err(SnipError::Stale)
        );
    }
    assert_eq!(
        f.core.image(a.clone(), t.clone(), "m1".into()),
        Err(SnipError::Unauthorized)
    );
    assert_eq!(
        f.core.image(b, t.clone(), "m32".into()),
        Err(SnipError::Unauthorized)
    );
    assert_eq!(f.trace(), before);
    let (w, h, rgba) = decode(&f.core.image(a, t, "m0".into()).unwrap());
    assert_eq!((w, h), (250, 150));
    assert!(rgba == pixels(&monitors()[0]), "synthetic frame mismatch");
}

#[test]
fn group2_freeze_all_before_windows_and_show_only_after_all_unique_ready() {
    let f = Fixture::new();
    let t = f.core.start(owner()).unwrap();
    let a = f.windows.overlay(&t, "m0");
    let b = f.windows.overlay(&t, "m1");
    let trace = f.trace();
    assert!(
        trace.iter().position(|s| s == "freeze:all").unwrap()
            < trace.iter().position(|s| s == "overlay:m0").unwrap()
    );
    assert!(!trace.iter().any(|s| s == "show"));
    assert_eq!(
        f.core.begin_selection(a.clone(), t.clone(), "m0".into()),
        Err(SnipError::Busy)
    );
    assert!(!f.core.ready(a.clone(), t.clone(), "m0".into()).unwrap());
    assert!(!f.core.ready(a.clone(), t.clone(), "m0".into()).unwrap());
    assert_eq!(f.core.inspect().readiness_count, 1);
    let stale = Token {
        generation: "999".into(),
        ..t.clone()
    };
    assert!(f.core.ready(b.clone(), stale, "m1".into()).is_err());
    assert_eq!(f.core.inspect().readiness_count, 1);
    assert!(f.core.ready(b.clone(), t.clone(), "m1".into()).unwrap());
    assert!(f.windows.visible(a.window_id));
    assert!(f.windows.visible(b.window_id));
    let before = f.trace();
    assert!(f.core.ready(a, t, "m0".into()).unwrap());
    assert_eq!(before, f.trace());
}

#[test]
fn group2_caps_reject_before_freeze_or_window_allocation() {
    for case in 0..5 {
        let f = Fixture::new();
        let mut ms = monitors();
        match case {
            0 => ms.clear(),
            1 => {
                ms = (0..33)
                    .map(|i| MonitorSnapshot {
                        id: format!("m{i}"),
                        ..monitors()[0].clone()
                    })
                    .collect();
            }
            2 => ms[0].rect.right = ms[0].rect.left + 16_385,
            3 => {
                ms[0].rect.right = ms[0].rect.left + 10_000;
                ms[0].rect.bottom = 4000;
            }
            _ => {
                ms = (0..3)
                    .map(|i| MonitorSnapshot {
                        id: format!("m{i}"),
                        rect: PhysicalRect {
                            left: 0,
                            top: 0,
                            right: 5000,
                            bottom: 5000,
                        },
                        ..monitors()[0].clone()
                    })
                    .collect();
            }
        }
        *f.capture.topology.lock().unwrap() = ms;
        assert_eq!(f.core.start(owner()), Err(SnipError::CaptureFailed));
        f.clean();
        assert!(!f
            .trace()
            .iter()
            .any(|s| s == "freeze:begin" || s.starts_with("overlay:")));
    }
}

#[test]
fn group2_monotonic_decode_timeout_drops_all_owned_resources_exactly_once() {
    let f = Fixture::new();
    let t = f.core.start(owner()).unwrap();
    let a = f.windows.overlay(&t, "m0");
    let ids = f.core.inspect().window_ids;
    assert!(f.core.inspect().frame_bytes > 0);
    f.core.ready(a.clone(), t.clone(), "m0".into()).unwrap();
    f.clock.advance(Duration::from_millis(4999));
    f.core.expire();
    assert_eq!(f.windows.count(), 2);
    f.clock.advance(Duration::from_millis(1));
    f.core.expire();
    f.core.expire();
    f.clean();
    for id in ids {
        assert_eq!(
            f.trace()
                .iter()
                .filter(|s| **s == format!("drop:{id}"))
                .count(),
            1
        );
        assert_eq!(
            f.trace()
                .iter()
                .filter(|s| **s == format!("close:{id}"))
                .count(),
            1
        );
    }
    assert!(f.core.ready(a, t, "m0".into()).is_err());
    f.clean();
}

#[test]
fn group2_capture_create_context_show_and_preview_failures_unwind() {
    for failure in [
        "capture",
        "overlay:m0",
        "overlay:m1",
        "context",
        "show",
        "show:m1",
        "armed",
        "preview",
        "context:preview",
        "show:preview",
    ] {
        let f = Fixture::new();
        let preview_failure = failure == "preview" || failure.ends_with(":preview");
        if failure == "capture" {
            *f.capture.fail.lock().unwrap() = true;
        } else if !preview_failure {
            *f.windows.fail.lock().unwrap() = Some(failure.into());
        }
        match f.core.start(owner()) {
            Err(_) => {}
            Ok(t) => {
                let a = f.windows.overlay(&t, "m0");
                let b = f.windows.overlay(&t, "m1");
                assert!(!f.core.ready(a.clone(), t.clone(), "m0".into()).unwrap());
                if preview_failure {
                    assert!(f.core.ready(b, t.clone(), "m1".into()).unwrap());
                    f.core
                        .begin_selection(a.clone(), t.clone(), "m0".into())
                        .unwrap();
                    *f.windows.fail.lock().unwrap() = Some(failure.into());
                    assert_eq!(
                        f.core.complete(a, t, "m0".into(), rect()),
                        Err(SnipError::WindowFailed)
                    );
                } else {
                    assert_eq!(
                        f.core.ready(b, t, "m1".into()),
                        Err(SnipError::WindowFailed)
                    );
                }
            }
        }
        f.clean();
        let trace = f.trace();
        for dropped in trace.iter().filter(|s| s.starts_with("drop:")) {
            assert_eq!(trace.iter().filter(|s| *s == dropped).count(), 1);
            let closed = dropped.replace("drop:", "close:");
            assert_eq!(trace.iter().filter(|s| **s == closed).count(), 1);
        }
        assert!(!trace.iter().any(|s| s == "file"));
        assert_eq!(trace.iter().filter(|s| *s == "clipboard").count(), usize::from(failure == "show:preview"),
            "failure {failure}: precommit failure must not Copy; preview-show failure follows exactly one automatic attempt");
        // Existing native outcome is committed/durable. A postcommit WindowFailed
        // is not proof of cancellation or old clipboard preservation. Coordinator
        // must expose retained outcome via a bounded observer (handoff seam pending).
        if failure == "show:preview" {
            assert_eq!(f.clipboard.crops.lock().unwrap().len(), 1);
        }
    }
}

#[test]
fn group3_first_monitor_binding_trusted_fractional_conversion_and_independent_png_crop() {
    let f = Fixture::new();
    let (t, a, b) = f.selecting();
    assert!(f
        .core
        .begin_selection(a.clone(), t.clone(), "m0".into())
        .unwrap());
    assert!(!f
        .core
        .begin_selection(b.clone(), t.clone(), "m1".into())
        .unwrap());
    assert_eq!(
        f.core.complete(b, t.clone(), "m1".into(), rect()),
        Err(SnipError::Unauthorized)
    );
    assert_eq!(
        f.core.complete(a.clone(), t.clone(), "m1".into(), rect()),
        Err(SnipError::Unauthorized)
    );
    assert!(f.core.complete(a, t.clone(), "m0".into(), rect()).unwrap());
    let trace = f.trace();
    assert_eq!(
        trace.iter().filter(|s| *s == "clipboard").count(),
        1,
        "one automatic Copy per accepted completion"
    );
    assert_eq!(f.clipboard.crops.lock().unwrap()[0].0, 126);
    assert_eq!(f.clipboard.crops.lock().unwrap()[0].1, 75);
    let p = f.windows.preview(&t);
    let c = f.core.context(p.clone()).unwrap();
    assert_eq!((c.width, c.height), (126, 75));
    let (w, h, rgba) = decode(&f.core.image(p, t, "m0".into()).unwrap());
    assert_eq!((w, h), (126, 75));
    // floor(10*1.25)=12, floor(20*1.25)=25; ceil(110*1.25)=138.
    let expected: Vec<u8> = (25..100)
        .flat_map(|y| (12..138).flat_map(move |x| [x as u8, y as u8, 71, 255]))
        .collect();
    assert!(
        rgba == expected,
        "synthetic crop pixels differ; contents redacted"
    );
    assert_eq!(f.core.inspect().frame_bytes, 0);
    assert_eq!(f.windows.count(), 1);
    assert!(
        trace.iter().position(|s| s == "preview").unwrap()
            < trace.iter().position(|s| s == "context:preview").unwrap()
            && trace.iter().position(|s| s == "context:preview").unwrap()
                < trace.iter().position(|s| s == "clipboard").unwrap()
            && trace.iter().position(|s| s == "clipboard").unwrap()
                < trace.iter().position(|s| s == "show:preview").unwrap(),
        "prepare hidden preview/context, then automatic Copy exactly once, then show preview"
    );
}

#[test]
fn postcommit_preview_show_failure_retains_actual_outcome_token_and_never_republishes() {
    for outcome in [ClipboardOutcome::Committed { durable: true }, ClipboardOutcome::DurabilityLost,
        ClipboardOutcome::Rejected("clipboard-busy".into()), ClipboardOutcome::Unknown] {
        let f = Fixture::new(); let (token, caller, _) = f.selecting();
        f.core.begin_selection(caller.clone(), token.clone(), "m0".into()).unwrap();
        *f.clipboard.outcome.lock().unwrap() = outcome.clone();
        *f.windows.fail.lock().unwrap() = Some("show:preview".into());
        assert_eq!(f.core.complete(caller.clone(), token.clone(), "m0".into(), rect()), Err(SnipError::WindowFailed));
        f.clean();
        let recorded = f.core.inspect().last_publication.expect("postcommit rollback must retain actual publication truth");
        assert_eq!(recorded.token, token);
        assert_eq!(recorded.outcome.metadata(), outcome.metadata());
        assert_eq!(f.clipboard.crops.lock().unwrap().len(), 1);
        assert!(f.core.complete(caller, token.clone(), "m0".into(), rect()).is_err());
        assert_eq!(f.clipboard.crops.lock().unwrap().len(), 1, "show failure cannot auto-republish same generation");
        *f.windows.fail.lock().unwrap() = None;
        let next = f.core.start(owner()).unwrap(); assert_ne!(next, token);
        let prior = f.core.inspect().last_publication.unwrap();
        assert_eq!(prior.token, token, "historical outcome cannot be relabeled as newer capture");
        assert_eq!(prior.outcome.metadata(), outcome.metadata());
    }
}

#[test]
fn group3_topology_change_and_invalid_rect_never_publish() {
    for change in [false, true] {
        let f = Fixture::new();
        let (t, a, _) = f.selecting();
        f.core
            .begin_selection(a.clone(), t.clone(), "m0".into())
            .unwrap();
        if change {
            f.capture.topology.lock().unwrap()[0].identity += 1;
        }
        let selection = if change {
            rect()
        } else {
            LogicalRect {
                width: f64::NAN,
                ..rect()
            }
        };
        assert_eq!(
            f.core.complete(a, t, "m0".into(), selection),
            Err(if change {
                SnipError::TopologyChanged
            } else {
                SnipError::InvalidSelection
            })
        );
        assert!(f.clipboard.crops.lock().unwrap().is_empty());
        assert!(f.publisher.images.lock().unwrap().is_empty());
        f.clean();
    }
}

#[test]
fn group4_duplicate_start_complete_cancel_and_old_token_cannot_touch_new_generation() {
    let f = Fixture::new();
    let (t, a, _) = f.selecting();
    let before = f.trace();
    assert_eq!(f.core.start(owner()), Err(SnipError::Busy));
    assert_eq!(before, f.trace());
    f.core
        .begin_selection(a.clone(), t.clone(), "m0".into())
        .unwrap();
    f.core
        .complete(a.clone(), t.clone(), "m0".into(), rect())
        .unwrap();
    let p = f.windows.preview(&t);
    let before = f.trace();
    assert!(f
        .core
        .complete(a.clone(), t.clone(), "m0".into(), rect())
        .is_err());
    assert_eq!(before, f.trace());
    f.core.dismiss(p.clone(), t.clone()).unwrap();
    assert!(f.core.dismiss(p, t.clone()).is_err());
    let n = f.core.start(owner()).unwrap();
    assert_ne!(n, t);
    let before = f.trace();
    assert!(f.core.cancel(a, t).is_err());
    assert_eq!(f.trace(), before);
    assert_eq!(f.core.inspect().token, Some(n.clone()));
    let a = f.windows.overlay(&n, "m0");
    f.core.cancel(a.clone(), n.clone()).unwrap();
    let before = f.trace();
    assert!(f.core.cancel(a, n).is_err());
    assert_eq!(f.trace(), before);
    f.clean();
}

#[test]
fn group4_expired_deferred_capture_cannot_install_windows_or_close_new_generation() {
    let f = Fixture::new();
    let (g, entered, release) = gate();
    *f.capture.gate.lock().unwrap() = Some(g);
    let core = f.core.clone();
    let worker = thread::spawn(move || core.start(owner()));
    entered.recv_timeout(Duration::from_secs(1)).unwrap();
    f.clock.advance(Duration::from_secs(5));
    f.core.expire();
    let newer = f.core.start(owner()).unwrap();
    let ids = f.core.inspect().window_ids;
    release.send(()).unwrap();
    assert!(worker.join().unwrap().is_err());
    assert_eq!(f.core.inspect().token, Some(newer));
    assert_eq!(f.core.inspect().window_ids, ids);
    assert_eq!(f.windows.count(), 2);
}

#[test]
fn group5_preview_only_outcomes_are_passed_through_without_invented_success() {
    let f = Fixture::new();
    let (t, p) = f.preview();
    let before = f.trace();
    assert_eq!(
        f.clipboard.crops.lock().unwrap().len(),
        1,
        "initial automatic Copy must occur exactly once"
    );
    for caller in [
        owner(),
        Caller {
            window_id: p.window_id + 1,
            ..p.clone()
        },
    ] {
        assert!(f.core.copy(caller.clone(), t.clone()).is_err());
        assert!(f.core.save(caller.clone(), t.clone()).is_err());
        assert!(f.core.dismiss(caller, t.clone()).is_err());
    }
    assert_eq!(f.trace(), before);
    assert_eq!(
        f.clipboard.crops.lock().unwrap().len(),
        1,
        "unauthorized actions cannot add publication"
    );
    for outcome in [
        ClipboardOutcome::Rejected("clipboard-busy".into()),
        ClipboardOutcome::Committed { durable: true },
        ClipboardOutcome::Committed { durable: false },
        ClipboardOutcome::Unknown,
        ClipboardOutcome::DurabilityLost,
        ClipboardOutcome::Superseded,
    ] {
        *f.clipboard.outcome.lock().unwrap() = outcome.clone();
        assert_eq!(
            f.core.copy(p.clone(), t.clone()).unwrap().metadata(),
            outcome.metadata()
        );
    }
    assert_eq!(f.core.inspect().phase, Some(Phase::Preview));
    assert_eq!(
        f.clipboard.crops.lock().unwrap().len(),
        7,
        "one automatic plus six explicit Copy outcomes"
    );
}

#[test]
fn group5_deferred_copy_serializes_operations_and_blocks_replacement() {
    let f = Fixture::new();
    let (t, p) = f.preview();
    let (g, entered, release) = gate();
    *f.clipboard.gate.lock().unwrap() = Some(g);
    let (core, token, caller) = (f.core.clone(), t.clone(), p.clone());
    let worker = thread::spawn(move || core.copy(caller, token));
    entered.recv_timeout(Duration::from_secs(1)).unwrap();
    let before = f.trace();
    let (core, token, caller) = (f.core.clone(), t.clone(), p.clone());
    let (tx, rx) = mpsc::sync_channel(1);
    let probe = thread::spawn(move || {
        let results = (
            core.start(owner()),
            core.copy(caller.clone(), token.clone()).map(|_| ()),
            core.save(caller.clone(), token.clone()),
            core.dismiss(caller, token),
        );
        tx.send(results).unwrap();
    });
    let result = rx.recv_timeout(Duration::from_millis(100));
    release.send(()).unwrap();
    worker.join().unwrap().unwrap();
    probe.join().unwrap();
    let (start, copy, save, dismiss) = result.unwrap();
    assert_eq!(start, Err(SnipError::Busy));
    assert_eq!(copy, Err(SnipError::Busy));
    assert_eq!(save, Err(SnipError::Busy));
    assert_eq!(dismiss, Err(SnipError::Busy));
    assert_eq!(before, f.trace());
    assert!(f.windows.visible(p.window_id));
    assert_eq!(f.core.inspect().token, Some(t));
}

#[test]
fn group6_deferred_picker_pins_image_rejects_operations_without_lock_and_releases_on_cancel_error()
{
    for picker_result in [
        Ok(None),
        Err(SaveCode::AccessDenied),
        Ok(Some(PathBuf::from("synthetic-only.png"))),
    ] {
        let f = Fixture::new();
        let (t, p) = f.preview();
        let (g, entered, release) = gate();
        *f.picker.gate.lock().unwrap() = Some(g);
        *f.picker.result.lock().unwrap() = picker_result.clone();
        let (core, token, caller) = (f.core.clone(), t.clone(), p.clone());
        let worker = thread::spawn(move || core.save(caller, token));
        entered.recv_timeout(Duration::from_secs(1)).unwrap();
        // Run *all* lock-taking probes on bounded thread; release picker before assertions/join.
        let (core, token, caller) = (f.core.clone(), t.clone(), p.clone());
        let (tx, rx) = mpsc::sync_channel(1);
        let probe = thread::spawn(move || {
            let pin = core.inspect().save_pin;
            let results = (
                core.start(owner()),
                core.copy(caller.clone(), token.clone()).map(|_| ()),
                core.dismiss(caller.clone(), token.clone()),
                core.save(caller, token),
            );
            tx.send((pin, results)).unwrap();
        });
        let results = rx.recv_timeout(Duration::from_millis(100));
        let before = f.trace();
        release.send(()).unwrap();
        let settled = worker.join().unwrap().unwrap();
        probe.join().unwrap();
        let (pin, (start, copy, dismiss, save)) =
            results.expect("coordinator lock held across native picker");
        assert!(pin);
        assert_eq!(start, Err(SnipError::Busy));
        assert_eq!(copy, Err(SnipError::Busy));
        assert_eq!(dismiss, Err(SnipError::Busy));
        assert_eq!(save, Err(SnipError::Busy));
        assert!(!before.iter().any(|s| s == "hide"));
        assert_eq!(before.iter().filter(|s| *s == "freeze:begin").count(), 1);
        assert_eq!(f.core.inspect().token, Some(t.clone()));
        assert!(!f.core.inspect().save_pin);
        assert!(f.windows.visible(p.window_id));
        match picker_result {
            Ok(None) => {
                assert_eq!(settled, SaveOutcome::Cancelled);
                assert!(f.publisher.images.lock().unwrap().is_empty());
            }
            Err(code) => {
                assert_eq!(settled, SaveOutcome::Error(code));
                assert!(f.publisher.images.lock().unwrap().is_empty());
            }
            Ok(Some(_)) => {
                assert_eq!(settled, SaveOutcome::Saved);
                let images = f.publisher.images.lock().unwrap();
                let (w, h, _) = decode(&images[0]);
                assert_eq!((w, h), (126, 75));
            }
        }
        assert!(f.core.copy(p, t).is_ok());
        assert!(f.core.start(owner()).is_ok());
    }
}

#[test]
fn replacement_cancel_capture_failure_and_readiness_timeout_restore_exact_old_preview() {
    for failure in [
        "cancel",
        "capture",
        "timeout",
        "show:m1",
        "preview",
        "context:preview",
        "show:preview",
    ] {
        let f = Fixture::new();
        let (old, preview) = f.preview();
        let old_image = f.core.inspect().image_bytes;
        if failure == "capture" {
            *f.capture.fail.lock().unwrap() = true;
        }
        let started = f.core.start(owner());
        if failure == "capture" {
            assert_eq!(started, Err(SnipError::CaptureFailed));
        } else {
            let new = started.unwrap();
            assert_ne!(old, new);
            assert!(!f.windows.visible(preview.window_id));
            assert_eq!(f.core.inspect().image_bytes, old_image);
            let before = f.trace();
            assert!(f.core.copy(preview.clone(), old.clone()).is_err());
            assert_eq!(before, f.trace());
            if failure == "cancel" {
                f.core.cancel(f.windows.overlay(&new, "m0"), new).unwrap();
            } else if failure == "timeout" {
                f.clock.advance(Duration::from_secs(5));
                f.core.expire();
            } else {
                let a = f.windows.overlay(&new, "m0");
                let b = f.windows.overlay(&new, "m1");
                f.core.ready(a.clone(), new.clone(), "m0".into()).unwrap();
                if failure == "show:m1" {
                    *f.windows.fail.lock().unwrap() = Some(failure.into());
                    assert!(f.core.ready(b, new, "m1".into()).is_err());
                } else {
                    f.core.ready(b, new.clone(), "m1".into()).unwrap();
                    f.core
                        .begin_selection(a.clone(), new.clone(), "m0".into())
                        .unwrap();
                    *f.windows.fail.lock().unwrap() = Some(failure.into());
                    assert!(f.core.complete(a, new, "m0".into(), rect()).is_err());
                }
            }
        }
        assert_eq!(f.core.inspect().token, Some(old.clone()));
        assert_eq!(f.core.inspect().phase, Some(Phase::Preview));
        assert_eq!(f.core.inspect().image_bytes, old_image);
        assert_eq!(f.core.inspect().frame_bytes, 0);
        assert!(f.windows.visible(preview.window_id));
        assert_eq!(f.windows.count(), 1);
        assert!(!f
            .trace()
            .iter()
            .any(|s| *s == format!("close:{}", preview.window_id)));
        assert!(f.core.copy(preview, old).is_ok());
    }
}

#[test]
fn replacement_success_disposes_old_preview_once_and_denies_old_authority() {
    let f = Fixture::new();
    let (old, preview) = f.preview();
    let (new, a, _) = f.selecting();
    let trace = f.trace();
    let hidden = trace
        .iter()
        .position(|s| *s == format!("hidden:{}", preview.window_id))
        .unwrap();
    let freeze = trace.iter().rposition(|s| s == "freeze:begin").unwrap();
    assert!(hidden < freeze);
    assert!(!trace
        .iter()
        .any(|s| *s == format!("drop:{}", preview.window_id)));
    f.core
        .begin_selection(a.clone(), new.clone(), "m0".into())
        .unwrap();
    f.core
        .complete(a, new.clone(), "m0".into(), rect())
        .unwrap();
    assert_eq!(
        f.trace()
            .iter()
            .filter(|s| **s == format!("drop:{}", preview.window_id))
            .count(),
        1
    );
    assert_eq!(
        f.trace()
            .iter()
            .filter(|s| **s == format!("close:{}", preview.window_id))
            .count(),
        1
    );
    let before = f.trace();
    assert!(f.core.copy(preview.clone(), old.clone()).is_err());
    assert!(f.core.dismiss(preview, old).is_err());
    assert_eq!(before, f.trace());
    assert_eq!(f.core.inspect().token, Some(new));
    assert_eq!(f.windows.count(), 1);
}

#[test]
fn coordinator_drop_disposes_current_and_retained_previous_concrete_resources() {
    let f = Fixture::new();
    f.preview();
    f.core.start(owner()).unwrap();
    let windows = f.windows.clone();
    let trace = f.trace.clone();
    assert_eq!(windows.count(), 3);
    drop(f);
    assert_eq!(windows.count(), 0);
    let trace = trace.lock().unwrap();
    for entry in trace.iter().filter(|s| s.starts_with("drop:")) {
        assert_eq!(trace.iter().filter(|s| *s == entry).count(), 1);
    }
}

#[test]
fn buffered_own_context_read_during_initialization_does_not_wait_on_coordinator_lock() {
    let f = Fixture::new();
    let (g, entered, release) = gate();
    *f.windows.context_gate.lock().unwrap() = Some(g);
    let core = f.core.clone();
    let worker = thread::spawn(move || core.start(owner()));
    entered.recv_timeout(Duration::from_secs(1)).unwrap();
    let buffered = f
        .windows
        .contexts
        .lock()
        .unwrap()
        .values()
        .next()
        .unwrap()
        .clone();
    let caller = f.windows.overlay(&buffered.token, "m0");
    let core = f.core.clone();
    let (tx, rx) = mpsc::sync_channel(1);
    let probe = thread::spawn(move || {
        tx.send(core.context(caller)).unwrap();
    });
    let result = rx.recv_timeout(Duration::from_millis(100));
    release.send(()).unwrap();
    worker.join().unwrap().unwrap();
    probe.join().unwrap();
    let context = result
        .expect("core lock held over context emission")
        .unwrap();
    assert_eq!(context.token, buffered.token);
    assert_eq!(context.monitor_id, "m0");
    assert_eq!((context.width, context.height), (250, 150));
    assert_eq!(f.core.inspect().readiness_count, 0);
    assert!(!f.trace().iter().any(|s| s == "show"));
}

#[test]
fn group6_publisher_error_releases_save_pin_and_keeps_preview() {
    let f = Fixture::new();
    let (t, p) = f.preview();
    *f.picker.result.lock().unwrap() = Ok(Some(PathBuf::from("synthetic-only.png")));
    *f.publisher.fail.lock().unwrap() = true;
    assert_eq!(
        f.core.save(p.clone(), t.clone()).unwrap(),
        SaveOutcome::Error(SaveCode::Failed)
    );
    assert!(!f.core.inspect().save_pin);
    assert_eq!(f.core.inspect().phase, Some(Phase::Preview));
    assert!(f.windows.visible(p.window_id));
    assert!(f.publisher.images.lock().unwrap().is_empty());
    assert!(f.core.copy(p, t).is_ok());
}

#[test]
fn group5_topology_invalidated_preview_cannot_publish_clipboard() {
    let f = Fixture::new();
    let (token, preview) = f.preview();
    f.capture.topology.lock().unwrap()[0].scale = 1.5;
    let publications = f.clipboard.crops.lock().unwrap().len();
    let result = f.core.copy(preview, token);
    assert_eq!(
        f.clipboard.crops.lock().unwrap().len(),
        publications,
        "topology invalidation must reject before clipboard publication"
    );
    assert!(
        result.is_err(),
        "invalidated preview cannot report a Copy outcome"
    );
}

#[test]
fn group6_topology_change_while_picker_pending_rejects_late_file_publication() {
    let f = Fixture::new();
    let (token, preview) = f.preview();
    *f.picker.result.lock().unwrap() = Ok(Some(PathBuf::from("synthetic-only.png")));
    let (g, entered, release) = gate();
    *f.picker.gate.lock().unwrap() = Some(g);
    let core = f.core.clone();
    let worker = thread::spawn(move || core.save(preview, token));
    entered.recv_timeout(Duration::from_secs(1)).unwrap();
    f.capture.topology.lock().unwrap()[0].identity += 1;
    release.send(()).unwrap();
    let result = worker.join().unwrap();
    assert!(
        f.publisher.images.lock().unwrap().is_empty(),
        "changed topology must reject before file publisher even after picker returns"
    );
    assert!(!matches!(result, Ok(SaveOutcome::Saved)));
    assert!(!f.core.inspect().save_pin);
}

#[test]
fn replacement_inspection_accounts_for_previous_concrete_preview_and_retained_crop() {
    let f = Fixture::new();
    let (_, preview) = f.preview();
    let bytes = f.core.inspect().image_bytes;
    let token = f.core.start(owner()).unwrap();
    let snapshot = f.core.inspect();
    assert_eq!(snapshot.token, Some(token));
    assert_eq!(snapshot.image_bytes, bytes);
    assert!(snapshot.frame_bytes > 0);
    assert!(
        snapshot.window_ids.contains(&preview.window_id),
        "inspection must expose previous concrete preview, not merely new overlays"
    );
    assert_eq!(snapshot.window_ids.len(), 3);
}

#[test]
fn group6_reused_preview_label_during_picker_cannot_inherit_publication_authority() {
    let f = Fixture::new();
    let (token, preview) = f.preview();
    *f.picker.result.lock().unwrap() = Ok(Some(PathBuf::from("synthetic-only.png")));
    let (g, entered, release) = gate();
    *f.picker.gate.lock().unwrap() = Some(g);
    let (core, caller) = (f.core.clone(), preview.clone());
    let worker = thread::spawn(move || core.save(caller, token));
    entered.recv_timeout(Duration::from_secs(1)).unwrap();
    // Native registry replacement: same label, different concrete live instance.
    let replacement_id = preview.window_id + 1000;
    {
        let mut live = f.windows.live.lock().unwrap();
        let old = live.remove(&preview.window_id).unwrap();
        let replacement = WindowRef {
            caller: Caller {
                window_id: replacement_id,
                ..preview
            },
            monitor_id: old.window.monitor_id.clone(),
        };
        live.insert(
            replacement_id,
            Resource {
                window: replacement,
                visible: true,
                trace: f.trace.clone(),
            },
        );
    }
    release.send(()).unwrap();
    let result = worker.join().unwrap();
    assert!(
        f.publisher.images.lock().unwrap().is_empty(),
        "native instance invalidation must be rechecked immediately before file publication"
    );
    assert!(!matches!(result, Ok(SaveOutcome::Saved)));
    assert!(!f.core.inspect().save_pin);
    assert!(f.windows.visible(replacement_id));
}
