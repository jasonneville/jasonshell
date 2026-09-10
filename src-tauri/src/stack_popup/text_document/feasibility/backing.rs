//! Immutable authenticated disk pages. Cache eviction drops plaintext only; it
//! never removes backing reachable from current/history/recovery roots.
use super::contract::ProtocolErrorCode;
use super::{checked_add, failure, io_failure, Result};
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub(crate) const BYTE_PAGE: usize = 64 * 1024;
pub(crate) const NODE_PAGE: usize = 4096;
const FRAME_OVERHEAD: u64 = 32;

#[derive(Debug)]
pub(crate) struct Quota {
    limit: u64,
    used: u64,
}
impl Quota {
    pub(crate) fn new(limit: u64) -> Self {
        Self { limit, used: 0 }
    }
    pub(crate) fn used(&self) -> u64 {
        self.used
    }
    #[cfg(test)]
    pub(super) fn exhaust_for_test(&mut self) {
        self.limit = self.used;
    }
    fn charge(&mut self, bytes: u64) -> Result<()> {
        let next = checked_add(self.used, bytes)?;
        if next > self.limit {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "private disk quota exhausted; dirty roots retained",
            ));
        }
        self.used = next;
        Ok(())
    }
}
pub(crate) type SharedQuota = Arc<Mutex<Quota>>;

/// Bookkeeping reservation only: no preallocation or original-sized allocation.
pub(crate) fn reservation(
    source: u64,
    add_high_water: u64,
    metadata_high_water: u64,
) -> Result<u64> {
    let originals = source.checked_mul(3).ok_or_else(|| {
        failure(
            ProtocolErrorCode::ResourceLimit,
            "source/staging/backup reservation overflow",
        )
    })?;
    let pages = checked_add(source, BYTE_PAGE as u64 - 1)? / BYTE_PAGE as u64;
    let fixed_index = checked_add(pages, 1)?
        .checked_mul(NODE_PAGE as u64 + FRAME_OVERHEAD)
        .ok_or_else(|| {
            failure(
                ProtocolErrorCode::ResourceLimit,
                "checkpoint reservation overflow",
            )
        })?;
    let add_and_staging = add_high_water.checked_mul(2).ok_or_else(|| {
        failure(
            ProtocolErrorCode::ResourceLimit,
            "add/staging reservation overflow",
        )
    })?;
    checked_add(
        checked_add(
            checked_add(
                checked_add(originals, add_and_staging)?,
                metadata_high_water,
            )?,
            fixed_index,
        )?,
        pages.checked_mul(FRAME_OVERHEAD).ok_or_else(|| {
            failure(
                ProtocolErrorCode::ResourceLimit,
                "crypto reservation overflow",
            )
        })?,
    )
}

