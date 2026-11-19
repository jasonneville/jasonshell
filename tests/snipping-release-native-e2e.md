# Snipping release native evidence

Bounded evidence handoff for the clean ordinary release `SingleDisplay` journey.
This records runner evidence; it is not a new execution log from the docs lane.

## Release and command

- Executable: `C:/dev/jasonshell/src-tauri/target/release/jason-shell.exe`
- Size: `46,894,592` bytes
- Modified: `2026-11-11 01:58:21`
- SHA-256: `4A191311426B1719F71ADAC9352F690DC85E757FE26469D34E6B071E52C6D407`
- PowerShell command (restores the exact prior process environment value):
  ```powershell
  $previous = [Environment]::GetEnvironmentVariable('JASONSHELL_SNIP_RELEASE_E2E_TESTS', 'Process')
  $hadPrevious = $null -ne $previous
  try {
      $env:JASONSHELL_SNIP_RELEASE_E2E_TESTS = '1'
      & pwsh -NoProfile -File tests/snipping-release-native-e2e.ps1 -AcceptanceProfile SingleDisplay
  } finally {
      if ($hadPrevious) {
          $env:JASONSHELL_SNIP_RELEASE_E2E_TESTS = $previous
      } else {
          Remove-Item Env:JASONSHELL_SNIP_RELEASE_E2E_TESTS -ErrorAction SilentlyContinue
      }
  }
  ```
- The embedded Common Controls v6 resource passed through standard SDK metadata
  extraction only. The executable had no temporary profiling hooks/instrumentation,
  trace, debugger, or CDP instrumentation; production event routing, concurrency,
  DPI, shadow, compact-transport code, and the fixture's scoped WinEvent hooks
  were present.

## Two fresh-process results

| Run | Button-up → first visible | Button-up → exact overlay | Selection / preview / dismiss |
|---|---:|---:|---|
| 1 | 779−144 = 635 ms | 801−144 = 657 ms | 816–1210 ms / 1595–1597 ms / 1789 ms |
| 2 | 709−130 = 579 ms | 728−130 = 598 ms | 742–1126 ms / 1468–1471 ms / 1637 ms |

Both runs verified exact `3840×2160` outer and client coverage at DPI 288,
capture exclusion, nonactivation, protected post-click bar-baseline
preservation, real protected selection, a `2880,54,960,792` preview, and
dismissal. Hidden MSCTFIME UI helper class 2 was verified with identity checks;
the probe used verified identities, not a generic own-window whitelist.
Clipboard verification found DIBV5 format 17, 288×288 32bpp, with sequence
238→247 and 247→256. Both runs completed with cleanup, normal owned-app and
fixture closure, WinEvent unhook, zero helpers, zero debug listeners,
environment restoration, absent trace output, and unchanged executable hash.

The visibility observation alone is not a readiness ACK; accepted selection and
preview establish the downstream journey. These two runs establish bounded
single-display repeatability, not an instantaneous-start guarantee, a general
performance budget, compact-payload measurement, pixel-equality proof, or
consumer-paste proof. The earlier instrumented 1061 ms observation is from a
different build and is not a controlled speed comparison. Work-area restoration
was not independently measured.

### Intermittent preview follow-up (pending separate consent)

Use a fresh release build from the fix checkout; the harness launches
`src-tauri/target/release/jason-shell.exe` relative to that checkout. Use the
`SingleDisplay` procedure above as the geometry and lifecycle baseline. The
script checks DIB metadata only; add a controlled manual
pass on the same build for content. Show a synthetic image at 1:1 physical pixel
scale with distinct color blocks and an asymmetric marker. Snip a known rectangle
whose size and expected content are recorded in advance. Check the actual
top-right thumbnail for the expected crop, orientation, and marker (not just
window visibility). Without clicking **Copy**, paste into a separate image
consumer and compare its dimensions and pixels against the expected crop. Repeat
with a visibly different crop to rule out a stale thumbnail or clipboard image.
Report unavailable thumbnails and paste mismatches independently. Use only
synthetic content; this pass changes the live clipboard and requires consent.

## Limits of the recorded release evidence

Normal component coverage (52) and codec coverage (11 passed, 1 ignored) were
bounded evidence, not a full-repository green result. For that run, the
coordinator/clipboard targets compiled, but ordinary integration execution was
blocked before test entry by `0xc0000139` from the missing Common Controls v6
resource. Standard-SDK temporary copies covered the coordinator
suite (43, including all 10 image cases) and clipboard/native suites (21 + 13
native ignored), with original/copy text byte-identical; copies were deleted.
The strict `LegacyDual` two-monitor profile is excluded from this evidence.

## Stack visibility regression evidence

The button regression was reproduced without launching the shell: native Stack
blur previously hid the popup directly without publishing `stack-popup:closed`.
TopBar retained `stackBrowserOpen=true`; its generic pointerdown dismissal called
the native toggle, which reopened the hidden popup with its retained folder
request before snipping started. An intentionally open popup instead received an
unwanted dirty-editor close request. The legacy focus-hold restore path was not
needed to reproduce this failure; this evidence does not identify it as the cause.

