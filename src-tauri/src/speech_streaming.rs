//! Bounded coordinator primitives for the local Parakeet TDT fallback.
//!
//! TDT calls are independent finite-window inference. This module deliberately does not
//! describe them as stateful model streaming.

use crate::speech::SpeechSessionNonce;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub(crate) const SEGMENT_SAMPLES: usize = 2_560;
pub(crate) const SEGMENT_QUEUE_CAPACITY: usize = 32;
pub(crate) const RECORDING_CAP: Duration = Duration::from_secs(300);
pub(crate) const FINALIZATION_TIMEOUT: Duration = Duration::from_secs(45);
// 64 x 160 ms = 10.24 s finite context. The 2.56 s overlap remains bounded.
pub(crate) const WINDOW_SEGMENTS: usize = 64;
pub(crate) const OVERLAP_SEGMENTS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InputCloseReason {
    Finish,
    RecordingCap,
    SafeFailure,
    Cancel,
    Shutdown,
}

#[derive(Debug)]
pub(crate) enum WorkerMessage {
    Segment(Box<[f32]>),
    Close(InputCloseReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IntakeResult {
    Accepted,
    Rejected(InputCloseReason),
    QueueFull,
}

pub(crate) struct BoundedIntake {
    sender: SyncSender<WorkerMessage>,
    close_state: AtomicU64,
    generation: u64,
    nonce: SpeechSessionNonce,
    activation_deadline: OnceLock<Instant>,
}

impl BoundedIntake {
    pub(crate) fn prepared(
        nonce: SpeechSessionNonce,
        generation: u64,
    ) -> (Self, Receiver<WorkerMessage>) {
        let (sender, receiver) = mpsc::sync_channel(SEGMENT_QUEUE_CAPACITY);
        (
            Self {
                sender,
                close_state: AtomicU64::new(0),
                generation,
                nonce,
                activation_deadline: OnceLock::new(),
            },
            receiver,
        )
    }

    pub(crate) fn activate(&self, activated_at: Instant) -> Result<(), ()> {
        self.activation_deadline
            .set(activated_at + RECORDING_CAP)
            .map_err(|_| ())
    }

    pub(crate) fn try_send(&self, now: Instant, segment: Box<[f32]>) -> IntakeResult {
        if let Some(deadline) = self.activation_deadline.get().copied() {
            if now >= deadline {
                // A completed 160 ms segment may straddle the cap. Preserve only its
                // accepted_prefix; samples at/after the deadline are never queued.
                let segment_duration = Duration::from_secs_f64(segment.len() as f64 / 16_000_f64);
                let started_at = now.checked_sub(segment_duration).unwrap_or(now);
                let samples_before_deadline = deadline
                    .saturating_duration_since(started_at)
                    .as_secs_f64()
                    .mul_add(16_000_f64, 0.0)
                    as usize;
                let accepted_prefix = samples_before_deadline.min(segment.len());
                if accepted_prefix > 0 {
                    let prefix = segment[..accepted_prefix].to_vec().into_boxed_slice();
                    if !matches!(self.try_send_segment(prefix), IntakeResult::Accepted) {
                        return IntakeResult::QueueFull;
                    }
                }
                let _ = self.first_close(InputCloseReason::RecordingCap);
                return IntakeResult::Rejected(InputCloseReason::RecordingCap);
            }
        }
        if let Some(reason) = self.reason() {
            return IntakeResult::Rejected(reason);
        }
        self.try_send_segment(segment)
    }

    fn try_send_segment(&self, segment: Box<[f32]>) -> IntakeResult {
        match self.sender.try_send(WorkerMessage::Segment(segment)) {
            Ok(()) => IntakeResult::Accepted,
            Err(TrySendError::Full(_)) => {
                let _ = self.first_close(InputCloseReason::SafeFailure);
                IntakeResult::QueueFull
            }
            Err(TrySendError::Disconnected(_)) => {
                let _ = self.first_close(InputCloseReason::SafeFailure);
                IntakeResult::Rejected(InputCloseReason::SafeFailure)
            }
        }
    }

    /// Callback entrypoint: samples current monotonic time without waiting.
    pub(crate) fn try_send_now(&self, segment: Box<[f32]>) -> IntakeResult {
        self.try_send(Instant::now(), segment)
    }

    /// Hands off a bounded pre-close tail after capture has stopped.
    pub(crate) fn try_send_tail(&self, segment: Box<[f32]>) -> IntakeResult {
        if let Some(reason) = self.reason() {
            if reason != InputCloseReason::RecordingCap {
                return IntakeResult::Rejected(reason);
            }
        }
        match self.sender.try_send(WorkerMessage::Segment(segment)) {
            Ok(()) => IntakeResult::Accepted,
            Err(TrySendError::Full(_)) => {
                let _ = self.first_close(InputCloseReason::SafeFailure);
                IntakeResult::QueueFull
            }
            Err(TrySendError::Disconnected(_)) => {
                let _ = self.first_close(InputCloseReason::SafeFailure);
                IntakeResult::Rejected(InputCloseReason::SafeFailure)
            }
        }
    }

    pub(crate) fn first_close(&self, reason: InputCloseReason) -> bool {
        if self
            .close_state
            .compare_exchange(0, reason_code(reason), Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false;
        }
        true
    }

    pub(crate) fn can_commit(&self, nonce: SpeechSessionNonce, generation: u64) -> bool {
        self.nonce == nonce
            && self.generation == generation
            && matches!(
                self.reason(),
                Some(InputCloseReason::Finish | InputCloseReason::RecordingCap)
            )
    }

    pub(crate) fn activation_deadline(&self) -> Option<Instant> {
        self.activation_deadline.get().copied()
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.close_state.load(Ordering::Acquire) != 0
    }

    pub(crate) fn recv_until_closed(
        &self,
        receiver: &Receiver<WorkerMessage>,
    ) -> Option<WorkerMessage> {
        // Receiver::recv_timeout periodically observes the out-of-band close state,
        // independent of bounded work-queue capacity.
        loop {
            match receiver.recv_timeout(Duration::from_millis(10)) {
                Ok(message) => return Some(message),
                Err(mpsc::RecvTimeoutError::Timeout) if self.is_closed() => return None,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return None,
            }
        }
    }

    fn reason(&self) -> Option<InputCloseReason> {
        code_reason(self.close_state.load(Ordering::Acquire))
    }
}

fn reason_code(reason: InputCloseReason) -> u64 {
    match reason {
        InputCloseReason::Finish => 1,
        InputCloseReason::RecordingCap => 2,
        InputCloseReason::SafeFailure => 3,
        InputCloseReason::Cancel => 4,
        InputCloseReason::Shutdown => 5,
    }
}

fn code_reason(code: u64) -> Option<InputCloseReason> {
    match code {
        1 => Some(InputCloseReason::Finish),
        2 => Some(InputCloseReason::RecordingCap),
        3 => Some(InputCloseReason::SafeFailure),
        4 => Some(InputCloseReason::Cancel),
        5 => Some(InputCloseReason::Shutdown),
        _ => None,
    }
}

/// Conservative overlap merge. Only token-boundary punctuation is ignored for matching.
pub(crate) fn merge_tdt_text(accumulated: &mut String, next: &str) {
    merge_tdt_window(accumulated, next, true);
}

pub(crate) fn merge_tdt_window(accumulated: &mut String, next: &str, final_window: bool) {
    let next = next.trim();
    if next.is_empty() {
        return;
    }
    let next = if final_window {
        next
    } else {
        next.trim_end_matches(['.', '?', '!'])
    };
    if accumulated.is_empty() {
        accumulated.push_str(next);
        return;
    }
    let prior_words: Vec<&str> = accumulated.split_whitespace().collect();
    let next_words: Vec<&str> = next.split_whitespace().collect();
    let overlap = (1..=prior_words.len().min(next_words.len()).min(16))
        .rev()
        .find(|count| {
            prior_words[prior_words.len() - count..]
                .iter()
                .zip(&next_words[..*count])
                .all(|(left, right)| {
                    normalized_boundary_token(left)
                        .eq_ignore_ascii_case(normalized_boundary_token(right))
                })
        })
        .unwrap_or(0);
    let suffix = next_words[overlap..].join(" ");
    if !suffix.is_empty() {
        accumulated.push(' ');
        accumulated.push_str(&suffix);
    } else if final_window {
        if let Some(punctuation @ ('.' | '?' | '!')) = next.chars().last() {
            if !accumulated.ends_with(['.', '?', '!']) {
                accumulated.push(punctuation);
            }
        }
    }
}

fn normalized_boundary_token(token: &str) -> &str {
    token.trim_end_matches(['.', ',', '?', '!', ';', ':'])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_tdt_text_matches_punctuation_variant_overlap_without_synthetic_period() {
        let mut transcript = String::from("please fix this");

        merge_tdt_text(&mut transcript, "this. I need help");

        assert_eq!(
            transcript, "please fix this I need help",
            "TDT window merge must match overlap punctuation-insensitively, keep one `this`, and not preserve/synthesize an interior boundary period"
        );
    }

    #[test]
    fn merge_tdt_text_matches_case_and_punctuation_variant_phrase_overlap() {
        let mut transcript = String::from("open the terminal and run the tests");

        merge_tdt_text(&mut transcript, "Run, the tests before commit");

        assert_eq!(
            transcript, "open the terminal and run the tests before commit",
            "TDT merge must normalize punctuation/case for overlap detection so boundary phrases are not duplicated"
        );
    }

    #[test]
    fn merge_tdt_text_does_not_false_overlap_distinct_technical_tokens() {
        let mut transcript = String::from("use C++");

        merge_tdt_text(&mut transcript, "C# next");

        assert_eq!(
            transcript, "use C++ C# next",
            "TDT merge must not strip semantic punctuation from technical tokens like C++ and C# into the same overlap key"
        );
    }
}
