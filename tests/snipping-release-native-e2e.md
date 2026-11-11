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

## Remaining repository limits

Normal component coverage (52) and codec coverage (11 passed, 1 ignored) remain
bounded evidence, not a full-repository green result. Coordinator/clipboard
targets compile, but ordinary integration execution remains blocked before test
entry by `0xc0000139` from the missing Common Controls v6 resource; that source
cause is unresolved. Standard-SDK temporary copies covered the coordinator
suite (43, including all 10 image cases) and clipboard/native suites (21 + 13
native ignored), with original/copy text byte-identical; copies were deleted.
The strict `LegacyDual` two-monitor profile is excluded from this evidence.
