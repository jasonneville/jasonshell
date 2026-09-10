//! Incremental strict codecs. CRLF and surrogate/scalar interiors never become
//! editable boundaries. Checkpoints include incomplete scalar/CR state.
use super::contract::{LineBreakKind, ProtocolErrorCode, TextEncoding};
use super::{checked_add, failure, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Checkpoint {
    pub encoding: TextEncoding,
    pub next_byte: u64,
    pub utf16_units: u64,
    pub newlines: u64,
    pending: [u8; 4],
    pending_len: usize,
    pending_start: u64,
    pending_cr: Option<(u64, u64)>,
    pub invalid_at: Option<u64>,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Scalar {
    pub value: char,
    pub byte_start: u64,
    pub byte_end: u64,
    pub newline: Option<LineBreakKind>,
}

pub(crate) fn detect(
    prefix: &[u8],
    selected: Option<TextEncoding>,
) -> Result<(TextEncoding, usize)> {
    if prefix.starts_with(&[0xff, 0xfe, 0, 0]) || prefix.starts_with(&[0, 0, 0xfe, 0xff]) {
        return Err(failure(
            ProtocolErrorCode::EncodingRequired,
            "UTF-32 is not a supported text encoding",
        ));
    }
    let bom = if prefix.starts_with(&[0xef, 0xbb, 0xbf]) {
        Some((TextEncoding::Utf8Bom, 3))
    } else if prefix.starts_with(&[0xff, 0xfe]) {
        Some((TextEncoding::Utf16Le, 2))
    } else if prefix.starts_with(&[0xfe, 0xff]) {
        Some((TextEncoding::Utf16Be, 2))
    } else {
        None
    };
    if let Some((encoding, length)) = bom {
        if selected.is_some_and(|selected| {
            selected != encoding
                && !(selected == TextEncoding::Utf8 && encoding == TextEncoding::Utf8Bom)
        }) {
            return Err(failure(
                ProtocolErrorCode::EncodingRequired,
                "selected encoding conflicts with BOM",
            ));
        }
        return Ok((encoding, length));
    }
    Ok((selected.unwrap_or(TextEncoding::Utf8), 0))
}
impl Checkpoint {
    pub(crate) fn new(encoding: TextEncoding, byte_start: u64) -> Self {
        Self {
            encoding,
            next_byte: byte_start,
            utf16_units: 0,
            newlines: 0,
            pending: [0; 4],
            pending_len: 0,
            pending_start: byte_start,
            pending_cr: None,
            invalid_at: None,
        }
    }
    pub(crate) fn at_boundary(&self) -> bool {
        self.pending_len == 0 && self.pending_cr.is_none() && self.invalid_at.is_none()
    }
    fn unit(&self) -> u16 {
        match self.encoding {
            TextEncoding::Utf16Be => u16::from_be_bytes([self.pending[0], self.pending[1]]),
            _ => u16::from_le_bytes([self.pending[0], self.pending[1]]),
        }
    }
    fn width(&self) -> Result<usize> {
        match self.encoding {
            TextEncoding::Utf8 | TextEncoding::Utf8Bom => match self.pending[0] {
                0..=0x7f => Ok(1),
                0xc2..=0xdf => Ok(2),
                0xe0..=0xef => Ok(3),
                0xf0..=0xf4 => Ok(4),
                _ => Err(failure(
                    ProtocolErrorCode::EncodingRequired,
                    "invalid UTF-8 leading byte",
                )),
            },
            _ if self.pending_len < 2 => Ok(2),
            _ => {
                if (0xd800..=0xdbff).contains(&self.unit()) {
                    Ok(4)
                } else {
                    Ok(2)
                }
            }
        }
    }
    fn scalar(&self) -> Result<char> {
        match self.encoding {
            TextEncoding::Utf8 | TextEncoding::Utf8Bom => {
                std::str::from_utf8(&self.pending[..self.pending_len])
                    .ok()
                    .and_then(|text| text.chars().next())
            }
            _ => {
                let first = self.unit();
                if self.pending_len == 4 {
                    let second = if self.encoding == TextEncoding::Utf16Be {
                        u16::from_be_bytes([self.pending[2], self.pending[3]])
                    } else {
                        u16::from_le_bytes([self.pending[2], self.pending[3]])
                    };
                    if !(0xdc00..=0xdfff).contains(&second) {
                        None
                    } else {
                        char::from_u32(
                            0x10000 + (((first as u32 - 0xd800) << 10) | (second as u32 - 0xdc00)),
                        )
                    }
                } else {
                    char::from_u32(first as u32)
                }
            }
        }
        .ok_or_else(|| {
            failure(
                ProtocolErrorCode::EncodingRequired,
                "invalid scalar; original bytes retained",
            )
        })
    }
    fn emit(&mut self, scalar: Scalar, sink: &mut impl FnMut(Scalar) -> Result<()>) -> Result<()> {
        self.utf16_units = checked_add(self.utf16_units, scalar.value.len_utf16() as u64)?;
        if scalar.newline.is_some() {
            self.newlines = checked_add(self.newlines, 1)?;
        }
        sink(scalar)
    }
    fn normalized(
        &mut self,
        scalar: Scalar,
        sink: &mut impl FnMut(Scalar) -> Result<()>,
    ) -> Result<()> {
        if let Some((start, end)) = self.pending_cr.take() {
            if scalar.value == '\n' {
                return self.emit(
                    Scalar {
                        value: '\n',
                        byte_start: start,
                        byte_end: scalar.byte_end,
                        newline: Some(LineBreakKind::Crlf),
                    },
                    sink,
                );
            }
            self.emit(
                Scalar {
                    value: '\n',
                    byte_start: start,
                    byte_end: end,
                    newline: Some(LineBreakKind::Cr),
                },
                sink,
            )?;
        }
        if scalar.value == '\r' {
            self.pending_cr = Some((scalar.byte_start, scalar.byte_end));
            Ok(())
        } else {
            self.emit(
                Scalar {
                    newline: if scalar.value == '\n' {
                        Some(LineBreakKind::Lf)
                    } else {
                        None
                    },
                    ..scalar
                },
                sink,
            )
        }
    }
    pub(crate) fn feed(
        &mut self,
        bytes: &[u8],
        eof: bool,
        mut sink: impl FnMut(Scalar) -> Result<()>,
    ) -> Result<()> {
        if self.invalid_at.is_some() {
            return Err(failure(
                ProtocolErrorCode::EncodingRequired,
                "decoder stopped at invalid source range",
            ));
        }
        for &byte in bytes {
            if self.pending_len == 0 {
                self.pending_start = self.next_byte;
            }
            self.pending[self.pending_len] = byte;
            self.pending_len += 1;
            self.next_byte = checked_add(self.next_byte, 1)?;
            let width = match self.width() {
                Ok(width) => width,
                Err(error) => {
                    self.invalid_at = Some(self.pending_start);
                    return Err(error);
                }
            };
            if self.pending_len < width {
                continue;
            }
            let value = match self.scalar() {
                Ok(value) => value,
                Err(error) => {
                    self.invalid_at = Some(self.pending_start);
                    return Err(error);
                }
            };
            let scalar = Scalar {
                value,
                byte_start: self.pending_start,
                byte_end: self.next_byte,
                newline: None,
            };
            self.pending_len = 0;
            self.normalized(scalar, &mut sink)?;
        }
        if eof {
            if self.pending_len != 0 {
                self.invalid_at = Some(self.pending_start);
                return Err(failure(
                    ProtocolErrorCode::EncodingRequired,
                    "incomplete encoded tail; original bytes retained",
                ));
            }
            if let Some((start, end)) = self.pending_cr.take() {
                self.emit(
                    Scalar {
                        value: '\n',
                        byte_start: start,
                        byte_end: end,
                        newline: Some(LineBreakKind::Cr),
                    },
                    &mut sink,
                )?;
            }
        }
        Ok(())
    }
}

/// Encode inserted text only. Untouched extents never pass through this encoder.
pub(crate) fn encode_insert(
    text: &str,
    encoding: TextEncoding,
    newline: LineBreakKind,
) -> Result<Vec<u8>> {
    if text.len() > super::contract::MAX_INLINE_TEXT_BYTES {
        return Err(failure(
            ProtocolErrorCode::ResourceLimit,
            "inline insert exceeds queue budget; spool required",
        ));
    }
    let mut bytes = Vec::with_capacity(text.len());
    let mut encode = |scalar: char| match encoding {
        TextEncoding::Utf8 | TextEncoding::Utf8Bom => {
            let mut raw = [0; 4];
            bytes.extend_from_slice(scalar.encode_utf8(&mut raw).as_bytes());
        }
        _ => {
            let mut raw = [0; 2];
            for &unit in scalar.encode_utf16(&mut raw).iter() {
                bytes.extend_from_slice(&if encoding == TextEncoding::Utf16Be {
                    unit.to_be_bytes()
                } else {
                    unit.to_le_bytes()
                });
            }
        }
    };
    for scalar in text.chars() {
        if scalar == '\r' {
            return Err(failure(
                ProtocolErrorCode::InvalidTextBoundary,
                "insert uses normalized LF, not raw CR",
            ));
        }
        if scalar == '\n' {
            match newline {
                LineBreakKind::Lf => encode('\n'),
                LineBreakKind::Cr => encode('\r'),
                LineBreakKind::Crlf => {
                    encode('\r');
                    encode('\n');
                }
            }
        } else {
            encode(scalar);
        }
    }
    Ok(bytes)
}
