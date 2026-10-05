//! Opt-in native clipboard integration. Synthetic data only; no desktop capture/shell launch.
//! The protocol handshake must pass BEFORE any clipboard mutation, including the sentinel.
#![cfg(windows)]

// Invoke the real public ProcessOwner API for lifecycle regression; helper remains
// a separate native process. No alternate implementation or source-string oracle.
#[path = "../src/snipping/mod.rs"]
mod snipping;

use serde_json::Value;
use std::{ffi::c_void, io::{BufRead, BufReader, Read, Write}, process::{Child, Command, Stdio}, sync::mpsc, time::{Duration, Instant}};
use windows::{core::w, Win32::{Foundation::HWND, UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE, WINDOW_EX_STYLE, WS_POPUP}}};

const SENTINEL: &str = "jasonshell-snipping-synthetic-sentinel";
const PIXELS: [[u8; 3]; 4] = [[11, 22, 33], [44, 55, 66], [77, 88, 99], [111, 122, 133]];

// Independent Win32 reader: does not call producer's GetData/serialization helpers.
#[link(name = "user32")]
unsafe extern "system" {
    fn OpenClipboard(owner: *mut c_void) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn SetClipboardData(format: u32, memory: *mut c_void) -> *mut c_void;
    fn GetClipboardData(format: u32) -> *mut c_void;
    fn IsClipboardFormatAvailable(format: u32) -> i32;
    fn RegisterClipboardFormatW(name: *const u16) -> u32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
    fn GlobalFree(memory: *mut c_void) -> *mut c_void;
    fn GlobalLock(memory: *mut c_void) -> *mut c_void;
    fn GlobalUnlock(memory: *mut c_void) -> i32;
    fn GlobalSize(memory: *mut c_void) -> usize;
}

struct Owner(HWND);
impl Owner {
    fn new() -> Self {
        // Hidden, tiny built-in STATIC window: no full-monitor window or activation.
        Self(unsafe { CreateWindowExW(WINDOW_EX_STYLE(0), w!("STATIC"), w!("Synthetic clipboard test owner"),
            WS_POPUP, 0, 0, 1, 1, None, None, None, None) }.expect("hidden clipboard owner"))
    }
}
impl Drop for Owner { fn drop(&mut self) { unsafe { let _ = DestroyWindow(self.0); } } }
struct Open;
impl Open {
    fn new(owner: &Owner) -> Self {
        assert_ne!(unsafe { OpenClipboard(owner.0.0) }, 0, "clipboard unavailable; do not retry by clearing private content");
        Self
    }
}
impl Drop for Open { fn drop(&mut self) { unsafe { CloseClipboard(); } } }
struct Memory(*mut c_void);
impl Drop for Memory { fn drop(&mut self) { if !self.0.is_null() { unsafe { GlobalFree(self.0); } } } }

fn pump_owner_messages(deadline: Instant) {
    // OleSetClipboard can synchronously notify the previous clipboard owner's HWND.
    // That HWND belongs to this test thread, not the stdout reader thread. Waiting on
    // mpsc/sleep without pumping prevents the native producer from completing.
    let mut message = MSG::default();
    while Instant::now() < deadline
        && unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
        unsafe { let _ = TranslateMessage(&message); DispatchMessageW(&message); }
    }
}

fn put_sentinel(owner: &Owner) {
    let text: Vec<u16> = SENTINEL.encode_utf16().chain([0]).collect();
    let mut memory = Memory(unsafe { GlobalAlloc(2, text.len() * 2) });
    assert!(!memory.0.is_null());
    let address = unsafe { GlobalLock(memory.0) };
    assert!(!address.is_null());
    unsafe {
        std::ptr::copy_nonoverlapping(text.as_ptr().cast::<u8>(), address.cast::<u8>(), text.len() * 2);
        GlobalUnlock(memory.0);
    }
    let _open = Open::new(owner);
    assert_ne!(unsafe { EmptyClipboard() }, 0);
    assert!(!unsafe { SetClipboardData(13, memory.0) }.is_null());
    memory.0 = std::ptr::null_mut(); // Windows owns it only after successful transfer.
}

fn format_bytes(format: u32) -> Vec<u8> {
    let handle = unsafe { GetClipboardData(format) };
    assert!(!handle.is_null(), "required synthetic clipboard format missing");
    let len = unsafe { GlobalSize(handle) };
    assert!(len > 0 && len <= 1024 * 1024, "unexpected clipboard allocation; no contents logged");
    let address = unsafe { GlobalLock(handle) };
    assert!(!address.is_null());
    let bytes = unsafe { std::slice::from_raw_parts(address.cast::<u8>(), len) }.to_vec();
    unsafe { GlobalUnlock(handle); }
    bytes
}

