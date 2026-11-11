//! Pure synthetic PNG correctness and opt-in encode-only measurement.
//! No Publisher/Win32 calls, capture, clipboard, foreground or shell launch.
//! Benchmark: cargo test --release --test snipping_png_codec
//!   warmed_real_resolution_encode_baseline -- --ignored --exact --nocapture --test-threads=1
//! Compare identical command/profile/inputs on this machine, serially: two warmups,
//! five samples, per-monitor and sequential-pair median/range. No universal timing
//! threshold: retain byte-identical format, checksums, pixels and budget behavior;
//! compare repeated before/after ranges separately from native journey latency.
#![cfg(windows)]
#![allow(dead_code)] // Include production modules without calling native entrypoints.

#[path = "../src/snipping/clipboard.rs"]
mod clipboard;
#[path = "../src/snipping/geometry.rs"]
mod geometry;

use clipboard::{png_rgba, Failure};
use geometry::MAX_STAGING_BYTES;
use sha2::{Digest, Sha256};
use std::{hint::black_box, io::Cursor, time::Instant};

#[derive(Clone, Copy, Default)]
struct CodecAllocations {
    allocations: usize,
    reallocations: usize,
    deallocations: usize,
    bytes: usize,
}
thread_local! {
    static WATCH_CODEC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CODEC_ALLOCATIONS: std::cell::Cell<CodecAllocations> = const { std::cell::Cell::new(CodecAllocations { allocations: 0, reallocations: 0, deallocations: 0, bytes: 0 }) };
}
struct CodecAllocator;
fn record_codec_allocation(bytes: usize, reallocation: bool, deallocation: bool) {
    let _ = WATCH_CODEC.try_with(|watch| {
        if watch.get() {
            let _ = CODEC_ALLOCATIONS.try_with(|counter| {
                let mut value = counter.get();
                value.allocations += usize::from(!reallocation && !deallocation);
                value.reallocations += usize::from(reallocation);
                value.deallocations += usize::from(deallocation);
                value.bytes += bytes;
                counter.set(value);
            });
        }
    });
}
// SAFETY: forwards every allocation/layout unchanged to System. Const TLS Cells
// only count the explicitly armed current test thread; no allocation in callbacks.
unsafe impl std::alloc::GlobalAlloc for CodecAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let result = unsafe { std::alloc::System.alloc(layout) };
        record_codec_allocation(layout.size(), false, false);
        result
    }
    unsafe fn alloc_zeroed(&self, layout: std::alloc::Layout) -> *mut u8 {
        let result = unsafe { std::alloc::System.alloc_zeroed(layout) };
        record_codec_allocation(layout.size(), false, false);
        result
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: std::alloc::Layout, size: usize) -> *mut u8 {
        let result = unsafe { std::alloc::System.realloc(pointer, layout, size) };
        record_codec_allocation(size, true, false);
        result
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: std::alloc::Layout) {
        record_codec_allocation(0, false, true);
        unsafe { std::alloc::System.dealloc(pointer, layout) };
    }
}
#[global_allocator]
static CODEC_ALLOCATOR: CodecAllocator = CodecAllocator;
fn count_codec_allocations<T>(action: impl FnOnce() -> T) -> (T, CodecAllocations) {
    struct Stop;
    impl Drop for Stop {
        fn drop(&mut self) {
            WATCH_CODEC.with(|watch| watch.set(false));
        }
    }
    CODEC_ALLOCATIONS.with(|counter| counter.set(CodecAllocations::default()));
    WATCH_CODEC.with(|watch| watch.set(true));
    let stop = Stop;
    let result = action();
    drop(stop);
    (result, CODEC_ALLOCATIONS.with(std::cell::Cell::get))
}

fn pixels(width: u32, height: u32, solid: Option<u8>) -> Vec<u8> {
    let mut bytes = vec![0; width as usize * height as usize * 4];
    for (index, pixel) in bytes.chunks_exact_mut(4).enumerate() {
        // Predetermined nonprivate opaque RGBA, including 0 and 255 channels.
        let x = index % width as usize;
        let y = index / width as usize;
        pixel.copy_from_slice(&[
            solid.unwrap_or((x & 255) as u8),
            solid.unwrap_or((y & 255) as u8),
            solid.unwrap_or(((x ^ y) & 255) as u8),
            255,
        ]);
    }
    bytes
}

fn png_len(width: u32, height: u32) -> usize {
    let raw = (width as usize * 4 + 1) * height as usize;
    raw + raw.div_ceil(65_535) * 5 + 63
}

