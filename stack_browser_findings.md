# Stack Browser Findings

Date: 2026-05-01
Workspace: `C:\dev\jasonshell`
Scope: investigation only, no production code changes

## Symptom

Opening larger folders in Stack Browser can stall at `80 of N items` and sometimes freeze or crash the app. Downloads is a good repro because it has roughly 649 items on this machine, with many `.zip`, `.exe`, `.rar`, and folder entries.

## Main findings

### 1. `80` is not random. It is hard-coded first-page size.

- Frontend initial page size is `80`: `src/lib/stackPopup.ts:164`.
- Subsequent pages jump to `500`: `src/lib/stackPopup.ts:165`.
- Folder open uses `listStackFolder(...)` and progressively applies each page: `src/components/StackPopupSurface.svelte:276-305`.
- Status text is derived from currently merged entries, so after first page user sees `80 of 680 items` or similar: `src/lib/stackPopupState.ts:241-249`.

Why user sees exact `80`:
- page 1 loads and renders
- page 2 starts after that
- if page 2 stalls, fails, or crashes process/webview, UI stays on first merged batch

### 2. Backend paging is not real paging.

`read_stack_folder_page(...)` re-enumerates whole directory for every page request:

- full `fs::read_dir(path)` scan: `src-tauri/src/stack_popup/paging.rs:15-28`
- full in-memory sort of all entries: `src-tauri/src/stack_popup/paging.rs:29-33`
- only after full scan/sort does it `skip(offset).take(limit)`: `src-tauri/src/stack_popup/paging.rs:35-41`

Effect:
- opening 649-item folder does not read 80 items fast and continue later
- page 1 still scans and sorts all 649 entries before first paint
- page 2 scans and sorts all 649 entries again
- page 3 scans and sorts all 649 entries again

So current design already violates intended "display instantly" behavior even before icon work is considered.

### 3. Page 2 is much heavier than page 1.

After first 80 rows, next request asks for 500 rows: `src/lib/stackPopup.ts:178-183`.

That means second request does all of this in one burst:
- re-scan full directory
- re-sort full directory
- materialize metadata for 500 items
- resolve icon for 500 items
- serialize 500 full `StackItem` payloads back over Tauri IPC

This lines up exactly with observed freeze pattern: first 80 appear, then app hits much larger synchronous second wave.

### 4. Each stack row resolves shell icon synchronously.

Each `StackItem` is built with `icon_data_url: stack_item_icon_data_url(&path)`: `src-tauri/src/stack_popup/items.rs:29-33`.

On Windows that calls `crate::task_windows::shell_file_icon_data_url(path).ok()`: `src-tauri/src/stack_popup/items.rs:45-48`.

That icon path uses synchronous `SHGetFileInfoW(...)` plus PNG encoding and base64 conversion:
- `SHGetFileInfoW`: `src-tauri/src/task_windows/icons.rs:97-108`
- PNG/base64 packaging: `src-tauri/src/task_windows/icons.rs:114-123`

Important implication:
- Stack Browser open path is doing shell icon extraction inline for every returned item.
- There is no timeout around `SHGetFileInfoW(...)` here.
- If shell icon resolution is slow for any file type/association, whole page blocks.

Downloads is especially hostile because it contains many archive and executable types, which increases shell icon work.

### 5. Stack Browser has no icon cache, unlike other surfaces.

Other parts of repo already defend against repeated icon resolution:
- Search has `SEARCH_ICON_CACHE`: `src-tauri/src/search/icons.rs:5-19`
- Process Manager caches by executable path: `src-tauri/src/process_manager.rs:662-679`

Stack Browser does not use a comparable cache. It resolves icon data fresh for every folder page load. That multiplies latency and memory traffic when reopening folders or when page 2/page 3 request many rows.

### 6. IPC payload size is inflated by embedded base64 icons.

`StackItem` includes `icon_data_url` in serialized page payloads: `src-tauri/src/stack_popup/models.rs:39-50`.

Because page 2 can contain 500 items, backend may ship hundreds of base64 PNG strings in one invoke response. Even if each icon is individually small, this is still a large synchronous payload spike compared with returning metadata only.

This is strong candidate for intermittent crash/freeze behavior because:
- payload size depends on file mix
- icon extraction cost depends on shell handlers and file associations
- second page size is 6.25x first page size

### 7. Current tests cover pagination correctness, not performance or resilience.

There is a Rust test proving big folders paginate without truncation: `src-tauri/src/stack_popup.rs:397-412`.

What it does not test:
- first paint latency
- repeated full-dir rescans per page
- icon extraction cost
- page payload size
- failure behavior when shell icon resolution is slow
- large-folder open in actual Stack Browser surface

## Root cause summary

Most likely root cause is not one bug but one bad load shape:

1. open folder
2. backend scans whole directory before first page returns
3. first page returns only 80 items
4. frontend immediately requests 500 more
5. backend again scans whole directory and now resolves/icons+serializes 500 rows
6. app freezes, stalls, or crashes during that heavy second wave
7. UI remains showing `80 of N items`

## Why crash is intermittent

Static code suggests several variability sources:
- shell icon extraction cost varies by file type and association
- Downloads contents are mixed, with many archives/executables/folders
- second-page payload size varies by icon output size
- repeated no-cache icon work raises CPU/memory pressure unpredictably

So same folder can sometimes finish and sometimes tip into freeze/crash.

## Practical fix direction

No code implemented in this pass, but fix should likely be this order:

1. Stop embedding icon data in initial folder listing path.
   - Return metadata first.
   - Resolve icons lazily or in a separate cached pipeline.

2. Replace fake paging with real paging/streaming.
   - Do not rescan/sort full directory for each page.
   - Keep one enumeration result or use streaming iterator state.

3. Remove `500`-row second burst.
   - Use smaller steady page size.
   - Prefer progressive append in tighter chunks.

4. Add stack-browser icon cache.
   - Reuse existing search/process-manager cache pattern.
   - Cache by normalized path or extension/type where acceptable.

5. Add instrumentation.
   - per-page duration
   - item count
   - icon resolution time
   - IPC payload size estimate
   - failure point logging for page 2+

6. Add large-folder regression coverage.
   - open 500+ item folder
   - assert first paint under target time
   - assert progressive growth beyond 80
   - assert no crash/freeze when many icons are present

## Evidence notes from live workspace

On this machine today:
- `C:\Users\jnev1\Downloads` currently contains `649` items.
- Top item groups are many folders, `.zip`, `.exe`, and `.rar` files.

That file mix is consistent with worst-case shell icon work for current Stack Browser load path.
