//! Tier 1 RS-02: COMMAND_PANEL `Resized` bursts must not hit settings.json per event.
//!
//! Wiring (end of `command_panel.rs`):
//! ```ignore
//! #[cfg(test)]
//! #[path = "command_panel_tier1_tests.rs"]
//! mod tier1_tests;
//! ```
//!
//! Required seams in `command_panel.rs` (pub(crate)/private; this is a child module):
//!
//! ```ignore
//! pub(crate) type PanelSizePhysical = (u32, u32); // (width, height) physical px
//!
//! /// Pure trailing-debounce state machine, injected time, no I/O, no sleeping.
//! pub(crate) struct SizeDebouncer { .. }
//! impl SizeDebouncer {
//!     /// `persisted` = size currently stored on disk (None = unknown).
//!     pub(crate) fn new(quiet: Duration, persisted: Option<PanelSizePhysical>) -> Self;
//!     /// Resized event: memory only. Re-recording the persisted size with nothing newer
//!     /// pending clears pending (no write). Each record restarts the quiet window.
//!     pub(crate) fn record(&mut self, size: PanelSizePhysical, now: Instant);
//!     /// Latest size not yet durably persisted (what show/restore reads must see).
//!     pub(crate) fn pending(&self) -> Option<PanelSizePhysical>;
//!     /// Instant at which `poll` will first yield (last record + quiet), None if idle.
//!     pub(crate) fn due_at(&self) -> Option<Instant>;
//!     /// Some(size) exactly once when now >= due_at and pending != persisted. The
//!     /// caller persists it and then calls report_ok / report_err.
//!     pub(crate) fn poll(&mut self, now: Instant) -> Option<PanelSizePhysical>;
//!     /// hide/blur/close/shutdown: take pending immediately regardless of the window.
//!     pub(crate) fn flush(&mut self) -> Option<PanelSizePhysical>;
//!     pub(crate) fn report_ok(&mut self, size: PanelSizePhysical);
//!     /// Failure: persisted unchanged; size stays pending (unless a newer one arrived)
//!     /// and is retried at `now + quiet`; later records/polls/flushes still work.
//!     pub(crate) fn report_err(&mut self, size: PanelSizePhysical, now: Instant);
//! }
//!
//! /// Worker-backed saver (own thread; never persists on the caller of `record`).
//! pub(crate) struct DebouncedSizeSaver { .. }
//! impl DebouncedSizeSaver {
//!     pub(crate) fn spawn(
//!         quiet: Duration,
//!         persisted: Option<PanelSizePhysical>,
//!         persist: impl Fn(PanelSizePhysical) -> Result<(), String> + Send + 'static,
//!     ) -> Self;
//!     pub(crate) fn record(&self, size: PanelSizePhysical);      // non-blocking
//!     pub(crate) fn pending(&self) -> Option<PanelSizePhysical>;
//!     /// Persist pending now (hide/blur/close). true => nothing pending afterwards.
//!     /// Bounded by `timeout`.
//!     pub(crate) fn flush(&self, timeout: Duration) -> bool;
//!     /// flush + stop worker; MUST return within ~`timeout` even if `persist` hangs
//!     /// (3 s graceful-exit contract).
//!     pub(crate) fn shutdown(&self, timeout: Duration);
//! }
//! ```
//! Production must: `Resized` -> `saver.record(..)` only (no settings I/O on the event
//! thread); hide_command_panel / Focused(false) hide / CloseRequested / graceful exit
//! call flush/shutdown; show/restore sizing uses `pending()` over settings.

use super::{DebouncedSizeSaver, SizeDebouncer};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const QUIET: Duration = Duration::from_millis(500);

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn burst_only_updates_memory_and_yields_one_persist_of_last_size_after_quiet() {
    let t0 = Instant::now();
    let mut d = SizeDebouncer::new(QUIET, Some((800, 600)));
    for i in 0..200u32 {
        d.record((801 + i, 601 + i), t0 + ms(u64::from(i) * 5));
        assert_eq!(d.poll(t0 + ms(u64::from(i) * 5)), None, "no persist mid-burst");
    }
    let last_at = t0 + ms(199 * 5);
    assert_eq!(d.pending(), Some((1000, 800)));
    assert_eq!(d.due_at(), Some(last_at + QUIET));
    assert_eq!(d.poll(last_at + QUIET - ms(1)), None, "quiet period not over");
    assert_eq!(d.poll(last_at + QUIET), Some((1000, 800)));
    assert_eq!(d.poll(last_at + QUIET + ms(1)), None, "exactly one persist");
}

#[test]
fn each_record_restarts_the_quiet_window() {
    let t0 = Instant::now();
    let mut d = SizeDebouncer::new(QUIET, None);
    d.record((10, 10), t0);
    d.record((11, 11), t0 + ms(400));
    assert_eq!(d.poll(t0 + ms(600)), None);
    assert_eq!(d.poll(t0 + ms(900)), Some((11, 11)));
}