fn assert_sentinel(owner: &Owner) {
    let _open = Open::new(owner);
    let bytes = format_bytes(13);
    let text: Vec<u16> = bytes.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).take_while(|unit| *unit != 0).collect();
    // Never print unexpected text; it could belong to an interfering real clipboard owner.
    assert!(String::from_utf16(&text).ok().as_deref() == Some(SENTINEL), "synthetic sentinel not preserved; clipboard contents redacted");
}

fn png_logical_len(bytes: &[u8]) -> usize {
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    let mut offset = 8usize;
    loop {
        assert!(offset + 12 <= bytes.len(), "truncated synthetic PNG framing");
        let len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let end = offset.checked_add(12).and_then(|start| start.checked_add(len)).unwrap();
        assert!(end <= bytes.len(), "synthetic PNG chunk exceeds HGLOBAL");
        if &bytes[offset + 4..offset + 8] == b"IEND" { assert_eq!(len, 0); return end; }
        offset = end;
    }
}

fn assert_png_padding(owner: &Owner) {
    let _open = Open::new(owner);
    let png_format = unsafe { RegisterClipboardFormatW(w!("PNG").as_ptr()) };
    assert_ne!(png_format, 0);
    let bytes = format_bytes(png_format); // ALL GlobalSize bytes, not just PNG decoder input.
    let logical = png_logical_len(&bytes);
    // Independent exact stored-deflate bound for this 2x2 RGBA8/filter-0 fixture.
    let filtered = 2 * (2 * 4 + 1);
    let expected = filtered + 5 * ((filtered + 65_534) / 65_535) + 6 + 57;
    assert_eq!(logical, expected);
    assert_ne!(logical % 8, 0, "fixture must exercise nonaligned logical PNG length");
    assert!(bytes[logical..].iter().all(|byte| *byte == 0), "PNG HGLOBAL padding disclosure; contents redacted");
    // GlobalSize may report exactly the logical allocation, even for unaligned input.
    // Check every byte actually exposed; never manufacture allocator rounding evidence.
    eprintln!("synthetic PNG allocation: logicalBytes={logical}; globalBytes={}; paddingBytes={}; roundedPaddingCoverage={}",
        bytes.len(), bytes.len() - logical, bytes.len() > logical);
}

fn assert_image(owner: &Owner) {
    let _open = Open::new(owner);
    assert_ne!(unsafe { IsClipboardFormatAvailable(8) }, 0, "CF_DIB not advertised");
    let dib = format_bytes(8);
    assert!(dib.len() >= 40);
    let u32_at = |offset| u32::from_le_bytes(dib[offset..offset + 4].try_into().unwrap());
    let header = u32_at(0) as usize;
    assert!(header >= 40 && header <= dib.len());
    assert_eq!(u32_at(4) as i32, 2);
    let height = u32_at(8) as i32;
    assert!(height == 2 || height == -2);
    assert_eq!(u16::from_le_bytes(dib[12..14].try_into().unwrap()), 1);
    let bits = u16::from_le_bytes(dib[14..16].try_into().unwrap());
    assert!(bits == 24 || bits == 32);
    assert_eq!(u32_at(16), 0, "required CF_DIB oracle uses BI_RGB");
    let stride = ((2 * usize::from(bits) + 31) / 32) * 4;
    assert!(dib.len() >= header + stride * 2);
    assert!(dib[header + stride * 2..].iter().all(|byte| *byte == 0), "DIB HGLOBAL padding disclosure; contents redacted");
    for (index, rgb) in PIXELS.iter().enumerate() {
        let row = if height > 0 { 1 - index / 2 } else { index / 2 };
        let offset = header + row * stride + (index % 2) * usize::from(bits / 8);
        assert!(dib[offset..offset + 3] == [rgb[2], rgb[1], rgb[0]], "synthetic DIB pixel mismatch; contents redacted");
    }
    // Current producer promises both DIB and PNG; do not let a missing PNG pass.
    let png_format = unsafe { RegisterClipboardFormatW(w!("PNG").as_ptr()) };
    assert_ne!(png_format, 0);
    assert_ne!(unsafe { IsClipboardFormatAvailable(png_format) }, 0, "PNG not advertised");
    {
        let bytes = format_bytes(png_format);
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().expect("synthetic PNG header");
        assert_eq!((reader.info().width, reader.info().height), (2, 2));
        let mut output = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut output).expect("synthetic PNG decode");
        assert_eq!((info.width, info.height), (2, 2));
        let channels = match info.color_type { png::ColorType::Rgb => 3, png::ColorType::Rgba => 4, _ => panic!("unexpected synthetic PNG color type") };
        for (pixel, rgb) in output[..info.buffer_size()].chunks_exact(channels).zip(PIXELS) {
            assert!(pixel[..3] == rgb, "synthetic PNG pixel mismatch; contents redacted");
            if channels == 4 { assert_eq!(pixel[3], 255, "synthetic PNG must be opaque"); }
        }
    }
}

