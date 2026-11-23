//! Read-only Chrome and Edge bookmarks. Runs inside the blocking search worker.

use crate::search::contracts::{
    iso_now, SearchProviderCacheState, SearchProviderHealth, SearchProviderHealthState,
    SearchProviderId, SearchProviderTiming, SearchResult, SearchResultAction, SearchResultKind,
};
use crate::search::providers::web::validate_search_open_url;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

/// Files larger than this are skipped unread.
pub(crate) const MAX_BOOKMARK_FILE_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_BOOKMARK_ENTRIES: usize = 20_000;
/// Across all browsers.
pub(crate) const MAX_BOOKMARK_PROFILES: usize = 16;
pub(crate) const MAX_BOOKMARK_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
/// An unreadable file is not retried sooner than this while it is unchanged.
pub(crate) const BOOKMARK_FAILURE_TTL: Duration = Duration::from_secs(5);
/// Background refreshes of one root are at least this far apart.
pub(crate) const BOOKMARK_DISCOVERY_INTERVAL: Duration = Duration::from_secs(2);
const FIRST_INDEX_WAIT: Duration = Duration::from_secs(1);
const MAX_TITLE_CHARS: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BookmarkEntry {
    pub(crate) title: String,
    pub(crate) url: String,
}

#[derive(Clone, Debug)]
pub(crate) struct BookmarkSnapshot {
    /// De-duplicated by URL.
    pub(crate) entries: Vec<BookmarkEntry>,
    pub(crate) cache: SearchProviderCacheState,
}

#[derive(Clone, Debug)]
pub(crate) struct BookmarksSearchRun {
    pub(crate) results: Vec<SearchResult>,
    pub(crate) timing: SearchProviderTiming,
    pub(crate) health: SearchProviderHealth,
}

struct IndexedBookmark {
    entry: BookmarkEntry,
    title_lower: String,
    /// Lowercase `title url`, so a keystroke does no per-entry allocation.
    haystack: String,
}

struct CachedFile {
    modified: Option<SystemTime>,
    len: u64,
    entries: Vec<BookmarkEntry>,
    /// Set when the file could not be read; the failure itself is cached for a while.
    failed_at: Option<Instant>,
}

type SharedIndex = Arc<Vec<IndexedBookmark>>;

#[derive(Default)]
struct RootState {
    /// Candidate paths and when they were discovered.
    discovered: Option<(Instant, Vec<PathBuf>)>,
    files: HashMap<PathBuf, CachedFile>,
    merged: SharedIndex,
}

pub(crate) trait BookmarkFileReader: Send + Sync {
    fn read(&self, path: &Path, max_bytes: u64) -> io::Result<String>;
}

struct FsBookmarkReader;

impl BookmarkFileReader for FsBookmarkReader {
    fn read(&self, path: &Path, max_bytes: u64) -> io::Result<String> {
        // Rust's default Windows share mode lets the browser keep writing.
        let mut bytes = Vec::new();
        fs::File::open(path)?
            .take(max_bytes + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > max_bytes {
            return Err(io::Error::other("bookmark file is too large"));
        }
        // Not UTF-8 means malformed, which is cached like any other empty file.
        Ok(String::from_utf8(bytes).unwrap_or_default())
    }
}

#[derive(Default)]
struct Flight {
    running: bool,
    /// Per root: when its first and its latest background refresh started.
    started: HashMap<PathBuf, (Instant, Instant)>,
}

pub(crate) struct BookmarkCache {
    reader: Arc<dyn BookmarkFileReader>,
    /// Held across disk I/O. Never taken on the path to a Local payload.
    roots: Mutex<HashMap<PathBuf, RootState>>,
    /// Completed indexes. Held only to clone an `Arc`.
    published: Mutex<HashMap<PathBuf, SharedIndex>>,
    flight: Mutex<Flight>,
    landed: Condvar,
    file_reads: AtomicUsize,
    discovery_scans: AtomicUsize,
}

impl Default for BookmarkCache {
    fn default() -> Self {
        Self::with_reader(Arc::new(FsBookmarkReader))
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}

impl BookmarkCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn with_reader(reader: Arc<dyn BookmarkFileReader>) -> Self {
        Self {
            reader,
            roots: Mutex::default(),
            published: Mutex::default(),
            flight: Mutex::default(),
            landed: Condvar::new(),
            file_reads: AtomicUsize::new(0),
            discovery_scans: AtomicUsize::new(0),
        }
    }

    /// Number of profile directory scans ever performed.
    pub(crate) fn discovery_scans(&self) -> usize {
        self.discovery_scans.load(Ordering::SeqCst)
    }

    /// Number of file reads (parses) ever performed.
    pub(crate) fn file_reads(&self) -> usize {
        self.file_reads.load(Ordering::SeqCst)
    }

