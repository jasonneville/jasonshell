//! Tier 0 WIN-06 follow-up: fullscreen transitions racing shell cleanup.
//!
//! Required seams in `appbar.rs` (all private; this file is a child module):
//!
//! ```ignore
//! /// Polls `flag` until it reads false (<= 10 ms poll). true => idle within `timeout`.
//! fn wait_for_flag_clear(flag: &AtomicBool, timeout: Duration) -> bool;
//! /// `wait_for_flag_clear(&FULLSCREEN_TRANSITION_ACTIVE, timeout)`.
//! /// `begin_cleanup_plan` must call it with a timeout <= 2 s before taking state.
//! fn wait_for_fullscreen_transition_idle(timeout: Duration) -> bool;
//!
//! #[derive(Clone, Copy, Debug, PartialEq)]
//! enum StaleWorkAreaAction { Keep, RestoreBaseline(RECT), Nothing }
//! /// current => Keep; stale + Some(b) => RestoreBaseline(b); stale + None => Nothing.
//! fn stale_transition_work_area_action(transition_current: bool, baseline: Option<RECT>) -> StaleWorkAreaAction;
//!
//! /// Hide side effects. `is_current()` is re-validated (under the runtime lock in
//! /// production) before EACH side effect. Order: [is_current] set(monitor) [is_current] park [is_current].
//! /// Stale before set(monitor): set(baseline) if Some, no monitor SPI, no park, Ok(false).
//! /// Stale after set(monitor): set(baseline) if Some, no park, Ok(false).
//! /// Stale after park: set(baseline) if Some, Ok(false).
//! /// Current throughout: Ok(true). Errors from set/park propagate for park; set errors are best-effort.
//! fn run_fullscreen_hide_effects_with(
//!     layout: ShellSurfaceLayout,
//!     baseline: Option<RECT>,
//!     is_current: impl FnMut() -> bool,
//!     set_work_area: impl FnMut(RECT) -> AppResult<()>,
//!     park: impl FnMut(ShellSurfaceLayout) -> AppResult<()>,
//! ) -> AppResult<bool>;
//!
//! /// Restore side effect: reserved work area. Stale before => set(baseline) if Some, Ok(false),
//! /// never set(reserved). Stale after set(reserved) => set(baseline) if Some, Ok(false).
//! fn apply_reserved_work_area_with(
//!     baseline: Option<RECT>,
//!     reserved: RECT,
//!     is_current: impl FnMut() -> bool,
//!     set_work_area: impl FnMut(RECT) -> AppResult<()>,
//! ) -> AppResult<bool>;
//! ```
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn rect(l: i32, t: i32, r: i32, b: i32) -> RECT {
    RECT {
        left: l,
        top: t,
        right: r,
        bottom: b,
    }
}

fn monitor() -> RECT {
    rect(0, 0, 1920, 1080)
}

fn baseline() -> RECT {
    rect(0, 0, 1920, 1040)
}

fn layout() -> ShellSurfaceLayout {
    ShellSurfaceLayout {
        monitor_rect: monitor(),
        top_hwnd: 1,
        bottom_hwnd: 2,
        top_rect: rect(0, 0, 1920, 28),
        bottom_rect: rect(0, 1032, 1920, 1080),
    }
}

#[derive(Debug, PartialEq)]
enum Call {
    Set(RECT),
    Park,
}

/// `is_current` answers true for the first `current_checks` calls, then false
/// (simulates cleanup running between the lock drop and a re-lock).
fn run_hide(current_checks: usize, base: Option<RECT>) -> (bool, Vec<Call>) {
    let mut checks = 0usize;
    let calls = std::cell::RefCell::new(Vec::new());
    let ok = run_fullscreen_hide_effects_with(
        layout(),
        base,
        || {
            checks += 1;
            checks <= current_checks
        },
        |r| {
            calls.borrow_mut().push(Call::Set(r));
            Ok(())
        },
        |_| {
            calls.borrow_mut().push(Call::Park);
            Ok(())
        },
    )
    .unwrap();
    (ok, calls.into_inner())
}

#[test]
fn hide_current_throughout_sets_monitor_then_parks() {
    let (ok, calls) = run_hide(usize::MAX, Some(baseline()));
    assert!(ok);
    assert_eq!(calls, [Call::Set(monitor()), Call::Park]);
}

