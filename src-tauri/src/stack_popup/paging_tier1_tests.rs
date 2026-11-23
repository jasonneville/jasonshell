//! Tier 1 RS-20 / RS-21: folder listing must not open a handle per plain file and must
//! not allocate lowercase strings per comparison; ordering/semantics stay identical.
//!
//! Wiring (end of `stack_popup/paging.rs`, next to the existing `mod tests`):
//! ```ignore
//! #[cfg(test)]
//! #[path = "paging_tier1_tests.rs"]
//! mod tier1_tests;
//! ```
//!
//! Required seams (this file is a child module of `paging`):
//!
//! paging.rs
//! ```ignore
//! /// Sort key computed ONCE per entry. Must equal `name.to_lowercase()` (same order as before).
//! pub(crate) fn sort_key_for(name: &str) -> Box<str>;
//!
//! /// `StackFolderEntrySummary` gains `sort_key: Box<str>` (filled from `sort_key_for`)
//! /// and a private `metadata: Option<fs::Metadata>` carried for non-reparse rows.
//! /// `compare_stack_entries` / `compare_newest_first` compare `sort_key`, never call
//! /// `to_lowercase`. Everything else (fields name/is_dir/modified_at/item) unchanged.
//!
//! /// Pure per-entry summary. `entry_metadata` = `DirEntry::metadata()` result (free on
//! /// Windows). `resolve_target` is the ONLY place a per-path handle may be opened
//! /// (`|| fs::metadata(&path).ok()` in production) and MUST be called iff
//! /// `file_type.is_symlink()` || reparse-point attribute set on `entry_metadata`
//! /// (or `entry_metadata` is Err, as the single fallback).
//! /// Symlink/junction/reparse: is_dir = target.is_dir() (false if None), modified_at =
//! /// target mtime (None if None)  == today's behaviour. Otherwise is_dir =
//! /// file_type.is_dir(), modified_at = entry_metadata mtime.
//! fn summarize_listing_entry(
//!     name: String,
//!     path: PathBuf,
//!     file_type: fs::FileType,
//!     entry_metadata: std::io::Result<fs::Metadata>,
//!     resolve_target: impl FnOnce() -> Option<fs::Metadata>,
//! ) -> StackFolderEntrySummary;
//! ```
//! `stack_folder_entry_summary(entry)` becomes a thin wrapper over it.
//!
//! items.rs (page build; used for non-reparse Filesystem rows only, symlink/reparse rows
//! keep calling `stack_item_metadata_from_path`):
//! ```ignore
//! /// Same result as `stack_item_metadata_from_path(path)` when `link_metadata` is
//! /// fresh and the entry is neither symlink nor reparse point; performs no fs call.
//! pub(crate) fn stack_item_metadata_from_known_metadata(
//!     path: PathBuf,
//!     link_metadata: &fs::Metadata,
//! ) -> StackItem;
//! ```
//! stack_popup.rs `suggest_stack_paths` must sort with a once-computed key
//! (`sort_by_cached_key(|s| paging::sort_key_for(&s.name))` or equivalent, stable) and
//! `finalize_zip_children` likewise.

