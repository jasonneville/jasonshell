# Weekly health audit — 11/16/2026

## Scope and method

Read `AGENTS.md`, the full current `master_spec.md`, `README.md`, `docs/README.md`, and `docs/current-state-brief.md` first. Sampled package/configuration, validation policy, docs-policy tests, command execution/path boundaries, native test output, and relevant source/tests across frontend and Rust. Findings below are current-source/test observations; older audit and changelog claims were treated only as leads. No source or product behavior was changed.

## Findings

1. **P1 — Verified validation gate is red and short-circuits later gates.** `package.json:23` chains `check && build && test:component && test:node && cargo:test && cargo:check`; the observed `npm run validate` exited 1 in Node tests (1,395 pass, 10 fail, 4 skipped, 3 todo), so this invocation did not reach its Rust stages. Failures included centered-search, Melt migration, changelog-policy hygiene, search-panel, source-contract registry, and taskbar-launcher contract tests. **Impact:** one advertised aggregate gate cannot provide complete frontend/backend evidence, and stale contract failures can mask whether Rust gates ran. **Next:** repair or explicitly quarantine each current baseline failure, and consider an aggregate runner that records every independent stage while preserving a nonzero final result.

2. **P1 — Verified current documentation contradicts the shortcut contract.** `docs/current-state-brief.md:28-30` still describes seven shortcuts and seven-field save behavior, while `README.md:26-30,235-240` and `master_spec.md:181-186` describe eight shortcuts including Quick Commands. **Impact:** onboarding and maintenance work can implement the wrong authorization, migration, or Settings contract. **Next:** reconcile the brief with source/tests and add one authoritative product-truth assertion for the shortcut count and names.

3. **P2 — Historical validation snapshots risk being mistaken for current evidence.** `README.md:28,320-321` and `master_spec.md:78` preserve older Node/component/Rust counts and a historical commit, while the current run reports 1,412 Node tests with 10 failures and current Rust suites pass independently. **Impact:** readers may mistake historical counts for current release evidence and misdiagnose regressions. **Next:** date validation snapshots, move point-in-time numbers to scheduled evidence, and keep active docs limited to reproducible commands/current status.

4. **P2 — Candidate dependency hygiene issue; no product consumer found in sampled search.** `package.json:36-40` declares `afplay`, `bun`, `bunx`, and `osascript`; the sampled repository source search found no product consumer, while the lockfile carries platform-specific Bun packages. **Impact:** if ownership is not intentional, these dependencies may add install volume, supply-chain review surface, and platform confusion in a Windows-only runtime; impact remains conditional pending ownership confirmation. **Next:** confirm intentional tooling ownership, then remove unused runtime dependencies and regenerate the lockfile in a dedicated dependency change (or document the exact consumer and isolation rationale).

5. **P2 — Verified native validation boundary remains material.** `npm run cargo:test` passed its enabled suites, but its output still showed consent/opt-in ignored native journeys (including snipping Escape, event routing, placement, clipboard, and packaged-model paths); live shell, Win32/WebView2, clipboard, monitor, and native hotkey smoke were not run by this audit. `master_spec.md:248-260` and `README.md:319-335` correctly describe much of this gap. **Impact:** static/fixture evidence cannot establish shell-state restoration, real focus/input delivery, DWM/WebView2 compositing, or cross-window behavior. **Next:** retain these as explicit acceptance gates and run only with human consent on a controlled Windows environment.

6. **P2 — Verified Rust warning debt weakens maintainability and boundary signal.** `npm run cargo:check` passed but emitted many warnings, including duplicated declarations of `GetCurrentProcess` and `GetProcessHandleCount` with different signatures in `src-tauri/src/snipping/clipboard_process.rs` and `src-tauri/src/stack_popup/text_document/feasibility/native.rs`, plus extensive dead-code warnings in the feasibility modules. **Impact:** warning volume can hide new FFI/ownership defects; clashing extern declarations are especially risky at a native boundary even when they currently compile. **Next:** consolidate shared FFI declarations or use one canonical binding, then isolate/remove dormant feasibility code or explicitly gate it and enforce a warning budget.

## Critical findings

No critical findings were verified in this sampled audit. This is not a vulnerability certification or a claim that untested native paths are safe.

## Validation and limits

- `Get-Date -Format 'MM/dd/yyyy HH:mm:ss K'` → `11/16/2026 12:01:50 -06:00`.
- `node --test tests/changelogPolicy.test.mjs tests/bootstrapWindowsContract.test.mjs tests/productTruthDocs.test.mjs` → 10/10 pass.
- `npm run check` → pass; Svelte check reported 0 errors and 0 warnings.
- `npm run build` → pass; Vite built 439 modules.
- `npm run validate` → fail at Node stage; 1,395 pass, 10 fail, 4 skipped, 3 todo; later stages were short-circuited.
- `npm run cargo:test` → enabled Rust tests passed; opt-in/consent-gated native tests remained ignored.
- `npm run cargo:check` → pass with the warning debt described above.
- `git diff --check` and `git status --short` → clean before this report; no unrelated dirty changes were present.
- Not run: browser automation, Tauri/native app launch, Win32 shell smoke, clipboard/work-area mutation, native integration opt-ins, or dependency installation. These require the prohibited consent/environment boundary for this report-only audit.
