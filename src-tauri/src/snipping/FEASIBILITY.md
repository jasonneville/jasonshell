# Opt-in native snipping gate

## Current process-isolated clipboard owner

Known commitment/durability is preserved across redundant flush, helper shutdown,
unreachability and timeout classification. Normal flush returns committed/durable=true
immediately when already acknowledged, before stopped/helper/transport checks. Only
starting a new publication replaces the current acknowledgement tracking.

Runner regression hook:
`--consent-synthetic-clipboard-test --clipboard-test-case durable-redundant-flush-timeout`.
It first performs actual native publish/materialization, then arms a narrow parent
send-dependency fault for redundant flush and calls normal ProcessOwner::flush.
The corrected short circuit leaves the fault unused: metadata reports faultUsed=false,
shortCircuited=true, not an observed OLE/IPC timeout. Initial publication/protocol
validation is unaffected; fault state resets for a new publication. Helper shutdown
and writer drainage occur before driver exit; independent DIB+PNG reading follows.

Focused validation: `npm run cargo:check` passed. With temporary/restored explicit
synthetic consent, `npm run cargo:test -- --test snipping_clipboard_native
snipping_clipboard_known_durable -- --ignored --test-threads=1 --nocapture` passed
both regressions (0.20s): real publish->shutdown->flush preservation and armed unused
redundant-transport fault. Independent pixels survived helper/producer exit in both.
Runner owns the full current-patch thirteen-case suite; review gate remains leader-owned.

The reusable caller path is now `clipboard_process::ProcessOwner`, not synchronous
STA/OLE on the application thread. Rust selects a trusted bundled helper executable
path (`snip-clipboard-owner.exe` for packaging); the harness uses its own executable's
private helper mode with the same implementation. No normal shell startup wiring or
packaging integration is added. One ProcessOwner should be retained for app lifetime.
Its mutable API serializes one command at a time; there is no unbounded command queue.

The helper owns its STA and IDataObject. Pixels transfer through inherited anonymous
stdin pipe as a checked 40-byte binary header plus opaque RGBA payload, never JSON,
disk, temp image, command-line pixels, or logging. Stdout carries <=4096-byte metadata
reports only. Framed input has magic/version, checked length/dimensions, bounded
allocation, known opcodes/flags and reserved-byte validation. Retained caller storage,
overlapping helper input, PNG/DIB staging and local GetData construction remain
budgeted. Consumer/OS-owned copies remain outside the local producer budget.

Parent startup handshake/operation response deadline is 2s (after process spawn).
Shutdown allows 500ms for command/exit, then terminates only its retained child/job
handles and polls cleanup for at most 250ms. The private job has KILL_ON_JOB_CLOSE;
no OpenProcess, PID lookup, arbitrary process kill, COM thread termination, or blocking
join of an unfinished worker occurs. Pipe writers own no COM state; cancellation
targets only their retained thread handles, and joining requires is_finished.
Job assignment failure fails closed. Kernel/process creation/termination calls assume
a responsive Windows kernel; this is not a hard real-time OS guarantee.

Successful OleSetClipboard emits a committed acknowledgement before flush. Confirmed
flush yields committed/durable=true, and OS-owned data survives helper/producer exit.
A normal nondurable commitment keeps the live helper serving delayed data until
explicit flush or app shutdown. Uninterruptible helper operations cannot block the
parent beyond the operation/cleanup budget. On timeout/disconnection without a
commit acknowledgement, report publication-unknown with committed/durable=null,
never rejection or a preserved-sentinel promise. With acknowledged commitment but
no durable acknowledgement, terminate with committed-warning/durability-lost.
Shutdown follows the same policy. Superseding clipboard ownership does not erase the
historical commitment. Unknown non-busy native set failure is also conservative
unknown, not fabricated precommit rejection. Known validation/allocation/injected
precommit failure and real clipboard-busy rejection remain unchanged-content paths.

No app-level locks are held by this client. Eventual integration must authorize and
bind generation/caller before calling it; these are not renderer commands. Stopped
owners cannot be reused after a timeout, preventing late replies from reaching a
new publication. Startup and pending-state responses remain metadata-only.