// Independent table CRC, not the production per-byte eight-bit loop.
fn crc32(bytes: &[u8]) -> u32 {
    let table: [u32; 256] = std::array::from_fn(|index| {
        let mut crc = index as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
        crc
    });
    !bytes.iter().fold(u32::MAX, |crc, byte| {
        (crc >> 8) ^ table[((crc ^ u32::from(*byte)) & 255) as usize]
    })
}

// Independent weighted-sum Adler identity (u64, bounded by these fixtures), not
// the producer's per-byte modulo recurrence or a future 5552-byte implementation.
fn adler32(bytes: &[u8]) -> u32 {
    let n = bytes.len() as u64;
    let a = (1 + bytes.iter().map(|byte| u64::from(*byte)).sum::<u64>()) % 65_521;
    let b = (n + bytes
        .iter()
        .enumerate()
        .map(|(index, byte)| (n - index as u64) * u64::from(*byte))
        .sum::<u64>())
        % 65_521;
    ((b as u32) << 16) | a as u32
}

fn assert_png(bytes: &[u8], width: u32, height: u32, expected: &[u8]) {
    assert_eq!(bytes.len(), png_len(width, height));
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut offset = 8;
    let mut idat = None;
    for kind in [b"IHDR", b"IDAT", b"IEND"] {
        let len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        assert_eq!(&bytes[offset + 4..offset + 8], kind);
        let end = offset + 8 + len;
        let payload = &bytes[offset + 8..end];
        assert_eq!(
            crc32(&bytes[offset + 4..end]),
            u32::from_be_bytes(bytes[end..end + 4].try_into().unwrap())
        );
        match kind {
            b"IHDR" => {
                let mut header = Vec::from(width.to_be_bytes());
                header.extend_from_slice(&height.to_be_bytes());
                header.extend_from_slice(&[8, 6, 0, 0, 0]);
                assert_eq!(payload, header);
            }
            b"IDAT" => idat = Some(payload),
            b"IEND" => assert!(payload.is_empty()),
            _ => unreachable!(),
        }
        offset = end + 4;
    }
    assert_eq!(offset, bytes.len(), "no trailing chunks/padding");
    let zlib = idat.unwrap();
    assert_eq!(&zlib[..2], &[0x78, 0x01]);
    let raw_len = (width as usize * 4 + 1) * height as usize;
    let mut raw = Vec::with_capacity(raw_len);
    let mut cursor = 2;
    while raw.len() < raw_len {
        let count = (raw_len - raw.len()).min(65_535);
        assert_eq!(zlib[cursor], u8::from(raw.len() + count == raw_len));
        let len = u16::from_le_bytes(zlib[cursor + 1..cursor + 3].try_into().unwrap());
        let nlen = u16::from_le_bytes(zlib[cursor + 3..cursor + 5].try_into().unwrap());
        assert_eq!(usize::from(len), count);
        assert_eq!(nlen, !len);
        raw.extend_from_slice(&zlib[cursor + 5..cursor + 5 + count]);
        cursor += 5 + count;
    }
    assert_eq!(cursor + 4, zlib.len());
    assert_eq!(
        adler32(&raw),
        u32::from_be_bytes(zlib[cursor..].try_into().unwrap())
    );
    let stride = width as usize * 4;
    for (row, expected_row) in raw
        .chunks_exact(stride + 1)
        .zip(expected.chunks_exact(stride))
    {
        assert_eq!(row[0], 0, "filter must remain None");
        assert_eq!(&row[1..], expected_row);
    }
    drop(raw);
    // Independent library decode, explicitly including CRC and Adler verification.
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.ignore_checksums(false);
    let mut reader = decoder.read_info().expect("synthetic PNG header");
    assert_eq!((reader.info().width, reader.info().height), (width, height));
    let mut decoded = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut decoded)
        .expect("strict independent PNG decode");
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    assert_eq!(&decoded[..info.buffer_size()], expected);
}

#[test]
fn independent_checksum_oracles_match_known_vectors() {
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    assert_eq!(crc32(b""), 0);
    assert_eq!(adler32(b""), 1);
}

#[test]
fn codec_preserves_pixels_checksums_and_stored_block_boundaries() {
    // Filtered lengths near 5552: 5549,5553,5557; near 65535:
    // 65533,65535,65537. Includes exact full final block and multirow transitions.
    for (width, height) in [
        (1, 1),
        (2, 3),
        (1387, 1),
        (1388, 1),
        (1389, 1),
        (16383, 1),
        (5461, 3),
        (16384, 1),
        (4096, 5),
    ] {
        for solid in [Some(0), Some(255), None] {
            let input = pixels(width, height, solid);
            let png = png_rgba(width, height, &input, input.len()).unwrap();
            assert!(png.capacity() >= png.len());
            assert!(input.len() + png.capacity() <= MAX_STAGING_BYTES);
            assert_png(&png, width, height, &input);
        }
    }
}

