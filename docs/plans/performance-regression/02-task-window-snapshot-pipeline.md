---
date: 2026-08-15
status: Implementation complete; release scenario comparison pending/manual blocked
requirements: [P02-FR1, P02-FR2, P02-FR3, P02-FR4, P02-FR5, P02-FR6, P02-FR7, P02-NFR1]
depends_on: [Plan 01]
scope: Replace duplicate task-window polling with one authoritative Rust snapshot producer
---

# Plan 02: Task Window Snapshot Pipeline

## Context and evidence

The strongest suspected root cause is duplicate steady task-window work: TopBar and BottomBar directly scan at 1 Hz. The remediation must create one authoritative Rust snapshot producer/event stream with sequenced payloads and last-snapshot bootstrap/fallback. `TASKBAR_REFRESH_WINDOWS_EVENT` becomes request-refresh-soon semantics, not a frontend polling trigger.

## Preflight

- Read `master_spec.md` before changing files.
- Inspect current `git status` and preserve unrelated dirty work.
- Confirm Plan 01 stop/go artifact exists: `test-results/performance-regression/<timestamp>/summary.md` with valid release baseline, or stop.
- Inspect symbols before editing; if listed symbols moved, document amendment before changing source.

## Requirements

- P02-FR1: Rust MUST own one authoritative task-window snapshot producer/event stream.
- P02-FR2: Snapshot payloads MUST be sequenced and provide last-snapshot bootstrap/fallback.
- P02-FR3: `list_open_task_windows` MUST remain compatible as cached/read-last snapshot or explicit fallback, not steady poll.
- P02-FR4: `TASKBAR_REFRESH_WINDOWS_EVENT` MUST mean request refresh soon.
- P02-FR5: Notification app-id to install-path lookup MUST use bounded TTL and short negative cache.
- P02-FR6: Per-scan PID CPU data MUST be reused within a scan.
- P02-FR7: Snapshot producer MUST NOT hold mutexes across `EnumWindows`, WinRT, icon extraction, or Tauri emit.
- P02-NFR1: Preserve all-monitor eligibility, `TaskbarWindow` shape, stable order, active/minimized/activity/notification semantics.

## Anti-design

- Do not merely move duplicate polls to TypeScript.
- Do not combine a Rust producer with permanent frontend polls.

## Exact file/symbol impact

- `src/components/TopBar.svelte::searchRefreshTimer`.
- `src/components/BottomBar.svelte::taskbarPollTimer` and `src/components/BottomBar.svelte::refreshTaskbarWindows`.
- `src/lib/taskbarWindows.ts::listOpenTaskWindows`.
- `src-tauri/src/task_windows/mod.rs::list_open_task_windows`.
- `src-tauri/src/task_windows/windows.rs::list_open_task_windows`.
- `src-tauri/src/task_windows/notifications.rs::{notification_count_for_process_path,app_install_path}`.
- `src/lib/taskbarUi.ts::TASKBAR_REFRESH_WINDOWS_EVENT`.
- `src-tauri/src/contracts.rs` event registry if new snapshot event is native-emitted.
- Existing tests to inspect/update: `tests/taskbarWindows.test.mjs`, `tests/taskbarUxState.test.mjs`.
- Inspect-for-moved-symbol rule: search declarations/usages first; if symbol/path moved or renamed, update this plan via documented amendment before implementation.

**Documented amendment:** pre-implementation symbol inspection found the task-window helpers already moved into `src-tauri/src/task_windows/windows.rs` and `src-tauri/src/task_windows/notifications.rs`; this plan now records the current paths.

## Phased tasks

| ID | Task | Depends on | Definition of done |
|---|---|---|---|
| P02-T1 | Capture current task-window command/event contract and Plan 01 baseline artifacts. | Plan 01 | Current semantics listed before changes. |
| P02-T2 | Add RED-first tests for no duplicate steady polls and sequenced snapshot bootstrap. | P02-T1 | Tests fail against current direct polling design. |
| P02-T3 | Implement Rust snapshot producer with short mutex scopes. | P02-T2 | Producer emits sequenced snapshots and stores last snapshot. |
| P02-T4 | Convert frontend consumers to subscribe/read last snapshot and request refresh soon. | P02-T3 | No permanent 1 Hz frontend polls remain for task windows. |
| P02-T5 | Add notification lookup TTL/negative cache and per-scan PID CPU reuse. | P02-T3 | Cache is bounded; misses happen outside locks. |
| P02-T6 | Validate semantics and compare release metrics to Plan 01. | P02-T4, P02-T5 | Taskbar behavior preserved; steady scan work reduced. |

## RED-first tests

- Proposed: `tests/taskWindowSnapshotPipeline.test.mjs` asserting frontend subscription/request-refresh semantics and no permanent duplicate polling.
- Existing: `tests/taskbarWindows.test.mjs`; `tests/taskbarUxState.test.mjs`.
- Rust focused: `cargo test --manifest-path src-tauri/Cargo.toml task_windows`.

## Acceptance criteria

- Given TopBar and BottomBar are mounted, when task windows update, then both consume the same sequenced snapshot stream.
- Given frontend emits `TASKBAR_REFRESH_WINDOWS_EVENT`, when Rust receives it, then it schedules/requests refresh soon instead of causing duplicate steady frontend polls.
- Given `list_open_task_windows` is invoked, when a recent snapshot exists, then it returns compatible cached/read-last data unless explicit fallback is needed.
- Given enumeration runs, when icon extraction or emit occurs, then no task-window mutex is held.

## Performance evidence path

Compare Plan 01 release scenarios: idle, 20+ windows, notifications, multi-monitor. Evidence must show reduced redundant scan/lookup work without regressions in CPU/thread/handle/control I/O medians.

## Stop/go gate

- Go only if taskbar semantics pass and Plan 01 release metrics show reduced steady scan/lookup work or no regression with documented rationale.
- Repo validation (`npm run check`, focused Node/Rust tests, `npm run validate`) is separate from scenario/perf acceptance.
- If notification or multi-monitor prerequisites are unavailable, mark affected scenario `blocked/not measured`; do not silently pass/fail unrelated repo validation.
- Plan cannot close until required release scenarios are measured or user explicitly accepts documented limitation.

## Rollback

Emergency rollback may stop rollout or revert implementation commit to prior behavior only temporarily. Retain failing regression tests and evidence. Do not prescribe known-bad direct-poll architecture as accepted final rollback, and do not declare plan complete until redesigned and revalidated. Mention/use feature flag only if implementation genuinely adds one.

## Risks

- Stale bootstrap snapshots could show old active/minimized state if sequence handling is wrong.
- Notification lookup caching could hide newly installed apps if TTL too long.
- Event fanout can race with frontend startup unless last-snapshot bootstrap is robust.

## Required implementation docs updates

- `master_spec.md`: document authoritative snapshot producer, event semantics, `list_open_task_windows` compatibility, and cache bounds.
- `changelog.md`: add concise implementation and validation bullets.

## Validation commands

- Focused: `node --test tests/taskbarWindows.test.mjs tests/taskbarUxState.test.mjs tests/taskWindowSnapshotPipeline.test.mjs`; `cargo test --manifest-path src-tauri/Cargo.toml task_windows`.
- Full: `npm run check`; `npm run cargo:check`; `npm run validate`.

## Invocation

`Implement Plan 02 from docs/plans/performance-regression/02-task-window-snapshot-pipeline.md. Preflight: read master_spec.md; inspect git status; preserve unrelated dirty work; confirm Plan 01 stop/go artifact. Follow RED-first tests, stop/go gates, validation, master_spec/changelog rules.`