pub(crate) struct PageStore {
    file: File,
    path: PathBuf,
    key: [u8; 32],
    store_id: [u8; 16],
    page_bytes: usize,
    pages: u64,
    quota: SharedQuota,
    cache: VecDeque<(u64, Arc<Vec<u8>>)>,
    cache_pages: usize,
    cache_peak: usize,
    read_pages: u64,
    writable: bool,
}
impl PageStore {
    #[cfg(windows)]
    pub(crate) fn create(
        path: &Path,
        page_bytes: usize,
        cache_bytes: usize,
        quota: SharedQuota,
    ) -> Result<Self> {
        if !matches!(page_bytes, BYTE_PAGE | NODE_PAGE) || cache_bytes > 32 * 1024 * 1024 {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "page/cache bound invalid",
            ));
        }
        let key = super::native::random::<32>()?;
        let protected = super::native::protect_key(&key, false);
        let protected = protected?;
        let store_id = super::native::random::<16>()?;
        quota
            .lock()
            .map_err(|_| failure(ProtocolErrorCode::IoFailure, "quota lock poisoned"))?
            .charge(protected.len() as u64 + 16)?;
        let mut key_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.with_extension("key"))
            .map_err(io_failure)?;
        key_file.write_all(&store_id).map_err(io_failure)?;
        key_file.write_all(&protected).map_err(io_failure)?;
        key_file.sync_all().map_err(io_failure)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(io_failure)?;
        Ok(Self {
            file,
            path: path.to_owned(),
            key,
            store_id,
            page_bytes,
            pages: 0,
            quota,
            cache: VecDeque::new(),
            cache_pages: cache_bytes / page_bytes,
            cache_peak: 0,
            read_pages: 0,
            writable: true,
        })
    }
    #[cfg(windows)]
    pub(crate) fn reopen(
        path: &Path,
        page_bytes: usize,
        cache_bytes: usize,
        quota: SharedQuota,
    ) -> Result<Self> {
        if !matches!(page_bytes, BYTE_PAGE | NODE_PAGE) || cache_bytes > 32 * 1024 * 1024 {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "page/cache bound invalid",
            ));
        }
        let mut protected = Vec::new();
        File::open(path.with_extension("key"))
            .map_err(io_failure)?
            .take(4097)
            .read_to_end(&mut protected)
            .map_err(io_failure)?;
        if protected.len() < 17 || protected.len() > 4096 {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "protected store header invalid",
            ));
        }
        let store_id = protected[..16]
            .try_into()
            .map_err(|_| failure(ProtocolErrorCode::IoFailure, "store ID invalid"))?;
        let key = super::native::protect_key(&protected[16..], true)?;
        if key.len() != 32 {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "protected page key invalid",
            ));
        }
        let key: [u8; 32] = key
            .try_into()
            .map_err(|_| failure(ProtocolErrorCode::IoFailure, "protected page key invalid"))?;
        let file = OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(io_failure)?;
        let bytes = file.metadata().map_err(io_failure)?.len();
        let frame = page_bytes as u64 + FRAME_OVERHEAD;
        if bytes % frame != 0 {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "incomplete immutable page; recovery classification required",
            ));
        }
        Ok(Self {
            file,
            path: path.to_owned(),
            key,
            store_id,
            page_bytes,
            pages: bytes / frame,
            quota,
            cache: VecDeque::new(),
            cache_pages: cache_bytes / page_bytes,
            cache_peak: 0,
            read_pages: 0,
            writable: false,
        })
    }
    fn aad(&self, page: u64, length: u32) -> [u8; 28] {
        let mut aad = [0; 28];
        aad[..16].copy_from_slice(&self.store_id);
        aad[16..24].copy_from_slice(&page.to_le_bytes());
        aad[24..].copy_from_slice(&length.to_le_bytes());
        aad
    }

    fn cipher(&self) -> Result<Aes256Gcm> {
        Aes256Gcm::new_from_slice(&self.key)
            .map_err(|_| failure(ProtocolErrorCode::IoFailure, "AES-256-GCM key invalid"))
    }

    fn nonce(page: u64) -> [u8; 12] {
        let mut nonce = [0; 12];
        nonce[4..].copy_from_slice(&page.to_be_bytes());
        nonce
    }
    pub(crate) fn append(&mut self, bytes: &[u8]) -> Result<u64> {
        if !self.writable {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "store sealed or failed; append requires a new store/key",
            ));
        }
        if bytes.len() > self.page_bytes {
            return Err(failure(
                ProtocolErrorCode::ResourceLimit,
                "immutable page exceeds bound",
            ));
        }
        let page = self.pages;
        let next = checked_add(page, 1)?;
        let position = page
            .checked_mul(self.page_bytes as u64 + FRAME_OVERHEAD)
            .ok_or_else(|| failure(ProtocolErrorCode::ResourceLimit, "page position overflow"))?;
        // Each store has its own random key; monotonic page ID supplies unique nonce.
        // Reopened stores are sealed: a failed write may have consumed a nonce
        // beyond physical EOF. Never guess that nonce was unused on restart.
        let nonce_bytes = Self::nonce(page);
        let mut plain = vec![0; self.page_bytes];
        plain[..bytes.len()].copy_from_slice(bytes);
        let aad = self.aad(page, bytes.len() as u32);
        let encrypted = self
            .cipher()?
            .encrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: &plain,
                    aad: &aad,
                },
            )
            .map_err(|_| {
                failure(
                    ProtocolErrorCode::IoFailure,
                    "AES-256-GCM encryption failed",
                )
            })?;
        plain.fill(0);
        if encrypted.len() != self.page_bytes + 16 {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "AES-256-GCM ciphertext length invalid",
            ));
        }
        self.quota
            .lock()
            .map_err(|_| failure(ProtocolErrorCode::IoFailure, "quota lock poisoned"))?
            .charge(self.page_bytes as u64 + FRAME_OVERHEAD)?;
        self.pages = next; // nonce consumed even if write fails; never retry it.
        self.writable = false;
        self.file
            .seek(SeekFrom::Start(position))
            .map_err(io_failure)?;
        self.file
            .write_all(&(bytes.len() as u32).to_le_bytes())
            .map_err(io_failure)?;
        self.file.write_all(&nonce_bytes).map_err(io_failure)?;
        self.file.write_all(&encrypted).map_err(io_failure)?;
        self.writable = true;
        self.cache_page(page, bytes.to_vec());
        Ok(page)
    }
    fn cache_page(&mut self, page: u64, bytes: Vec<u8>) -> Arc<Vec<u8>> {
        let value = Arc::new(bytes);
        if self.cache_pages > 0 {
            while self.cache.len() >= self.cache_pages {
                self.cache.pop_front();
            }
            self.cache.push_back((page, value.clone()));
            self.cache_peak = self.cache_peak.max(self.cache.len() * self.page_bytes);
        }
        value
    }
    pub(crate) fn read(&mut self, page: u64) -> Result<Arc<Vec<u8>>> {
        if page >= self.pages {
            return Err(failure(
                ProtocolErrorCode::InvalidTextBoundary,
                "immutable page outside store",
            ));
        }
        if let Some(index) = self.cache.iter().position(|entry| entry.0 == page) {
            let entry = self
                .cache
                .remove(index)
                .ok_or_else(|| failure(ProtocolErrorCode::IoFailure, "cache position invalid"))?;
            let value = entry.1.clone();
            self.cache.push_back(entry);
            return Ok(value);
        }
        let position = page
            .checked_mul(self.page_bytes as u64 + FRAME_OVERHEAD)
            .ok_or_else(|| failure(ProtocolErrorCode::ResourceLimit, "page position overflow"))?;
        self.file
            .seek(SeekFrom::Start(position))
            .map_err(io_failure)?;
        let mut header = [0; 16];
        self.file.read_exact(&mut header).map_err(io_failure)?;
        let length = u32::from_le_bytes(
            header[..4]
                .try_into()
                .map_err(|_| failure(ProtocolErrorCode::IoFailure, "page length invalid"))?,
        ) as usize;
        if length > self.page_bytes {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "authenticated page length invalid",
            ));
        }
        let mut encrypted = vec![0; self.page_bytes + 16];
        self.file.read_exact(&mut encrypted).map_err(io_failure)?;
        let nonce_bytes: [u8; 12] = header[4..]
            .try_into()
            .map_err(|_| failure(ProtocolErrorCode::IoFailure, "page nonce invalid"))?;
        if nonce_bytes != Self::nonce(page) {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "immutable page nonce does not match page identity",
            ));
        }
        let aad = self.aad(page, length as u32);
        let mut plain = self
            .cipher()?
            .decrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: &encrypted,
                    aad: &aad,
                },
            )
            .map_err(|_| {
                failure(
                    ProtocolErrorCode::IoFailure,
                    "immutable page authentication failed",
                )
            })?;
        if plain.len() != self.page_bytes {
            return Err(failure(
                ProtocolErrorCode::IoFailure,
                "immutable page plaintext length invalid",
            ));
        }
        plain.truncate(length);
        self.read_pages = checked_add(self.read_pages, 1)?;
        Ok(self.cache_page(page, plain))
    }
    pub(crate) fn flush(&self) -> Result<()> {
        self.file.sync_all().map_err(io_failure)
    }
    pub(crate) fn evict_cache(&mut self) {
        self.cache.clear();
    }
    pub(crate) fn pages(&self) -> u64 {
        self.pages
    }
    pub(crate) fn cache_peak(&self) -> usize {
        self.cache_peak
    }
    pub(crate) fn cache_resident(&self) -> usize {
        self.cache.len() * self.page_bytes
    }
    pub(crate) fn disk_bytes(&self) -> u64 {
        self.pages * (self.page_bytes as u64 + FRAME_OVERHEAD)
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for PageStore {
    fn drop(&mut self) {
        self.key.fill(0);
    }
}

/// Byte addressing of original pages is arithmetic, not an original-sized array.
pub(crate) fn read_original(
    store: &mut PageStore,
    offset: u64,
    output: &mut [u8],
) -> Result<usize> {
    let mut copied = 0;
    while copied < output.len() {
        let absolute = checked_add(offset, copied as u64)?;
        let page = absolute / BYTE_PAGE as u64;
        if page >= store.pages() {
            break;
        }
        let bytes = store.read(page)?;
        let start = (absolute % BYTE_PAGE as u64) as usize;
        if start >= bytes.len() {
            break;
        }
        let count = (bytes.len() - start).min(output.len() - copied);
        output[copied..copied + count].copy_from_slice(&bytes[start..start + count]);
        copied += count;
    }
    Ok(copied)
}
