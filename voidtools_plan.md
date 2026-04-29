# NO-CODE GATE

No production code, tests, settings migrations, dependency changes, bundling changes, installer changes, or `master_spec.md` edits may start from this work item until this plan exists, is reviewed, and the implementation scope is explicitly approved. This file is the only intended change for this planning task.

Before any future implementation phase touches production files, that phase must first add or update its red tests, run them, and record the expected failing result. Production code may start only after the red tests fail for the intended reason, not because of syntax errors, missing test infrastructure, or unrelated repo failures.

Before any Everything SDK DLL, Everything installer, ES executable, or downloaded Voidtools artifact is bundled, downloaded, cached, launched, or referenced by an installer flow, a separate supply-chain and legal approval gate must pass with:

- Official Voidtools URL captured from the current official download page or official SDK page.
- Exact artifact name and version.
- SHA-256 checksum recorded from an approved source or computed from the reviewed artifact.
- License and redistribution approval for JasonShell use.
- Provenance note covering who approved it, when it was obtained, where it is stored, and whether it is bundled or downloaded at runtime.
- Negative decision path: if any item is missing, JasonShell must use fallback search only and must not download, bundle, or execute the artifact.

# JasonShell Search Rewrite Plan: Flow Launcher Model + Voidtools Everything

Status: Draft plan
Date: 2026-04-29
Scope: Plan only
Primary goal: Rewrite JasonShell search in Rust and TypeScript so it feels realtime, covers Flow Launcher-like launcher domains, and relies heavily on Voidtools Everything for file and folder search.

## Source Evidence

Flow Launcher patterns inspected under `C:\dev\Flow.Launcher`:

- `Flow.Launcher.Core\Plugin\QueryBuilder.cs` parses empty/home queries, trims whitespace, collapses terms, detects action keywords from non-global plugins, and separates `OriginalQuery`, `TrimmedQuery`, `Search`, `SearchTerms`, and `ActionKeyword`.
- `Flow.Launcher.Plugin\Query.cs` defines the query contract with original text, trimmed text, search text, term helpers, action keyword, home-query flag, and requery flag.
- `Flow.Launcher.Plugin\Result.cs` defines a broad result contract: title, subtitle, copy text, autocomplete text, icon path/data, glyph, score, highlight indexes, action, async action, preview data, selected-count opt-in, record key, badge, and suggestion text.
- `Flow.Launcher\ViewModel\MainViewModel.cs` uses cancellation tokens for stale query flows, a channel for result updates, plugin fanout, deep-cloned results, selected-count boosts, plugin priority boosts, and top-most overrides that can force scores near `Result.MaxScore`.
- `Plugins\Flow.Launcher.Plugin.Explorer\Search\SearchManager.cs` routes path search, quick access, file/folder search, content search, index search, and action-keyword type filters. It deduplicates by path/title/subtitle and handles provider availability errors distinctly.
- `Plugins\Flow.Launcher.Plugin.Explorer\Search\Everything\EverythingSearchManager.cs` implements Everything-backed index, content, and path providers, checks whether Everything is running, exposes a click-to-install path, and streams cancellable async results.
- `Plugins\Flow.Launcher.Plugin.Explorer\Search\Everything\EverythingAPI.cs` serializes Everything SDK access through a semaphore, builds the query string, applies max/offset/sort/match-path/request flags, reads full paths, types, run count, and highlight data, increments run counters, and resets the SDK state after each query.
- `Plugins\Flow.Launcher.Plugin.Explorer\Search\Everything\EverythingApiDllImport.cs` is a direct `Everything.dll` P/Invoke wrapper over `Everything_SetSearchW`, `Everything_QueryW`, result getters, sort flags, run counts, and highlight APIs.
- Flow bundles SDK DLLs at `Plugins\Flow.Launcher.Plugin.Explorer\EverythingSDK\x64\Everything.dll` and `...\x86\Everything.dll`.
- Flow app-source plugins include Program, WebSearch, BrowserBookmark, Calculator, Shell, Sys, and WindowsSettings.
- Flow Everything installer flow checks registry and Scoop paths, otherwise prompts for an executable or downloads an older Everything package. JasonShell should not copy that version blindly.

JasonShell current contracts inspected under `C:\dev\jasonshell`:

- `src/components/TopBar.svelte` owns query text, selected index, immediate search-panel publishes, latest-only render sequencing, deferred system search, Ctrl+K focus, outside-click/blur dismissal, `search-panel:interaction`, `search-panel:closed`, and activation.
- `src/lib/searchPanel.ts` defines the current result shape, payload shape, Tauri commands, and events: `search-panel:update`, `activate`, `select`, `pin-folder`, `interaction`, `closed`.
- `src-tauri/src/search_sources/index.rs` owns the warmed cache, provider cache TTLs, in-flight query cap, background refresh, `search-index:refreshed`, and provider/local merge.
- `src-tauri/src/search_sources.rs` exposes `SystemSearchResult` and async `search_system`, using `spawn_blocking`.
- `src-tauri/src/settings.rs` owns `jasonshell-settings-v1.json`, schema/version normalization, corrupt-file backup, secret-key rejection, and `ShellUiSettings`.
- `src/lib/shellPreferences.ts` owns renderer-local preferences in `localStorage["jasonshell.uiPreferences"]` and syncs them with `BroadcastChannel`.

Voidtools official-doc evidence provided by parent:

- Current downloads page lists Everything 1.4.1.1032, ES 1.1.0.30, and `Everything-SDK.zip`.
- Everything SDK provides DLL/Lib IPC and requires Everything running.
- Installer/service setup can require administrator elevation.
- Everything service lets a standard user index NTFS and exposes filenames only.

Local environment evidence:

- Local Everything is absent.

## Desired User Behavior

JasonShell search must support two modes:

- Top-right mode: keep search embedded in the top bar, using the existing top-bar input and the anchored `search-panel`.
- Centered hotkey mode: Ctrl+K opens a centered search surface, Flow Launcher style, while preserving the same search engine, result contracts, activation behavior, and stale-response protection.

