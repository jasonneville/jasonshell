//! Stack text editor protocol v1.
//!
//! This boundary contains no document storage, Tauri commands, or CodeMirror
//! dependency. It is the shared lossless schema and validation layer for the
//! future document actor and Stack Browser controller. File-sized counters are
//! decimal strings on the JSON wire; local offsets are bounded UTF-16 units in
//! one validated lease.

use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const SCHEMA_REVISION: &str = "stack-text-editor.v1";
/// P02-only additive lease fields are identified separately from the frozen v1 wire.
pub const P02_FEASIBILITY_SCHEMA_REVISION: &str = "stack-text-editor.p02-feasibility.v2";

pub const MAX_SESSION_ID_BYTES: usize = 96;
pub const MAX_SOURCE_GENERATION_BYTES: usize = 32;
pub const MAX_VIEW_GENERATION_BYTES: usize = 32;
pub const MAX_LEASE_ID_BYTES: usize = 96;
pub const MAX_SELECTION_ID_BYTES: usize = 96;
pub const MAX_ANCHOR_ID_BYTES: usize = 96;
pub const MAX_OPERATION_ID_BYTES: usize = 96;
pub const MAX_JOB_ID_BYTES: usize = 96;
pub const MAX_SPOOL_ID_BYTES: usize = 96;
pub const MAX_REQUEST_ID_BYTES: usize = 96;
pub const MAX_PATH_BYTES: usize = 32 * 1024;
pub const MAX_QUERY_BYTES: usize = 16 * 1024;
pub const MAX_INLINE_TEXT_BYTES: usize = 1024 * 1024;
pub const FIRST_READ_BYTES: usize = 64 * 1024;
pub const IO_BLOCK_BYTES: usize = 256 * 1024;
pub const PROJECTION_UTF16_UNITS: usize = 128 * 1024;
pub const PROJECTION_SEGMENTS: usize = 2048;
pub const PROJECTION_ROWS: usize = 4096;
pub const MAX_LEASE_BOUNDARIES: usize = 128 * 1024 + 2048;
pub const MAX_LEASE_BYTES: usize = 1024 * 1024;
pub const MAX_IN_FLIGHT_RANGE_PAYLOADS: usize = 4;
pub const MAX_PENDING_EDIT_BYTES: usize = 1024 * 1024;
pub const MAX_SPECULATIVE_OPERATIONS: usize = 64;
pub const MAX_SELECTIONS_PER_SESSION: usize = 64;
pub const MAX_LEASES_PER_SESSION: usize = 8;
pub const MAX_JOBS_PER_SESSION: usize = 32;
pub const MAX_JOB_EVENTS_PER_SESSION: usize = 256;
pub const MAX_RETAINED_OPERATIONS: usize = 1024;
pub const MAX_RETIRED_OPERATIONS: usize = 1024;
pub const MAX_RESULT_ITEMS: usize = 256;
pub const MAX_OPERATION_IDS_IN_BARRIER: usize = 64;
pub const MAX_SPOOL_BYTES_PER_SESSION: u64 = 64 * 1024 * 1024;
pub const MAX_SPOOL_BYTES_GLOBAL: u64 = 256 * 1024 * 1024;
pub const SELECTION_RETIREMENT_REVISION_WINDOW: u64 = 256;
pub const MAX_ERROR_MESSAGE_BYTES: usize = 512;

const U64_MAX: u128 = u64::MAX as u128;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecimalU64(String);

