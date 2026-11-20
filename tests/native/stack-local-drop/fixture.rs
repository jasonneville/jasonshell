#![cfg(all(test, windows))]

// Only the command result DTOs are substituted; all filesystem operations come
// from the production module, without building or starting the shell.
mod models {
    pub struct StackLocalDropResult {
        pub moved: usize,
        pub failures: Vec<StackPasteFailure>,
    }
    pub struct StackPasteFailure {
        pub path: String,
        pub message: String,
    }
}

mod local_drop {
    include!("../../../src-tauri/src/stack_popup/local_drop.rs");

    #[test]
    fn validated_destination_allows_write_but_blocks_rename_and_binds_leaf() {
        let sandbox = super::Sandbox::new();
        let dest = sandbox.folder("Dest");
        let moved = sandbox.0.join("Moved");
        let write_open = || {
            OpenOptions::new()
                .write(true)
                .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE).0)
                .custom_flags((FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT).0)
                .open(&dest)
        };
        drop(write_open().expect("fixture directory supports write open when unpinned"));
        let (pin, _, is_dir) = open_destination(&dest).expect("validate and pin actual destination");
        assert!(is_dir);
        drop(write_open().expect("destination needs compatible write sharing"));
        assert_eq!(std::fs::rename(&dest, &moved).unwrap_err().raw_os_error(), Some(32));
        let source = sandbox.file("source.txt", b"source");
        let (file, _, _) = open_entry(&source, true, true).expect("open verified source");
        assert!(rename_without_replacement(&file, &pin, OsStr::new("..\\escaped.txt")).is_err());
        rename_without_replacement(&file, &pin, OsStr::new("inside.txt")).expect("rename into verified handle");
        drop(file);
        super::missing(&source);
        super::missing(&sandbox.0.join("escaped.txt"));
        assert_eq!(std::fs::read(dest.join("inside.txt")).unwrap(), b"source");
        drop(pin);
        std::fs::rename(&dest, &moved).expect("rename works after pin release");
    }
}

use local_drop::{Receipt, Sources};
use std::{fs, path::{Path, PathBuf}, process::Command, sync::atomic::{AtomicUsize, Ordering}};

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!("stack-local-drop-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&root).expect("create isolated temporary directory");
        Self(root)
    }

    fn file(&self, name: &str, data: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, data).expect("write fixture file");
        path
    }

    fn folder(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).expect("create fixture folder");
        path
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("Fixture cleanup failed for {:?}: {error}", self.0);
        }
    }
}

fn receipt(paths: &[PathBuf]) -> Receipt {
    let receipt = Receipt::new("fixture".into(), 1, Sources::snapshot(paths).expect("snapshot real sources"));
    assert!(receipt.matches("fixture", 1));
    receipt
}

fn missing(path: &Path) {
    assert!(!path.try_exists().expect("inspect fixture path"), "unexpected path: {path:?}");
}

#[test]
fn prepared_selection_preserves_trailing_nonbreaking_space() {
    let sandbox = Sandbox::new();
    let dest = sandbox.folder("Dest");
    let ordinary = sandbox.file("report.txt", b"ordinary");
    let selected_name = "report.txt\u{00a0}";
    let selected = sandbox.file(selected_name, b"selected");
    assert_eq!(fs::read(&ordinary).unwrap(), b"ordinary");
    assert_eq!(fs::read(&selected).unwrap(), b"selected");
    let listed = selected.to_str().expect("Unicode fixture path").to_owned();
    let sources = Sources::snapshot_listed_paths(&[listed]).expect("snapshot exact listed source");
    let result = Receipt::new("fixture-nbsp".into(), 1, sources)
        .commit(&dest)
        .expect("move exact listed source");
    assert_eq!(result.moved, 1);
    assert!(result.failures.is_empty());
    missing(&selected);
    assert_eq!(fs::read(ordinary).unwrap(), b"ordinary");
    assert_eq!(fs::read(dest.join(selected_name)).unwrap(), b"selected");
    missing(&dest.join("report.txt"));
}

#[test]
fn moves_both_selected_files_once() {
    let sandbox = Sandbox::new();
    let dest = sandbox.folder("Dest");
    let a = sandbox.file("alpha.txt", b"alpha");
    let b = sandbox.file("beta space.txt", b"beta");
    let result = receipt(&[a.clone(), b.clone()]).commit(&dest).expect("move selected files");
    assert_eq!(result.moved, 2, "move failures: {:?}", result.failures.iter().map(|failure| &failure.message).collect::<Vec<_>>());
    assert!(result.failures.is_empty());
    missing(&a);
    missing(&b);
    assert_eq!(fs::read(dest.join("alpha.txt")).unwrap(), b"alpha");
    assert_eq!(fs::read(dest.join("beta space.txt")).unwrap(), b"beta");
    assert_eq!(fs::read_dir(dest).unwrap().count(), 2);
}

#[test]
fn conflict_preserves_existing_destination_and_source() {
    let sandbox = Sandbox::new();
    let dest = sandbox.folder("Dest");
    let source = sandbox.file("same.txt", b"source");
    let existing = sandbox.file("Dest/same.txt", b"existing");
    let result = receipt(std::slice::from_ref(&source)).commit(&dest).expect("file-specific conflict");
    assert_eq!(result.moved, 0);
    assert_eq!(result.failures.len(), 1);
    assert_eq!(Path::new(&result.failures[0].path), source);
    assert!(!result.failures[0].message.is_empty());
    assert_eq!(fs::read(source).unwrap(), b"source");
    assert_eq!(fs::read(existing).unwrap(), b"existing");
}

#[test]
fn destination_replaced_by_junction_before_commit_cannot_redirect_move() {
    let sandbox = Sandbox::new();
    let outside = sandbox.folder("Outside");
    let dest = sandbox.folder("Dest");
    let source = sandbox.file("stay.txt", b"source");
    let receipt = receipt(std::slice::from_ref(&source));
    fs::remove_dir(&dest).expect("remove original destination");
    let junction = Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J", "Dest", "Outside"])
        .current_dir(&sandbox.0)
        .output()
        .expect("run Windows junction setup");
    assert!(junction.status.success(), "Junction setup unavailable (test not verified): {}", String::from_utf8_lossy(&junction.stderr));
    assert!(receipt.commit(&dest).is_err(), "junction must be rejected");
    missing(&outside.join("stay.txt"));
    assert_eq!(fs::read(source).unwrap(), b"source");
}