use super::{
    collect_stack_folder_entries, finalize_zip_children, sort_key_for, summarize_listing_entry,
    virtual_zip_stack_item, StackFolderEntryItem, StackFolderEntrySummary,
};
use crate::stack_popup::items::{
    stack_item_metadata_from_known_metadata, stack_item_metadata_from_path,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "jasonshell-tier1-paging-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn make_junction(link: &Path, target: &Path) -> bool {
    std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn make_dir_symlink(link: &Path, target: &Path) -> bool {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

fn make_file_symlink(link: &Path, target: &Path) -> bool {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

fn mtime_ms(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
}

/// Oracle: exactly the pre-change summary (fs::metadata per path).
fn old_summary(entry: &fs::DirEntry) -> (String, bool, Option<u64>) {
    let path = entry.path();
    let file_type = entry.file_type().unwrap();
    let is_dir = if file_type.is_dir() {
        true
    } else if file_type.is_symlink() {
        fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false)
    } else {
        false
    };
    let name = entry.file_name().to_string_lossy().into_owned();
    (name, is_dir, fs::metadata(&path).ok().and_then(|m| mtime_ms(&m)))
}

/// Oracle: the pre-change comparator (allocating lowercase per comparison).
fn old_cmp(
    a: &(String, bool, Option<u64>),
    b: &(String, bool, Option<u64>),
    newest_first: bool,
) -> std::cmp::Ordering {
    if newest_first {
        match (a.2, b.2) {
            (Some(x), Some(y)) => y.cmp(&x).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())),
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, None) => a.0.to_lowercase().cmp(&b.0.to_lowercase()),
        }
    } else {
        let rank = |d: bool| if d { 0u8 } else { 1u8 };
        rank(a.1)
            .cmp(&rank(b.1))
            .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
    }
}

const UNICODE_NAMES: &[&str] = &[
    "apple.txt", "Banana.txt", "cherry.txt", "Zebra.txt", "zulu.txt", "Éclair.txt", "ärger.txt",
    "Ünder.txt", "éa.txt", "Ωmega.txt", "ωave.txt", "ǆ.txt", "Straße.txt", "strasse.txt",
    "_under.txt", "10.txt", "2.txt", "日本.txt", "İstanbul.txt", "ıstanbul.txt",
];

#[test]
fn sort_key_is_the_lowercased_name() {
    for name in UNICODE_NAMES.iter().chain(&["ÉCLAIR", "", "A b C"]) {
        assert_eq!(&*sort_key_for(name), name.to_lowercase().as_str(), "{name}");
    }
}