struct Driver { child: Child, reports: mpsc::Receiver<String> }
impl Driver {
    fn start(args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_snip-feasibility"))
            .args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
            .spawn().expect("standalone native driver");
        let stdout = child.stdout.take().unwrap();
        let (sender, reports) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.by_ref().take(4097).read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) if line.len() <= 4096 => { if sender.send(line).is_err() { break; } }
                    _ => break,
                }
            }
        });
        Self { child, reports }
    }
    fn report(&self) -> Value {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            pump_owner_messages(deadline);
            match self.reports.try_recv() {
                Ok(line) => return serde_json::from_str(&line).expect("metadata-only JSON report required"),
                Err(mpsc::TryRecvError::Disconnected) => panic!("native report stream closed before report"),
                Err(mpsc::TryRecvError::Empty) => {}
            }
            assert!(Instant::now() < deadline, "bounded JSON native report missing after five seconds");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    fn exit(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            pump_owner_messages(deadline);
            if let Some(status) = self.child.try_wait().unwrap() { assert!(status.success(), "native test driver failed"); break; }
            assert!(Instant::now() < deadline, "native test driver did not exit within budget");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn release(&mut self) { self.child.stdin.as_mut().unwrap().write_all(b"release-after-observation\n").unwrap(); }
}
impl Drop for Driver {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() { let _ = self.child.kill(); }
        let _ = self.child.wait(); // Only this test's child; never the running shell.
    }
}

fn preflight() {
    assert_eq!(std::env::var("JASONSHELL_SNIP_CLIPBOARD_TESTS").as_deref(), Ok("1"), "requires explicit synthetic clipboard consent env; run serially");
    let mut driver = Driver::start(&["--clipboard-test-protocol"]);
    assert_eq!(driver.report(), serde_json::json!({ "protocol": "snipping-clipboard-test-v1", "sideEffects": false }));
    driver.exit(); // No sentinel/window until this exact nonmutating handshake succeeds.
}

fn publish(case: &str, hold: bool) -> Driver {
    let mut args = vec!["--consent-synthetic-clipboard-test", "--clipboard-test-case", case];
    if hold { args.push("--hold-until-reader"); }
    Driver::start(&args)
}

#[test]
#[ignore = "opt-in native test-driver readiness; run serially with explicit consent env"]
fn snipping_clipboard_protocol_is_available() { preflight(); }

#[test]
#[ignore = "opt-in synthetic clipboard replacement; run --ignored --test-threads=1 with consent env"]
fn snipping_clipboard_precommit_failures_preserve_sentinel() {
    preflight();
    let owner = Owner::new();
    for case in ["invalid-image", "budget", "allocation", "ole-set-rejected"] {
        put_sentinel(&owner);
        let mut driver = publish(case, false);
        let report = driver.report();
        assert_eq!(report["status"], "rejected");
        assert_eq!(report["committed"], false);
        assert_eq!(report["durable"], false);
        assert_eq!(report["case"], case);
        driver.exit();
        assert_sentinel(&owner);
    }
}

#[test]
#[ignore = "opt-in synthetic clipboard replacement and bounded busy lock; run serially"]
fn snipping_clipboard_real_busy_rejection_preserves_sentinel() {
    preflight();
    let owner = Owner::new();
    put_sentinel(&owner);
    let open = Open::new(&owner);
    let mut driver = publish("real-busy", false);
    let report = driver.report();
    assert_eq!(report["status"], "rejected");
    assert_eq!(report["code"], "clipboard-busy");
    assert_eq!(report["committed"], false);
    driver.exit();
    drop(open);
    assert_sentinel(&owner);
}

#[test]
#[ignore = "opt-in synthetic clipboard replacement; independent reader runs only after producer exit"]
fn snipping_clipboard_formats_and_pixels_persist_after_producer_exit() {
    preflight();
    let owner = Owner::new();
    put_sentinel(&owner);
    let mut driver = publish("success", false);
    let report = driver.report();
    assert_eq!(report["status"], "committed");
    assert_eq!(report["committed"], true);
    assert_eq!(report["durable"], true);
    driver.exit();
    assert_image(&owner);
}

