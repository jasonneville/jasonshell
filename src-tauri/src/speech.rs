use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const MAX_RECORDING_DURATION: Duration = Duration::from_secs(300);
pub const MAX_SPEECH_OPERATION_DURATION: Duration = Duration::from_secs(45);
pub const SPEECH_BUSY_ERROR: &str = "Speech is busy";
pub const SPEECH_STATE_ERROR: &str = "Speech request is no longer active";
pub const SPEECH_TIMEOUT_CODE: &str = "timeout";
pub const MAX_SPEECH_HISTORY_ENTRIES: usize = 5;
/// Maximum retained transcript bytes per process-session history entry.
pub const MAX_SPEECH_HISTORY_TRANSCRIPT_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SpeechSessionNonce(pub u64);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SpeechStatusKind {
    Idle,
    Recording,
    Transcribing,
    Copied,
    Error,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FinalizationReason {
    RecordingCap,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechStatusEvent {
    pub status: SpeechStatusKind,
    pub nonce: Option<SpeechSessionNonce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finalization_reason: Option<FinalizationReason>,
}

/// A privacy-preserving, normalized capture meter for the active speech session.
/// It deliberately contains no captured audio or transcription data.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechVoiceLevelEvent {
    pub nonce: SpeechSessionNonce,
    pub level: f32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechStatusResponse {
    pub status: SpeechStatusKind,
    pub nonce: Option<SpeechSessionNonce>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopSpeechCaptureRequest {
    pub nonce: SpeechSessionNonce,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSpeechCaptureRequest {
    pub reservation_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SpeechHotkeyActivation {
    Start {
        #[serde(rename = "reservationId")]
        reservation_id: u64,
    },
    Stop {
        nonce: SpeechSessionNonce,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopySpeechHistoryTranscriptRequest {
    pub nonce: SpeechSessionNonce,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSpeechCaptureResponse {
    pub nonce: SpeechSessionNonce,
    pub status: SpeechStatusKind,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechHistoryEntry {
    pub nonce: SpeechSessionNonce,
    pub transcript: String,
    pub outcome: String,
}

#[derive(Debug)]
enum Phase {
    Idle,
    Recording {
        nonce: SpeechSessionNonce,
        #[cfg(test)]
        started_at: Duration,
    },
    Transcribing {
        nonce: SpeechSessionNonce,
        session_started_at: Duration,
    },
    Terminal {
        nonce: SpeechSessionNonce,
        status: SpeechStatusKind,
    },
}

#[derive(Debug)]
pub struct SpeechController {
    phase: Phase,
    last_nonce: u64,
}

impl Default for SpeechController {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            last_nonce: 0,
        }
    }
}

impl SpeechController {
    pub fn start(&mut self, _now: Duration) -> Result<SpeechStatusEvent, &'static str> {
        if matches!(
            self.phase,
            Phase::Recording { .. } | Phase::Transcribing { .. }
        ) {
            return Err(SPEECH_BUSY_ERROR);
        }
        let Some(next_nonce) = self.last_nonce.checked_add(1) else {
            return Err(SPEECH_STATE_ERROR);
        };
        self.last_nonce = next_nonce;
        let nonce = SpeechSessionNonce(next_nonce);
        self.phase = Phase::Recording {
            nonce,
            #[cfg(test)]
            started_at: _now,
        };
        Ok(Self::event(SpeechStatusKind::Recording, Some(nonce), None))
    }

    pub fn stop(
        &mut self,
        nonce: SpeechSessionNonce,
        now: Duration,
    ) -> Result<SpeechStatusEvent, &'static str> {
        if !matches!(self.phase, Phase::Recording { nonce: current, .. } if current == nonce) {
            return Err(SPEECH_STATE_ERROR);
        }
        let Phase::Recording { .. } = self.phase else {
            return Err(SPEECH_STATE_ERROR);
        };
        self.phase = Phase::Transcribing {
            nonce,
            session_started_at: now,
        };
        Ok(Self::event(
            SpeechStatusKind::Transcribing,
            Some(nonce),
            None,
        ))
    }

    pub fn stop_for_recording_cap(
        &mut self,
        nonce: SpeechSessionNonce,
        now: Duration,
    ) -> Result<SpeechStatusEvent, &'static str> {
        let mut event = self.stop(nonce, now)?;
        event.finalization_reason = Some(FinalizationReason::RecordingCap);
        Ok(event)
    }

    #[cfg(test)]
    pub fn expire_recording(
        &mut self,
        nonce: SpeechSessionNonce,
        now: Duration,
    ) -> Option<SpeechStatusEvent> {
        let Phase::Recording {
            nonce: current,
            started_at,
        } = self.phase
        else {
            return None;
        };
        if current != nonce || now.saturating_sub(started_at) < MAX_RECORDING_DURATION {
            return None;
        }
        self.phase = Phase::Terminal {
            nonce,
            status: SpeechStatusKind::Error,
        };
        Some(Self::event(
            SpeechStatusKind::Error,
            Some(nonce),
            Some(SPEECH_TIMEOUT_CODE),
        ))
    }

    pub fn expire_transcription(
        &mut self,
        nonce: SpeechSessionNonce,
        now: Duration,
    ) -> Option<SpeechStatusEvent> {
        let Phase::Transcribing {
            nonce: current,
            session_started_at,
        } = self.phase
        else {
            return None;
        };
        if current != nonce
            || now.saturating_sub(session_started_at) < MAX_SPEECH_OPERATION_DURATION
        {
            return None;
        }
        self.phase = Phase::Terminal {
            nonce,
            status: SpeechStatusKind::Error,
        };
        Some(Self::event(
            SpeechStatusKind::Error,
            Some(nonce),
            Some(SPEECH_TIMEOUT_CODE),
        ))
    }

    pub fn complete_copied(
        &mut self,
        nonce: SpeechSessionNonce,
        now: Duration,
    ) -> Option<SpeechStatusEvent> {
        self.complete(nonce, now, SpeechStatusKind::Copied, None)
    }

    pub fn complete_error(
        &mut self,
        nonce: SpeechSessionNonce,
        now: Duration,
    ) -> Option<SpeechStatusEvent> {
        self.complete_error_code(nonce, now, "state-race")
    }

    pub fn complete_error_code(
        &mut self,
        nonce: SpeechSessionNonce,
        now: Duration,
        code: &'static str,
    ) -> Option<SpeechStatusEvent> {
        self.complete(nonce, now, SpeechStatusKind::Error, Some(code))
    }

    pub fn reset(&mut self, nonce: SpeechSessionNonce) -> Option<SpeechStatusEvent> {
        if !matches!(self.phase, Phase::Terminal { nonce: current, .. } if current == nonce) {
            return None;
        }
        self.phase = Phase::Idle;
        Some(Self::event(SpeechStatusKind::Idle, Some(nonce), None))
    }

    pub fn status(&self) -> SpeechStatusResponse {
        match self.phase {
            Phase::Idle => SpeechStatusResponse {
                status: SpeechStatusKind::Idle,
                nonce: None,
            },
            Phase::Recording { nonce, .. } => SpeechStatusResponse {
                status: SpeechStatusKind::Recording,
                nonce: Some(nonce),
            },
            Phase::Transcribing { nonce, .. } => SpeechStatusResponse {
                status: SpeechStatusKind::Transcribing,
                nonce: Some(nonce),
            },
            Phase::Terminal { nonce, status } => SpeechStatusResponse {
                status,
                nonce: Some(nonce),
            },
        }
    }

    pub fn can_complete(&self, nonce: SpeechSessionNonce, now: Duration) -> bool {
        matches!(
            self.phase,
            Phase::Transcribing { nonce: current, session_started_at }
                if current == nonce
                    && now.saturating_sub(session_started_at) < MAX_SPEECH_OPERATION_DURATION
        )
    }

    pub fn remaining_operation_time(
        &self,
        nonce: SpeechSessionNonce,
        now: Duration,
    ) -> Option<Duration> {
        let Phase::Transcribing {
            nonce: current,
            session_started_at,
        } = self.phase
        else {
            return None;
        };
        (current == nonce).then(|| {
            MAX_SPEECH_OPERATION_DURATION.saturating_sub(now.saturating_sub(session_started_at))
        })
    }

    fn complete(
        &mut self,
        nonce: SpeechSessionNonce,
        now: Duration,
        status: SpeechStatusKind,
        error: Option<&str>,
    ) -> Option<SpeechStatusEvent> {
        let Phase::Transcribing {
            nonce: current,
            session_started_at,
        } = self.phase
        else {
            return None;
        };
        if current != nonce {
            return None;
        }
        if now.saturating_sub(session_started_at) >= MAX_SPEECH_OPERATION_DURATION {
            self.phase = Phase::Terminal {
                nonce,
                status: SpeechStatusKind::Error,
            };
            return Some(Self::event(
                SpeechStatusKind::Error,
                Some(nonce),
                Some(SPEECH_TIMEOUT_CODE),
            ));
        }
        self.phase = Phase::Terminal { nonce, status };
        Some(Self::event(status, Some(nonce), error))
    }

    fn event(
        status: SpeechStatusKind,
        nonce: Option<SpeechSessionNonce>,
        error: Option<&str>,
    ) -> SpeechStatusEvent {
        SpeechStatusEvent {
            status,
            nonce,
            error: error.map(str::to_owned),
            finalization_reason: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    #[test]
    fn start_and_matched_stop_advance_lifecycle() {
        let mut controller = SpeechController::default();
        let recording = controller.start(at(1)).expect("idle start should succeed");
        assert_eq!(recording.status, SpeechStatusKind::Recording);
        assert_eq!(recording.nonce, Some(SpeechSessionNonce(1)));

        let transcribing = controller
            .stop(SpeechSessionNonce(1), at(2))
            .expect("matched stop should succeed");
        assert_eq!(transcribing.status, SpeechStatusKind::Transcribing);
    }

    #[test]
    fn duplicate_start_and_stop_are_rejected() {
        let mut controller = SpeechController::default();
        controller
            .start(at(1))
            .expect("initial start should succeed");
        assert_eq!(controller.start(at(2)), Err(SPEECH_BUSY_ERROR));
        controller
            .stop(SpeechSessionNonce(1), at(3))
            .expect("initial stop should succeed");
        assert_eq!(
            controller.stop(SpeechSessionNonce(1), at(4)),
            Err(SPEECH_STATE_ERROR)
        );
    }

    #[test]
    fn stale_nonce_cannot_stop_complete_or_reset_current_session() {
        let mut controller = SpeechController::default();
        controller.start(at(1)).expect("first start should succeed");
        controller
            .stop(SpeechSessionNonce(1), at(2))
            .expect("first stop should succeed");
        controller
            .complete_error(SpeechSessionNonce(1), at(2))
            .expect("first completion should succeed");
        let current = controller
            .start(at(3))
            .expect("terminal start should reset");
        assert_eq!(current.nonce, Some(SpeechSessionNonce(2)));

        assert_eq!(
            controller.complete_copied(SpeechSessionNonce(1), at(4)),
            None
        );
        assert_eq!(controller.reset(SpeechSessionNonce(1)), None);
        assert_eq!(
            controller.stop(SpeechSessionNonce(1), at(4)),
            Err(SPEECH_STATE_ERROR)
        );
        assert_eq!(controller.status().status, SpeechStatusKind::Recording);
    }

    #[test]
    fn recording_expires_at_three_hundred_seconds_for_matching_session() {
        let mut controller = SpeechController::default();
        let nonce = SpeechSessionNonce(1);
        controller.start(at(5)).expect("start should succeed");
        assert_eq!(
            controller.expire_recording(
                nonce,
                Duration::from_secs(5) + MAX_RECORDING_DURATION - Duration::from_millis(1)
            ),
            None
        );
        let expired = controller
            .expire_recording(nonce, at(305))
            .expect("recording should expire at cap");
        assert_eq!(expired.status, SpeechStatusKind::Error);
        assert_eq!(expired.error.as_deref(), Some(SPEECH_TIMEOUT_CODE));
    }

    #[test]
    fn transcription_expires_at_operation_limit_and_rejects_late_completion() {
        let mut controller = SpeechController::default();
        let nonce = SpeechSessionNonce(1);
        controller.start(at(1)).expect("start should succeed");
        controller.stop(nonce, at(2)).expect("stop should succeed");

        assert_eq!(controller.expire_transcription(nonce, at(46)), None);
        let expired = controller
            .expire_transcription(nonce, at(47))
            .expect("transcription should expire at cap");
        assert_eq!(expired.status, SpeechStatusKind::Error);
        assert_eq!(expired.nonce, Some(nonce));
        assert_eq!(expired.error.as_deref(), Some(SPEECH_TIMEOUT_CODE));
        assert_eq!(controller.complete_copied(nonce, at(47)), None);
        assert_eq!(controller.complete_error(nonce, at(47)), None);
    }

    #[test]
    fn transcription_expiry_ignores_stale_nonce() {
        let mut controller = SpeechController::default();
        controller.start(at(1)).expect("start should succeed");
        controller
            .stop(SpeechSessionNonce(1), at(2))
            .expect("stop should succeed");

        assert_eq!(
            controller.expire_transcription(SpeechSessionNonce(2), at(47)),
            None
        );
        assert_eq!(controller.status().status, SpeechStatusKind::Transcribing);
    }

    #[test]
    fn transcription_completion_fails_closed_at_exact_limit_before_expiry_task() {
        let mut controller = SpeechController::default();
        let nonce = SpeechSessionNonce(1);
        controller.start(at(1)).expect("start should succeed");
        controller.stop(nonce, at(2)).expect("stop should succeed");

        let completed = controller
            .complete_copied(nonce, at(47))
            .expect("matched completion should terminate with error");
        assert_eq!(completed.status, SpeechStatusKind::Error);
        assert_eq!(completed.error.as_deref(), Some(SPEECH_TIMEOUT_CODE));
        assert_eq!(controller.expire_transcription(nonce, at(47)), None);
    }

    #[test]
    fn transcription_completion_before_limit_can_copy() {
        let mut controller = SpeechController::default();
        let nonce = SpeechSessionNonce(1);
        controller.start(at(1)).expect("start should succeed");
        controller.stop(nonce, at(2)).expect("stop should succeed");

        let completed = controller
            .complete_copied(nonce, at(46))
            .expect("completion before cap should succeed");
        assert_eq!(completed.status, SpeechStatusKind::Copied);
        assert_eq!(completed.error, None);
    }

    #[test]
    fn finalization_limit_is_measured_from_input_close() {
        let mut controller = SpeechController::default();
        let nonce = SpeechSessionNonce(1);
        controller.start(at(1)).expect("start should succeed");
        controller
            .stop(nonce, at(300))
            .expect("stop should succeed");

        assert!(controller.can_complete(nonce, at(344)));
        assert!(!controller.can_complete(nonce, at(345)));
        let expired = controller
            .expire_transcription(nonce, at(345))
            .expect("session should expire 45 seconds after input close");
        assert_eq!(expired.status, SpeechStatusKind::Error);
    }

    #[test]
    fn completion_authorization_rejects_stale_and_expired_work() {
        let mut controller = SpeechController::default();
        controller.start(at(1)).expect("start should succeed");
        controller
            .stop(SpeechSessionNonce(1), at(2))
            .expect("stop should succeed");
        assert!(controller.can_complete(SpeechSessionNonce(1), at(46)));
        assert!(!controller.can_complete(SpeechSessionNonce(2), at(46)));
        assert!(!controller.can_complete(SpeechSessionNonce(1), at(47)));
    }

    #[test]
    fn matched_terminal_completion_can_reset_and_stale_completion_is_ignored() {
        let mut controller = SpeechController::default();
        controller.start(at(1)).expect("start should succeed");
        controller
            .stop(SpeechSessionNonce(1), at(2))
            .expect("stop should succeed");
        let copied = controller
            .complete_copied(SpeechSessionNonce(1), at(46))
            .expect("completion should succeed");
        assert_eq!(copied.status, SpeechStatusKind::Copied);
        let idle = controller
            .reset(SpeechSessionNonce(1))
            .expect("matched terminal reset should succeed");
        assert_eq!(idle.status, SpeechStatusKind::Idle);
        assert_eq!(idle.nonce, Some(SpeechSessionNonce(1)));
        assert_eq!(
            controller.complete_error(SpeechSessionNonce(1), at(122)),
            None
        );
    }

    #[test]
    fn statuses_serialize_to_exact_public_strings() {
        let statuses = [
            SpeechStatusKind::Idle,
            SpeechStatusKind::Recording,
            SpeechStatusKind::Transcribing,
            SpeechStatusKind::Copied,
            SpeechStatusKind::Error,
        ];
        assert_eq!(
            serde_json::to_string(&statuses).expect("statuses should serialize"),
            r#"["idle","recording","transcribing","copied","error"]"#
        );
    }
}
