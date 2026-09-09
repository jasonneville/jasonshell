//! Executable stack-text-editor.v2 contract. All global arithmetic stays checked u64.
#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

pub const SCHEMA: &str = "stack-text-editor.v2";
pub const PROJECTION_UNITS: usize = 131_072;
pub const MAX_ROWS: usize = 4096;
pub const MAX_SEGMENTS: usize = 2048;

// Missing and explicit null are distinct wire shapes; serde's default Option accepts both.
fn required_nullable<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
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
    InvalidRequest,
    RetryRetired,
    ContextRequired,
}
type Result<T> = std::result::Result<T, ErrorCode>;
pub fn decimal(value: &str) -> Result<u64> {
    if value.is_empty()
        || value.len() > 20
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(ErrorCode::InvalidRequest);
    }
    value.parse().map_err(|_| ErrorCode::InvalidRequest)
}
fn id(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 96
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(ErrorCode::InvalidRequest);
    }
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Utf8,
    Utf16le,
    Utf16be,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Eol {
    Lf,
    Cr,
    Crlf,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Newline {
    pub local_offset: usize,
    pub kind: Eol,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Segment {
    pub encoding: Encoding,
    pub byte_start: String,
    pub text: String,
    pub newlines: Vec<Newline>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Line {
    pub state: String,
    #[serde(deserialize_with = "required_nullable")]
    pub count: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Context {
    pub before: String,
    pub after: String,
    #[serde(deserialize_with = "required_nullable")]
    pub continuation_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lease {
    pub schema: String,
    pub session_id: String,
    pub source_generation: String,
    pub document_revision: String,
    pub view_generation: String,
    pub lease_id: String,
    pub segments: Vec<Segment>,
    pub line: Line,
    pub context: Context,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionVersion {
    pub session_id: String,
    pub source_generation: String,
    pub document_revision: String,
    pub input_sequence: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Affinity {
    Before,
    After,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Endpoint {
    pub byte: String,
    pub affinity: Affinity,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub session_id: String,
    pub source_generation: String,
    pub document_revision: String,
    pub input_sequence: String,
    pub selection_id: String,
    pub anchor: Endpoint,
    pub head: Endpoint,
    pub direction: String,
    pub state: String,
    pub expires_after_revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Barrier {
    pub document_revision: String,
    pub input_sequence: String,
    pub operation_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "lowercase", deny_unknown_fields)]
pub enum Receipt {
    Pending,
    Accepted {
        #[serde(rename = "documentRevision")]
        document_revision: String,
    },
    Rejected {
        code: ErrorCode,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WirePoint {
    pub lease_id: String,
    pub view_generation: String,
    pub segment: usize,
    pub offset: usize,
    pub affinity: Affinity,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Insertion {
    Inline {
        text: String,
    },
    Spool {
        #[serde(rename = "spoolId")]
        spool_id: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    pub schema: String,
    pub session_id: String,
    pub source_generation: String,
    pub request_id: String,
}
macro_rules! requests {
    ($($variant:ident => $wire:literal { $($field:ident : $ty:ty),* }),* $(,)?) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(tag = "command")]
        pub enum Request { $(#[serde(rename = $wire, rename_all = "camelCase")]
            $variant { #[serde(flatten)] envelope: Envelope, $($field: $ty),* }),* }
    };
}
requests! {
    Get => "get_stack_text_session" {},
    Read => "read_stack_text_window" { byte: String, view_generation: String },
    Cancel => "cancel_stack_text_job" { job_id: String },
    Release => "release_stack_text_selection" { selection_id: String },
    Continue => "continue_stack_text_context" { continuation_id: String, view_generation: String },
    Close => "close_stack_text_document" { barrier: Barrier, disposition: String },
    Reload => "reload_stack_text_document" { barrier: Barrier, disposition: String, encoding: Encoding },
    Save => "save_stack_text_document" { barrier: Barrier, operation_id: String },
    SaveAs => "save_stack_text_document_as" { barrier: Barrier, operation_id: String, destination: String },
    Undo => "undo_stack_text_edit" { barrier: Barrier, operation_id: String },
    Redo => "redo_stack_text_edit" { barrier: Barrier, operation_id: String },
    Search => "search_stack_text_document" { barrier: Barrier, query: String, cursor: Option<String> },
    Select => "create_stack_text_selection" { barrier: Barrier, anchor: WirePoint, head: WirePoint },
    Export => "export_stack_text_selection" { barrier: Barrier, selection_id: String, operation_id: String, mode: String },
    Apply => "apply_stack_text_edits" { barrier: Barrier, operation_id: String, selection_id: String, insertion: Insertion },
    Replace => "replace_stack_text_matches" { barrier: Barrier, operation_id: String, selection_id: String, insertion: Insertion },
    Import => "import_stack_text_clipboard" { barrier: Barrier, operation_id: String, selection_id: String, insertion: Insertion },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenRequest {
    pub schema: String,
    pub request_id: String,
    pub path: String,
    pub encoding: Option<Encoding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Session {
    pub session_id: String,
    pub source_generation: String,
    pub document_revision: String,
    pub input_sequence: String,
    pub source_bytes: String,
    pub encoding: Encoding,
    pub bom_bytes: u8,
    pub inserted_eol: Eol,
    pub dirty: bool,
    pub saved_revision: String,
    pub durable_revision: String,
    pub read_only_reason: Option<ErrorCode>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Job {
    pub job_id: String,
    pub session_id: String,
    pub source_generation: String,
    pub document_revision: String,
    pub phase: String,
    pub completed_bytes: String,
    pub total_bytes: Option<String>,
    pub cancellation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Failure {
    pub code: ErrorCode,
    pub retry: String,
    pub disposition: String,
    pub operation_id: Option<String>,
    pub job_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchPage {
    pub selection_ids: Vec<String>,
    pub cursor: Option<String>,
    pub complete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transfer {
    pub spool_id: String,
    pub bytes: String,
    pub sha256: String,
    pub state: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
pub enum ResponseBody {
    Session(Session),
    Lease(Lease),
    Selection(Selection),
    Receipt(Receipt),
    Job(Job),
    Error(Failure),
    Search(SearchPage),
    Transfer(Transfer),
    Released(()),
    Closed(()),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WireResult {
    pub schema: String,
    pub request_id: String,
    pub session_id: Option<String>,
    pub source_generation: Option<String>,
    #[serde(flatten)]
    pub body: ResponseBody,
}
fn decode_exact<T: serde::de::DeserializeOwned + Serialize>(value: Value) -> Result<T> {
    let decoded: T =
        serde_json::from_value(value.clone()).map_err(|_| ErrorCode::InvalidRequest)?;
    if serde_json::to_value(&decoded).map_err(|_| ErrorCode::InvalidRequest)? != value {
        return Err(ErrorCode::InvalidRequest);
    }
    Ok(decoded)
}
pub fn validate_open(value: Value) -> Result<OpenRequest> {
    let request: OpenRequest = decode_exact(value)?;
    if request.schema != SCHEMA || request.path.is_empty() || request.path.contains('\0') {
        return Err(ErrorCode::InvalidRequest);
    }
    if request.path.encode_utf16().count() > 32767 {
        return Err(ErrorCode::ResourceLimit);
    }
    id(&request.request_id)?;
    Ok(request)
}
pub fn validate_result(value: Value) -> Result<WireResult> {
    let result: WireResult = decode_exact(value)?;
    if result.schema != SCHEMA
        || result.session_id.is_none() != result.source_generation.is_none()
        || (!matches!(result.body, ResponseBody::Error(_)) && result.session_id.is_none())
    {
        return Err(ErrorCode::InvalidRequest);
    }
    id(&result.request_id)?;
    if let Some(s) = &result.session_id {
        id(s)?;
    }
    if let Some(g) = &result.source_generation {
        id(g)?;
    }
    let owner = |s: &str, g: &str| -> Result<()> {
        if Some(s) != result.session_id.as_deref() || Some(g) != result.source_generation.as_deref()
        {
            return Err(ErrorCode::Unauthorized);
        }
        Ok(())
    };
    match &result.body {
        ResponseBody::Lease(l) => {
            validate_lease(l)?;
            owner(&l.session_id, &l.source_generation)?;
        }
        ResponseBody::Session(s) => {
            owner(&s.session_id, &s.source_generation)?;
            decimal(&s.input_sequence)?;
            decimal(&s.source_bytes)?;
            let revision = decimal(&s.document_revision)?;
            if decimal(&s.saved_revision)? > revision
                || decimal(&s.durable_revision)? > revision
                || (s.bom_bytes != 0
                    && s.bom_bytes != if s.encoding == Encoding::Utf8 { 3 } else { 2 })
            {
                return Err(ErrorCode::InvalidRequest);
            }
        }
        ResponseBody::Selection(s) => {
            owner(&s.session_id, &s.source_generation)?;
            id(&s.selection_id)?;
            decimal(&s.input_sequence)?;
            let (revision, expires, anchor, head) = (
                decimal(&s.document_revision)?,
                decimal(&s.expires_after_revision)?,
                decimal(&s.anchor.byte)?,
                decimal(&s.head.byte)?,
            );
            if !["active", "invalidated"].contains(&s.state.as_str())
                || !["forward", "backward"].contains(&s.direction.as_str())
                || (s.state == "active"
                    && (expires < revision
                        || s.direction != if anchor > head { "backward" } else { "forward" }))
            {
                return Err(ErrorCode::InvalidRequest);
            }
        }
        ResponseBody::Receipt(Receipt::Accepted { document_revision }) => {
            decimal(document_revision)?;
        }
        ResponseBody::Receipt(_) => (),
        ResponseBody::Job(j) => {
            owner(&j.session_id, &j.source_generation)?;
            id(&j.job_id)?;
            decimal(&j.document_revision)?;
            let completed = decimal(&j.completed_bytes)?;
            if let Some(total) = &j.total_bytes {
                if completed > decimal(total)? {
                    return Err(ErrorCode::InvalidRequest);
                }
            }
            if ![
                "queued",
                "reading",
                "staging",
                "flushing",
                "publishing",
                "published",
                "durable",
                "failed",
                "cancelled",
                "ambiguous",
            ]
            .contains(&j.phase.as_str())
                || ![
                    "none",
                    "requested",
                    "removed",
                    "acknowledged",
                    "already-published",
                    "ambiguous",
                ]
                .contains(&j.cancellation.as_str())
                || (["published", "durable"].contains(&j.phase.as_str())
                    && !["none", "already-published"].contains(&j.cancellation.as_str()))
                || (["removed", "acknowledged"].contains(&j.cancellation.as_str())
                    && j.phase != "cancelled")
                || ((j.phase == "ambiguous") != (j.cancellation == "ambiguous"))
            {
                return Err(ErrorCode::InvalidRequest);
            }
        }
        ResponseBody::Error(e) => {
            if let Some(s) = &e.operation_id {
                id(s)?;
            }
            if let Some(s) = &e.job_id {
                id(s)?;
            }
            if !["never", "same-request", "new-request"].contains(&e.retry.as_str())
                || !["unchanged", "retained", "ambiguous"].contains(&e.disposition.as_str())
                || ((e.code == ErrorCode::PublicationAmbiguous) != (e.disposition == "ambiguous"))
                || (e.code == ErrorCode::PublicationAmbiguous && e.retry != "never")
            {
                return Err(ErrorCode::InvalidRequest);
            }
        }
        ResponseBody::Search(s) => {
            if s.selection_ids.len() > 256 {
                return Err(ErrorCode::ResourceLimit);
            }
            let mut seen = HashSet::new();
            for selection in &s.selection_ids {
                id(selection)?;
                if !seen.insert(selection) {
                    return Err(ErrorCode::InvalidRequest);
                }
            }
            if s.complete != s.cursor.is_none() {
                return Err(ErrorCode::InvalidRequest);
            }
            if let Some(c) = &s.cursor {
                id(c)?;
            }
        }
        ResponseBody::Transfer(t) => {
            id(&t.spool_id)?;
            decimal(&t.bytes)?;
            if t.sha256.len() != 64
                || !t
                    .sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || !["staged", "ready"].contains(&t.state.as_str())
            {
                return Err(ErrorCode::InvalidRequest);
            }
        }
        ResponseBody::Released(_) | ResponseBody::Closed(_) => (),
    }
    Ok(result)
}

pub fn resolve_point<'a>(
    point: &WirePoint,
    lease: &'a Lease,
) -> Result<(&'a Lease, usize, usize, Affinity)> {
    if point.lease_id != lease.lease_id || point.view_generation != lease.view_generation {
        return Err(ErrorCode::StaleRevision);
    }
    lease_byte(lease, point.segment, point.offset)?;
    Ok((lease, point.segment, point.offset, point.affinity))
}
pub fn remap_selection(
    selection: &Selection,
    state: &SessionVersion,
    from: &str,
    to: &str,
    inserted: &str,
) -> Result<Selection> {
    if selection.session_id != state.session_id {
        return Err(ErrorCode::Unauthorized);
    }
    if selection.source_generation != state.source_generation {
        return Err(ErrorCode::SourceChanged);
    }
    let (revision, previous) = (
        decimal(&state.document_revision)?,
        decimal(&selection.document_revision)?,
    );
    if revision <= previous {
        return Err(ErrorCode::StaleRevision);
    }
    let mut next = selection.clone();
    next.document_revision = state.document_revision.clone();
    next.input_sequence = state.input_sequence.clone();
    if selection.state == "invalidated" || revision > decimal(&selection.expires_after_revision)? {
        next.state = "invalidated".into();
        return Ok(next);
    }
    if Some(revision) != previous.checked_add(1) {
        return Err(ErrorCode::StaleRevision);
    }
    next.anchor = remap_endpoint(&selection.anchor, from, to, inserted)?;
    next.head = remap_endpoint(&selection.head, from, to, inserted)?;
    next.direction = if decimal(&next.anchor.byte)? > decimal(&next.head.byte)? {
        "backward"
    } else {
        "forward"
    }
    .into();
    Ok(next)
}

pub fn segment_boundaries(segment: &Segment) -> Result<BTreeMap<usize, u64>> {
    if segment.text.encode_utf16().count() > PROJECTION_UNITS || segment.newlines.len() >= MAX_ROWS
    {
        return Err(ErrorCode::ResourceLimit);
    }
    let mut eols = BTreeMap::new();
    let mut previous = None;
    for eol in &segment.newlines {
        if previous.is_some_and(|p| eol.local_offset <= p) {
            return Err(ErrorCode::InvalidTextBoundary);
        }
        eols.insert(eol.local_offset, eol.kind);
        previous = Some(eol.local_offset);
    }
    let mut byte = decimal(&segment.byte_start)?;
    let mut local = 0;
    let mut boundaries = BTreeMap::from([(0, byte)]);
    for scalar in segment.text.chars() {
        let eol = eols.remove(&local);
        if scalar == '\r' || (scalar == '\n') != eol.is_some() {
            return Err(ErrorCode::InvalidTextBoundary);
        }
        let width = match segment.encoding {
            Encoding::Utf8 => scalar.len_utf8(),
            _ => scalar.len_utf16() * 2,
        };
        byte = byte
            .checked_add((width * if eol == Some(Eol::Crlf) { 2 } else { 1 }) as u64)
            .ok_or(ErrorCode::InvalidRequest)?;
        local += scalar.len_utf16();
        boundaries.insert(local, byte);
    }
    if !eols.is_empty() {
        return Err(ErrorCode::InvalidTextBoundary);
    }
    Ok(boundaries)
}
pub fn validate_lease(lease: &Lease) -> Result<()> {
    if lease.schema != SCHEMA {
        return Err(ErrorCode::InvalidRequest);
    }
    id(&lease.session_id)?;
    id(&lease.source_generation)?;
    id(&lease.lease_id)?;
    decimal(&lease.document_revision)?;
    decimal(&lease.view_generation)?;
    if lease.segments.is_empty() || lease.segments.len() > MAX_SEGMENTS {
        return Err(ErrorCode::ResourceLimit);
    }
    let (mut units, mut rows, mut end) = (0, 1, None);
    for segment in &lease.segments {
        let boundaries = segment_boundaries(segment)?;
        if end.is_some_and(|e| Some(&e) != boundaries.get(&0)) {
            return Err(ErrorCode::InvalidTextBoundary);
        }
        end = boundaries.last_key_value().map(|(_, byte)| *byte);
        units += segment.text.encode_utf16().count();
        rows += segment.newlines.len();
        if units > PROJECTION_UNITS || rows > MAX_ROWS {
            return Err(ErrorCode::ResourceLimit);
        }
    }
    match lease.line.state.as_str() {
        "exact"
            if decimal(
                lease
                    .line
                    .count
                    .as_deref()
                    .ok_or(ErrorCode::InvalidRequest)?,
            )? > 0 =>
        {
            ()
        }
        "unknown" | "indexing" if lease.line.count.is_none() => (),
        _ => return Err(ErrorCode::InvalidRequest),
    }
    if !["complete", "continued"].contains(&lease.context.before.as_str())
        || !["complete", "continued"].contains(&lease.context.after.as_str())
    {
        return Err(ErrorCode::InvalidRequest);
    }
    if lease.context.before == "continued" || lease.context.after == "continued" {
        id(lease
            .context
            .continuation_id
            .as_deref()
            .ok_or(ErrorCode::InvalidRequest)?)?;
    } else if lease.context.continuation_id.is_some() {
        return Err(ErrorCode::InvalidRequest);
    }
    Ok(())
}
pub fn lease_byte(lease: &Lease, segment: usize, offset: usize) -> Result<u64> {
    validate_lease(lease)?;
    let segment = lease
        .segments
        .get(segment)
        .ok_or(ErrorCode::InvalidRequest)?;
    segment_boundaries(segment)?
        .get(&offset)
        .copied()
        .ok_or(ErrorCode::InvalidTextBoundary)
}
pub fn require_complete_context(lease: &Lease) -> Result<()> {
    validate_lease(lease)?;
    if lease.context.before != "complete" || lease.context.after != "complete" {
        return Err(ErrorCode::ContextRequired);
    }
    Ok(())
}
pub fn create_selection(
    state: &SessionVersion,
    selection_id: &str,
    anchor: (&Lease, usize, usize, Affinity),
    head: (&Lease, usize, usize, Affinity),
) -> Result<Selection> {
    id(selection_id)?;
    let endpoint =
        |(lease, segment, offset, affinity): (&Lease, usize, usize, Affinity)| -> Result<Endpoint> {
            if lease.session_id != state.session_id {
                return Err(ErrorCode::Unauthorized);
            }
            if lease.source_generation != state.source_generation {
                return Err(ErrorCode::SourceChanged);
            }
            if lease.document_revision != state.document_revision {
                return Err(ErrorCode::StaleRevision);
            }
            Ok(Endpoint {
                byte: lease_byte(lease, segment, offset)?.to_string(),
                affinity,
            })
        };
    let (anchor, head) = (endpoint(anchor)?, endpoint(head)?);
    let direction = if decimal(&anchor.byte)? > decimal(&head.byte)? {
        "backward"
    } else {
        "forward"
    };
    Ok(Selection {
        session_id: state.session_id.clone(),
        source_generation: state.source_generation.clone(),
        document_revision: state.document_revision.clone(),
        input_sequence: state.input_sequence.clone(),
        selection_id: selection_id.into(),
        anchor,
        head,
        direction: direction.into(),
        state: "active".into(),
        expires_after_revision: decimal(&state.document_revision)?
            .saturating_add(256)
            .to_string(),
    })
}
/// Arithmetic only, not a wire validator. The document owner must validate
/// endpoint/edit boundaries against its encoding and CRLF map before calling.
pub fn remap_endpoint(
    endpoint: &Endpoint,
    from: &str,
    to: &str,
    inserted: &str,
) -> Result<Endpoint> {
    let (p, start, end, inserted) = (
        decimal(&endpoint.byte)?,
        decimal(from)?,
        decimal(to)?,
        decimal(inserted)?,
    );
    if end < start {
        return Err(ErrorCode::InvalidRequest);
    }
    let next = if p < start {
        Some(p)
    } else if p > end {
        (p - (end - start)).checked_add(inserted)
    } else {
        start.checked_add(if endpoint.affinity == Affinity::After {
            inserted
        } else {
            0
        })
    }
    .ok_or(ErrorCode::InvalidRequest)?;
    Ok(Endpoint {
        byte: next.to_string(),
        affinity: endpoint.affinity,
    })
}
pub struct OperationLedger {
    active: HashMap<String, (String, Receipt)>,
    retired: HashSet<String>,
    capacity: usize,
}
impl OperationLedger {
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity > 1024 {
            return Err(ErrorCode::InvalidRequest);
        }
        Ok(Self {
            active: HashMap::new(),
            retired: HashSet::new(),
            capacity,
        })
    }
    pub fn begin(&mut self, operation_id: &str, digest: &str) -> Result<Option<Receipt>> {
        id(operation_id)?;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ErrorCode::InvalidRequest);
        }
        if self.retired.contains(operation_id) {
            return Err(ErrorCode::RetryRetired);
        }
        if let Some((previous, receipt)) = self.active.get(operation_id) {
            return if previous == digest {
                Ok(Some(receipt.clone()))
            } else {
                Err(ErrorCode::InvalidRequest)
            };
        }
        if self.active.len() >= self.capacity {
            return Err(ErrorCode::ResourceLimit);
        }
        self.active
            .insert(operation_id.into(), (digest.into(), Receipt::Pending));
        Ok(None)
    }
    pub fn finish(&mut self, operation_id: &str, receipt: Receipt) -> Result<()> {
        if let Receipt::Accepted { document_revision } = &receipt {
            decimal(document_revision)?;
        }
        let record = self
            .active
            .get_mut(operation_id)
            .ok_or(ErrorCode::InvalidRequest)?;
        if record.1 != Receipt::Pending || receipt == Receipt::Pending {
            return Err(ErrorCode::InvalidRequest);
        }
        record.1 = receipt;
        Ok(())
    }
    pub fn accepted(&self, operation_id: &str, revision: &str) -> bool {
        matches!(self.active.get(operation_id), Some((_, Receipt::Accepted { document_revision })) if decimal(document_revision).ok().zip(decimal(revision).ok()).is_some_and(|(a,b)| a <= b))
    }
    pub fn retire(&mut self, operation_id: &str, durable_revision: &str) -> Result<()> {
        if !self.accepted(operation_id, durable_revision) {
            return Err(ErrorCode::StaleRevision);
        }
        if self.retired.len() >= self.capacity {
            return Err(ErrorCode::ResourceLimit);
        }
        self.retired.insert(operation_id.into());
        self.active.remove(operation_id);
        Ok(())
    }
}
fn validate_barrier(barrier: &Barrier) -> Result<()> {
    decimal(&barrier.document_revision)?;
    decimal(&barrier.input_sequence)?;
    if barrier.operation_ids.len() > 64 {
        return Err(ErrorCode::ResourceLimit);
    }
    let mut seen = HashSet::new();
    for operation in &barrier.operation_ids {
        id(operation)?;
        if !seen.insert(operation) {
            return Err(ErrorCode::InvalidRequest);
        }
    }
    Ok(())
}
pub fn assert_barrier(
    state: &SessionVersion,
    barrier: &Barrier,
    ledger: &OperationLedger,
) -> Result<()> {
    validate_barrier(barrier)?;
    if barrier.document_revision != state.document_revision
        || barrier.input_sequence != state.input_sequence
        || barrier
            .operation_ids
            .iter()
            .any(|operation| !ledger.accepted(operation, &state.document_revision))
    {
        return Err(ErrorCode::StaleRevision);
    }
    Ok(())
}
pub fn validate_request(value: Value) -> Result<Request> {
    let obj = value.as_object().ok_or(ErrorCode::InvalidRequest)?;
    let command = obj
        .get("command")
        .and_then(Value::as_str)
        .ok_or(ErrorCode::InvalidRequest)?;
    let fields: &[&str] = match command {
        "get_stack_text_session" => &[],
        "read_stack_text_window" => &["byte", "viewGeneration"],
        "cancel_stack_text_job" => &["jobId"],
        "release_stack_text_selection" => &["selectionId"],
        "continue_stack_text_context" => &["continuationId", "viewGeneration"],
        "close_stack_text_document" => &["barrier", "disposition"],
        "reload_stack_text_document" => &["barrier", "disposition", "encoding"],
        "save_stack_text_document" | "undo_stack_text_edit" | "redo_stack_text_edit" => {
            &["barrier", "operationId"]
        }
        "save_stack_text_document_as" => &["barrier", "operationId", "destination"],
        "search_stack_text_document" => &["barrier", "query", "cursor"],
        "create_stack_text_selection" => &["barrier", "anchor", "head"],
        "export_stack_text_selection" => &["barrier", "selectionId", "operationId", "mode"],
        "apply_stack_text_edits" | "replace_stack_text_matches" | "import_stack_text_clipboard" => {
            &["barrier", "operationId", "selectionId", "insertion"]
        }
        _ => return Err(ErrorCode::InvalidRequest),
    };
    let common = [
        "schema",
        "sessionId",
        "sourceGeneration",
        "requestId",
        "command",
    ];
    if obj.len() != common.len() + fields.len()
        || obj
            .keys()
            .any(|k| !common.contains(&k.as_str()) && !fields.contains(&k.as_str()))
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let request: Request =
        serde_json::from_value(value.clone()).map_err(|_| ErrorCode::InvalidRequest)?;
    if obj.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(ErrorCode::InvalidRequest);
    }
    for key in ["sessionId", "sourceGeneration", "requestId"] {
        id(obj[key].as_str().ok_or(ErrorCode::InvalidRequest)?)?;
    }
    for key in fields {
        let v = obj.get(*key).ok_or(ErrorCode::InvalidRequest)?;
        let string = || v.as_str().ok_or(ErrorCode::InvalidRequest);
        match *key {
            "barrier" => validate_barrier(
                &serde_json::from_value(v.clone()).map_err(|_| ErrorCode::InvalidRequest)?,
            )?,
            "byte" | "viewGeneration" => {
                decimal(string()?)?;
            }
            "jobId" | "operationId" | "selectionId" | "continuationId" => id(string()?)?,
            "cursor" => {
                if !v.is_null() {
                    id(string()?)?;
                }
            }
            "disposition" => {
                if !["discard", "cancel"].contains(&string()?) {
                    return Err(ErrorCode::InvalidRequest);
                }
            }
            "mode" => {
                if !["copy", "cut", "export"].contains(&string()?) {
                    return Err(ErrorCode::InvalidRequest);
                }
            }
            "destination" | "query" => {
                let s = string()?;
                if s.encode_utf16().count() > if *key == "query" { 4096 } else { 32767 } {
                    return Err(ErrorCode::ResourceLimit);
                }
                if s.is_empty() || (*key == "destination" && s.contains('\0')) {
                    return Err(ErrorCode::InvalidRequest);
                }
            }
            "anchor" | "head" => {
                let p: WirePoint =
                    serde_json::from_value(v.clone()).map_err(|_| ErrorCode::InvalidRequest)?;
                id(&p.lease_id)?;
                decimal(&p.view_generation)?;
                if p.segment >= MAX_SEGMENTS || p.offset > PROJECTION_UNITS {
                    return Err(ErrorCode::InvalidRequest);
                }
            }
            "insertion" => match serde_json::from_value::<Insertion>(v.clone())
                .map_err(|_| ErrorCode::InvalidRequest)?
            {
                Insertion::Inline { text } => {
                    if text.encode_utf16().count() > 524288 {
                        return Err(ErrorCode::ResourceLimit);
                    }
                }
                Insertion::Spool { spool_id } => id(&spool_id)?,
            },
            _ => (),
        }
    }
    Ok(request)
}

/// Canonical ASCII JSON for recipient-computed SHA-256. No caller-provided
/// fingerprint is authoritative. Object keys sort lexically; array order stays.
pub fn canonical_request(value: Value) -> Result<String> {
    validate_request(value.clone())?;
    fn sorted(value: Value) -> Value {
        match value {
            Value::Object(map) => Value::Object(
                map.into_iter()
                    .collect::<std::collections::BTreeMap<_, _>>()
                    .into_iter()
                    .map(|(key, value)| (key, sorted(value)))
                    .collect(),
            ),
            Value::Array(values) => Value::Array(values.into_iter().map(sorted).collect()),
            value => value,
        }
    }
    let json = serde_json::to_string(&sorted(value)).map_err(|_| ErrorCode::InvalidRequest)?;
    let mut ascii = String::new();
    for ch in json.chars() {
        if ch < '\u{7f}' {
            ascii.push(ch);
        } else {
            for unit in ch.encode_utf16(&mut [0; 2]) {
                use std::fmt::Write;
                write!(ascii, "\\u{unit:04x}").map_err(|_| ErrorCode::InvalidRequest)?;
            }
        }
    }
    Ok(ascii)
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_canonical_request_bytes() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/stack-text-request-v2.json"
        ))
        .unwrap();
        assert_eq!(
            super::canonical_request(fixture["request"].clone()).unwrap(),
            fixture["canonical"].as_str().unwrap()
        );
    }
    use super::*;
    use serde_json::json;
    fn vectors() -> Value {
        serde_json::from_str(include_str!(
            "../../../../tests/fixtures/stack-text-v2.json"
        ))
        .unwrap()
    }
    fn lease() -> Lease {
        serde_json::from_value(json!({"schema":SCHEMA,"sessionId":"s1","sourceGeneration":"g1","documentRevision":"0","viewGeneration":"1","leaseId":"l1","segments":[{"encoding":"utf8","byteStart":"0","text":"abc","newlines":[]}],"line":{"state":"unknown","count":null},"context":{"before":"complete","after":"complete","continuationId":null}})).unwrap()
    }
    fn state() -> SessionVersion {
        SessionVersion {
            session_id: "s1".into(),
            source_generation: "g1".into(),
            document_revision: "0".into(),
            input_sequence: "0".into(),
        }
    }
    #[test]
    fn shared_lossless_integers() {
        let v = vectors();
        for n in v["integers"]["valid"].as_array().unwrap() {
            assert!(decimal(n.as_str().unwrap()).is_ok());
        }
        for n in v["integers"]["invalid"].as_array().unwrap() {
            assert!(n.as_str().map(decimal).is_none_or(|r| r.is_err()));
        }
    }
    #[test]
    fn shared_scalar_crlf_utf16_maps() {
        for mut v in vectors()["segments"].as_array().unwrap().clone() {
            let offsets = v.as_object_mut().unwrap().remove("offsets").unwrap();
            let bytes = v.as_object_mut().unwrap().remove("bytes").unwrap();
            let segment: Segment = serde_json::from_value(v).unwrap();
            let map = segment_boundaries(&segment).unwrap();
            for (offset, byte) in offsets
                .as_array()
                .unwrap()
                .iter()
                .zip(bytes.as_array().unwrap())
            {
                assert_eq!(
                    map[&(offset.as_u64().unwrap() as usize)],
                    decimal(byte.as_str().unwrap()).unwrap()
                );
            }
            assert!(!map.contains_key(&2));
        }
    }
    #[test]
    fn newline_forgery_overflow_and_unknown_fields_reject() {
        let mut l = lease();
        l.segments[0].text = "\n".into();
        assert!(validate_lease(&l).is_err());
        l.segments[0].newlines.push(Newline {
            local_offset: 0,
            kind: Eol::Crlf,
        });
        assert!(validate_lease(&l).is_ok());
        l.segments[0].byte_start = u64::MAX.to_string();
        assert!(validate_lease(&l).is_err());
        assert!(serde_json::from_str::<Segment>(
            r#"{"encoding":"utf8","byteStart":"0","text":"\ud800","newlines":[]}"#
        )
        .is_err());
        assert!(serde_json::from_value::<Segment>(
            json!({"encoding":"utf8","byteStart":"0","text":"a","newlines":[],"extra":1})
        )
        .is_err());
    }
    #[test]
    fn global_rows_and_projection_are_bounded() {
        let mut l = lease();
        l.segments[0].text = "\n".repeat(4095);
        l.segments[0].newlines = (0..4095)
            .map(|local_offset| Newline {
                local_offset,
                kind: Eol::Lf,
            })
            .collect();
        assert!(validate_lease(&l).is_ok());
        l.segments.push(Segment {
            encoding: Encoding::Utf8,
            byte_start: "4095".into(),
            text: "\n".into(),
            newlines: vec![Newline {
                local_offset: 0,
                kind: Eol::Lf,
            }],
        });
        assert_eq!(validate_lease(&l), Err(ErrorCode::ResourceLimit));
    }
    #[test]
    fn combined_cross_lease_selection_checks_ownership_revision_direction() {
        let a = lease();
        let mut b = lease();
        b.lease_id = "l2".into();
        b.segments[0].byte_start = "9007199254740993".into();
        let s = create_selection(
            &state(),
            "r1",
            (&b, 0, 2, Affinity::After),
            (&a, 0, 1, Affinity::Before),
        )
        .unwrap();
        assert_eq!(s.anchor.byte, "9007199254740995");
        assert_eq!(s.direction, "backward");
        b.session_id = "forged".into();
        assert!(create_selection(
            &state(),
            "r1",
            (&b, 0, 2, Affinity::After),
            (&a, 0, 1, Affinity::Before)
        )
        .is_err());
    }
    #[test]
    fn affinity_remapping_is_checked() {
        let p = Endpoint {
            byte: "5".into(),
            affinity: Affinity::After,
        };
        assert_eq!(remap_endpoint(&p, "5", "5", "3").unwrap().byte, "8");
        assert!(remap_endpoint(&p, "0", "0", &u64::MAX.to_string()).is_err());
    }
    #[test]
    fn incomplete_context_never_certifies_grapheme_boundary() {
        let mut l = lease();
        l.context.after = "continued".into();
        l.context.continuation_id = Some("c1".into());
        assert_eq!(
            require_complete_context(&l),
            Err(ErrorCode::ContextRequired)
        );
        l.context.continuation_id = None;
        assert!(validate_lease(&l).is_err());
    }
    #[test]
    fn exact_once_retirement_and_barriers() {
        let mut ledger = OperationLedger::new(2).unwrap();
        let digest = "a".repeat(64);
        assert_eq!(ledger.begin("o1", &digest).unwrap(), None);
        assert_eq!(ledger.begin("o1", &digest).unwrap(), Some(Receipt::Pending));
        assert!(ledger.begin("o1", &"b".repeat(64)).is_err());
        ledger
            .finish(
                "o1",
                Receipt::Accepted {
                    document_revision: "1".into(),
                },
            )
            .unwrap();
        assert!(ledger.retire("o1", "0").is_err());
        ledger.retire("o1", "1").unwrap();
        assert_eq!(ledger.begin("o1", &digest), Err(ErrorCode::RetryRetired));
        let mut barrier = Barrier {
            document_revision: "0".into(),
            input_sequence: "0".into(),
            operation_ids: vec![],
        };
        assert!(assert_barrier(&state(), &barrier, &ledger).is_ok());
        barrier.input_sequence = "1".into();
        assert!(assert_barrier(&state(), &barrier, &ledger).is_err());
    }
    #[test]
    fn camel_case_requests_reject_missing_barrier_and_extra_insertion_fields() {
        let mut req = json!({"schema":SCHEMA,"sessionId":"s1","sourceGeneration":"g1","requestId":"q1","command":"close_stack_text_document","barrier":{"documentRevision":"0","inputSequence":"0","operationIds":[]},"disposition":"cancel"});
        assert!(validate_request(req.clone()).is_ok());
        req.as_object_mut().unwrap().remove("barrier");
        assert!(validate_request(req).is_err());
        assert!(serde_json::from_value::<Insertion>(
            json!({"kind":"inline","text":"a","spoolId":"x"})
        )
        .is_err());
    }
    #[test]
    fn shared_result_wire_vectors_and_explicit_nulls() {
        let vectors: Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/stack-text-results-v2.json"
        ))
        .unwrap();
        for (group, valid) in [("valid", true), ("invalid", false)] {
            for body in vectors[group].as_array().unwrap() {
                let mut value = json!({"schema":SCHEMA,"requestId":"q1","sessionId":"s1","sourceGeneration":"g1"});
                value
                    .as_object_mut()
                    .unwrap()
                    .extend(body.as_object().unwrap().clone());
                assert_eq!(validate_result(value.clone()).is_ok(), valid, "{value}");
            }
        }
        let mut value = serde_json::to_value(lease()).unwrap();
        value["line"].as_object_mut().unwrap().remove("count");
        assert!(serde_json::from_value::<Lease>(value).is_err());
    }
    #[test]
    fn open_validation_and_selection_lifecycle() {
        let mut request =
            json!({"schema":SCHEMA,"requestId":"q1","path":"C:\\test.txt","encoding":null});
        assert!(validate_open(request.clone()).is_ok());
        request.as_object_mut().unwrap().remove("encoding");
        assert!(validate_open(request).is_err());
        let l = lease();
        let point = WirePoint {
            lease_id: "l1".into(),
            view_generation: "1".into(),
            segment: 0,
            offset: 2,
            affinity: Affinity::After,
        };
        assert!(resolve_point(&point, &l).is_ok());
        let mut stale = point.clone();
        stale.view_generation = "0".into();
        assert!(resolve_point(&stale, &l).is_err());
        let selection = create_selection(
            &state(),
            "r1",
            (&l, 0, 2, Affinity::After),
            (&l, 0, 0, Affinity::Before),
        )
        .unwrap();
        let mut next = state();
        next.document_revision = "1".into();
        next.input_sequence = "1".into();
        let mapped = remap_selection(&selection, &next, "1", "1", "1").unwrap();
        assert_eq!(mapped.anchor.byte, "3");
        assert_eq!(mapped.head.byte, "0");
        next.document_revision = "257".into();
        assert_eq!(
            remap_selection(&mapped, &next, "0", "0", "0")
                .unwrap()
                .state,
            "invalidated"
        );
    }
}