#[test]
#[ignore = "opt-in synthetic clipboard replacement; requires postcommit flush failpoint and STA keepalive"]
fn snipping_clipboard_postcommit_warning_is_truthful_and_owner_remains_live() {
    preflight();
    let owner = Owner::new();
    put_sentinel(&owner);
    let mut driver = publish("flush-after-commit", true);
    let report = driver.report();
    assert_eq!(report["status"], "committed-warning");
    assert_eq!(report["code"], "clipboard-committed-not-durable");
    assert_eq!(report["committed"], true);
    assert_eq!(report["durable"], false);
    assert!(driver.child.try_wait().unwrap().is_none(), "committed delayed owner exited before materialization");
    assert_image(&owner); // GetClipboardData exercises the live owner's actual IDataObject.
    assert_png_padding(&owner);
    driver.release();
    let durable = driver.report();
    assert_eq!(durable["status"], "committed");
    assert_eq!(durable["durable"], true);
    driver.exit();
    assert_image(&owner);
}

#[test]
#[ignore = "opt-in synthetic clipboard replacement; requires native repeated/failure cleanup observations"]
fn snipping_clipboard_repeated_failure_cleanup_returns_to_warmed_baseline() {
    preflight();
    let mut driver = publish("repeat-cleanup-20", false);
    let report = driver.report();
    assert_eq!(report["iterations"], 20);
    assert_eq!(report["successfulPublications"], 20);
    assert_eq!(report["failedPublications"], 20);
    for field in ["liveObjects", "liveGlobalAllocations", "liveStagedBytes"] {
        assert_eq!(report["after"][field], 0, "retained owned clipboard resource: {field}");
    }
    for field in ["gdi", "user", "processHandles"] {
        let before = report["before"][field].as_u64().expect("warmed native baseline");
        let after = report["after"][field].as_u64().expect("native cleanup count");
        assert!(after <= before, "native count grew after warmup: {field}");
    }
    driver.exit();
}

#[test]
#[ignore = "opt-in synthetic clipboard replacement; inspect full rounded PNG HGLOBAL after producer exit"]
fn snipping_clipboard_nonaligned_png_padding_is_zero_after_producer_exit() {
    preflight();
    let owner = Owner::new();
    put_sentinel(&owner);
    let mut driver = publish("success", false);
    let report = driver.report();
    assert_eq!(report["committed"], true);
    assert_eq!(report["durable"], true);
    driver.exit();
    assert_image(&owner);
    assert_png_padding(&owner);
}

fn assert_permanent_stall(case: &str, unknown: bool, shutdown_stall: bool) {
    preflight();
    let owner = Owner::new();
    put_sentinel(&owner);
    let started = Instant::now();
    let mut driver = publish(case, false);
    let report = driver.report();
    assert_eq!(report["case"], case);
    if unknown {
        assert_eq!(report["status"], "publication-unknown");
        assert_eq!(report["code"], "clipboard-publication-unknown");
        assert!(report.get("committed").is_some_and(Value::is_null));
        assert!(report.get("durable").is_some_and(Value::is_null));
        // No preservation assertion: absence of acknowledgement is NOT rejection.
    } else {
        assert_eq!(report["status"], "committed-warning");
        assert_eq!(report["code"], "clipboard-durability-lost");
        assert_eq!(report["committed"], true);
        assert_eq!(report["durable"], false);
    }
    for field in ["forcedShutdown", "helperExited", "writerDrained"] { assert_eq!(report[field], true, "private helper cleanup: {field}"); }
    assert_eq!(report["failpoint"], "permanent-helper-stall-at-native-boundary-not-an-OS-COM-hang-claim");
    let operation_ms = report["operationMs"].as_u64().expect("operation timing");
    let shutdown_ms = report["shutdownMs"].as_u64().expect("shutdown timing");
    eprintln!("private helper stall: case={case}; status={}; operationMs={operation_ms}; shutdownMs={shutdown_ms}; helperExited={}; writerDrained={}",
        report["status"], report["helperExited"], report["writerDrained"]);
    assert!(operation_ms <= 3000, "operation exceeded declared scheduling/cleanup bound");
    assert!(shutdown_ms <= 1000, "shutdown exceeded declared scheduling/cleanup bound");
    if shutdown_stall { assert!(shutdown_ms >= 450, "permanent shutdown stall did not exercise deadline"); }
    else { assert!(operation_ms >= 1800, "permanent operation stall did not exercise deadline"); }
    driver.exit();
    assert!(started.elapsed() <= Duration::from_millis(4500), "whole standalone driver exceeded bounded completion");
}

