//! Deterministic production queue tests; no window, process, or audio device.
#[path = "../src/quick_command_notification_queue.rs"]
mod queue;

use queue::{BannerAction, BannerStatus, Queue, RunTarget, BANNER_LIFETIME, READINESS_LIMIT};
use std::time::{Duration, Instant};

fn target(run: u32) -> RunTarget {
    RunTarget {
        command_id: "saved-command".into(),
        run_id: format!("private-run-{run}"),
    }
}

fn add(queue: &mut Queue, run: u32) -> bool {
    queue.enqueue(
        target(run),
        format!("Run {run}"),
        BannerStatus::Success,
        true,
    )
}

fn offer_and_ack(queue: &mut Queue, now: Instant) -> String {
    let id = queue.offer(now, true).unwrap().id.clone();
    assert!(queue.acknowledge(&id, now));
    id
}

#[test]
fn only_hidden_normal_exits_are_eligible_and_failed_saves_do_not_reserve_slots() {
    use BannerStatus::{Failure, Success};
    for code in [None, Some(0), Some(9)] {
        for stopped in [false, true] {
            for hidden in [false, true] {
                for exiting in [false, true] {
                    let expected = if hidden && !stopped && !exiting {
                        code.map(|code| if code == 0 { Success } else { Failure })
                    } else {
                        None
                    };
                    assert_eq!(
                        BannerStatus::eligible(code, stopped, hidden, exiting),
                        expected
                    );
                }
            }
        }
    }
    let mut queue = Queue::default();
    assert!(!queue.enqueue(target(1), "Unsaved".into(), Failure, false));
    assert!(queue.offer(Instant::now(), true).is_none());
    assert!(add(&mut queue, 1));
    assert!(!add(&mut queue, 1));
}

#[test]
fn overflow_preserves_presented_run_and_stale_actions_or_timer_do_not_affect_replacement() {
    let now = Instant::now();
    let mut queue = Queue::default();
    assert!(add(&mut queue, 1));
    let first = offer_and_ack(&mut queue, now);
    assert!(queue.settle_show(&first, true, now));
    for run in 2..=5 {
        assert!(add(&mut queue, run));
    }
    assert_eq!(queue.banner().unwrap().label, "Run 1");
    assert!(queue.act(&first, BannerAction::Dismiss, now));
    let later = now + Duration::from_secs(1);
    let third = offer_and_ack(&mut queue, later);
    assert_eq!(queue.ready_to_show(later).unwrap().label, "Run 3");
    assert!(queue.settle_show(&third, true, later));
    assert!(!queue.act(&first, BannerAction::Open, later));
    assert!(!queue.settle_show(&first, true, later));
    queue.expire(now + BANNER_LIFETIME);
    assert_eq!(queue.banner().unwrap().id, third);
    queue.expire(later + BANNER_LIFETIME);
    assert_eq!(
        queue.offer(later + BANNER_LIFETIME, true).unwrap().label,
        "Run 4"
    );
}

#[test]
fn failed_show_does_not_publish_or_arm_and_confirmed_show_settles_only_once() {
    let now = Instant::now();
    let mut queue = Queue::default();
    add(&mut queue, 1);
    add(&mut queue, 2);
    let first = offer_and_ack(&mut queue, now);
    assert!(queue.banner().is_none());
    assert!(!queue.settle_show(&first, false, now));
    assert!(queue.wake_at().is_none());
    assert!(queue.banner().is_none());
    let second = offer_and_ack(&mut queue, now);
    assert!(queue.settle_show(&second, true, now));
    assert!(!queue.settle_show(&second, true, now));
    assert_eq!(queue.wake_at(), Some(now + BANNER_LIFETIME));
    assert_eq!(
        serde_json::to_value(queue.banner().unwrap()).unwrap(),
        serde_json::json!({
            "id": second, "label": "Run 2", "status": "success"
        })
    );
}

#[test]
fn open_is_one_shot_and_clear_invalidates_pending_or_unclaimed_work() {
    let now = Instant::now();
    let mut queue = Queue::default();
    add(&mut queue, 1);
    add(&mut queue, 2);
    let first = offer_and_ack(&mut queue, now);
    queue.settle_show(&first, true, now);
    assert!(queue.act(&first, BannerAction::Open, now));
    assert!(queue.banner().is_none());
    assert!(queue.offer(now, true).is_none());
    assert!(!queue.act(&first, BannerAction::Dismiss, now));
    assert_eq!(queue.claim_open(&first, now), Some(target(1)));
    assert_eq!(queue.claim_open(&first, now), None);
    let second = offer_and_ack(&mut queue, now);
    queue.settle_show(&second, true, now);
    assert!(!queue.act(&second, BannerAction::Open, now + BANNER_LIFETIME));
    assert!(queue.act(&second, BannerAction::Open, now));
    add(&mut queue, 3);
    queue.clear();
    assert_eq!(queue.claim_open(&second, now), None);
    assert!(queue.presentation().is_none());
    assert!(queue.wake_at().is_none());
}

#[test]
fn missing_expired_and_stale_renderer_acks_cannot_show_or_start_the_display_clock() {
    let now = Instant::now();
    let mut queue = Queue::default();
    assert!(add(&mut queue, 1));
    assert!(add(&mut queue, 2));
    assert!(queue.offer(now, false).is_none());
    assert!(queue.ready_to_show(now).is_none());
    let first = queue.offer(now, true).unwrap().id.clone();
    assert!(!queue.settle_show(&first, true, now));
    assert!(!queue.act(&first, BannerAction::Dismiss, now));
    let timeout = now + READINESS_LIMIT;
    assert_eq!(queue.wake_at(), Some(timeout));
    assert!(!queue.acknowledge(&first, timeout));
    let second = queue.offer(timeout, true).unwrap().id.clone();
    assert_ne!(first, second);
    assert!(!queue.acknowledge(&first, timeout));
    assert!(queue.ready_to_show(timeout).is_none());
    assert_eq!(queue.wake_at(), Some(timeout + READINESS_LIMIT));
    assert!(queue.acknowledge(&second, timeout));
    assert!(!queue.acknowledge(&second, timeout));
    assert_eq!(queue.ready_to_show(timeout).unwrap().id, second);
    assert!(queue.banner().is_none());
    let presented = timeout + Duration::from_millis(200);
    assert!(queue.settle_show(&second, true, presented));
    assert_eq!(queue.wake_at(), Some(presented + BANNER_LIFETIME));
}