Search must feel realtime:

- Keystrokes update visible query and panel payload immediately.
- Expensive provider work runs behind the scenes.
- Stale provider responses cannot overwrite newer query state.
- Existing visible results stay on screen until better authoritative results arrive.

Search must be broad:

- Apps and Start Menu programs.
- Open windows.
- Files and folders from Everything.
- Path navigation.
- Shell commands.
- System commands and Windows Settings.
- Calculator.
- Web search templates.
- Browser bookmarks if practical.
- Developer/JasonShell commands such as control plane.

## Functional Requirements

FR-1: JasonShell MUST introduce a Flow-inspired query model with original query, trimmed query, normalized search text, search terms, optional action keyword, home-query flag, and requery flag.

FR-2: JasonShell MUST preserve the current `SearchPanelPayload` delivery model unless an explicit migration plan updates all event, command, and test contracts together.

FR-3: Search execution MUST use latest-query cancellation or generation gates for every async provider.

FR-4: Search providers MUST fan out in parallel where safe and return partial results without blocking input rendering.

FR-5: File and folder search MUST prefer Voidtools Everything when Everything is installed and running.

FR-6: JasonShell MUST detect Everything availability by checking bundled SDK DLL availability, installed Everything process/IPC readiness, installed executable paths, and service state where feasible.

FR-7: If Everything is not available, JasonShell MUST expose a safe setup flow that can install or launch Everything only after user consent.

FR-8: Auto-install MUST prefer a pinned bundled installer or pinned downloaded artifact with checksum verification. It MUST NOT silently fetch or run an unverified binary.

FR-9: Everything setup MUST handle admin requirements explicitly. If service install/elevation is needed, JasonShell MUST explain why and surface a controlled elevation path.

FR-10: Everything search MUST support filename/path search, folder-only search, file-only search, max result limits, fast sort options, full path matching, and highlighted match data when available.

FR-11: Everything content search MUST be disabled by default unless benchmarks prove it is acceptable. If enabled later, it MUST be opt-in and visibly labeled as slower.

FR-12: Search ranking MUST combine base fuzzy match score, provider priority, result type priority, user selected-count boosts, Everything run count when available, and optional top-most pins.

FR-13: Search ranking MUST cap score boosts to avoid overflow and deterministic order instability.

FR-14: Result identity MUST be stable across sessions through provider kind plus canonical path, command ID, app ID, window HWND token, or URL template ID.

FR-15: Search settings MUST be persisted in JSON shell settings, not only renderer localStorage, for behavior that affects Rust search and window mode.

FR-16: UI-only cosmetic preferences MAY remain in `src/lib/shellPreferences.ts`, but search mode and Everything behavior MUST live in `jasonshell-settings-v1.json`.

FR-17: Search settings MUST include at minimum:

- `ui.searchMode`: `"topRight"` or `"centeredHotkey"`.
- `search.everything.enabled`: boolean.
- `search.everything.installMode`: `"ask"` by default; possible future values `"disabled"` and `"managed"`.
- `search.everything.sdkSource`: `"bundled"` or `"system"`.
- `search.everything.maxResults`: bounded integer.
- `search.everything.fullPathSearch`: boolean.
- `search.everything.sort`: safe enum.
- `search.everything.contentSearchEnabled`: boolean default false.
- `search.resultLimit`: bounded integer.

FR-18: The settings migration MUST preserve existing v1 settings and either remain schema-compatible with defaulted optional fields or intentionally bump the settings version with tests.

FR-19: The top-right search UI MUST remain layout-stable when enabled.

FR-20: The centered hotkey UI MUST be keyboard-first, accessible, dismissible with Escape/outside click/focus loss, and must not break top-bar shell reservation behavior.

FR-21: Search activation MUST continue to open apps, files, folders, commands, windows, settings, calculator results, and web-search targets through typed activation handlers.

FR-22: Search must preserve current cross-webview safety rules: explicit targets, payload fallback fetch, sequence gates, and no assumptions that an auxiliary webview is visible before payload delivery.

FR-23: Search MUST keep Windows Search or current warmed cache as fallback when Everything is absent, disabled, not running, or temporarily unavailable.

FR-24: Search MUST report provider health in non-secret diagnostic state: Everything missing, SDK missing, IPC unavailable, service unavailable, admin required, indexing, ready, degraded fallback.

FR-25: No provider may recursively scan broad filesystem roots on the keystroke path.

## Non-Functional Requirements

NFR-1: Typing-to-visible-query update MUST complete in under 16 ms in renderer unit/DOM timing tests.

NFR-2: Typing-to-first-cached-or-local-results SHOULD complete in under 50 ms for warm catalog queries.

NFR-3: Typing-to-Everything-results SHOULD complete in under 100 ms p95 on a normal warm Everything index for common 2+ character queries.

NFR-4: Search provider fanout MUST remain bounded; no unbounded thread/process creation.

NFR-5: Everything SDK calls MUST be serialized or otherwise proven thread-safe, following Flow's semaphore model until Rust implementation proves safe parallel use.

NFR-6: Everything FFI MUST use typed wrappers and return `Result`, not panic, for expected SDK, IPC, and service failures.

NFR-7: Unsafe Rust for DLL loading/FFI MUST be isolated in one module with documented safety invariants.

NFR-8: Result payloads MUST stay small enough for realtime webview delivery. Large icons/previews must be cached or lazy.

NFR-9: Settings must not store secrets, tokens, unbounded user content, or raw search histories unless explicitly approved later.

NFR-10: The full validation target remains `npm run validate`, with focused search, settings, and benchmark commands added before full validation.

## Proposed Architecture

### Rust Search Core

Add a new provider-oriented search engine under `src-tauri/src/search_sources/`:

- `query.rs`: Flow-inspired parser and normalized `SearchQuery`.
- `provider.rs`: provider trait, provider IDs, health, partial result envelope, cancellation/generation inputs.
- `everything.rs`: high-level Everything provider.
- `everything_ffi.rs`: narrow unsafe SDK wrapper around `Everything.dll`.
- `everything_install.rs`: detection, consent, installer launch, checksum verification, admin/service handling.
- `rank.rs` or extend `scoring.rs`: unified scoring with selected-count and top-most support.
- `commands.rs` or existing module exports: Tauri commands for search and provider health if needed.

Keep or adapt existing modules:

- `apps.rs` remains app indexing provider.
- `files.rs` becomes fallback/warm cache, not primary broad search when Everything is ready.
- `windows_search.rs` remains fallback provider.
- `index.rs` either becomes the provider coordinator or delegates to a new coordinator.

### TypeScript Search/UI Core

Keep the current immediate-publish pattern in `TopBar.svelte`, but extract more of it:

- `src/lib/searchPanel.ts`: extend result kind and payload only through typed migration.
- `src/lib/searchSettings.ts`: typed frontend wrapper for persistent JSON search settings.
- `src/lib/searchQuery.ts`: renderer mirror of query normalization only if needed for immediate UI; Rust remains authoritative for providers.
- `src/features/search/searchUxState.ts`: add centered-mode state and keyboard behavior helpers.
- `src/components/SearchPanelSurface.svelte`: keep render-only behavior; support top-right and centered layout variants if using same surface.
- Possible new `src/components/CenteredSearchSurface.svelte` only if separate webview/window simplifies geometry and focus handling.

### Settings Ownership

Behavioral search settings belong in Rust JSON settings:

- Extend `ShellSettings` with `search: SearchSettings` or extend `ShellUiSettings` for `searchMode` and add a top-level `search` object for provider behavior.
- Add defaults and migration tests.
- Keep renderer-only cosmetic flags in `shellPreferences.ts`.

### Everything Bundling Strategy

Preferred implementation path:

1. Bundle `Everything.dll` from official `Everything-SDK.zip` for x64 and x86 under a JasonShell-owned directory such as `src-tauri/vendor/everything-sdk/{x64,x86}/Everything.dll`.
2. Load the correct DLL by process architecture.
3. Detect whether Everything IPC is available.
4. If not running but installed, launch `Everything.exe -startup` after user consent.
5. If not installed, offer managed setup:
   - Use a pinned official installer version or bundled installer.
   - Verify checksum before launch.
   - Explain admin/service requirements.
   - Launch installer through controlled shell execution.
6. Fall back to Windows Search/current cache if the user declines or setup fails.

Do not ship a silent installer path. Do not run a downloaded binary without checksum verification. Do not make content search default.

## Result Contract Target

Target Rust result contract should be a superset of current `SearchPanelResult`:

```ts
type SearchResultKind =
  | "app"
  | "window"
  | "folder"
  | "file"
  | "command"
  | "setting"
  | "calculator"
  | "web"
  | "bookmark";

interface SearchResult {
  id: string;
  providerId: string;
  kind: SearchResultKind;
  title: string;
  subtitle: string;
  terms: string;
  priority: number;
  score: number;
  iconDataUrl?: string;
  path?: string;
  url?: string;
  actionId: string;
  copyText?: string;
  autoCompleteText?: string;
  titleHighlightData?: number[];
  subtitleHighlightData?: number[];
  recordKey?: string;
  providerHealth?: "ready" | "degraded" | "unavailable";
}
```

Migration rule: keep current fields stable until all callers and tests move to the new shape.

## API Contracts

These contracts must be finalized and red-tested before production code uses them. Field names are target contracts, not permission to migrate callers piecemeal without a coordinated test update.

### Settings contract

```ts
type SearchMode = "topRight" | "centeredHotkey";
type EverythingInstallMode = "ask" | "disabled" | "managed";
type EverythingSdkSource = "bundled" | "system";
type EverythingSortMode = "nameAsc" | "pathAsc" | "dateModifiedDesc" | "runCountDesc";

interface SearchSettingsContract {
  ui: {
    searchMode: SearchMode;
  };
  search: {
    resultLimit: number;
    everything: {
      enabled: boolean;
      installMode: EverythingInstallMode;
      sdkSource: EverythingSdkSource;
      maxResults: number;
      fullPathSearch: boolean;
      sort: EverythingSortMode;
      contentSearchEnabled: boolean;
    };
  };
}
```

### Provider health contract

```ts
type SearchProviderId =
  | "apps"
  | "openWindows"
  | "everything"
  | "windowsSearch"
  | "warmedCache"
  | "commands"
  | "calculator"
  | "web"
  | "bookmarks";

type ProviderHealthState =
  | "ready"
  | "degraded"
  | "unavailable"
  | "indexing"
  | "adminRequired"
  | "disabled";

interface ProviderHealthContract {
  providerId: SearchProviderId;
  state: ProviderHealthState;
  reasonCode?:
    | "sdkMissing"
    | "ipcUnavailable"
    | "serviceUnavailable"
    | "notInstalled"
    | "notRunning"
    | "userDisabled"
    | "checksumBlocked"
    | "licenseBlocked"
    | "fallbackActive";
  message: string;
  canRequestSetup: boolean;
  checkedAtIso: string;
}
```

### Install consent and result contract

```ts
type EverythingSetupAction = "launchInstalled" | "downloadInstaller" | "runBundledInstaller" | "openOfficialDownload";

interface EverythingSetupConsentRequest {
  action: EverythingSetupAction;
  officialUrl: string;
  artifactName?: string;
  version?: string;
  sha256?: string;
  licenseApproved: boolean;
  provenanceApproved: boolean;
  requiresAdmin: boolean;
  explainsFilenameExposure: boolean;
}

type EverythingSetupStatus = "declined" | "launched" | "installed" | "blocked" | "failed";

interface EverythingSetupResult {
  status: EverythingSetupStatus;
  health: ProviderHealthContract;
  reasonCode?: ProviderHealthContract["reasonCode"] | "userDeclined" | "adminDeclined" | "launchFailed";
  message: string;
}
```