#[test]
fn resize_back_to_persisted_size_does_not_write() {
    let t0 = Instant::now();
    let mut d = SizeDebouncer::new(QUIET, Some((800, 600)));
    d.record((900, 700), t0);
    d.record((800, 600), t0 + ms(50));
    assert_eq!(d.pending(), None);
    assert_eq!(d.poll(t0 + ms(2000)), None);
    assert_eq!(d.flush(), None);

    // Same after a successful persist: A -> persisted, then back to A after B wandering.
    d.record((1000, 700), t0 + ms(3000));
    let s = d.poll(t0 + ms(3600)).unwrap();
    d.report_ok(s);
    d.record((1000, 700), t0 + ms(4000));
    assert_eq!(d.poll(t0 + ms(5000)), None, "identical to persisted");
}

#[test]
fn flush_takes_pending_immediately_once() {
    let t0 = Instant::now();
    let mut d = SizeDebouncer::new(QUIET, None);
    d.record((640, 480), t0);
    assert_eq!(d.flush(), Some((640, 480)));
    d.report_ok((640, 480));
    assert_eq!(d.flush(), None);
    assert_eq!(d.pending(), None);
    assert_eq!(d.poll(t0 + ms(5000)), None);
}

#[test]
fn pending_is_visible_until_persist_is_reported_ok() {
    let t0 = Instant::now();
    let mut d = SizeDebouncer::new(QUIET, None);
    d.record((700, 500), t0);
    let s = d.poll(t0 + QUIET).unwrap();
    // in flight: a show/restore read must still see it
    assert_eq!(d.pending(), Some((700, 500)));
    d.report_ok(s);
    assert_eq!(d.pending(), None);
}

#[test]
fn failed_write_does_not_wedge_later_writes() {
    let t0 = Instant::now();
    let mut d = SizeDebouncer::new(QUIET, Some((1, 1)));
    d.record((700, 500), t0);
    let s = d.poll(t0 + QUIET).unwrap();
    d.report_err(s, t0 + QUIET);
    assert_eq!(d.pending(), Some((700, 500)), "still pending after failure");
    assert_eq!(d.due_at(), Some(t0 + QUIET + QUIET));
    assert_eq!(d.poll(t0 + QUIET + ms(10)), None);
    assert_eq!(d.poll(t0 + QUIET + QUIET), Some((700, 500)), "retried");
    d.report_ok((700, 500));

    // a newer size during/after failure wins and is persisted
    d.record((710, 510), t0 + ms(3000));
    let s = d.poll(t0 + ms(3500)).unwrap();
    d.record((720, 520), t0 + ms(3600)); // arrives while write in flight
    d.report_err(s, t0 + ms(3700));
    assert_eq!(d.pending(), Some((720, 520)), "newer size not clobbered by failed older");
    assert_eq!(d.poll(t0 + ms(4100)), Some((720, 520)));
}

#[test]
fn newer_size_during_inflight_write_is_persisted_after_ok() {
    let t0 = Instant::now();
    let mut d = SizeDebouncer::new(QUIET, None);
    d.record((100, 100), t0);
    let s = d.poll(t0 + QUIET).unwrap();
    d.record((200, 200), t0 + QUIET + ms(10));
    d.report_ok(s);
    assert_eq!(d.pending(), Some((200, 200)));
    assert_eq!(d.poll(t0 + QUIET + ms(10) + QUIET), Some((200, 200)));
}

// ---- worker-backed saver (short real quiet window; condition waits only) ----

type Calls = Arc<Mutex<Vec<((u32, u32), std::thread::ThreadId)>>>;

fn recording_saver(
    quiet: Duration,
    persisted: Option<(u32, u32)>,
) -> (DebouncedSizeSaver, Calls, mpsc::Receiver<(u32, u32)>) {
    let calls: Calls = Arc::default();
    let (tx, rx) = mpsc::channel();
    let c = Arc::clone(&calls);
    let saver = DebouncedSizeSaver::spawn(quiet, persisted, move |size| {
        c.lock().unwrap().push((size, std::thread::current().id()));
        let _ = tx.send(size);
        Ok(())
    });
    (saver, calls, rx)
}

#[test]
fn saver_burst_persists_once_last_size_off_the_recording_thread() {
    let (saver, calls, rx) = recording_saver(ms(60), Some((1, 1)));
    for i in 0..300u32 {
        saver.record((500 + i, 400 + i));
    }
    assert_eq!(saver.pending(), Some((799, 699)));
    assert!(calls.lock().unwrap().is_empty(), "record() must not persist inline");
    assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), (799, 699));
    assert!(
        rx.recv_timeout(ms(250)).is_err(),
        "exactly one persist for the burst"
    );
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_ne!(calls[0].1, std::thread::current().id(), "persist runs off caller thread");
    drop(calls);
    assert_eq!(saver.pending(), None);
    saver.shutdown(Duration::from_secs(1));
}

