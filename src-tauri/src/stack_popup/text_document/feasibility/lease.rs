//! P02 emits the frozen P01 ViewLease directly; no second wire schema.
use super::contract::{
    DecimalU64, LeaseBoundary, LineBreakKind, LineBreakSpan, LineCertainty, ProtocolErrorCode,
    SessionEnvelope, TextEncoding, ViewLease, ViewLeaseSegment, MAX_LEASE_BYTES, PROJECTION_ROWS,
    PROJECTION_UTF16_UNITS,
};
use super::{
    decode::{encode_insert, Checkpoint, Scalar},
    failure,
    index::PagedDocument,
    Result,
};

pub(crate) struct LeaseTags<'a> {
    pub session: &'a str,
    pub lease: &'a str,
    pub source: u64,
    pub revision: u64,
    pub view: u64,
}

/// Bounded decoding may read past the last displayed scalar; only real decoded
/// scalar boundaries enter the lease. A CR held for its next byte is not exposed.
pub(crate) fn build_lease(
    tags: LeaseTags<'_>,
    mut checkpoint: Checkpoint,
    bytes: &[u8],
    eof: bool,
    line: Option<u64>,
) -> Result<ViewLease> {
    if bytes.len() > MAX_LEASE_BYTES {
        return Err(failure(
            ProtocolErrorCode::ResourceLimit,
            "lease byte input exceeds bound",
        ));
    }
    let mut segment = ViewLeaseSegment {
        segment_id: format!("{}-0", tags.lease),
        source_byte_start: checkpoint.next_byte.into(),
        source_byte_end: checkpoint.next_byte.into(),
        local_utf16_start: 0,
        local_utf16_end: 0,
        text: String::new(),
        boundaries: Vec::new(),
        line_breaks: Vec::new(),
        line_start: line.map(DecimalU64::from),
        line_count: None,
    };
    let mut full = false;
    let decoded = checkpoint.feed(bytes, eof, |scalar: Scalar| {
        if full {
            return Ok(());
        }
        let units = scalar.value.len_utf16();
        if segment.local_utf16_end + units > PROJECTION_UTF16_UNITS
            || scalar.newline.is_some() && segment.line_breaks.len() + 1 >= PROJECTION_ROWS
        {
            full = true;
            return Ok(());
        }
        if segment.boundaries.is_empty() {
            segment.source_byte_start = scalar.byte_start.into();
            segment.boundaries.push(LeaseBoundary {
                local_utf16_offset: 0,
                source_byte_offset: scalar.byte_start.into(),
            });
        }
        if let Some(kind) = scalar.newline {
            segment.line_breaks.push(LineBreakSpan {
                local_utf16_offset: segment.local_utf16_end,
                source_byte_start: scalar.byte_start.into(),
                source_byte_length: u8::try_from(scalar.byte_end - scalar.byte_start).map_err(
                    |_| {
                        failure(
                            ProtocolErrorCode::InvalidTextBoundary,
                            "newline span invalid",
                        )
                    },
                )?,
                kind,
            });
        }
        segment.text.push(scalar.value);
        segment.local_utf16_end += units;
        segment.source_byte_end = scalar.byte_end.into();
        segment.boundaries.push(LeaseBoundary {
            local_utf16_offset: segment.local_utf16_end,
            source_byte_offset: scalar.byte_end.into(),
        });
        Ok(())
    });
    if let Err(error) = decoded {
        if error.code != ProtocolErrorCode::EncodingRequired {
            return Err(error);
        }
    }
    if segment.boundaries.is_empty() {
        segment.boundaries.push(LeaseBoundary {
            local_utf16_offset: 0,
            source_byte_offset: segment.source_byte_start.clone(),
        });
    }
    segment.line_count = Some((segment.line_breaks.len() as u64 + 1).into());
    let invalid_at = checkpoint.invalid_at.map(DecimalU64::from);
    let lease = ViewLease {
        lease_id: tags.lease.to_string(),
        session_id: tags.session.to_string(),
        source_generation: tags.source.into(),
        document_revision: tags.revision.into(),
        view_generation: tags.view.into(),
        source_byte_start: segment.source_byte_start.clone(),
        source_byte_end: segment.source_byte_end.clone(),
        local_utf16_length: segment.local_utf16_end,
        line_certainty: if line.is_some() {
            LineCertainty::Exact
        } else {
            LineCertainty::Unknown
        },
        source_state: if invalid_at.is_some() {
            super::contract::SourceState::DecisionRequired
        } else {
            super::contract::SourceState::Ready
        },
        invalid_at,
        segments: vec![segment],
        expires_after_revision: tags.revision.into(),
    };
    lease.validate()?;
    Ok(lease)
}

pub(crate) fn insertion_newline(lease: &ViewLease) -> LineBreakKind {
    lease
        .segments
        .iter()
        .find_map(|segment| segment.line_breaks.first().map(|span| span.kind))
        .unwrap_or(LineBreakKind::Crlf)
}

/// Internal experiment helper. Caller authorization belongs to the native entry;
/// every local byte conversion still validates frozen source/revision/view tags.
pub(crate) fn replace_local(
    document: &mut PagedDocument,
    lease: &ViewLease,
    envelope: &SessionEnvelope,
    actual_window: &tauri::WebviewWindow,
    start: usize,
    end: usize,
    text: &str,
    encoding: TextEncoding,
    newline: LineBreakKind,
) -> Result<()> {
    lease.authorize_for(envelope, actual_window.label())?;
    replace_validated_local(document, lease, start, end, text, encoding, newline)
}
pub(super) fn replace_validated_local(
    document: &mut PagedDocument,
    lease: &ViewLease,
    start: usize,
    end: usize,
    text: &str,
    encoding: TextEncoding,
    newline: LineBreakKind,
) -> Result<()> {
    lease.validate()?;
    if lease.document_revision.as_u64() != document.revision {
        return Err(failure(
            ProtocolErrorCode::StaleRevision,
            "local edit lease is not the accepted document revision",
        ));
    }
    let start = lease.local_to_byte(start)?.as_u64();
    let end = lease.local_to_byte(end)?.as_u64();
    let bytes = encode_insert(text, encoding, newline)?;
    document.replace_bytes(start, end, &bytes)
}
