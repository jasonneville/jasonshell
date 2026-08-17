---
date: 2026-08-15
status: Implemented; release acceptance pending 10-minute interactive churn
requirements: [P03-FR1, P03-FR2, P03-FR3, P03-FR4, P03-FR5, P03-FR6, P03-NFR1]
depends_on: [Plan 01]
recommended_after: [Plan 02]
scope: Bound native icon caches and measure memory growth
---

# Plan 03: Native Cache and Memory Bounds

## Context and evidence

Uncaptured local diagnostic observation from 2026-08-15, with no durable artifact: current dev process tree showed 19 app/WebView processes, about 1.58 GB working set, about 1.22 GB private bytes, and 523 threads. Vite showed 0.65-1 GB private bytes and about 61.5k handles. This is not evidence and requires Plan 01 remeasurement before acceptance.

Several native icon caches appear unbounded or insufficiently bounded. This plan targets task-window encoded icon cache plus existing icon caches in exact feature files.

## Preflight

- Read `master_spec.md` before changing files.
- Inspect current `git status` and preserve unrelated dirty work.
- Confirm Plan 01 stop/go artifact exists: `test-results/performance-regression/<timestamp>/summary.md` with valid release baseline, or stop.
- Inspect listed cache symbols/files before editing; if moved, document plan amendment before implementation.

## Requirements

- P03-FR1: Icon caches MUST be bounded by TTL and LRU-like capacity without adding a dependency if practical.
- P03-FR2: Caches MUST store encoded strings, not `HICON` handles.
- P03-FR3: Cache identity/invalidation MUST preserve per-window icon correctness.
- P03-FR4: Cache misses and extraction MUST happen outside locks.
- P03-FR5: Negative caches MUST be safe and short TTL only.
- P03-FR6: Persistent WebView memory MUST be measured before lifecycle changes.
- P03-NFR1: Lazy-window creation is a separate follow-up and MUST NOT be automatically included.

## Exact file/symbol impact

- `src-tauri/src/task_windows/icons.rs::window_icon_data_url`.
- `src-tauri/src/process_manager.rs` icon cache helpers; inspect exact declarations before editing.
- `src-tauri/src/stack_popup/icons.rs` icon cache helpers; inspect exact declarations before editing.
- `src-tauri/src/search/icons.rs` icon cache helpers; inspect exact declarations before editing.
- Proposed helper: local bounded cache helper module if existing utilities are insufficient; determine path during implementation after inspecting current Rust module layout.
- Existing tests to inspect: `tests/processManagerWiring.test.mjs`; taskbar/search/stack icon source tests if present.
- Inspect-for-moved-symbol rule: search declarations/usages first; if symbol/path moved or renamed, update this plan via documented amendment before implementation.

## Phased tasks

| ID | Task | Depends on | Definition of done |
|---|---|---|---|
| P03-T1 | Inventory current icon cache keys, values, lock scopes, and invalidation assumptions. | Plan 01 | Inventory maps each cache to correctness risk. |
| P03-T2 | Add RED-first tests for bounded cache eviction/TTL and extraction outside locks. | P03-T1 | Tests fail before helper/cache changes. |
| P03-T3 | Implement no-new-dep TTL/LRU-like helper or adapt existing helper. | P03-T2 | Helper supports capacity, TTL, negative TTL, and encoded string values. |
| P03-T4 | Migrate task-window/process/stack/search icon caches. | P03-T3 | All target caches bounded; no `HICON` retention. |
| P03-T5 | Run 10-minute churn memory slope evidence. | P03-T4 | Report release slope, cache size, private/WS deltas. |

## RED-first tests

- Proposed: Rust unit tests for bounded helper eviction, TTL expiry, negative TTL, and no lock during miss callback.
- Existing focused: `cargo test --manifest-path src-tauri/Cargo.toml task_windows`; `node --test tests/processManagerWiring.test.mjs` if frontend contracts are touched.

## Acceptance criteria

- Given icon churn exceeds cache capacity, when new entries are inserted, then older/expired entries are evicted and memory does not grow unbounded.
- Given a window icon changes identity, when a snapshot refresh occurs, then cached icon identity does not show an incorrect per-window icon.
- Given extraction misses, when the extractor runs, then cache mutexes are not held.
- Given 10-minute churn test runs in release mode, then private bytes and working set slope are reported against Plan 01 baseline.

## Performance evidence path

Use Plan 01 release schema plus 10-minute churn scenario. Measure process private bytes, working set, threads, handles, cache counts, and slope. Dev Vite metrics may be recorded only as diagnostic context, never acceptance evidence.

## Stop/go gate

- Go only if cache identity correctness is preserved and release 10-minute churn slope is non-regressive against Plan 01.
- Repo validation (`npm run check`, focused Rust/Node tests, `npm run validate`) is separate from scenario/perf acceptance.
- If churn prerequisites are unavailable, mark scenario `blocked/not measured`; do not silently pass/fail unrelated repo validation.
- Plan cannot close until required release scenarios are measured or user explicitly accepts documented limitation.

## Rollback

Revert cache migrations to previous per-feature caches if correctness regresses. Keep bounded helper tests as evidence for next design if helper is sound.

## Risks

- Over-aggressive eviction can cause visible icon flicker or repeated shell extraction.
- Weak identity keys can show wrong icons for multiple windows of same executable.
- Negative caching can mask transient shell icon availability.

## Required implementation docs updates

- `master_spec.md`: document bounded cache policy and memory evidence only after implementation.
- `changelog.md`: add implementation and validation bullets.

## Validation commands

- Focused: `cargo test --manifest-path src-tauri/Cargo.toml task_windows`; `node --test tests/processManagerWiring.test.mjs` if touched.
- Full: `npm run check`; `npm run cargo:test`; `npm run validate`.

## Invocation

`Implement Plan 03 from docs/plans/performance-regression/03-native-cache-and-memory-bounds.md. Preflight: read master_spec.md; inspect git status; preserve unrelated dirty work; confirm Plan 01 stop/go artifact. Follow RED-first tests, stop/go gates, validation, master_spec/changelog rules.`
