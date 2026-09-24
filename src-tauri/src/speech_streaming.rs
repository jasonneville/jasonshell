//! Bounded coordinator primitives for the local Parakeet TDT fallback.
//!
//! TDT calls are independent finite-window inference. This module deliberately does not
//! describe them as stateful model streaming.

use crate::speech::SpeechSessionNonce;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub(crate) const SEGMENT_SAMPLES: usize = 2_560;
pub(crate) const SEGMENT_QUEUE_CAPACITY: usize = 32;
pub(crate) const RECORDING_CAP: Duration = Duration::from_secs(300);
pub(crate) const FINALIZATION_TIMEOUT: Duration = Duration::from_secs(45);
pub(crate) const WINDOW_SEGMENTS: usize = 8;
pub(crate) const OVERLAP_SEGMENTS: usize = 2;

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
    input_closed: AtomicBool,
    close_reason: AtomicU64,
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
                input_closed: AtomicBool::new(false),
                close_reason: AtomicU64::new(0),
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
        if self.input_closed.load(Ordering::Acquire) {
            return IntakeResult::Rejected(self.reason().unwrap_or(InputCloseReason::SafeFailure));
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
        if self.input_closed.load(Ordering::Acquire)
            && self.reason() != Some(InputCloseReason::RecordingCap)
        {
            return IntakeResult::Rejected(self.reason().unwrap_or(InputCloseReason::SafeFailure));
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
            .input_closed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false;
        }
        self.close_reason
            .store(reason_code(reason), Ordering::Release);
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
        self.input_closed.load(Ordering::Acquire)
    }

    pub(crate) fn recv_until_closed(
        &self,
        receiver: &Receiver<WorkerMessage>,
    ) -> Option<WorkerMessage> {
        // Receiver::recv_timeout periodically observes close_signal (`input_closed`),
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
        code_reason(self.close_reason.load(Ordering::Acquire))
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

/// Conservative exact-token overlap merge. Ambiguous boundaries are retained.
pub(crate) fn merge_tdt_text(accumulated: &mut String, next: &str) {
    let next = next.trim();
    if next.is_empty() {
        return;
    }
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
                .all(|(left, right)| left.eq_ignore_ascii_case(right))
        })
        .unwrap_or(0);
    let suffix = next_words[overlap..].join(" ");
    if !suffix.is_empty() {
        accumulated.push(' ');
        accumulated.push_str(&suffix);
    }
}
