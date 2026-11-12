# Remaining operational native acceptance seams

These are requirements, **not passing tests or a substitute executor**. Runner owns implementation of the eventual test artifacts. Native owner owns production seams. At the continuation's source inspection, `system_info.rs` supplied pure types/accounting; `native_runtime.rs` supplied scheduling/cache state; the referenced `volume_probe.rs` had not yet been created. Keep hardware/Tauri-shell startup excluded.

## Actual probe executor

Exercise the same executor and `ProbeBudget` that production volume collection uses. A native test-only command/child factory may inject an absolute **test-owned** helper executable and fixed modes; never add renderer executable/path/argument overrides. No shell, provider, installer, inherited console or real user-process termination. The helper must self-expire even when the parent assertion fails; parent cleanup must kill then wait its own child.

Required controls/observations:
- Start/ready handshake supplies the child's PID, with at most a two-second setup bound.
- Fixed modes: valid bounded JSON, malformed JSON, nonzero exit, stalled stdout/never exit, and stdout flood beyond the production cap.
- Observe actual child exit/reap separately from kill request; an executor wait barrier or production-used injected child wrapper may hold `wait` pending so slot retention is deterministic. Do not duplicate kill/wait implementation in the test.
- Inject monotonic deadline/cancellation only through native test construction, or use the real 1500ms production deadline with an outer hard bound and generous CI overhead. No timing-only inference of reaping.

Required cases:
1. Reserve two distinct roots; third root and duplicate root denied. Start one stalled helper. Retire its epoch and reopen. Kill must be requested, old data never published, and its slot/root must remain unavailable **until executor wait/reap completes**. Another independent slot/lane can progress. Release the wait barrier; the same slot becomes available once, not twice.
2. At 1499ms no deadline kill; at 1500ms kill requested once. Production must invoke child `kill`, then `wait`, then budget `reaped`; never `reaped` on request, read failure, cancellation or timeout alone. A failed kill must still lead to bounded wait/retry ownership, not a released live-child slot.
3. A helper holding stdout open cannot block the lane past its deadline. Flood output is capped in allocation/bytes read, reported truthfully unavailable/truncated, killed and reaped; no unbounded `read_to_end` or detached reader survives epoch retirement.
4. Malformed JSON/nonzero exit yield sanitized query failure and no partial arbitrary payload; normal bounded JSON returns distinct caller-available vs total-free bytes.
5. Completion after tab switch/hide/reopen cannot publish. Deadline/epoch invalidation cannot create >2 owned live children or >1/root. Repeated opens cannot accumulate retired-but-unreaped workers.

Budget-only assertions already exist in `src-tauri/tests/system_info_acceptance.rs`; they do not establish these executor/OS properties. A real helper test should verify its PID is signaled/exited (or no longer openable as live) after executor completion, not trust only a returned `reaped: true` flag.

## Cache read seam

Current IPC handler directly clones the cache under an immutable lock; Node source checks cover that call path only. For executable no-hardware-enqueue evidence, expose a native-only method used by the actual IPC handler, e.g. `RuntimeInner::read_snapshot(generation,revision,after_revision,visible,focused) -> Result<Option<SystemInfoSnapshot>,_>`. Keep the pure cache/state portion independently includeable without Tauri startup. Do not introduce a parallel test cache model.

Tests will seed one warming/current cache and hold lane admissions outstanding, then repeatedly read with omitted/old/equal/new revision: old returns a detached clone, equal/new returns `None`; read count cannot change cache revision, deadlines, probes, admissions or collector-call counters. Invalid epoch, inactive tab, hidden/unfocused state and unsafe revisions deny **before work**. Holding a fake collector pending must not block cache reads. Clone mutation cannot affect the stored cache. Warming cache must exist immediately after activation, and prior-epoch cache must be inaccessible after hide/open/tab changes.

## Concrete-instance authorization limit

Pure `authorize_manager` tests establish short-circuit label filtering and sanitized denial; scoped Node tests establish actual handler order and the HWND comparison adapter. They cannot operationally certify live Tauri window identity. A native-only injectable registry/identity reader used by that **actual adapter** can enable same-label/different-identity, missing registry, destroyed/replaced window, and failed-identity lookup tests without creating windows. The existing boolean predicate seam alone does not expose that comparison. Live hidden/visible Tauri windows remain outside this run's consent/safety boundary.

## Browser limit

Production-mounted fixture remains required. Only visible `wmux browser` is authorized; `wmux` is absent. Fixture bundling does not establish E2E. No invisible fallback or live shell launch.