    /// Synchronous: revalidates every known file now.
    pub(crate) fn snapshot(&self, local_app_data: &Path) -> BookmarkSnapshot {
        self.snapshot_at(local_app_data, Instant::now())
    }

    pub(crate) fn snapshot_at(&self, local_app_data: &Path, now: Instant) -> BookmarkSnapshot {
        let (merged, cache) = self.refresh(local_app_data, now);
        BookmarkSnapshot {
            entries: merged.iter().map(|item| item.entry.clone()).collect(),
            cache,
        }
    }

    /// The last completed index, without any I/O.
    pub(crate) fn last_known(&self, local_app_data: &Path) -> Option<BookmarkSnapshot> {
        self.published_index(local_app_data).map(|merged| BookmarkSnapshot {
            entries: merged.iter().map(|item| item.entry.clone()).collect(),
            cache: SearchProviderCacheState::Hit,
        })
    }

    fn published_index(&self, local_app_data: &Path) -> Option<SharedIndex> {
        lock(&self.published).get(local_app_data).map(Arc::clone)
    }

    /// Starts a background refresh unless one is running or this root was refreshed recently.
    pub(crate) fn refresh_if_due(self: &Arc<Self>, local_app_data: &Path) -> bool {
        let now = Instant::now();
        {
            let mut flight = lock(&self.flight);
            let recent = flight
                .started
                .get(local_app_data)
                .is_some_and(|(_, last)| now.duration_since(*last) < BOOKMARK_DISCOVERY_INTERVAL);
            if flight.running || recent {
                return false;
            }
            flight.running = true;
            flight
                .started
                .entry(local_app_data.to_path_buf())
                .or_insert((now, now))
                .1 = now;
        }
        // Dropped on completion, panic, or a failed spawn.
        let landing = Landing(Arc::clone(self));
        let root = local_app_data.to_path_buf();
        std::thread::Builder::new()
            .name("search-bookmarks-refresh".into())
            .spawn(move || {
                landing.0.refresh(&root, Instant::now());
            })
            .is_ok()
    }

    /// Waits for the first-ever index of a root. The wait is measured from the first refresh, so
    /// a hung profile delays at most one second of final payloads in total.
    pub(crate) fn wait_for_first_index(
        self: &Arc<Self>,
        local_app_data: &Path,
        still_wanted: impl Fn() -> bool,
    ) {
        while still_wanted() && self.published_index(local_app_data).is_none() {
            // Another root's refresh may hold the single flight.
            self.refresh_if_due(local_app_data);
            let flight = lock(&self.flight);
            let expired = flight
                .started
                .get(local_app_data)
                .is_some_and(|(first, _)| first.elapsed() >= FIRST_INDEX_WAIT);
            if expired || (!flight.running && flight.started.contains_key(local_app_data)) {
                return;
            }
            drop(self.landed.wait_timeout(flight, Duration::from_millis(20)));
        }
    }

    fn refresh(
        &self,
        local_app_data: &Path,
        now: Instant,
    ) -> (SharedIndex, SearchProviderCacheState) {
        let mut roots = lock(&self.roots);
        let state = roots.entry(local_app_data.to_path_buf()).or_default();
        let candidates = match state.discovered.take().filter(|(at, _)| {
            now.saturating_duration_since(*at) < BOOKMARK_DISCOVERY_INTERVAL
        }) {
            Some(discovered) => discovered,
            None => {
                self.discovery_scans.fetch_add(1, Ordering::SeqCst);
                (now, bookmark_file_candidates(local_app_data))
            }
        };
        let mut previous = std::mem::take(&mut state.files);
        let mut order = Vec::new();
        let mut read_any = false;
        let mut total_bytes = 0;
        for path in candidates.1.iter().cloned() {
            let Some((modified, len)) = file_stamp(&path) else {
                continue;
            };
            if total_bytes + len > MAX_BOOKMARK_TOTAL_BYTES {
                continue;
            }
            total_bytes += len;
            let unchanged = previous.remove(&path).filter(|cached| {
                cached.modified == modified
                    && cached.len == len
                    && cached.failed_at.is_none_or(|failed| {
                        now.saturating_duration_since(failed) < BOOKMARK_FAILURE_TTL
                    })
            });
            let cached = unchanged.unwrap_or_else(|| {
                read_any = true;
                let entries = self.read_entries(&path);
                CachedFile {
                    modified,
                    len,
                    failed_at: entries.is_none().then_some(now),
                    entries: entries.unwrap_or_default(),
                }
            });
            order.push(path.clone());
            state.files.insert(path, cached);
        }
        state.discovered = Some(candidates);
        if read_any || !previous.is_empty() {
            state.merged = Arc::new(merge_entries(
                order.iter().filter_map(|path| state.files.get(path)),
            ));
        }
        let merged = Arc::clone(&state.merged);
        lock(&self.published).insert(local_app_data.to_path_buf(), Arc::clone(&merged));
        let cache = if read_any {
            SearchProviderCacheState::Miss
        } else {
            SearchProviderCacheState::Hit
        };
        (merged, cache)
    }

