//! VIS-20b tier-1 tests: banner pause/resume. NOT wired by the test owner.
//!
//! Wire from main.rs (fixer):
//!   #[cfg(test)]
//!   #[path = "quick_command_notification_tier1_tests.rs"]
//!   mod quick_command_notification_tier1_tests;
//!
//! Required production seams (the symbols below are intentionally absent today, which is the RED evidence):
//!   * `BannerAction::Pause` and `BannerAction::Resume`, deserialized from the strings "pause" / "resume"
//!     (same `lowercase` serde rule as Open/Dismiss). They travel through the EXISTING
//!     `act_on_quick_command_notification(id, action)` command; no new command, ACL unchanged.
//!   * `pub(crate) const MAX_PAUSE_TOTAL: Duration = Duration::from_secs(30);` cumulative pause budget per banner.
//!   * `Queue::is_paused(&self) -> bool` (true only while the CURRENT shown banner is paused).
//!   * `Queue::act(id, Pause|Resume, now) -> bool` returns true iff `id` is the current Shown banner and
//!     `now` is before that banner's effective deadline; otherwise false and nothing changes.
//!   * Effective deadline = shown_at + BANNER_LIFETIME + (paused time already spent, capped by MAX_PAUSE_TOTAL).
//!     While paused `wake_at()` returns shown_at + BANNER_LIFETIME + MAX_PAUSE_TOTAL (the moment the pause budget
//!     would be exhausted), so a dead renderer can never pin a banner past that instant.
//!   * `Queue::expire`, `act(Dismiss|Open)`, `claim_open`, `clear` keep working while paused and leave
//!     `is_paused() == false` afterwards; a newly shown banner always starts unpaused with a fresh budget.

use crate::quick_command_notification_queue::{
    Banner, BannerAction, BannerStatus, Queue, RunTarget, BANNER_LIFETIME, MAX_PAUSE_TOTAL,
};
use std::time::{Duration, Instant};

fn secs(base: Instant, s: u64) -> Instant {
    base + Duration::from_secs(s)
}

fn target(n: u32) -> RunTarget {
    RunTarget {
        command_id: "build".into(),
        run_id: n.to_string(),
    }
}

/// Enqueue, offer, acknowledge and show one banner at `at`; returns its id.
fn show(queue: &mut Queue, run: u32, at: Instant) -> String {
    assert!(queue.enqueue(target(run), format!("Build {run}"), BannerStatus::Success, true));
    let id = queue.offer(at, true).expect("offered").id.clone();
    assert!(queue.acknowledge(&id, at));
    assert!(queue.settle_show(&id, true, at));
    id
}

fn current(queue: &Queue) -> Option<Banner> {
    queue.banner().cloned()
}

#[test]
fn constants_are_the_documented_bounds() {
    assert_eq!(BANNER_LIFETIME, Duration::from_secs(6));
    assert_eq!(MAX_PAUSE_TOTAL, Duration::from_secs(30));
}

#[test]
fn pause_and_resume_are_new_values_of_the_existing_action_enum() {
    let parse = |text: &str| serde_json::from_str::<BannerAction>(text);
    assert!(matches!(parse("\"pause\""), Ok(BannerAction::Pause)));
    assert!(matches!(parse("\"resume\""), Ok(BannerAction::Resume)));
    assert!(matches!(parse("\"open\""), Ok(BannerAction::Open)));
    assert!(matches!(parse("\"dismiss\""), Ok(BannerAction::Dismiss)));
    assert!(parse("\"freeze\"").is_err());
    assert!(parse("\"Pause\"").is_err(), "serde rule stays lowercase");
}

#[test]
fn unpaused_banner_expires_at_the_normal_lifetime() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    show(&mut queue, 1, t0);
    assert!(!queue.is_paused());
    assert_eq!(queue.wake_at(), Some(secs(t0, 6)));
    queue.expire(secs(t0, 6));
    assert!(current(&queue).is_none());
}

#[test]
fn resume_extends_the_deadline_by_exactly_the_paused_time() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    let id = show(&mut queue, 1, t0);
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 2)));
    assert!(queue.is_paused());
    assert!(queue.act(&id, BannerAction::Resume, secs(t0, 5)));
    assert!(!queue.is_paused());
    assert_eq!(queue.wake_at(), Some(secs(t0, 9)), "6s lifetime + 3s paused");
    queue.expire(secs(t0, 8));
    assert!(current(&queue).is_some(), "still shown before the extended deadline");
    queue.expire(secs(t0, 9));
    assert!(current(&queue).is_none());
}

