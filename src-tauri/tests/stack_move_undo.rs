#![cfg(windows)]

#[path = "../src/stack_popup/local_drop.rs"]
#[allow(dead_code)]
mod local_drop;
#[path = "../src/stack_popup/models.rs"]
#[allow(dead_code)]
mod models;

// Unrelated model dependencies; no Tauri window, clipboard, or shell is started.
mod terminal {
    #[derive(Default)]
    pub struct StackTerminalRegistry;
}
mod clipboard {
    pub struct NativeCutPublication;
}
mod file_ops {
    pub fn now_ms() -> u64 {
        unreachable!("move undo does not report clipboard progress")
    }
}

use local_drop::{stage_deleted_items, undo_moves, Receipt, Sources, UndoItem, UndoState};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        loop {
            let root = std::env::temp_dir().join(format!(
                "jasonshell-undo-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&root) {
                Ok(()) => return Self(root),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("isolated move directory: {err}"),
            }
        }
    }

    fn move_file(&self) -> (PathBuf, PathBuf, Vec<UndoItem>) {
        self.move_file_from(&self.0)
    }

    fn move_file_from(&self, source_dir: &std::path::Path) -> (PathBuf, PathBuf, Vec<UndoItem>) {
        let origin = source_dir.join("Résumé.txt");
        let dest = source_dir.join("Dest");
        let target = dest.join(origin.file_name().unwrap());
        fs::create_dir(&dest).unwrap();
        fs::write(&origin, b"original bytes").unwrap();
        let sources = Sources::snapshot(std::slice::from_ref(&origin)).expect("source snapshot");
        let (result, items) = Receipt::new("prepare-id".into(), 7, sources)
            .commit(&dest)
            .expect("native local drop");
        assert_eq!(result.moved, 1, "{:?}", result.failures);
        assert!(result.failures.is_empty());
        assert!(!origin.exists());
        assert_eq!(fs::read(&target).unwrap(), b"original bytes");
        (origin, target, items)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn native_undo_restores_original_name_and_contents_once_for_receipt_owner() {
    let fixture = Fixture::new();
    let (origin, target, items) = fixture.move_file();
    let mut state = UndoState::default();
    let permit = state.reserve().unwrap();
    let id = state
        .replace(&permit, Some(7), Some(("undo-id".into(), items)))
        .expect("opaque undo receipt");
    drop(permit);

    let _permit = state.reserve().unwrap();
    assert!(state.take(&id, 8).is_err());
    assert!(state.take("wrong-receipt", 7).is_err());
    assert!(!origin.exists());
    let result = undo_moves(state.take(&id, 7).expect("owner receipt still valid"));
    assert_eq!(result.moved, 1, "{:?}", result.failures);
    assert!(result.failures.is_empty());
    assert_eq!(fs::read(&origin).unwrap(), b"original bytes");
    assert!(!target.exists());
    assert!(state.take(&id, 7).is_err());
}

#[test]
fn native_undo_preserves_both_files_when_original_name_is_occupied() {
    let fixture = Fixture::new();
    let (origin, target, items) = fixture.move_file();
    fs::write(&origin, b"new occupant").unwrap();

    let result = undo_moves(items);

    assert_eq!(result.moved, 0);
    assert_eq!(result.failures.len(), 1);
    assert_eq!(fs::read(&origin).unwrap(), b"new occupant");
    assert_eq!(fs::read(&target).unwrap(), b"original bytes");
}

#[test]
fn native_undo_rejects_replaced_destination_directory() {
    let fixture = Fixture::new();
    let (origin, target, items) = fixture.move_file();
    let destination = target.parent().unwrap();
    let parked = fixture.0.join("Parked");
    fs::rename(destination, &parked).unwrap();
    fs::create_dir(destination).unwrap();
    fs::write(&target, b"replacement bytes").unwrap();

    let result = undo_moves(items);

    assert_eq!(result.moved, 0);
    assert_eq!(result.failures.len(), 1);
    assert!(!origin.exists());
    assert_eq!(fs::read(&target).unwrap(), b"replacement bytes");
    assert_eq!(
        fs::read(parked.join(target.file_name().unwrap())).unwrap(),
        b"original bytes"
    );
}

#[test]
fn native_local_drop_undo_restores_exact_bytes_beyond_max_path() {
    use std::os::windows::ffi::OsStrExt;

    let fixture = Fixture::new();
    let mut source = fixture.0.clone();
    while source.as_os_str().encode_wide().count() <= 260 {
        source.push("long-stack-folder-".repeat(4));
    }
    fs::create_dir_all(&source).unwrap();
    let (origin, target, items) = fixture.move_file_from(&source);

    let result = undo_moves(items);

    assert_eq!(result.moved, 1, "{:?}", result.failures);
    assert!(result.failures.is_empty());
    assert_eq!(fs::read(&origin).unwrap(), b"original bytes");
    assert!(!target.exists());
}

#[test]
fn staged_delete_preserves_payload_and_manifest_and_undo_never_replaces() {
    let fixture = Fixture::new();
    let staging = fixture.0.join("stack-deleted");
    fs::create_dir(&staging).unwrap();
    let restored = fixture.0.join("Résumé.txt");
    let collided = fixture.0.join("Occupied.txt");
    let paths = [restored.clone(), collided.clone()];
    fs::write(&restored, b"restore bytes").unwrap();
    fs::write(&collided, b"staged bytes").unwrap();

    let (result, items) = stage_deleted_items(&paths, &staging).unwrap();
    assert_eq!(result.deleted, 2, "{:?}", result.failures);
    assert!(result.failures.is_empty());
    let batch = fs::read_dir(&staging).unwrap().next().unwrap().unwrap().path();
    let payloads: Vec<_> = paths.iter().enumerate().map(|(index, path)| {
        assert!(!path.exists());
        let slot = batch.join(index.to_string());
        let manifest: serde_json::Value = serde_json::from_slice(&fs::read(slot.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["originalPath"].as_str(), path.to_str());
        slot.join("payload").join(path.file_name().unwrap())
    }).collect();
    assert_eq!(fs::read(&payloads[0]).unwrap(), b"restore bytes");
    assert_eq!(fs::read(&payloads[1]).unwrap(), b"staged bytes");
    let (denied, denied_items) = stage_deleted_items(&[fixture.0.clone(), staging.clone(), payloads[1].clone()], &staging).unwrap();
    assert_eq!(denied.deleted, 0);
    assert_eq!(denied.failures.len(), 3);
    assert!(denied_items.is_empty());

    fs::write(&collided, b"occupant bytes").unwrap();
    let mut state = UndoState::default();
    let permit = state.reserve().unwrap();
    let id = state.replace(&permit, Some(7), Some(("delete-id".into(), items))).unwrap();
    drop(permit);
    let _permit = state.reserve().unwrap();
    let result = undo_moves(state.take(&id, 7).unwrap());
    assert_eq!(result.moved, 1, "{:?}", result.failures);
    assert_eq!(result.failures.len(), 1);
    assert_eq!(fs::read(&restored).unwrap(), b"restore bytes");
    assert!(!payloads[0].exists());
    assert_eq!(fs::read(&collided).unwrap(), b"occupant bytes");
    assert_eq!(fs::read(&payloads[1]).unwrap(), b"staged bytes");
    assert!(state.take(&id, 7).is_err());
}

#[test]
fn staged_delete_undo_refuses_replaced_ancestor_even_when_immediate_parent_matches() {
    let fixture = Fixture::new();
    let staging = fixture.0.join("stack-deleted");
    let ancestor = fixture.0.join("Ancestor");
    let parent = ancestor.join("Parent");
    let original = parent.join("File.txt");
    fs::create_dir(&staging).unwrap();
    fs::create_dir_all(&parent).unwrap();
    fs::write(&original, b"retained payload").unwrap();
    let (result, items) = stage_deleted_items(std::slice::from_ref(&original), &staging).unwrap();
    assert_eq!(result.deleted, 1, "{:?}", result.failures);
    let batch = fs::read_dir(&staging).unwrap().next().unwrap().unwrap().path();

    let parked = fixture.0.join("Parked");
    fs::rename(&ancestor, &parked).unwrap();
    fs::create_dir(&ancestor).unwrap();
    fs::rename(parked.join("Parent"), &parent).unwrap();
    let result = undo_moves(items);

    assert_eq!(result.moved, 0);
    assert_eq!(result.failures.len(), 1);
    assert!(!original.exists());
    assert_eq!(fs::read(batch.join("0/payload/File.txt")).unwrap(), b"retained payload");
}
