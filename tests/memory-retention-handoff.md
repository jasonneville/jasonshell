# Frontend memory-retention RED handoff

Scope: test/benchmark artifacts only. Production, process-sort/native runner scopes,
global baselines and dependencies unchanged. No live Windows/Tauri app launch.

## Repeat exactly

```powershell
npm run test:component -- tests/components/memory-retention.test.ts tests/components/snip-memory.test.ts tests/components/command-memory-runtime.test.ts tests/components/snipping-product.test.ts tests/components/command-panel.test.ts
```

Final observed result: 53 pass, 8 expected RED,5 files. No collection/setup errors.
Raw local output: `C:\Users\jnev1\AppData\Local\Temp\opencode\memory-red-final.txt`.
The tests print JSON fixture metrics on every run; rerun unchanged after the fix
for directly comparable retained-object evidence. These are NOT app RSS, heap
profiler, native allocation, or frame-time claims.

## Established policy and acceptance

- Native `quick_commands.rs`: `HISTORY_LIMIT=20`, `TRANSCRIPT_LIMIT=256`.
  `list_quick_command_history` combines runtime and saved history, optionally
  filters command ID, sorts newest start/finish first, then truncates20. Normal
  renderer requests omit ID: its native history window is global, not20/command.
  Saved completed history is also capped globally20. Active native list rows can
  fall outside that cap. Renderer must preserve genuinely active known runs for
  Stop/input safety in addition to recent20 completed runs; never infer completion
  merely from absence in the list. No deletion/stop policy is being invented.
- Transcript native deque discards oldest rows after256. Renderer event union
  currently defeats this bound and poll convergence never removes old rows.
  Keep exact most recent sequenced256, deduplicate/reconcile repeated rows, and
  reject late rows below that window. Polls must not resurrect terminal runs or
  retired histories. Use bounded chronology/watermark/state, not unbounded ID
  tombstones. Tests do not dictate implementation architecture.
- Active pending control metadata may need independent bounded ownership when its
  request row ages out of transcript. The mounted fixture emits a pending request,
  then600 later output events, then24 completed runs and a recent20 poll omitting
  that active run. Input prompt/identity/maxLength, selected command, and enabled
  Stop must survive. Submit must target exact command/run/PID/request. Terminal
  stopped/exit must clear controls; old prompt/event/snapshot cannot reactivate.
  Native snapshot only includes retained transcript, not a separate pending
  request field: preserve live known pending metadata safely; a cold mount after
  native eviction has no such metadata and is NOT certified by this fixture.
- Mounted journey uses production `CommandPanelSurface.svelte` and mocked Tauri.
  Select command, Previous runs,24×600 output+exit, delayed old running response,
  current native256-row snapshots, late evicted events. Expect20 run cards and
  5120 transcript rows, first body `row-346`, terminal `exit` last. No retired PID
  101–104, no false Stop for completed stream, selection/tab coherent.
- Snip transport currently accepts Uint8Array/ArrayBuffer, rejects number[] and
  empty payloads. Preserve that contract. Full and offset/length subviews,
  including SharedArrayBuffer-backed views, must produce byte-identical PNG Blob
  payload without explicit `bytes.slice()` allocation. Preserve offset/length;
  whole backing buffer would leak sentinel bytes and fail tests. Shared backing
  must remain TypeScript-safe. Required Blob snapshot copy is NOT an optimization
  target or counted as removable copying.
- Snipping existing product cases cover unmount/replacement/stale responses,
  previous URL release, and no revocation of replacement URL by stale operations.
  Keep them green; no need to rewrite already-correct URL ownership.

## Deterministic retained-data metrics

| Fixture | Before | Required after |
| --- | --- | --- |
| 600 sequenced rows, native convergence/replay | 1run /600rows /6000body chars | 1/256/2560 |
| 40completed×601 rows +1active prompt | 41runs /24041rows /240410chars | 21/5121/51210 |
| Mounted24completed×601 | 24runs /14424rows /98232chars | 20/5120/35720 |
| Full Uint8Array | 1 explicit slice /10bytes | 0/0 |
| Offset subview (normal/shared) | 1 explicit slice /6bytes | 0/0 |

Numbers measure retained transcript body only, not stdout/stderr duplication or
JavaScript object overhead. Polls are explicitly controlled, not wall-clock driven.
No memory-usage improvement percentage is claimed. Browser/hardware/native shell
journeys remain unrun; explicit human consent required before live shell smoke.

## Implementer gate

Designer/fixer: change only assigned production files; do not weaken tests. All
new retention, controls, byte-preservation and established lifecycle tests above
must pass. Runner then reruns focused checks and records new JSON beside these
before values. Independent code review remains leader-owned; no nested delegation.
Search guidance: prefer scoped `fff.grep`/`fff.multi_grep`, filenames via
`fff.find_files`; indexed empty results are not proof of absence. Forward this
guidance on any handoff. No broad rediscovery needed.