#[test]
fn codec_rejects_invalid_inputs_without_large_allocations() {
    for (width, height, input, retained) in [
        (0, 1, vec![], 0),
        (1, 0, vec![], 0),
        (1, 1, vec![0; 3], 4),
        (1, 1, vec![0; 5], 5),
        (1, 1, vec![0; 4], 3),
        (16385, 1, vec![0; 16385 * 4], 16385 * 4),
        (1, 16385, vec![0; 16385 * 4], 16385 * 4),
        (16384, 16384, vec![], 0),
        (u32::MAX, u32::MAX, vec![], 0),
    ] {
        assert_eq!(
            png_rgba(width, height, &input, retained),
            Err(Failure::InvalidImage)
        );
    }
}

#[test]
fn codec_enforces_retained_plus_actual_output_capacity_budget() {
    assert_eq!(MAX_STAGING_BYTES, 512 * 1024 * 1024);
    let input = pixels(1, 1, None);
    let total = png_len(1, 1);
    let exact = png_rgba(1, 1, &input, MAX_STAGING_BYTES - total).unwrap();
    assert_eq!(exact.len(), total);
    assert_eq!(exact.capacity(), total);
    for retained in [MAX_STAGING_BYTES - total + 1, MAX_STAGING_BYTES, usize::MAX] {
        assert_eq!(png_rgba(1, 1, &input, retained), Err(Failure::Budget));
    }
}

#[test]
fn real_resolution_payloads_decode_pixel_identically() {
    for (width, height, golden_sha256) in [
        (
            2560,
            1440,
            "0226e03af2d0658b2b91bfd955939e79893bdb1009887cf975709110bad277cd",
        ),
        (
            3840,
            2160,
            "e95f10d85bdf7b17b7debc5caf4d24b038162bf5a78806d40f00507dd61421ea",
        ),
    ] {
        let input = pixels(width, height, None);
        let png = png_rgba(width, height, &input, input.len()).unwrap();
        assert_png(&png, width, height, &input);
        assert_eq!(
            format!("{:x}", Sha256::digest(&png)),
            golden_sha256,
            "future checksum optimization must preserve exact PNG bytes"
        );
        eprintln!(
            "CODEC exact width={width} height={height} inputBytes={} pngBytes={} sha256={:x}",
            input.len(),
            png.len(),
            Sha256::digest(&png)
        );
    }
}

#[test]
#[ignore = "opt-in synthetic release encode-only timing; no native side effects"]
fn warmed_real_resolution_encode_baseline() {
    assert!(
        !cfg!(debug_assertions),
        "measure --release, never debug timings"
    );
    let dimensions = [(2560, 1440), (3840, 2160)];
    let inputs: Vec<_> = dimensions
        .iter()
        .map(|&(w, h)| pixels(w, h, None))
        .collect();
    // Production retains both frozen monitor buffers during each monitor encode.
    let retained: usize = inputs.iter().map(Vec::len).sum();
    for _ in 0..2 {
        for (index, &(width, height)) in dimensions.iter().enumerate() {
            black_box(png_rgba(width, height, black_box(&inputs[index]), retained).unwrap());
        }
    }
    let mut samples = [Vec::new(), Vec::new()];
    let mut pair_samples = Vec::new();
    for _ in 0..5 {
        let mut pair_ms = 0.0;
        for (index, &(width, height)) in dimensions.iter().enumerate() {
            let start = Instant::now();
            let png = black_box(
                png_rgba(
                    black_box(width),
                    black_box(height),
                    black_box(&inputs[index]),
                    black_box(retained),
                )
                .unwrap(),
            );
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            samples[index].push(ms);
            pair_ms += ms;
            // No input generation, decoding, hashing, output destruction or logging
            // inside timed span. PNG allocation/encoding are necessarily included.
            assert_eq!(png.len(), png_len(width, height));
            assert!(retained + png.capacity() <= MAX_STAGING_BYTES);
        }
        pair_samples.push(pair_ms);
    }
    for (index, &(width, height)) in dimensions.iter().enumerate() {
        samples[index].sort_by(f64::total_cmp);
        eprintln!("CODEC baseline width={width} height={height} pngBytes={} warmups=2 samples=5 minMs={:.3} medianMs={:.3} maxMs={:.3} sortedMs={:?}",
            png_len(width, height), samples[index][0], samples[index][2], samples[index][4], samples[index]);
    }
    pair_samples.sort_by(f64::total_cmp);
    eprintln!("CODEC baseline sequentialPair retainedBytes={retained} minMs={:.3} medianMs={:.3} maxMs={:.3} sortedMs={pair_samples:?} nativeLatencyAttribution=false",
        pair_samples[0], pair_samples[2], pair_samples[4]);
}