### Activation contract

```ts
type SearchActivationKind =
  | "openApp"
  | "focusWindow"
  | "openFile"
  | "openFolder"
  | "runCommand"
  | "openSetting"
  | "copyCalculatorResult"
  | "openWebUrl"
  | "openBookmark";

interface SearchActivationRequest {
  resultId: string;
  providerId: SearchProviderId;
  actionId: string;
  kind: SearchActivationKind;
  recordKey: string;
  payload: Record<string, string | number | boolean>;
  requiresConfirmation: boolean;
}

interface SearchActivationResult {
  resultId: string;
  handled: boolean;
  message?: string;
}
```

### Centered surface contract

```ts
interface CenteredSearchSurfaceContract {
  label: "search-panel" | "centered-search";
  mode: "centeredHotkey";
  requestId: string;
  query: string;
  sequence: number;
  anchor: "screenCenter";
  closeReasons: Array<"escape" | "outsideClick" | "focusLoss" | "activation" | "settingsChanged">;
  accessibility: {
    role: "combobox";
    listboxId: string;
    activeOptionId?: string;
  };
}
```

## Data Models

| Model | Owner | Persistence | Required fields | Notes |
| --- | --- | --- | --- | --- |
| `SearchQuery` | Rust search core | none | `original`, `trimmed`, `search`, `terms`, `actionKeyword`, `isHomeQuery`, `isRequery` | Mirrors Flow-style parsing. Renderer may mirror only for immediate UI display. |
| `SearchSettings` | Rust settings | `jasonshell-settings-v1.json` or approved migration | `ui.searchMode`, `search.resultLimit`, `search.everything.*` | Renderer `localStorage` cannot own behavior that changes Rust providers or window mode. |
| `ProviderHealth` | Rust provider coordinator | transient diagnostic state | `providerId`, `state`, `reasonCode`, `message`, `canRequestSetup`, `checkedAtIso` | Must be non-secret and safe to show in settings or diagnostics. |
| `EverythingArtifactProvenance` | Release/build docs or approved source file | durable review record | `officialUrl`, `artifactName`, `version`, `sha256`, `licenseApproval`, `approvedBy`, `approvedAtIso`, `storagePath`, `distributionMode` | Required before bundling, downloading, caching, or launching any Voidtools artifact. |
| `EverythingSetupConsent` | UI/Rust setup flow | transient, optional last-decline timestamp if approved | `action`, `officialUrl`, `version`, `sha256`, `requiresAdmin`, `licenseApproved`, `provenanceApproved`, `explainsFilenameExposure` | Must exist before launch/download/install actions. |
| `SearchResult` | Provider coordinator | transient plus optional usage record | `id`, `providerId`, `kind`, `title`, `subtitle`, `score`, `actionId`, `recordKey` | IDs must be stable across sessions for apps, commands, paths, URLs, and durable provider items. |
| `SearchUsageRecord` | Ranking module | approved settings/cache file only | `recordKey`, `selectedCount`, `lastSelectedAtIso` | Must not store raw query history unless separately approved. |
| `CenteredSearchSurfaceState` | Renderer/Rust surface boundary | transient | `mode`, `requestId`, `query`, `sequence`, `activeOptionId`, `closeReason` | Must preserve sequence gates and fallback payload fetch behavior. |

## Out of Scope

- Silent Everything install, silent service setup, silent elevation, or silent launch of downloaded executables.
- Bundling or downloading Voidtools artifacts without the supply-chain/legal approval gate.
- Enabling Everything content search by default.
- Network-backed web suggestions or remote search providers beyond static user-approved URL templates.
- Browser bookmark ingestion without explicit privacy review and user approval.
- Storing raw search history, secrets, tokens, credentials, or unbounded user content.
- Replacing current `SearchPanelPayload` without a coordinated migration of Rust commands, TS wrappers, renderer surfaces, and tests.
- Broad recursive filesystem scans on the keystroke path.
- Changing top/bottom AppBar reservation behavior as part of centered search unless explicitly required and smoke-tested.

## Approval Gates

Each gate must be marked approved or blocked before the dependent phase starts:

- Gate A: Plan and scope approval. Required before Phase 1 code/tests.
- Gate B: Flow parity map and quality corpus approval. Required before Phase 1 code/tests. Must map Flow query, result, provider fanout, ranking, activation, and Everything behavior to JasonShell target contracts.
- Gate C: API contract approval for settings, provider health, install consent/result, activation, and centered surface. Required before contract implementation.
- Gate D: Red-test approval per Phase 1-7. Required before production changes in that phase.
- Gate E: Supply-chain/legal approval for Voidtools SDK/installer artifacts. Required before bundling, downloading, caching, or executing artifacts.
- Gate F: Privacy approval for filename exposure, bookmark ingestion, web templates, and usage persistence. Required before providers that touch those areas.
- Gate G: Centered surface approval. Required before adding a new Tauri webview/window or changing existing `search-panel` geometry contracts.
- Gate H: Speed and quality parity approval. Required before release readiness. Must run the exact parity command and meet pass thresholds or document approved skips.

## Implementation Phases

### Phase 0: Spec Approval and Baseline Capture

Files to touch:

- `voidtools_plan.md` only until approval.

Tasks:

- Review this plan.
- Create and approve a Flow parity map before code. Minimum map rows: query parsing, result fields, provider fanout/cancellation, Everything search, provider health, install/setup, scoring boosts, activation, and centered launcher UX.
- Create and approve the quality parity corpus fixture before code. The fixture must include stable query IDs, expected result IDs, provider expectations, skip rules, and pass thresholds.
- Confirm whether Everything installer may be bundled or downloaded.
- Confirm whether centered hotkey mode should use the existing `search-panel` webview or a new centered webview.
- Confirm whether bookmark/web/calculator providers are required in the first implementation slice or can be phased after Everything parity.