#[test]
fn hide_stale_before_any_effect_only_restores_baseline() {
    let (ok, calls) = run_hide(0, Some(baseline()));
    assert!(!ok);
    assert_eq!(calls, [Call::Set(baseline())], "never full-monitor, never park");
}

#[test]
fn hide_cleanup_between_spi_and_park_ends_on_baseline_without_parking() {
    // current for the pre-SPI check only; cleanup lands before the pre-park check.
    let (ok, calls) = run_hide(1, Some(baseline()));
    assert!(!ok);
    assert_eq!(calls, [Call::Set(monitor()), Call::Set(baseline())]);
    assert_eq!(calls.last(), Some(&Call::Set(baseline())), "final SPI target is baseline");
}

#[test]
fn hide_cleanup_after_park_still_ends_on_baseline() {
    let (ok, calls) = run_hide(2, Some(baseline()));
    assert!(!ok);
    assert_eq!(calls, [Call::Set(monitor()), Call::Park, Call::Set(baseline())]);
}

#[test]
fn hide_stale_without_baseline_never_touches_monitor_spi() {
    let (ok, calls) = run_hide(0, None);
    assert!(!ok);
    assert!(calls.is_empty(), "{calls:?}");
}

fn run_reserve(current_checks: usize, base: Option<RECT>) -> (bool, Vec<RECT>) {
    let mut checks = 0usize;
    let calls = std::cell::RefCell::new(Vec::new());
    let reserved = rect(0, 28, 1920, 1032);
    let ok = apply_reserved_work_area_with(
        base,
        reserved,
        || {
            checks += 1;
            checks <= current_checks
        },
        |r| {
            calls.borrow_mut().push(r);
            Ok(())
        },
    )
    .unwrap();
    (ok, calls.into_inner())
}

#[test]
fn reserve_current_applies_reserved_area() {
    let (ok, calls) = run_reserve(usize::MAX, Some(baseline()));
    assert!(ok);
    assert_eq!(calls, [rect(0, 28, 1920, 1032)]);
}

#[test]
fn reserve_stale_before_apply_restores_baseline_not_reserved() {
    let (ok, calls) = run_reserve(0, Some(baseline()));
    assert!(!ok);
    assert_eq!(calls, [baseline()]);
}

#[test]
fn reserve_cleanup_right_after_apply_ends_on_baseline() {
    let (ok, calls) = run_reserve(1, Some(baseline()));
    assert!(!ok);
    assert_eq!(calls, [rect(0, 28, 1920, 1032), baseline()]);
}

#[test]
fn reserve_stale_without_baseline_applies_nothing() {
    let (ok, calls) = run_reserve(0, None);
    assert!(!ok);
    assert!(calls.is_empty());
}

#[test]
fn stale_action_truth_table() {
    assert_eq!(stale_transition_work_area_action(true, Some(baseline())), StaleWorkAreaAction::Keep);
    assert_eq!(stale_transition_work_area_action(true, None), StaleWorkAreaAction::Keep);
    assert_eq!(
        stale_transition_work_area_action(false, Some(baseline())),
        StaleWorkAreaAction::RestoreBaseline(baseline())
    );
    assert_eq!(stale_transition_work_area_action(false, None), StaleWorkAreaAction::Nothing);
}

#[test]
fn bounded_wait_returns_immediately_when_idle() {
    let flag = AtomicBool::new(false);
    let started = Instant::now();
    assert!(wait_for_flag_clear(&flag, Duration::from_secs(2)));
    assert!(started.elapsed() < Duration::from_millis(500));
}

#[test]
fn bounded_wait_observes_release_from_another_thread() {
    let flag = Arc::new(AtomicBool::new(true));
    let releaser = {
        let flag = Arc::clone(&flag);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            flag.store(false, Ordering::Release);
        })
    };
    let started = Instant::now();
    assert!(wait_for_flag_clear(&flag, Duration::from_secs(2)));
    assert!(started.elapsed() >= Duration::from_millis(50), "must actually wait for release");
    assert!(started.elapsed() < Duration::from_millis(1500));
    releaser.join().unwrap();
}

#[test]
fn bounded_wait_gives_up_after_timeout_when_transition_never_ends() {
    let flag = AtomicBool::new(true);
    let started = Instant::now();
    assert!(!wait_for_flag_clear(&flag, Duration::from_millis(120)));
    let elapsed = started.elapsed();
    assert!(elapsed >= Duration::from_millis(100), "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
}