// New TRANSPORT API acceptance. Existing stored/save/clipboard golden oracles
// above remain unchanged. flate2 is an independent TEST decoder only: its heap
// constructors are deliberately NOT an acceptable production encoder strategy.
fn assert_transport_png(bytes: &[u8], width: u32, height: u32, expected: &[u8]) {
    use std::io::Read;
    assert!(bytes.len() <= png_len(width, height));
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let mut cursor = 8;
    let mut zlib = None;
    for kind in [b"IHDR", b"IDAT", b"IEND"] {
        let len = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        assert_eq!(&bytes[cursor + 4..cursor + 8], kind);
        let end = cursor + 8 + len;
        assert_eq!(
            crc32(&bytes[cursor + 4..end]),
            u32::from_be_bytes(bytes[end..end + 4].try_into().unwrap())
        );
        let data = &bytes[cursor + 8..end];
        match kind {
            b"IHDR" => {
                let mut header = Vec::from(width.to_be_bytes());
                header.extend_from_slice(&height.to_be_bytes());
                header.extend_from_slice(&[8, 6, 0, 0, 0]);
                assert_eq!(data, header);
            }
            b"IDAT" => zlib = Some(data),
            b"IEND" => assert!(data.is_empty()),
            _ => unreachable!(),
        }
        cursor = end + 4;
    }
    assert_eq!(cursor, bytes.len());
    let zlib = zlib.unwrap();
    assert_eq!(&zlib[..2], &[0x78, 0x01]);
    assert!(
        ((zlib[2] >> 1) & 3) <= 1,
        "stored fallback or fixed Huffman only"
    );
    let raw_len = (width as usize * 4 + 1) * height as usize;
    let mut decoder = flate2::read::ZlibDecoder::new(zlib);
    let mut raw = Vec::with_capacity(raw_len);
    decoder
        .by_ref()
        .take(raw_len as u64 + 1)
        .read_to_end(&mut raw)
        .expect("independent bounded zlib decode");
    assert_eq!(raw.len(), raw_len);
    assert_eq!(
        decoder.total_in(),
        zlib.len() as u64,
        "no trailing zlib stream"
    );
    assert_eq!(
        adler32(&raw),
        u32::from_be_bytes(zlib[zlib.len() - 4..].try_into().unwrap())
    );
    for (row, wanted) in raw
        .chunks_exact(width as usize * 4 + 1)
        .zip(expected.chunks_exact(width as usize * 4))
    {
        assert_eq!(row[0], 0, "transport must retain filter0 across rows/runs");
        assert_eq!(&row[1..], wanted);
    }
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.ignore_checksums(false);
    let mut reader = decoder.read_info().unwrap();
    let mut output = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut output)
        .expect("independent fixed-Huffman-capable PNG decoder");
    assert_eq!((info.width, info.height), (width, height));
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    assert_eq!(&output[..info.buffer_size()], expected);
}

fn entropy_pixels(width: u32, height: u32) -> Vec<u8> {
    let mut value = 0x1234_5678u32;
    let mut input = vec![0; width as usize * height as usize * 4];
    let mut previous = [0; 4];
    for pixel in input.chunks_exact_mut(4) {
        for byte in &mut pixel[..3] {
            value ^= value << 13;
            value ^= value >> 17;
            value ^= value << 5;
            *byte = value as u8;
        }
        pixel[3] = 255;
        if pixel == previous {
            pixel[0] ^= 1;
        }
        previous.copy_from_slice(pixel);
    }
    input
}

#[test]
fn transport_flat_payload_is_lossless_and_materially_smaller() {
    let input = [11, 22, 33, 255].repeat(64 * 64);
    let stored = png_rgba(64, 64, &input, input.len()).unwrap();
    let compact = clipboard::png_rgba_transport(64, 64, &input, input.len()).unwrap();
    assert_transport_png(&compact, 64, 64, &input);
    assert!(
        compact.len() < stored.len() / 20,
        "flat transport PNG must materially shrink, not just change codec label"
    );
    assert!(compact.capacity() <= stored.capacity());
}