Acceptance criteria:

- AC-0.1: Plan includes no-code gate, phases, files, tests, benchmarks, risks, and unknowns.
- AC-0.2: No files other than `voidtools_plan.md` changed.
- AC-0.3: Flow parity map is approved and names which Flow behaviors are required, deferred, or rejected for JasonShell.
- AC-0.4: Quality parity corpus is approved before any production search code starts.
- AC-0.5: Supply-chain/legal gate is either approved for a specific Voidtools artifact or explicitly blocked with fallback-only implementation scope.

### Phase 1: Search Contracts and Settings Spec

Files likely to touch after approval:

- `src-tauri/src/settings.rs`
- `src/lib/searchSettings.ts`
- `src/ipc/commands.ts`
- `src/lib/searchPanel.ts`
- `tests/contractsSettings.test.mjs`
- `tests/searchSettings.test.mjs`
- Rust settings tests in `settings.rs`

Tasks:

- Red-test gate before production changes:
  - Add failing settings contract tests for defaults, migration, invalid enum handling, persistence, and secret-key rejection.
  - Add failing API contract tests for `SearchSettingsContract`, `ProviderHealthContract`, install consent/result, activation, and centered-surface payload shape.
  - Run `node --test tests\contractsSettings.test.mjs tests\searchSettings.test.mjs tests\searchContracts.test.mjs` and record failures that prove missing contracts, not broken infrastructure.
- Add typed search settings defaults.
- Decide v1 optional-field compatibility vs settings version bump.
- Add load/save normalization and secret-key rejection coverage.
- Define result contract migration and provider health contract.
- Add settings-panel controls only if user wants UI in first slice; otherwise JSON-only support is acceptable for this task.

Acceptance criteria:

- AC-1.1: Missing settings file defaults to top-right mode and Everything enabled with install mode `ask`.
- AC-1.2: Existing v1 JSON without search fields loads with default search settings.
- AC-1.3: Invalid enum values normalize or fail in a documented way.
- AC-1.4: Search behavior settings persist in `jasonshell-settings-v1.json`.
- AC-1.5: Renderer localStorage remains cosmetic only and does not own Rust search mode.

### Phase 2: Query Parser and Provider Coordinator

Files likely to touch:

- `src-tauri/src/search_sources.rs`
- `src-tauri/src/search_sources/index.rs`
- `src-tauri/src/search_sources/scoring.rs`
- New `src-tauri/src/search_sources/query.rs`
- New `src-tauri/src/search_sources/provider.rs`
- Rust search tests
- `src/lib/systemSearch.ts`
- `src/features/search/searchUxState.ts`
- `tests/searchUxState.test.mjs`

Tasks:

- Red-test gate before production changes:
  - Add failing Rust query parser tests for empty, whitespace, action keyword, path, quoted path, multi-term, and requery cases.
  - Add failing provider coordinator tests for generation rejection, partial results, slow provider behavior, bounded fanout, and no broad filesystem scan on keystroke path.
  - Add failing Node state tests for generation/sequence UI handling if renderer behavior changes.
  - Run `cargo test --manifest-path src-tauri\Cargo.toml search_sources::query search_sources::provider` and `node --test tests\searchUxState.test.mjs`; record intended red failures.
- Implement Flow-inspired query parsing.
- Add provider fanout coordinator with generation/cancellation gates.
- Preserve `spawn_blocking` or bounded background execution for blocking providers.
- Keep existing warmed cache as fallback.
- Add provider health payloads.

Acceptance criteria:

- AC-2.1: Empty/whitespace query becomes home query and avoids provider fanout.
- AC-2.2: Action keyword queries route only to matching provider sets.
- AC-2.3: Newer query generation rejects stale provider results.
- AC-2.4: Provider fanout returns partial results when one provider is slow or unavailable.
- AC-2.5: No broad filesystem scan occurs on the keystroke path.

### Phase 3: Everything SDK Provider

Files likely to touch:

- New `src-tauri/src/search_sources/everything.rs`
- New `src-tauri/src/search_sources/everything_ffi.rs`
- New `src-tauri/vendor/everything-sdk/x64/Everything.dll`
- New `src-tauri/vendor/everything-sdk/x86/Everything.dll` if x86 supported
- `src-tauri/Cargo.toml`
- Rust Everything tests with mockable trait boundary

Tasks:

- Red-test gate before production changes:
  - Add failing Rust tests through a mockable Everything boundary for missing DLL, not running, IPC error, SDK error reset, file/folder type mapping, run count, highlight data, max limit, sort, and stale generation rejection.
  - Add failing health mapping tests proving SDK/IPC failures degrade without panics.
  - Run `cargo test --manifest-path src-tauri\Cargo.toml everything search_sources` and record intended red failures.
- Add architecture-aware SDK DLL loading.
- Wrap SDK functions needed for search, result type, full path, sort, max, offset, match path, run count, highlighted file name, reset, and last error.
- Serialize SDK access with a mutex/semaphore unless proven unnecessary.
- Map SDK errors into typed provider health.
- Implement filename/path file and folder search.
- Keep content search disabled by default.

Acceptance criteria:

- AC-3.1: SDK missing returns degraded provider health without panic.
- AC-3.2: Everything not running returns degraded provider health and fallback continues.
- AC-3.3: Query wrapper always resets SDK state after completed or failed query.
- AC-3.4: Cancellation/generation stops result application for stale queries.
- AC-3.5: File/folder type mapping is correct for Everything file, folder, and volume results.
- AC-3.6: Highlight data is parsed into zero-based indexes.

### Phase 4: Everything Detection, Launch, and Managed Setup

Files likely to touch:

- New `src-tauri/src/search_sources/everything_install.rs`
- `src-tauri/src/main.rs`
- `src/ipc/commands.ts`
- `src/lib/searchSettings.ts`
- Optional settings-panel files if UI controls are added
- Rust install/detection tests with command-runner abstraction