#[test]
fn plain_files_and_dirs_never_use_the_per_path_resolver() {
    let root = temp_root("plain");
    fs::write(root.join("a.txt"), b"hello").unwrap();
    fs::write(root.join("noext"), b"").unwrap();
    fs::create_dir(root.join("sub")).unwrap();

    let mut seen = 0;
    for entry in fs::read_dir(&root).unwrap() {
        let entry = entry.unwrap();
        let calls = Cell::new(0u32);
        let old = old_summary(&entry);
        let summary = summarize_listing_entry(
            entry.file_name().to_string_lossy().into_owned(),
            entry.path(),
            entry.file_type().unwrap(),
            entry.metadata(),
            || {
                calls.set(calls.get() + 1);
                fs::metadata(entry.path()).ok()
            },
        );
        assert_eq!(calls.get(), 0, "{} must not open a handle", old.0);
        assert_eq!((summary.name.clone(), summary.is_dir, summary.modified_at), old);
        assert_eq!(&*summary.sort_key, old.0.to_lowercase().as_str());
        assert!(matches!(&summary.item, StackFolderEntryItem::Filesystem(p) if *p == entry.path()));
        seen += 1;
    }
    assert_eq!(seen, 3);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn junction_and_symlink_entries_keep_resolved_behaviour_via_single_resolver_call() {
    let root = temp_root("links");
    let target_dir = root.join("target-dir");
    let target_file = root.join("target-file.txt");
    fs::create_dir(&target_dir).unwrap();
    fs::write(&target_file, b"payload").unwrap();

    let mut expected_links = Vec::new();
    if make_junction(&root.join("junction"), &target_dir) {
        expected_links.push("junction");
    }
    if make_dir_symlink(&root.join("dirlink"), &target_dir) {
        expected_links.push("dirlink");
    }
    if make_file_symlink(&root.join("filelink"), &target_file) {
        expected_links.push("filelink");
    }
    // broken links (target removed after creation)
    let doomed = root.join("doomed-dir");
    fs::create_dir(&doomed).unwrap();
    if make_junction(&root.join("broken-junction"), &doomed) {
        expected_links.push("broken-junction");
    }
    let doomed_file = root.join("doomed.txt");
    fs::write(&doomed_file, b"x").unwrap();
    if make_file_symlink(&root.join("broken-filelink"), &doomed_file) {
        expected_links.push("broken-filelink");
    }
    fs::remove_dir(&doomed).unwrap_or(());
    fs::remove_file(&doomed_file).unwrap_or(());

    if expected_links.is_empty() {
        eprintln!("SKIP junction/symlink assertions: cannot create links in this environment");
        let _ = fs::remove_dir_all(&root);
        return;
    }

    let mut resolved = Vec::new();
    for entry in fs::read_dir(&root).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        let calls = Cell::new(0u32);
        let old = old_summary(&entry);
        let summary = summarize_listing_entry(
            name.clone(),
            entry.path(),
            entry.file_type().unwrap(),
            entry.metadata(),
            || {
                calls.set(calls.get() + 1);
                fs::metadata(entry.path()).ok()
            },
        );
        assert_eq!(
            (summary.name.clone(), summary.is_dir, summary.modified_at),
            old,
            "{name}: kind/mtime must equal today's resolved behaviour"
        );
        if expected_links.contains(&name.as_str()) {
            assert_eq!(calls.get(), 1, "{name}: link resolved exactly once");
            resolved.push(name);
        } else {
            assert_eq!(calls.get(), 0, "{name}: plain entry must not resolve");
        }
    }
    resolved.sort();
    let mut want: Vec<String> = expected_links.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(resolved, want);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn entry_metadata_error_falls_back_to_resolver_once() {
    let root = temp_root("md-err");
    let file = root.join("f.txt");
    fs::write(&file, b"x").unwrap();
    let entry = fs::read_dir(&root).unwrap().next().unwrap().unwrap();
    let calls = Cell::new(0u32);
    let summary = summarize_listing_entry(
        "f.txt".into(),
        file.clone(),
        entry.file_type().unwrap(),
        Err(std::io::Error::other("boom")),
        || {
            calls.set(calls.get() + 1);
            fs::metadata(&file).ok()
        },
    );
    assert_eq!(calls.get(), 1);
    assert!(!summary.is_dir);
    assert_eq!(summary.modified_at, mtime_ms(&fs::metadata(&file).unwrap()));
    let _ = fs::remove_dir_all(&root);
}

fn names(entries: &[StackFolderEntrySummary]) -> Vec<String> {
    entries.iter().map(|e| e.name.clone()).collect()
}

#[test]
fn listing_order_matches_old_comparator_name_sort_with_unicode_and_dirs_first() {
    let root = temp_root("order-name");
    let mut seen = std::collections::HashSet::new();
    for name in UNICODE_NAMES {
        // skip names that collide on the (case-insensitive) filesystem
        if !seen.insert(name.to_lowercase()) {
            continue;
        }
        if fs::write(root.join(name), b"x").is_err() {
            seen.remove(&name.to_lowercase());
        }
    }
    for dir in ["Beta-dir", "alpha-dir", "Ärztedir", "zeta-dir"] {
        fs::create_dir(root.join(dir)).unwrap();
    }

    let (entries, warnings) = collect_stack_folder_entries(&root.to_string_lossy(), false).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");

    let mut oracle: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .map(|e| old_summary(&e.unwrap()))
        .collect();
    oracle.sort_by(|a, b| old_cmp(a, b, false)); // stable, same as old sort_by
    assert_eq!(
        names(&entries),
        oracle.iter().map(|o| o.0.clone()).collect::<Vec<_>>()
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn listing_order_matches_old_comparator_newest_first_with_ties() {
    let root = temp_root("order-newest");
    let base = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let plan: &[(&str, u64)] = &[
        ("old.txt", 0),
        ("tie-b.txt", 500),
        ("Tie-A.txt", 500),
        ("tie-é.txt", 500),
        ("newest.txt", 900),
        ("Ärger.txt", 100),
        ("zzz.txt", 100),
    ];
    for (name, offset) in plan {
        let path = root.join(name);
        fs::write(&path, b"x").unwrap();
        let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_modified(base + Duration::from_secs(*offset)).unwrap();
    }
    let (entries, _) = collect_stack_folder_entries(&root.to_string_lossy(), true).unwrap();
    let mut oracle: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .map(|e| old_summary(&e.unwrap()))
        .collect();
    oracle.sort_by(|a, b| old_cmp(a, b, true));
    assert_eq!(
        names(&entries),
        oracle.iter().map(|o| o.0.clone()).collect::<Vec<_>>()
    );
    assert_eq!(entries[0].name, "newest.txt");
    // tie group (same mtime) ordered by lowercase name
    assert_eq!(
        names(&entries)[1..4],
        ["Tie-A.txt".to_string(), "tie-b.txt".to_string(), "tie-é.txt".to_string()]
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn zip_children_finalize_order_matches_old_comparator() {
    let archive = Path::new("C:\\fake\\a.zip");
    let mut by_name = HashMap::new();
    let names_in: Vec<(String, bool)> = UNICODE_NAMES
        .iter()
        .map(|n| (n.to_string(), false))
        .chain(["Beta-dir", "alpha-dir", "Ärztedir"].iter().map(|n| (n.to_string(), true)))
        .collect();
    let mut lowered = std::collections::HashSet::new();
    let mut kept = Vec::new();
    for (name, is_dir) in names_in {
        if !lowered.insert(name.to_lowercase()) {
            continue; // avoid ties: HashMap iteration order is not deterministic
        }
        by_name.insert(name.clone(), virtual_zip_stack_item(archive, "", &name, is_dir, 1));
        kept.push((name, is_dir, None::<u64>));
    }
    let mut warnings = Vec::new();
    let got = finalize_zip_children(by_name, archive, false, &mut warnings);
    kept.sort_by(|a, b| old_cmp(a, b, false));
    assert_eq!(names(&got), kept.iter().map(|k| k.0.clone()).collect::<Vec<_>>());
}

#[test]
fn suggest_stack_paths_order_matches_old_comparator_and_lists_only_dirs() {
    let root = temp_root("suggest");
    let dirs = ["Beta", "alpha", "Ärzte", "ärger", "Zulu", "éclair", "Ünder", "ωave", "Ωmega"];
    for d in dirs {
        fs::create_dir(root.join(d)).unwrap();
    }
    fs::write(root.join("a-file.txt"), b"x").unwrap();
    let got = crate::stack_popup::suggest_stack_paths(
        root.to_string_lossy().into_owned(),
        String::new(),
        Some(50),
    )
    .unwrap();
    let mut oracle: Vec<String> = fs::read_dir(&root)
        .unwrap()
        .filter_map(|e| {
            let e = e.unwrap();
            e.path().is_dir().then(|| e.file_name().to_string_lossy().into_owned())
        })
        .collect();
    oracle.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    assert_eq!(got.iter().map(|s| s.name.clone()).collect::<Vec<_>>(), oracle);
    assert_eq!(got.len(), dirs.len());

    // prefix filter still case-insensitive incl. Unicode
    let filtered = crate::stack_popup::suggest_stack_paths(
        root.to_string_lossy().into_owned(),
        "Ä".to_string(),
        Some(50),
    )
    .unwrap();
    let mut want: Vec<String> = dirs
        .iter()
        .filter(|d| d.to_lowercase().starts_with(&"Ä".to_lowercase()))
        .map(|d| d.to_string())
        .collect();
    want.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
    assert_eq!(filtered.iter().map(|s| s.name.clone()).collect::<Vec<_>>(), want);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn page_item_from_carried_metadata_equals_stat_based_item_for_plain_rows() {
    let root = temp_root("page-item");
    let file = root.join("Report.PDF");
    fs::write(&file, vec![0u8; 1234]).unwrap();
    let hidden = root.join(".hidden");
    fs::write(&hidden, b"h").unwrap();
    let sub = root.join("Sub");
    fs::create_dir(&sub).unwrap();
    let ro = root.join("ro.txt");
    fs::write(&ro, b"r").unwrap();
    let mut perms = fs::metadata(&ro).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&ro, perms).unwrap();

    for path in [&file, &hidden, &sub, &ro] {
        let carried = fs::symlink_metadata(path).unwrap();
        let from_known = stack_item_metadata_from_known_metadata(path.clone(), &carried);
        let from_stat = stack_item_metadata_from_path(path.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&from_known).unwrap(),
            serde_json::to_value(&from_stat).unwrap(),
            "{}",
            path.display()
        );
    }
    let mut perms = fs::metadata(&ro).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    let _ = fs::set_permissions(&ro, perms);
    let _ = fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------- review follow-ups

fn plain_summary(path: &Path) -> StackFolderEntrySummary {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let entry = fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap())
        .find(|e| e.file_name().to_string_lossy() == name)
        .unwrap();
    summarize_listing_entry(
        name,
        path.to_path_buf(),
        entry.file_type().unwrap(),
        entry.metadata(),
        || None,
    )
}

#[test]
fn session_byte_estimate_counts_sort_key_and_carried_metadata() {
    use super::stack_folder_entry_summary_estimated_bytes as est;
    let root = temp_root("estimate");
    let file = root.join("Estimate.txt");
    fs::write(&file, b"x").unwrap();
    let with_md = plain_summary(&file);
    assert!(with_md.metadata.is_some(), "plain rows carry metadata");
    let mut without_md = with_md.clone();
    without_md.metadata = None;
    assert_eq!(
        est(&with_md),
        est(&without_md) + std::mem::size_of::<fs::Metadata>()
    );
    let mut long_key = without_md.clone();
    long_key.sort_key = "k".repeat(1000).into_boxed_str();
    assert_eq!(
        est(&long_key) - est(&without_md),
        1000 - without_md.sort_key.len()
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn drop_carried_metadata_clears_session_rows_only_for_that_session() {
    use super::StackFolderListingSessionStore;
    use std::time::Instant;
    let root = temp_root("drop-md");
    let a = root.join("a.txt");
    let b = root.join("b.txt");
    fs::write(&a, b"1").unwrap();
    fs::write(&b, b"22").unwrap();
    let entries = vec![plain_summary(&a), plain_summary(&b)];
    let mut store = StackFolderListingSessionStore::default();
    let path = root.to_string_lossy().into_owned();
    let id = store.start_session(&path, entries, 2, Instant::now());
    let before = store.session_page(&path, &id, 0, 10).unwrap();
    assert!(before.0.iter().all(|e| e.metadata.is_some()));
    store.drop_carried_metadata(&id);
    let after = store.session_page(&path, &id, 0, 10).unwrap();
    assert_eq!(after.0.len(), 2);
    assert!(after.0.iter().all(|e| e.metadata.is_none()));
    store.drop_carried_metadata("no-such-session"); // must not panic
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn continuation_page_shows_fresh_size_after_file_changes_between_pages() {
    use super::read_stack_folder_page_with_session;
    let root = temp_root("fresh-continuation");
    for n in ["a.txt", "b.txt", "c.txt"] {
        fs::write(root.join(n), b"old").unwrap();
    }
    let path = root.to_string_lossy().into_owned();
    let first = read_stack_folder_page_with_session(&path, None, 0, 1).unwrap();
    assert_eq!(first.items[0].name, "a.txt");
    let session = first.session_id.clone().expect("more pages => session");
    fs::write(root.join("b.txt"), vec![b'x'; 4096]).unwrap();
    let second = read_stack_folder_page_with_session(&path, Some(&session), 1, 1).unwrap();
    assert_eq!(second.items[0].name, "b.txt");
    assert_eq!(second.items[0].size_bytes, Some(4096), "must not show stale size");
    let want = mtime_ms(&fs::metadata(root.join("b.txt")).unwrap());
    assert_eq!(second.items[0].modified_at, want, "must not show stale mtime");
    let _ = fs::remove_dir_all(&root);
}