#[test]
fn transport_runlength_and_row_filter_boundaries_are_lossless() {
    for width in [
        1, 2, 3, 4, 7, 8, 9, 10, 11, 16, 17, 31, 32, 33, 63, 64, 65, 66, 67, 129, 130, 256, 257,
        258, 259, 1024,
    ] {
        for rgba in [
            [0, 0, 0, 255],
            [255; 4],
            [7, 19, 251, 255],
            [7, 19, 251, 0],
            [7, 19, 251, 127],
        ] {
            let input = rgba.repeat(width as usize * 3);
            let output = clipboard::png_rgba_transport(width, 3, &input, input.len()).unwrap();
            assert_transport_png(&output, width, 3, &input);
            assert!(output.capacity() <= png_len(width, 3));
        }
    }
}

#[test]
fn transport_mixed_runs_and_nonrepeated_pixels_are_lossless() {
    let (width, height) = (129, 7);
    let mut input = entropy_pixels(width, height);
    for row in input.chunks_exact_mut(width as usize * 4) {
        for pixel in row[..64 * 4].chunks_exact_mut(4) {
            pixel.copy_from_slice(&[3, 8, 27, 255]);
        }
    }
    let output = clipboard::png_rgba_transport(width, height, &input, input.len()).unwrap();
    assert_transport_png(&output, width, height, &input);
    assert!(output.capacity() <= png_len(width, height));
}

#[test]
fn transport_nonrepeating_entropy_falls_back_to_identical_stored_bytes() {
    let (width, height) = (127, 17);
    let input = entropy_pixels(width, height);
    assert!(input
        .chunks_exact(4)
        .collect::<Vec<_>>()
        .windows(2)
        .all(|pair| pair[0] != pair[1]));
    let stored = png_rgba(width, height, &input, input.len()).unwrap();
    let output = clipboard::png_rgba_transport(width, height, &input, input.len()).unwrap();
    assert_eq!(
        output, stored,
        "incompressible fallback must reuse exact original stored format"
    );
    assert_transport_png(&output, width, height, &input);
}

#[test]
fn transport_invalid_input_and_retained_capacity_budget_match_existing_contract() {
    for (width, height, input, retained) in [
        (0, 1, vec![], 0),
        (1, 0, vec![], 0),
        (1, 1, vec![0; 3], 4),
        (1, 1, vec![0; 5], 5),
        (1, 1, vec![0; 4], 3),
        (16385, 1, vec![0; 16385 * 4], 16385 * 4),
        (1, 16385, vec![0; 16385 * 4], 16385 * 4),
        (16384, 16384, vec![], 0),
        (u32::MAX, u32::MAX, vec![], 0),
    ] {
        assert_eq!(
            clipboard::png_rgba_transport(width, height, &input, retained),
            Err(Failure::InvalidImage)
        );
    }
    let input = [11, 22, 33, 255].repeat(64 * 64);
    let limit = png_len(64, 64);
    let exact = clipboard::png_rgba_transport(64, 64, &input, MAX_STAGING_BYTES - limit).unwrap();
    assert!(exact.capacity() <= limit);
    assert!(MAX_STAGING_BYTES - limit + exact.capacity() <= MAX_STAGING_BYTES);
    assert_transport_png(&exact, 64, 64, &input);
    for retained in [MAX_STAGING_BYTES - limit + 1, MAX_STAGING_BYTES, usize::MAX] {
        assert_eq!(
            clipboard::png_rgba_transport(64, 64, &input, retained),
            Err(Failure::Budget)
        );
    }
}

#[test]
fn transport_owns_only_one_output_allocation_in_compact_and_fallback_paths() {
    for input in [[11, 22, 33, 255].repeat(64 * 64), entropy_pixels(64, 64)] {
        let (output, counts) =
            count_codec_allocations(|| clipboard::png_rgba_transport(64, 64, &input, input.len()));
        let output = output.unwrap();
        assert_eq!(
            counts.allocations, 1,
            "one fallibly preallocated output; no heap dictionary/workspace"
        );
        assert_eq!(
            counts.reallocations, 0,
            "capacity exhaustion must reuse Vec for stored fallback"
        );
        assert_eq!(
            counts.deallocations, 0,
            "no temporary heap workspace released before return"
        );
        assert_eq!(counts.bytes, output.capacity());
        assert_transport_png(&output, 64, 64, &input);
    }
    let input = [11, 22, 33, 255];
    let (result, counts) =
        count_codec_allocations(|| clipboard::png_rgba_transport(1, 1, &input, MAX_STAGING_BYTES));
    assert_eq!(result, Err(Failure::Budget));
    assert_eq!(
        (
            counts.allocations,
            counts.reallocations,
            counts.deallocations
        ),
        (0, 0, 0)
    );
}