impl DecimalU64 {
    pub fn parse(value: &str) -> Result<Self, String> {
        if value.is_empty()
            || (value.len() > 1 && value.starts_with('0'))
            || !value.bytes().all(|byte| byte.is_ascii_digit())
            || value.len() > 20
        {
            return Err("value must be a canonical unsigned decimal string".to_string());
        }
        let parsed = value
            .parse::<u128>()
            .map_err(|_| "value is not a decimal integer".to_string())?;
        if parsed > U64_MAX {
            return Err("value exceeds u64".to_string());
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
            .parse::<u64>()
            .expect("DecimalU64 invariant validated at construction")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<u64> for DecimalU64 {
    fn from(value: u64) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for DecimalU64 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for DecimalU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for DecimalU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum ProtocolErrorCode {
    Unauthorized,
    UnsupportedTarget,
    EncodingRequired,
    InvalidTextBoundary,
    StaleRevision,
    SourceChanged,
    SharingViolation,
    Readonly,
    ResourceLimit,
    Cancelled,
    IoFailure,
    PublicationAmbiguous,
    AlreadyPublished,
    OperationIdConflict,
    OperationIdRetired,
    SelectionExpired,
    ClipboardUnavailable,
    JobNotFound,
    InvalidSchema,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolError {
    pub code: ProtocolErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<DecimalU64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_revision: Option<DecimalU64>,
}

fn protocol_error(
    code: ProtocolErrorCode,
    message: impl Into<String>,
    field: Option<&str>,
) -> ProtocolError {
    ProtocolError {
        code,
        message: message.into(),
        retryable: matches!(
            code,
            ProtocolErrorCode::Cancelled | ProtocolErrorCode::IoFailure
        ),
        field: field.map(str::to_string),
        operation_id: None,
        job_id: None,
        expected_revision: None,
        actual_revision: None,
    }
}

fn invalid(message: impl Into<String>, field: &str) -> ProtocolError {
    protocol_error(ProtocolErrorCode::InvalidSchema, message, Some(field))
}

fn validate_decimal(value: &DecimalU64, field: &str) -> Result<(), ProtocolError> {
    DecimalU64::parse(value.as_str())
        .map(|_| ())
        .map_err(|error| invalid(error, field))
}

fn validate_id(value: &str, field: &str, maximum_bytes: usize) -> Result<(), ProtocolError> {
    if value.is_empty() || value.len() > maximum_bytes {
        return Err(invalid(
            format!("{field} has an invalid bounded length"),
            field,
        ));
    }
    if !value.bytes().enumerate().all(|(index, byte)| {
        byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'.' | b'_' | b':' | b'-'))
    }) {
        return Err(invalid(
            format!("{field} has invalid identifier characters"),
            field,
        ));
    }
    Ok(())
}

fn validate_text(value: &str, field: &str, maximum_bytes: usize) -> Result<(), ProtocolError> {
    if value.is_empty() || value.len() > maximum_bytes {
        return Err(invalid(
            format!("{field} has an invalid bounded length"),
            field,
        ));
    }
    Ok(())
}

fn validate_bounded_text(
    value: &str,
    field: &str,
    maximum_bytes: usize,
) -> Result<(), ProtocolError> {
    if value.len() > maximum_bytes {
        return Err(protocol_error(
            ProtocolErrorCode::ResourceLimit,
            format!("{field} exceeds its bounded size"),
            Some(field),
        ));
    }
    Ok(())
}

fn contains_lf_at_utf16_offset(text: &str, offset: usize) -> bool {
    let mut cursor = 0usize;
    for character in text.chars() {
        if cursor == offset {
            return character == '\n';
        }
        cursor += character.len_utf16();
    }
    false
}

fn is_utf16_boundary(text: &str, offset: usize) -> bool {
    let mut cursor = 0usize;
    for character in text.chars() {
        if cursor == offset {
            return true;
        }
        cursor += character.len_utf16();
    }
    cursor == offset
}

fn validate_local_offset(value: usize, field: &str) -> Result<(), ProtocolError> {
    if value > PROJECTION_UTF16_UNITS {
        return Err(protocol_error(
            ProtocolErrorCode::InvalidTextBoundary,
            format!("{field} is outside the projection"),
            Some(field),
        ));
    }
    Ok(())
}

fn validate_sha256(value: &str, field: &str) -> Result<(), ProtocolError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(invalid(
            format!("{field} must be lowercase SHA-256 hex"),
            field,
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionState {
    pub accepted_revision: DecimalU64,
    pub durable_revision: Option<DecimalU64>,
    pub saved_revision: Option<DecimalU64>,
}

impl RevisionState {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_decimal(&self.accepted_revision, "acceptedRevision")?;
        if let Some(value) = &self.durable_revision {
            validate_decimal(value, "durableRevision")?;
            if value.as_u64() > self.accepted_revision.as_u64() {
                return Err(invalid(
                    "durableRevision cannot exceed acceptedRevision",
                    "durableRevision",
                ));
            }
        }
        if let Some(value) = &self.saved_revision {
            validate_decimal(value, "savedRevision")?;
            if value.as_u64() > self.accepted_revision.as_u64() {
                return Err(invalid(
                    "savedRevision cannot exceed acceptedRevision",
                    "savedRevision",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionEnvelope {
    pub schema_revision: String,
    pub session_id: String,
    pub source_generation: DecimalU64,
    pub document_revision: DecimalU64,
    pub view_generation: DecimalU64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lease_id: Option<String>,
}

impl SessionEnvelope {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.schema_revision != SCHEMA_REVISION {
            return Err(invalid("unsupported schemaRevision", "schemaRevision"));
        }
        validate_id(&self.session_id, "sessionId", MAX_SESSION_ID_BYTES)?;
        validate_decimal(&self.source_generation, "sourceGeneration")?;
        validate_decimal(&self.document_revision, "documentRevision")?;
        validate_decimal(&self.view_generation, "viewGeneration")?;
        if let Some(value) = &self.lease_id {
            validate_id(value, "leaseId", MAX_LEASE_ID_BYTES)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputBarrier {
    pub accepted_revision: DecimalU64,
    pub local_input_sequence: DecimalU64,
    pub operation_ids: Vec<String>,
}

impl InputBarrier {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_decimal(&self.accepted_revision, "acceptedRevision")?;
        validate_decimal(&self.local_input_sequence, "localInputSequence")?;
        if self.operation_ids.len() > MAX_OPERATION_IDS_IN_BARRIER {
            return Err(protocol_error(
                ProtocolErrorCode::ResourceLimit,
                "preceding input barrier is full",
                Some("operationIds"),
            ));
        }
        for operation_id in &self.operation_ids {
            validate_id(operation_id, "operationId", MAX_OPERATION_ID_BYTES)?;
        }
        Ok(())
    }
    pub fn validate_state(
        &self,
        accepted: &DecimalU64,
        local_sequence: &DecimalU64,
        acknowledged_ids: &BTreeSet<String>,
    ) -> Result<(), ProtocolError> {
        self.validate()?;
        if &self.accepted_revision != accepted
            || &self.local_input_sequence != local_sequence
            || self
                .operation_ids
                .iter()
                .any(|id| !acknowledged_ids.contains(id))
        {
            return Err(protocol_error(
                ProtocolErrorCode::StaleRevision,
                "visible input has not crossed the accepted snapshot barrier",
                None,
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseBoundary {
    pub local_utf16_offset: usize,
    pub source_byte_offset: DecimalU64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineBreakKind {
    Lf,
    Crlf,
    Cr,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineBreakSpan {
    pub local_utf16_offset: usize,
    pub source_byte_start: DecimalU64,
    pub source_byte_length: u8,
    pub kind: LineBreakKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ViewLeaseSegment {
    pub segment_id: String,
    pub source_byte_start: DecimalU64,
    pub source_byte_end: DecimalU64,
    pub local_utf16_start: usize,
    pub local_utf16_end: usize,
    pub text: String,
    pub boundaries: Vec<LeaseBoundary>,
    pub line_breaks: Vec<LineBreakSpan>,
    pub line_start: Option<DecimalU64>,
    pub line_count: Option<DecimalU64>,
}

impl ViewLeaseSegment {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_id(&self.segment_id, "segmentId", MAX_LEASE_ID_BYTES)?;
        validate_decimal(&self.source_byte_start, "sourceByteStart")?;
        validate_decimal(&self.source_byte_end, "sourceByteEnd")?;
        if self.source_byte_end.as_u64() < self.source_byte_start.as_u64() {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidTextBoundary,
                "segment source range is inverted",
                Some("sourceByteEnd"),
            ));
        }
        validate_local_offset(self.local_utf16_start, "localUtf16Start")?;
        validate_local_offset(self.local_utf16_end, "localUtf16End")?;
        if self.local_utf16_end < self.local_utf16_start {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidTextBoundary,
                "segment local range is inverted",
                Some("localUtf16End"),
            ));
        }
        validate_bounded_text(&self.text, "text", MAX_LEASE_BYTES)?;
        if self.text.contains('\r')
            || self.text.encode_utf16().count() != self.local_utf16_end - self.local_utf16_start
        {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidTextBoundary,
                "segment text does not match normalized UTF-16 range",
                Some("text"),
            ));
        }
        if self.boundaries.is_empty()
            || self.boundaries.len() > MAX_LEASE_BOUNDARIES
            || self.line_breaks.len() > PROJECTION_ROWS
        {
            return Err(protocol_error(
                ProtocolErrorCode::ResourceLimit,
                "segment map is outside bounds",
                Some("boundaries"),
            ));
        }
        let first = &self.boundaries[0];
        if first.local_utf16_offset != self.local_utf16_start
            || first.source_byte_offset != self.source_byte_start
        {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidTextBoundary,
                "map start does not match source and local range",
                Some("boundaries"),
            ));
        }
        let mut local = self.local_utf16_start;
        let mut byte = self.source_byte_start.as_u64();
        let mut boundaries = self.boundaries.iter().skip(1);
        let mut breaks = self.line_breaks.iter();
        let mut utf8 = true;
        let mut utf16 = true;
        for scalar in self.text.chars() {
            let next = boundaries.next().ok_or_else(|| {
                protocol_error(
                    ProtocolErrorCode::InvalidTextBoundary,
                    "missing scalar boundary",
                    Some("boundaries"),
                )
            })?;
            let width = next
                .source_byte_offset
                .as_u64()
                .checked_sub(byte)
                .ok_or_else(|| {
                    protocol_error(
                        ProtocolErrorCode::InvalidTextBoundary,
                        "inverted byte boundary",
                        Some("boundaries"),
                    )
                })?;
            if next.local_utf16_offset != local + scalar.len_utf16() {
                return Err(protocol_error(
                    ProtocolErrorCode::InvalidTextBoundary,
                    "invalid scalar boundary",
                    Some("boundaries"),
                ));
            }
            let mut utf8_width = scalar.len_utf8() as u64;
            let mut utf16_width = (scalar.len_utf16() * 2) as u64;
            if scalar == '\n' {
                let span = breaks.next().ok_or_else(|| {
                    protocol_error(
                        ProtocolErrorCode::InvalidTextBoundary,
                        "missing newline span",
                        Some("lineBreaks"),
                    )
                })?;
                if span.local_utf16_offset != local
                    || span.source_byte_start.as_u64() != byte
                    || span.source_byte_length as u64 != width
                {
                    return Err(protocol_error(
                        ProtocolErrorCode::InvalidTextBoundary,
                        "newline map does not match its scalar boundary",
                        Some("lineBreaks"),
                    ));
                }
                if span.kind == LineBreakKind::Crlf {
                    utf8_width = 2;
                    utf16_width = 4;
                }
            }
            utf8 &= width == utf8_width;
            utf16 &= width == utf16_width;
            if !utf8 && !utf16 {
                return Err(protocol_error(
                    ProtocolErrorCode::InvalidTextBoundary,
                    "byte map is not a supported lossless encoding",
                    Some("boundaries"),
                ));
            }
            local = next.local_utf16_offset;
            byte = next.source_byte_offset.as_u64();
        }
        if boundaries.next().is_some()
            || breaks.next().is_some()
            || byte != self.source_byte_end.as_u64()
        {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidTextBoundary,
                "map does not exactly cover source text",
                Some("boundaries"),
            ));
        }
        if let Some(line_start) = &self.line_start {
            validate_decimal(line_start, "lineStart")?;
            if line_start.as_u64() == 0 {
                return Err(protocol_error(
                    ProtocolErrorCode::InvalidTextBoundary,
                    "line number is one based",
                    Some("lineStart"),
                ));
            }
        }
        if let Some(line_count) = &self.line_count {
            validate_decimal(line_count, "lineCount")?;
            if line_count.as_u64() != self.line_breaks.len() as u64 + 1 {
                return Err(protocol_error(
                    ProtocolErrorCode::InvalidTextBoundary,
                    "line count does not match text",
                    Some("lineCount"),
                ));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LineCertainty {
    Exact,
    Unknown,
    Indexing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ViewLease {
    pub lease_id: String,
    pub session_id: String,
    pub source_generation: DecimalU64,
    pub document_revision: DecimalU64,
    pub view_generation: DecimalU64,
    pub source_byte_start: DecimalU64,
    pub source_byte_end: DecimalU64,
    pub local_utf16_length: usize,
    pub line_certainty: LineCertainty,
    #[serde(default)]
    pub source_state: SourceState,
    #[serde(default)]
    pub invalid_at: Option<DecimalU64>,
    pub segments: Vec<ViewLeaseSegment>,
    pub expires_after_revision: DecimalU64,
}

impl ViewLease {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_id(&self.lease_id, "leaseId", MAX_LEASE_ID_BYTES)?;
        validate_id(&self.session_id, "sessionId", MAX_SESSION_ID_BYTES)?;
        validate_decimal(&self.source_generation, "sourceGeneration")?;
        validate_decimal(&self.document_revision, "documentRevision")?;
        validate_decimal(&self.view_generation, "viewGeneration")?;
        validate_decimal(&self.source_byte_start, "sourceByteStart")?;
        validate_decimal(&self.source_byte_end, "sourceByteEnd")?;
        validate_local_offset(self.local_utf16_length, "localUtf16Length")?;
        validate_decimal(&self.expires_after_revision, "expiresAfterRevision")?;
        if let Some(invalid_at) = &self.invalid_at {
            validate_decimal(invalid_at, "invalidAt")?;
            if invalid_at.as_u64() < self.source_byte_end.as_u64() {
                return Err(protocol_error(
                    ProtocolErrorCode::InvalidTextBoundary,
                    "invalid source marker precedes the readable lease end",
                    Some("invalidAt"),
                ));
            }
            if self.source_state != SourceState::DecisionRequired {
                return Err(protocol_error(
                    ProtocolErrorCode::InvalidTextBoundary,
                    "invalid source marker requires decision-required state",
                    Some("sourceState"),
                ));
            }
        } else if self.source_state == SourceState::DecisionRequired {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidTextBoundary,
                "decision-required source state needs invalid marker",
                Some("invalidAt"),
            ));
        }
        if self.segments.is_empty() || self.segments.len() > PROJECTION_SEGMENTS {
            return Err(protocol_error(
                ProtocolErrorCode::ResourceLimit,
                "lease segment count is outside bounds",
                Some("segments"),
            ));
        }
        let mut local = 0usize;
        let mut source = self.source_byte_start.as_u64();
        let mut boundaries = 0usize;
        let mut bytes = 0u64;
        let mut line_breaks = 0usize;
        for segment in &self.segments {
            segment.validate()?;
            if segment.local_utf16_start != local || segment.source_byte_start.as_u64() != source {
                return Err(protocol_error(
                    ProtocolErrorCode::InvalidTextBoundary,
                    "lease segments are not contiguous",
                    Some("segments"),
                ));
            }
            local = segment.local_utf16_end;
            source = segment.source_byte_end.as_u64();
            boundaries = boundaries.saturating_add(segment.boundaries.len());
            bytes = bytes.saturating_add(
                segment
                    .source_byte_end
                    .as_u64()
                    .saturating_sub(segment.source_byte_start.as_u64()),
            );
            line_breaks = line_breaks.saturating_add(segment.line_breaks.len());
        }
        if local != self.local_utf16_length || source != self.source_byte_end.as_u64() {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidTextBoundary,
                "lease ranges do not cover the declared projection",
                Some("segments"),
            ));
        }
        if boundaries > MAX_LEASE_BOUNDARIES
            || line_breaks.saturating_add(1) > PROJECTION_ROWS
            || bytes > MAX_LEASE_BYTES as u64
        {
            return Err(protocol_error(
                ProtocolErrorCode::ResourceLimit,
                "lease projection exceeds bounded resources",
                Some("segments"),
            ));
        }
        Ok(())
    }

    pub fn local_to_byte(&self, local_offset: usize) -> Result<DecimalU64, ProtocolError> {
        validate_local_offset(local_offset, "localOffset")?;
        for segment in &self.segments {
            if local_offset < segment.local_utf16_start || local_offset > segment.local_utf16_end {
                continue;
            }
            let boundary = segment
                .boundaries
                .iter()
                .find(|candidate| candidate.local_utf16_offset == local_offset)
                .ok_or_else(|| {
                    protocol_error(
                        ProtocolErrorCode::InvalidTextBoundary,
                        "local offset is inside a code point or CRLF unit",
                        Some("localOffset"),
                    )
                })?;
            return Ok(boundary.source_byte_offset.clone());
        }
        Err(protocol_error(
            ProtocolErrorCode::InvalidTextBoundary,
            "local offset is outside the lease",
            Some("localOffset"),
        ))
    }

    pub fn byte_to_local(&self, byte_offset: &DecimalU64) -> Result<usize, ProtocolError> {
        validate_decimal(byte_offset, "byteOffset")?;
        for segment in &self.segments {
            if byte_offset.as_u64() < segment.source_byte_start.as_u64()
                || byte_offset.as_u64() > segment.source_byte_end.as_u64()
            {
                continue;
            }
            let boundary = segment
                .boundaries
                .iter()
                .find(|candidate| candidate.source_byte_offset == *byte_offset)
                .ok_or_else(|| {
                    protocol_error(
                        ProtocolErrorCode::InvalidTextBoundary,
                        "byte offset is inside a code point or CRLF unit",
                        Some("byteOffset"),
                    )
                })?;
            return Ok(boundary.local_utf16_offset);
        }
        Err(protocol_error(
            ProtocolErrorCode::InvalidTextBoundary,
            "byte offset is outside the lease",
            Some("byteOffset"),
        ))
    }
    pub fn authorize_for(
        &self,
        envelope: &SessionEnvelope,
        actual_caller: &str,
    ) -> Result<(), ProtocolError> {
        envelope.validate()?;
        if actual_caller != "stack-popup" || envelope.session_id != self.session_id {
            return Err(protocol_error(
                ProtocolErrorCode::Unauthorized,
                "lease is not owned by caller",
                None,
            ));
        }
        if envelope.source_generation != self.source_generation {
            return Err(protocol_error(
                ProtocolErrorCode::SourceChanged,
                "lease source generation changed",
                None,
            ));
        }
        if envelope.document_revision != self.document_revision
            || envelope.view_generation != self.view_generation
            || envelope.lease_id.as_deref() != Some(self.lease_id.as_str())
            || envelope.document_revision.as_u64() > self.expires_after_revision.as_u64()
        {
            return Err(protocol_error(
                ProtocolErrorCode::StaleRevision,
                "lease is stale",
                None,
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionAffinity {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionDirection {
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionEndpoint {
    pub anchor_id: String,
    pub byte_offset: DecimalU64,
    pub affinity: SelectionAffinity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentSelection {
    pub selection_id: String,
    pub session_id: String,
    pub source_generation: DecimalU64,
    pub document_revision: DecimalU64,
    pub anchor: SelectionEndpoint,
    pub head: SelectionEndpoint,
    pub direction: SelectionDirection,
    pub expires_after_revision: DecimalU64,
}

impl DocumentSelection {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_id(&self.selection_id, "selectionId", MAX_SELECTION_ID_BYTES)?;
        validate_id(&self.session_id, "sessionId", MAX_SESSION_ID_BYTES)?;
        validate_decimal(&self.source_generation, "sourceGeneration")?;
        validate_decimal(&self.document_revision, "documentRevision")?;
        validate_endpoint(&self.anchor)?;
        validate_endpoint(&self.head)?;
        validate_decimal(&self.expires_after_revision, "expiresAfterRevision")?;
        if self.expires_after_revision.as_u64() < self.document_revision.as_u64() {
            return Err(protocol_error(
                ProtocolErrorCode::SelectionExpired,
                "selection expires before it is created",
                Some("expiresAfterRevision"),
            ));
        }
        Ok(())
    }
}

fn validate_endpoint(endpoint: &SelectionEndpoint) -> Result<(), ProtocolError> {
    validate_id(&endpoint.anchor_id, "anchorId", MAX_ANCHOR_ID_BYTES)?;
    validate_decimal(&endpoint.byte_offset, "byteOffset")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectionRemap {
    pub selection_id: String,
    pub document_revision: DecimalU64,
    pub anchor: Option<SelectionEndpoint>,
    pub head: Option<SelectionEndpoint>,
    pub status: SelectionRemapStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SelectionRemapStatus {
    Exact,
    Deleted,
    Expired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ViewTextRange {
    Lease {
        lease_id: String,
        start_utf16: usize,
        end_utf16: usize,
    },
    Selection {
        selection_id: String,
    },
}

impl ViewTextRange {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::Lease {
                lease_id,
                start_utf16,
                end_utf16,
            } => {
                validate_id(lease_id, "leaseId", MAX_LEASE_ID_BYTES)?;
                validate_local_offset(*start_utf16, "startUtf16")?;
                validate_local_offset(*end_utf16, "endUtf16")?;
                if end_utf16 < start_utf16 {
                    return Err(protocol_error(
                        ProtocolErrorCode::InvalidTextBoundary,
                        "text range is inverted",
                        Some("endUtf16"),
                    ));
                }
            }
            Self::Selection { selection_id } => {
                validate_id(selection_id, "selectionId", MAX_SELECTION_ID_BYTES)?
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TextInsert {
    Inline { text: String },
    Spool { spool_id: String },
}

impl TextInsert {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        match self {
            Self::Inline { text } => validate_bounded_text(text, "text", MAX_INLINE_TEXT_BYTES),
            Self::Spool { spool_id } => validate_id(spool_id, "spoolId", MAX_SPOOL_ID_BYTES),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextEdit {
    pub range: ViewTextRange,
    pub insert: TextInsert,
}

impl TextEdit {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.range.validate()?;
        self.insert.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationIdentity {
    pub operation_id: String,
    pub request_digest: String,
}

impl OperationIdentity {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_id(&self.operation_id, "operationId", MAX_OPERATION_ID_BYTES)?;
        validate_sha256(&self.request_digest, "requestDigest")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationReceipt {
    pub operation_id: String,
    pub outcome: MutationOutcome,
    pub revision: RevisionState,
    pub remapped_selections: Vec<SelectionRemap>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MutationOutcome {
    Accepted,
    Duplicate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NewlinePolicy {
    Lf,
    Crlf,
    Cr,
    Mixed,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceState {
    Opening,
    Snapshotting,
    Ready,
    Changed,
    Readonly,
    DecisionRequired,
    Cancelled,
    QuotaExceeded,
    Conflict,
    ReadLimited,
}

impl Default for SourceState {
    fn default() -> Self {
        Self::Ready
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobKind {
    Open,
    Read,
    Index,
    Search,
    Replace,
    Save,
    SaveAs,
    Reload,
    ClipboardExport,
    ClipboardImport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobPhase {
    Queued,
    Reading,
    Indexing,
    Spooling,
    Staging,
    Publishing,
    Verifying,
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Ambiguous,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobProgress {
    pub completed_bytes: DecimalU64,
    pub total_bytes: Option<DecimalU64>,
    pub completed_items: DecimalU64,
    pub total_items: Option<DecimalU64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextJobStatus {
    pub job_id: String,
    pub session_id: String,
    pub kind: JobKind,
    pub phase: JobPhase,
    pub state: JobState,
    pub source_generation: DecimalU64,
    pub document_revision: DecimalU64,
    pub progress: JobProgress,
    pub cancellable: bool,
    pub publication: PublicationState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PublicationState {
    NotStarted,
    NotPublished,
    Published,
    Ambiguous,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextSpool {
    pub spool_id: String,
    pub session_id: String,
    pub bytes: DecimalU64,
    pub sha256: String,
    pub encoding: SpoolEncoding,
    pub state: SpoolState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpoolEncoding {
    Utf8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpoolState {
    Staged,
    Consumed,
    Released,
}

impl TextSpool {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        validate_id(&self.spool_id, "spoolId", MAX_SPOOL_ID_BYTES)?;
        validate_id(&self.session_id, "sessionId", MAX_SESSION_ID_BYTES)?;
        validate_decimal(&self.bytes, "bytes")?;
        if self.bytes.as_u64() > MAX_SPOOL_BYTES_PER_SESSION {
            return Err(protocol_error(
                ProtocolErrorCode::ResourceLimit,
                "spool exceeds per-session quota",
                Some("bytes"),
            ));
        }
        validate_sha256(&self.sha256, "sha256")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenStackTextDocumentRequest {
    pub schema_revision: String,
    pub request_id: String,
    pub caller_surface: String,
    pub source_path: String,
    pub access: AccessMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessMode {
    Edit,
    ReadOnly,
}

impl OpenStackTextDocumentRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.schema_revision != SCHEMA_REVISION {
            return Err(invalid("unsupported schemaRevision", "schemaRevision"));
        }
        validate_id(&self.request_id, "requestId", MAX_REQUEST_ID_BYTES)?;
        if self.caller_surface != "stack-popup" {
            return Err(protocol_error(
                ProtocolErrorCode::Unauthorized,
                "caller surface is not authorized",
                Some("callerSurface"),
            ));
        }
        validate_text(&self.source_path, "sourcePath", MAX_PATH_BYTES)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenStackTextDocumentResult {
    pub session_id: String,
    pub source_generation: DecimalU64,
    pub revision: RevisionState,
    pub source_state: SourceState,
    pub encoding: Option<TextEncoding>,
    pub newline_policy: NewlinePolicy,
    pub initial_lease: Option<ViewLease>,
    pub job: Option<TextJobStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ViewTarget {
    Byte { byte_start: DecimalU64 },
    Line { line_number: DecimalU64 },
    Anchor { anchor_id: String },
    DocumentStart,
    DocumentEnd,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadStackTextWindowRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub request_id: String,
    pub target: ViewTarget,
    pub byte_budget: usize,
    pub row_budget: usize,
    pub utf16_budget: usize,
}

impl ReadStackTextWindowRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_id(&self.request_id, "requestId", MAX_REQUEST_ID_BYTES)?;
        match &self.target {
            ViewTarget::Byte { byte_start } => validate_decimal(byte_start, "byteStart")?,
            ViewTarget::Line { line_number } => validate_decimal(line_number, "lineNumber")?,
            ViewTarget::Anchor { anchor_id } => {
                validate_id(anchor_id, "anchorId", MAX_ANCHOR_ID_BYTES)?
            }
            ViewTarget::DocumentStart | ViewTarget::DocumentEnd => {}
        }
        if self.byte_budget == 0
            || self.byte_budget > IO_BLOCK_BYTES
            || self.row_budget == 0
            || self.row_budget > PROJECTION_ROWS
            || self.utf16_budget == 0
            || self.utf16_budget > PROJECTION_UTF16_UNITS
        {
            return Err(protocol_error(
                ProtocolErrorCode::ResourceLimit,
                "read budgets are outside bounds",
                Some("byteBudget"),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadStackTextWindowResult {
    pub envelope: SessionEnvelope,
    pub lease: ViewLease,
    pub source_state: SourceState,
    pub line_certainty: LineCertainty,
    pub job: Option<TextJobStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyStackTextEditsRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub expected_revision: DecimalU64,
    #[serde(flatten)]
    pub operation: OperationIdentity,
    pub edits: Vec<TextEdit>,
    pub preceding_input: Option<InputBarrier>,
}

impl ApplyStackTextEditsRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_decimal(&self.expected_revision, "expectedRevision")?;
        self.operation.validate()?;
        if self.edits.is_empty() || self.edits.len() > MAX_SPECULATIVE_OPERATIONS {
            return Err(protocol_error(
                ProtocolErrorCode::ResourceLimit,
                "edit batch count is outside bounds",
                Some("edits"),
            ));
        }
        for edit in &self.edits {
            edit.validate()?;
        }
        if let Some(barrier) = &self.preceding_input {
            barrier.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UndoRedoStackTextEditRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub expected_revision: DecimalU64,
    #[serde(flatten)]
    pub operation: OperationIdentity,
    pub preceding_input: InputBarrier,
}

impl UndoRedoStackTextEditRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_decimal(&self.expected_revision, "expectedRevision")?;
        self.operation.validate()?;
        self.preceding_input.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchStackTextDocumentRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub request_id: String,
    pub job_id: String,
    pub preceding_input: InputBarrier,
    pub query: String,
    pub match_case: bool,
    pub direction: SearchDirection,
    pub scope: SearchScope,
    pub selection_id: Option<String>,
    pub limit: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchDirection {
    Forward,
    Backward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchScope {
    Document,
    Selection,
}

impl SearchStackTextDocumentRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_id(&self.request_id, "requestId", MAX_REQUEST_ID_BYTES)?;
        validate_id(&self.job_id, "jobId", MAX_JOB_ID_BYTES)?;
        self.preceding_input.validate()?;
        validate_text(&self.query, "query", MAX_QUERY_BYTES)?;
        if self.limit == 0 || self.limit > MAX_RESULT_ITEMS {
            return Err(protocol_error(
                ProtocolErrorCode::ResourceLimit,
                "search limit is outside bounds",
                Some("limit"),
            ));
        }
        if matches!(self.scope, SearchScope::Selection) && self.selection_id.is_none() {
            return Err(invalid(
                "selection search requires selectionId",
                "selectionId",
            ));
        }
        if let Some(selection_id) = &self.selection_id {
            validate_id(selection_id, "selectionId", MAX_SELECTION_ID_BYTES)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceStackTextMatchesRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub expected_revision: DecimalU64,
    #[serde(flatten)]
    pub operation: OperationIdentity,
    pub preceding_input: InputBarrier,
    pub query: String,
    pub replacement: TextInsert,
    pub match_case: bool,
    pub scope: SearchScope,
    pub selection_id: Option<String>,
    pub search_revision: DecimalU64,
}

impl ReplaceStackTextMatchesRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_decimal(&self.expected_revision, "expectedRevision")?;
        self.operation.validate()?;
        self.preceding_input.validate()?;
        validate_text(&self.query, "query", MAX_QUERY_BYTES)?;
        self.replacement.validate()?;
        validate_decimal(&self.search_revision, "searchRevision")?;
        if matches!(self.scope, SearchScope::Selection) && self.selection_id.is_none() {
            return Err(invalid(
                "selection replace requires selectionId",
                "selectionId",
            ));
        }
        if let Some(selection_id) = &self.selection_id {
            validate_id(selection_id, "selectionId", MAX_SELECTION_ID_BYTES)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveStackTextDocumentRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub expected_revision: DecimalU64,
    #[serde(flatten)]
    pub operation: OperationIdentity,
    pub preceding_input: InputBarrier,
}

impl SaveStackTextDocumentRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_decimal(&self.expected_revision, "expectedRevision")?;
        self.operation.validate()?;
        self.preceding_input.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveStackTextDocumentAsRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub expected_revision: DecimalU64,
    #[serde(flatten)]
    pub operation: OperationIdentity,
    pub preceding_input: InputBarrier,
    pub destination_path: String,
}

impl SaveStackTextDocumentAsRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_decimal(&self.expected_revision, "expectedRevision")?;
        self.operation.validate()?;
        self.preceding_input.validate()?;
        validate_text(&self.destination_path, "destinationPath", MAX_PATH_BYTES)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CloseStackTextDocumentRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub expected_revision: DecimalU64,
    pub preceding_input: InputBarrier,
    pub disposition: Disposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Disposition {
    Save,
    Discard,
    Cancel,
}

impl CloseStackTextDocumentRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_decimal(&self.expected_revision, "expectedRevision")?;
        self.preceding_input.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelStackTextJobRequest {
    pub schema_revision: String,
    pub session_id: String,
    pub job_id: String,
    pub request_id: String,
}

impl CancelStackTextJobRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.schema_revision != SCHEMA_REVISION {
            return Err(invalid("unsupported schemaRevision", "schemaRevision"));
        }
        validate_id(&self.session_id, "sessionId", MAX_SESSION_ID_BYTES)?;
        validate_id(&self.job_id, "jobId", MAX_JOB_ID_BYTES)?;
        validate_id(&self.request_id, "requestId", MAX_REQUEST_ID_BYTES)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancellationReceipt {
    pub job_id: String,
    pub outcome: CancellationOutcome,
    pub phase: CancellationPhase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CancellationOutcome {
    Cancelled,
    AlreadyPublished,
    PublicationAmbiguous,
    AlreadyComplete,
    NotFound,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CancellationPhase {
    Queued,
    Cooperative,
    NotCancellable,
    Published,
    Ambiguous,
    Complete,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateStackTextSelectionRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub request_id: String,
    pub lease_id: String,
    pub start_utf16: usize,
    pub end_utf16: usize,
    pub direction: SelectionDirection,
    pub anchor_affinity: SelectionAffinity,
    pub head_affinity: SelectionAffinity,
}

impl CreateStackTextSelectionRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_id(&self.request_id, "requestId", MAX_REQUEST_ID_BYTES)?;
        validate_id(&self.lease_id, "leaseId", MAX_LEASE_ID_BYTES)?;
        validate_local_offset(self.start_utf16, "startUtf16")?;
        validate_local_offset(self.end_utf16, "endUtf16")?;
        if self.end_utf16 < self.start_utf16 {
            return Err(protocol_error(
                ProtocolErrorCode::InvalidTextBoundary,
                "selection range is inverted",
                Some("endUtf16"),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemapStackTextSelectionRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub request_id: String,
    pub selection_id: String,
    pub lease_id: Option<String>,
}

impl RemapStackTextSelectionRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_id(&self.request_id, "requestId", MAX_REQUEST_ID_BYTES)?;
        validate_id(&self.selection_id, "selectionId", MAX_SELECTION_ID_BYTES)?;
        if let Some(lease_id) = &self.lease_id {
            validate_id(lease_id, "leaseId", MAX_LEASE_ID_BYTES)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseStackTextSelectionRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub request_id: String,
    pub selection_id: String,
}

impl ReleaseStackTextSelectionRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_id(&self.request_id, "requestId", MAX_REQUEST_ID_BYTES)?;
        validate_id(&self.selection_id, "selectionId", MAX_SELECTION_ID_BYTES)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportStackTextSelectionRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub expected_revision: DecimalU64,
    #[serde(flatten)]
    pub operation: OperationIdentity,
    pub preceding_input: InputBarrier,
    pub selection_id: String,
}

impl ExportStackTextSelectionRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_decimal(&self.expected_revision, "expectedRevision")?;
        self.operation.validate()?;
        self.preceding_input.validate()?;
        validate_id(&self.selection_id, "selectionId", MAX_SELECTION_ID_BYTES)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportStackTextClipboardRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub expected_revision: DecimalU64,
    #[serde(flatten)]
    pub operation: OperationIdentity,
    pub preceding_input: InputBarrier,
    pub source: ClipboardSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClipboardSource {
    Clipboard,
    TextDrop,
}

impl ImportStackTextClipboardRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_decimal(&self.expected_revision, "expectedRevision")?;
        self.operation.validate()?;
        self.preceding_input.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseStackTextSpoolRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub request_id: String,
    pub spool_id: String,
}

impl ReleaseStackTextSpoolRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_id(&self.request_id, "requestId", MAX_REQUEST_ID_BYTES)?;
        validate_id(&self.spool_id, "spoolId", MAX_SPOOL_ID_BYTES)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReloadStackTextDocumentRequest {
    #[serde(flatten)]
    pub envelope: SessionEnvelope,
    pub expected_revision: DecimalU64,
    #[serde(flatten)]
    pub operation: OperationIdentity,
    pub preceding_input: InputBarrier,
    pub disposition: Disposition,
    pub encoding: Option<TextEncoding>,
}

impl ReloadStackTextDocumentRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.envelope.validate()?;
        validate_decimal(&self.expected_revision, "expectedRevision")?;
        self.preceding_input.validate()?;
        self.operation.validate()
    }
}

/// SHA-256 over sorted-key JSON of a validated request, excluding its digest.
pub fn request_digest(payload: &serde_json::Value) -> Result<String, ProtocolError> {
    fn canonical(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let sorted: BTreeMap<_, _> = map
                    .iter()
                    .map(|(key, value)| (key.clone(), canonical(value)))
                    .collect();
                serde_json::Value::Object(sorted.into_iter().collect())
            }
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.iter().map(canonical).collect())
            }
            value => value.clone(),
        }
    }
    let mut value = canonical(payload);
    value
        .as_object_mut()
        .ok_or_else(|| invalid("request must be an object", "request"))?
        .remove("requestDigest");
    let bytes =
        serde_json::to_vec(&value).map_err(|_| invalid("request cannot be encoded", "request"))?;
    if bytes.len() > MAX_INLINE_TEXT_BYTES + 64 * 1024 {
        return Err(protocol_error(
            ProtocolErrorCode::ResourceLimit,
            "request exceeds transport bound",
            None,
        ));
    }
    Ok(format!("{:x}", super::hash::Sha256::digest(&bytes)))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationAdmission {
    New,
    Pending,
    Duplicate { accepted_revision: DecimalU64 },
    Conflict,
    Retired,
}

#[derive(Clone, Debug)]
struct LedgerEntry {
    digest: String,
    accepted_revision: Option<DecimalU64>,
}

/// Bounded exact-once adapter for tests and the future document actor.
///
/// Retirement is explicit and requires a durable revision at or beyond the
/// accepted revision. A full retired set returns ResourceLimit; it never drops
/// tombstones and accidentally re-applies an expired retry.
#[derive(Clone, Debug, Default)]
pub struct BoundedOperationLedger {
    active: BTreeMap<String, LedgerEntry>,
    retired: BTreeSet<String>,
}

impl BoundedOperationLedger {
    pub fn admit_payload(
        &mut self,
        operation: &OperationIdentity,
        payload: &serde_json::Value,
    ) -> Result<OperationAdmission, ProtocolError> {
        if request_digest(payload)? != operation.request_digest {
            return Ok(OperationAdmission::Conflict);
        }
        self.admit(operation)
    }

    pub fn admit(
        &mut self,
        operation: &OperationIdentity,
    ) -> Result<OperationAdmission, ProtocolError> {
        operation.validate()?;
        if self.retired.contains(&operation.operation_id) {
            return Ok(OperationAdmission::Retired);
        }
        if let Some(entry) = self.active.get(&operation.operation_id) {
            return Ok(if entry.digest != operation.request_digest {
                OperationAdmission::Conflict
            } else if let Some(revision) = &entry.accepted_revision {
                OperationAdmission::Duplicate {
                    accepted_revision: revision.clone(),
                }
            } else {
                OperationAdmission::Pending
            });
        }
        if self.active.len() >= MAX_RETAINED_OPERATIONS {
            return Err(protocol_error(
                ProtocolErrorCode::ResourceLimit,
                "operation ledger admission is full",
                None,
            ));
        }
        self.active.insert(
            operation.operation_id.clone(),
            LedgerEntry {
                digest: operation.request_digest.clone(),
                accepted_revision: None,
            },
        );
        Ok(OperationAdmission::New)
    }

    pub fn mark_accepted(
        &mut self,
        operation_id: &str,
        accepted_revision: DecimalU64,
    ) -> Result<(), ProtocolError> {
        validate_id(operation_id, "operationId", MAX_OPERATION_ID_BYTES)?;
        let entry = self.active.get_mut(operation_id).ok_or_else(|| {
            protocol_error(
                ProtocolErrorCode::OperationIdRetired,
                "operation is not active",
                None,
            )
        })?;
        if entry
            .accepted_revision
            .as_ref()
            .is_some_and(|revision| revision != &accepted_revision)
        {
            return Err(protocol_error(
                ProtocolErrorCode::OperationIdConflict,
                "accepted receipt is immutable",
                None,
            ));
        }
        entry.accepted_revision = Some(accepted_revision);
        Ok(())
    }

    pub fn retire(
        &mut self,
        operation_id: &str,
        durable_revision: &DecimalU64,
    ) -> Result<(), ProtocolError> {
        validate_id(operation_id, "operationId", MAX_OPERATION_ID_BYTES)?;
        if self.retired.len() >= MAX_RETIRED_OPERATIONS {
            return Err(protocol_error(
                ProtocolErrorCode::ResourceLimit,
                "retirement ledger is full; persist a checkpoint before continuing",
                None,
            ));
        }
        let entry = self.active.get(operation_id).ok_or_else(|| {
            protocol_error(
                ProtocolErrorCode::OperationIdRetired,
                "operation is not active",
                None,
            )
        })?;
        if entry
            .accepted_revision
            .as_ref()
            .is_none_or(|revision| durable_revision.as_u64() < revision.as_u64())
        {
            return Err(protocol_error(
                ProtocolErrorCode::StaleRevision,
                "operation cannot retire before durable acknowledgement",
                None,
            ));
        }
        self.active.remove(operation_id);
        self.retired.insert(operation_id.to_string());
        Ok(())
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }
    pub fn retired_count(&self) -> usize {
        self.retired.len()
    }
}

pub fn remap_selection_after_edit(
    selection: &DocumentSelection,
    revision: DecimalU64,
    deleted_start: &DecimalU64,
    deleted_end: &DecimalU64,
    inserted_bytes: &DecimalU64,
) -> Result<SelectionRemap, ProtocolError> {
    selection.validate()?;
    validate_decimal(&revision, "revision")?;
    validate_decimal(deleted_start, "deletedStart")?;
    validate_decimal(deleted_end, "deletedEnd")?;
    validate_decimal(inserted_bytes, "insertedBytes")?;
    if revision.as_u64() > selection.expires_after_revision.as_u64() {
        return Ok(SelectionRemap {
            selection_id: selection.selection_id.clone(),
            document_revision: revision,
            anchor: None,
            head: None,
            status: SelectionRemapStatus::Expired,
        });
    }
    if deleted_end.as_u64() < deleted_start.as_u64() {
        return Err(protocol_error(
            ProtocolErrorCode::InvalidTextBoundary,
            "deleted range is inverted",
            Some("deletedEnd"),
        ));
    }
    let delta =
        inserted_bytes.as_u64() as i128 - (deleted_end.as_u64() - deleted_start.as_u64()) as i128;
    let remap = |endpoint: &SelectionEndpoint| -> Result<Option<SelectionEndpoint>, ProtocolError> {
        let offset = endpoint.byte_offset.as_u64();
        if offset > deleted_start.as_u64() && offset < deleted_end.as_u64() {
            return Ok(None);
        }
        let next = if offset == deleted_start.as_u64() {
            offset as i128
                + if endpoint.affinity == SelectionAffinity::After {
                    inserted_bytes.as_u64() as i128
                } else {
                    0
                }
        } else if offset >= deleted_end.as_u64() {
            offset as i128 + delta
        } else {
            offset as i128
        };
        let next = u64::try_from(next).map_err(|_| {
            protocol_error(
                ProtocolErrorCode::InvalidTextBoundary,
                "selection remap exceeds u64",
                Some("byteOffset"),
            )
        })?;
        Ok(Some(SelectionEndpoint {
            anchor_id: endpoint.anchor_id.clone(),
            byte_offset: DecimalU64::from(next),
            affinity: endpoint.affinity,
        }))
    };
    let anchor = remap(&selection.anchor)?;
    let head = remap(&selection.head)?;
    let status = if anchor.is_some() && head.is_some() {
        SelectionRemapStatus::Exact
    } else {
        SelectionRemapStatus::Deleted
    };
    Ok(SelectionRemap {
        selection_id: selection.selection_id.clone(),
        document_revision: revision,
        anchor,
        head,
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope() -> SessionEnvelope {
        SessionEnvelope {
            schema_revision: SCHEMA_REVISION.to_string(),
            session_id: "session-1".to_string(),
            source_generation: DecimalU64::from(1),
            document_revision: DecimalU64::from(0),
            view_generation: DecimalU64::from(1),
            lease_id: Some("lease-1".to_string()),
        }
    }

    #[test]
    fn decimal_u64_is_string_only_and_lossless() {
        let encoded =
            serde_json::to_string(&DecimalU64::parse("9007199254740993").expect("valid u64"))
                .expect("encode");
        assert_eq!(encoded, "\"9007199254740993\"");
        assert!(serde_json::from_str::<DecimalU64>("9007199254740992").is_err());
        assert!(serde_json::from_str::<DecimalU64>("\"18446744073709551615\"").is_ok());
        assert!(serde_json::from_str::<DecimalU64>("\"18446744073709551616\"").is_err());
    }

    #[test]
    fn crlf_maps_to_one_local_unit_and_two_source_bytes() {
        let segment = ViewLeaseSegment {
            segment_id: "segment-1".to_string(),
            source_byte_start: DecimalU64::from(0),
            source_byte_end: DecimalU64::from(3),
            local_utf16_start: 0,
            local_utf16_end: 2,
            text: "a\n".to_string(),
            boundaries: vec![
                LeaseBoundary {
                    local_utf16_offset: 0,
                    source_byte_offset: DecimalU64::from(0),
                },
                LeaseBoundary {
                    local_utf16_offset: 1,
                    source_byte_offset: DecimalU64::from(1),
                },
                LeaseBoundary {
                    local_utf16_offset: 2,
                    source_byte_offset: DecimalU64::from(3),
                },
            ],
            line_breaks: vec![LineBreakSpan {
                local_utf16_offset: 1,
                source_byte_start: DecimalU64::from(1),
                source_byte_length: 2,
                kind: LineBreakKind::Crlf,
            }],
            line_start: Some(DecimalU64::from(1)),
            line_count: Some(DecimalU64::from(2)),
        };
        segment.validate().expect("CRLF segment is valid");
        let lease = ViewLease {
            lease_id: "lease-1".to_string(),
            session_id: "session-1".to_string(),
            source_generation: DecimalU64::from(1),
            document_revision: DecimalU64::from(0),
            view_generation: DecimalU64::from(1),
            source_byte_start: DecimalU64::from(0),
            source_byte_end: DecimalU64::from(3),
            local_utf16_length: 2,
            line_certainty: LineCertainty::Exact,
            source_state: SourceState::Ready,
            invalid_at: None,
            segments: vec![segment],
            expires_after_revision: DecimalU64::from(256),
        };
        lease.validate().expect("lease is valid");
        assert_eq!(lease.local_to_byte(1).expect("newline start").as_str(), "1");
        assert_eq!(lease.local_to_byte(2).expect("after newline").as_str(), "3");
        assert!(lease.byte_to_local(&DecimalU64::from(2)).is_err());
    }

    #[test]
    fn ledger_is_exact_once_and_never_reapplies_retired_ids() {
        let mut ledger = BoundedOperationLedger::default();
        let operation = OperationIdentity {
            operation_id: "op-1".to_string(),
            request_digest: "00".repeat(32),
        };
        assert_eq!(
            ledger.admit(&operation).expect("admit"),
            OperationAdmission::New
        );
        ledger
            .mark_accepted("op-1", DecimalU64::from(2))
            .expect("accepted");
        assert_eq!(
            ledger.admit(&operation).expect("duplicate"),
            OperationAdmission::Duplicate {
                accepted_revision: DecimalU64::from(2)
            }
        );
        let changed = OperationIdentity {
            operation_id: "op-1".to_string(),
            request_digest: "11".repeat(32),
        };
        assert_eq!(
            ledger.admit(&changed).expect("conflict"),
            OperationAdmission::Conflict
        );
        ledger.retire("op-1", &DecimalU64::from(2)).expect("retire");
        assert_eq!(
            ledger.admit(&operation).expect("retired"),
            OperationAdmission::Retired
        );
    }

    #[test]
    fn selection_remap_preserves_affinity_and_marks_deleted_anchor() {
        let selection = DocumentSelection {
            selection_id: "selection-1".to_string(),
            session_id: "session-1".to_string(),
            source_generation: DecimalU64::from(1),
            document_revision: DecimalU64::from(0),
            anchor: SelectionEndpoint {
                anchor_id: "anchor-a".to_string(),
                byte_offset: DecimalU64::from(2),
                affinity: SelectionAffinity::Before,
            },
            head: SelectionEndpoint {
                anchor_id: "anchor-h".to_string(),
                byte_offset: DecimalU64::from(5),
                affinity: SelectionAffinity::After,
            },
            direction: SelectionDirection::Forward,
            expires_after_revision: DecimalU64::from(256),
        };
        let remap = remap_selection_after_edit(
            &selection,
            DecimalU64::from(1),
            &DecimalU64::from(3),
            &DecimalU64::from(7),
            &DecimalU64::from(1),
        )
        .expect("remap");
        assert_eq!(remap.status, SelectionRemapStatus::Deleted);
        assert!(remap.anchor.is_some());
        assert!(remap.head.is_none());
    }

    fn lease_with_lf_segments(segment_break_counts: &[usize]) -> ViewLease {
        let mut local = 0usize;
        let mut source = 0u64;
        let segments = segment_break_counts
            .iter()
            .enumerate()
            .map(|(index, break_count)| {
                let text = "\n".repeat(*break_count);
                let boundaries = (0..=*break_count)
                    .map(|offset| LeaseBoundary {
                        local_utf16_offset: local + offset,
                        source_byte_offset: DecimalU64::from(source + offset as u64),
                    })
                    .collect();
                let line_breaks = (0..*break_count)
                    .map(|offset| LineBreakSpan {
                        local_utf16_offset: local + offset,
                        source_byte_start: DecimalU64::from(source + offset as u64),
                        source_byte_length: 1,
                        kind: LineBreakKind::Lf,
                    })
                    .collect();
                let segment = ViewLeaseSegment {
                    segment_id: format!("segment-{}", index + 1),
                    source_byte_start: DecimalU64::from(source),
                    source_byte_end: DecimalU64::from(source + *break_count as u64),
                    local_utf16_start: local,
                    local_utf16_end: local + *break_count,
                    text,
                    boundaries,
                    line_breaks,
                    line_start: Some(DecimalU64::from((local + 1) as u64)),
                    line_count: Some(DecimalU64::from((*break_count + 1) as u64)),
                };
                local += *break_count;
                source += *break_count as u64;
                segment
            })
            .collect();
        ViewLease {
            lease_id: "lease-1".to_string(),
            session_id: "session-1".to_string(),
            source_generation: DecimalU64::from(1),
            document_revision: DecimalU64::from(0),
            view_generation: DecimalU64::from(1),
            source_byte_start: DecimalU64::from(0),
            source_byte_end: DecimalU64::from(source),
            local_utf16_length: local,
            line_certainty: LineCertainty::Exact,
            source_state: SourceState::Ready,
            invalid_at: None,
            segments,
            expires_after_revision: DecimalU64::from(256),
        }
    }

    #[test]
    fn lease_row_ceiling_is_global_across_segments_and_includes_base_row() {
        assert!(lease_with_lf_segments(&[3000, 3000]).validate().is_err());
        let boundary = lease_with_lf_segments(&[2048, 2047]);
        boundary
            .validate()
            .expect("4096 total rows are valid across segments");
        assert_eq!(boundary.local_to_byte(4095).unwrap().as_u64(), 4095);
        assert_eq!(boundary.byte_to_local(&4095.into()).unwrap(), 4095);
        assert!(lease_with_lf_segments(&[4096]).validate().is_err());
        assert!(lease_with_lf_segments(&[2048, 2048]).validate().is_err());
    }

    #[test]
    fn shared_wire_maps_and_forged_boundaries() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/stack-text-editor-protocol.json"
        ))
        .unwrap();
        for case in cases.as_array().unwrap() {
            let Some(raw_lease) = case.get("lease") else {
                continue;
            };
            assert_eq!(
                case.get("schemaRevision")
                    .and_then(serde_json::Value::as_str),
                Some(SCHEMA_REVISION)
            );
            assert_eq!(
                case.get("feasibilitySchemaRevision")
                    .and_then(serde_json::Value::as_str),
                Some(P02_FEASIBILITY_SCHEMA_REVISION)
            );
            let lease: ViewLease = serde_json::from_value(raw_lease.clone()).unwrap();
            lease.validate().unwrap();
            for boundary in &lease.segments[0].boundaries {
                assert_eq!(
                    lease.local_to_byte(boundary.local_utf16_offset).unwrap(),
                    boundary.source_byte_offset
                );
                assert_eq!(
                    lease.byte_to_local(&boundary.source_byte_offset).unwrap(),
                    boundary.local_utf16_offset
                );
            }
            assert!(lease.byte_to_local(&DecimalU64::from(2)).is_err());
            let mut forged = lease.clone();
            forged.segments[0].boundaries[0].source_byte_offset = DecimalU64::from(1);
            assert!(forged.validate().is_err());
        }
    }

    #[test]
    fn selection_affinity_and_overflow_are_checked() {
        let mut selection = DocumentSelection {
            selection_id: "s".into(),
            session_id: "session-1".into(),
            source_generation: 1.into(),
            document_revision: 0.into(),
            expires_after_revision: 256.into(),
            direction: SelectionDirection::Forward,
            anchor: SelectionEndpoint {
                anchor_id: "a".into(),
                byte_offset: 2.into(),
                affinity: SelectionAffinity::Before,
            },
            head: SelectionEndpoint {
                anchor_id: "b".into(),
                byte_offset: 2.into(),
                affinity: SelectionAffinity::After,
            },
        };
        let result =
            remap_selection_after_edit(&selection, 1.into(), &2.into(), &2.into(), &3.into())
                .unwrap();
        assert_eq!(result.anchor.unwrap().byte_offset.as_u64(), 2);
        assert_eq!(result.head.unwrap().byte_offset.as_u64(), 5);
        selection.head.byte_offset = u64::MAX.into();
        assert!(
            remap_selection_after_edit(&selection, 1.into(), &2.into(), &2.into(), &3.into())
                .is_err()
        );
    }

    #[test]
    fn actual_edit_wire_deserializes_and_rejects_unknown_fields() {
        let wire = serde_json::json!({
            "schemaRevision": SCHEMA_REVISION, "sessionId": "s", "sourceGeneration": "1",
            "documentRevision": "0", "viewGeneration": "1", "leaseId": "l", "expectedRevision": "0",
            "operationId": "o", "requestDigest": "00".repeat(32),
            "edits": [{"range":{"kind":"lease","leaseId":"l","startUtf16":0,"endUtf16":0},"insert":{"kind":"inline","text":"x"}}]
        });
        let parsed: ApplyStackTextEditsRequest = serde_json::from_value(wire.clone()).unwrap();
        parsed.validate().unwrap();
        let mut invalid = wire;
        invalid["forged"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ApplyStackTextEditsRequest>(invalid).is_err());
    }

    #[test]
    fn insertion_wire_rejects_unpaired_surrogates_and_opposite_variant_fields() {
        let invalid_apply = r#"{"schemaRevision":"stack-text-editor.v1","sessionId":"s","sourceGeneration":"1","documentRevision":"0","viewGeneration":"1","leaseId":"l","expectedRevision":"0","operationId":"o","requestDigest":"0000000000000000000000000000000000000000000000000000000000000000","edits":[{"range":{"kind":"lease","leaseId":"l","startUtf16":0,"endUtf16":0},"insert":{"kind":"inline","text":"\uD800"}}]}"#;
        let invalid_replace = r#"{"schemaRevision":"stack-text-editor.v1","sessionId":"s","sourceGeneration":"1","documentRevision":"0","viewGeneration":"1","leaseId":"l","expectedRevision":"0","operationId":"o","requestDigest":"0000000000000000000000000000000000000000000000000000000000000000","precedingInput":{"acceptedRevision":"0","localInputSequence":"0","operationIds":[]},"query":"x","replacement":{"kind":"inline","text":"\uD800"},"matchCase":false,"scope":"document","searchRevision":"0"}"#;
        assert!(serde_json::from_str::<ApplyStackTextEditsRequest>(invalid_apply).is_err());
        assert!(serde_json::from_str::<ReplaceStackTextMatchesRequest>(invalid_replace).is_err());
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/stack-text-editor-protocol.json"
        ))
        .unwrap();
        for case in cases
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| case.get("request").is_some())
        {
            let rejected = match case["kind"].as_str() {
                Some("apply") => {
                    serde_json::from_value::<ApplyStackTextEditsRequest>(case["request"].clone())
                        .is_err()
                }
                Some("replace") => serde_json::from_value::<ReplaceStackTextMatchesRequest>(
                    case["request"].clone(),
                )
                .is_err(),
                _ => panic!("unknown negative wire kind"),
            };
            assert!(rejected, "{}", case["name"]);
        }
    }

    #[test]
    fn close_and_reload_require_preceding_input_before_disposition() {
        let mut close = serde_json::json!({
            "schemaRevision": SCHEMA_REVISION, "sessionId": "s", "sourceGeneration": "1",
            "documentRevision": "0", "viewGeneration": "1", "leaseId": "l", "expectedRevision": "0",
            "disposition": "discard"
        });
        assert!(serde_json::from_value::<CloseStackTextDocumentRequest>(close.clone()).is_err());
        close["precedingInput"] = serde_json::json!({
            "acceptedRevision": "0", "localInputSequence": "7", "operationIds": ["o"]
        });
        let parsed_close: CloseStackTextDocumentRequest = serde_json::from_value(close).unwrap();
        parsed_close.validate().unwrap();
        let acknowledged = BTreeSet::from(["o".to_string()]);
        parsed_close
            .preceding_input
            .validate_state(&0.into(), &7.into(), &acknowledged)
            .unwrap();
        assert!(parsed_close
            .preceding_input
            .validate_state(&0.into(), &8.into(), &acknowledged)
            .is_err());
        assert!(parsed_close
            .preceding_input
            .validate_state(&0.into(), &7.into(), &BTreeSet::new())
            .is_err());

        let mut reload = serde_json::json!({
            "schemaRevision": SCHEMA_REVISION, "sessionId": "s", "sourceGeneration": "1",
            "documentRevision": "0", "viewGeneration": "1", "leaseId": "l", "expectedRevision": "0",
            "operationId": "reload-1", "requestDigest": "00".repeat(32), "disposition": "discard"
        });
        assert!(serde_json::from_value::<ReloadStackTextDocumentRequest>(reload.clone()).is_err());
        reload["precedingInput"] = serde_json::json!({
            "acceptedRevision": "0", "localInputSequence": "7", "operationIds": ["o"]
        });
        let parsed_reload: ReloadStackTextDocumentRequest = serde_json::from_value(reload).unwrap();
        parsed_reload.validate().unwrap();
        parsed_reload
            .preceding_input
            .validate_state(&0.into(), &7.into(), &acknowledged)
            .unwrap();
        assert!(parsed_reload
            .preceding_input
            .validate_state(&0.into(), &8.into(), &acknowledged)
            .is_err());
        assert!(parsed_reload
            .preceding_input
            .validate_state(&1.into(), &7.into(), &acknowledged)
            .is_err());
        assert!(parsed_reload
            .preceding_input
            .validate_state(&0.into(), &7.into(), &BTreeSet::new())
            .is_err());
    }

    #[test]
    fn payload_receipts_and_retirement_stay_bounded() {
        let payload = serde_json::json!({"operationId":"op-1","text":"first"});
        let digest = request_digest(&payload).unwrap();
        assert_eq!(
            digest,
            "26872d1fe91b931a5fe0fc4f56ed4079484a0abcbcd4fce27a0c8f46de23aa7f"
        );
        let operation = OperationIdentity {
            operation_id: "op-1".into(),
            request_digest: digest,
        };
        let mut ledger = BoundedOperationLedger::default();
        assert_eq!(
            ledger.admit_payload(&operation, &payload).unwrap(),
            OperationAdmission::New
        );
        assert_eq!(
            ledger.admit_payload(&operation, &payload).unwrap(),
            OperationAdmission::Pending
        );
        assert!(ledger.retire("op-1", &0.into()).is_err());
        ledger.mark_accepted("op-1", 1.into()).unwrap();
        assert!(ledger.mark_accepted("op-1", 2.into()).is_err());
        assert_eq!(
            ledger
                .admit_payload(
                    &operation,
                    &serde_json::json!({"operationId":"op-1","text":"second"})
                )
                .unwrap(),
            OperationAdmission::Conflict
        );
        for index in 0..MAX_RETIRED_OPERATIONS {
            let id = format!("retire-{index}");
            ledger
                .admit(&OperationIdentity {
                    operation_id: id.clone(),
                    request_digest: "00".repeat(32),
                })
                .unwrap();
            ledger.mark_accepted(&id, 1.into()).unwrap();
            ledger.retire(&id, &1.into()).unwrap();
        }
        assert!(ledger.retire("op-1", &1.into()).is_err());
        assert_eq!(ledger.active_count(), 1);
        assert_eq!(ledger.retired_count(), MAX_RETIRED_OPERATIONS);
    }

    #[test]
    fn authoritative_ownership_and_input_barrier_reject_stale_requests() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/stack-text-editor-protocol.json"
        ))
        .unwrap();
        let lease: ViewLease = serde_json::from_value(cases[0]["lease"].clone()).unwrap();
        let mut request = envelope();
        lease.authorize_for(&request, "stack-popup").unwrap();
        assert!(lease.authorize_for(&request, "top-bar").is_err());
        request.session_id = "other".into();
        assert!(lease.authorize_for(&request, "stack-popup").is_err());
        request = envelope();
        request.view_generation = 2.into();
        assert!(lease.authorize_for(&request, "stack-popup").is_err());
        let barrier = InputBarrier {
            accepted_revision: 1.into(),
            local_input_sequence: 3.into(),
            operation_ids: vec!["op-1".into()],
        };
        let acknowledged = BTreeSet::from(["op-1".into()]);
        barrier
            .validate_state(&1.into(), &3.into(), &acknowledged)
            .unwrap();
        assert!(barrier
            .validate_state(&1.into(), &4.into(), &acknowledged)
            .is_err());
        assert!(barrier
            .validate_state(&1.into(), &3.into(), &BTreeSet::new())
            .is_err());
    }
}