    fn read_entries(&self, path: &Path) -> Option<Vec<BookmarkEntry>> {
        self.file_reads.fetch_add(1, Ordering::SeqCst);
        let json = self.reader.read(path, MAX_BOOKMARK_FILE_BYTES).ok()?;
        Some(parse_bookmarks_json(&json, MAX_BOOKMARK_ENTRIES))
    }
}

struct Landing(Arc<BookmarkCache>);

impl Drop for Landing {
    fn drop(&mut self) {
        lock(&self.0.flight).running = false;
        self.0.landed.notify_all();
    }
}

fn file_stamp(path: &Path) -> Option<(Option<SystemTime>, u64)> {
    let metadata = fs::symlink_metadata(path).ok()?;
    (metadata.is_file() && metadata.len() <= MAX_BOOKMARK_FILE_BYTES)
        .then(|| (metadata.modified().ok(), metadata.len()))
}

fn merge_entries<'a>(files: impl Iterator<Item = &'a CachedFile>) -> Vec<IndexedBookmark> {
    let mut seen = HashSet::new();
    files
        .flat_map(|file| &file.entries)
        .filter(|entry| seen.insert(entry.url.as_str()))
        .take(MAX_BOOKMARK_ENTRIES)
        .map(|entry| {
            let title_lower = entry.title.to_lowercase();
            IndexedBookmark {
                haystack: format!("{title_lower} {}", entry.url.to_lowercase()),
                title_lower,
                entry: entry.clone(),
            }
        })
        .collect()
}

/// Existing regular `Bookmarks` files of the default and numbered profiles, sorted and capped.
pub(crate) fn bookmark_file_candidates(local_app_data: &Path) -> Vec<PathBuf> {
    let [chrome, edge] = [["Google", "Chrome"], ["Microsoft", "Edge"]].map(|[vendor, browser]| {
        let root = local_app_data.join(vendor).join(browser).join("User Data");
        let mut files = fs::read_dir(root)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_name().to_str().is_some_and(is_profile_name))
            .map(|entry| entry.path().join("Bookmarks"))
            .filter(|path| fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file()))
            .collect::<Vec<_>>();
        files.sort();
        files
    });
    // Neither browser may use up the other's half of the cap.
    let chrome_share = (MAX_BOOKMARK_PROFILES - edge.len().min(MAX_BOOKMARK_PROFILES / 2))
        .min(chrome.len());
    let mut files = chrome
        .into_iter()
        .take(chrome_share)
        .chain(edge.into_iter().take(MAX_BOOKMARK_PROFILES - chrome_share))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn is_profile_name(name: &str) -> bool {
    name == "Default"
        || name.strip_prefix("Profile ").is_some_and(|number| {
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
        })
}

/// Malformed or oddly shaped JSON yields no entries; folders are flattened.
pub(crate) fn parse_bookmarks_json(json: &str, max_entries: usize) -> Vec<BookmarkEntry> {
    // serde_json bounds recursion, so hostile nesting is a parse error, not a stack overflow.
    let Ok(document) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    let Some(roots) = document.get("roots").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    let mut pending = roots.values().rev().collect::<Vec<_>>();
    while let Some(node) = pending.pop() {
        if entries.len() >= max_entries {
            break;
        }
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            pending.extend(children.iter().rev());
        } else if node.get("type").and_then(Value::as_str) == Some("url") {
            entries.extend(bookmark_entry(node));
        }
    }
    entries
}

fn bookmark_entry(node: &Value) -> Option<BookmarkEntry> {
    let url = node.get("url").and_then(Value::as_str)?;
    let url = validate_search_open_url(url).ok()?;
    let title = node
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .chars()
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .take(MAX_TITLE_CHARS)
        .collect::<String>();
    let title = title.trim();
    Some(BookmarkEntry {
        title: if title.is_empty() {
            url.clone()
        } else {
            title.to_string()
        },
        url,
    })
}

pub(crate) fn search_bookmarks(
    query: &str,
    limit: usize,
    local_app_data: &Path,
    cache: &BookmarkCache,
) -> BookmarksSearchRun {
    let started_at = iso_now();
    let started = Instant::now();
    let (indexed, cache_state) = cache.refresh(local_app_data, started);
    bookmarks_run(query, limit, &indexed, cache_state, started_at, started)
}