`tests/snippingStackVisibility.test.mjs` initially executed the actual extracted
production pointer/start handlers against the verified native toggle contract:
2 passed, 4 failed. Blur-closed Stack became visible (expected false, actual true);
open Stack received one close request (expected zero); repeated snips received
three, and retry captured reopened Stack. These are handler/adapter observations,
not live HWND or captured-pixel observations.

The fix exempts the snip button (including its icon descendants) from generic
pointerdown dismissal, uses the existing toggle command's `closeOnly: true`
option for ordinary outside dismissal, and routes accepted native Stack blur
through shared hide cleanup/closure publication. Hidden close-only requests are
no-ops; visible requests retain the dirty-editor guard. Ordinary launcher toggles
and pinned-folder pointer/focus-hold paths retain their separate behavior.

Focused reproducible automated commands (no live shell):

```powershell
node --test tests/snippingStackVisibility.test.mjs tests/stackBasicTextEditorUi.test.mjs tests/topBarFolderReorder.test.mjs
npm run test:component -- tests/components/stack-close-guard.test.ts
pwsh -NoProfile -File tests/snipping-release-profile.test.ps1
pwsh -NoProfile -File tests/snipping-release-focus.test.ps1
npm run validate
```

The regression fixture now executes both production IPC adapters, including the
actual `closeOnly` argument, and distinguishes hidden no-op, visible guarded
close, ordinary toggle, nested snip icon, and pin/Stack pointer targets. Native
branch ordering and accepted blur cleanup have source-contract checks, not
executed Tauri lifecycle evidence.

Focused GREEN: all 49 Node checks in the three-file command above passed,
including 14 Stack/snipping regression checks; the mounted dirty-close guard
suite passed 3/3. Pure profile/C# compilation passed four valid and 17 rejected
profiles without invoking native methods. Pure focus checks passed the original
baseline/IME cases plus one approved Stack origin and eight rejected variants.
The first full validation run exposed two additional stale Node contracts
(direct blur-hide expectation and an unregistered new source-contract test).
That first aggregate exited 1: check, build, component tests, Rust tests, and Rust
check each passed with exit 0; Node exited 1. After updating those contracts,
`npm run test:node` passed: 1,431 passed, zero failed, four skipped, three todo.
This standalone rerun is not itself an aggregate six-stage validation pass.

Clean `npm run validate` rerun completed successfully after the stale-contract
fixes (2026-11-16); aggregate exit 0:

| Stage | Result | Exit |
|---|---|---:|
| check | PASS; zero Svelte errors/warnings | 0 |
| build | PASS | 0 |
| test:component | PASS; 30 files, 575 tests | 0 |
| test:node | PASS; 1,431 passed, 0 failed, 4 skipped, 3 todo | 0 |
| cargo:test | PASS; application unit target 1,000 passed, 0 failed, 10 ignored; additional targets passed | 0 |
| cargo:check | PASS; existing dead-code warnings remain | 0 |

The later review-driven BlurClosed PowerShell-only artifact adjustment was
independently rerun through both pure PowerShell suites and the 14-case Node
regression suite; all passed. Those scripts are not included in the six-stage
aggregate. Production inputs remained unchanged during that adjustment.

The opt-in native runner now accepts `-StackVisibilityCase NeverOpened`,
`ExplicitClosed`, `BlurClosed`, or `Open`; `Open` requires `SingleDisplay`.
Each invocation starts one fresh owned release process. Other than NeverOpened,
cases need an existing safe pinned folder; the runner does not create pins or
modify folder contents. A PID/thread/HWND/instance-scoped WinEvent observer
records transient Stack SHOW/HIDE events from before the snip click through
preview dismissal, alongside final visibility. Open also checks ordinary Stack
has no capture-exclusion affinity. The `Unobserved` default retains the original
journey without a Stack assertion.

Review identified that the original BlurClosed setup's fixture-B click could be
underneath newly opened topmost Stack. BlurClosed now measures Stack bounds,
chooses a deterministic 64px owned-fixture rectangle outside those bounds with
8px clearance and inside the monitor work area, and moves only fixture B.
Native hit ownership and unchanged fixture identity are checked before injecting
the click; foreground equality and a new owned OS focus event confirm transfer.
Unavailable space fails closed rather than moving Stack or clicking through it.
The pure profile suite verifies four overlap cases (including the original
covered click and negative monitor coordinates) and five invalid/no-space cases;
parser/C# compilation and existing focus oracles still pass. None of these pure
checks calls a native method or establishes live blur behavior.

**No Stack native case was executed: separate human consent is required.** The
old two-run release results above predate this fix and do not validate it. Native
Alt+S, Escape, delayed cleanup, failure/retry, repeat interactions, and controlled
captured-pixel equality remain unverified. Capture affinity and mocked visibility
are not pixel-inclusion proof. Delivered WinEvents can expose transient reopening
missed by polling, but recorder shutdown is not a compositor or future-event
barrier. No new screenshots, capture exclusions, timing sleeps, or persistent
diagnostic logging were added by the Stack observation extension.
