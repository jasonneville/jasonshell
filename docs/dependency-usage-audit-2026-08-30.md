# Dependency Usage Audit - 2026-08-30

## Scope and decision rule

Packages audited: `afplay`, `bun`, `bunx`, and `osascript`.

Removal requires dependency-graph evidence, source and script/config evidence, build/test evidence after removal, lockfile-diff review, and maintainer acceptance. Inconclusive evidence keeps the dependency. This audit does not propose or perform removal.

## Evidence collected

### Package graph and scripts

- `npm ls afplay bun bunx osascript --all` reports all four packages as direct root dependencies.
- `npm explain` for each package identifies the root project declaration as its installation reason; no other package requires any of them.
- `package.json` scripts use npm, Vite, TypeScript, Cargo, and Tauri entrypoints. No script invokes or imports any audited package by name.

### Source, config, and tests

- Searches across TypeScript, Svelte, Rust, scripts, config, and tests found no import, `require`, executable invocation, or Rust integration for `afplay`, `bunx`, or `osascript`.
- Source occurrences of `bun` in `src/features/top-bar/topBarUxState.ts` and `src/features/terminal/terminalTabTitle.ts`, including the `bunx` alternative in the top-bar classifier, classify user-entered command text. They do not import or execute the installed `bun` or `bunx` packages.
- An `osascript` occurrence under `.rust-skills` is tooling documentation, not application code.
- No audited package has an established application runtime path from inspected source or tests.

### Lockfiles and installation effects

- `package-lock.json` records all four as direct dependencies.
- `afplay` is a leaf package.
- `bun@1.3.13` exposes `bun` and `bunx` binaries, runs `node install.js` during postinstall, and selects optional platform binary packages.
- `bunx@0.1.0` is a separate JavaScript shell package depending on `bun-utilities`; it is not merely the `bun` package's binary alias.
- `osascript` depends on `duplex-child-process`.
- Root `bun.lock` records `afplay` and `osascript` but does not record the current direct `bun` and `bunx` declarations. This mismatch indicates mixed or stale package-manager history. It prevents a confident claim that binary/tooling use outside inspected npm scripts never existed or is no longer expected.

## Package decisions

| Package | Decision | Established fact | Remaining uncertainty |
| --- | --- | --- | --- |
| `afplay` | Keep | Direct dependency; no application import or invocation found. | `bun.lock` retains it, and no maintainer-approved removal experiment establishes whether external/macOS tooling expects it. |
| `bun` | Keep | Direct dependency; no application import or invocation found; install has binary and postinstall effects. | Mixed lockfile history and possible external developer-tool use remain unresolved. |
| `bunx` | Keep | Direct dependency; no application import or invocation found; package is distinct from Bun's `bunx` binary. | Declaration intent and possible external developer-tool use remain unresolved. |
| `osascript` | Keep | Direct dependency; no application import or invocation found. | `bun.lock` retains it, and no maintainer-approved removal experiment establishes whether external/macOS tooling expects it. |

Confidence is high that inspected application source, repository scripts, config, and tests do not currently reference these packages. Confidence is insufficient for removal because repository evidence cannot disprove unrecorded tooling use, lockfile history conflicts, no maintainer accepted removal, and no post-removal build/test evidence exists.

## Validation evidence

- Focused product-truth docs contract: passed 3/3.
- `npm run check`: passed with 0 errors and 3 existing unused-selector warnings in `StackGitPanel.svelte`.
- `npm run build`: passed with the same 3 existing unused-selector warnings.
- `npm run test:node`: passed 817 tests with 3 todo and 0 failures.
- `npm run cargo:test`: passed 629 tests with 3 ignored and 0 failures.
- `npm run cargo:check`: passed with 46 existing warnings.
- `npm run validate`: passed; repeated Node, Svelte, build, Rust test, and Cargo check gates with the same existing warnings.

These runs validate repository health with dependencies retained. They cannot prove safe removal because no removal was authorized or performed.

## Requirement conclusion

- FR-6: package scripts, TypeScript/Svelte, Rust, config, tests, dependency graph, package metadata, and both lockfiles inspected; final build/test results are recorded after execution.
- FR-7: no removal proposed because maintainer acceptance and post-removal evidence do not exist.
- FR-8: inconclusive packages retained with uncertainty recorded above.
- FR-9: `package.json`, `package-lock.json`, and `bun.lock` remain unchanged; no speculative removal performed.