/// Searches the last completed index only: no disk I/O and no waiting on a refresh.
pub(crate) fn search_known_bookmarks(
    query: &str,
    limit: usize,
    local_app_data: &Path,
    cache: &BookmarkCache,
) -> BookmarksSearchRun {
    let started_at = iso_now();
    let started = Instant::now();
    let (indexed, cache_state) = match cache.published_index(local_app_data) {
        Some(indexed) => (indexed, SearchProviderCacheState::Hit),
        None => (SharedIndex::default(), SearchProviderCacheState::Indexing),
    };
    bookmarks_run(query, limit, &indexed, cache_state, started_at, started)
}

fn bookmarks_run(
    query: &str,
    limit: usize,
    indexed: &[IndexedBookmark],
    cache_state: SearchProviderCacheState,
    started_at: String,
    started: Instant,
) -> BookmarksSearchRun {
    let results = rank_bookmarks(query, limit, indexed);

    BookmarksSearchRun {
        timing: SearchProviderTiming {
            provider_id: SearchProviderId::Bookmarks,
            started_at,
            ended_at: Some(iso_now()),
            duration_ms: started.elapsed().as_secs_f64() * 1000.0,
            cache: cache_state,
            cache_age_ms: None,
            result_count: results.len(),
            applied: true,
            discarded_as_stale: false,
        },
        health: SearchProviderHealth {
            provider_id: SearchProviderId::Bookmarks,
            state: SearchProviderHealthState::Ready,
            reason_code: None,
            message: None,
        },
        results,
    }
}

fn rank_bookmarks(query: &str, limit: usize, indexed: &[IndexedBookmark]) -> Vec<SearchResult> {
    let query = query.trim().to_lowercase();
    let tokens = query.split_whitespace().collect::<Vec<_>>();
    // Every URL contains ":" and "/", so punctuation alone would match everything.
    if !query.chars().any(char::is_alphanumeric) {
        return Vec::new();
    }
    let mut matches = indexed
        .iter()
        .filter(|item| tokens.iter().all(|token| item.haystack.contains(token)))
        .map(|item| (BookmarkMatch::of(item, &query, &tokens), item))
        .collect::<Vec<_>>();
    matches.sort_by(|(left_rank, left), (right_rank, right)| {
        left_rank
            .cmp(right_rank)
            .then_with(|| left.title_lower.cmp(&right.title_lower))
            .then_with(|| left.entry.url.cmp(&right.entry.url))
    });
    matches
        .into_iter()
        .take(limit)
        .map(|(tier, item)| bookmark_result(tier, &item.entry))
        .collect()
}

/// Ordered strongest first. Carried in `match_reason`; ranking and the renderer depend on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum BookmarkMatch {
    Exact,
    Prefix,
    TokenPrefix,
    TitleSubstring,
    UrlSubstring,
}

impl BookmarkMatch {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Prefix => "prefix",
            Self::TokenPrefix => "tokenPrefix",
            Self::TitleSubstring | Self::UrlSubstring => "substring",
        }
    }

    /// Anything but a match on the start of the title; shown only once every provider finished.
    pub(crate) fn is_weak(self) -> bool {
        self >= Self::TokenPrefix
    }

    /// Anything unrecognized is treated as the weakest match.
    pub(crate) fn of_row(row: &SearchResult) -> Option<Self> {
        (row.provider_id == SearchProviderId::Bookmarks).then(|| {
            [Self::Exact, Self::Prefix, Self::TokenPrefix]
                .into_iter()
                .find(|tier| tier.reason() == row.match_reason)
                .unwrap_or(Self::UrlSubstring)
        })
    }

    fn of(item: &IndexedBookmark, query: &str, tokens: &[&str]) -> Self {
        let title = &item.title_lower;
        if title == query {
            return Self::Exact;
        }
        if title.starts_with(query) {
            return Self::Prefix;
        }
        if !tokens.iter().all(|token| title.contains(token)) {
            return Self::UrlSubstring;
        }
        let starts_word = |token: &&str| {
            title
                .split(|ch: char| !ch.is_alphanumeric())
                .any(|word| word.starts_with(*token))
        };
        if tokens.iter().all(starts_word) {
            Self::TokenPrefix
        } else {
            Self::TitleSubstring
        }
    }
}

fn bookmark_result(tier: BookmarkMatch, entry: &BookmarkEntry) -> SearchResult {
    let key = format!("bookmark:{}", entry.url);
    SearchResult {
        id: key.clone(),
        provider_id: SearchProviderId::Bookmarks,
        kind: SearchResultKind::Bookmark,
        title: entry.title.clone(),
        subtitle: Some(entry.url.clone()),
        path: None,
        action: SearchResultAction::OpenBookmark {
            url: entry.url.clone(),
        },
        terms: Vec::new(),
        aliases: Vec::new(),
        score: 0,
        provider_signal: 0,
        match_reason: tier.reason().to_string(),
        record_key: key,
        title_highlight_data: Vec::new(),
        subtitle_highlight_data: Vec::new(),
        icon_data_url: None,
    }
}
