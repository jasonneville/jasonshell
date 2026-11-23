//! Tier 0 WIN-16 acceptance: single-window Close is request-only and sends
//! at most one WM_CLOSE unless delivery genuinely failed.
//!
//! Required seam in `actions.rs` (pure):
//!
//! ```ignore
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! enum CloseFollowup { Done, PostFallback }
//! /// Decides what follows the synchronous SendMessageTimeoutW(WM_CLOSE).
//! /// send_succeeded                                   => Done
//! /// !send_succeeded && last_error == 1460 (TIMEOUT)  => Done (a Save prompt is
//! ///     probably open; a second WM_CLOSE would duplicate/answer it)
//! /// !send_succeeded && other error && window_exists  => PostFallback
//! /// !send_succeeded && other error && !window_exists => Done
//! fn close_followup(send_succeeded: bool, last_error: u32, window_exists: bool) -> CloseFollowup;
//! ```
//! `close_window_with_identity` must call `close_followup(` and only call
//! PostMessageW inside the `PostFallback` arm. Source bans live in
//! tests/tier0TaskCloseRequestOnly.test.mjs.
use super::*;

const ERROR_TIMEOUT: u32 = 1460;

#[test]
fn successful_send_is_done_regardless_of_window_state() {
    for exists in [true, false] {
        for err in [0, 5, ERROR_TIMEOUT] {
            assert_eq!(
                close_followup(true, err, exists),
                CloseFollowup::Done,
                "err {err} exists {exists}"
            );
        }
    }
}

#[test]
fn timeout_never_posts_a_second_close() {
    for exists in [true, false] {
        assert_eq!(
            close_followup(false, ERROR_TIMEOUT, exists),
            CloseFollowup::Done,
            "exists {exists}"
        );
    }
}

#[test]
fn real_send_failure_with_surviving_window_falls_back_to_post() {
    for err in [5u32, 1400, 0, 1] {
        assert_eq!(
            close_followup(false, err, true),
            CloseFollowup::PostFallback,
            "err {err}"
        );
    }
}

#[test]
fn real_send_failure_with_gone_window_is_done() {
    for err in [5u32, 1400, 0] {
        assert_eq!(close_followup(false, err, false), CloseFollowup::Done, "err {err}");
    }
}