### New runner-facing permanent-stall cases

Use existing explicit synthetic driver consent:

```powershell
snip-feasibility.exe --consent-synthetic-clipboard-test --clipboard-test-case helper-hang-operation
snip-feasibility.exe --consent-synthetic-clipboard-test --clipboard-test-case helper-hang-flush
snip-feasibility.exe --consent-synthetic-clipboard-test --clipboard-test-case helper-hang-shutdown
```

These stall the private helper permanently at the publishing, actual-postcommit/
preflush, or shutdown boundary. They are not assertions that Windows itself hung
inside COM. Metadata declares that distinction explicitly. Reports include
operationMs, shutdownMs, forcedShutdown, helperExited, writerDrained and outcome.
Runner should require exit=true, writerDrained=true, forcedShutdown=true and preserve
the distinction between unknown and acknowledged commitment. Operation driver bound
is 3000ms, shutdown bound 1000ms (includes scheduling/cleanup tolerance); core client
deadlines are tighter as above. No test artifact or deadline was weakened.

Observed standalone smoke before final handoff: unknown-operation timeout 2008ms;
acknowledged postcommit timeout 2010ms; forced shutdown 506ms. All private helpers
exited; no OS-COM-hang inference. Independent DIB+PNG after producer exit passed again
in 0.16s. Twenty success plus twenty precommit failure cycles returned helper-owned
objects/global allocations/staged bytes to zero; warmed handles stayed gdi0/user3/
processHandles147. Runner owns full six-case rerun and permanent-stall assertions;
independent source review remains leader-owned. Reserved-AppBar/manual gates stay open.

## Current clipboard driver (supersedes provisional clipboard notes below)

### Same-patch review fixes and remaining deadline blocker

HGLOBAL allocations are completely zero initialized, including allocator rounding.
Each Global records logical content length; GetData copies that length, never
GlobalSize padding. Actual GlobalSize still determines locally owned allocation
counters/budget. Returned STGMEDIUM allocations transfer to COM; they cease being
locally owned. Arbitrarily retained/multiple consumer copies and OS/OLE internal
storage are explicitly outside the 512 MiB local-producer budget. No process-wide,
system-wide or global-consumer allocation guarantee is claimed.

PNG encoding no longer calls the compression library. RGBA8/filter-0 bytes stream
into one exactly preallocated PNG Vec using zlib stored blocks (65535 payload bytes
per block), scalar Adler32/CRC32 state and a 13-byte IHDR array. For filtered payload
R=RGBA bytes+height, zlib length is R+5*ceil(R/65535)+6, PNG length is zlib+57.
There is no hidden encoder heap/scratch. Producer reservation includes retained input
and temporary Vec lengths; Vec capacity excess, prepared HGLOBAL actual sizes and
each in-progress GetData construction are separately charged. Opaque input is
required so PNG and BI_RGB DIB semantics agree. Uncompressed PNG can be larger but
is exact and budgeted before commit. Frozen input must still be truthfully reported.

Shutdown is explicit and idempotent: Durable, Superseded, or DurabilityLost. While
the owner remains alive, a nondurable committed image continues serving delayed
GetData. At explicit app shutdown, one final flush retry batch is attempted; on
failure local owner is released on its creating STA with a durability-loss warning.
It does not clear/restore clipboard, promise surviving delayed formats, or convert
the earlier Copy commitment to rejection. Drop applies that same loss policy and
uninitializes OLE; the previous infinite recovery loop is gone.

The synchronous Publisher is now helper-internal. Its individual native OLE calls
still have no interruptible deadline, but the process-isolated parent bounds response
and shutdown as defined above. It must not be called on an application/UI thread.

WM_NCDESTROY invalidates window ownership and clears userdata. Every native window
operation and destructor checks that exact stable-state binding, not IsWindow alone.
Standalone hidden-window check:
`snip-feasibility.exe --consent-synthetic-native --window-lifetime-check`.
It sends WM_CLOSE, checks stale operations reject, creates a replacement, drops the
old wrapper and checks replacement survives. Passed; observed handleReused=false
(not evidence of forced real HWND reuse). No visible window/capture/clipboard/shell
action occurs in this check.