#[test]
fn a_paused_banner_does_not_expire_at_the_original_deadline() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    let id = show(&mut queue, 1, t0);
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 2)));
    queue.expire(secs(t0, 10));
    assert_eq!(current(&queue).map(|b| b.id), Some(id));
    assert_eq!(queue.wake_at(), Some(secs(t0, 36)), "bounded wake while paused: lifetime + 30s budget");
}

#[test]
fn pause_is_bounded_so_a_dead_renderer_cannot_pin_a_banner() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    let id = show(&mut queue, 1, t0);
    // A second completion waits behind the paused banner.
    assert!(queue.enqueue(target(2), "Build 2".into(), BannerStatus::Failure, true));
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 1)));
    queue.expire(secs(t0, 36) - Duration::from_millis(1));
    assert!(current(&queue).is_some(), "not before the cap");
    queue.expire(secs(t0, 36));
    assert!(current(&queue).is_none(), "cap reached: expiry proceeds even though no resume ever came");
    assert!(!queue.is_paused());
    // Expiry advances to the waiting banner.
    let next = queue.offer(secs(t0, 36), true).expect("waiting banner offered").id.clone();
    assert_ne!(next, id);
}

#[test]
fn pause_budget_is_cumulative_across_pauses_and_never_exceeds_thirty_seconds() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    let id = show(&mut queue, 1, t0);
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 1)));
    assert!(queue.act(&id, BannerAction::Resume, secs(t0, 21))); // 20s used
    assert_eq!(queue.wake_at(), Some(secs(t0, 26)));
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 22)));
    assert_eq!(queue.wake_at(), Some(secs(t0, 36)), "only 10s of budget left: same absolute cap");
    assert!(queue.act(&id, BannerAction::Resume, secs(t0, 30))); // 8s more => 28s used
    assert_eq!(queue.wake_at(), Some(secs(t0, 34)));
    // The last two seconds of budget cannot be exceeded by a third long pause.
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 31)));
    queue.expire(secs(t0, 36));
    assert!(current(&queue).is_none());
}

#[test]
fn pause_and_resume_are_idempotent() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    let id = show(&mut queue, 1, t0);
    // Resume without a pause changes nothing.
    assert!(queue.act(&id, BannerAction::Resume, secs(t0, 1)));
    assert_eq!(queue.wake_at(), Some(secs(t0, 6)));
    assert!(!queue.is_paused());
    // Repeated pause keeps the first pause start.
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 2)));
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 3)));
    assert!(queue.act(&id, BannerAction::Resume, secs(t0, 5)));
    assert_eq!(queue.wake_at(), Some(secs(t0, 9)), "3s paused, not 2s or 4s");
    // Repeated resume keeps the deadline.
    assert!(queue.act(&id, BannerAction::Resume, secs(t0, 6)));
    assert_eq!(queue.wake_at(), Some(secs(t0, 9)));
}

#[test]
fn only_the_current_shown_banner_can_be_paused() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    assert!(!queue.act("1", BannerAction::Pause, t0), "no banner at all");
    // Present but not yet shown: Awaiting and Acknowledged phases reject pause.
    assert!(queue.enqueue(target(1), "Build 1".into(), BannerStatus::Success, true));
    let id = queue.offer(t0, true).unwrap().id.clone();
    assert!(!queue.act(&id, BannerAction::Pause, t0));
    assert!(queue.acknowledge(&id, t0));
    assert!(!queue.act(&id, BannerAction::Pause, t0));
    assert!(queue.settle_show(&id, true, t0));
    // Wrong or stale id.
    assert!(!queue.act("999", BannerAction::Pause, secs(t0, 1)));
    assert!(!queue.is_paused());
    assert!(!queue.act("999", BannerAction::Resume, secs(t0, 1)));
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 1)));
    assert!(queue.act(&id, BannerAction::Dismiss, secs(t0, 2)));
    assert!(!queue.act(&id, BannerAction::Pause, secs(t0, 3)), "retired id is stale");
    assert!(!queue.act(&id, BannerAction::Resume, secs(t0, 3)));
    assert!(!queue.is_paused());
}

#[test]
fn pause_or_resume_after_the_effective_deadline_is_rejected() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    let id = show(&mut queue, 1, t0);
    assert!(!queue.act(&id, BannerAction::Pause, secs(t0, 6)), "deadline reached");
    assert!(!queue.is_paused());
    // A paused banner whose budget ran out rejects a late resume.
    let mut queue = Queue::default();
    let id = show(&mut queue, 2, t0);
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 1)));
    assert!(!queue.act(&id, BannerAction::Resume, secs(t0, 36)));
}

