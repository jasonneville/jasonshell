//! STA/OLE image publication. Rejection precedes commitment; flush establishes durability.
use super::geometry::{checked_rgba_len, CroppedImage, MAX_STAGING_BYTES};
use std::{mem::ManuallyDrop, sync::atomic::{AtomicUsize, Ordering}, time::{Duration, Instant}};
use windows::{core::{implement, Error, HRESULT, w}, Win32::{Foundation::*, System::{Com::*, DataExchange::RegisterClipboardFormatW, Memory::*, Ole::*}, UI::{Shell::SHCreateStdEnumFmtEtc, WindowsAndMessaging::*}}};
use windows_core::{BOOL, Ref};

static OBJECTS: AtomicUsize = AtomicUsize::new(0);
static GLOBALS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static STAGED: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure { InvalidImage, Budget, Allocation, Apartment, Busy, SetRejected, SetUnknown, NotDurable }
impl Failure {
    pub fn code(self) -> &'static str { match self {
        Self::InvalidImage => "clipboard-invalid-image", Self::Budget => "clipboard-budget",
        Self::Allocation => "clipboard-allocation", Self::Apartment => "clipboard-sta-unavailable",
        Self::Busy => "clipboard-busy", Self::SetRejected => "clipboard-set-rejected",
        Self::SetUnknown => "clipboard-publication-unknown",
        Self::NotDurable => "clipboard-committed-not-durable",
    } }
}
/// Explicit dependency failpoints, reachable only through the opt-in standalone driver.
#[derive(Clone, Copy, Default)]
pub struct Failpoints { pub allocation: bool, pub set: bool, pub flush: bool, pub stall_before_set: bool, pub stall_after_commit: bool, pub stall_shutdown: bool }
#[derive(Clone, Copy)]
pub enum PublicationStage { Publishing, Committed }

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedCounts { pub live_objects: usize, pub live_global_allocations: usize, pub live_staged_bytes: usize }
pub fn owned_counts() -> OwnedCounts { OwnedCounts { live_objects: OBJECTS.load(Ordering::SeqCst),
    live_global_allocations: GLOBALS.load(Ordering::SeqCst), live_staged_bytes: STAGED.load(Ordering::SeqCst) } }

struct Staged(usize);
impl Staged {
    fn new(bytes: usize) -> Self { STAGED.fetch_add(bytes, Ordering::SeqCst); Self(bytes) }
}
impl Drop for Staged { fn drop(&mut self) { STAGED.fetch_sub(self.0, Ordering::SeqCst); } }

struct Charge(usize);
impl Charge {
    fn new(bytes: usize) -> Result<Self, Failure> {
        BYTES.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| old.checked_add(bytes).filter(|total| *total <= MAX_STAGING_BYTES))
            .map_err(|_| Failure::Budget)?;
        Ok(Self(bytes))
    }
}
impl Drop for Charge { fn drop(&mut self) { BYTES.fetch_sub(self.0, Ordering::SeqCst); } }

struct Global { handle: HGLOBAL, logical_len: usize, _charge: Charge, _staged: Staged }
impl Global {
    fn copy(bytes: &[u8], fail: bool) -> Result<Self, Failure> {
        if fail { return Err(Failure::Allocation); }
        let mut charge = Charge::new(bytes.len())?;
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes.len()) }.map_err(|_| Failure::Allocation)?;
        GLOBALS.fetch_add(1, Ordering::SeqCst);
        let actual = unsafe { GlobalSize(handle) };
        if actual < bytes.len() {
            unsafe { let _ = GlobalFree(handle); }
            GLOBALS.fetch_sub(1, Ordering::SeqCst);
            return Err(Failure::Allocation);
        }
        if actual > bytes.len() {
            match Charge::new(actual - bytes.len()) {
                Ok(extra) => { charge.0 += extra.0; std::mem::forget(extra); }
                Err(error) => { unsafe { let _ = GlobalFree(handle); } GLOBALS.fetch_sub(1, Ordering::SeqCst); return Err(error); }
            }
        }
        let value = Self { handle, logical_len: bytes.len(), _charge: charge, _staged: Staged::new(actual) };
        let pointer = unsafe { GlobalLock(handle) };
        if pointer.is_null() { return Err(Failure::Allocation); }
        unsafe {
            // Initialize rounding/padding too; consumers may inspect GlobalSize bytes.
            std::ptr::write_bytes(pointer.cast::<u8>(), 0, actual);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len());
            let _ = GlobalUnlock(handle);
        }
        Ok(value)
    }
    fn transfer(self) -> HGLOBAL {
        let value = ManuallyDrop::new(self);
        let handle = value.handle;
        // COM owns the memory after transfer. Accounting tracks local ownership only.
        GLOBALS.fetch_sub(1, Ordering::SeqCst);
        unsafe { drop(std::ptr::read(&value._charge)); }
        unsafe { drop(std::ptr::read(&value._staged)); }
        handle
    }
}
#[link(name = "kernel32")]
unsafe extern "system" { fn GlobalFree(memory: HGLOBAL) -> HGLOBAL; }
#[link(name = "ole32")]
unsafe extern "system" {
    #[link_name = "OleIsCurrentClipboard"]
    fn is_current_clipboard(data: *mut std::ffi::c_void) -> HRESULT;
}
impl Drop for Global { fn drop(&mut self) { unsafe { let _ = GlobalFree(self.handle); } GLOBALS.fetch_sub(1, Ordering::SeqCst); } }

