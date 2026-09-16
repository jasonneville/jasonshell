//! P04 experiment-only encrypted recovery schema. Payload and manifest pages use
//! the existing per-store DPAPI key envelope and AES-256-GCM authenticated pages.

use super::contract::ProtocolErrorCode;
use super::{
    backing::{PageStore, SharedQuota, BYTE_PAGE},
    failure, Result,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

const VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct Edit {
    start: u64,
    end: u64,
    bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    base_page: u64,
    base_revision: u64,
    edit_pages: Vec<u64>,
    committed: bool,
}

pub(crate) struct RecoveryRecord {
    pages: PageStore,
    manifest: Manifest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScanClassification {
    Recoverable,
    Quarantined,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ScanResult {
    pub(crate) classification: ScanClassification,
    pub(crate) diagnostic: &'static str,
}

pub(crate) fn scan(path: &Path, quota: SharedQuota) -> ScanResult {
    match RecoveryRecord::open(path, quota).and_then(RecoveryRecord::reconstruct) {
        Ok(_) => ScanResult {
            classification: ScanClassification::Recoverable,
            diagnostic: "recovery record authenticated",
        },
        Err(_) => ScanResult {
            classification: ScanClassification::Quarantined,
            diagnostic: "recovery record quarantined; assets retained",
        },
    }
}

impl RecoveryRecord {
    pub(crate) fn create(
        path: &Path,
        quota: SharedQuota,
        base: &[u8],
        base_revision: u64,
    ) -> Result<Self> {
        if base.len() > BYTE_PAGE {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "recovery base page exceeds bound",
            ));
        }
        let mut pages = PageStore::create(path, BYTE_PAGE, 0, quota)?;
        let base_page = pages.append(base)?;
        pages.flush()?;
        Ok(Self {
            pages,
            manifest: Manifest {
                version: VERSION,
                base_page,
                base_revision,
                edit_pages: Vec::new(),
                committed: false,
            },
        })
    }

    pub(crate) fn append_edit(&mut self, start: u64, end: u64, bytes: &[u8]) -> Result<()> {
        if start > end {
            return Err(failure(
                ProtocolErrorCode::InvalidTextBoundary,
                "recovery edit range invalid",
            ));
        }
        let encoded = serde_json::to_vec(&Edit {
            start,
            end,
            bytes: bytes.to_vec(),
        })
        .map_err(|_| {
            failure(
                ProtocolErrorCode::IoFailure,
                "recovery edit encoding failed",
            )
        })?;
        let page = self.pages.append(&encoded)?;
        self.pages.flush()?;
        self.manifest.edit_pages.push(page);
        Ok(())
    }

    pub(crate) fn commit(&mut self) -> Result<()> {
        self.manifest.committed = true;
        let encoded = serde_json::to_vec(&self.manifest).map_err(|_| {
            failure(
                ProtocolErrorCode::IoFailure,
                "recovery manifest encoding failed",
            )
        })?;
        self.pages.append(&encoded)?;
        self.pages.flush()
    }

    pub(crate) fn open(path: &Path, quota: SharedQuota) -> Result<Self> {
        let mut pages = PageStore::reopen(path, BYTE_PAGE, 0, quota)?;
        if pages.pages() < 2 {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "recovery record incomplete",
            ));
        }
        let manifest_bytes = pages.read(pages.pages() - 1)?;
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|_| failure(ProtocolErrorCode::IoFailure, "recovery manifest invalid"))?;
        if manifest.version != VERSION
            || !manifest.committed
            || manifest.base_page != 0
            || manifest
                .edit_pages
                .iter()
                .enumerate()
                .any(|(i, page)| *page != i as u64 + 1)
            || manifest.edit_pages.len() as u64 + 2 != pages.pages()
        {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "recovery manifest inconsistent",
            ));
        }
        Ok(Self { pages, manifest })
    }

    pub(crate) fn reconstruct(mut self) -> Result<Vec<u8>> {
        let mut result = self.pages.read(self.manifest.base_page)?.as_ref().clone();
        for page in &self.manifest.edit_pages {
            let raw = self.pages.read(*page)?;
            let edit: Edit = serde_json::from_slice(&raw)
                .map_err(|_| failure(ProtocolErrorCode::IoFailure, "recovery edit invalid"))?;
            let start = usize::try_from(edit.start).map_err(|_| {
                failure(
                    ProtocolErrorCode::ResourceLimit,
                    "recovery edit offset overflow",
                )
            })?;
            let end = usize::try_from(edit.end).map_err(|_| {
                failure(
                    ProtocolErrorCode::ResourceLimit,
                    "recovery edit offset overflow",
                )
            })?;
            if start > end || end > result.len() {
                return Err(failure(
                    ProtocolErrorCode::InvalidTextBoundary,
                    "recovery edit outside base",
                ));
            }
            result.splice(start..end, edit.bytes);
        }
        Ok(result)
    }
}