#[test]
fn saver_flush_persists_pending_without_waiting_for_quiet_window() {
    let (saver, calls, rx) = recording_saver(Duration::from_secs(60), None);
    saver.record((640, 480));
    assert!(saver.flush(Duration::from_secs(2)));
    assert_eq!(rx.recv_timeout(Duration::from_secs(1)).unwrap(), (640, 480));
    assert_eq!(saver.pending(), None);
    assert!(saver.flush(Duration::from_secs(1)), "idempotent");
    assert_eq!(calls.lock().unwrap().len(), 1);
    saver.shutdown(Duration::from_secs(1));
}

#[test]
fn saver_shutdown_flushes_pending() {
    let (saver, calls, _rx) = recording_saver(Duration::from_secs(60), None);
    saver.record((321, 123));
    saver.shutdown(Duration::from_secs(2));
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, (321, 123));
}

#[test]
fn saver_shutdown_is_bounded_when_persist_hangs() {
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let release_rx = Mutex::new(release_rx);
    let saver = DebouncedSizeSaver::spawn(ms(10), None, move |_| {
        let _ = release_rx.lock().unwrap().recv_timeout(Duration::from_secs(10));
        Ok(())
    });
    saver.record((300, 300));
    std::thread::sleep(ms(80)); // let the worker enter the hanging persist
    let started = Instant::now();
    saver.shutdown(ms(300));
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "shutdown must honour its timeout (3 s graceful-exit contract), took {:?}",
        started.elapsed()
    );
    let _ = release_tx.send(());
}

#[test]
fn saver_failed_write_does_not_wedge_later_ones() {
    let attempts = Arc::new(Mutex::new(0u32));
    let (tx, rx) = mpsc::channel();
    let a = Arc::clone(&attempts);
    let saver = DebouncedSizeSaver::spawn(ms(30), None, move |size| {
        let mut n = a.lock().unwrap();
        *n += 1;
        if *n == 1 {
            return Err("disk full".to_string());
        }
        let _ = tx.send(size);
        Ok(())
    });
    saver.record((100, 100));
    // retry of the failed size, or a later size, must still land
    saver.record((101, 101));
    let got = rx.recv_timeout(Duration::from_secs(5)).expect("a later write succeeds");
    assert_eq!(got, (101, 101));
    saver.shutdown(Duration::from_secs(1));
}

// ---------------------------------------------------------------- failure backoff

/// Drive one failed attempt at the debouncer's own due time; returns that attempt's `now`.
fn fail_once(d: &mut SizeDebouncer, size: (u32, u32)) -> Instant {
    let due = d.due_at().expect("a retry must be scheduled");
    assert_eq!(d.poll(due), Some(size));
    d.report_err(size, due);
    due
}

#[test]
fn consecutive_failures_back_off_exponentially_capped_then_give_up_at_five() {
    let t0 = Instant::now();
    let quiet = Duration::from_secs(10);
    let mut d = SizeDebouncer::new(quiet, None);
    d.record((300, 200), t0);
    let expect = [10u64, 20, 30, 30]; // min(10*2^(n-1), 30) for n = 1..4
    for secs in expect {
        let at = fail_once(&mut d, (300, 200));
        assert_eq!(d.due_at(), Some(at + Duration::from_secs(secs)));
    }
    let at = fail_once(&mut d, (300, 200)); // 5th
    let _ = at;
    assert_eq!(d.due_at(), None, "gives up after 5 consecutive failures");
    assert_eq!(d.pending(), Some((300, 200)));
    assert_eq!(d.poll(t0 + Duration::from_secs(100_000)), None);
    assert_eq!(d.flush(), Some((300, 200)), "explicit flush after give-up still works");
}

#[test]
fn default_quiet_doubles_each_failure() {
    let t0 = Instant::now();
    let mut d = SizeDebouncer::new(QUIET, None);
    d.record((1, 1), t0);
    for factor in [1u32, 2, 4, 8] {
        let at = fail_once(&mut d, (1, 1));
        assert_eq!(d.due_at(), Some(at + QUIET * factor));
    }
}

#[test]
fn failure_counter_resets_on_record_and_on_report_ok() {
    let t0 = Instant::now();
    let mut d = SizeDebouncer::new(QUIET, None);
    d.record((1, 1), t0);
    fail_once(&mut d, (1, 1));
    fail_once(&mut d, (1, 1));
    fail_once(&mut d, (1, 1)); // next delay would be 8*quiet
    d.record((2, 2), t0 + ms(60_000));
    let at = fail_once(&mut d, (2, 2));
    assert_eq!(d.due_at(), Some(at + QUIET), "record resets the failure streak");

    let ok_at = at + ms(10);
    let s = d.poll(d.due_at().unwrap()).unwrap();
    d.report_ok(s);
    d.record((3, 3), ok_at + ms(1_000));
    let at = fail_once(&mut d, (3, 3));
    assert_eq!(d.due_at(), Some(at + QUIET), "report_ok resets the failure streak");
}