fn crc(bytes: &[u8]) -> u32 {
    let mut value = u32::MAX;
    for byte in bytes {
        value ^= u32::from(*byte);
        for _ in 0..8 { value = (value >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(value & 1)); }
    }
    !value
}

fn png_lengths(image: &CroppedImage) -> Result<(usize, usize, usize), Failure> {
    let raw = image.pixels.len().checked_add(image.height as usize).ok_or(Failure::Budget)?;
    let blocks = raw.checked_add(65_534).ok_or(Failure::Budget)? / 65_535;
    let zlib = raw.checked_add(blocks.checked_mul(5).ok_or(Failure::Budget)?).and_then(|n| n.checked_add(6)).ok_or(Failure::Budget)?;
    let total = zlib.checked_add(57).ok_or(Failure::Budget)?;
    if zlib > u32::MAX as usize { return Err(Failure::Budget); }
    Ok((raw, zlib, total))
}

/// PNG RGBA8, filter 0, zlib stored blocks. Exactly one preallocated output Vec;
/// fixed scalar/13-byte scratch, no compression-library hidden heap allocation.
fn encode_png(image: &CroppedImage, output: &mut Vec<u8>) -> Result<(), Failure> {
    encode_rgba(image.width, image.height, &image.pixels, output)
}
fn encode_rgba(width: u32, height: u32, pixels: &[u8], output: &mut Vec<u8>) -> Result<(), Failure> {
    let raw_len = pixels.len().checked_add(height as usize).ok_or(Failure::Budget)?;
    let blocks = raw_len.checked_add(65_534).ok_or(Failure::Budget)? / 65_535;
    let zlib_len = raw_len.checked_add(blocks.checked_mul(5).ok_or(Failure::Budget)?).and_then(|n| n.checked_add(6)).ok_or(Failure::Budget)?;
    let total = zlib_len.checked_add(57).ok_or(Failure::Budget)?;
    if output.capacity() < total { return Err(Failure::Budget); }
    output.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    output.extend_from_slice(&13u32.to_be_bytes());
    let start = output.len();
    output.extend_from_slice(b"IHDR");
    let mut header = [0u8; 13];
    header[..4].copy_from_slice(&width.to_be_bytes());
    header[4..8].copy_from_slice(&height.to_be_bytes());
    header[8] = 8; header[9] = 6;
    output.extend_from_slice(&header);
    output.extend_from_slice(&crc(&output[start..]).to_be_bytes());
    output.extend_from_slice(&(zlib_len as u32).to_be_bytes());
    let start = output.len();
    output.extend_from_slice(b"IDAT");
    output.extend_from_slice(&[0x78, 0x01]);
    let stride = width as usize * 4;
    let mut bytes = pixels.chunks_exact(stride).flat_map(|row| std::iter::once(0u8).chain(row.iter().copied()));
    let mut remaining = raw_len;
    let (mut a, mut b) = (1u32, 0u32);
    while remaining != 0 {
        let count = remaining.min(65_535) as u16;
        remaining -= usize::from(count);
        output.push(if remaining == 0 { 1 } else { 0 });
        output.extend_from_slice(&count.to_le_bytes());
        output.extend_from_slice(&(!count).to_le_bytes());
        for _ in 0..count {
            let byte = bytes.next().ok_or(Failure::InvalidImage)?;
            output.push(byte); a = (a + u32::from(byte)) % 65_521; b = (b + a) % 65_521;
        }
    }
    output.extend_from_slice(&((b << 16) | a).to_be_bytes());
    output.extend_from_slice(&crc(&output[start..]).to_be_bytes());
    output.extend_from_slice(&0u32.to_be_bytes());
    output.extend_from_slice(b"IEND");
    output.extend_from_slice(&crc(b"IEND").to_be_bytes());
    if output.len() != total { return Err(Failure::InvalidImage); }
    Ok(())
}

