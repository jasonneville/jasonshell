// Included at actual Coordinator module scope under cfg(test). No codec replacement.
// Test-only mutex access for native callback contention coverage. Keeps the real
// Coordinator lock/state private in production and never installs an OS hook.
pub(crate) fn snip_test_with_coordinator_lock(coordinator: &Coordinator, action: impl FnOnce()) {
    let _guard = coordinator.inner.lock().unwrap();
    action();
}
mod image_concurrency_verification {
    use super::*;
    use std::{cell::RefCell, collections::HashMap, io::Cursor, sync::{mpsc, atomic::{AtomicUsize, Ordering}}, thread};
    struct Gate { entered: mpsc::Sender<()>, release: mpsc::Receiver<()> }
    impl Gate {
        fn wait(self) {
            self.entered.send(()).unwrap();
            self.release.recv_timeout(Duration::from_secs(3)).expect("bounded fixture gate release");
        }
    }
    thread_local! { static ENCODE_GATE: RefCell<Option<(Token, String, Gate)>> = const { RefCell::new(None) }; }
    pub(super) fn before_encode(token: &Token, monitor: &str) {
        // Default no-op, restricted to the explicitly configured worker thread.
        if let Some((expected, id, gate)) = ENCODE_GATE.with(|slot| slot.borrow_mut().take()) {
            assert_eq!(token, &expected); assert_eq!(monitor, id); gate.wait();
        }
    }
    fn gate() -> (Gate, mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (entered, observed) = mpsc::channel(); let (release, waiting) = mpsc::channel();
        (Gate { entered, release: waiting }, observed, release)
    }
    struct Work<T> { result: mpsc::Receiver<T>, release: Option<mpsc::Sender<()>>, join: Option<thread::JoinHandle<()>> }
    impl<T: Send + 'static> Work<T> {
        fn spawn(release: mpsc::Sender<()>, action: impl FnOnce() -> T + Send + 'static) -> Self {
            let (send, result) = mpsc::channel();
            let join = thread::spawn(move || { let _ = send.send(action()); });
            Self { result, release: Some(release), join: Some(join) }
        }
        fn finish(mut self) -> T {
            let _ = self.release.take().unwrap().send(());
            let result = self.result.recv_timeout(Duration::from_secs(3)).expect("bounded worker completion");
            self.join.take().unwrap().join().unwrap(); result
        }
    }
    impl<T> Drop for Work<T> {
        fn drop(&mut self) {
            if let Some(release) = self.release.take() { let _ = release.send(()); }
            if let Some(join) = self.join.take() {
                // On assertion unwind release first. Join only once completion is
                // observed; gate itself also has a bounded failure path.
                if join.is_finished() || self.result.recv_timeout(Duration::from_secs(3)).is_ok() { let _ = join.join(); }
            }
        }
    }
    fn bar() -> Caller { Caller { label: "top-bar".into(), window_id: 1 } }
    fn monitors() -> Vec<MonitorSnapshot> {
        [("m0", 8, 6), ("m1", 7, 5)].into_iter().enumerate().map(|(i, (id, width, height))| MonitorSnapshot {
            id: id.into(), rect: PhysicalRect { left: 0, top: 0, right: width, bottom: height }, scale: 1.0, identity: i as u64 + 1,
        }).collect()
    }
    fn pixels(m: &MonitorSnapshot) -> Vec<u8> {
        let (w, h) = m.rect.dimensions().unwrap();
        (0..h).flat_map(|y| (0..w).flat_map(move |x| [x as u8, y as u8, 73, 255])).collect()
    }
    struct Capture { freeze_calls: AtomicUsize }
    impl CapturePort for Capture {
        fn snapshot(&self) -> Result<Vec<MonitorSnapshot>, SnipError> { Ok(monitors()) }
        fn freeze(&self, ms: &[MonitorSnapshot]) -> Result<Vec<Frame>, SnipError> {
            self.freeze_calls.fetch_add(1, Ordering::SeqCst);
            // Stable tiny-buffer capacities for logical budget boundary tests.
            Ok(ms.iter().map(|m| Frame { monitor: m.clone(), rgba: pixels(m).into_boxed_slice().into_vec() }).collect())
        }
    }
    struct Windows { next: Mutex<u64>, live: Mutex<HashMap<u64, WindowRef>>, contexts: Mutex<Vec<Context>>, preparing: Mutex<Option<Gate>>, effects: Mutex<Vec<&'static str>>, fail_preview_show: std::sync::atomic::AtomicBool }
    impl WindowPort for Windows {
        fn create_overlay(&self, t: &Token, m: &MonitorSnapshot) -> Result<WindowRef, SnipError> {
            let mut next = self.next.lock().unwrap(); *next += 1;
            let w = WindowRef { caller: Caller { label: format!("snip-overlay-{}-{}", t.generation, m.id), window_id: *next }, monitor_id: Some(m.id.clone()) };
            self.live.lock().unwrap().insert(*next, w.clone()); Ok(w)
        }
        fn create_preview(&self, t: &Token, m: &MonitorSnapshot, _: u32, _: u32) -> Result<WindowRef, SnipError> {
            let mut next = self.next.lock().unwrap(); *next += 1;
            let w = WindowRef { caller: Caller { label: format!("snip-preview-{}", t.generation), window_id: *next }, monitor_id: Some(m.id.clone()) };
            self.live.lock().unwrap().insert(*next, w.clone()); Ok(w)
        }
        fn emit_context(&self, _: &WindowRef, c: &Context) -> Result<(), SnipError> {
            self.contexts.lock().unwrap().push(c.clone());
            if c.monitor_id == "m1" {
                let gate = self.preparing.lock().unwrap().take();
                if let Some(gate) = gate { gate.wait(); }
            }
            Ok(())
        }
        fn emit_armed(&self, _: &WindowRef, _: &Token) -> Result<(), SnipError> { self.effects.lock().unwrap().push("armed"); Ok(()) }
        fn show(&self, w: &WindowRef) -> Result<(), SnipError> {
            self.effects.lock().unwrap().push("show");
            if w.caller.label.starts_with("snip-preview-") && self.fail_preview_show.load(Ordering::SeqCst) { return Err(SnipError::WindowFailed); }
            Ok(())
        }
        fn hide(&self, _: &WindowRef) -> Result<(), SnipError> { panic!("no previous preview") }
        fn close(&self, w: WindowRef) { assert_eq!(self.live.lock().unwrap().remove(&w.caller.window_id), Some(w)); self.effects.lock().unwrap().push("close"); }
        fn still_same(&self, w: &WindowRef) -> bool {
            w.caller == bar() || self.live.lock().unwrap().get(&w.caller.window_id).is_some_and(|live| live.caller == w.caller && w.monitor_id.as_ref().is_none_or(|m| live.monitor_id.as_ref() == Some(m)))
        }
    }
    struct NoPublication;
    struct HeldClipboard {
        gate: Mutex<Option<Gate>>,
        attempts: Mutex<Vec<(u32, u32, Vec<u8>)>>,
        outcome: ClipboardOutcome,
    }
    impl ClipboardPort for HeldClipboard {
        fn publish(&self, crop: &CroppedImage, _: usize) -> ClipboardOutcome {
            self.attempts.lock().unwrap().push((crop.width, crop.height, crop.pixels.clone()));
            let gate = self.gate.lock().unwrap().take();
            if let Some(gate) = gate { gate.wait(); }
            self.outcome.clone()
        }
    }
    impl ClipboardPort for NoPublication { fn publish(&self, _: &CroppedImage, _: usize) -> ClipboardOutcome { panic!("no clipboard") } }
    impl PickerPort for NoPublication { fn pick_png(&self) -> Result<Option<PathBuf>, SaveCode> { panic!("no picker") } }
    impl PublisherPort for NoPublication { fn publish_png(&self, _: &Path, _: &[u8]) -> Result<(), SaveCode> { panic!("no file") } }
    struct Clock(Mutex<Instant>);
    impl ClockPort for Clock { fn now(&self) -> Instant { *self.0.lock().unwrap() } }
    struct Fixture { core: Arc<Coordinator>, windows: Arc<Windows>, clock: Arc<Clock>, capture: Arc<Capture> }
    impl Fixture {
        fn new() -> Self {
            Self::with_clipboard(Arc::new(NoPublication))
        }
        fn with_clipboard(clipboard: Arc<dyn ClipboardPort>) -> Self {
            let windows = Arc::new(Windows { next: Mutex::new(10), live: Mutex::new(HashMap::new()), contexts: Mutex::new(Vec::new()), preparing: Mutex::new(None), effects: Mutex::new(Vec::new()), fail_preview_show: false.into() });
            let clock = Arc::new(Clock(Mutex::new(Instant::now())));
            let capture = Arc::new(Capture { freeze_calls: AtomicUsize::new(0) });
            Self { core: Arc::new(Coordinator::new(capture.clone(), windows.clone(), clipboard, Arc::new(NoPublication), Arc::new(NoPublication), clock.clone())), windows, clock, capture }
        }
        fn callers(&self) -> (Caller, Caller) {
            let live = self.windows.live.lock().unwrap();
            let get = |id: &str| live.values().find(|w| w.monitor_id.as_deref() == Some(id)).unwrap().caller.clone();
            (get("m0"), get("m1"))
        }
        fn held_image(&self, caller: Caller, token: Token, monitor: &str) -> Work<Result<Vec<u8>, SnipError>> {
            let (gate, entered, release) = gate(); let core = self.core.clone(); let monitor = monitor.to_string();
            let work = Work::spawn(release, move || {
                ENCODE_GATE.with(|slot| *slot.borrow_mut() = Some((token.clone(), monitor.clone(), gate)));
                core.image(caller, token, monitor)
            });
            entered.recv_timeout(Duration::from_secs(2)).expect("actual pre-encode boundary entered"); work
        }
        fn clean(&self) {
            let s = self.core.inspect(); assert!(s.phase.is_none()); assert_eq!(s.frame_bytes, 0); assert_eq!(s.image_bytes, 0); assert!(self.windows.live.lock().unwrap().is_empty());
        }
    }
    fn held_completion(outcome: ClipboardOutcome) -> (Fixture, Arc<HeldClipboard>, Work<Result<bool, SnipError>>, Caller, Token, Caller, Caller) {
        let (publish_gate, entered, release) = gate();
        let clipboard = Arc::new(HeldClipboard { gate: Mutex::new(Some(publish_gate)), attempts: Mutex::new(Vec::new()), outcome });
        let f = Fixture::with_clipboard(clipboard.clone());
        let token = f.core.start(bar()).unwrap();
        let (a, b) = f.callers();
        assert!(!f.core.ready(a.clone(), token.clone(), "m0".into()).unwrap());
        assert!(f.core.ready(b.clone(), token.clone(), "m1".into()).unwrap());
        assert!(f.core.begin_selection(a.clone(), token.clone(), "m0".into()).unwrap());
        let core = f.core.clone();
        let completing_token = token.clone();
        let completing_overlay = a.clone();
        let complete = Work::spawn(release, move || core.complete(completing_overlay, completing_token, "m0".into(), LogicalRect { x: 2.0, y: 1.0, width: 3.0, height: 2.0 }));
        entered.recv_timeout(Duration::from_secs(2)).expect("fake clipboard publication entered");
        let caller = f.windows.live.lock().unwrap().values().find(|w| w.caller.label.starts_with("snip-preview-")).unwrap().caller.clone();
        (f, clipboard, complete, caller, token, a, b)
    }

    fn preview_waiter(f: &Fixture, caller: Caller, token: Token, wait: Duration) -> Work<Result<Vec<u8>, SnipError>> {
        let core = f.core.clone();
        let (entered, observed) = mpsc::channel();
        let (unused, _) = mpsc::channel();
        let waiter = Work::spawn(unused, move || {
            Coordinator::observe_next_preview_wait(entered);
            core.image_with_preview_wait(caller, token, "m0".into(), wait)
        });
        observed.recv_timeout(Duration::from_secs(2)).expect("preview waiter reached the condvar under the coordinator lock");
        // Observe the atomic unlock before driving a state transition or releasing Copy.
        drop(f.core.inner.lock().unwrap());
        waiter
    }

    #[test]
    fn snip_preview_image_timeout_while_publication_is_held_does_not_cancel_completion() {
        let (f, clipboard, complete, caller, token, _, _) = held_completion(ClipboardOutcome::Rejected("fixture-no-os-clipboard".into()));
        let result = preview_waiter(&f, caller.clone(), token.clone(), Duration::from_millis(25)).finish();
        assert_eq!(result, Err(SnipError::ReadinessTimeout));
        assert_eq!(reservations(&f), 0);
        assert_eq!(complete.finish(), Ok(true));
        assert_eq!(clipboard.attempts.lock().unwrap().len(), 1);
        assert_eq!(f.core.dismiss(caller, token), Ok(true));
        f.clean();
    }

    #[test]
    fn snip_preview_image_waiter_wakes_without_pixels_when_postcommit_show_rolls_back() {
        let (f, clipboard, complete, caller, token, _, _) = held_completion(ClipboardOutcome::Committed { durable: true });
        f.windows.fail_preview_show.store(true, Ordering::SeqCst);
        let waiter = preview_waiter(&f, caller, token.clone(), PREVIEW_IMAGE_WAIT);
        assert_eq!(complete.finish(), Err(SnipError::WindowFailed));
        let publication = f.core.inspect().last_publication.unwrap();
        assert_eq!(publication.token, token);
        assert!(matches!(publication.outcome, ClipboardOutcome::Committed { durable: true }));
        assert_eq!(clipboard.attempts.lock().unwrap().len(), 1);
        f.clean();
        let replacement = f.core.start(bar()).unwrap();
        assert!(matches!(waiter.finish(), Err(SnipError::Inactive | SnipError::Stale | SnipError::Unauthorized)));
        assert_eq!(reservations(&f), 0);
        assert_eq!(f.core.inspect().token, Some(replacement.clone()));
        assert_eq!(f.core.cancel(f.callers().0, replacement), Ok(true));
        f.clean();
    }

    #[test]
    fn snip_preview_image_retirement_wakes_waiter_before_clipboard_returns() {
        let (f, clipboard, complete, caller, token, _, _) = held_completion(ClipboardOutcome::Committed { durable: true });
        let waiter = preview_waiter(&f, caller.clone(), token, PREVIEW_IMAGE_WAIT);
        f.core.preview_retired(&caller);
        assert_eq!(waiter.finish(), Err(SnipError::Unauthorized));
        assert_eq!(reservations(&f), 0);
        assert!(f.core.inspect().last_publication.is_none());
        let _ = complete.finish();
        assert_eq!(clipboard.attempts.lock().unwrap().len(), 1);
    }

    #[test]
    fn snip_preview_image_shutdown_wakes_waiter_before_clipboard_returns() {
        let (f, clipboard, complete, caller, token, _, _) = held_completion(ClipboardOutcome::Committed { durable: true });
        let waiter = preview_waiter(&f, caller, token, PREVIEW_IMAGE_WAIT);
        f.core.stop_images();
        assert_eq!(waiter.finish(), Err(SnipError::Inactive));
        assert_eq!(reservations(&f), 0);
        assert!(f.core.inspect().last_publication.is_none());
        let _ = complete.finish();
        assert_eq!(clipboard.attempts.lock().unwrap().len(), 1);
    }

    #[test]
    fn snip_preview_image_waits_during_automatic_copy_and_returns_exact_crop_once() {
        let (f, clipboard, complete, caller, token, a, b) = held_completion(ClipboardOutcome::Rejected("fixture-no-os-clipboard".into()));
        let context = f.core.context(caller.clone()).unwrap();
        let emitted = f.windows.contexts.lock().unwrap().last().unwrap().clone();
        for c in [context, emitted] {
            assert_eq!((c.token, c.phase, c.monitor_id.as_str(), c.width, c.height), (token.clone(), Phase::Preview, "m0", 3, 2));
        }
        assert_eq!(*f.windows.effects.lock().unwrap(), ["show", "show", "armed", "armed"], "preview stays hidden while clipboard is paused");
        assert_eq!(f.core.cancel(a, token.clone()), Err(SnipError::Busy));
        assert_eq!(f.core.start(bar()), Err(SnipError::Busy));
        let mut forged = caller.clone();
        forged.window_id += 1000;
        let mut wrong_token = token.clone();
        wrong_token.capture_id.replace_range(..1, if token.capture_id.starts_with('f') { "0" } else { "f" });
        let invalid = [
            (forged, token.clone(), "m0", SnipError::Unauthorized),
            (b, token.clone(), "m0", SnipError::Unauthorized),
            (caller.clone(), token.clone(), "m1", SnipError::Unauthorized),
            (caller.clone(), wrong_token, "m0", SnipError::Stale),
            (caller.clone(), Token { generation: "999".into(), capture_id: token.capture_id.clone() }, "m0", SnipError::Stale),
        ];
        let core = f.core.clone();
        let (invalid_release, _) = mpsc::channel();
        let invalid_requests = Work::spawn(invalid_release, move || invalid.map(|(c, t, m, expected)| (core.image(c, t, m.into()), expected)));
        for (result, expected) in invalid_requests.finish() {
            assert_eq!(result, Err(expected));
        }
        let request = preview_waiter(&f, caller.clone(), token.clone(), PREVIEW_IMAGE_WAIT);
        assert!(matches!(request.result.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert_eq!(complete.finish(), Ok(true));
        let expected: Vec<u8> = (1..3).flat_map(|y| (2..5).flat_map(move |x| [x, y, 73, 255])).collect();
        assert_eq!(*clipboard.attempts.lock().unwrap(), [(3, 2, expected.clone())], "one automatic attempt with exact crop");
        let bytes = request.finish().expect("original request succeeds without retry");
        let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
        let mut rgba = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut rgba).unwrap();
        assert_eq!((info.width, info.height), (3, 2));
        assert_eq!(&rgba[..info.buffer_size()], expected);
        assert_eq!(reservations(&f), 0);
        {
            let _pressure = LogicalPressure::leave_available(&f.core, 0);
            assert_eq!(f.core.image(caller.clone(), token.clone(), "m0".into()), Err(SnipError::ImageFailed));
        }
        let late = f.held_image(caller.clone(), token.clone(), "m0");
        assert_eq!(f.core.dismiss(caller, token.clone()), Ok(true));
        let replacement = f.core.start(bar()).unwrap();
        assert_ne!(replacement, token);
        assert!(matches!(late.finish(), Err(SnipError::Stale | SnipError::Inactive | SnipError::Unauthorized)));
        assert_eq!(f.core.inspect().token, Some(replacement.clone()));
        let (replacement_overlay, _) = f.callers();
        assert_eq!(f.core.cancel(replacement_overlay, replacement), Ok(true));
        f.clean();
    }
    fn assert_png(bytes: &[u8], m: &MonitorSnapshot) {
        let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
        let mut rgba = vec![0; reader.output_buffer_size()]; let info = reader.next_frame(&mut rgba).unwrap();
        assert_eq!((info.width, info.height), m.rect.dimensions().unwrap());
        assert_eq!(&rgba[..info.buffer_size()], pixels(m));
    }
    fn reservations(f: &Fixture) -> usize { f.core.inner.lock().unwrap().image_reservations }
    fn image_charge(f: &Fixture, monitor: &str) -> usize {
        let inner = f.core.inner.lock().unwrap();
        let bytes = inner.active.as_ref().unwrap().frames.iter().find(|frame| frame.monitor.id == monitor).unwrap().rgba.capacity();
        bytes * 2 + 65_536
    }
    // Inject only logical occupancy of the real private ledger. Production's
    // unchanged MAX_STAGING_BYTES and admission/RAII paths remain under test.
    // This is arithmetic evidence, NOT 512MiB allocation/IPC lifetime profiling.
    struct LogicalPressure<'a> { core: &'a Coordinator, bytes: usize }
    impl<'a> LogicalPressure<'a> {
        fn leave_available(core: &'a Coordinator, available: usize) -> Self {
            let mut inner = core.inner.lock().unwrap();
            let bytes = MAX_STAGING_BYTES.checked_sub(retained_bytes(&inner)).unwrap().checked_sub(available).unwrap();
            inner.image_reservations += bytes;
            Self { core, bytes }
        }
    }
    impl Drop for LogicalPressure<'_> {
        fn drop(&mut self) { self.core.inner.lock().unwrap().image_reservations -= self.bytes; }
    }
    #[test]
    fn snip_image_authorized_flat_frame_uses_compact_transport_png() {
        let f = Fixture::new(); let token = f.core.start(bar()).unwrap(); let (caller, _) = f.callers();
        let monitor = monitors()[0].clone(); let (width, height) = monitor.rect.dimensions().unwrap();
        let expected = [11, 22, 33, 255].repeat(width as usize * height as usize);
        {
            let mut inner = f.core.inner.lock().unwrap();
            inner.active.as_mut().unwrap().frames[0] = Arc::new(Frame {
                monitor: monitor.clone(), rgba: expected.clone().into_boxed_slice().into_vec(),
            });
        }
        let stored = super::super::clipboard::png_rgba(width, height, &expected, expected.len()).unwrap();
        let output = f.core.image(caller.clone(), token.clone(), monitor.id.clone()).unwrap();
        let mut decoder = png::Decoder::new(Cursor::new(&output)); decoder.ignore_checksums(false);
        let mut reader = decoder.read_info().unwrap(); let mut rgba = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut rgba).unwrap();
        assert_eq!((info.width, info.height), (width, height));
        assert_eq!(&rgba[..info.buffer_size()], expected);
        assert_eq!(reservations(&f), 0);
        f.core.cancel(caller, token).unwrap(); f.clean();
        assert!(output.len() < stored.len(), "authorized Coordinator::image must use compact transport, not unchanged stored Save/clipboard PNG; actual={} stored={}", output.len(), stored.len());
    }
    #[test]
    fn snip_image_concurrent_preparing_monitors_both_encode_then_arm() {
        let f = Fixture::new(); let (gate, entered, release) = gate(); *f.windows.preparing.lock().unwrap() = Some(gate);
        let core = f.core.clone(); let start = Work::spawn(release, move || core.start(bar()));
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        let state = f.core.inspect(); assert_eq!(state.phase, Some(Phase::Preparing));
        let token = state.token.unwrap(); let (a, b) = f.callers();
        let first = f.held_image(a.clone(), token.clone(), "m0");
        // Exact caller/token rejection remains before any codec work during overlap.
        let mut forged = a.clone(); forged.window_id += 1000;
        assert_eq!(f.core.image(forged, token.clone(), "m0".into()), Err(SnipError::Unauthorized));
        let stale = Token { generation: "999".into(), capture_id: token.capture_id.clone() };
        assert_eq!(f.core.image(b.clone(), stale, "m1".into()), Err(SnipError::Stale));
        let second = f.core.image(b.clone(), token.clone(), "m1".into());
        let first = first.finish(); let started = start.finish();
        assert_eq!(started, Ok(token.clone()));
        assert!(second.is_ok(), "first real encoder held at channel barrier; second authorized Preparing monitor must succeed, got {second:?}");
        assert_png(&first.unwrap(), &monitors()[0]); assert_png(&second.unwrap(), &monitors()[1]);
        assert!(!f.core.ready(a.clone(), token.clone(), "m0".into()).unwrap());
        assert!(f.core.ready(b, token.clone(), "m1".into()).unwrap());
        assert_eq!(f.core.inspect().readiness_count, 2);
        assert_eq!(*f.windows.effects.lock().unwrap(), ["show", "show", "armed", "armed"]);
        assert!(f.core.begin_selection(a.clone(), token.clone(), "m0".into()).unwrap());
        f.core.cancel(a, token).unwrap(); f.clean();
    }
    #[test]
    fn snip_image_cancel_during_encode_rejects_late_old_generation_without_touching_new() {
        let f = Fixture::new(); let token = f.core.start(bar()).unwrap(); let (a, _) = f.callers();
        let work = f.held_image(a.clone(), token.clone(), "m0");
        let cancelled = f.core.cancel(a, token.clone());
        let newer = if cancelled == Ok(true) { Some(f.core.start(bar()).unwrap()) } else { None };
        let late = work.finish();
        assert_eq!(cancelled, Ok(true), "image read must not prevent authorized cancellation");
        assert!(matches!(late, Err(SnipError::Stale | SnipError::Inactive)), "cancelled generation must never return encoded pixels");
        let newer = newer.unwrap(); assert_ne!(token, newer); assert_eq!(f.core.inspect().token, Some(newer.clone()));
        let (a, _) = f.callers(); assert_png(&f.core.image(a.clone(), newer.clone(), "m0".into()).unwrap(), &monitors()[0]);
        f.core.cancel(a, newer).unwrap(); f.clean();
    }
    #[test]
    fn snip_image_deadline_during_encode_discards_late_png_and_releases_resources() {
        let f = Fixture::new(); let token = f.core.start(bar()).unwrap(); let (a, _) = f.callers();
        let work = f.held_image(a, token.clone(), "m0");
        *f.clock.0.lock().unwrap() += Duration::from_secs(5); f.core.expire();
        let late = work.finish();
        assert!(matches!(late, Err(SnipError::ReadinessTimeout | SnipError::Stale | SnipError::Inactive)), "expired image must not escape: {late:?}");
        f.core.expire(); f.clean();
        assert!(!f.windows.effects.lock().unwrap().iter().any(|e| *e == "show" || *e == "armed"));
        let newer = f.core.start(bar()).unwrap(); let (a, _) = f.callers();
        assert_eq!(f.core.image(a.clone(), token, "m0".into()), Err(SnipError::Stale));
        f.core.cancel(a, newer).unwrap(); f.clean();
    }
    #[test]
    fn snip_image_reservation_exact_limit_refuses_one_byte_over_then_releases() {
        let f = Fixture::new(); let token = f.core.start(bar()).unwrap(); let (a, b) = f.callers();
        let first_charge = image_charge(&f, "m0"); let second_charge = image_charge(&f, "m1");
        let first = f.held_image(a.clone(), token.clone(), "m0");
        assert_eq!(reservations(&f), first_charge);
        {
            let pressure = LogicalPressure::leave_available(&f.core, second_charge - 1);
            let charged = reservations(&f);
            assert_eq!(f.core.image(b.clone(), token.clone(), "m1".into()), Err(SnipError::ImageFailed));
            assert_eq!(reservations(&f), charged, "refused admission must not leak a charge");
            drop(pressure);
        }
        assert_eq!(reservations(&f), first_charge);
        {
            let pressure = LogicalPressure::leave_available(&f.core, second_charge);
            let charged = reservations(&f);
            let second = f.core.image(b, token.clone(), "m1".into()).unwrap();
            assert_png(&second, &monitors()[1]);
            assert_eq!(reservations(&f), charged, "successful codec must release its exact reservation");
            drop(pressure);
        }
        assert_eq!(reservations(&f), first_charge);
        assert_png(&first.finish().unwrap(), &monitors()[0]);
        assert_eq!(reservations(&f), 0);
        f.core.cancel(a, token).unwrap(); f.clean();
    }
    #[test]
    fn snip_image_cancelled_source_charge_blocks_over_cap_replacement_until_release() {
        let f = Fixture::new(); let token = f.core.start(bar()).unwrap(); let (a, _) = f.callers();
        let charge = image_charge(&f, "m0");
        let first = f.held_image(a.clone(), token.clone(), "m0");
        f.core.cancel(a, token).unwrap(); f.clean();
        assert_eq!(reservations(&f), charge, "retired immutable source stays charged while encoder owns it");
        let frozen = monitors().iter().map(|m| pixels(m).len()).sum::<usize>();
        {
            let pressure = LogicalPressure::leave_available(&f.core, frozen - 1);
            let charged = reservations(&f);
            assert_eq!(f.core.start(bar()), Err(SnipError::CaptureFailed), "replacement must count retired read reservations");
            f.clean(); assert_eq!(reservations(&f), charged);
            drop(pressure);
        }
        let newer = f.core.start(bar()).unwrap();
        assert_eq!(reservations(&f), charge);
        assert_eq!(first.finish(), Err(SnipError::Stale));
        assert_eq!(reservations(&f), 0, "late old-generation result releases only its own charge");
        assert_eq!(f.core.inspect().token, Some(newer.clone()));
        let (a, _) = f.callers(); assert_png(&f.core.image(a.clone(), newer.clone(), "m0".into()).unwrap(), &monitors()[0]);
        assert_eq!(reservations(&f), 0); f.core.cancel(a, newer).unwrap(); f.clean();
    }
    #[test]
    fn snip_image_real_codec_rejection_releases_reservation_without_mutation_pin() {
        let f = Fixture::new(); let token = f.core.start(bar()).unwrap(); let (a, _) = f.callers();
        // Small malformed source forces actual PNG validation failure after a
        // reservation. This does NOT simulate allocator failure or valid capture.
        let original = {
            let mut inner = f.core.inner.lock().unwrap();
            let frame = &mut inner.active.as_mut().unwrap().frames[0];
            let malformed = Arc::new(Frame { monitor: frame.monitor.clone(), rgba: vec![73; 4] });
            std::mem::replace(frame, malformed)
        };
        let failed = f.core.image(a.clone(), token.clone(), "m0".into());
        f.core.inner.lock().unwrap().active.as_mut().unwrap().frames[0] = original;
        assert_eq!(failed, Err(SnipError::ImageFailed)); assert_eq!(reservations(&f), 0);
        assert!(!f.core.inner.lock().unwrap().active.as_ref().unwrap().busy);
        assert_png(&f.core.image(a.clone(), token.clone(), "m0".into()).unwrap(), &monitors()[0]);
        assert_eq!(reservations(&f), 0); f.core.cancel(a, token).unwrap(); f.clean();
    }
    #[test]
    fn snip_image_early_preparing_ack_records_all_but_arms_only_after_start_finishes() {
        let f = Fixture::new(); let (gate, entered, release) = gate(); *f.windows.preparing.lock().unwrap() = Some(gate);
        let core = f.core.clone(); let start = Work::spawn(release, move || core.start(bar()));
        entered.recv_timeout(Duration::from_secs(2)).unwrap();
        let token = f.core.inspect().token.unwrap(); let (a, b) = f.callers();
        assert_png(&f.core.image(a.clone(), token.clone(), "m0".into()).unwrap(), &monitors()[0]);
        assert_png(&f.core.image(b.clone(), token.clone(), "m1".into()).unwrap(), &monitors()[1]);
        assert!(!f.core.ready(a.clone(), token.clone(), "m0".into()).unwrap());
        assert!(!f.core.ready(b.clone(), token.clone(), "m1".into()).unwrap());
        assert_eq!(f.core.inspect().phase, Some(Phase::Preparing));
        assert_eq!(f.core.inspect().readiness_count, 2);
        assert!(!f.core.inner.lock().unwrap().active.as_ref().unwrap().armed);
        assert!(f.windows.effects.lock().unwrap().is_empty(), "no premature show/arm during construction");
        assert_eq!(f.core.begin_selection(a.clone(), token.clone(), "m0".into()), Err(SnipError::Busy));
        assert_eq!(start.finish(), Ok(token.clone()));
        assert!(f.core.inner.lock().unwrap().active.as_ref().unwrap().armed);
        assert_eq!(*f.windows.effects.lock().unwrap(), ["show", "show", "armed", "armed"]);
        assert!(f.core.ready(b, token.clone(), "m1".into()).unwrap());
        assert_eq!(f.windows.effects.lock().unwrap().len(), 4, "duplicate ACK cannot re-show/re-arm");
        f.core.cancel(a, token).unwrap(); f.clean();
    }
    #[test]
    fn snip_image_caller_retired_during_encode_cannot_receive_pixels_or_leak_charge() {
        let f = Fixture::new(); let token = f.core.start(bar()).unwrap(); let (a, _) = f.callers();
        let work = f.held_image(a.clone(), token.clone(), "m0");
        let retired = f.windows.live.lock().unwrap().remove(&a.window_id).unwrap();
        let late = work.finish();
        // Restore adapter ownership only so coordinator's exact cleanup can run.
        f.windows.live.lock().unwrap().insert(a.window_id, retired);
        assert_eq!(late, Err(SnipError::Unauthorized)); assert_eq!(reservations(&f), 0);
        assert_eq!(f.core.inspect().token, Some(token.clone()));
        f.core.cancel(a, token).unwrap(); f.clean();
    }
    #[test]
    fn snip_image_replacement_capture_transient_dib_counts_retired_reads_before_freeze() {
        let lengths: Vec<_> = monitors().iter().map(|m| pixels(m).len()).collect();
        let frozen = lengths.iter().sum::<usize>();
        let largest_dib = *lengths.iter().max().unwrap();
        let transient = frozen + largest_dib;
        // Both cases execute actual Coordinator::start against the unchanged
        // production 512MiB cap. Logical occupancy avoids large allocations.
        for one_byte_over in [false, true] {
            let f = Fixture::new(); let old = f.core.start(bar()).unwrap(); let (a, _) = f.callers();
            let first = f.held_image(a.clone(), old.clone(), "m0");
            f.core.cancel(a, old).unwrap(); f.clean();
            let retired_charge = reservations(&f);
            assert!(retired_charge > 0);
            let available = transient - usize::from(one_byte_over);
            let pressure = LogicalPressure::leave_available(&f.core, available);
            f.capture.freeze_calls.store(0, Ordering::SeqCst);
            let result = f.core.start(bar());
            let freeze_calls = f.capture.freeze_calls.load(Ordering::SeqCst);
            let admitted = result.is_ok();
            // Unwind all owned synthetic state BEFORE the deliberate RED assertion.
            if let Ok(token) = &result { let (a, _) = f.callers(); f.core.cancel(a, token.clone()).unwrap(); }
            f.clean(); drop(pressure);
            assert_eq!(reservations(&f), retired_charge);
            assert!(matches!(first.finish(), Err(SnipError::Stale | SnipError::Inactive)));
            assert_eq!(reservations(&f), 0);
            if one_byte_over {
                assert!(result == Err(SnipError::CaptureFailed) && freeze_calls == 0,
                    "retired read + frozen({frozen}) + largest DIB({largest_dib}) exceeds production cap by 1: expected CaptureFailed before freeze; admitted={admitted}, freeze_calls={freeze_calls}");
            } else {
                assert!(admitted && freeze_calls == 1, "exact logical transient boundary must remain legal: result={result:?}, freeze_calls={freeze_calls}");
                eprintln!("exact replacement transient boundary admitted; frozen={frozen},largestDib={largest_dib},freeze_calls=1");
            }
        }
    }
}