#[test]
#[ignore = "opt-in synthetic clipboard; private helper stalls before set, outcome must remain unknown"]
fn snipping_clipboard_permanent_pre_set_stall_is_unknown_and_drains_writer() {
    assert_permanent_stall("helper-hang-operation", true, false);
}

#[test]
#[ignore = "opt-in synthetic clipboard; private helper stalls after acknowledged native commit"]
fn snipping_clipboard_permanent_acknowledged_stall_reports_loss_and_drains_writer() {
    assert_permanent_stall("helper-hang-flush", false, false);
}

#[test]
#[ignore = "opt-in synthetic clipboard; private helper shutdown stall must not block owner indefinitely"]
fn snipping_clipboard_permanent_shutdown_stall_reports_loss_and_drains_writer() {
    assert_permanent_stall("helper-hang-shutdown", false, true);
}

#[test]
#[ignore = "opt-in hidden synthetic native windows; actual WM_CLOSE invalidation, no fabricated HWND reuse"]
fn snipping_hidden_window_external_destroy_invalidates_authority_without_harming_replacement() {
    preflight();
    let mut driver = Driver::start(&["--consent-synthetic-native", "--window-lifetime-check"]);
    let report = driver.report();
    assert_eq!(report["windowLifetime"], "passed");
    assert_eq!(report["hiddenOnly"], true);
    let reused = report["handleReused"].as_bool().expect("actual HWND reuse observation required");
    driver.exit();
    eprintln!("hidden-window destruction check: actual handleReused={reused}; no forced-reuse claim");
}

#[test]
#[ignore = "opt-in synthetic clipboard; acknowledged durable image must remain durable after shutdown and flush"]
fn snipping_clipboard_known_durable_shutdown_then_flush_never_reports_loss() {
    use snipping::{clipboard::Failpoints, clipboard_process::ProcessOwner, geometry::CroppedImage};
    preflight();
    let program = std::path::Path::new(env!("CARGO_BIN_EXE_snip-feasibility"));
    let mut publisher = ProcessOwner::spawn(program).expect("private native helper");
    let image = CroppedImage { width: 2, height: 2,
        pixels: PIXELS.iter().flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255]).collect() };
    let retained = image.pixels.capacity();
    let published = publisher.publish(image, retained, Failpoints::default()).metadata();
    assert_eq!(published["committed"], true);
    assert_eq!(published["durable"], true);
    let shutdown = publisher.shutdown();
    assert!(shutdown.exited && shutdown.writer_drained);
    let acknowledged = shutdown.outcome.metadata();
    assert_eq!(acknowledged["committed"], true);
    assert_eq!(acknowledged["durable"], true);
    let subsequent = publisher.flush().metadata();
    // Independent Windows reader establishes that the pixels still survive exit,
    // before the RED status assertion: this is metadata loss, not actual data loss.
    let owner = Owner::new();
    assert_image(&owner);
    assert_eq!(subsequent, acknowledged,
        "flush after successful durable shutdown must preserve acknowledged commitment/durability");
}

#[test]
#[ignore = "opt-in synthetic clipboard; redundant flush must short-circuit known durable state"]
fn snipping_clipboard_known_durable_redundant_timeout_never_reports_loss() {
    preflight();
    // Arm response unavailability, then verify normal flush avoids transport entirely.
    let mut driver = publish("durable-redundant-flush-timeout", false);
    let report = driver.report();
    assert_eq!(report["case"], "durable-redundant-flush-timeout");
    assert_eq!(report["initialCommitted"], true);
    assert_eq!(report["initialDurable"], true);
    assert_eq!(report["committed"], true);
    assert_eq!(report["durable"], true);
    assert_eq!(report["status"], "committed");
    assert_eq!(report["failpoint"], "redundant-flush-response-unavailable-after-confirmed-durable-publication");
    assert_eq!(report["writerDrained"], true);
    assert_eq!(report["helperExited"], true);
    assert_eq!(report["faultUsed"], false);
    assert_eq!(report["shortCircuited"], true);
    assert!(report["operationMs"].as_u64().expect("bounded operation timing") <= 3000);
    eprintln!("known-durable redundant flush: faultUsed=false; shortCircuited=true; committed=true; durable=true; operationMs={}", report["operationMs"]);
    driver.exit();
    let owner = Owner::new();
    assert_image(&owner);
}
