//! Test-only RB-02 bridge into canonical `stack-text-editor.v2` validators.
//! Not a command, actor, provider, or production adapter.

use super::contract::{
    DecimalU64, LineBreakKind, LineCertainty, SourceState, TextEncoding, ViewLease,
};
use super::scheduler::CancelOutcome;
use crate::stack_popup::text_document::protocol as v2;

#[derive(Clone, Copy)]
pub(super) struct Context<'a> {
    pub before: &'a str,
    pub after: &'a str,
    pub continuation_id: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    LegacySourceOutcomeHasNoV2LeaseEquivalent(SourceState),
    LegacySourceOutcomeHasNoV2ErrorEquivalent {
        source_state: SourceState,
        invalid_at: Option<u64>,
    },
    InvalidLegacyLease,
    BoundaryMismatch,
    Canonical(v2::ErrorCode),
}

pub(super) fn source_error(
    source_state: SourceState,
    invalid_at: Option<&DecimalU64>,
) -> Result<v2::ErrorCode, Refusal> {
    match source_state {
        SourceState::Cancelled => Ok(v2::ErrorCode::Cancelled),
        SourceState::QuotaExceeded => Ok(v2::ErrorCode::ResourceLimit),
        SourceState::Conflict => Ok(v2::ErrorCode::SharingViolation),
        SourceState::ReadLimited => Ok(v2::ErrorCode::IoFailure),
        SourceState::Changed => Ok(v2::ErrorCode::SourceChanged),
        SourceState::Readonly => Ok(v2::ErrorCode::Readonly),
        SourceState::Opening
        | SourceState::Snapshotting
        | SourceState::Ready
        | SourceState::DecisionRequired => {
            Err(Refusal::LegacySourceOutcomeHasNoV2ErrorEquivalent {
                source_state,
                invalid_at: invalid_at.map(DecimalU64::as_u64),
            })
        }
    }
}

pub(super) fn lease(
    old: &ViewLease,
    encoding: TextEncoding,
    context: Context<'_>,
) -> Result<v2::Lease, Refusal> {
    old.validate().map_err(|_| Refusal::InvalidLegacyLease)?;
    if old.source_state != SourceState::Ready || old.invalid_at.is_some() {
        return Err(Refusal::LegacySourceOutcomeHasNoV2LeaseEquivalent(
            old.source_state,
        ));
    }
    let encoding = match encoding {
        TextEncoding::Utf8 | TextEncoding::Utf8Bom => v2::Encoding::Utf8,
        TextEncoding::Utf16Le => v2::Encoding::Utf16le,
        TextEncoding::Utf16Be => v2::Encoding::Utf16be,
    };
    let segments = old
        .segments
        .iter()
        .map(|segment| v2::Segment {
            encoding,
            byte_start: segment.source_byte_start.as_u64().to_string(),
            text: segment.text.clone(),
            newlines: segment
                .line_breaks
                .iter()
                .map(|line| v2::Newline {
                    local_offset: line.local_utf16_offset - segment.local_utf16_start,
                    kind: match line.kind {
                        LineBreakKind::Lf => v2::Eol::Lf,
                        LineBreakKind::Cr => v2::Eol::Cr,
                        LineBreakKind::Crlf => v2::Eol::Crlf,
                    },
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    for (old_segment, segment) in old.segments.iter().zip(&segments) {
        let rebuilt = v2::segment_boundaries(segment).map_err(Refusal::Canonical)?;
        if old_segment.boundaries.iter().any(|boundary| {
            rebuilt.get(&(boundary.local_utf16_offset - old_segment.local_utf16_start))
                != Some(&boundary.source_byte_offset.as_u64())
        }) || rebuilt.last_key_value().map(|(_, byte)| *byte)
            != Some(old_segment.source_byte_end.as_u64())
        {
            return Err(Refusal::BoundaryMismatch);
        }
    }
    let result = v2::Lease {
        schema: v2::SCHEMA.into(),
        session_id: old.session_id.clone(),
        source_generation: old.source_generation.as_u64().to_string(),
        document_revision: old.document_revision.as_u64().to_string(),
        view_generation: old.view_generation.as_u64().to_string(),
        lease_id: old.lease_id.clone(),
        segments,
        line: v2::Line {
            state: match old.line_certainty {
                LineCertainty::Exact => "exact",
                LineCertainty::Unknown => "unknown",
                LineCertainty::Indexing => "indexing",
            }
            .into(),
            count: match old.line_certainty {
                LineCertainty::Exact => Some(
                    old.segments
                        .iter()
                        .map(|segment| segment.line_breaks.len() as u64)
                        .sum::<u64>()
                        .saturating_add(1)
                        .to_string(),
                ),
                _ => None,
            },
        },
        context: v2::Context {
            before: context.before.into(),
            after: context.after.into(),
            continuation_id: context.continuation_id.map(Into::into),
        },
    };
    v2::validate_lease(&result).map_err(Refusal::Canonical)?;
    Ok(result)
}

pub(super) fn failure_result(
    code: v2::ErrorCode,
    retry: &str,
    disposition: &str,
) -> serde_json::Value {
    serde_json::json!({"schema":v2::SCHEMA,"requestId":"rb02","sessionId":null,"sourceGeneration":null,
        "kind":"error","data":{"code":code,"retry":retry,"disposition":disposition,"operationId":null,"jobId":null}})
}

pub(super) fn job_result(phase: &str, cancellation: &str) -> serde_json::Value {
    serde_json::json!({"schema":v2::SCHEMA,"requestId":"rb02","sessionId":"s1","sourceGeneration":"1",
        "kind":"job","data":{"jobId":"j1","sessionId":"s1","sourceGeneration":"1","documentRevision":"0",
        "phase":phase,"completedBytes":"0","totalBytes":"1","cancellation":cancellation}})
}

pub(super) fn cancellation_result(outcome: CancelOutcome) -> serde_json::Value {
    match outcome {
        CancelOutcome::QueuedRemoved => job_result("cancelled", "removed"),
        CancelOutcome::RequestedDriverDependent => job_result("reading", "requested"),
        CancelOutcome::FinishedOrUnknown => {
            failure_result(v2::ErrorCode::PublicationAmbiguous, "never", "ambiguous")
        }
    }
}