Tasks:

- Red-test gate before production changes:
  - Add failing tests with command-runner and downloader abstractions for no-consent no-launch, checksum mismatch block, user decline fallback, admin-required reporting, official URL/version/checksum/provenance requirements, and setup health update.
  - Add failing tests that block download/bundle/execute when license approval or provenance approval is missing.
  - Run `cargo test --manifest-path src-tauri\Cargo.toml everything_install` and record intended red failures.
- Detect installed Everything from registry, common install paths, Scoop path, and running process/IPC.
- Detect service/admin requirement state where possible.
- If installed but not running, ask before launching `Everything.exe -startup`.
- If absent, ask before installing.
- Use pinned official Everything 1.4.1.1032 or later approved version, with checksum recorded in source.
- Support bundled installer if selected by user.
- Verify checksum before execution.
- Surface admin elevation and service implications.

Acceptance criteria:

- AC-4.1: No installer or executable launches without explicit user consent.
- AC-4.2: Downloaded installer checksum mismatch blocks execution.
- AC-4.3: User decline leaves search in fallback mode without errors.
- AC-4.4: Admin-required state is reported clearly before elevation.
- AC-4.5: Setup result updates provider health and search status.
- AC-4.6: Missing official URL, version, SHA-256 checksum, license approval, or provenance approval blocks bundling, download, cache, launch, and installer execution.

### Phase 5: Ranking, Usage, and Quality Parity

Files likely to touch:

- `src-tauri/src/search_sources/scoring.rs`
- `src/lib/searchRanking.ts`
- New or updated usage persistence module
- Node and Rust ranking tests

Tasks:

- Red-test gate before production changes:
  - Add failing ranking tests for selected-count boost, top-most override, provider priority, result type priority, Everything run-count boost, duplicate collapse, exact-match priority, deterministic tie-breaks, and overflow caps.
  - Add failing quality parity tests using the approved fixture schema and expected IDs.
  - Run `node --test tests\searchQualityParity.test.mjs tests\searchRanking.test.mjs` and `cargo test --manifest-path src-tauri\Cargo.toml search_sources::scoring`; record intended red failures.
- Merge Flow-inspired scoring: base match, provider priority, result type priority, selected-count boost, Everything run count, and top-most override.
- Preserve deterministic tie-breakers.
- Decide whether selected count remains localStorage or moves to JSON settings/cache.
- Add record keys for files, folders, commands, apps, windows, and web results.

Acceptance criteria:

- AC-5.1: Frequently selected result moves upward without hiding exact high-quality matches.
- AC-5.2: Top-most result can be forced near top deterministically.
- AC-5.3: Score math cannot overflow.
- AC-5.4: Duplicate paths from Everything and fallback providers collapse into one result.
- AC-5.5: Exact filename/app matches outrank weak substring matches.

### Phase 6: UI Modes and Search Surfaces

Files likely to touch:

- `src/components/TopBar.svelte`
- `src/components/SearchPanelSurface.svelte`
- Optional new `src/components/CenteredSearchSurface.svelte`
- `src/components/TopBar.css`
- Search surface CSS
- `src/lib/searchPanel.ts`
- `src-tauri/src/search_panel.rs`
- Possible new Rust centered panel/window module
- `src/lib/shellSurface.ts`
- `src/App.svelte`
- Capability/window config files if a new webview is added
- Node UI tests

Tasks:

- Red-test gate before production changes:
  - Add failing UI/state tests for top-right behavior preservation, centered Ctrl+K open/focus, listbox option navigation, Enter activation, Escape/outside/focus close, sequence gates, fallback payload fetch, and no overlap at supported widths.
  - Add failing Rust/window contract tests if a new centered webview or label is introduced.
  - Run `node --test tests\searchPanelState.test.mjs tests\searchUxState.test.mjs tests\centeredSearchSurface.test.mjs` and `cargo test --manifest-path src-tauri\Cargo.toml centered_search_surface -- --nocapture`; record intended red failures.
- Keep top-right mode behavior stable.
- Add centered hotkey mode controlled by JSON settings.
- Ensure Ctrl+K opens the configured mode.
- Preserve immediate payload publish, sequence gates, fallback payload fetch, result interaction event, and closed event.
- Add accessible listbox/option behavior for centered mode.

Acceptance criteria:

- AC-6.1: Top-right mode matches current layout and behavior.
- AC-6.2: Centered mode opens from Ctrl+K, focuses input, shows results, supports arrows/Enter/Escape, and closes predictably.
- AC-6.3: Switching settings changes mode after reload or documented live refresh.
- AC-6.4: Search result activation works from both modes.
- AC-6.5: Text and controls do not overlap at desktop and narrow widths.

### Phase 7: Additional Flow-Like Providers

Files likely to touch:

- `src-tauri/src/search_sources/apps.rs`
- New provider modules for commands, calculator, web, bookmarks, Windows settings, shell/system commands
- `src/lib/searchCatalog.ts`
- `src/components/SearchPanelSurface.svelte`
- Provider tests

Tasks:

- Red-test gate before production changes:
  - Add failing provider tests for stable IDs, typed action IDs, dangerous-command exclusion/confirmation, calculator no-process execution, static web template opening, bookmark privacy gating, and provider failure isolation.
  - Add failing activation contract tests for every enabled provider kind.
  - Run `node --test tests\searchProviderContracts.test.mjs tests\searchActivation.test.mjs` and `cargo test --manifest-path src-tauri\Cargo.toml flow_like_search_providers -- --nocapture`; record intended red failures.
- Map Flow Program provider concepts to current app indexing.
- Add command/system providers for shutdown, restart, lock, recycle bin, index options, settings, and JasonShell control plane where safe.
- Add calculator provider for simple expressions.
- Add web-search provider templates.
- Add bookmark provider only if browser data access is safe and scoped.
- Keep command activation explicit and guarded.

