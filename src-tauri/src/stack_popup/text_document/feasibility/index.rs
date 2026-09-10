//! Persistent AVL piece tree and history on encrypted disk pages. The only
//! resident roots are current/saved/undo/redo IDs; metadata caches are bounded.
use super::contract::ProtocolErrorCode;
use super::{
    backing::{PageStore, BYTE_PAGE},
    checked_add,
    decode::Checkpoint,
    failure, Result,
};
use serde::{Deserialize, Serialize};

type Root = Option<u64>;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum Store {
    Original,
    Add,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct Piece {
    pub store: Store,
    pub offset: u64,
    pub bytes: u64,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct Metrics {
    pub bytes: u64,
    pub utf16: Option<u64>,
    pub newlines: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
enum Record {
    Leaf {
        piece: Piece,
        metrics: Metrics,
    },
    Branch {
        left: u64,
        right: u64,
        height: u8,
        metrics: Metrics,
    },
    History {
        root: Root,
        previous: Root,
    },
}
impl Record {
    fn metrics(&self) -> Result<Metrics> {
        match self {
            Self::Leaf { metrics, .. } | Self::Branch { metrics, .. } => Ok(*metrics),
            _ => Err(failure(
                ProtocolErrorCode::IoFailure,
                "history page used as piece node",
            )),
        }
    }
    fn height(&self) -> Result<u8> {
        match self {
            Self::Leaf { .. } => Ok(1),
            Self::Branch { height, .. } if *height <= 128 => Ok(*height),
            _ => Err(failure(
                ProtocolErrorCode::IoFailure,
                "piece height invalid",
            )),
        }
    }
}

pub(crate) struct PagedDocument {
    pub nodes: PageStore,
    pub added: PageStore,
    root: Root,
    saved: Root,
    undo: Root,
    redo: Root,
    pub revision: u64,
}
impl PagedDocument {
    pub(crate) fn new(nodes: PageStore, added: PageStore, original_bytes: u64) -> Result<Self> {
        let mut document = Self {
            nodes,
            added,
            root: None,
            saved: None,
            undo: None,
            redo: None,
            revision: 0,
        };
        document.root = document.leaf(Piece {
            store: Store::Original,
            offset: 0,
            bytes: original_bytes,
        })?;
        document.saved = document.root;
        Ok(document)
    }
    fn read(&mut self, id: u64) -> Result<Record> {
        let bytes = self.nodes.read(id)?;
        serde_json::from_slice(&bytes).map_err(|_| {
            failure(
                ProtocolErrorCode::IoFailure,
                "invalid authenticated piece node",
            )
        })
    }
    fn write(&mut self, record: &Record) -> Result<u64> {
        let bytes = serde_json::to_vec(record)
            .map_err(|_| failure(ProtocolErrorCode::IoFailure, "piece serialization failed"))?;
        self.nodes.append(&bytes)
    }
    fn leaf(&mut self, piece: Piece) -> Result<Root> {
        if piece.bytes == 0 {
            return Ok(None);
        }
        checked_add(piece.offset, piece.bytes)?;
        self.write(&Record::Leaf {
            piece,
            metrics: Metrics {
                bytes: piece.bytes,
                utf16: None,
                newlines: None,
            },
        })
        .map(Some)
    }
    fn height(&mut self, root: Root) -> Result<u8> {
        root.map(|id| self.read(id).and_then(|node| node.height()))
            .transpose()
            .map(|height| height.unwrap_or(0))
    }
    fn size(&mut self, root: Root) -> Result<u64> {
        root.map(|id| self.read(id).and_then(|node| node.metrics()))
            .transpose()
            .map(|metrics| metrics.map(|metrics| metrics.bytes).unwrap_or(0))
    }
    pub(crate) fn metrics(&mut self) -> Result<Metrics> {
        match self.root {
            Some(id) => self.read(id)?.metrics(),
            None => Ok(Metrics {
                bytes: 0,
                utf16: Some(0),
                newlines: Some(0),
            }),
        }
    }
    fn branch(&mut self, left: Root, right: Root) -> Result<Root> {
        let (Some(left), Some(right)) = (left, right) else {
            return Ok(left.or(right));
        };
        let l = self.read(left)?;
        let r = self.read(right)?;
        let height =
            l.height()?.max(r.height()?).checked_add(1).ok_or_else(|| {
                failure(ProtocolErrorCode::ResourceLimit, "piece height overflow")
            })?;
        if height > 128 {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "piece tree height bound",
            ));
        }
        // Cross-piece CR/LF and partial scalar state are deliberately unknown until
        // a streaming checkpoint pass validates the concatenation.
        self.write(&Record::Branch {
            left,
            right,
            height,
            metrics: Metrics {
                bytes: checked_add(l.metrics()?.bytes, r.metrics()?.bytes)?,
                utf16: None,
                newlines: None,
            },
        })
        .map(Some)
    }
    fn children(&mut self, root: Root) -> Result<(Root, Root)> {
        match root.map(|id| self.read(id)).transpose()? {
            Some(Record::Branch { left, right, .. }) => Ok((Some(left), Some(right))),
            _ => Err(failure(
                ProtocolErrorCode::IoFailure,
                "expected internal AVL node",
            )),
        }
    }
    fn balance(&mut self, left: Root, right: Root) -> Result<Root> {
        let lh = self.height(left)?;
        let rh = self.height(right)?;
        if lh > rh + 1 {
            let (ll, lr) = self.children(left)?;
            if self.height(ll)? >= self.height(lr)? {
                let joined = self.branch(lr, right)?;
                return self.branch(ll, joined);
            }
            let (lrl, lrr) = self.children(lr)?;
            let a = self.branch(ll, lrl)?;
            let b = self.branch(lrr, right)?;
            return self.branch(a, b);
        }
        if rh > lh + 1 {
            let (rl, rr) = self.children(right)?;
            if self.height(rr)? >= self.height(rl)? {
                let joined = self.branch(left, rl)?;
                return self.branch(joined, rr);
            }
            let (rll, rlr) = self.children(rl)?;
            let a = self.branch(left, rll)?;
            let b = self.branch(rlr, rr)?;
            return self.branch(a, b);
        }
        self.branch(left, right)
    }
    fn join(&mut self, left: Root, right: Root) -> Result<Root> {
        if left.is_none() || right.is_none() {
            return Ok(left.or(right));
        }
        let lh = self.height(left)?;
        let rh = self.height(right)?;
        if lh > rh + 1 {
            let (ll, lr) = self.children(left)?;
            let joined = self.join(lr, right)?;
            self.balance(ll, joined)
        } else if rh > lh + 1 {
            let (rl, rr) = self.children(right)?;
            let joined = self.join(left, rl)?;
            self.balance(joined, rr)
        } else {
            self.branch(left, right)
        }
    }
    fn split(&mut self, root: Root, byte: u64, depth: u8) -> Result<(Root, Root)> {
        if depth >= 128 {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "piece traversal depth bound",
            ));
        }
        let Some(id) = root else {
            return if byte == 0 {
                Ok((None, None))
            } else {
                Err(failure(
                    ProtocolErrorCode::InvalidTextBoundary,
                    "split outside empty document",
                ))
            };
        };
        let node = self.read(id)?;
        let length = node.metrics()?.bytes;
        if byte > length {
            return Err(failure(
                ProtocolErrorCode::InvalidTextBoundary,
                "split outside document",
            ));
        }
        if byte == 0 {
            return Ok((None, root));
        }
        if byte == length {
            return Ok((root, None));
        }
        match node {
            Record::Leaf { piece, .. } => {
                let a = self.leaf(Piece {
                    bytes: byte,
                    ..piece
                })?;
                let b = self.leaf(Piece {
                    offset: checked_add(piece.offset, byte)?,
                    bytes: piece.bytes - byte,
                    ..piece
                })?;
                Ok((a, b))
            }
            Record::Branch { left, right, .. } => {
                let left_size = self.size(Some(left))?;
                if byte < left_size {
                    let (a, b) = self.split(Some(left), byte, depth + 1)?;
                    Ok((a, self.join(b, Some(right))?))
                } else {
                    let (a, b) = self.split(Some(right), byte - left_size, depth + 1)?;
                    Ok((self.join(Some(left), a)?, b))
                }
            }
            _ => Err(failure(
                ProtocolErrorCode::IoFailure,
                "history record in piece traversal",
            )),
        }
    }
    /// Byte boundaries must originate from a validated lease/strict decoder.
    /// Failure leaves the old logical root/history/revision intact, even after disk writes.
    pub(crate) fn replace_bytes(&mut self, start: u64, end: u64, inserted: &[u8]) -> Result<()> {
        let size = self.size(self.root)?;
        if start > end || end > size {
            return Err(failure(
                ProtocolErrorCode::InvalidTextBoundary,
                "edit range outside document",
            ));
        }
        if inserted.len() > super::contract::MAX_INLINE_TEXT_BYTES * 4 {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "encoded inline insert requires spool",
            ));
        }
        checked_add(size - (end - start), inserted.len() as u64)?;
        let next_revision = checked_add(self.revision, 1)?;
        let mut added_root = None;
        for bytes in inserted.chunks(BYTE_PAGE) {
            let page = self.added.append(bytes)?;
            let offset = page.checked_mul(BYTE_PAGE as u64).ok_or_else(|| {
                failure(
                    ProtocolErrorCode::ResourceLimit,
                    "add-store offset overflow",
                )
            })?;
            let leaf = self.leaf(Piece {
                store: Store::Add,
                offset,
                bytes: bytes.len() as u64,
            })?;
            added_root = self.join(added_root, leaf)?;
        }
        let (left, rest) = self.split(self.root, start, 0)?;
        let (_, right) = self.split(rest, end - start, 0)?;
        let joined = self.join(left, added_root)?;
        let next = self.join(joined, right)?;
        let undo = self.write(&Record::History {
            root: self.root,
            previous: self.undo,
        })?;
        self.root = next;
        self.undo = Some(undo);
        self.redo = None;
        self.revision = next_revision;
        Ok(())
    }
    fn history_step(&mut self, undo: bool) -> Result<bool> {
        let Some(id) = (if undo { self.undo } else { self.redo }) else {
            return Ok(false);
        };
        let Record::History { root, previous } = self.read(id)? else {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "history page invalid",
            ));
        };
        let next_revision = checked_add(self.revision, 1)?;
        let opposite = self.write(&Record::History {
            root: self.root,
            previous: if undo { self.redo } else { self.undo },
        })?;
        if undo {
            self.undo = previous;
            self.redo = Some(opposite);
        } else {
            self.redo = previous;
            self.undo = Some(opposite);
        }
        self.root = root;
        self.revision = next_revision;
        Ok(true)
    }
    pub(crate) fn undo(&mut self) -> Result<bool> {
        self.history_step(true)
    }
    pub(crate) fn redo(&mut self) -> Result<bool> {
        self.history_step(false)
    }
    pub(crate) fn dirty(&self) -> bool {
        self.root != self.saved
    }
    pub(crate) fn evict_caches(&mut self) {
        self.nodes.evict_cache();
        self.added.evict_cache();
    }
    pub(crate) fn flush(&self) -> Result<()> {
        self.added.flush()?;
        self.nodes.flush()
    }
    fn locate(&mut self, mut byte: u64) -> Result<(Piece, u64)> {
        let mut node = self
            .root
            .ok_or_else(|| failure(ProtocolErrorCode::InvalidTextBoundary, "empty document"))?;
        for _ in 0..128 {
            match self.read(node)? {
                Record::Leaf { piece, .. } if byte < piece.bytes => return Ok((piece, byte)),
                Record::Branch { left, right, .. } => {
                    let size = self.size(Some(left))?;
                    if byte < size {
                        node = left;
                    } else {
                        byte -= size;
                        node = right;
                    }
                }
                _ => {
                    return Err(failure(
                        ProtocolErrorCode::InvalidTextBoundary,
                        "piece lookup outside document",
                    ))
                }
            }
        }
        Err(failure(
            ProtocolErrorCode::ResourceLimit,
            "piece lookup depth bound",
        ))
    }
    pub(crate) fn read_range(
        &mut self,
        offset: u64,
        output: &mut [u8],
        mut original: impl FnMut(u64, &mut [u8]) -> Result<usize>,
    ) -> Result<usize> {
        if output.len() > super::contract::IO_BLOCK_BYTES {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "document range exceeds I/O block",
            ));
        }
        let size = self.size(self.root)?;
        if offset > size {
            return Err(failure(
                ProtocolErrorCode::InvalidTextBoundary,
                "document range outside extent",
            ));
        }
        let needed = (size - offset).min(output.len() as u64) as usize;
        let mut copied = 0;
        while copied < needed {
            let (piece, within) = self.locate(checked_add(offset, copied as u64)?)?;
            let count = (piece.bytes - within).min((needed - copied) as u64) as usize;
            let at = checked_add(piece.offset, within)?;
            let read = match piece.store {
                Store::Original => original(at, &mut output[copied..copied + count])?,
                Store::Add => {
                    let bytes = self.added.read(at / BYTE_PAGE as u64)?;
                    let begin = (at % BYTE_PAGE as u64) as usize;
                    let end = begin.checked_add(count).ok_or_else(|| {
                        failure(ProtocolErrorCode::ResourceLimit, "add range overflow")
                    })?;
                    let slice = bytes.get(begin..end).ok_or_else(|| {
                        failure(
                            ProtocolErrorCode::IoFailure,
                            "piece exceeds immutable add page",
                        )
                    })?;
                    output[copied..copied + count].copy_from_slice(slice);
                    count
                }
            };
            if read != count {
                return Err(failure(
                    ProtocolErrorCode::SourceChanged,
                    "original extent short read",
                ));
            }
            copied += read;
        }
        Ok(copied)
    }
}

