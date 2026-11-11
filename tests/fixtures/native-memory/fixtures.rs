#![allow(dead_code, unused_imports)]
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    collections::{HashMap, HashSet, VecDeque},
    fs,
    hint::black_box,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, SyncSender},
        Mutex,
    },
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Default, Debug)]
struct Counts {
    allocs: usize,
    bytes: usize,
    reallocs: usize,
    explicit_text_clone_bytes: usize,
}

// Observer wraps only the extracted reader's explicit text.clone expression.
// It does not count decoder copies, event serialization, or OS/IPC copies.
fn counted_text_clone(text: &str) -> String {
    ON.with(|on| {
        if on.get() {
            COUNTS.with(|cell| {
                let mut count = cell.get();
                count.explicit_text_clone_bytes += text.len();
                cell.set(count);
            })
        }
    });
    text.to_owned()
}
thread_local! {static ON:Cell<bool>=const{Cell::new(false)};static COUNTS:Cell<Counts>=Cell::new(Counts::default());static PEAK:Cell<usize>=const{Cell::new(0)};static ICONS:Cell<usize>=const{Cell::new(0)};}
struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            ON.with(|on| {
                if on.get() {
                    COUNTS.with(|c| {
                        let mut v = c.get();
                        v.allocs += 1;
                        v.bytes += l.size();
                        c.set(v)
                    })
                }
            })
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = System.realloc(p, l, n);
        if !q.is_null() {
            ON.with(|on| {
                if on.get() {
                    COUNTS.with(|c| {
                        let mut v = c.get();
                        v.allocs += 1;
                        v.reallocs += 1;
                        v.bytes += n;
                        c.set(v)
                    })
                }
            })
        }
        q
    }
}
#[global_allocator]
static ALLOC: Counting = Counting;
fn measure<T>(f: impl FnOnce() -> T) -> (T, Counts, u128) {
    COUNTS.with(|c| c.set(Counts::default()));
    ON.with(|c| c.set(true));
    let t = Instant::now();
    let v = black_box(f());
    let ns = t.elapsed().as_nanos();
    ON.with(|c| c.set(false));
    (v, COUNTS.with(Cell::get), ns)
}
const APP_INDEX_TTL: Duration = Duration::from_secs(60);
fn icon_data_url_for_path(p: &Path) -> Option<String> {
    ICONS.with(|n| n.set(n.get() + 1));
    Some(format!("fixture:{}", p.display()))
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Failure {
    InvalidImage,
    Budget,
    Allocation,
    SetRejected,
}
const MAX_STAGING_BYTES: usize = 512 * 1024 * 1024;
struct CroppedImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}
fn checked_rgba_len(w: u32, h: u32, _max: usize) -> Result<usize, ()> {
    if w == 0 || h == 0 {
        return Err(());
    }
    (w as usize)
        .checked_mul(h as usize)
        .and_then(|v| v.checked_mul(4))
        .ok_or(())
}
#[derive(Default, Clone, Copy)]
struct Failpoints {
    allocation: bool,
}
static STAGED: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
fn peak() {
    let v = STAGED.load(Ordering::SeqCst);
    PEAK.with(|n| n.set(n.get().max(v)))
}
struct Staged(usize);
impl Staged {
    fn new(n: usize) -> Self {
        STAGED.fetch_add(n, Ordering::SeqCst);
        peak();
        Self(n)
    }
}
impl Drop for Staged {
    fn drop(&mut self) {
        STAGED.fetch_sub(self.0, Ordering::SeqCst);
    }
}
struct Charge(usize);
impl Charge {
    fn new(n: usize) -> Result<Self, Failure> {
        BYTES
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                v.checked_add(n).filter(|x| *x <= MAX_STAGING_BYTES)
            })
            .map_err(|_| Failure::Budget)?;
        Ok(Self(n))
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        BYTES.fetch_sub(self.0, Ordering::SeqCst);
    }
}
// Synthetic HGLOBAL: rounds to 16, retains all bytes/padding; NOT Win32 evidence.
struct Global {
    bytes: Vec<u8>,
    logical_len: usize,
    _staged: Staged,
    _charge: Charge,
}
impl Global {
    fn copy(b: &[u8], fail: bool) -> Result<Self, Failure> {
        if fail {
            return Err(Failure::Allocation);
        }
        let n = (b.len() + 15) & !15;
        let mut bytes = vec![0; n];
        bytes[..b.len()].copy_from_slice(b);
        Ok(Self {
            bytes,
            logical_len: b.len(),
            _staged: Staged::new(n),
            _charge: Charge::new(n)?,
        })
    }
}
struct Prepared {
    dib: Global,
    png: Global,
    png_format: u16,
    _charge: Charge,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum StackTerminalOutputStream {
    Stdout,
}
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StackTerminalOutputChunk {
    session_id: String,
    stream: StackTerminalOutputStream,
    text: String,
    sequence: u64,
}
struct TerminalReaderMessage {
    stream: StackTerminalOutputStream,
    text: String,
    sequence: u64,
}
#[derive(Clone, Debug)]
struct ProcessInfo {
    pid: u32,
    name: String,
    parent_pid: Option<u32>,
    parent_name: Option<String>,
    child_process_count: u32,
    descendant_process_count: u32,
}
const MIN_CAPTURE_SAMPLES: usize = 1600;
#[derive(Debug, PartialEq)]
enum SpeechFailure {
    CaptureEmpty,
    CaptureShort,
    AsrInputInvalid,
}
#[derive(Clone)]
struct AppHandle {
    emitted: std::sync::Arc<Mutex<Vec<StackTerminalOutputChunk>>>,
}
impl AppHandle {
    fn emit_to(
        &self,
        _target: &str,
        _event: &str,
        chunk: &StackTerminalOutputChunk,
    ) -> Result<(), ()> {
        self.emitted.lock().unwrap().push(chunk.clone());
        Ok(())
    }
}
mod contracts {
    pub mod events {
        pub const STACK_TERMINAL_OUTPUT: &str = "fixture";
    }
}
include!("generated.rs");
fn entries() -> Vec<AppIndexEntry> {
    (0..4000)
        .map(|i| AppIndexEntry {
            title: format!("Fixture Application {i:04}"),
            path: PathBuf::from(format!(
                r"C:\Fixture\Programs\Vendor {i:04}\Application.exe"
            )),
            source: "currentUserStartMenu".into(),
            aliases: vec![
                format!("fixture alias {i:04}"),
                format!("Fixture App {i:04}"),
            ],
            priority: 1550,
        })
        .collect()
}
#[test]
fn snapshot_before() {
    let cache = CachedAppIndex {
        indexed_at_epoch_secs: 100,
        entries: entries().into(),
    };
    let (_, c, ns) = measure(|| {
        for _ in 0..100 {
            black_box(cached_app_entries_from_cache(Some(&cache), 110, false));
        }
    });
    println!("snapshots_4000_x100 {c:?} ns={ns}");
}
#[test]
fn acceptance_snapshot_pointer_sharing() {
    let cache = CachedAppIndex {
        indexed_at_epoch_secs: 100,
        entries: entries().into(),
    };
    let a = cached_app_entries_from_cache(Some(&cache), 110, false);
    let b = cached_app_entries_from_cache(Some(&cache), 110, false);
    assert_eq!(
        a.entries.as_ptr(),
        b.entries.as_ptr(),
        "snapshot must share immutable generation"
    );
    drop(cache);
    assert_eq!(a.entries[3999].title, b.entries[3999].title);
}
#[cfg(shared_cache_fixture)]
#[test]
fn snapshot_old_generation_released_after_last_reader() {
    let mut cache = CachedAppIndex {
        indexed_at_epoch_secs: 100,
        entries: entries().into(),
    };
    let weak = Arc::downgrade(&cache.entries);
    let a = cached_app_entries_from_cache(Some(&cache), 110, false);
    let b = cached_app_entries_from_cache(Some(&cache), 110, false);
    cache.entries = Vec::<AppIndexEntry>::new().into();
    assert!(weak.upgrade().is_some());
    assert_eq!(a.entries[3999].title, "Fixture Application 3999");
    drop(a);
    assert!(weak.upgrade().is_some());
    drop(b);
    assert!(weak.upgrade().is_none());
}
#[test]
fn cache_states() {
    let c = CachedAppIndex {
        indexed_at_epoch_secs: 100,
        entries: entries().into(),
    };
    for (now, flight, state, refresh, age) in [
        (90, false, SearchProviderCacheState::Hit, false, 0),
        (160, false, SearchProviderCacheState::Hit, false, 60000),
        (161, false, SearchProviderCacheState::Refresh, true, 61000),
        (110, true, SearchProviderCacheState::Refresh, false, 10000),
    ] {
        let s = cached_app_entries_from_cache(Some(&c), now, flight);
        assert_eq!(
            (s.cache_state, s.refresh_needed, s.cache_age_ms),
            (state, refresh, Some(age))
        );
    }
    for (flight, state, refresh) in [
        (false, SearchProviderCacheState::Miss, true),
        (true, SearchProviderCacheState::Indexing, false),
    ] {
        let s = cached_app_entries_from_cache(None, 100, flight);
        assert_eq!(
            (s.cache_state, s.refresh_needed, s.cache_age_ms),
            (state, refresh, None)
        );
    }
}
#[test]
fn persisted_schema_roundtrip() {
    let c = CachedAppIndex {
        indexed_at_epoch_secs: 100,
        entries: entries().into(),
    };
    let expected =
        serde_json::json!({"version":2,"indexedAtEpochSecs":100,"entries":&c.entries[..]});
    let bytes = serialize_current(&c);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
        expected
    );
    let root = PathBuf::from("test-results/native-memory/persisted-fixture");
    fs::create_dir_all(&root).unwrap();
    let p = root.join("index.json");
    write_persisted_app_index(&p, &c).unwrap();
    assert_eq!(fs::read(&p).unwrap(), bytes);
    let read = read_persisted_app_index(&p).unwrap().unwrap();
    assert_eq!(serialize_current(&read), bytes);
    for corrupt in [
        b"{not-json".as_slice(),
        b"{\"version\":1,\"indexedAtEpochSecs\":100,\"entries\":[]}",
    ] {
        fs::write(&p, corrupt).unwrap();
        assert!(read_persisted_app_index(&p).is_err());
    }
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn persisted_before() {
    let c = CachedAppIndex {
        indexed_at_epoch_secs: 100,
        entries: entries().into(),
    };
    let (v, count, ns) = measure(|| serialize_current(&c));
    println!("persisted_4000 json_bytes={} {count:?} ns={ns}", v.len());
}
#[test]
fn acceptance_persisted_no_entry_clone() {
    let c = CachedAppIndex {
        indexed_at_epoch_secs: 100,
        entries: entries().into(),
    };
    let (_, count, _) = measure(|| serialize_current(&c));
    assert!(
        count.allocs < 100,
        "serialization should allocate output, not clone 4000 entries: {count:?}"
    );
}
#[test]
fn persisted_owned_schema_exact_bytes() {
    let c = CachedAppIndex {
        indexed_at_epoch_secs: 100,
        entries: entries().into(),
    };
    let old = PersistedAppIndexCache {
        version: 2,
        indexed_at_epoch_secs: 100,
        entries: c.entries.iter().cloned().collect(),
    };
    assert_eq!(serialize_current(&c), serde_json::to_vec(&old).unwrap());
}
fn baseline_rank(e: &[AppIndexEntry], q: &str, n: usize) -> Vec<SearchResult> {
    let t = query_tokens(q);
    if t.is_empty() {
        return vec![];
    }
    let mut r = e
        .iter()
        .filter_map(|e| score_app(e, &t).map(|(s, m)| app_result(e, s, m)))
        .collect::<Vec<_>>();
    r.sort_by(|l, r| {
        r.score
            .cmp(&l.score)
            .then(l.title.cmp(&r.title))
            .then(l.record_key.cmp(&r.record_key))
    });
    r.truncate(n);
    r
}
#[test]
fn rank_equivalence() {
    let mut e = entries();
    for (title, p, alias) in [
        ("Fixture", r"C:\A.exe", "hidden"),
        ("Fixture", "c:/a.exe", "alias"),
        ("fixture", r"C:\é.exe", "f"),
        ("Éditeur", r"C:\Unicode\应用.exe", "fixture"),
    ] {
        e.push(AppIndexEntry {
            title: title.into(),
            path: p.into(),
            source: "test".into(),
            aliases: vec![alias.into()],
            priority: 1550,
        });
    }
    for q in ["fixture", "hidden", "éd", "alias", " "] {
        for n in [0, 1, 30, 5000] {
            let actual = rank_apps(&e, q, n);
            let expected = baseline_rank(&e, q, n);
            assert_eq!(
                serde_json::to_vec(&actual).unwrap(),
                serde_json::to_vec(&expected).unwrap(),
                "q={q} limit={n}"
            );
            assert_eq!(actual, expected);
        }
    }
}
#[test]
fn acceptance_rank_hydrates_retained_only() {
    let e = entries();
    for n in [0, 1, 30, 5000] {
        ICONS.with(|c| c.set(0));
        let (r, c, ns) = measure(|| rank_apps(&e, "fixture", n));
        let calls = ICONS.with(Cell::get);
        println!(
            "rank limit={n} rows={} icon_calls={calls} {c:?} ns={ns}",
            r.len()
        );
        assert_eq!(calls, r.len(), "only retained rows may resolve icons");
    }
}
#[test]
fn rank_before() {
    let e = entries();
    for n in [0, 1, 30, 5000] {
        ICONS.with(|c| c.set(0));
        let (r, c, ns) = measure(|| rank_apps(&e, "fixture", n));
        println!(
            "rank limit={n} rows={} icons={} {c:?} ns={ns}",
            r.len(),
            ICONS.with(Cell::get)
        );
    }
}

#[test]
fn search_native_result_trace() {
    let results = rank_apps(&entries(), "fixture", 30);
    assert_eq!(results.len(), 30);
    fs::write(
        "test-results/native-memory/search-ranking-trace.json",
        serde_json::to_vec(&results).unwrap(),
    )
    .unwrap();
}
fn image(w: u32, h: u32) -> CroppedImage {
    let mut p = vec![0; w as usize * h as usize * 4];
    for (i, p) in p.chunks_exact_mut(4).enumerate() {
        p.copy_from_slice(&[(i % 251) as u8, (i % 241) as u8, (i % 239) as u8, 255]);
    }
    CroppedImage {
        width: w,
        height: h,
        pixels: p,
    }
}
fn clip_case(w: u32, h: u32) -> (usize, usize) {
    let i = image(w, h);
    PEAK.with(|n| n.set(0));
    let p = prepare(&i, i.pixels.len(), Failpoints::default()).unwrap();
    let peak = PEAK.with(Cell::get);
    let stride = w as usize * 4;
    assert_eq!(p.dib.logical_len, i.pixels.len() + 40);
    for (row, src) in p.dib.bytes[40..p.dib.logical_len]
        .chunks_exact(stride)
        .zip(i.pixels.chunks_exact(stride).rev())
    {
        for (a, b) in row.chunks_exact(4).zip(src.chunks_exact(4)) {
            assert_eq!(a, [b[2], b[1], b[0], 255]);
        }
    }
    let mut png = Vec::with_capacity(png_lengths(&i).unwrap().2);
    encode_png(&i, &mut png).unwrap();
    assert_eq!(&p.png.bytes[..p.png.logical_len], png);
    assert!(p.png.bytes[p.png.logical_len..].iter().all(|b| *b == 0));
    let bound = p.dib.bytes.len() + p.png.bytes.len() + png.capacity();
    println!("clipboard_synthetic {w}x{h} staged_peak={peak} release_bound={bound}");
    drop(p);
    assert_eq!(STAGED.load(Ordering::SeqCst), 0);
    assert_eq!(BYTES.load(Ordering::SeqCst), 0);
    (peak, bound)
}
#[test]
fn clipboard_before() {
    for (w, h) in [(1, 1), (3, 7), (1920, 1080), (3840, 2160)] {
        clip_case(w, h);
    }
}
#[test]
fn acceptance_clipboard_dib_released_before_png_copy() {
    let (p, b) = clip_case(3840, 2160);
    assert!(
        p <= b,
        "DIB vector/accounting must release before PNG global allocation"
    );
}
#[test]
fn clipboard_failure_cleanup() {
    let i = image(1, 1);
    assert!(matches!(
        prepare(&i, 4, Failpoints { allocation: true }),
        Err(Failure::Allocation)
    ));
    assert_eq!(STAGED.load(Ordering::SeqCst), 0);
    assert_eq!(BYTES.load(Ordering::SeqCst), 0);
}
fn terminal_run(bytes: Vec<u8>, cap: usize) -> Vec<TerminalReaderMessage> {
    let (tx, rx) = mpsc::sync_channel(cap);
    spawn_terminal_reader(
        std::io::Cursor::new(bytes),
        tx,
        StackTerminalOutputStream::Stdout,
        None,
        "fixture".into(),
        "fixture".into(),
    );
    rx.try_iter().collect()
}
#[test]
fn terminal_boundaries() {
    for b in [vec![], b"ok".to_vec(), vec![0xff, 0xfe], vec![0xe2, 0x82]] {
        let expected = String::from_utf8_lossy(&b).into_owned();
        let r = terminal_run(b, 16);
        assert_eq!(
            r.iter().map(|m| m.text.as_str()).collect::<String>(),
            expected
        );
        for (i, m) in r.iter().enumerate() {
            assert_eq!(m.sequence, i as u64 + 1);
        }
    }
    let mut b = vec![b'a'; 4095];
    b.extend_from_slice("€".as_bytes());
    assert_eq!(
        terminal_run(b, 16)
            .iter()
            .map(|m| m.text.as_str())
            .collect::<String>(),
        format!("{}€", "a".repeat(4095))
    );
    assert_eq!(terminal_run(vec![b'a'; 8192], 1).len(), 1);
    let (tx, rx) = mpsc::sync_channel(1);
    drop(rx);
    spawn_terminal_reader(
        std::io::Cursor::new(vec![b'a'; 8192]),
        tx,
        StackTerminalOutputStream::Stdout,
        None,
        "fixture".into(),
        "fixture".into(),
    );
}
#[test]
fn terminal_push_poll_trace() {
    let (tx, rx) = mpsc::sync_channel(16);
    let app = AppHandle {
        emitted: Default::default(),
    };
    let mut b = vec![b'a'; 4095];
    b.extend_from_slice("€".as_bytes());
    b.extend_from_slice(&[b'x', 0xe2, 0x82]);
    spawn_terminal_reader(
        std::io::Cursor::new(b),
        tx,
        StackTerminalOutputStream::Stdout,
        Some(app.clone()),
        "fixture".into(),
        "fixture".into(),
    );
    let pushed = app.emitted.lock().unwrap().clone();
    let polled = rx
        .try_iter()
        .map(|m| StackTerminalOutputChunk {
            session_id: "fixture".into(),
            stream: m.stream,
            text: m.text,
            sequence: m.sequence,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_vec(&pushed).unwrap(),
        serde_json::to_vec(&polled).unwrap()
    );
    fs::write(
        "test-results/native-memory/terminal-trace.json",
        serde_json::to_vec(&serde_json::json!({"pushed":pushed,"polled":polled})).unwrap(),
    )
    .unwrap();
    let (tx, rx) = mpsc::sync_channel(1);
    let app = AppHandle {
        emitted: Default::default(),
    };
    spawn_terminal_reader(
        std::io::Cursor::new(vec![b'x'; 12288]),
        tx,
        StackTerminalOutputStream::Stdout,
        Some(app.clone()),
        "fixture".into(),
        "fixture".into(),
    );
    assert_eq!(
        app.emitted
            .lock()
            .unwrap()
            .iter()
            .map(|m| m.sequence)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert_eq!(rx.try_iter().map(|m| m.sequence).collect::<Vec<_>>(), [1]);
}
#[test]
fn terminal_before() {
    let b = vec![b'a'; 16 * 1024 * 1024];
    let (r, c, ns) = measure(|| terminal_run(b, 4096));
    println!("terminal_16MiB_4096 rows={} {c:?} ns={ns}", r.len());
    assert_eq!(r.len(), 4096);
}
#[test]
fn acceptance_terminal_no_text_clone() {
    let b = vec![b'a'; 16 * 1024 * 1024];
    let (_, c, _) = measure(|| terminal_run(b, 4096));
    assert_eq!(c.explicit_text_clone_bytes, 0);
    assert!(
        c.bytes < 20 * 1024 * 1024,
        "one decoded allocation per chunk, not two: {c:?}"
    );
}
fn flatten_candidate(s: &VecDeque<Box<[f32]>>) -> Vec<f32> {
    let mut v = Vec::with_capacity(s.iter().map(|s| s.len()).sum());
    for s in s {
        v.extend_from_slice(s)
    }
    v
}
#[test]
fn speech_measure_first() {
    for sizes in [
        vec![],
        vec![0],
        vec![2560; 64],
        vec![2560; 8],
        vec![2560, 17],
        vec![1, 3],
    ] {
        let s = sizes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (0..*n)
                    .map(|j| f32::from_bits((i * 2560 + j) as u32))
                    .collect::<Vec<_>>()
                    .into_boxed_slice()
            })
            .collect::<VecDeque<_>>();
        let (a, ca, ta) = measure(|| flatten_current(&s));
        let (b, cb, tb) = measure(|| flatten_candidate(&s));
        assert_eq!(
            a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
        println!(
            "speech segments={} samples={} current={ca:?} ns={ta} presize={cb:?} ns={tb}",
            s.len(),
            a.len()
        );
    }
}
#[test]
fn speech_validation_bitwise() {
    for input in [
        vec![],
        vec![0.; 1],
        vec![0.; 1599],
        vec![0.; 1600],
        {
            let mut v = vec![0.; 1600];
            v[3] = f32::NAN;
            v
        },
        {
            let mut v = vec![0.; 1600];
            v[1] = f32::INFINITY;
            v
        },
        {
            let mut v = vec![0.; 1600];
            v[1] = f32::NEG_INFINITY;
            v
        },
        {
            let mut v = vec![0.; 1600];
            v[1] = -0.;
            v
        },
    ] {
        let s = VecDeque::from([input.into_boxed_slice()]);
        let a = flatten_current(&s);
        let b = flatten_candidate(&s);
        assert_eq!(
            a.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            b.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
        assert_eq!(validate_audio(&a), validate_audio(&b));
    }
}
#[test]
fn acceptance_speech_one_allocation() {
    let s = (0..64)
        .map(|_| vec![0.; 2560].into_boxed_slice())
        .collect::<VecDeque<_>>();
    let (_, c, _) = measure(|| flatten_current(&s));
    assert_eq!((c.allocs, c.reallocs, c.bytes), (1, 0, 655360));
}
const fn table() -> [u32; 256] {
    let mut t = [0; 256];
    let mut i = 0;
    while i < 256 {
        let mut v = i as u32;
        let mut j = 0;
        while j < 8 {
            v = (v >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(v & 1));
            j += 1
        }
        t[i] = v;
        i += 1
    }
    t
}
static REFERENCE_CRC_TABLE: [u32; 256] = table();
fn reference_crc_table(b: &[u8]) -> u32 {
    let mut v = u32::MAX;
    for b in b {
        v = (v >> 8) ^ REFERENCE_CRC_TABLE[((v ^ u32::from(*b)) & 255) as usize]
    }
    !v
}
#[test]
fn crc_equivalence_and_before() {
    assert_eq!(crc(b"123456789"), 0xcbf43926);
    assert_eq!(crc(&[]), 0);
    for b in 0..=255 {
        assert_eq!(crc(&[b]), reference_crc_table(&[b]));
    }
    for n in [3, 65534, 65535, 65536, 1920 * 1080 * 4, 3840 * 2160 * 4] {
        let b = (0..n).map(|i| (i % 251) as u8).collect::<Vec<_>>();
        let (a, _, ta) = measure(|| crc(black_box(&b)));
        let (c, _, tc) = measure(|| reference_crc_table(black_box(&b)));
        assert_eq!(a, c);
        println!(
            "crc bytes={n} current_ns={ta} candidate_table_ns={tc} candidate_static_bytes={}",
            std::mem::size_of_val(&REFERENCE_CRC_TABLE)
        );
    }
}
#[test]
fn descendants_reference() {
    for pairs in [
        vec![(1, vec![1])],
        vec![(1, vec![2]), (2, vec![1, 3, 3])],
        vec![(1, vec![2, 3]), (2, vec![4]), (99, vec![100])],
    ] {
        let map = pairs.into_iter().collect::<HashMap<_, _>>();
        for p in [1, 2, 3, 99, 500] {
            let d = descendant_pids(p, &map);
            let mut visited = HashSet::new();
            let mut stack = map.get(&p).cloned().unwrap_or_default();
            while let Some(x) = stack.pop() {
                if visited.insert(x) {
                    if let Some(c) = map.get(&x) {
                        stack.extend(c)
                    }
                }
            }
            let mut expected = visited.into_iter().collect::<Vec<_>>();
            expected.sort_unstable();
            assert_eq!(d, expected);
        }
    }
}
#[test]
fn process_before() {
    for shape in ["chain", "star", "mixed"] {
        let mut m = HashMap::new();
        for n in 2..=1000 {
            m.entry(if shape == "star" {
                1
            } else if shape == "mixed" {
                n / 2
            } else {
                n - 1
            })
            .or_insert_with(Vec::new)
            .push(n);
        }
        let (_, c, ns) = measure(|| {
            for p in 1..=1000 {
                black_box(descendant_pids(p, &m).len());
            }
        });
        println!("process_{shape}_1000 {c:?} ns={ns}");
    }
}
fn processes(shape: &str) -> Vec<ProcessInfo> {
    (1..=1000)
        .map(|pid| ProcessInfo {
            pid,
            name: format!("fixture-{pid}"),
            parent_pid: if pid == 1 {
                None
            } else {
                Some(if shape == "star" {
                    1
                } else if shape == "mixed" {
                    pid / 2
                } else {
                    pid - 1
                })
            },
            parent_name: None,
            child_process_count: 0,
            descendant_process_count: 0,
        })
        .collect()
}
fn check_enrich(mut p: Vec<ProcessInfo>) {
    let map = children_by_parent(&p);
    let names = p
        .iter()
        .map(|p| (p.pid, p.name.clone()))
        .collect::<HashMap<_, _>>();
    let expected = p
        .iter()
        .map(|p| {
            (
                map.get(&p.pid).map_or(0, Vec::len) as u32,
                descendant_pids(p.pid, &map).len() as u32,
                p.parent_pid.and_then(|x| names.get(&x).cloned()),
            )
        })
        .collect::<Vec<_>>();
    enrich_process_tree(&mut p);
    assert_eq!(
        p.iter()
            .map(|p| (
                p.child_process_count,
                p.descendant_process_count,
                p.parent_name.clone()
            ))
            .collect::<Vec<_>>(),
        expected
    );
}
#[test]
fn process_enrich_equivalence() {
    for s in ["chain", "star", "mixed"] {
        check_enrich(processes(s))
    }
    let mut p = processes("mixed");
    p.truncate(8);
    p[0].parent_pid = Some(1);
    p[1].parent_pid = Some(3);
    p[2].parent_pid = Some(2);
    p[3].parent_pid = Some(9999);
    p[4].parent_pid = None;
    p.push(p[1].clone());
    check_enrich(p);
}

#[test]
fn process_origin_reachable_and_duplicate_semantics() {
    let mut self_loop = processes("chain");
    self_loop.truncate(1);
    self_loop[0].parent_pid = Some(1);
    enrich_process_tree(&mut self_loop);
    assert_eq!(
        (
            self_loop[0].child_process_count,
            self_loop[0].descendant_process_count
        ),
        (1, 1)
    );
    let mut cycle = processes("chain");
    cycle.truncate(2);
    cycle[0].parent_pid = Some(2);
    cycle.push(cycle[1].clone());
    let map = children_by_parent(&cycle);
    assert_eq!(descendant_pids(1, &map), vec![1, 2]);
    enrich_process_tree(&mut cycle);
    assert_eq!(
        (
            cycle[0].child_process_count,
            cycle[0].descendant_process_count
        ),
        (2, 2)
    );
    assert_eq!(
        (
            cycle[1].child_process_count,
            cycle[1].descendant_process_count
        ),
        (1, 2)
    );
}
#[test]
fn process_enrich_before() {
    for s in ["chain", "star", "mixed"] {
        let mut p = processes(s);
        let (_, c, ns) = measure(|| enrich_process_tree(&mut p));
        println!("enrich_{s}_1000 {c:?} ns={ns}");
    }
}
#[test]
fn acceptance_process_reuses_count_scratch() {
    let mut p = processes("chain");
    let (_, c, _) = measure(|| enrich_process_tree(&mut p));
    assert!(
        c.allocs < 7000,
        "reusable count-only traversal should avoid per-PID allocations: {c:?}"
    );
}
#[test]
fn repeat_cpu_samples() {
    for n in [1920 * 1080 * 4, 3840 * 2160 * 4] {
        let b = (0..n).map(|i| (i % 251) as u8).collect::<Vec<_>>();
        for round in 0..5 {
            let (a, _, ta) = measure(|| crc(black_box(&b)));
            let (c, _, tb) = measure(|| reference_crc_table(black_box(&b)));
            assert_eq!(a, c);
            println!("crc_repeat bytes={n} round={round} current_ns={ta} candidate_table_ns={tb}");
        }
    }
    let s = (0..64)
        .map(|_| vec![0.125; 2560].into_boxed_slice())
        .collect::<VecDeque<_>>();
    for round in 0..5 {
        let (_, a, ta) = measure(|| flatten_current(black_box(&s)));
        let (_, b, tb) = measure(|| flatten_candidate(black_box(&s)));
        println!("speech_repeat round={round} current={a:?} ns={ta} presize={b:?} ns={tb}");
    }
}
#[test]
fn repeat_first_memory_samples() {
    let c = CachedAppIndex {
        indexed_at_epoch_secs: 100,
        entries: entries().into(),
    };
    for round in 0..5 {
        let (_, a, ta) = measure(|| {
            for _ in 0..100 {
                black_box(cached_app_entries_from_cache(Some(&c), 110, false));
            }
        });
        let (_, b, tb) = measure(|| rank_apps(&c.entries, "fixture", 30));
        let input = vec![b'a'; 16 * 1024 * 1024];
        let (_, d, td) = measure(|| terminal_run(input, 4096));
        println!("first_repeat round={round} snapshot={a:?} ns={ta} rank30={b:?} ns={tb} terminal={d:?} ns={td}");
    }
}

#[test]
fn repeat_process_samples() {
    for shape in ["chain", "star", "mixed"] {
        for round in 0..5 {
            let mut p = processes(shape);
            let (_, count, ns) = measure(|| enrich_process_tree(&mut p));
            println!("process_repeat shape={shape} round={round} {count:?} ns={ns}");
        }
    }
}