/// Shared exact-size PNG encoder, borrowing native pixels without another raw copy.
pub fn png_rgba(width: u32, height: u32, pixels: &[u8], retained: usize) -> Result<Vec<u8>, Failure> {
    let len = checked_rgba_len(width, height, MAX_STAGING_BYTES).map_err(|_| Failure::InvalidImage)?;
    if pixels.len() != len || retained < len || width > 16_384 || height > 16_384
        || u64::from(width) * u64::from(height) > 33_600_000 { return Err(Failure::InvalidImage); }
    let raw = len.checked_add(height as usize).ok_or(Failure::Budget)?;
    let blocks = raw.checked_add(65_534).ok_or(Failure::Budget)? / 65_535;
    let total = raw.checked_add(blocks.checked_mul(5).ok_or(Failure::Budget)?).and_then(|n| n.checked_add(63)).ok_or(Failure::Budget)?;
    if retained.checked_add(total).is_none_or(|n| n > MAX_STAGING_BYTES) { return Err(Failure::Budget); }
    let mut output = Vec::new();
    output.try_reserve_exact(total).map_err(|_| Failure::Allocation)?;
    if retained.checked_add(output.capacity()).is_none_or(|n| n > MAX_STAGING_BYTES) { return Err(Failure::Budget); }
    encode_rgba(width, height, pixels, &mut output)?;
    Ok(output)
}

struct Prepared { dib: Global, png: Global, png_format: u16, _charge: Charge }
fn prepare(image: &CroppedImage, retained: usize, fail: Failpoints) -> Result<Prepared, Failure> {
    let len = checked_rgba_len(image.width, image.height, MAX_STAGING_BYTES).map_err(|_| Failure::InvalidImage)?;
    if image.pixels.len() != len || image.width > 16_384 || image.height > 16_384
        || image.pixels.chunks_exact(4).any(|pixel| pixel[3] != 255) { return Err(Failure::InvalidImage); }
    if retained < len { return Err(Failure::Budget); }
    let dib_len = len.checked_add(40).ok_or(Failure::Budget)?;
    let (_, _, png_limit) = png_lengths(image)?;
    // Covers retained input and temporary Vec capacities. Globals and each GetData
    // construction acquire separate actual-size charges. COM-owned returned copies
    // leave this ownership/budget domain; consumers can retain arbitrary copies.
    let reserve = retained.checked_add(dib_len).and_then(|v| v.checked_add(png_limit)).ok_or(Failure::Budget)?;
    let charge = Charge::new(reserve)?;
    if fail.allocation { let _ = Global::copy(&[], true)?; }
    let mut dib = Vec::new();
    dib.try_reserve_exact(dib_len).map_err(|_| Failure::Allocation)?;
    let _dib_staged = Staged::new(dib.capacity());
    let _dib_rounding = Charge::new(dib.capacity() - dib_len)?;
    dib.resize(dib_len, 0);
    dib[0..4].copy_from_slice(&40u32.to_le_bytes());
    dib[4..8].copy_from_slice(&(image.width as i32).to_le_bytes());
    dib[8..12].copy_from_slice(&(image.height as i32).to_le_bytes());
    dib[12..14].copy_from_slice(&1u16.to_le_bytes());
    dib[14..16].copy_from_slice(&32u16.to_le_bytes());
    dib[20..24].copy_from_slice(&(len as u32).to_le_bytes());
    let stride = image.width as usize * 4;
    for (dst, src) in dib[40..].chunks_exact_mut(stride).zip(image.pixels.chunks_exact(stride).rev()) {
        for (out, rgba) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) { out.copy_from_slice(&[rgba[2], rgba[1], rgba[0], 255]); }
    }
    let mut output = Vec::new();
    output.try_reserve_exact(png_limit).map_err(|_| Failure::Allocation)?;
    let _png_staged = Staged::new(output.capacity());
    let _png_rounding = Charge::new(output.capacity() - png_limit)?;
    encode_png(image, &mut output)?;
    let format = unsafe { RegisterClipboardFormatW(w!("PNG")) };
    if format == 0 || format > u16::MAX as u32 { return Err(Failure::SetRejected); }
    let dib = Global::copy(&dib, false)?;
    let png = Global::copy(&output, false)?;
    Ok(Prepared { dib, png, png_format: format as u16, _charge: charge })
}