After these fixes: cargo check passed; Rust core10 and Node core4 passed; independent
DIB+PNG reader-after-producer-exit test passed (0.12s). Runner owns the full six-case
native rerun and new sustained-flush-failure/deadline/padding/forced-reuse coverage.
Previous parent message-pump blocker was fixed by runner, who reported six cases
passed before this patch. Those six prior results are not current-patch full evidence.

The helper internally uses a synchronous, creator-thread STA Publisher; application
callers use the deadline-bound ProcessOwner above. DIB and PNG are prepared before
OleSetClipboard; that call is the commit point. Three bounded set/flush attempts are
used. `publish` returns rejection before commit or a committed durability Boolean;
false must be presented as committed-but-not-durable, never “nothing copied”. Owner
and STA stay alive for delayed GetData until flush materializes OS-owned data.

Readiness has no initialization/native effects:
`snip-feasibility.exe --clipboard-test-protocol`.
Synthetic-only cases use
`--consent-synthetic-clipboard-test --clipboard-test-case CASE`, with
`--hold-until-reader` required only for `flush-after-commit`. Known cases match
the runner contract; unknown/duplicate options fail closed. No visible windows,
desktop capture, shell startup, save, or upload occurs on these paths. JSON lines
are metadata-only and bounded to 4096 bytes. stdin observation is bounded to 5s;
the stdin reader never owns COM/image resources.

Aggregate reservation covers local producer staging against 512 MiB, with exact
PNG encoding/counter boundaries defined above. All existing frozen buffers must be
included in the caller's retained-byte argument.

Current process-path validation: cargo check and both helper/harness builds passed.
Targeted readiness, real busy sentinel preservation, delayed warning/live owner/
recovery, and independent DIB+PNG reader-after-producer-exit tests passed (0.02/0.08/
0.10/0.08s). Final permanent-stall smoke: operation unknown 2007ms, postcommit loss
2010ms, shutdown loss 508ms; all forced child exits and writer-drained observations
true, all exit 0. Existing pure14 were green before this P1-only change; not rerun
this round. Runner fixed its parent message pump without weakening deadlines and
owns the full six-case current-patch rerun plus new timeout assertions. Independent
review and reserved-AppBar/manual gates remain open.

Normal shell startup does not register these modules or launch this binary.
`default-run` remains `jason-shell`. Existing dependencies only.

Build/check: `npm run cargo:check`.

