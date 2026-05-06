# Backend Blocking Lock Boundaries P6

Status: Draft, implementation not started  
Source: `findings.md`, Suggested Fix Order item 6  
Priority: P6, run after `shell_popup_layout_scroll_p5.md`

## Goal

Remove backend IPC stall risks from blocking file operations, slow shell icon extraction under locks, and task preview window operations under runtime mutexes.

## Issue

Findings show:

- Stack archive extraction and recursive file operations can block Tauri command threads.
- Process Manager holds icon-cache mutex while doing shell icon extraction.
- Task Preview holds runtime mutex while performing Tauri window operations.

Recent Stack Browser icon work already provides the target pattern: short lock for cache lookup/store, expensive shell work outside lock, and `spawn_blocking` for blocking native work.

## Phase 1: Stack Long-Running File Operations

Acceptance criteria:

- Archive extraction does not block the async command thread.
- Recursive copy/move/delete work runs behind `tauri::async_runtime::spawn_blocking` or an equivalent job boundary.
- Existing user-visible semantics and error messages remain stable unless a job model is explicitly introduced.
- Cheap IPC remains responsive during a large fake tree operation.

Tests to write:

- Rust source/behavior test proving archive `.status()` is not called on the command path without `spawn_blocking`.
- Rust/source tests for recursive file-op functions in `stack_popup/file_ops.rs`.
- Integration or deterministic fake test: start large operation, call a cheap command, cheap command returns within a bounded time.
- Error regression for partial failure/no panic.

Implementation tasks:

- Wrap blocking archive extraction in `spawn_blocking`.
- Wrap recursive copy/move/delete operations in `spawn_blocking`.
- Consider a job/progress/cancel model only if implementation would otherwise leave UI blind for long operations; keep as a later extension if too broad.
- Preserve path validation and existing command response contracts.

Subagents to run:

- `stack-file-job-test-worker`: use `rust-skills`, `tdd-guide`.
- `stack-file-job-impl-worker`: use `rust-skills`, `senior-backend`.
- `stack-file-job-adversarial-reviewer`: use `adversarial-reviewer`.

Validation gate:

- `cargo test --manifest-path src-tauri/Cargo.toml stack_popup`
- Focused Node Stack Browser tests if IPC wrappers change.
- `npm run check`

## Phase 2: Process Manager Icon Cache Lock Split

Acceptance criteria:

- Process icon cache lookup and store use short mutex scopes.
- Shell icon extraction happens outside the cache mutex.
- Concurrent process refreshes do not serialize all icon cache access behind a slow miss.
- Cached icons are still reused.

Tests to write:

- Rust source contract proving the shell extraction call is outside the live cache guard.
- Rust unit/concurrency test with a fake slow extractor if code is refactored to allow injection.
- Regression test for cache hit reuse.

Implementation tasks:

- Split helpers into lookup, resolve miss, and store phases.
- Mirror `src-tauri/src/stack_popup/icons.rs` lock discipline.
- Clone only the key/path data required to drop the lock before extraction.

Subagents to run:

- `process-icon-cache-test-worker`: use `rust-skills`, `tdd-guide`.
- `process-icon-cache-impl-worker`: use `rust-skills`, `senior-backend`.
- `process-cache-adversarial-reviewer`: use `adversarial-reviewer`.

Validation gate:

- `cargo test --manifest-path src-tauri/Cargo.toml process_manager`
- `cargo check --manifest-path src-tauri/Cargo.toml`

## Phase 3: Task Preview Runtime Mutex Boundary

Acceptance criteria:

- No `MutexGuard` remains live across Tauri window calls such as `emit`, `set_position`, `show`, `set_focus`, or `hide`.
- Stale hover requests remain rejected.
- Rapid hover/leave cannot revive an old preview.
- DWM thumbnail handle lifecycle remains correct.

Tests to write:

- Rust unit for request A, hide, request B freshness.
- Source contract banning a runtime mutex guard across Tauri window operations.
- Manual smoke: rapid hover across task tiles.

Implementation tasks:

- Copy needed freshness state while locked.
- Drop lock before Tauri window operations.
- Reacquire only for post-call state updates or error bookkeeping.

Subagents to run:

- `task-preview-lock-test-worker`: use `rust-skills`, `tdd-guide`.
- `task-preview-lock-impl-worker`: use `rust-skills`, `senior-backend`.
- `preview-lock-adversarial-reviewer`: use `adversarial-reviewer`.

Validation gate:

- `cargo test --manifest-path src-tauri/Cargo.toml task_preview`
- Task preview Node/source tests if touched.
- `npm run check`

## Phase 4: Backend QA

Acceptance criteria:

- No new broad async job model is introduced without tests for cancellation/error behavior.
- No new lock is held across blocking shell or Tauri operations.
- Full validation is green or exact unrelated blocker is recorded.

Tests to write:

- Only as needed for QA-discovered gaps.

Implementation tasks:

- Run adversarial review with Saboteur focus on deadlocks, starvation, partial file failures, and stale preview state.

Subagents to run:

- `backend-blocking-adversarial-reviewer`: use `adversarial-reviewer`, `rust-skills`.

Validation gate:

- `npm run validate` when feasible.