fn format(id: u16) -> FORMATETC { FORMATETC { cfFormat: id, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0 as u32,
    lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 } }

#[implement(IDataObject)]
struct ImageData { prepared: Prepared }
impl Drop for ImageData { fn drop(&mut self) { OBJECTS.fetch_sub(1, Ordering::SeqCst); } }
impl ImageData {
    fn supported(&self, input: *const FORMATETC) -> windows::core::Result<u16> {
        let value = unsafe { input.as_ref() }.ok_or_else(|| Error::from_hresult(DV_E_FORMATETC))?;
        if (value.cfFormat != 8 && value.cfFormat != self.prepared.png_format) || value.dwAspect != DVASPECT_CONTENT.0 as u32
            || value.lindex != -1 || value.tymed & TYMED_HGLOBAL.0 as u32 == 0 { return Err(Error::from_hresult(DV_E_FORMATETC)); }
        Ok(value.cfFormat)
    }
}
impl IDataObject_Impl for ImageData_Impl {
    fn GetData(&self, input: *const FORMATETC) -> windows::core::Result<STGMEDIUM> {
        let id = self.supported(input)?;
        let source = if id == 8 { &self.prepared.dib } else { &self.prepared.png };
        let pointer = unsafe { GlobalLock(source.handle) };
        if pointer.is_null() { return Err(Error::from_hresult(E_OUTOFMEMORY)); }
        let len = source.logical_len;
        let result = Global::copy(unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), len) }, false);
        unsafe { let _ = GlobalUnlock(source.handle); }
        let memory = result.map_err(|_| Error::from_hresult(E_OUTOFMEMORY))?;
        Ok(STGMEDIUM { tymed: TYMED_HGLOBAL.0 as u32, u: STGMEDIUM_0 { hGlobal: memory.transfer() }, pUnkForRelease: ManuallyDrop::new(None) })
    }
    fn GetDataHere(&self, _: *const FORMATETC, _: *mut STGMEDIUM) -> windows::core::Result<()> { Err(Error::from_hresult(E_NOTIMPL)) }
    fn QueryGetData(&self, input: *const FORMATETC) -> HRESULT { self.supported(input).map_or_else(|e| e.code(), |_| HRESULT(0)) }
    fn GetCanonicalFormatEtc(&self, _: *const FORMATETC, output: *mut FORMATETC) -> HRESULT {
        if let Some(output) = unsafe { output.as_mut() } { output.ptd = std::ptr::null_mut(); } DATA_S_SAMEFORMATETC
    }
    fn SetData(&self, _: *const FORMATETC, _: *const STGMEDIUM, _: BOOL) -> windows::core::Result<()> { Err(Error::from_hresult(E_NOTIMPL)) }
    fn EnumFormatEtc(&self, direction: u32) -> windows::core::Result<IEnumFORMATETC> {
        if direction != DATADIR_GET.0 as u32 { return Err(Error::from_hresult(E_NOTIMPL)); }
        unsafe { SHCreateStdEnumFmtEtc(&[format(8), format(self.prepared.png_format)]) }
    }
    fn DAdvise(&self, _: *const FORMATETC, _: u32, _: Ref<'_, IAdviseSink>) -> windows::core::Result<u32> { Err(Error::from_hresult(OLE_E_ADVISENOTSUPPORTED)) }
    fn DUnadvise(&self, _: u32) -> windows::core::Result<()> { Err(Error::from_hresult(OLE_E_ADVISENOTSUPPORTED)) }
    fn EnumDAdvise(&self) -> windows::core::Result<IEnumSTATDATA> { Err(Error::from_hresult(OLE_E_ADVISENOTSUPPORTED)) }
}

