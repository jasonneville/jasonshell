//! Synthetic PNG/file-boundary acceptance; never invokes picker, captures desktop or saves user data.
//! API contract in tests/snipping-product-handoff.md. Missing module is intentional RED.
#[path = "../src/snipping/save.rs"]
mod save;

use save::{publish_png, publish_png_with_failpoint, SaveFailpoint};
use std::{fs, path::PathBuf, sync::atomic::{AtomicU64, Ordering}};

static NEXT: AtomicU64 = AtomicU64::new(0);
const PIXELS: [u8; 16] = [11,22,33,255,44,55,66,255,77,88,99,255,111,122,133,255];
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        // Approved local opencode temp root, not an arbitrary user destination.
        let root = PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("Windows local app data"))
            .join("Temp").join("opencode").join(format!("snip-save-test-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn target(&self) -> PathBuf { self.0.join("synthetic résumé image.png") }
    fn assert_only_target(&self) {
        assert_eq!(fs::read_dir(&self.0).unwrap().count(), 1, "staged sibling leaked");
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
fn png() -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&PIXELS).unwrap();
    }
    bytes
}
fn assert_image(path: &std::path::Path) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(fs::read(path).unwrap())).read_info().unwrap();
    assert_eq!((reader.info().width, reader.info().height), (2, 2));
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert!(pixels[..info.buffer_size()] == PIXELS, "synthetic saved PNG pixel mismatch; contents redacted");
}

#[test]
fn snipping_save_new_unicode_destination_roundtrips_exact_png_without_leaked_sibling() {
    let fixture = Fixture::new();
    publish_png(&fixture.target(), &png()).unwrap();
    assert_image(&fixture.target());
    fixture.assert_only_target();
}

#[test]
fn snipping_save_authorized_overwrite_publishes_complete_image() {
    let fixture = Fixture::new();
    fs::write(fixture.target(), b"synthetic-existing-file").unwrap();
    publish_png(&fixture.target(), &png()).unwrap();
    assert_image(&fixture.target());
    fixture.assert_only_target();
}

#[test]
fn snipping_save_each_prepublication_failure_preserves_existing_destination_exactly() {
    for fail in [SaveFailpoint::CreateSibling, SaveFailpoint::WriteSibling, SaveFailpoint::FlushSibling, SaveFailpoint::PublishSibling] {
        let fixture = Fixture::new();
        fs::write(fixture.target(), b"synthetic-existing-file").unwrap();
        assert!(publish_png_with_failpoint(&fixture.target(), &png(), fail).is_err());
        assert!(fs::read(fixture.target()).unwrap() == b"synthetic-existing-file", "destination truncated before publication");
        fixture.assert_only_target();
    }
}

#[test]
fn snipping_save_each_prepublication_failure_creates_no_new_destination_or_sibling() {
    for fail in [SaveFailpoint::CreateSibling, SaveFailpoint::WriteSibling, SaveFailpoint::FlushSibling, SaveFailpoint::PublishSibling] {
        let fixture = Fixture::new();
        assert!(publish_png_with_failpoint(&fixture.target(), &png(), fail).is_err());
        assert!(!fixture.target().exists());
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 0);
    }
}
