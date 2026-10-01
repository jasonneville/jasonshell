//! Disk acceptance tests. Loader callback deliberately substitutes for real ONNX inference.
#[path = "../src/speech_model_install.rs"]
mod speech_model_install;
#[path = "../src/speech_model.rs"]
mod speech_model;

use speech_model_install::{install_archive, resolve_installed_model, InstallLimits};
use std::{fs, io::Write, path::{Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}};

const FILES: [&str; 3] = ["encoder-model.int8.onnx", "decoder_joint-model.int8.onnx", "vocab.txt"];
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("jasonshell-install-test-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn data(&self) -> PathBuf { self.0.join("app-local-data") }
    fn archive(&self, name: &str, entries: &[(&str, &[u8], u8)]) -> PathBuf {
        let mut bytes = Vec::new();
        for (name, contents, kind) in entries {
            let mut header = [0u8; 512];
            assert!(name.len() < 100);
            header[..name.len()].copy_from_slice(name.as_bytes());
            header[100..108].copy_from_slice(b"0000644\0");
            header[108..116].copy_from_slice(b"0000000\0");
            header[116..124].copy_from_slice(b"0000000\0");
            header[124..136].copy_from_slice(format!("{:011o}\0", contents.len()).as_bytes());
            header[136..148].copy_from_slice(b"00000000000\0");
            header[148..156].fill(b' ');
            header[156] = *kind;
            if *kind == b'1' || *kind == b'2' { header[157..164].copy_from_slice(b"outside"); }
            header[257..263].copy_from_slice(b"ustar\0");
            header[263..265].copy_from_slice(b"00");
            let sum: u32 = header.iter().map(|byte| *byte as u32).sum();
            header[148..156].copy_from_slice(format!("{:06o}\0 ", sum).as_bytes());
            bytes.extend(header);
            bytes.extend_from_slice(contents);
            bytes.resize(bytes.len() + (512 - contents.len() % 512) % 512, 0);
        }
        bytes.extend([0u8; 1024]);
        let target = self.0.join(name);
        if name.ends_with(".gz") || name.ends_with(".tgz") {
            let mut writer = flate2::write::GzEncoder::new(fs::File::create(&target).unwrap(), flate2::Compression::default());
            writer.write_all(&bytes).unwrap();
            writer.finish().unwrap();
        } else { fs::write(&target, bytes).unwrap(); }
        target
    }
}
impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