/// Creator-thread-owned STA. No spawned worker or timed-out abandoned publication.
/// Keep one owner alive for the coordinated STA lifetime, not per UI interaction.
pub(super) struct Publisher { owner: Option<IDataObject>, durable: bool, shutdown_outcome: Option<ShutdownOutcome>, _thread: std::marker::PhantomData<std::rc::Rc<()>> }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownOutcome { Durable, Superseded, DurabilityLost }
impl Publisher {
    pub fn new() -> Result<Self, Failure> {
        unsafe { OleInitialize(None) }.map_err(|_| Failure::Apartment)?;
        Ok(Self { owner: None, durable: true, shutdown_outcome: None, _thread: std::marker::PhantomData })
    }
    pub fn publish(&mut self, image: &CroppedImage, retained: usize, fail: Failpoints) -> Result<bool, Failure> {
        self.publish_observed(image, retained, fail, |_| {})
    }
    /// Helper-process-only instrumentation: metadata acknowledgement before and
    /// after the native commit boundary; never conveys image bytes to stdout.
    pub fn publish_observed(&mut self, image: &CroppedImage, retained: usize, fail: Failpoints, mut observe: impl FnMut(PublicationStage)) -> Result<bool, Failure> {
        if self.shutdown_outcome.is_some() { return Err(Failure::Apartment); }
        // Never replace a still-undurable committed owner. Coordinator authorizes before this synchronous boundary.
        if !self.durable { return Err(Failure::NotDurable); }
        let prepared = prepare(image, retained, fail)?;
        OBJECTS.fetch_add(1, Ordering::SeqCst);
        let object: IDataObject = ImageData { prepared }.into();
        let mut result = Err(Failure::SetRejected);
        let deadline = Instant::now() + Duration::from_secs(2);
        for attempt in 0..3 {
            if Instant::now() >= deadline { break; }
            observe(PublicationStage::Publishing);
            if fail.stall_before_set { loop { std::thread::park(); } }
            let set = if fail.set { Err(Error::from_hresult(E_FAIL)) } else { unsafe { OleSetClipboard(&object) } };
            match set {
                Ok(()) => { result = Ok(()); break; }
                Err(error) if error.code() == CLIPBRD_E_CANT_OPEN => { result = Err(Failure::Busy); }
                Err(_) => { result = Err(if fail.set { Failure::SetRejected } else { Failure::SetUnknown }); break; }
            }
            if attempt < 2 { pump(); std::thread::sleep(Duration::from_millis(10)); }
        }
        result?;
        self.owner = Some(object);
        self.durable = false;
        observe(PublicationStage::Committed);
        if fail.stall_after_commit { loop { std::thread::park(); } }
        if !fail.flush { let _ = self.flush(); }
        Ok(self.durable) // false is committed warning, never rejection.
    }
    pub fn flush(&mut self) -> Result<(), Failure> {
        if let Some(outcome) = self.shutdown_outcome { return if outcome == ShutdownOutcome::Durable { Ok(()) } else { Err(Failure::NotDurable) }; }
        if self.durable { return Ok(()); }
        let deadline = Instant::now() + Duration::from_secs(2);
        for attempt in 0..3 {
            if Instant::now() >= deadline { break; }
            if unsafe { OleFlushClipboard() }.is_ok() {
                self.durable = true; self.owner.take(); pump(); return Ok(());
            }
            if attempt < 2 { pump(); std::thread::sleep(Duration::from_millis(10)); }
        }
        Err(Failure::NotDurable)
    }
    /// Explicit app shutdown policy: one final bounded retry batch, then release
    /// the apartment's owner with a truthful durability-loss outcome. Never clears
    /// or restores arbitrary clipboard content. Already materialized data may remain;
    /// delayed formats may become unavailable. Call before apartment teardown and
    /// present DurabilityLost as a postcommit warning, never a rejected Copy.
    /// Retry/deadline limits cannot interrupt an in-flight synchronous OS/OLE call.
    pub fn shutdown(&mut self) -> ShutdownOutcome {
        if let Some(outcome) = self.shutdown_outcome { return outcome; }
        if self.durable { self.shutdown_outcome = Some(ShutdownOutcome::Durable); return ShutdownOutcome::Durable; }
        let current = self.owner.as_ref().map(|owner| unsafe { is_current_clipboard(windows_core::Interface::as_raw(owner)) }.0);
        if current == Some(1) { self.owner.take(); self.shutdown_outcome = Some(ShutdownOutcome::Superseded); return ShutdownOutcome::Superseded; }
        if self.flush().is_ok() { self.shutdown_outcome = Some(ShutdownOutcome::Durable); return ShutdownOutcome::Durable; }
        self.owner.take();
        self.shutdown_outcome = Some(ShutdownOutcome::DurabilityLost);
        ShutdownOutcome::DurabilityLost
    }
}
impl Drop for Publisher {
    fn drop(&mut self) {
        if self.shutdown() == ShutdownOutcome::DurabilityLost {
            eprintln!("clipboard-shutdown: committed data durability lost; delayed formats may be unavailable");
        }
        self.owner.take();
        unsafe { OleUninitialize(); }
    }
}
pub fn pump() {
    let mut message = MSG::default();
    while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
        unsafe { let _ = TranslateMessage(&message); DispatchMessageW(&message); }
    }
}