#[test]
fn dismiss_and_open_retire_a_paused_banner_even_after_the_original_deadline() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    let id = show(&mut queue, 1, t0);
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 2)));
    assert!(queue.act(&id, BannerAction::Dismiss, secs(t0, 20)), "20s > 6s lifetime but paused");
    assert!(current(&queue).is_none());
    assert!(!queue.is_paused());

    let mut queue = Queue::default();
    let id = show(&mut queue, 2, t0);
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 2)));
    assert!(queue.act(&id, BannerAction::Open, secs(t0, 20)));
    assert!(!queue.is_paused());
    assert!(current(&queue).is_none());
    let claimed = queue.claim_open(&id, secs(t0, 21)).expect("open claim survives the pause");
    assert_eq!(claimed, target(2));
}

#[test]
fn panel_open_clear_and_window_destroy_retire_a_paused_banner() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    let id = show(&mut queue, 1, t0);
    assert!(queue.enqueue(target(2), "Build 2".into(), BannerStatus::Success, true));
    assert!(queue.act(&id, BannerAction::Pause, secs(t0, 2)));
    queue.clear(); // clear_queue is what panel-open, graceful exit, renderer destroy and retire_window call
    assert!(current(&queue).is_none());
    assert!(!queue.is_paused());
    assert!(queue.wake_at().is_none());
    assert!(queue.offer(secs(t0, 3), true).is_none(), "waiting banners were dropped too");
}

#[test]
fn next_banner_starts_unpaused_with_a_fresh_budget() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    let first = show(&mut queue, 1, t0);
    assert!(queue.enqueue(target(2), "Build 2".into(), BannerStatus::Failure, true));
    assert!(queue.act(&first, BannerAction::Pause, secs(t0, 1)));
    assert!(queue.act(&first, BannerAction::Dismiss, secs(t0, 10)));
    assert!(!queue.is_paused());

    let t1 = secs(t0, 11);
    let second = queue.offer(t1, true).expect("second offered").id.clone();
    assert!(queue.acknowledge(&second, t1));
    assert!(queue.settle_show(&second, true, t1));
    assert!(!queue.is_paused());
    assert_eq!(queue.wake_at(), Some(secs(t1, 6)), "fresh full lifetime");
    // Fresh 30s budget: a 29s pause is honoured in full.
    assert!(queue.act(&second, BannerAction::Pause, secs(t1, 1)));
    assert!(queue.act(&second, BannerAction::Resume, secs(t1, 30)));
    assert_eq!(queue.wake_at(), Some(secs(t1, 35)));
}

#[test]
fn next_banner_is_unpaused_after_the_cap_expired_the_previous_one() {
    let t0 = Instant::now();
    let mut queue = Queue::default();
    let first = show(&mut queue, 1, t0);
    assert!(queue.enqueue(target(2), "Build 2".into(), BannerStatus::Success, true));
    assert!(queue.act(&first, BannerAction::Pause, secs(t0, 1)));
    queue.expire(secs(t0, 40));
    let t1 = secs(t0, 40);
    let second = queue.offer(t1, true).unwrap().id.clone();
    assert!(queue.acknowledge(&second, t1));
    assert!(queue.settle_show(&second, true, t1));
    assert!(!queue.is_paused());
    assert_eq!(queue.wake_at(), Some(secs(t1, 6)));
}

// The command surface must not grow: pause/resume reuse the existing command, window authorization and ACL.
#[test]
fn pause_reuses_the_existing_authorized_command_and_adds_no_acl_surface() {
    let source = include_str!("quick_command_notification.rs");
    assert_eq!(source.matches("#[tauri::command]").count(), 3, "no new tauri command for pause/resume");
    let act = source
        .split("pub(crate) fn act_on_quick_command_notification(")
        .nth(1)
        .expect("act command exists");
    let body = act.split("\n}\n").next().unwrap();
    assert!(body.contains("authorize("), "caller-window authorization still runs first");
    assert!(body.contains("commands::ACT_ON_QUICK_COMMAND_NOTIFICATION"));
    assert!(body.contains("QUICK_COMMAND_NOTIFICATION_LABEL"), "only the notification window may act");
    let build = include_str!("../build.rs");
    assert!(build.contains("\"act_on_quick_command_notification\""));
    for forbidden in ["pause_quick_command_notification", "resume_quick_command_notification"] {
        assert!(!build.contains(forbidden) && !source.contains(forbidden), "{forbidden} must not exist");
        assert!(!include_str!("contracts.rs").contains(forbidden));
    }
}