With explicit native-window consent:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --bin snip-feasibility -- --consent-synthetic-native
```

The bounded harness covers every enumerated monitor with a synthetic grid, verifies
physical capture in memory, changes the grid to check frozen cropping, shows an
excluded nonactivating preview, checks foreground HWND, then destroys its windows
and compares native monitor/work-area rectangles. No shell/AppBar/taskbar APIs,
files, image logs, uploads, or clipboard mutation in this default invocation.
One run takes a few seconds. Any mismatch fails the gate; do not proceed to full
feature integration until the leader investigates. Window classes, HWNDs, DPI
context, DCs, selected bitmaps and raw capture buffers have scoped ownership.

`--interactive` adds bounded 12-second overlay drag and 8-second preview-click
windows. Actual pointer evidence requires a person interacting during those periods;
style flags alone do not establish click usability or pointer-boundary behavior.

`--replace-clipboard-synthetic` explicitly replaces the clipboard with a 128x96
synthetic CF_DIB. It does not restore previous clipboard contents. Run only after
valuable clipboard content is removed and native consent is confirmed. An independent
reader and approved paste consumer are still needed to establish interoperability
and persistence after process exit. Publication is immediate, not delayed rendering;
failure after EmptyClipboard is not transactional.

Limits: dimension 16384; 33,600,000 pixels per monitor; all frozen Rust frames together
at most 256 MiB; frozen buffers plus one live GDI DIB at most 512 MiB. Crop checks
borrowed source plus output against 512 MiB. Clipboard callers must supply truthful
total retained bytes including the image, and its extra HGLOBAL is budgeted before
allocation/EmptyClipboard. Full integration must enforce aggregate stage ownership;
these primitives do not account for unrelated buffers held by other workers.

Initial live invocation on 2026-10-01 passed physical capture/grid, frozen crop,
nonactivating preview show, previous-preview exclusion and full-monitor overlay
bounds on one 2560x1440 monitor at scale 1.25. It FAILED the final work-area baseline
comparison. Native build-out stopped. Cause is not established. No clipboard was
changed. Shell/AppBar interaction, clickable preview, native pointer boundaries,
repeat/failure cleanup, command caller injection, external clipboard consumer,
negative-origin and mixed-DPI hardware remain unproven. The preview is intentionally
nonactivating: handoff line 131's “preview focuses” is not the approved contract.

## Follow-up work-area diagnosis

The same bounded command was rerun with numeric baseline, targets-visible,
overlay-visible and final-immediate monitor/work rectangles, thread DPI context,
and foreground HWND/PID/class/bounds. No titles or image contents are recorded.
When the immediate comparison fails, additional 100/250/500/1000ms observations
diagnose asynchronous restoration; they do not turn that failure into success.

Follow-up observation: baseline and final work rectangles were both
`(0,0)..(2560,1440)`, physical monitor unchanged, thread context `34` at both points,
and `AreDpiAwarenessContextsEqual(context, PER_MONITOR_AWARE_V2)` true throughout.
The bounded run exited 0 and repeated the capture/focus/exclusion passes. Unlike
the initial run's `(0,23)..(2560,1404)` baseline, the follow-up baseline had no reserved
work area. Thus it does not prove preservation/restoration of reserved AppBars.

Read-only process observation found an already-running `jason-shell` PID 6724;
foreground PID 26252 was OpenChamber, restored by Windows after target destruction.
No shell was launched or controlled. In `appbar.rs`, an external foreground window
covering the monitor matches fullscreen detection; the guard polls every 250ms and
`hide_shell_for_fullscreen` unregisters AppBars and sets full-monitor work area.
The standalone synthetic target/overlay meets that geometry predicate. This is a
source-backed external movement mechanism, not proof of the historical first-run
cause: its final rectangle and DPI context were not recorded. No comparison or
window-placement source defect was established; do not patch shell behavior on
this evidence alone. Leader must coordinate a reserved-baseline shell observation.

## Historical pre-OLE clipboard contract blocker (superseded above)

Current explicit EmptyClipboard/SetClipboardData publication is provisional and
must not become the final product Copy implementation. Allocation, validation and
OpenClipboard rejection precede clearing, but a SetClipboardData failure can leave
the old clipboard cleared. This is insufficient for the approved rejection contract.

A viable implementation candidate follows the existing `speech_clipboard.rs`
STA/OLE ownership precedent: stage a bounded immutable image IDataObject exposing
CF_DIB (and optionally separately budgeted PNG); validate generation/caller and
win pending-to-publishing cancellation before OleSetClipboard; do not explicitly
EmptyClipboard. Treat successful OleSetClipboard as commitment, retaining the
object in its apartment while serving delayed requests. A rejected publication
must be independently tested against an unchanged synthetic sentinel (including
clipboard-busy and controlled precommit failure). Do not assume arbitrary clipboard
restoration or label postcommit errors as rejection.

OleFlushClipboard must establish materialized persistence before process exit.
If flush fails after commit, report committed-but-not-durable, keep the owner/message
pump alive, and retry within a defined shutdown policy rather than claiming a
precommit failure. GetData HGLOBAL copies, staged image storage and overlapping
previous committed owner must be included in aggregate allocation accounting.
Independent Win32 reader, approved consumer paste, busy sentinel preservation and
reader-after-process-exit checks are required before adopting this candidate.
This replacement has not been implemented or run; clipboard remained untouched.