Acceptance criteria:

- AC-7.1: Provider results include stable IDs and typed action IDs.
- AC-7.2: Dangerous system commands require confirmation or are excluded.
- AC-7.3: Calculator results do not call external processes.
- AC-7.4: Web results open only configured URL templates.
- AC-7.5: Provider failures do not break file/folder search.

### Phase 8: Tests, Benchmarks, and Validation Gate

Files likely to touch:

- `tests/searchPanelState.test.mjs`
- `tests/searchUxState.test.mjs`
- `tests/systemSearchState.test.mjs`
- `tests/searchSettings.test.mjs`
- `tests/searchQualityParity.test.mjs`
- Rust tests under `src-tauri/src/search_sources/*`
- New benchmark harness under `tests/benchmarks/` or `src-tauri/benches/`
- `package.json` scripts for focused search benchmarks if approved

Tasks:

- Add red tests from acceptance criteria before implementation.
- Add mock Everything provider tests.
- Add integration tests for fallback behavior.
- Add quality parity corpus comparing JasonShell result ordering against expected Flow-inspired outcomes.
- Add speed benchmarks for warm query, Everything query, cancellation, and UI publish path.

Required tests:

- Query parser: empty, whitespace, action keyword, path, quoted path, multi-term, requery.
- Settings: defaults, migration, invalid enum, persistence, secret rejection.
- Everything FFI wrapper: mock success, IPC error, memory error, missing DLL, not running, reset-on-error.
- Provider coordinator: parallel fanout, bounded in-flight, stale rejection, partial results, fallback.
- Ranking: selected-count boost, top-most override, priority boost, duplicate collapse, overflow cap.
- UI: immediate publish on input, sequence increments, fallback payload fetch, blur/interaction close behavior, top-right mode, centered mode.
- Activation: app, window, file, folder, command, calculator, web.

Required benchmarks:

- Renderer input handler and immediate payload build: p95 under 16 ms.
- Warm local search: p95 under 50 ms.
- Mock Everything provider search: overhead under 10 ms excluding SDK.
- Real Everything search on installed machine: p95 under 100 ms for representative 2+ character queries.
- Cancellation storm: 20 rapid queries must apply only the newest generation.
- Quality parity: curated query corpus must meet minimum expected top-3/top-5 results.

Validation commands:

- `npx tsc -p tsconfig.test.json`
- `node --test tests\searchPanelState.test.mjs tests\searchUxState.test.mjs tests\systemSearchState.test.mjs tests\searchSettings.test.mjs tests\searchQualityParity.test.mjs`
- `node tests\fixtures\searchParity\runSearchParity.mjs --fixture tests\fixtures\searchParity\corpus.json --report target\search-parity.json --strict`
- `cargo fmt --manifest-path src-tauri\Cargo.toml --check`
- `cargo test --manifest-path src-tauri\Cargo.toml search_sources`
- `cargo test --manifest-path src-tauri\Cargo.toml settings`
- Search benchmark command to be added during implementation
- Final `npm run validate`

### Phase 9: Live Smoke and Release Readiness

Files likely to touch:

- No new implementation files unless smoke finds blockers.
- Update durable specs only after implementation approval; not part of this planning task.

Tasks:

- Test with Everything absent.
- Test install prompt decline.
- Test installed but not running.
- Test running Everything with service enabled.
- Test standard user behavior.
- Test admin-required installer path.
- Test top-right and centered modes.
- Test file/folder activation and pin-folder action.
- Test rapid typing and closing surfaces.

Acceptance criteria:

- AC-9.1: Everything absent uses fallback and reports degraded health.
- AC-9.2: Everything running returns broad file/folder results quickly.
- AC-9.3: Declining install does not nag every keystroke.
- AC-9.4: Centered mode does not destabilize top/bottom AppBars.
- AC-9.5: Full validation passes after smoke follow-ups.

## Quality Parity Corpus

Create and approve a repeatable corpus before implementation. The corpus is a blocking fixture, not an informal checklist.

Fixture path:

- `tests\fixtures\searchParity\corpus.json`

Runner command:

- `node tests\fixtures\searchParity\runSearchParity.mjs --fixture tests\fixtures\searchParity\corpus.json --report target\search-parity.json --strict`

Fixture schema:

```json
{
  "version": 1,
  "queries": [
    {
      "id": "app-terminal-exact",
      "query": "terminal",
      "mode": "topRight",
      "providersRequired": ["apps"],
      "providersOptional": ["everything", "windowsSearch"],
      "expectedTop1Ids": ["app:windows-terminal"],
      "expectedTop3Ids": ["app:windows-terminal"],
      "expectedTop5Ids": ["app:windows-terminal", "command:open-control-plane"],
      "forbiddenProviderMonopoly": true,
      "skipWhen": [],
      "notes": "Exact app match must win."
    }
  ]
}
```

Required query groups:

- Apps: `terminal`, `code`, `notepad`, `settings`.
- Open windows: exact app title, partial title, recently active title.
- Files: exact filename, extension query, nested path query, recent document.
- Folders: user profile folders, Desktop, Downloads, project folders.
- Path search: `C:\`, `%APPDATA%`, partial folder path.
- Commands: JasonShell control plane, refresh search, hide search.
- Windows settings: display, bluetooth, network, apps.
- Calculator: `2+2`, `sqrt 144`, `15% of 80`.
- Web: `g cats`, `yt rust tauri`, if provider enabled.

Expected IDs:

- Every fixture row must include at least one `expectedTop1Ids`, `expectedTop3Ids`, or `expectedTop5Ids` entry unless it is explicitly negative coverage.
- Expected IDs must use stable prefixes: `app:`, `window:`, `file:`, `folder:`, `command:`, `setting:`, `calculator:`, `web:`, or `bookmark:`.
- Path-backed expected IDs must use canonical normalized paths or approved fixture aliases such as `folder:shell:Downloads`.
- Provider health expectations must be explicit for Everything-ready, Everything-absent, and fallback-only runs.

Pass/fail thresholds:

- Top-1 exact app/window/path match for obvious exact queries.
- At least 95% of non-skipped rows must satisfy their required Top-1/Top-3/Top-5 expectations.
- 100% of P0 rows must pass. P0 rows include exact app, exact command, exact path, activation safety, and provider-health fallback cases.
- At least 90% of Everything-ready file/folder rows must include the expected file or folder in Top-3.
- At least 95% of fallback-only rows must include expected app/command/window results in Top-5.
- 0 rows may violate dangerous-command confirmation/exclusion rules.
- No provider class may monopolize all top results for mixed queries unless exact-match quality justifies it and the fixture row marks `allowProviderMonopoly: true`.

Skip rules:

- Skip `requiresEverything` rows only when provider health is `unavailable`, `disabled`, or `adminRequired`; skipped rows must be reported with reason codes.
- Skip bookmark rows unless bookmark-provider privacy approval has passed.
- Skip web rows unless static URL templates have been approved.
- Skip real-speed Everything rows on machines without Everything installed, but mock-overhead and fallback parity rows must still run.
- Skips do not count as passes and cannot hide P0 failures unless the row explicitly names the missing external dependency.

Speed gates:

- Renderer input handler and immediate payload build p95 under 16 ms.
- Warm local search p95 under 50 ms.
- Mock Everything provider overhead p95 under 10 ms excluding SDK.
- Real Everything provider p95 under 100 ms for representative 2+ character queries when Everything is ready.
- Cancellation storm of 20 rapid queries applies only the newest generation.

Benchmark command:

- `node tests\fixtures\searchParity\runSearchParity.mjs --fixture tests\fixtures\searchParity\corpus.json --report target\search-parity.json --bench --strict`

## Risk and Unknowns

- R-1: Everything SDK license/redistribution requirements must be checked before bundling DLLs or installers.
- R-2: Everything installer and service setup can require admin rights; implementation must avoid silent elevation.
- R-3: Everything service exposes filenames only, but search still reveals local file names in JasonShell UI. Privacy expectations need confirmation.
- R-4: Everything content search may be slow; keep disabled by default.
- R-5: FFI thread safety is uncertain; serialize access initially.
- R-6: Everything 1.5 alpha has different HTTP/IPC options; this plan targets stable 1.4 plus SDK unless explicitly changed.
- R-7: x86 support may be unnecessary if JasonShell ships x64 only; decide before bundling x86 DLL.
- R-8: Centered search may need a new Tauri webview/window, capability updates, surface routing, and AppBar interaction smoke.
- R-9: Browser bookmark provider can touch private user data; scope and consent need explicit approval.
- R-10: Web suggestions can add network calls; first implementation should use static URL templates unless approved.
- R-11: Existing `searchPanel.ts` result kind union is narrow; migration must be coordinated with all render and activation paths.
- R-12: Current settings schema is version 1; adding required fields without defaults could break existing settings.
- R-13: Local machine currently lacks Everything, so real-speed benchmarks require install/setup or CI fixture strategy.
- R-14: Official versions can change; pinned installer/DLL versions and checksums must be refreshed at implementation time from official voidtools sources.

## Files To Touch By Implementation Area

Plan only:

- `voidtools_plan.md`

Rust search/settings:

- `src-tauri/src/settings.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/search_sources.rs`
- `src-tauri/src/search_sources/index.rs`
- `src-tauri/src/search_sources/scoring.rs`
- `src-tauri/src/search_sources/apps.rs`
- `src-tauri/src/search_sources/files.rs`
- `src-tauri/src/search_sources/windows_search.rs`
- New `src-tauri/src/search_sources/query.rs`
- New `src-tauri/src/search_sources/provider.rs`
- New `src-tauri/src/search_sources/everything.rs`
- New `src-tauri/src/search_sources/everything_ffi.rs`
- New `src-tauri/src/search_sources/everything_install.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/vendor/everything-sdk/...` if approved

Frontend/search UI:

- `src/components/TopBar.svelte`
- `src/components/TopBar.css`
- `src/components/SearchPanelSurface.svelte`
- Optional new `src/components/CenteredSearchSurface.svelte`
- `src/App.svelte`
- `src/lib/shellSurface.ts`
- `src/lib/searchPanel.ts`
- `src/lib/searchCatalog.ts`
- `src/lib/searchRanking.ts`
- `src/lib/systemSearch.ts`
- `src/features/search/searchUxState.ts`
- New `src/lib/searchSettings.ts`
- New `src/lib/searchQuery.ts` if needed
- `src/ipc/commands.ts`

Tauri windows/capabilities if centered surface is new:

- `src-tauri/src/search_panel.rs` or new centered panel module
- Tauri capability/config files that enumerate windows/commands

Tests and benchmarks:

- Existing search and settings Node tests
- New `tests/searchSettings.test.mjs`
- New `tests/searchQualityParity.test.mjs`
- New benchmark harness and scripts
- Rust unit tests in touched modules

## Approval Checklist Before Code Starts

- Approve Flow parity map: required, deferred, and rejected Flow behaviors.
- Approve quality parity corpus fixture, expected IDs, pass thresholds, skip rules, and exact runner command.
- Approve API contracts for settings, provider health, install consent/result, activation, and centered surface.
- Decide bundle vs download installer strategy.
- Decide pinned Everything version, official URL, artifact name, SHA-256 checksum, license approval, and provenance record.
- Decide x64-only vs x64/x86 SDK bundling.
- Decide whether centered hotkey uses existing `search-panel` or a new surface.
- Decide whether first implementation includes web/bookmark/calculator providers or defers them after Everything file/folder parity.
- Confirm whether JSON-only search settings are sufficient for first slice or settings-panel UI is required immediately.
- Confirm privacy wording for Everything filename exposure.
- Confirm every Phase 1-7 implementation slice has a red-test gate and exact red command before production changes.