fn bundle(prefix: &str, bytes: &'static [u8]) -> Vec<(String, &'static [u8], u8)> {
    FILES.iter().map(|name| (format!("{prefix}{name}"), bytes, b'0')).collect()
}
fn archive_bundle(f: &Fixture, name: &str, entries: &[(String, &'static [u8], u8)]) -> PathBuf {
    f.archive(name, &entries.iter().map(|(name, bytes, kind)| (name.as_str(), *bytes, *kind)).collect::<Vec<_>>())
}
fn validate_fixture(path: &Path) -> Result<(), String> {
    for name in FILES { if fs::read(path.join(name)).map_err(|e| e.to_string())? != b"valid-fixture" { return Err("invalid ONNX fixture".into()); } }
    Ok(())
}
fn limits() -> InstallLimits { InstallLimits { max_entries: 32, max_file_bytes: 1024, max_total_bytes: 4096 } }

#[test]
fn root_nested_tar_and_gzip_install_then_resolve_without_original_archive() {
    for (extension, prefix) in [("tar", ""), ("tar.gz", "parakeet-tdt-0.6b-v2-int8/"), ("tgz", "models/parakeet-tdt-0.6b-v2-int8/")] {
        let f = Fixture::new();
        assert!(resolve_installed_model(&f.data()).is_none());
        let archive = archive_bundle(&f, &format!("model.{extension}"), &bundle(prefix, b"valid-fixture"));
        let installed = install_archive(&archive, &f.data(), limits(), validate_fixture).unwrap();
        assert!(installed.starts_with(f.data()));
        fs::remove_file(archive).unwrap();
        // Fresh resolver call models restart: no archive, repository, or loader state retained.
        let restarted = resolve_installed_model(&f.data()).expect("persistent installation survives restart");
        assert_eq!(installed, restarted);
        validate_fixture(&restarted).unwrap();
    }
}

#[test]
fn invalid_load_incomplete_empty_and_ambiguous_imports_preserve_working_install() {
    let f = Fixture::new();
    let original = archive_bundle(&f, "original.tar", &bundle("", b"valid-fixture"));
    let installed = install_archive(&original, &f.data(), limits(), validate_fixture).unwrap();
    let mut ambiguous = bundle("one/", b"valid-fixture");
    ambiguous.extend(bundle("two/", b"valid-fixture"));
    let cases = [bundle("", b"bad-onnx"), bundle("", b""), bundle("", b"valid-fixture")[..2].to_vec(), ambiguous];
    for (index, entries) in cases.iter().enumerate() {
        let archive = archive_bundle(&f, &format!("invalid-{index}.tar"), entries);
        assert!(install_archive(&archive, &f.data(), limits(), validate_fixture).is_err());
        assert_eq!(resolve_installed_model(&f.data()), Some(installed.clone()));
        validate_fixture(&installed).unwrap();
    }
}

#[test]
fn traversal_windows_paths_ads_and_special_entries_fail_closed_before_loader() {
    for name in ["../escaped", "/absolute", "C:/drive", "C:relative", "//server/share", "folder\\escape", "vocab.txt:stream", "CON", "folder/../escape"] {
        let f = Fixture::new();
        let mut entries = bundle("", b"valid-fixture");
        entries.push((name.into(), b"evil", b'0'));
        let archive = archive_bundle(&f, "hostile.tar", &entries);
        assert!(install_archive(&archive, &f.data(), limits(), |_| -> Result<(), String> { panic!("unsafe archive reached loader") }).is_err(), "must reject {name}");
        assert!(resolve_installed_model(&f.data()).is_none());
        assert!(!f.0.join("escaped").exists());
    }
    for kind in [b'1', b'2', b'3', b'4', b'6'] {
        let f = Fixture::new();
        let mut entries = bundle("", b"valid-fixture");
        entries.push(("special".into(), b"", kind));
        let archive = archive_bundle(&f, "special.tar", &entries);
        assert!(install_archive(&archive, &f.data(), limits(), |_| -> Result<(), String> { panic!("special entry reached loader") }).is_err());
    }
}

#[test]
fn duplicate_entries_and_resource_limits_reject_before_loader() {
    let f = Fixture::new();
    let mut duplicate = bundle("", b"valid-fixture");
    duplicate.push((FILES[0].into(), b"valid-fixture", b'0'));
    let archive = archive_bundle(&f, "duplicate.tar", &duplicate);
    assert!(install_archive(&archive, &f.data(), limits(), |_| -> Result<(), String> { panic!("duplicate reached loader") }).is_err());
    let archive = archive_bundle(&f, "bounds.tgz", &bundle("", b"valid-fixture"));
    for bound in [InstallLimits { max_entries: 2, ..limits() }, InstallLimits { max_file_bytes: 2, ..limits() }, InstallLimits { max_total_bytes: 20, ..limits() }] {
        assert!(install_archive(&archive, &f.data(), bound, |_| -> Result<(), String> { panic!("over-limit archive reached loader") }).is_err());
        assert!(resolve_installed_model(&f.data()).is_none());
    }
}

#[test]
fn successful_replacement_and_broken_installed_layout_are_detected() {
    let f = Fixture::new();
    let archive = archive_bundle(&f, "first.tar", &bundle("", b"valid-fixture"));
    install_archive(&archive, &f.data(), limits(), validate_fixture).unwrap();
    let archive = archive_bundle(&f, "replacement.tar", &bundle("", b"replacement"));
    let installed = install_archive(&archive, &f.data(), limits(), |_| Ok(())).unwrap();
    assert_eq!(fs::read(installed.join(FILES[0])).unwrap(), b"replacement");
    fs::remove_file(installed.join(FILES[2])).unwrap();
    assert!(resolve_installed_model(&f.data()).is_none());
}

#[test]
fn corrupt_archives_and_unsupported_extensions_never_replace_installed_model() {
    let f = Fixture::new();
    let archive = archive_bundle(&f, "original.tar", &bundle("", b"valid-fixture"));
    let installed = install_archive(&archive, &f.data(), limits(), validate_fixture).unwrap();
    for (name, contents) in [("corrupt.tar", &b"not a tar"[..]), ("corrupt.tgz", &b"not gzip"[..]), ("truncated.tar.gz", &b"\x1f\x8b\x08\x00"[..])] {
        let path = f.0.join(name);
        fs::write(&path, contents).unwrap();
        assert!(install_archive(&path, &f.data(), limits(), |_| -> Result<(), String> { panic!("corrupt archive reached loader") }).is_err());
        validate_fixture(&installed).unwrap();
    }
    let unsupported = archive_bundle(&f, "model.zip", &bundle("", b"valid-fixture"));
    assert!(install_archive(&unsupported, &f.data(), limits(), |_| -> Result<(), String> { panic!("unsupported archive reached loader") }).is_err());
    assert_eq!(resolve_installed_model(&f.data()), Some(installed));
}

#[test]
fn validation_observes_staging_before_publication_and_failure_cleans_staging() {
    let f = Fixture::new();
    let archive = archive_bundle(&f, "original.tar", &bundle("", b"valid-fixture"));
    let previous = install_archive(&archive, &f.data(), limits(), validate_fixture).unwrap();
    let replacement = archive_bundle(&f, "replacement.tgz", &bundle("model/", b"replacement"));
    let result = install_archive(&replacement, &f.data(), limits(), |staging| {
        assert!(staging.file_name().unwrap().to_str().unwrap().starts_with("staging-"));
        assert_eq!(resolve_installed_model(&f.data()), Some(previous.clone()));
        assert_eq!(fs::read(staging.join(FILES[0])).unwrap(), b"replacement");
        Err("loader refused model".into())
    });
    assert!(result.is_err());
    assert_eq!(resolve_installed_model(&f.data()), Some(previous));
    let entries = fs::read_dir(f.data().join("speech-models")).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(entries.len(), 1, "failed staging removed; only original publication remains");
}

#[test]
fn actual_onnx_loader_rejects_nonempty_invalid_models_before_publication() {
    let f = Fixture::new();
    let archive = archive_bundle(&f, "invalid-onnx.tar", &bundle("", b"not-an-onnx-model"));
    let result = install_archive(&archive, &f.data(), limits(), |path| {
        speech_model::load_parakeet_tdt(path).map(|_| ()).map_err(|error| error.to_string())
    });
    assert!(result.is_err(), "real loader must reject invalid ONNX bytes");
    assert!(resolve_installed_model(&f.data()).is_none());
    assert_eq!(fs::read_dir(f.data().join("speech-models")).unwrap().count(), 0);
}