/// Sparse fixed-byte checkpoints are themselves paged; no per-line resident list.
pub(crate) struct CheckpointIndex {
    pub pages: PageStore,
    pub decoder: Checkpoint,
    pub covered: u64,
    pub complete: bool,
}
impl CheckpointIndex {
    pub(crate) fn new(mut pages: PageStore, decoder: Checkpoint) -> Result<Self> {
        let bytes = serde_json::to_vec(&decoder).map_err(|_| {
            failure(
                ProtocolErrorCode::IoFailure,
                "checkpoint serialization failed",
            )
        })?;
        pages.append(&bytes)?;
        Ok(Self {
            covered: decoder.next_byte,
            pages,
            decoder,
            complete: false,
        })
    }
    pub(crate) fn push(&mut self, bytes: &[u8], eof: bool) -> Result<()> {
        if bytes.len() > BYTE_PAGE || self.complete {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "checkpoint block exceeds bound or index complete",
            ));
        }
        self.decoder.feed(bytes, eof, |_| Ok(()))?;
        let checkpoint = serde_json::to_vec(&self.decoder).map_err(|_| {
            failure(
                ProtocolErrorCode::IoFailure,
                "checkpoint serialization failed",
            )
        })?;
        self.pages.append(&checkpoint)?;
        self.covered = self.decoder.next_byte;
        self.complete = eof;
        Ok(())
    }
    pub(crate) fn nearest(&mut self, byte: u64) -> Result<Checkpoint> {
        let mut low = 0;
        let mut high = self.pages.pages();
        while low + 1 < high {
            let middle = low + (high - low) / 2;
            let candidate: Checkpoint = serde_json::from_slice(&self.pages.read(middle)?)
                .map_err(|_| failure(ProtocolErrorCode::IoFailure, "checkpoint page invalid"))?;
            if candidate.next_byte <= byte {
                low = middle;
            } else {
                high = middle;
            }
        }
        serde_json::from_slice(&self.pages.read(low)?)
            .map_err(|_| failure(ProtocolErrorCode::IoFailure, "checkpoint page invalid"))
    }
    pub(crate) fn known_lines(&self) -> Option<u64> {
        self.complete
            .then(|| self.decoder.newlines.checked_add(1))
            .flatten()
    }
}
