Summary: 124 Node test files, 77 Rust files with tests, 3 smoke/test scripts, 1620 discovered individual describe/test entries.

# Test Case Audit

Scope: `tests/*.test.mjs`, package test scripts/config, Rust tests run by `cargo test`, and clearly named smoke/test PowerShell scripts under `scripts/`.
ASCII-only audit artifact. No production or test behavior changed.

## Test conventions discovered

- `npm run test:node`: runs `node scripts/clean-dist-tests.mjs && tsc -p tsconfig.test.json && node --test tests/*.test.mjs`.
- `npm run test:search`: aliases `npm run test:node`.
- `npm run cargo:test`: runs `cargo test --manifest-path src-tauri/Cargo.toml`.
- `npm run smoke:runtime`: runs `scripts/runtime-smoke.ps1 -DryRun`.
- `npm run smoke:fullscreen`: runs `scripts/smoke-fullscreen-appbar.ps1`.
- `npm run validate`: runs Svelte check, build, Node tests, Rust tests, then cargo check.

## JavaScript Node test files

### tests/audioAutoRefresh.test.mjs

Audits audio auto refresh behavior through 4 discovered test entries.

- Line 9: test `audio refresh has typed reasons without dead cross-window event contract` - Covers Audio refresh has typed reasons without dead cross-window event contract.
- Line 16: test `audio panel does not subscribe to removed refresh event contract` - Covers Audio panel does not subscribe to removed refresh event contract.
- Line 26: test `audio panel debounces event bursts while keeping manual refresh button` - Covers Audio panel debounces event bursts while keeping manual refresh button.
- Line 36: test `audio panel keeps a bounded polling fallback only while visible` - Covers Audio panel keeps a bounded polling fallback only while visible.

### tests/audioControls.test.mjs

Audits audio controls behavior through 5 discovered test entries.

- Line 20: test `audio wrapper exposes stable command constants and clamps slider values` - Covers Audio wrapper exposes stable command constants and clamps slider values.
- Line 38: test `top bar adds sound control left of time with immediate audio command calls` - Covers Top bar adds sound control left of time with immediate audio command calls.
- Line 52: test `audio panel surface owns usable dialog controls and immediate audio commands` - Covers Audio panel surface owns usable dialog controls and immediate audio commands.
- Line 69: test `audio panel ignores stale failed commands and reverts current failures` - Covers Audio panel ignores stale failed commands and reverts current failures.
- Line 82: test `audio panel is a dedicated top-bar anchored webview so controls are not clipped by the compact top bar` - Covers Audio panel is a dedicated top-bar anchored webview so controls are not clipped by the compact top bar.

### tests/automationProviders.test.mjs

Audits automation providers behavior through 4 discovered test entries.

- Line 28: test `automation IPC contracts are registered without enabling arbitrary execution` - Covers Automation IPC contracts are registered without enabling arbitrary execution.
- Line 45: test `automation wrapper enforces safe forwarding contract shape` - Covers Automation wrapper enforces safe forwarding contract shape.
- Line 74: test `provider contract rejects secret and executable/plugin config in TS wrapper` - Covers Provider contract rejects secret and executable/plugin config in TS wrapper.
- Line 92: test `provider IPC contracts are registered and deny arbitrary plugin execution` - Covers Provider IPC contracts are registered and deny arbitrary plugin execution.

### tests/backendBlockingLockBoundaries.test.mjs

Audits backend blocking lock boundaries behavior through 6 discovered test entries.

- Line 29: test `stack archive extraction and recursive file ops run behind blocking task boundaries` - Covers Stack archive extraction and recursive file ops run behind blocking task boundaries.
- Line 51: test `process manager icon extraction is outside cache mutex guard` - Covers Process manager icon extraction is outside cache mutex guard.
- Line 62: test `bounded icon cache helper enforces ttl and lru-like capacity` - Covers Bounded icon cache helper enforces ttl and lru-like capacity.
- Line 73: test `task preview window operations happen after runtime guard is dropped` - Covers Task preview window operations happen after runtime guard is dropped.
- Line 82: test `appbar activation and cleanup native side effects happen outside runtime mutex guard` - Covers Appbar activation and cleanup native side effects happen outside runtime mutex guard.
- Line 98: test `appbar cleanup planning extracts guards but stop and join stay outside planner` - Covers Appbar cleanup planning extracts guards but stop and join stay outside planner.

### tests/backendBlockingLockBoundariesP6.test.mjs

Audits backend blocking lock boundaries p6 behavior through 5 discovered test entries.

- Line 9: test `P6 phase 1 Stack archive extraction runs process status behind spawn_blocking` - Covers P6 phase 1 Stack archive extraction runs process status behind spawn blocking.
- Line 31: test `P6 phase 1 archive spawn_blocking remains while Phase 1 safety owns timeout/resource hardening` - Covers P6 phase 1 archive spawn blocking remains while Phase 1 safety owns timeout/resource hardening.
- Line 48: test `P6 phase 1 Stack recursive paste and delete commands use async blocking boundaries` - Covers P6 phase 1 Stack recursive paste and delete commands use async blocking boundaries.
- Line 67: test `P6 phase 1 keeps cheap stack commands off the new blocking job boundary` - Covers P6 phase 1 keeps cheap stack commands off the new blocking job boundary.
- Line 74: test `P6 phase 1 clipboard source has RAII guards and drop-effect ordering` - Covers P6 phase 1 clipboard source has RAII guards and drop-effect ordering.

### tests/bootstrapWindowsContract.test.mjs

Audits bootstrap windows contract behavior through 2 discovered test entries.

- Line 8: test `windows bootstrap script installs required prerequisites and launches tauri dev` - Covers Windows bootstrap script installs required prerequisites and launches tauri dev.
- Line 40: test `readme documents bootstrap script and lockfile-safe install path` - Covers Readme documents bootstrap script and lockfile-safe install path.

### tests/bottomBarPreviewRequestId.test.mjs

Audits bottom bar preview request id behavior through 1 discovered test entries.

- Line 7: test `bottom bar allocates preview request ids from shared native state` - Covers Bottom bar allocates preview request ids from shared native state.

### tests/centeredSearchSurface.test.mjs

Audits centered search surface behavior through 11 discovered test entries.

- Line 11: test `centered search command is registered in the frontend IPC map` - Covers Centered search command is registered in the frontend IPC map.
- Line 16: test `top bar opens configured centered search from focus and typing without Ctrl+K reliance` - Covers Top bar opens configured centered search from focus and typing without Ctrl+K reliance.
- Line 34: test `top-bar direct input path publishes lightweight pending state and queues deferred search work` - Covers Top-bar direct input path publishes lightweight pending state and queues deferred search work.
- Line 57: test `centered search surface contract accepts screen-center combobox payloads` - Covers Centered search surface contract accepts screen-center combobox payloads.
- Line 74: test `search panel surface owns centered input and resize grip wiring` - Covers Search panel surface owns centered input and resize grip wiring.
- Line 84: test `centered search surface input owns combobox popup and status description` - Covers Centered search surface input owns combobox popup and status description.
- Line 104: test `centered search surface keeps listbox options non-tabbable and pin control out of tab flow` - Covers Centered search surface keeps listbox options non-tabbable and pin control out of tab flow.
- Line 115: test `centered search surface keeps selection and activation focus on the input` - Covers Centered search surface keeps selection and activation focus on the input.
- Line 132: test `centered search surface targets top-bar explicitly for cross-window search intents` - Covers Centered search surface targets top-bar explicitly for cross-window search intents.
- Line 152: test `centered search surface keeps a local optimistic query draft while backend search catches up` - Covers Centered search surface keeps a local optimistic query draft while backend search catches up.
- Line 167: test `centered search surface hides immediately on escape and only refocuses when panel focus is elsewhere` - Covers Centered search surface hides immediately on escape and only refocuses when panel focus is elsewhere.

### tests/changelogPolicy.test.mjs

Audits changelog policy behavior through 2 discovered test entries.

- Line 9: test `master spec no longer owns per-request change ledger protocol` - Covers Master spec no longer owns per-request change ledger protocol.
- Line 16: test `dedicated changelog policy and agent instructions route history out of master spec` - Covers Dedicated changelog policy and agent instructions route history out of master spec.

### tests/changelogPolicyHygiene.test.mjs

Audits changelog policy hygiene behavior through 3 discovered test entries.

- Line 10: test `master spec stays behavior-focused and delegates per-change history to changelog policy` - Covers Master spec stays behavior-focused and delegates per-change history to changelog policy.
- Line 19: test `dedicated changelog policy owns future ledger protocol without rewriting history` - Covers Dedicated changelog policy owns future ledger protocol without rewriting history.
- Line 31: test `repo agent instructions route changelog work out of master spec` - Covers Repo agent instructions route changelog work out of master spec.

### tests/commandPanelCloseButton.test.mjs

Audits command panel close button behavior through 1 discovered test entries.

- Line 8: test `command panel close button is accessible and styled like destructive shell close controls` - Covers Command panel close button is accessible and styled like destructive shell close controls.

### tests/commandPanelTheme.test.mjs

Audits command panel theme behavior through 2 discovered test entries.

- Line 8: test `quick commands surface uses shared shell theme tokens for primary surfaces` - Covers Quick commands surface uses shared shell theme tokens for primary surfaces.
- Line 46: test `quick commands keep compact icon controls and context-only history/edit actions` - Covers Quick commands keep compact icon controls and context-only history/edit actions.

### tests/commandPanelWiring.test.mjs

Audits command panel wiring behavior through 12 discovered test entries.

- Line 26: test `command panel is routed as a dedicated auxiliary shell surface` - Covers Command panel is routed as a dedicated auxiliary shell surface.
- Line 46: test `command panel contracts and wrappers use constant-backed IPC and event names` - Covers Command panel contracts and wrappers use constant-backed IPC and event names.
- Line 63: test `top bar command button is left of tray button and enforces popup exclusivity` - Covers Top bar command button is left of tray button and enforces popup exclusivity.
- Line 76: test `command panel surface includes compact list actions, resize controls, and command-block editor flow` - Covers Command panel surface includes compact list actions, resize controls, and command-block editor flow.
- Line 228: test `quick command ordering uses versioned migration, authoritative arrays, and one mutation pipeline` - Covers Quick command ordering uses versioned migration, authoritative arrays, and one mutation pipeline.
- Line 286: test `quick command create action uses copied add icon while retaining accessible name and behavior` - Covers Quick command create action uses copied add icon while retaining accessible name and behavior.
- Line 298: test `quick command saved-command actions use copied icons while retaining names and handlers` - Covers Quick command saved-command actions use copied icons while retaining names and handlers.
- Line 320: test `command panel Rust placement clamps inside monitor work area and shrinks only when needed` - Covers Command panel Rust placement clamps inside monitor work area and shrinks only when needed.
- Line 330: test `command panel close lifecycle avoids resize and minimize/maximize disappearance` - Covers Command panel close lifecycle avoids resize and minimize/maximize disappearance.
- Line 341: test `command panel transcript host remains a labelled read-only focusable log with copy and context handlers` - Covers Command panel transcript host remains a labelled read-only focusable log with copy and context handlers.
- Line 351: test `command panel keeps stopping run visible with disabled stop affordance` - Covers Command panel keeps stopping run visible with disabled stop affordance.
- Line 359: test `command panel terminal events clear active quick command state immediately` - Covers Command panel terminal events clear active quick command state immediately.

### tests/contextMenuPosition.test.mjs

Audits context menu position behavior through 6 discovered test entries.

- Line 20: test `keeps a context menu at the requested point when it fits` - Covers Keeps a context menu at the requested point when fits.
- Line 31: test `flips and clamps a context menu into visible viewport space` - Covers Flips and clamps a context menu into visible viewport space.
- Line 42: test `uses padding when the menu is larger than the available viewport` - Covers Uses padding when the menu is larger than the available viewport.
- Line 54: test `returns scrollable placement when menu is taller than the available viewport` - Covers Returns scrollable placement when menu is taller than the available viewport.
- Line 66: test `stack popup wires computed context menu max-height CSS variables` - Covers Stack popup wires computed context menu max-height CSS variables.
- Line 84: test `stack submenu stays attached to trigger and scrolls internally` - Covers Stack submenu stays attached to trigger and scrolls internally.

### tests/contractsSettings.test.mjs

Audits contracts settings behavior through 10 discovered test entries.

- Line 127: test `frontend IPC contracts expose command, event, and surface constants for future wrappers` - Covers Frontend IPC contracts expose command, event, and surface constants for future wrappers.
- Line 182: test `event registry documents subset authority and excludes removed audio refresh event` - Covers Event registry documents subset authority and excludes removed audio refresh event.
- Line 189: test `shipped shell window surfaces have matching frontend routes, IPC registry entries, and capability targets` - Covers Shipped shell window surfaces have matching frontend routes, IPC registry entries, and capability targets.
- Line 236: test `surface parity failure output names future missing capability targets` - Covers Surface parity failure output names future missing capability targets.
- Line 248: test `Rust event contracts are authoritative and cover frontend event constants` - Covers Rust event contracts are authoritative and cover frontend event constants.
- Line 320: test `dead audio refresh event contract is not exposed without a Rust emitter` - Covers Dead audio refresh event contract is not exposed without a Rust emitter.
- Line 329: test `settings wrapper declares versioned schema and stable command names` - Covers Settings wrapper declares versioned schema and stable command names.
- Line 366: test `settings wrapper refuses secret-like keys before persistence` - Covers Settings wrapper refuses secret-like keys before persistence.
- Line 381: test `frontend diagnostics contract redacts sensitive diagnostic data` - Covers Frontend diagnostics contract redacts sensitive diagnostic data.
- Line 418: test `backend settings and diagnostics commands are registered with hardened app config` - Covers Backend settings and diagnostics commands are registered with hardened app config.

### tests/controlPlaneRouting.test.mjs

Audits control plane routing behavior through 1 discovered test entries.

- Line 5: test `control-plane is routed as a hidden persistent shell surface with safe IPC` - Covers Control-plane is routed as a hidden persistent shell surface with safe IPC.

### tests/controlPlaneState.test.mjs

Audits control plane state behavior through 4 discovered test entries.

- Line 53: test `derives settings and dashboard sections from existing frontend contracts without rendering secrets` - Covers Derives settings and dashboard sections from existing frontend contracts without rendering secrets.
- Line 141: test `bounds rendering data while preserving overflow counts and provider budgets` - Covers Bounds rendering data while preserving overflow counts and provider budgets.
- Line 177: test `filters sections and exposes keyboard navigation actions for accessible control-plane tabs` - Covers Filters sections and exposes keyboard navigation actions for accessible control-plane tabs.
- Line 197: test `control-plane component includes semantic settings and dashboard regions without adding IPC authority` - Covers Control-plane component includes semantic settings and dashboard regions without adding IPC authority.

### tests/devTools.test.mjs

Audits dev tools behavior through 6 discovered test entries.

- Line 55: test `developer tooling IPC constants and task events are centralized and registered` - Covers Developer tooling IPC constants and task events are centralized and registered.
- Line 79: test `top bar no longer exposes project context launcher` - Covers Top bar no longer exposes project context launcher.
- Line 89: test `tool launch requests derive safe terminal and editor argv templates from workspace state` - Covers Tool launch requests derive safe terminal and editor argv templates from workspace state.
- Line 105: test `task requests and history rankings preserve workspace process metadata references` - Covers Task requests and history rankings preserve workspace process metadata references.
- Line 160: test `task request helper refuses undeclared task objects instead of minting argv payloads` - Covers Task request helper refuses undeclared task objects instead of minting argv payloads.
- Line 169: test `Rust tooling foundation rejects command-line execution seams and registers capabilities` - Covers Rust tooling foundation rejects command-line execution seams and registers capabilities.

### tests/developerProviders.test.mjs

Audits developer providers behavior through 5 discovered test entries.

- Line 17: test `bounds developer providers per source and across the merged result set` - Covers Bounds developer providers per source and across the merged result set.
- Line 44: test `ranks active workspace matches above external files while preserving provider groups` - Covers Ranks active workspace matches above external files while preserving provider groups.
- Line 80: test `filters scoped providers to the active workspace when requested` - Covers Filters scoped providers to the active workspace when requested.
- Line 125: test `exposes saved-search persistence and scope contracts` - Covers Exposes saved-search persistence and scope contracts.
- Line 164: test `static developer dashboard command is active through top-bar activation` - Covers Static developer dashboard command is active through top-bar activation.

### tests/distTestsHygiene.test.mjs

Audits dist tests hygiene behavior through 3 discovered test entries.

- Line 10: test `test:node cleans repo-local dist-tests before compiling helpers` - Covers Test:node cleans repo-local dist-tests before compiling helpers.
- Line 19: test `generated dist-tests output stays ignored and documented as cleaned before compile` - Covers Generated dist-tests output stays ignored and documented as cleaned before compile.
- Line 25: test `tests do not import root-level stale dist-tests outputs` - Covers Tests do not import root-level stale dist-tests outputs.

### tests/explorerTaskbarSuppression.test.mjs

Audits explorer taskbar suppression behavior through 1 discovered test entries.

- Line 7: test `explorer suppression source contract exposes owned taskbar collection and identity checks` - Covers Explorer suppression source contract exposes owned taskbar collection and identity checks.

### tests/folderDrag.test.mjs

Audits folder drag behavior through 6 discovered test entries.

- Line 11: test `normalizes local file uri paths` - Covers Normalizes local file uri paths.
- Line 22: test `normalizes unc file uri paths` - Covers Normalizes unc file uri paths.
- Line 29: test `encodes windows paths as file uris` - Covers Encodes windows paths as file uris.
- Line 34: test `extracts every path from multi-line uri lists` - Covers Extracts every path from multi-line uri lists.
- Line 51: test `publishes shared folder drag payloads for top-bar drops` - Covers Publishes shared folder drag payloads for top-bar drops.
- Line 68: test `treats native Explorer Files drags as folder payload candidates` - Covers Treats native Explorer Files drags as folder payload candidates.

### tests/frontendUiPolicy.test.mjs

Audits frontend ui policy behavior through 3 discovered test entries.

- Line 30: test `app and component UI sources do not use active gradients` - Covers App and component UI sources do not use active gradients.
- Line 43: test `stack browser exposes editable path input and clickable segment navigation` - Covers Stack browser exposes editable path input and clickable segment navigation.
- Line 55: test `stack browser sort headers expose active class helper and aria sort wiring` - Covers Stack browser sort headers expose active class helper and aria sort wiring.

### tests/globalCloseGlyphUniformity.test.mjs

Audits global close glyph uniformity behavior through 1 discovered test entries.

- Line 18: test `global close glyphs use shared close icon and search icon is official` - Covers Global close glyphs use shared close icon and search icon is official.

### tests/masterSpecSearchHygiene.test.mjs

Audits master spec search hygiene behavior through 2 discovered test entries.

- Line 8: it `## Change Ledger` - Covers ## Change Ledger.
- Line 11: test `current search spec bans stale deferred/coalesced visible query scheduling` - Covers Current search spec bans stale deferred/coalesced visible query scheduling.

### tests/meltMigrationWiring.test.mjs

Audits melt migration wiring behavior through 7 discovered test entries.

- Line 30: test `Melt UI migration uses the Svelte 5 Melt package only` - Covers Melt UI migration uses the Svelte 5 Melt package only.
- Line 42: test `shared JasonShell primitives wrap Melt builders` - Covers Shared JasonShell primitives wrap Melt builders.
- Line 99: test `settings and control-plane surfaces consume Melt-backed controls without collapsing Tauri surfaces` - Covers Settings and control-plane surfaces consume Melt-backed controls without collapsing Tauri surfaces.
- Line 138: test `search panel keeps pinning input-owned while safe buttons use Melt-backed controls` - Covers Search panel keeps pinning input-owned while safe buttons use Melt-backed controls.
- Line 162: test `stack-popup safe controls use MeltActionButton while risky grid/ref controls stay raw by design` - Covers Stack-popup safe controls use MeltActionButton while risky grid/ref controls stay raw by design.
- Line 181: test `top-bar action and pinned-folder controls use Melt-backed buttons without breaking shell flows` - Covers Top-bar action and pinned-folder controls use Melt-backed buttons without breaking shell flows.
- Line 198: test `bottom-bar command buttons use Melt-backed action buttons without changing taskbar semantics` - Covers Bottom-bar command buttons use Melt-backed action buttons without changing taskbar semantics.

### tests/multiFixPhase6SpecValidation.test.mjs

Audits multi fix phase6spec validation behavior through 2 discovered test entries.

- Line 7: test `master spec records multi-fix phase 5 and phase 6 durable behavior` - Covers Master spec records multi-fix phase 5 and phase 6 durable behavior.
- Line 23: test `master spec validation notes include required phase acceptance checks` - Covers Master spec validation notes include required phase acceptance checks.

### tests/overlayDismissalWiring.test.mjs

Audits overlay dismissal wiring behavior through 5 discovered test entries.

- Line 5: test `stack browser delete confirmation stays inside stack popup webview` - Covers Stack browser delete confirmation stays inside stack popup webview.
- Line 26: test `stack browser properties suppresses one focus-loss without delete focus restore hold` - Covers Stack browser properties suppresses one focus-loss without delete focus restore hold.
- Line 47: test `stack browser exposes persisted resize grip and resize command wiring` - Covers Stack browser exposes persisted resize grip and resize command wiring.
- Line 74: test `search panel has outside-dismiss and result-interaction handshake` - Covers Search panel has outside-dismiss and result-interaction handshake.
- Line 93: test `outside-click and blur search dismissal route through reset-closing path` - Covers Outside-click and blur search dismissal route through reset-closing path.

### tests/performanceBaselineContract.test.mjs

Audits performance baseline contract behavior through 27 discovered test entries.

- Line 91: test `performance artifact root is timestamped and ignored` - Covers Performance artifact root is timestamped and ignored.
- Line 102: test `performance artifact root timestamp cannot collide within the same second` - Covers Performance artifact root timestamp cannot collide within the same second.
- Line 117: test `launch mode contract separates release acceptance from dev diagnostics` - Covers Launch mode contract separates release acceptance from dev diagnostics.
- Line 130: test `scenario matrix is exactly the seven Plan 01 scenarios` - Covers Scenario matrix is exactly the seven Plan 01 scenarios.
- Line 140: test `scenario validation rejects arbitrary seven-item overrides by exact equality` - Covers Scenario validation rejects arbitrary seven-item overrides by exact equality.
- Line 149: test `scenario validation rejects reordering, not membership-only set comparison` - Covers Scenario validation rejects reordering, not membership-only set comparison.
- Line 161: test `each scenario runs exactly three times` - Covers Each scenario runs exactly three times.
- Line 169: test `per-run JSON schema includes mode, timestamps, scenario, status, metrics, and I/O availability` - Covers Per-run JSON schema includes mode, timestamps, scenario, status, metrics, and I/O availability.
- Line 193: test `run status transitions only after measurable scenario, with blocked and unavailable paths explicit` - Covers Run status transitions only after measurable scenario, with blocked and unavailable paths explicit.
- Line 206: test `all Plan 01 scenarios have real handler or prereq logic, not source anchors` - Covers All Plan 01 scenarios have real handler or prereq logic, not source anchors.
- Line 218: test `release binary discovery is deterministic produced jason-shell.exe executable only` - Covers Release binary discovery is deterministic produced jason-shell.exe executable only.
- Line 239: test `release binary discovery dynamically finds produced src-tauri target release jason-shell.exe` - Covers Release binary discovery dynamically finds produced src-tauri target release jason-shell.exe.
- Line 256: test `explicit release binary path must resolve under allowed release roots before return` - Covers Explicit release binary path must resolve under allowed release roots before return.
- Line 282: test `release root canonicalization tolerates absent target release roots` - Covers Release root canonicalization tolerates absent target release roots.
- Line 296: test `timeout parameter is executable and used around scenario launch` - Covers Timeout parameter is executable and used around scenario launch.
- Line 309: test `timeout enforcement has no unreachable anchors or Start-Job pid leak` - Covers Timeout enforcement has no unreachable anchors or Start-Job pid leak.
- Line 323: test `CPU metric is interval delta, not single cumulative snapshot` - Covers CPU metric is interval delta, not single cumulative snapshot.
- Line 333: test `runner samples CPU from live process before waiting for exit, then cleans up process tree` - Covers Runner samples CPU from live process before waiting for exit, then cleans up process tree.
- Line 349: test `readiness settle loop exits while app is alive before deadline instead of waiting alive until deadline` - Covers Readiness settle loop exits while app is alive before deadline instead of waiting alive until deadline.
- Line 361: test `CPU schema exposes explicit units and rate fields` - Covers CPU schema exposes explicit units and rate fields.
- Line 371: test `dev launch and cleanup cannot orphan child process trees` - Covers Dev launch and cleanup cannot orphan child process trees.
- Line 384: test `dev cleanup kills spawned child tree even when npm parent exits first` - Covers Dev cleanup kills spawned child tree even when npm parent exits first.
- Line 399: test `median summary separates release acceptance and dev diagnostics` - Covers Median summary separates release acceptance and dev diagnostics.
- Line 409: test `summary exposes pass blocked error counts and denies release acceptance unless all release scenarios measured` - Covers Summary exposes pass blocked error counts and denies release acceptance unless all release scenarios measured.
- Line 421: test `budgets are measured-baseline-relative and contain no fabricated numeric thresholds` - Covers Budgets are measured-baseline-relative and contain no fabricated numeric thresholds.
- Line 432: test `residual risk and manual prereq output supports blocked/not measured scenarios` - Covers Residual risk and manual prereq output supports blocked/not measured scenarios.
- Line 442: test `manual and complex scenarios are confirmable or concretely detectable, not permanently false` - Covers Manual and complex scenarios are confirmable or concretely detectable, not permanently false.

### tests/persistentSurfaceLifecycle.test.mjs

Audits persistent surface lifecycle behavior through 11 discovered test entries.

- Line 78: test `audio panel stays idle on hidden persistent mount` - Covers Audio panel stays idle on hidden persistent mount.
- Line 87: test `audio panel own close event stops visible polling state` - Covers Audio panel own close event stops visible polling state.
- Line 95: test `audio refresh events do not schedule hidden-panel work` - Covers Audio refresh events do not schedule hidden-panel work.
- Line 104: test `process manager close invalidates in-flight refreshes` - Covers Process manager close invalidates in-flight refreshes.
- Line 113: test `async unlistener registry calls late-resolving unlisten exactly once after destroy` - Covers Async unlistener registry calls late-resolving unlisten exactly once after destroy.
- Line 133: test `audio panel open refreshes immediately before starting polling` - Covers Audio panel open refreshes immediately before starting polling.
- Line 143: test `stack popup documents the disposed guard reference pattern` - Covers Stack popup documents the disposed guard reference pattern.
- Line 155: test `${componentName} guards async Tauri listener cleanup after destroy` - Covers ${componentName} guards async Tauri listener cleanup after destroy.
- Line 164: test `${componentName} uses guarded async listener lifecycle for persistent surface mount` - Covers ${componentName} uses guarded async listener lifecycle for persistent surface mount.
- Line 169: test `command panel async callbacks return when disposed before mutating state` - Covers Command panel async callbacks return when disposed before mutating state.
- Line 181: test `tray panel async callbacks return when disposed before mutating state` - Covers Tray panel async callbacks return when disposed before mutating state.

### tests/persistentTerminalPanel.test.mjs

Audits persistent terminal panel behavior through 18 discovered test entries.

- Line 33: test `persistent terminal is its own shell surface and uses delayed first-open startup` - Covers Persistent terminal is its own shell surface and uses delayed first-open startup.
- Line 50: test `top bar terminal button sits before quick commands and toggles terminal panel` - Covers Top bar terminal button sits before quick commands and toggles terminal panel.
- Line 78: test `terminal top bar icon keeps completion state without prompt glyph text` - Covers Terminal top bar icon keeps completion state without prompt glyph text.
- Line 85: test `terminal top bar animation is reserved for important submitted commands` - Covers Terminal top bar animation is reserved for important submitted commands.
- Line 96: test `Stack Browser no longer owns the visible terminal panel xterm internals` - Covers Stack Browser no longer owns the visible terminal panel xterm internals.
- Line 103: test `terminal panel owns xterm, startup status, errors, and poll fallback` - Covers Terminal panel owns xterm, startup status, errors, and poll fallback.
- Line 223: test `material symbol icon set keeps official 960-grid paths and equal inline sizing` - Covers Material symbol icon set keeps official 960-grid paths and equal inline sizing.
- Line 270: test `terminal tabs are backend-session authoritative and are not capped at four` - Covers Terminal tabs are backend-session authoritative and are not capped at four.
- Line 286: test `terminal tab plus creates whole-page tabs while split right and down are separate toolbar buttons` - Covers Terminal tab plus creates whole-page tabs while split right and down are separate toolbar buttons.
- Line 311: test `terminal recursive split panes keep a source-contract tree and pane focus does not collapse layout` - Covers Terminal recursive split panes keep a source-contract tree and pane focus does not collapse layout.
- Line 330: test `terminal tabs are horizontal rectangular tabs` - Covers Terminal tabs are horizontal rectangular tabs.
- Line 342: test `terminal tab close lives in the header and replaces the status dot on hover` - Covers Terminal tab close lives in the header and replaces the status dot on hover.
- Line 356: test `terminal tab close does not recreate a replacement session while other tabs exist` - Covers Terminal tab close does not recreate a replacement session while other tabs exist.
- Line 368: test `terminal split and tab lifecycle guards stale runtimes and DOM identity` - Covers Terminal split and tab lifecycle guards stale runtimes and DOM identity.
- Line 385: test `terminal tabs replay retained output and remember hidden-tab chunks` - Covers Terminal tabs replay retained output and remember hidden-tab chunks.
- Line 404: test `phase 7 terminal panel owns real per-pane xterm runtimes and split resize` - Covers Phase 7 terminal panel owns real per-pane xterm runtimes and split resize.
- Line 437: test `persistent terminal output is routed to the terminal panel window` - Covers Persistent terminal output is routed to the terminal panel window.
- Line 447: test `contracts list terminal panel surface and commands` - Covers Contracts list terminal panel surface and commands.

### tests/processManagerAccessibility.test.mjs

Audits process manager accessibility behavior through 3 discovered test entries.

- Line 32: test `process manager keeps auto-refresh grid silent and uses dedicated status live region` - Covers Process manager keeps auto-refresh grid silent and uses dedicated status live region.
- Line 45: test `process manager auto refresh does not announce every successful timer tick` - Covers Process manager auto refresh does not announce every successful timer tick.
- Line 58: test `process manager filter and grid expose visible focus styles` - Covers Process manager filter and grid expose visible focus styles.

### tests/processManagerCloseButton.test.mjs

Audits process manager close button behavior through 1 discovered test entries.

- Line 30: test `process manager has an accessible task-preview-style red X close button` - Covers Process manager has an accessible task-preview-style red X close button.

### tests/processManagerState.test.mjs

Audits process manager state behavior through 13 discovered test entries.

- Line 28: test `sortProcesses sorts names with numeric locale semantics` - Covers SortProcesses sorts names with numeric locale semantics.
- Line 35: test `sortProcesses sorts metrics descending and pushes unknown values last` - Covers SortProcesses sorts metrics descending and pushes unknown values last.
- Line 50: test `classifies taskbar-active applications before conservative Windows process heuristics` - Covers Classifies taskbar-active applications before conservative Windows process heuristics.
- Line 62: test `sortProcesses keeps Applications, Background processes, then Windows processes while sorting inside groups` - Covers SortProcesses keeps Applications, Background processes, then Windows processes while sorting inside groups.
- Line 76: test `orderProcessRefresh preserves volatile metric reading order while refreshing row values` - Covers OrderProcessRefresh preserves volatile metric reading order while refreshing row values.
- Line 98: test `orderProcessRefresh preserves volatile order within groups while group order stays stable` - Covers OrderProcessRefresh preserves volatile order within groups while group order stays stable.
- Line 120: test `detects volatile process sort columns` - Covers Detects volatile process sort columns.
- Line 128: test `sortProcesses sorts process start time with unknown values last by default` - Covers SortProcesses sorts process start time with unknown values last by default.
- Line 135: test `nextProcessSortState toggles same column and chooses metric defaults` - Covers NextProcessSortState toggles same column and chooses metric defaults.
- Line 146: test `formatters keep compact task-manager style labels` - Covers Formatters keep compact task-manager style labels.
- Line 166: test `aggregateProcessMetrics sums visible task-manager percentages and clamps totals` - Covers AggregateProcessMetrics sums visible task-manager percentages and clamps totals.
- Line 190: test `processDeveloperSummary includes ports workspace parent descendants and command line` - Covers ProcessDeveloperSummary includes ports workspace parent descendants and command line.
- Line 213: test `killConfirmationIncludesImmutableIdentity plan carries creation time and normalized image path` - Covers KillConfirmationIncludesImmutableIdentity plan carries creation time and normalized image path.

### tests/processManagerUxState.test.mjs

Audits process manager ux state behavior through 12 discovered test entries.

- Line 25: test `filters processes by name, pid, parent pid, path, and status tokens` - Covers Filters processes by name, pid, parent pid, path, and status tokens.
- Line 33: test `builds tree-aware process rows while preserving supplied sibling order` - Covers Builds tree-aware process rows while preserving supplied sibling order.
- Line 44: test `promotes taskbar-active processes to readable top-level rows` - Covers Promotes taskbar-active processes to readable top-level rows.
- Line 55: test `builds Task Manager process groups in stable visual order` - Covers Builds Task Manager process groups in stable visual order.
- Line 66: test `filtering keeps all process groups visible while rows are scoped to matches` - Covers Filtering keeps all process groups visible while rows are scoped to matches.
- Line 74: test `process group expansion defaults open and toggles per group independently` - Covers Process group expansion defaults open and toggles per group independently.
- Line 90: test `enriches processes with taskbar-active window metadata` - Covers Enriches processes with taskbar-active window metadata.
- Line 121: test `extracts unique taskbar-active process ids` - Covers Extracts unique taskbar-active process ids.
- Line 129: test `normalizes metric bars and safe kill confirmation state` - Covers Normalizes metric bars and safe kill confirmation state.
- Line 145: test `plans kill-tree guardrails without enabling unsafe default tree kill` - Covers Plans kill-tree guardrails without enabling unsafe default tree kill.
- Line 162: test `builds behavioral kill confirmation payload with identity copied from plan` - Covers Builds behavioral kill confirmation payload with identity copied from plan.
- Line 183: test `keeps classified process kill errors visible and bounded` - Covers Keeps classified process kill errors visible and bounded.

### tests/processManagerWiring.test.mjs

Audits process manager wiring behavior through 1 discovered test entries.

- Line 5: test `process manager surface and commands are routed through app and Rust command table` - Covers Process manager surface and commands are routed through app and Rust command table.

### tests/productTruthDocs.test.mjs

Audits product truth docs behavior through 3 discovered test entries.

- Line 12: test `README pairs required product truths with near-copy language` - Covers README pairs required product truths with near-copy language.
- Line 19: test `master_spec repeats the same four truths` - Covers Master spec repeats the same four truths.
- Line 26: test `control plane user-facing copy keeps workspace startup/restoration and automation forwarding constrained` - Covers Control plane user-facing copy keeps workspace startup/restoration and automation forwarding constrained.

### tests/quickCommands.test.mjs

Audits quick commands behavior through 28 discovered test entries.

- Line 36: test `quick command wrapper exposes stable mode contract and defaults` - Covers Quick command wrapper exposes stable mode contract and defaults.
- Line 45: test `quick command settings coercion normalizes entries and validates security rules` - Covers Quick command settings coercion normalizes entries and validates security rules.
- Line 88: test `quick command settings support sequential command blocks` - Covers Quick command settings support sequential command blocks.
- Line 131: test `quick command args textarea helpers preserve argv-per-line semantics` - Covers Quick command args textarea helpers preserve argv-per-line semantics.
- Line 136: test `quick command block textarea helpers preserve command-per-line semantics` - Covers Quick command block textarea helpers preserve command-per-line semantics.
- Line 147: test `quick command run request validates id and wrapper uses IPC constants` - Covers Quick command run request validates id and wrapper uses IPC constants.
- Line 163: test `quick command order helpers move by stable id and retain equal-label order` - Covers Quick command order helpers move by stable id and retain equal-label order.
- Line 178: test `quick command coercion preserves authoritative order and detects legacy order version` - Covers Quick command coercion preserves authoritative order and detects legacy order version.
- Line 191: test `quick command order version normalization matches Rust validation` - Covers Quick command order version normalization matches Rust validation.
- Line 205: test `quick command URL opener is command-panel only and rejects unsafe URLs` - Covers Quick command URL opener is command-panel only and rejects unsafe URLs.
- Line 217: test `quick command URL validator accepts http and https and rejects unsafe forms` - Covers Quick command URL validator accepts http and https and rejects unsafe forms.
- Line 226: test `quick command settings discard legacy history without a run id` - Covers Quick command settings discard legacy history without a run id.
- Line 234: test `quick command pending input derives from transcript request markers` - Covers Quick command pending input derives from transcript request markers.
- Line 261: test `quick command input helpers bound max length and trim unicode by code point` - Covers Quick command input helpers bound max length and trim unicode by code point.
- Line 268: test `quick command history merge flips running false on exit snapshots` - Covers Quick command history merge flips running false on exit snapshots.
- Line 282: test `quick command history merge cannot resurrect ended run from stale running response` - Covers Quick command history merge cannot resurrect ended run from stale running response.
- Line 293: test `quick command pending input maps prompt kinds and confirm can be empty` - Covers Quick command pending input maps prompt kinds and confirm can be empty.
- Line 319: test `quick command history merge prefers runId and transcript sequence` - Covers Quick command history merge prefers runId and transcript sequence.
- Line 329: test `quick command backend emits merged transcript snapshots with ordered stream chunks` - Covers Quick command backend emits merged transcript snapshots with ordered stream chunks.
- Line 339: test `quick command backend uses suspended spawn plus per-run job object stop authority` - Covers Quick command backend uses suspended spawn plus per-run job object stop authority.
- Line 351: test `quick command backend assigns one sequence per terminal semantic entry and reuses it in payload plus persisted transcript` - Covers Quick command backend assigns one sequence per terminal semantic entry and reuses in payload plus persisted transcript.
- Line 358: test `quick command backend exit snapshot preserves redaction contract` - Covers Quick command backend exit snapshot preserves redaction contract.
- Line 364: test `quick command backend history and transcript payload order stay stable under bounded retention` - Covers Quick command backend history and transcript payload order stay stable under bounded retention.
- Line 372: test `quick command backend decodes terminal bytes and strips ansi controls before transcript storage` - Covers Quick command backend decodes terminal bytes and strips ansi controls before transcript storage.
- Line 379: test `Quick Command stop does not shell out to taskkill tree kill by default` - Covers Quick Command stop does not shell out to taskkill tree kill by default.
- Line 386: test `Quick Command stop uses async blocking boundary` - Covers Quick Command stop uses async blocking boundary.
- Line 392: test `Quick Command live update exposes stopping state or pending stopped transition` - Covers Quick Command live update exposes stopping state or pending stopped transition.
- Line 400: test `quick command duplicate labels stay unique case-insensitively` - Covers Quick command duplicate labels stay unique case-insensitively.

### tests/quickIcons.test.mjs

Audits quick icons behavior through 11 discovered test entries.

- Line 46: test `normalizes Windows-like taskbar pin target keys for diagnostics` - Covers Normalizes Windows-like taskbar pin target keys for diagnostics.
- Line 57: test `preserves Explorer launchers without app-managed quick icon dedupe` - Covers Preserves Explorer launchers without app-managed quick icon dedupe.
- Line 64: test `reconciles local Explorer launcher order across native refreshes` - Covers Reconciles local Explorer launcher order across native refreshes.
- Line 74: test `orders Explorer launchers by local drag order without mutating launcher data` - Covers Orders Explorer launchers by local drag order without mutating launcher data.
- Line 82: test `uses task-window-style launcher drag threshold and displacement ordering` - Covers Uses task-window-style launcher drag threshold and displacement ordering.
- Line 104: test `launcher pointer release suppresses click only after a real drag` - Covers Launcher pointer release suppresses click only after a real drag.
- Line 113: test `BottomBar keeps launcher order state but no longer renders inline launcher strip` - Covers BottomBar keeps launcher order state but no longer renders inline launcher strip.
- Line 122: test `bottom bar quick launch button opens alphabetical upward list` - Covers Bottom bar quick launch button opens alphabetical upward list.
- Line 135: test `quick launch protocol source contracts keep camelCase payloads and scoped events` - Covers Quick launch protocol source contracts keep camelCase payloads and scoped events.
- Line 163: test `bottom bar renders only Explorer taskbar pins before open windows` - Covers Bottom bar renders only Explorer taskbar pins before open windows.
- Line 171: test `frontend no longer exposes app-managed quick icon IPC or settings path` - Covers Frontend no longer exposes app-managed quick icon IPC or settings path.

### tests/quickIconsPhase3.test.mjs

Audits quick icons phase3 behavior through 3 discovered test entries.

- Line 12: test `frontend retires phase 3 app-managed quick-icon IPC and settings contracts` - Covers Frontend retires phase 3 app-managed quick-icon IPC and settings contracts.
- Line 25: test `frontend taskbar context menu wrapper exposes Explorer launcher and task-window menus only` - Covers Frontend taskbar context menu wrapper exposes Explorer launcher and task-window menus only.
- Line 37: test `bottom bar renders Explorer taskbar pins with no separate quick-icon strip` - Covers Bottom bar renders Explorer taskbar pins with no separate quick-icon strip.

### tests/quickLaunchReliability.test.mjs

Audits quick launch reliability behavior through 11 discovered test entries.

- Line 11: test `Explorer pin launch failures never remove or hide launcher buttons` - Covers Explorer pin launch failures never remove or hide launcher buttons.
- Line 23: test `app-managed quick icon frontend path is retired` - Covers App-managed quick icon frontend path is retired.
- Line 29: test `quick launch button closes on pointerdown and suppresses the racing reopen click` - Covers Quick launch button closes on pointerdown and suppresses the racing reopen click.
- Line 45: test `quick launch close handler leaves suppression for trailing click consumption only` - Covers Quick launch close handler leaves suppression for trailing click consumption only.
- Line 57: test `quick launch panel exposes only admin right-click action` - Covers Quick launch panel exposes only admin right-click action.
- Line 65: test `quick launch selected row keeps visible focus indicator distinct from hover selection` - Covers Quick launch selected row keeps visible focus indicator distinct from hover selection.
- Line 86: test `quick launch opens admin context menu from keyboard menu keys` - Covers Quick launch opens admin context menu from keyboard menu keys.
- Line 99: test `quick launch panel ignores stale closed nonce and clears state on valid close` - Covers Quick launch panel ignores stale closed nonce and clears state on valid close.
- Line 108: test `quick launch backend owns blur hold and release around native menu` - Covers Quick launch backend owns blur hold and release around native menu.
- Line 119: test `access-denied launch retries original shortcut with runas and AppX target fallback stays elevated` - Covers Access-denied launch retries original shortcut with runas and AppX target fallback stays elevated.
- Line 128: test `quick launch command surface is registered in native IPC and contracts` - Covers Quick launch command surface is registered in native IPC and contracts.

### tests/runtimeSmokeHarnessContract.test.mjs

Audits runtime smoke harness contract behavior through 5 discovered test entries.

- Line 32: test `runtime smoke script defaults to dry run and records evidence path` - Covers Runtime smoke script defaults to dry run and records evidence path.
- Line 52: test `runtime smoke script requires explicit consent for desktop mutation and process termination` - Covers Runtime smoke script requires explicit consent for desktop mutation and process termination.
- Line 76: test `runtime smoke docs forbid automated assistive technology claims without manual evidence` - Covers Runtime smoke docs forbid automated assistive technology claims without manual evidence.
- Line 81: test `package scripts expose non-destructive runtime smoke entrypoint` - Covers Package scripts expose non-destructive runtime smoke entrypoint.
- Line 91: test `official node tests clean dist-tests before execution` - Covers Official node tests clean dist-tests before execution.

### tests/searchAppCacheRefresh.test.mjs

Audits search app cache refresh behavior through 3 discovered test entries.

- Line 18: test `app cache miss/indexing retries the latest trimmed query without needing a trailing space` - Covers App cache miss/indexing retries the latest trimmed query without needing a trailing space.
- Line 39: test `app cache retry stops when app cache is hit or another provider is warming` - Covers App cache retry stops when app cache is hit or another provider is warming.
- Line 58: test `top bar schedules app-cache warm retries from progress and complete provider timings` - Covers Top bar schedules app-cache warm retries from progress and complete provider timings.

### tests/searchClearButtons.test.mjs

Audits search clear buttons behavior through 5 discovered test entries.

- Line 27: test `top-bar search renders icon-only button that opens centered search` - Covers Top-bar search renders icon-only button that opens centered search.
- Line 44: test `centered search panel renders independent clear button for non-empty displayed query` - Covers Centered search panel renders independent clear button for non-empty displayed query.
- Line 54: test `clearing advances latest search sequence so stale provider response is rejected` - Covers Clearing advances latest search sequence so stale provider response is rejected.
- Line 65: test `centered panel clear routes through query reset path and does not call provider directly` - Covers Centered panel clear routes through query reset path and does not call provider directly.
- Line 73: test `centered clear emits empty query with newer input sequence and keeps focus` - Covers Centered clear emits empty query with newer input sequence and keeps focus.

### tests/searchCloseReset.test.mjs

Audits search close reset behavior through 4 discovered test entries.

- Line 38: test `shared active-search reset clears local query, results, selection, and stale-response gates` - Covers Shared active-search reset clears local query, results, selection, and stale-response gates.
- Line 53: test `explicit close uses shared reset and publishes blank payload for centered and top-bar sync` - Covers Explicit close uses shared reset and publishes blank payload for centered and top-bar sync.
- Line 64: test `native search-panel closed event uses the same reset path` - Covers Native search-panel closed event uses the same reset path.
- Line 72: test `successful result activation routes through close reset instead of manual partial clears` - Covers Successful result activation routes through close reset instead of manual partial clears.

### tests/searchContracts.test.mjs

Audits search contracts behavior through 6 discovered test entries.

- Line 19: test `search result kind union expands without breaking current result callers` - Covers Search result kind union expands without breaking current result callers.
- Line 44: test `provider health contract reports safe Everything setup states` - Covers Provider health contract reports safe Everything setup states.
- Line 93: test `install consent and result contracts enforce no silent launch or unsafe artifact path` - Covers Install consent and result contracts enforce no silent launch or unsafe artifact path.
- Line 138: test `activation contract covers Flow-like result actions` - Covers Activation contract covers Flow-like result actions.
- Line 177: test `centered surface contract exists without changing current search-panel label` - Covers Centered surface contract exists without changing current search-panel label.
- Line 205: test `IPC command constants include registered provider health and setup names` - Covers IPC command constants include registered provider health and setup names.

### tests/searchEngineContracts.test.mjs

Audits search engine contracts behavior through 12 discovered test entries.

- Line 57: test `search engine request and response contracts match phase 1 shape` - Covers Search engine request and response contracts match phase 1 shape.
- Line 93: test `app search index refresh payload is typed so legacy refresh events are ignored` - Covers App search index refresh payload is typed so legacy refresh events are ignored.
- Line 119: test `search result validator requires explicit safe action contracts` - Covers Search result validator requires explicit safe action contracts.
- Line 139: test `settings action safety allows ms-settings pages and control.exe only` - Covers Settings action safety allows ms-settings pages and control.exe only.
- Line 181: test `control panel panel result keeps executable path separate from safe args` - Covers Control panel panel result keeps executable path separate from safe args.
- Line 201: test `progress payload supports immediate local rows and stale provider batches` - Covers Progress payload supports immediate local rows and stale provider batches.
- Line 245: test `progress payload converts to panel payload and merges by stable record key` - Covers Progress payload converts to panel payload and merges by stable record key.
- Line 306: test `provider timing accepts persistent app-cache indexing state and cache age` - Covers Provider timing accepts persistent app-cache indexing state and cache age.
- Line 327: test `search engine response maps degraded Everything health into concise local-results status lines` - Covers Search engine response maps degraded Everything health into concise local-results status lines.
- Line 407: test `search result contract accepts highlight spans for fuzzy matches` - Covers Search result contract accepts highlight spans for fuzzy matches.
- Line 429: test `search result contract rejects malformed highlight span arrays` - Covers Search result contract rejects malformed highlight span arrays.
- Line 440: test `new search engine wrapper is isolated from legacy catalog and ranking hot paths` - Covers New search engine wrapper is isolated from legacy catalog and ranking hot paths.

### tests/searchOverhaulPhase0.test.mjs

Audits search overhaul phase0 behavior through 28 discovered test entries.

- Line 60: it `,` - Covers ,.
- Line 102: test `phase 0 query fixture covers requested baseline problem queries` - Covers Phase 0 query fixture covers requested baseline problem queries.
- Line 123: test `phase 0 prefix corpus is deterministic and bounded for baseline measurement` - Covers Phase 0 prefix corpus is deterministic and bounded for baseline measurement.
- Line 142: test `phase 0 performance fixture documents forbidden input-handler dependencies` - Covers Phase 0 performance fixture documents forbidden input-handler dependencies.
- Line 155: test `phase 0 performance script exists and writes ignored artifacts` - Covers Phase 0 performance script exists and writes ignored artifacts.
- Line 176: test `phase 0 performance script derives no-scan guard from harness and source` - Covers Phase 0 performance script derives no-scan guard from harness and source.
- Line 212: test `phase 0 harness validator rejects malformed scan and boundary samples` - Covers Phase 0 harness validator rejects malformed scan and boundary samples.
- Line 240: test `phase 0 harness exists and preserves local progress on unavailable provider states` - Covers Phase 0 harness exists and preserves local progress on unavailable provider states.
- Line 267: test `phase 0 legacy-remnant checklist covers old search interference files` - Covers Phase 0 legacy-remnant checklist covers old search interference files.
- Line 279: test `phase 7 audit expectations are explicit and validated` - Covers Phase 7 audit expectations are explicit and validated.
- Line 281: test `phase 7 audit expectations are explicit and validated` - Covers Phase 7 audit expectations are explicit and validated.
- Line 283: it `\n` - Covers \n.
- Line 308: test `phase 8 QA and performance expectations are testable from the overhaul plan` - Covers Phase 8 QA and performance expectations are testable from the overhaul plan.
- Line 310: test `phase 8 QA and performance expectations are testable from the overhaul plan` - Covers Phase 8 QA and performance expectations are testable from the overhaul plan.
- Line 327: test `phase 1 contracts exist in new search modules` - Covers Phase 1 contracts exist in new search modules.
- Line 349: test `phase 2 settings provider dataset backs required Windows settings intents` - Covers Phase 2 settings provider dataset backs required Windows settings intents.
- Line 374: test `top-bar input handler keeps forbidden work out of the direct input path` - Covers Top-bar input handler keeps forbidden work out of the direct input path.
- Line 388: test `rapid typing publishes pending or current-best payload before provider resolution and gates stale responses` - Covers Rapid typing publishes pending or current-best payload before provider resolution and gates stale responses.
- Line 405: test `phase 0 script parses Rust JSON and keeps scenario p50/p95 on raw samples` - Covers Phase 0 script parses Rust JSON and keeps scenario p50/p95 on raw samples.
- Line 417: test `phase 6 removes TypeScript hot path legacy imports recorded by phase 0` - Covers Phase 6 removes TypeScript hot path legacy imports recorded by phase 0.
- Line 430: test `top-bar import block has no legacy catalog ranking or legacy system wrapper import` - Covers Top-bar import block has no legacy catalog ranking or legacy system wrapper import.
- Line 443: test `phase 1 new frontend wrapper exposes a registered command name` - Covers Phase 1 new frontend wrapper exposes a registered command name.
- Line 452: test `future Rust search hot path does not route through legacy search_sources coordinator or fallbacks` - Covers Future Rust search hot path does not route through legacy search sources coordinator or fallbacks.
- Line 465: test `new Rust search subtree does not import old search_sources index provider windows_search or files modules` - Covers New Rust search subtree does not import old search sources index provider windows search or files modules.
- Line 486: test `legacy search_system command is no longer registered in production command maps` - Covers Legacy search system command is no longer registered in production command maps.
- Line 499: test `phase 3 everything provider has cached health, bounded simple-name request, and timings` - Covers Phase 3 everything provider has cached health, bounded simple-name request, and timings.
- Line 501: test `phase 3 everything provider has cached health, bounded simple-name request, and timings` - Covers Phase 3 everything provider has cached health, bounded simple-name request, and timings.
- Line 513: test `phase 4 app and local providers use cached bounded indexes instead of per-query start menu scans` - Covers Phase 4 app and local providers use cached bounded indexes instead of per-query start menu scans.

### tests/searchOverhaulPhase6.test.mjs

Audits search overhaul phase6 behavior through 6 discovered test entries.

- Line 46: test `phase 6 fixture defines input-pending-latest-stale pipeline and required seams` - Covers Phase 6 fixture defines input-pending-latest-stale pipeline and required seams.
- Line 60: test `phase 6 input handler updates query before any provider ranking storage filesystem or native show work` - Covers Phase 6 input handler updates query before any provider ranking storage filesystem or native show work.
- Line 82: test `phase 6 top-bar publishes pending payload before deferred engine request and applies latest response only` - Covers Phase 6 top-bar publishes pending payload before deferred engine request and applies latest response only.
- Line 103: test `phase 6 activation preserves window focus and control panel action contracts` - Covers Phase 6 activation preserves window focus and control panel action contracts.
- Line 112: test `phase 6 top-bar hot path imports new search engine and no legacy catalog ranking or system-search wrappers` - Covers Phase 6 top-bar hot path imports new search engine and no legacy catalog ranking or system-search wrappers.
- Line 131: test `phase 6 hot path does not use legacy source files or browser storage for visible result production` - Covers Phase 6 hot path does not use legacy source files or browser storage for visible result production.

### tests/searchPanelState.test.mjs

Audits search panel state behavior through 27 discovered test entries.

- Line 20: test `search panel empty state stays hidden when status text already explains startup or loading` - Covers Search panel empty state stays hidden when status text already explains startup or loading.
- Line 29: test `applies a typed search payload with visible results and selection` - Covers Applies a typed search payload with visible results and selection.
- Line 53: test `applies filesystem search results with launch paths` - Covers Applies filesystem search results with launch paths.
- Line 76: test `includes backend app and file results in the visible catalog` - Covers Includes backend app and file results in the visible catalog.
- Line 102: test `catalog exposes system settings intents before incidental filesystem matches` - Covers Catalog exposes system settings intents before incidental filesystem matches.
- Line 122: test `ignores stale system search responses while keeping latest query live` - Covers Ignores stale system search responses while keeping latest query live.
- Line 127: test `retries empty indexed search while the cache is warming` - Covers Retries empty indexed search while the cache is warming.
- Line 133: test `reveals only valid selected search rows` - Covers Reveals only valid selected search rows.
- Line 139: test `refreshes current system search after an index event` - Covers Refreshes current system search after an index event.
- Line 146: test `keeps search panel show and publish idempotent for realtime typing` - Covers Keeps search panel show and publish idempotent for realtime typing.
- Line 164: test `search panel payload signature includes presentation and ranking metadata` - Covers Search panel payload signature includes presentation and ranking metadata.
- Line 193: test `search panel state rejects stale query and phase regressions for the same sequence` - Covers Search panel state rejects stale query and phase regressions for the same sequence.
- Line 233: test `search panel state keeps useful rows for typing and empty error payloads` - Covers Search panel state keeps useful rows for typing and empty error payloads.
- Line 273: test `search panel state clears old typing rows for a changed query and normalizes trailing-space identity` - Covers Search panel state clears old typing rows for a changed query and normalizes trailing-space identity.
- Line 333: test `search panel state applies rapid Windows settings typing payloads without stale rows` - Covers Search panel state applies rapid Windows settings typing payloads without stale rows.
- Line 367: test `search panel state allows complete payload after recoverable provider error` - Covers Search panel state allows complete payload after recoverable provider error.
- Line 407: test `search panel state source has no stray debug console literals` - Covers Search panel state source has no stray debug console literals.
- Line 415: test `top-bar defers expensive search render work out of the input handler` - Covers Top-bar defers expensive search render work out of the input handler.
- Line 457: test `top-bar cancels search work on native panel close and listens only to app index refreshes` - Covers Top-bar cancels search work on native panel close and listens only to app index refreshes.
- Line 478: test `top-bar tracks and clears delayed rail scroll button updates` - Covers Top-bar tracks and clears delayed rail scroll button updates.
- Line 496: test `search-panel fallback fetches cannot overwrite newer event payloads` - Covers Search-panel fallback fetches cannot overwrite newer event payloads.
- Line 504: test `centered search surface keeps local typing immediate without unconditional payload refocus` - Covers Centered search surface keeps local typing immediate without unconditional payload refocus.
- Line 520: test `top-bar search uses icon button and centered open flow` - Covers Top-bar search uses icon button and centered open flow.
- Line 530: test `centered search query events cannot fall back to legacy anchored mode` - Covers Centered search query events cannot fall back to legacy anchored mode.
- Line 536: test `search panel renders a flat visibleRows model instead of grouped buckets` - Covers Search panel renders a flat visibleRows model instead of grouped buckets.
- Line 545: test `search panel keyboard and aria state follow visibleRows order` - Covers Search panel keyboard and aria state follow visibleRows order.
- Line 567: test `search panel minimum layout keeps header outside internal result scroller` - Covers Search panel minimum layout keeps header outside internal result scroller.

### tests/searchRanking.test.mjs

Audits search ranking behavior through 13 discovered test entries.

- Line 10: test `selected-count boost moves frequent result up without hiding exact match` - Covers Selected-count boost moves frequent result up without hiding exact match.
- Line 35: test `Everything-only result sets are ranked instead of left in provider order` - Covers Everything-only result sets are ranked instead of left in provider order.
- Line 63: test `top-most override wins deterministically for equal query matches` - Covers Top-most override wins deterministically for equal query matches.
- Line 89: test `provider and result type priority prefer Everything file results over Windows fallback duplicates` - Covers Provider and result type priority prefer Everything file results over Windows fallback duplicates.
- Line 120: test `score math is capped and deterministic` - Covers Score math is capped and deterministic.
- Line 135: test `exact app matches outrank Everything folders for launcher-style queries` - Covers Exact app matches outrank Everything folders for launcher-style queries.
- Line 162: test `exact app intents outrank high priority Everything folders` - Covers Exact app intents outrank high priority Everything folders.
- Line 189: test `fuzzy app token matches Spotify-style launcher intent` - Covers Fuzzy app token matches Spotify-style launcher intent.
- Line 216: test `Windows Settings and Control Panel intents outrank incidental folders` - Covers Windows Settings and Control Panel intents outrank incidental folders.
- Line 266: test `bare settings query outranks exact incidental Everything folder` - Covers Bare settings query outranks exact incidental Everything folder.
- Line 294: test `nonmatching results do not pass through on provider or type boosts alone` - Covers Nonmatching results do not pass through on provider or type boosts alone.
- Line 310: test `control panel query can match control-plane command alias before files` - Covers Control panel query can match control-plane command alias before files.
- Line 337: test `exact filename matches outrank weak substring matches` - Covers Exact filename matches outrank weak substring matches.

### tests/searchSettings.test.mjs

Audits search settings behavior through 7 discovered test entries.

- Line 15: test `search settings defaults match JSON-owned Everything safe defaults` - Covers Search settings defaults match JSON-owned Everything safe defaults.
- Line 36: test `shell settings include search settings as behavior source of truth` - Covers Shell settings include search settings as behavior source of truth.
- Line 43: test `v1 settings without search fields coerce to search defaults` - Covers V1 settings without search fields coerce to search defaults.
- Line 63: test `invalid search setting enums and bounds normalize to documented safe defaults` - Covers Invalid search setting enums and bounds normalize to documented safe defaults.
- Line 90: test `search settings bounds match Rust normalization limits` - Covers Search settings bounds match Rust normalization limits.
- Line 104: test `search settings reject secret-like keys before persistence` - Covers Search settings reject secret-like keys before persistence.
- Line 119: test `everything setup consent blocks unsafe download, bundle, or execution paths` - Covers Everything setup consent blocks unsafe download, bundle, or execution paths.

### tests/searchTypingFreezePhase1.test.mjs

Audits search typing freeze phase1 behavior through 13 discovered test entries.

- Line 29: test `phase 1 top-bar input handler publishes draft and starts search for every input event` - Covers Phase 1 top-bar input handler publishes draft and starts search for every input event.
- Line 55: test `phase 1 search query execution is not debounced or coalesced after input draft advances` - Covers Phase 1 search query execution is not debounced or coalesced after input draft advances.
- Line 68: test `draft query change cancels old search work before deferred apply window` - Covers Draft query change cancels old search work before deferred apply window.
- Line 86: test `rapid Windows settings typing publishes pending visible state for every key before provider flush` - Covers Rapid Windows settings typing publishes pending visible state for every key before provider flush.
- Line 121: test `top-bar input events enqueue exact current value instead of prior draft state` - Covers Top-bar input events enqueue exact current value instead of prior draft state.
- Line 131: test `centered search query events reject stale source-order payloads and use exact query` - Covers Centered search query events reject stale source-order payloads and use exact query.
- Line 146: test `centered search out-of-order Wi event cannot overwrite later Windows settings query` - Covers Centered search out-of-order Wi event cannot overwrite later Windows settings query.
- Line 165: test `app-index refresh uses current draft query during deferred apply window` - Covers App-index refresh uses current draft query during deferred apply window.
- Line 177: test `rapid Brave typing creates exact current requests for every prefix` - Covers Rapid Brave typing creates exact current requests for every prefix.
- Line 189: test `phase 1 close cancels queued and scheduled search work and invalidates stale responses` - Covers Phase 1 close cancels queued and scheduled search work and invalidates stale responses.
- Line 201: test `phase 1 rapid Firefox input keeps final draft current and only latest provider response applies` - Covers Phase 1 rapid Firefox input keeps final draft current and only latest provider response applies.
- Line 221: test `phase 1 Rust search command runs coordinator behind async blocking boundary` - Covers Phase 1 Rust search command runs coordinator behind async blocking boundary.
- Line 235: test `spacebar freshness workaround is replaced by automatic same-query retry` - Covers Spacebar freshness workaround is replaced by automatic same-query retry.

### tests/searchUxState.test.mjs

Audits search ux state behavior through 27 discovered test entries.

- Line 40: test `builds flat visible rows with Best match first and grouped remaining rows after it` - Covers Builds flat visible rows with Best match first and grouped remaining rows after.
- Line 63: test `keeps backend canonical order across every visible row without grouped tail` - Covers Keeps backend canonical order across every visible row without grouped tail.
- Line 75: test `best match removes top backend rows from later groups while preserving group order inside remainder` - Covers Best match removes top backend rows from later groups while preserving group order inside remainder.
- Line 98: test `caps only leading consecutive apps at four and preserves tail order with interleaved apps` - Covers Caps only leading consecutive apps at four and preserves tail order with interleaved apps.
- Line 140: test `keeps all app rows visible when results are app-only` - Covers Keeps all app rows visible when results are app-only.
- Line 153: test `labels primary and secondary actions by result kind` - Covers Labels primary and secondary actions by result kind.
- Line 171: test `category rows default to seven visible items and advertise remaining rows` - Covers Category rows default to seven visible items and advertise remaining rows.
- Line 189: test `expanded groups reveal hidden rows and clear overflow state for that category` - Covers Expanded groups reveal hidden rows and clear overflow state for that category.
- Line 207: test `groups new Flow-like result kinds into settings and commands visible sections` - Covers Groups new Flow-like result kinds into settings and commands visible sections.
- Line 222: test `visible row helpers map backend selection into visual order` - Covers Visible row helpers map backend selection into visual order.
- Line 240: test `phase 1 query-gate ordering keeps first visible row equal to backend rank 1 across representative intents` - Covers Phase 1 query-gate ordering keeps first visible row equal to backend rank 1 across representative intents.
- Line 308: test `visible rows keep duplicate backend ids uniquely keyable while preserving raw activation ids` - Covers Visible rows keep duplicate backend ids uniquely keyable while preserving raw activation ids.
- Line 329: test `visible row identity resolves second duplicate backend id to the clicked visible row` - Covers Visible row identity resolves second duplicate backend id to the clicked visible row.
- Line 348: test `progressive new-query local rows replace stale prior-query working set before complete` - Covers Progressive new-query local rows replace stale prior-query working set before complete.
- Line 372: test `progressive same-normalized-query snapshots replace stale best match without needing trailing-space rerun` - Covers Progressive same-normalized-query snapshots replace stale best match without needing trailing-space rerun.
- Line 425: test `visible rows preserve fuzzy highlight span data for panel rendering` - Covers Visible rows preserve fuzzy highlight span data for panel rendering.
- Line 442: test `top-bar keyboard traversal can map visible-row movement back to backend result indices` - Covers Top-bar keyboard traversal can map visible-row movement back to backend result indices.
- Line 474: test `ranking accepts injected usage so frequent results can outrank equal matches` - Covers Ranking accepts injected usage so frequent results can outrank equal matches.
- Line 483: test `ranking caches usage map instead of reading storage for every rank` - Covers Ranking caches usage map instead of reading storage for every rank.
- Line 513: test `search panel fallback polling is bounded with backoff` - Covers Search panel fallback polling is bounded with backoff.
- Line 523: test `search input state can advance while expensive result refresh is deferred latest-only` - Covers Search input state can advance while expensive result refresh is deferred latest-only.
- Line 541: test `latest-only search execution queue keeps rapid Firefox draft immediate and runs final provider request only` - Covers Latest-only search execution queue keeps rapid Firefox draft immediate and runs final provider request only.
- Line 561: test `search engine controller ignores out-of-order stale provider responses` - Covers Search engine controller ignores out-of-order stale provider responses.
- Line 572: test `provider cache warm retry allows stale refresh payloads with nonzero app rows` - Covers Provider cache warm retry allows stale refresh payloads with nonzero app rows.
- Line 583: test `provider cache warm retry ignores non-app cache timings and cache hits` - Covers Provider cache warm retry ignores non-app cache timings and cache hits.
- Line 600: test `search mode defaults to centered and preserves explicit top-right routing` - Covers Search mode defaults to centered and preserves explicit top-right routing.
- Line 608: test `search keyboard actions are shared by top-right and centered modes` - Covers Search keyboard actions are shared by top-right and centered modes.

### tests/settingsPanelWiring.test.mjs

Audits settings panel wiring behavior through 6 discovered test entries.

- Line 19: test `settings panel is routed as an anchored auxiliary shell surface` - Covers Settings panel is routed as an anchored auxiliary shell surface.
- Line 31: test `top-left JasonShell button opens settings instead of search` - Covers Top-left JasonShell button opens settings instead of search.
- Line 41: test `settings panel exposes live theme, font, date, clock, and useful UI preferences` - Covers Settings panel exposes live theme, font, date, clock, and useful UI preferences.
- Line 62: test `settings panel keeps Stack Browser terminal profile inside JSON shell settings section` - Covers Settings panel keeps Stack Browser terminal profile inside JSON shell settings section.
- Line 100: test `settings panel scrolls vertically so lower controls remain reachable` - Covers Settings panel scrolls vertically so lower controls remain reachable.
- Line 107: test `settings panel Rust placement clamps to the top-bar host bounds` - Covers Settings panel Rust placement clamps to the top-bar host bounds.

### tests/settingsPowerActionsPhase5.test.mjs

Audits settings power actions phase5 behavior through 3 discovered test entries.

- Line 12: test `settings panel exposes three power actions behind in-panel confirmation` - Covers Settings panel exposes three power actions behind in-panel confirmation.
- Line 28: test `settings power action wrapper accepts enum-only request shape` - Covers Settings power action wrapper accepts enum-only request shape.
- Line 36: test `Rust power command validates enum and builds non-shell execution plans` - Covers Rust power command validates enum and builds non-shell execution plans.

### tests/shellBarResize.test.mjs

Audits shell bar resize behavior through 8 discovered test entries.

- Line 17: test `JSON shell settings default both shell bar height locks on` - Covers JSON shell settings default both shell bar height locks on.
- Line 29: test `shell bar drag math grows top downward and bottom upward within bounds` - Covers Shell bar drag math grows top downward and bottom upward within bounds.
- Line 39: test `settings sync preserves optimistic resize heights until drag persistence settles` - Covers Settings sync preserves optimistic resize heights until drag persistence settles.
- Line 47: test `resize drags optimistically update local height and coalesce native IPC` - Covers Resize drags optimistically update local height and coalesce native IPC.
- Line 62: test `height persistence and locks use merge-safe backend commands instead of whole settings saves` - Covers Height persistence and locks use merge-safe backend commands instead of whole settings saves.
- Line 74: test `settings panel exposes lock toggles for both shell bars` - Covers Settings panel exposes lock toggles for both shell bars.
- Line 83: test `top and bottom bars gate visible resize handles behind unlocked settings` - Covers Top and bottom bars gate visible resize handles behind unlocked settings.
- Line 99: test `backend resize command re-reserves appbars and updates work area` - Covers Backend resize command re-reserves appbars and updates work area.

### tests/shellOpenCloseEvents.test.mjs

Audits shell open close events behavior through 5 discovered test entries.

- Line 25: test `search close helpers target top-bar for native and explicit close reset` - Covers Search close helpers target top-bar for native and explicit close reset.
- Line 37: test `audio close event is delivered to both top-bar and audio-panel owners` - Covers Audio close event is delivered to both top-bar and audio-panel owners.
- Line 50: test `audio panel surface listens for own close event and stops polling` - Covers Audio panel surface listens for own close event and stops polling.
- Line 64: test `stack popup remains visible when focus moves to top-bar pinned folders` - Covers Stack popup remains visible when focus moves to top-bar pinned folders.
- Line 78: test `tray open event targets tray-panel and reloads icons on every show` - Covers Tray open event targets tray-panel and reloads icons on every show.

### tests/shellPopupLayoutScrollPhase1.test.mjs

Audits shell popup layout scroll phase1 behavior through 4 discovered test entries.

- Line 27: test `stack context menu and submenu clamp to viewport with internal vertical scroll` - Covers Stack context menu and submenu clamp to viewport with internal vertical scroll.
- Line 57: test `tray panel keeps root fixed and gives icon content an internal scroller` - Covers Tray panel keeps root fixed and gives icon content an internal scroller.
- Line 80: test `process manager keeps header and body in one horizontal scroll context` - Covers Process manager keeps header and body in one horizontal scroll context.
- Line 131: test `search panel uses flex column with internal result overflow and no approximate result max-height` - Covers Search panel uses flex column with internal result overflow and no approximate result max-height.

### tests/shellPreferences.test.mjs

Audits shell preferences behavior through 6 discovered test entries.

- Line 28: test `Open Sans is the default shell font and bundled font options are backed by local downloaded assets` - Covers Open Sans is the default shell font and bundled font options are backed by local downloaded assets.
- Line 54: test `normalizes shell preferences and applies dataset plus CSS font variables` - Covers Normalizes shell preferences and applies dataset plus CSS font variables.
- Line 97: test `parses and installs strict https fonts.google.com links as custom CSS2 font preferences` - Covers Parses and installs strict https fonts.google.com links as custom CSS2 font preferences.
- Line 142: test `injects one safe stylesheet link per custom Google Fonts CSS URL` - Covers Injects one safe stylesheet link per custom Google Fonts CSS URL.
- Line 189: test `formats custom date strings and clock options for the top bar` - Covers Formats custom date strings and clock options for the top bar.
- Line 196: test `preferences sync handles storage, BroadcastChannel updates, cleanup, and local listeners` - Covers Preferences sync handles storage, BroadcastChannel updates, cleanup, and local listeners.

### tests/sourceContractIntentRegistry.test.mjs

Audits source contract intent registry behavior through 3 discovered test entries.

- Line 134: test `source-text tests have explicit intent tags in the registry` - Covers Source-text tests have explicit intent tags in the registry.
- Line 140: test `source-contract registry keeps boundary tests separate from behavior legacy guards` - Covers Source-contract registry keeps boundary tests separate from behavior legacy guards.
- Line 148: test `source-contract registry entries point at existing tests` - Covers Source-contract registry entries point at existing tests.

### tests/stackBrowserCloseButton.test.mjs

Audits stack browser close button behavior through 1 discovered test entries.

- Line 29: test `stack browser has an accessible rectangular red X close button wired to the surface close path` - Covers Stack browser has an accessible rectangular red X close button wired to the surface close path.

### tests/stackBrowserContextActions.test.mjs

Audits stack browser context actions behavior through 8 discovered test entries.

- Line 23: test `plans editor, terminal, and copy actions without executing them` - Covers Plans editor, terminal, and copy actions without executing them.
- Line 43: test `plans template creation paths under the current folder target` - Covers Plans template creation paths under the current folder target.
- Line 55: test `plans git-aware operations and marks restore as confirmation-gated` - Covers Plans git-aware operations and marks restore as confirmation-gated.
- Line 80: test `omits git operations outside the repository root` - Covers Omits git operations outside the repository root.
- Line 93: test `plans archive extraction only for supported file rows` - Covers Plans archive extraction only for supported file rows.
- Line 138: test `plans Properties for file, folder, and current folder near the bottom` - Covers Plans Properties for file, folder, and current folder near the bottom.
- Line 161: test `omits Properties for virtual or missing filesystem rows` - Covers Omits Properties for virtual or missing filesystem rows.
- Line 175: test `treats zip file rows as stack-browsable archives for double click navigation` - Covers Treats zip file rows as stack-browsable archives for double click navigation.

### tests/stackBrowserGitStatus.test.mjs

Audits stack browser git status behavior through 2 discovered test entries.

- Line 13: test `stack git status backend and API use git porcelain outside folder listing` - Covers Stack git status backend and API use git porcelain outside folder listing.
- Line 22: test `stack browser renders minimal git summary and row badges` - Covers Stack browser renders minimal git summary and row badges.

### tests/stackBrowserMarginSelection.test.mjs

Audits stack browser margin selection behavior through 3 discovered test entries.

- Line 18: test `classifies full details body and margin surfaces as marquee start zones` - Covers Classifies full details body and margin surfaces as marquee start zones.
- Line 24: test `blocks interactive row and chrome targets from marquee start` - Covers Blocks interactive row and chrome targets from marquee start.
- Line 38: test `details pane owns pointerdown for full margin selection, rows and resize keep ownership` - Covers Details pane owns pointerdown for full margin selection, rows and resize keep ownership.

### tests/stackBrowserPathAutocomplete.test.mjs

Audits stack browser path autocomplete behavior through 10 discovered test entries.

- Line 17: test `parses Windows path autocomplete parent and segment at caret` - Covers Parses Windows path autocomplete parent and segment at caret.
- Line 32: test `parses UNC paths and rejects relative or empty autocomplete inputs` - Covers Parses UNC paths and rejects relative or empty autocomplete inputs.
- Line 41: test `path autocomplete has IPC wrapper and stale-aware UI wiring` - Covers Path autocomplete has IPC wrapper and stale-aware UI wiring.
- Line 66: test `RightArrow accepting inline path completion keeps focus and caret for chained completion` - Covers RightArrow accepting inline path completion keeps focus and caret for chained completion.
- Line 78: test `RightArrow accept immediately refreshes suggestions for committed path` - Covers RightArrow accept immediately refreshes suggestions for committed path.
- Line 87: test `Tab cycles through matching directory completions without opening the folder` - Covers Tab cycles through matching directory completions without opening the folder.
- Line 107: test `Tab cycle skips an exact prefix directory before walking sibling completions` - Covers Tab cycle skips an exact prefix directory before walking sibling completions.
- Line 122: test `path blur timeout cannot reset draft during chained RightArrow accept flow` - Covers Path blur timeout cannot reset draft during chained RightArrow accept flow.
- Line 134: test `builds inline path completion suffix for matching suggestion` - Covers Builds inline path completion suffix for matching suggestion.
- Line 145: test `omits inline completion when suggestion does not extend typed segment` - Covers Omits inline completion when suggestion does not extend typed segment.

### tests/stackBrowserPhase1Safety.test.mjs

Audits stack browser phase1safety behavior through 8 discovered test entries.

- Line 68: test `WP0 command inventory exposes Stack Browser Phase 1 parity blocker before auth work` - Covers WP0 command inventory exposes Stack Browser Phase 1 parity blocker before auth work.
- Line 75: test `WP0 auth matrix source contract classifies every current Stack Browser Phase 1 command` - Covers WP0 auth matrix source contract classifies every current Stack Browser Phase 1 command.
- Line 91: test `WP0 scoped Phase 1 handlers authorize before side effects with window injection` - Covers WP0 scoped Phase 1 handlers authorize before side effects with window injection.
- Line 112: test `Stack Browser toggle authorizes top-bar before changing native visibility` - Covers Stack Browser toggle authorizes top-bar before changing native visibility.
- Line 122: test `WP0 phase 1 file mutation auth excludes terminal-panel for copy-cut-paste-delete-create` - Covers WP0 phase 1 file mutation auth excludes terminal-panel for copy-cut-paste-delete-create.
- Line 124: it `\n` - Covers \n.
- Line 130: test `WP0 auth helper fails closed for unknown commands with stable IPC error` - Covers WP0 auth helper fails closed for unknown commands with stable IPC error.
- Line 137: test `WP0 disallows cross-surface terminal target spoofing by contract` - Covers WP0 disallows cross-surface terminal target spoofing by contract.

### tests/stackBrowserTerminal.test.mjs

Audits stack browser terminal behavior through 28 discovered test entries.

- Line 31: test `stack terminal output chunks require sequenced dedupe identity` - Covers Stack terminal output chunks require sequenced dedupe identity.
- Line 37: test `stack terminal IPC command names are stable and backend-registered` - Covers Stack terminal IPC command names are stable and backend-registered.
- Line 60: test `terminal reader never blocks on an undrained fallback output queue` - Covers Terminal reader never blocks on an undrained fallback output queue.
- Line 67: test `phase 1 terminal startup has visible nonblank lifecycle state before first output` - Covers Phase 1 terminal startup has visible nonblank lifecycle state before first output.
- Line 86: test `terminal control-only output does not hide startup overlay as first visible output` - Covers Terminal control-only output does not hide startup overlay as first visible output.
- Line 97: test `terminal poll fallback keeps writing chunks after first output with sequence dedupe` - Covers Terminal poll fallback keeps writing chunks after first output with sequence dedupe.
- Line 107: test `phase 1 terminal output uses stack-terminal:output listener with persistent-surface cleanup` - Covers Phase 1 terminal output uses stack-terminal:output listener with persistent-surface cleanup.
- Line 125: test `phase 1 terminal resize command contract is exposed across IPC, capabilities, and fitted xterm geometry` - Covers Phase 1 terminal resize command contract is exposed across IPC, capabilities, and fitted xterm geometry.
- Line 143: test `terminal TUI input stays low-latency and transparent` - Covers Terminal TUI input stays low-latency and transparent.
- Line 161: test `phase 1 terminal poll and resize paths serialize frontend reads for one active session` - Covers Phase 1 terminal poll and resize paths serialize frontend reads for one active session.
- Line 177: test `phase 3 terminal output is push-first and batched before xterm writes` - Covers Phase 3 terminal output is push-first and batched before xterm writes.
- Line 195: test `phase 4 extracts Stack terminal pane and modern xterm addons from parent surface` - Covers Phase 4 extracts Stack terminal pane and modern xterm addons from parent surface.
- Line 217: test `stack terminal text metrics stay monospace and PTY redraw sequences stay xterm-owned` - Covers Stack terminal text metrics stay monospace and PTY redraw sequences stay xterm-owned.
- Line 240: test `phase 4 terminal link detector recognizes safe URL, path, localhost, and git hash targets` - Covers Phase 4 terminal link detector recognizes safe URL, path, localhost, and git hash targets.
- Line 251: test `phase 3 backend keeps live sessions registered and records PTY size metadata` - Covers Phase 3 backend keeps live sessions registered and records PTY size metadata.
- Line 272: test `terminal auth precedes side effects in poll and stop, with no drain/remove/kill before auth` - Covers Terminal auth precedes side effects in poll and stop, with no drain/remove/kill before auth.
- Line 285: test `phase 1 terminal PowerShell launch plan uses trusted path explicitly and has fallback coverage` - Covers Phase 1 terminal PowerShell launch plan uses trusted path explicitly and has fallback coverage.
- Line 300: test `stack popup API exposes typed terminal profiles and command wrappers` - Covers Stack popup API exposes typed terminal profiles and command wrappers.
- Line 320: test `shell settings include stack browser terminal profile defaults` - Covers Shell settings include stack browser terminal profile defaults.
- Line 329: test `stack browser no longer exposes embedded CLI toggle controls` - Covers Stack browser no longer exposes embedded CLI toggle controls.
- Line 335: test `persistent terminal starts on delayed idle or first open, accepts input, and polls output` - Covers Persistent terminal starts on delayed idle or first open, accepts input, and polls output.
- Line 388: test `stack terminal supports xterm selection and copy` - Covers Stack terminal supports xterm selection and copy.
- Line 424: test `terminal removes xterm assistive mirrors from visible layout` - Covers Terminal removes xterm assistive mirrors from visible layout.
- Line 438: test `terminal cwd changes update Stack Browser path and breadcrumbs immediately` - Covers Terminal cwd changes update Stack Browser path and breadcrumbs immediately.
- Line 448: test `stack terminal PowerShell profile loads normal shell affordances, preserves Tab completion cycling, and normalizes extended cwd paths` - Covers Stack terminal PowerShell profile loads normal shell affordances, preserves Tab completion cycling, and normalizes extended cwd paths.
- Line 497: test `stack terminal profile uses JSON shell settings from settings panel, not Stack Browser toolbar` - Covers Stack terminal profile uses JSON shell settings from settings panel, not Stack Browser toolbar.
- Line 511: test `stack terminal layout is flat, dense, and isolated from file grid interactions` - Covers Stack terminal layout is flat, dense, and isolated from file grid interactions.
- Line 528: test `durable docs mention persistent terminal panel behavior and validation` - Covers Durable docs mention persistent terminal panel behavior and validation.

### tests/stackBrowserToolbarIcons.test.mjs

Audits stack browser toolbar icons behavior through 5 discovered test entries.

- Line 32: test `shared Material Symbols registry includes every stack browser toolbar icon` - Covers Shared Material Symbols registry includes every stack browser toolbar icon.
- Line 51: test `stack browser uses official outlined Material Symbol paths` - Covers Stack browser uses official outlined Material Symbol paths.
- Line 73: test `stack browser icon buttons have no background or border` - Covers Stack browser icon buttons have no background or border.
- Line 81: test `stack browser toolbar text buttons are Material Symbol icon buttons with accessible labels` - Covers Stack browser toolbar text buttons are Material Symbol icon buttons with accessible labels.
- Line 108: test `stack browser search label uses icon-only chrome while context menus keep text labels` - Covers Stack browser search label uses icon-only chrome while context menus keep text labels.

### tests/stackBrowserTopBarPinFlow.test.mjs

Audits stack browser top bar pin flow behavior through 1 discovered test entries.

- Line 51: test `Stack Browser pin mutation updates the top-bar display immediately without startup reload` - Covers Stack Browser pin mutation updates the top-bar display immediately without startup reload.

### tests/stackGitPanelRedContract.test.mjs

Audits stack git panel red contract behavior through 37 discovered test entries.

- Line 28: test `stack git API exposes no-AI diff, unstage, and destructive revert command contracts` - Covers Stack git API exposes no-AI diff, unstage, and destructive revert command contracts.
- Line 54: test `stack git commit history exposes commit file list and per-file diff contracts` - Covers Stack git commit history exposes commit file list and per-file diff contracts.
- Line 70: test `stack git stash exposes dedicated file list and per-file diff contracts` - Covers Stack git stash exposes dedicated file list and per-file diff contracts.
- Line 91: test `stack git status tracks staged and unstaged state plus optional ahead behind counts` - Covers Stack git status tracks staged and unstaged state plus optional ahead behind counts.
- Line 99: test `extracted StackGitPanel owns OpenChamber-like no-AI sections and controls` - Covers Extracted StackGitPanel owns OpenChamber-like no-AI sections and controls.
- Line 113: test `stack git pure state groups paths and gates safe operations` - Covers Stack git pure state groups paths and gates safe operations.
- Line 142: test `stack git status exposes context-specific additions and deletions fields` - Covers Stack git status exposes context-specific additions and deletions fields.
- Line 153: test `destructive discard requires explicit confirmation and never targets untracked files` - Covers Destructive discard requires explicit confirmation and never targets untracked files.
- Line 162: test `stack git API exposes first-class history and stash command contracts without AI` - Covers Stack git API exposes first-class history and stash command contracts without AI.
- Line 199: test `stack git history files and diffs use stale-safe loading, friendly status labels, and accessible disclosure` - Covers Stack git history files and diffs use stale-safe loading, friendly status labels, and accessible disclosure.
- Line 218: test `stack git stash backend uses fixed argv, bounded message, explicit untracked flag, and spawn_blocking` - Covers Stack git stash backend uses fixed argv, bounded message, explicit untracked flag, and spawn blocking.
- Line 240: test `stack git stash file backend uses dedicated stash refs instead of commit-hash validators` - Covers Stack git stash file backend uses dedicated stash refs instead of commit-hash validators.
- Line 249: test `StackGitPanel renders visible history and stash UI with destructive confirmations` - Covers StackGitPanel renders visible history and stash UI with destructive confirmations.
- Line 272: test `stack git stash drawer matches history geometry and opens after the closing row shell` - Covers Stack git stash drawer matches history geometry and opens after the closing row shell.
- Line 283: test `stack git commit push validates remote early and preserves refresh warnings after success` - Covers Stack git commit push validates remote early and preserves refresh warnings after success.
- Line 292: test `Stack Browser navigation shortcuts never capture Backspace from editable controls` - Covers Stack Browser navigation shortcuts never capture Backspace from editable controls.
- Line 300: test `StackGitPanel uses OpenChamber Git display geometry instead of card tabs and split panes` - Covers StackGitPanel uses OpenChamber Git display geometry instead of card tabs and split panes.
- Line 323: test `StackGitPanel replaces the file grid instead of overlaying it` - Covers StackGitPanel replaces the file grid instead of overlaying.
- Line 331: test `StackGitPanel branch button opens a grouped branch dropdown, not the Branches view` - Covers StackGitPanel branch button opens a grouped branch dropdown, not the Branches view.
- Line 348: test `StackGitPanel branch dropdown rows checkout and create branch from selected source ref` - Covers StackGitPanel branch dropdown rows checkout and create branch from selected source ref.
- Line 367: test `StackGitPanel branch dropdown measures available space and cleans resize listener lifecycle` - Covers StackGitPanel branch dropdown measures available space and cleans resize listener lifecycle.
- Line 397: test `StackGitPanel marks local branches checked out in another worktree` - Covers StackGitPanel marks local branches checked out in another worktree.
- Line 406: test `stack git branches checkout prefers local branch for matching remote tail` - Covers Stack git branches checkout prefers local branch for matching remote tail.
- Line 413: test `stack git checkout refreshes changed files and resets dropdown state to the checked out branch` - Covers Stack git checkout refreshes changed files and resets dropdown state to the checked out branch.
- Line 421: test `stack git dropdown current branch uses the same live status source as the header label` - Covers Stack git dropdown current branch uses the same live status source as the header label.
- Line 431: test `stack git dropdown confirms local branch deletion and protects current and remote branches` - Covers Stack git dropdown confirms local branch deletion and protects current and remote branches.
- Line 452: test `linked-worktree branch deletion confirms once then removes optimistically and forces backend cleanup` - Covers Linked-worktree branch deletion confirms once then removes optimistically and forces backend cleanup.
- Line 466: test `stack git create branch frontend contract forwards optional source branch` - Covers Stack git create branch frontend contract forwards optional source branch.
- Line 471: test `StackGitPanel inserts staged and unstaged diff drawers directly below the clicked row` - Covers StackGitPanel inserts staged and unstaged diff drawers directly below the clicked row.
- Line 569: test `stack git stash selection reconciles invalid refs and clears loading state when empty` - Covers Stack git stash selection reconciles invalid refs and clears loading state when empty.
- Line 588: test `StackGitPanel changes view collapses staged and unstaged groups with history-file row shell reuse` - Covers StackGitPanel changes view collapses staged and unstaged groups with history-file row shell reuse.
- Line 656: test `StackGitPanel shows sequential operation progress, detailed failures, and expandable output below header` - Covers StackGitPanel shows sequential operation progress, detailed failures, and expandable output below header.
- Line 672: test `stack git path emphasis splits Windows and Git separators` - Covers Stack git path emphasis splits Windows and Git separators.
- Line 683: test `stack git diff row parser hides patch headers and exposes source line gutters` - Covers Stack git diff row parser hides patch headers and exposes source line gutters.
- Line 729: test `stack git diff row parser safely blanks malformed and no-hunk content` - Covers Stack git diff row parser safely blanks malformed and no-hunk content.
- Line 749: test `stack git diff row parser resets gutters at file boundaries and blanks truncation notices` - Covers Stack git diff row parser resets gutters at file boundaries and blanks truncation notices.
- Line 769: test `StackGitPanel shared diff renderer owns one centered gutter for changes, history, and stashes` - Covers StackGitPanel shared diff renderer owns one centered gutter for changes, history, and stashes.

### tests/stackOpenWithAsyncBoundary.test.mjs

Audits stack open with async boundary behavior through 3 discovered test entries.

- Line 63: test `stack Open With command names remain stable` - Covers Stack Open With command names remain stable.
- Line 73: test `stack Open With commands are async and authorize before blocking work` - Covers Stack Open With commands are async and authorize before blocking work.
- Line 87: test `stack Open With filesystem and launch work is routed inside spawn_blocking` - Covers Stack Open With filesystem and launch work is routed inside spawn blocking.

### tests/stackPopupContextMenu.test.mjs

Audits stack popup context menu behavior through 5 discovered test entries.

- Line 9: test `row context menu exposes Open with picker plus suggested developer apps` - Covers Row context menu exposes Open with picker plus suggested developer apps.
- Line 22: test `Open with flyout has a bridge so rightward mouse movement stays inside submenu zone` - Covers Open with flyout has a bridge so rightward mouse movement stays inside submenu zone.
- Line 29: test `Open with picker is backed by a Tauri command wrapper` - Covers Open with picker is backed by a Tauri command wrapper.
- Line 34: test `background context menu is available off rows and keeps selection actions` - Covers Background context menu is available off rows and keeps selection actions.
- Line 57: test `stack browser publishes native-friendly file drag payloads as copy operations` - Covers Stack browser publishes native-friendly file drag payloads as copy operations.

### tests/stackPopupDiagnosticsWiring.test.mjs

Audits stack popup diagnostics wiring behavior through 1 discovered test entries.

- Line 7: test `stack folder listing publishes baseline diagnostics hooks for timing and payload metrics` - Covers Stack folder listing publishes baseline diagnostics hooks for timing and payload metrics.

### tests/stackPopupGitStatus.test.mjs

Audits stack popup git status behavior through 19 discovered test entries.

- Line 27: test `stack git status backend is a separate non-listing command using git porcelain` - Covers Stack git status backend is a separate non-listing command using git porcelain.
- Line 60: test `stack popup API exposes typed git branch counts and per-path status entries` - Covers Stack popup API exposes typed git branch counts and per-path status entries.
- Line 86: test `stack popup API exposes typed git workbench command wrappers ahead of backend wiring` - Covers Stack popup API exposes typed git workbench command wrappers ahead of backend wiring.
- Line 102: test `stack git mutation results preserve bounded command output for on-demand inspection` - Covers Stack git mutation results preserve bounded command output for on-demand inspection.
- Line 112: test `stack git nonzero classifier preserves labeled stdout and stderr in merged diagnostics` - Covers Stack git nonzero classifier preserves labeled stdout and stderr in merged diagnostics.
- Line 121: test `stack git branches expose linked-worktree occupancy from porcelain-z output` - Covers Stack git branches expose linked-worktree occupancy from porcelain-z output.
- Line 131: test `stack git create branch contract accepts optional source branch and uses fixed source checkout argv` - Covers Stack git create branch contract accepts optional source branch and uses fixed source checkout argv.
- Line 147: test `stack git delete branch contract is local-only, confirmed by UI, and uses safe fixed argv` - Covers Stack git delete branch contract is local-only, confirmed by UI, and uses safe fixed argv.
- Line 162: test `stack git branch deletion can force-remove its linked worktree first` - Covers Stack git branch deletion can force-remove its linked worktree first.
- Line 181: test `stack git remote checkout preserves remote branch tail as local branch name` - Covers Stack git remote checkout preserves remote branch tail as local branch name.
- Line 188: test `stack git remote url builder uses GitLab nested-group routes and root fallback` - Covers Stack git remote url builder uses GitLab nested-group routes and root fallback.
- Line 196: test `stack git status detached branch stays empty instead of synthesizing short sha` - Covers Stack git status detached branch stays empty instead of synthesizing short sha.
- Line 203: test `stack popup loads git status outside folder listing and guards stale responses` - Covers Stack popup loads git status outside folder listing and guards stale responses.
- Line 256: test `stack popup renders branch summary and minimal row git badges` - Covers Stack popup renders branch summary and minimal row git badges.
- Line 289: test `stack git changes popup splits staged and unstaged sections and hides empties` - Covers Stack git changes popup splits staged and unstaged sections and hides empties.
- Line 297: test `stack git status dialog is upgraded into a dense developer workbench` - Covers Stack git status dialog is upgraded into a dense developer workbench.
- Line 335: test `stack git workbench rejects stale async data and confirms mutating git commands` - Covers Stack git workbench rejects stale async data and confirms mutating git commands.
- Line 358: test `stack git badges have distinct compact colors without changing row layout columns` - Covers Stack git badges have distinct compact colors without changing row layout columns.
- Line 371: test `stack git workbench uses edge-to-edge OpenChamber geometry without card nesting` - Covers Stack git workbench uses edge-to-edge OpenChamber geometry without card nesting.

### tests/stackPopupMarqueeWiring.test.mjs

Audits stack popup marquee wiring behavior through 3 discovered test entries.

- Line 14: test `stack browser marquee starts only from details background and spacer surfaces` - Covers Stack browser marquee starts only from details background and spacer surfaces.
- Line 23: test `stack browser marquee preserves row drag and resize pointer ownership` - Covers Stack browser marquee preserves row drag and resize pointer ownership.
- Line 30: test `stack browser marquee has overlay styling and modest gutter affordance` - Covers Stack browser marquee has overlay styling and modest gutter affordance.

### tests/stackPopupNewTextFileFlow.test.mjs

Audits stack popup new text file flow behavior through 3 discovered test entries.

- Line 32: test `created text file rename plan selects created row and immediately enters rename editor` - Covers Created text file rename plan selects created row and immediately enters rename editor.
- Line 46: test `new text file creation delegates rename behavior to the view-model helper` - Covers New text file creation delegates rename behavior to the view-model helper.
- Line 55: test `new folder and ordinary rename still use the existing inline editor focus path` - Covers New folder and ordinary rename still use the existing inline editor focus path.

### tests/stackPopupPagingPhase1Wiring.test.mjs

Audits stack popup paging phase1wiring behavior through 2 discovered test entries.

- Line 8: test `stack folder first-paint path materializes metadata rows without shell icon extraction calls` - Covers Stack folder first-paint path materializes metadata rows without shell icon extraction calls.
- Line 13: test `stack popup rows render fallback icon shape when iconDataUrl is absent` - Covers Stack popup rows render fallback icon shape when iconDataUrl is absent.

### tests/stackPopupPagingPhase2Wiring.test.mjs

Audits stack popup paging phase2wiring behavior through 3 discovered test entries.

- Line 8: test `stack folder paging uses steady chunk sizes and no 500-row follow-up burst` - Covers Stack folder paging uses steady chunk sizes and no 500-row follow-up burst.
- Line 19: test `stack folder paging forwards a listing session id between page requests` - Covers Stack folder paging forwards a listing session id between page requests.
- Line 26: test `stack folder paging continuation slices session page then clones slice only` - Covers Stack folder paging continuation slices session page then clones slice only.

### tests/stackPopupPagingPhase3Wiring.test.mjs

Audits stack popup paging phase3wiring behavior through 3 discovered test entries.

- Line 12: test `stack popup exposes an icon-resolution command path distinct from metadata paging` - Covers Stack popup exposes an icon-resolution command path distinct from metadata paging.
- Line 18: test `stack popup icon hydration queue is bounded and does not schedule unbounded work` - Covers Stack popup icon hydration queue is bounded and does not schedule unbounded work.
- Line 24: test `stack popup tracks icon hydration completion separately from metadata loading` - Covers Stack popup tracks icon hydration completion separately from metadata loading.

### tests/stackPopupPagingPhase4Wiring.test.mjs

Audits stack popup paging phase4wiring behavior through 3 discovered test entries.

- Line 14: test `stack popup emits visible-row window updates for virtualization-aware progressive loading` - Covers Stack popup emits visible-row window updates for virtualization-aware progressive loading.
- Line 19: test `stack popup progressive page handling uses a guarded focus helper instead of unconditional per-page focus` - Covers Stack popup progressive page handling uses a guarded focus helper instead of unconditional per-page focus.
- Line 29: test `stack popup virtual window contract remains explicit for progressive append compatibility` - Covers Stack popup virtual window contract remains explicit for progressive append compatibility.

### tests/stackPopupPagingPhase5Wiring.test.mjs

Audits stack popup paging phase5wiring behavior through 4 discovered test entries.

- Line 11: test `stack folder diagnostics include phase-5 timing milestones and icon-cache counters` - Covers Stack folder diagnostics include phase-5 timing milestones and icon-cache counters.
- Line 19: test `stack folder listing emits explicit metadata first-paint and metadata-complete diagnostics phases` - Covers Stack folder listing emits explicit metadata first-paint and metadata-complete diagnostics phases.
- Line 24: test `stack popup icon hydration tracks icon-cache hits and misses for completion diagnostics` - Covers Stack popup icon hydration tracks icon-cache hits and misses for completion diagnostics.
- Line 31: test `stack popup keeps stale guards around async icon diagnostics completion` - Covers Stack popup keeps stale guards around async icon diagnostics completion.

### tests/stackPopupPagingPhase6Responsiveness.test.mjs

Audits stack popup paging phase6responsiveness behavior through 8 discovered test entries.

- Line 9: test `stack icon command is async and moves shell icon extraction off the command thread` - Covers Stack icon command is async and moves shell icon extraction off the command thread.
- Line 16: test `icon hydration prioritizes visible rows without restarting the whole folder queue` - Covers Icon hydration prioritizes visible rows without restarting the whole folder queue.
- Line 24: test `visible-row priority and window telemetry use filtered visible entries` - Covers Visible-row priority and window telemetry use filtered visible entries.
- Line 39: test `folder search input refreshes filtered visible-row icon priority without scroll events` - Covers Folder search input refreshes filtered visible-row icon priority without scroll events.
- Line 52: test `icon progress counts queued visible-priority rows as unresolved` - Covers Icon progress counts queued visible-priority rows as unresolved.
- Line 63: test `icon hydration applies one merged state update per resolved batch` - Covers Icon hydration applies one merged state update per resolved batch.
- Line 70: test `icon cache miss resolves shell icon outside the cache mutex` - Covers Icon cache miss resolves shell icon outside the cache mutex.
- Line 83: test `stack/search/process icon caches are bounded by ttl and capacity constants` - Covers Stack/search/process icon caches are bounded by ttl and capacity constants.

### tests/stackPopupState.test.mjs

Audits stack popup state behavior through 39 discovered test entries.

- Line 59: test `keeps stack navigation history across folder switching` - Covers Keeps stack navigation history across folder switching.
- Line 73: test `does not duplicate current folder when reopened after hide/show` - Covers Does not duplicate current folder when reopened after hide/show.
- Line 81: test `normalizes stack popup open request payloads` - Covers Normalizes stack popup open request payloads.
- Line 89: test `keys stack popup open requests by request id when available` - Covers Keys stack popup open requests by request id when available.
- Line 98: test `normalizes extended windows paths for display and navigation` - Covers Normalizes extended windows paths for display and navigation.
- Line 107: test `builds valid breadcrumb paths for drive and unc folders` - Covers Builds valid breadcrumb paths for drive and unc folders.
- Line 118: test `drops forward history after branching to a different folder` - Covers Drops forward history after branching to a different folder.
- Line 128: test `retains previous entries while a different folder is loading` - Covers Retains previous entries while a different folder is loading.
- Line 151: test `matches canonical git status paths to stack entry paths` - Covers Matches canonical git status paths to stack entry paths.
- Line 180: test `typed path validation success commits current path history and listing together` - Covers Typed path validation success commits current path history and listing together.
- Line 199: test `typed path validation failure leaves previous folder state non-retained` - Covers Typed path validation failure leaves previous folder state non-retained.
- Line 220: test `retains previous entries while navigating history loads another folder` - Covers Retains previous entries while navigating history loads another folder.
- Line 242: test `clears retained-row state once the requested folder page arrives` - Covers Clears retained-row state once the requested folder page arrives.
- Line 261: test `applies entries and preserves valid selection` - Covers Applies entries and preserves valid selection.
- Line 280: test `supports toggle, range, and select-all stack selection` - Covers Supports toggle, range, and select-all stack selection.
- Line 302: test `applies marquee-selected stack entry paths while filtering stale and duplicate paths` - Covers Applies marquee-selected stack entry paths while filtering stale and duplicate paths.
- Line 319: test `preserves visible selections and drops stale selections after refresh` - Covers Preserves visible selections and drops stale selections after refresh.
- Line 331: test `finds the next type-to-select match from current selection` - Covers Finds the next type-to-select match from current selection.
- Line 339: test `sorts stack entries deterministically while preserving folders first` - Covers Sorts stack entries deterministically while preserving folders first.
- Line 361: test `sorts modified desc with folders/files grouped by timestamp and nulls last` - Covers Sorts modified desc with folders/files grouped by timestamp and nulls last.
- Line 376: test `updates stack sort column and toggles direction` - Covers Updates stack sort column and toggles direction.
- Line 390: test `reports stack sort header aria state and active indicator` - Covers Reports stack sort header aria state and active indicator.
- Line 415: test `suggests useful Open With apps for developer text files` - Covers Suggests useful Open With apps for developer text files.
- Line 430: test `ignores stale folder entry payloads` - Covers Ignores stale folder entry payloads.
- Line 447: test `applies complete stack folder listings with partial warning status` - Covers Applies complete stack folder listings with partial warning status.
- Line 479: test `reports progressive count status as soon as first metadata page lands` - Covers Reports progressive count status as soon as first metadata page lands.
- Line 490: test `merges incremental stack folder pages for immediate first render` - Covers Merges incremental stack folder pages for immediate first render.
- Line 520: test `progressive stack folder merge grows visible rows monotonically by page` - Covers Progressive stack folder merge grows visible rows monotonically by page.
- Line 558: test `switching folders resets progressive merge and avoids old-row append into new folder` - Covers Switching folders resets progressive merge and avoids old-row append into new folder.
- Line 587: test `ignores stale folder listing payloads` - Covers Ignores stale folder listing payloads.
- Line 612: test `extracts stack item names from windows paths` - Covers Extracts stack item names from windows paths.
- Line 617: test `classifies stack browser row icons by folder and file extension` - Covers Classifies stack browser row icons by folder and file extension.
- Line 633: test `stack browser fallback icon metadata does not use text abbreviations` - Covers Stack browser fallback icon metadata does not use text abbreviations.
- Line 644: test `updates row icons in place without changing progressive row ordering` - Covers Updates row icons in place without changing progressive row ordering.
- Line 673: test `preserves hydrated row icons when same-folder listing refresh omits icon data` - Covers Preserves hydrated row icons when same-folder listing refresh omits icon data.
- Line 708: test `same-folder refresh keeps previous icon only when incoming icon is nullish and explicit icon wins` - Covers Same-folder refresh keeps previous icon only when incoming icon is nullish and explicit icon wins.
- Line 737: test `reports icon hydration progress distinctly from metadata listing completion` - Covers Reports icon hydration progress distinctly from metadata listing completion.
- Line 745: test `status transitions cover loading, progressive count, metadata done, and icon completion` - Covers Status transitions cover loading, progressive count, metadata done, and icon completion.
- Line 760: test `regression: progressive merge grows past legacy 80-row first page without stalling` - Covers Regression: progressive merge grows past legacy 80-row first page without stalling.

### tests/stackPopupViewModel.test.mjs

Audits stack popup view model behavior through 13 discovered test entries.

- Line 16: test `declares background context menu ignore selectors for interactive stack chrome` - Covers Declares background context menu ignore selectors for interactive stack chrome.
- Line 44: test `calculates a bounded virtual row window for large folders` - Covers Calculates a bounded virtual row window for large folders.
- Line 62: test `keeps small folders unvirtualized to preserve simple row semantics` - Covers Keeps small folders unvirtualized to preserve simple row semantics.
- Line 72: test `calculates scroll positions that keep keyboard-selected virtual rows mounted` - Covers Calculates scroll positions that keep keyboard-selected virtual rows mounted.
- Line 87: test `normalizes stack browser marquee rectangles from either drag direction` - Covers Normalizes stack browser marquee rectangles from either drag direction.
- Line 98: test `selects visible stack rows whose bounds intersect the marquee rectangle` - Covers Selects visible stack rows whose bounds intersect the marquee rectangle.
- Line 122: test `additive stack browser marquee keeps existing selection and appends intersecting rows once` - Covers Additive stack browser marquee keeps existing selection and appends intersecting rows once.
- Line 143: test `virtual stack browser marquee selects unmounted rows by row geometry during autoscroll` - Covers Virtual stack browser marquee selects unmounted rows by row geometry during autoscroll.
- Line 164: test `virtual stack browser additive marquee preserves drag-start and prior drag selections once rows unmount` - Covers Virtual stack browser additive marquee preserves drag-start and prior drag selections once rows unmount.
- Line 185: test `collapses middle breadcrumbs while preserving root and current folder` - Covers Collapses middle breadcrumbs while preserving root and current folder.
- Line 209: test `builds explicit delete prompt state from visible selection only` - Covers Builds explicit delete prompt state from visible selection only.
- Line 224: test `delete prompt falls back to selectedPath and rejects empty selections` - Covers Delete prompt falls back to selectedPath and rejects empty selections.
- Line 239: test `filters stack browser entries by name and path search text` - Covers Filters stack browser entries by name and path search text.

### tests/stackTextEditorAdmission.test.mjs

Audits stack text editor admission behavior through 10 discovered test entries.

- Line 4: test `checked quota includes every retained asset; unknown and overflow refuse` - Covers Checked quota includes every retained asset; unknown and overflow refuse.
- Line 14: test `input cap refuses new admission, preserves rejected text, requires spool for bulk` - Covers Input cap refuses new admission, preserves rejected text, requires spool for bulk.
- Line 15: it `o1` - Covers O1.
- Line 16: it `o2` - Covers O2.
- Line 16: it `big` - Covers Big.
- Line 20: it `n${n}` - Covers N${n}.
- Line 21: it `full` - Covers Full.
- Line 23: test `interactive and demand outrank background; bounded jobs cancel and close` - Covers Interactive and demand outrank background; bounded jobs cancel and close.
- Line 30: test `denied clipboard never cuts; invalid range never reaches clipboard` - Covers Denied clipboard never cuts; invalid range never reaches clipboard.
- Line 39: test `clipboard completion cannot cut after intervening edit or undo` - Covers Clipboard completion cannot cut after intervening edit or undo.

### tests/stackTextEditorFingerprint.test.mjs

Audits stack text editor fingerprint behavior through 1 discovered test entries.

- Line 7: test `canonical request bytes match Rust and changed Unicode payload rejects same ID` - Covers Canonical request bytes match Rust and changed Unicode payload rejects same ID.

### tests/stackTextEditorHarness.test.mjs

Audits stack text editor harness behavior through 8 discovered test entries.

- Line 9: test `seeded corpus preserves complete UTF-8/UTF-16 patterns/BOM and JSON framing` - Covers Seeded corpus preserves complete UTF-8/UTF-16 patterns/BOM and JSON framing.
- Line 27: test `corpus metadata and first chunk independent of requested file size` - Covers Corpus metadata and first chunk independent of requested file size.
- Line 33: test `small byte oracle random operations match independent splice and preserve rejection` - Covers Small byte oracle random operations match independent splice and preserve rejection.
- Line 45: test `controlled range I/O withholds EOF, reorders completions and reports failures` - Covers Controlled range I/O withholds EOF, reorders completions and reports failures.
- Line 54: test `oracle preserves UTF BOMs and rejects encoded scalar/CRLF interiors` - Covers Oracle preserves UTF BOMs and rejects encoded scalar/CRLF interiors.
- Line 71: test `publication failpoints never disguise prepublication failure or postpublication ambiguity` - Covers Publication failpoints never disguise prepublication failure or postpublication ambiguity.
- Line 85: test `credits bound blocked work, keep latest demand and reject unavailable workers` - Covers Credits bound blocked work, keep latest demand and reject unavailable workers.
- Line 97: test `metadata allowlist rejects source text at record and serialization boundaries` - Covers Metadata allowlist rejects source text at record and serialization boundaries.

### tests/stackTextEditorProtocol.test.mjs

Audits stack text editor protocol behavior through 10 discovered test entries.

- Line 18: test `lossless wire integers reject noncanonical and numeric inputs` - Covers Lossless wire integers reject noncanonical and numeric inputs.
- Line 22: test `shared UTF-8/UTF-16 scalar/CRLF vectors map exact bytes above 2^53` - Covers Shared UTF-8/UTF-16 scalar/CRLF vectors map exact bytes above 2^53.
- Line 28: test `reject malformed scalars, forged newline maps, source overflow, unknown fields` - Covers Reject malformed scalars, forged newline maps, source overflow, unknown fields.
- Line 34: test `lease bounds count rows across all segments including base row` - Covers Lease bounds count rows across all segments including base row.
- Line 43: test `one backend selection combines two independent leases and retains direction` - Covers One backend selection combines two independent leases and retains direction.
- Line 53: test `anchor affinity remaps insertion, deletion and overflow without float conversion` - Covers Anchor affinity remaps insertion, deletion and overflow without float conversion.
- Line 60: test `truncated grapheme context is never certified complete` - Covers Truncated grapheme context is never certified complete.
- Line 67: test `ledger exact-once, changed payload, rejected retention and safe retirement` - Covers Ledger exact-once, changed payload, rejected retention and safe retirement.
- Line 80: test `snapshot barrier rejects pending, missing and stale visible input` - Covers Snapshot barrier rejects pending, missing and stale visible input.
- Line 86: test `all snapshot-dependent requests require barrier; strict insertion variants` - Covers All snapshot-dependent requests require barrier; strict insertion variants.

### tests/stackTextEditorResults.test.mjs

Audits stack text editor results behavior through 5 discovered test entries.

- Line 6: test `shared result wire vectors agree with Rust including explicit nulls` - Covers Shared result wire vectors agree with Rust including explicit nulls.
- Line 13: test `open has exact explicit encoding, bounded path and no caller authority` - Covers Open has exact explicit encoding, bounded path and no caller authority.
- Line 19: test `result error and job schemas reject ambiguity and wrong nullable fields` - Covers Result error and job schemas reject ambiguity and wrong nullable fields.
- Line 30: test `selection remaps both endpoints and expires rather than reviving invalidated handles` - Covers Selection remaps both endpoints and expires rather than reviving invalidated handles.
- Line 38: test `wire endpoint resolves only through matching authoritative lease view` - Covers Wire endpoint resolves only through matching authoritative lease view.

### tests/surfaceCodeSplitting.test.mjs

Audits surface code splitting behavior through 3 discovered test entries.

- Line 36: test `App lazy-loads routed surfaces instead of statically importing surface components` - Covers App lazy-loads routed surfaces instead of statically importing surface components.
- Line 37: it `/` - Covers /.
- Line 47: test `surface loader has a dynamic import for every supported shell surface` - Covers Surface loader has a dynamic import for every supported shell surface.

### tests/systemTray.test.mjs

Audits system tray behavior through 5 discovered test entries.

- Line 9: test `normalizes tray icons with stable labels and image fallback` - Covers Normalizes tray icons with stable labels and image fallback.
- Line 19: test `treats placeholder-only tray snapshots as fallback glyphs` - Covers Treats placeholder-only tray snapshots as fallback glyphs.
- Line 29: test `drops duplicate tray ids from snapshots` - Covers Drops duplicate tray ids from snapshots.
- Line 38: test `preserves app and overflow tray entries when source-qualified ids differ` - Covers Preserves app and overflow tray entries when source-qualified ids differ.
- Line 50: test `serializes tray click request payloads for backend relay` - Covers Serializes tray click request payloads for backend relay.

### tests/taskPreviewRetention.test.mjs

Audits task preview retention behavior through 6 discovered test entries.

- Line 36: test `preview outer root owns full-bounds hover retention handlers` - Covers Preview outer root owns full-bounds hover retention handlers.
- Line 46: test `preview pointer leave ignores top-half/internal transitions and hides only outside root` - Covers Preview pointer leave ignores top-half/internal transitions and hides only outside root.
- Line 55: test `scheduled hide keeps preview alive until backend hide event arrives` - Covers Scheduled hide keeps preview alive until backend hide event arrives.
- Line 63: test `preview close button is accessible red X and does not activate preview` - Covers Preview close button is accessible red X and does not activate preview.
- Line 84: test `close previewed task window wrapper validates external hwnd and command wiring` - Covers Close previewed task window wrapper validates external hwnd and command wiring.
- Line 96: test `task window close falls back to terminating the owning process` - Covers Task window close falls back to terminating the owning process.

### tests/taskPreviewTextPolish.test.mjs

Audits task preview text polish behavior through 4 discovered test entries.

- Line 21: test `preview header separates primary title from secondary process text` - Covers Preview header separates primary title from secondary process text.
- Line 29: test `preview close button stays out of text flow with reserved header space` - Covers Preview close button stays out of text flow with reserved header space.
- Line 43: test `preview text truncates long title and process labels without stealing frame space` - Covers Preview text truncates long title and process labels without stealing frame space.
- Line 53: test `native and captured preview frames keep unobstructed dominant layout` - Covers Native and captured preview frames keep unobstructed dominant layout.

### tests/taskWindowSnapshotPipeline.test.mjs

Audits task window snapshot pipeline behavior through 6 discovered test entries.

- Line 14: test `task window snapshots are producer-driven, not permanent frontend polls` - Covers Task window snapshots are producer-driven, not permanent frontend polls.
- Line 23: test `task window snapshot stream is sequenced with last snapshot fallback` - Covers Task window snapshot stream is sequenced with last snapshot fallback.
- Line 30: test `notification lookup cache is bounded and negative-cached` - Covers Notification lookup cache is bounded and negative-cached.
- Line 38: test `taskbar snapshot consumers reject stale or equal sequences` - Covers Taskbar snapshot consumers reject stale or equal sequences.
- Line 45: test `producer avoids holding locks across enum/winrt/icon emit` - Covers Producer avoids holding locks across enum/winrt/icon emit.
- Line 51: test `taskbar snapshot worker keeps watchdog cadence and prioritizes native refreshes` - Covers Taskbar snapshot worker keeps watchdog cadence and prioritizes native refreshes.

### tests/taskbarAttentionModel.test.mjs

Audits taskbar attention model behavior through 6 discovered test entries.

- Line 26: test `phase 0 native attention contract stays independent from phase 2/4 fields` - Covers Phase 0 native attention contract stays independent from phase 2/4 fields.
- Line 32: test `native attention flash hook exists in native_hooks.rs and does not infer flash from toast terms` - Covers Native attention flash hook exists in native hooks.rs and does not infer flash from toast terms.
- Line 55: test `flash fixture lives under task_windows and has bounded deterministic helper args` - Covers Flash fixture lives under task windows and has bounded deterministic helper args.
- Line 81: test `task_windows facade wrappers stay exposed and native hooks module stays private` - Covers Task windows facade wrappers stay exposed and native hooks module stays private.
- Line 92: test `attention survives missing creation time until a window snapshot resolves it` - Covers Attention survives missing creation time until a window snapshot resolves.
- Line 99: test `bottom bar binds attention only on the requesting tile, not the whole group` - Covers Bottom bar binds attention only on the requesting tile, not the whole group.

### tests/taskbarGalleryContract.test.mjs

Audits taskbar gallery contract behavior through 17 discovered test entries.

- Line 16: test `native capsule wiring imports and uses gallery helpers` - Covers Native capsule wiring imports and uses gallery helpers.
- Line 24: test `capsule renders native gallery affordance only` - Covers Capsule renders native gallery affordance only.
- Line 34: test `capsule outer width follows the shared equal flex contract` - Covers Capsule outer width follows the shared equal flex contract.
- Line 43: test `capsule hover opens gallery after a cancellable dwell` - Covers Capsule hover opens gallery after a cancellable dwell.
- Line 53: test `gallery lifecycle closes on escape and stale snapshot` - Covers Gallery lifecycle closes on escape and stale snapshot.
- Line 65: test `direct task buttons keep existing action semantics` - Covers Direct task buttons keep existing action semantics.
- Line 71: test `gallery preview generations use shared allocation and native pass-through` - Covers Gallery preview generations use shared allocation and native pass-through.
- Line 82: test `gallery blur honors native context-menu focus hold` - Covers Gallery blur honors native context-menu focus hold.
- Line 86: test `gallery preview close is nonce-scoped to the exact window` - Covers Gallery preview close is nonce-scoped to the exact window.
- Line 96: test `closing a gallery preview retains the gallery until snapshot count leaves capsule mode` - Covers Closing a gallery preview retains the gallery until snapshot count leaves capsule mode.
- Line 107: test `gallery menu resolves the clicked window pid natively` - Covers Gallery menu resolves the clicked window pid natively.
- Line 113: test `hover-open gallery does not steal native focus` - Covers Hover-open gallery does not steal native focus.
- Line 122: test `gallery closes after pointer leaves both tabs and task preview` - Covers Gallery closes after pointer leaves both tabs and task preview.
- Line 139: test `gallery surface is one horizontal tab strip with exact window labels` - Covers Gallery surface is one horizontal tab strip with exact window labels.
- Line 153: test `gallery tabs match bottom-bar task button styling` - Covers Gallery tabs match bottom-bar task button styling.
- Line 165: test `same-session refresh preserves focus and updates native geometry from tab count` - Covers Same-session refresh preserves focus and updates native geometry from tab count.
- Line 177: test `snapshot closes gallery when its group leaves capsule mode` - Covers Snapshot closes gallery when its group leaves capsule mode.

### tests/taskbarGalleryUxState.test.mjs

Audits taskbar gallery ux state behavior through 3 discovered test entries.

- Line 13: test `gallery filtering matches title and process name while preserving order` - Covers Gallery filtering matches title and process name while preserving order.
- Line 25: test `gallery keyboard focus handles arrows Home and End without wrapping` - Covers Gallery keyboard focus handles arrows Home and End without wrapping.
- Line 35: test `gallery focus reconciliation keeps focused HWND, removes stale HWND, and handles empty lists` - Covers Gallery focus reconciliation keeps focused HWND, removes stale HWND, and handles empty lists.

### tests/taskbarGroupDisplayMode.test.mjs

Audits taskbar group display mode behavior through 12 discovered test entries.

- Line 36: test `supported policies keep one-window groups direct` - Covers Supported policies keep one-window groups direct.
- Line 42: test `auto policy capsules groups with two or more windows` - Covers Auto policy capsules groups with two or more windows.
- Line 49: test `pressure only collapses multi-window groups under auto policy` - Covers Pressure only collapses multi-window groups under auto policy.
- Line 54: test `always policy capsules multi-window groups while preserving single direct window` - Covers Always policy capsules multi-window groups while preserving single direct window.
- Line 59: test `never policy keeps multi-window groups direct even under pressure` - Covers Never policy keeps multi-window groups direct even under pressure.
- Line 63: test `representative window prefers active child and otherwise first stable child` - Covers Representative window prefers active child and otherwise first stable child.
- Line 71: test `gallery items preserve original window order` - Covers Gallery items preserve original window order.
- Line 81: test `aggregate state derives active minimized attention busy and toast metadata from children` - Covers Aggregate state derives active minimized attention busy and toast metadata from children.
- Line 96: test `strip pressure enters only after the deficit exceeds the enter threshold` - Covers Strip pressure enters only after the deficit exceeds the enter threshold.
- Line 114: test `strip pressure remains active near the boundary even with small spare width` - Covers Strip pressure remains active near the boundary even with small spare width.
- Line 124: test `strip pressure exits only when spare width reaches the exit threshold` - Covers Strip pressure exits only when spare width reaches the exit threshold.
- Line 142: test `strip pressure preserves previous pressure when available width is zero or unmeasured` - Covers Strip pressure preserves previous pressure when available width is zero or unmeasured.

### tests/taskbarGroups.test.mjs

Audits taskbar groups behavior through 30 discovered test entries.

- Line 32: test `groups open task windows by application identity` - Covers Groups open task windows by application identity.
- Line 46: test `keeps group toast count at the highest window count` - Covers Keeps group toast count at the highest window count.
- Line 56: test `marks a task window group attentive when any child requests attention` - Covers Marks a task window group attentive when any child requests attention.
- Line 66: test `marks a task window group busy when any eligible contained window is busy` - Covers Marks a task window group busy when any eligible contained window is busy.
- Line 75: test `suppresses generic busy task window group indicators` - Covers Suppresses generic busy task window group indicators.
- Line 83: test `suppresses generic windows with llm text in the title` - Covers Suppresses generic windows with llm text in the title.
- Line 104: test `allows terminal and llm busy task window group indicators` - Covers Allows terminal and llm busy task window group indicators.
- Line 114: test `allows browser busy indicators only with download metadata` - Covers Allows browser busy indicators only with download metadata.
- Line 124: test `requires browser identity from process metadata for download busy indicators` - Covers Requires browser identity from process metadata for download busy indicators.
- Line 139: test `reconciles dragged group order as windows appear and disappear` - Covers Reconciles dragged group order as windows appear and disappear.
- Line 146: test `moves task window groups without mutating the original order` - Covers Moves task window groups without mutating the original order.
- Line 154: test `moves task window groups after the target for end placement` - Covers Moves task window groups after the target for end placement.
- Line 161: test `starts pointer taskbar group drag only after movement threshold` - Covers Starts pointer taskbar group drag only after movement threshold.
- Line 167: test `keeps below-threshold pointer gestures eligible for click activation` - Covers Keeps below-threshold pointer gestures eligible for click activation.
- Line 171: test `calculates pointer drag delta for visible tile movement` - Covers Calculates pointer drag delta for visible tile movement.
- Line 176: test `calculates drag reorder compensation from captured rects instead of live transforms` - Covers Calculates drag reorder compensation from captured rects instead of live transforms.
- Line 187: test `returns zero drag compensation when the captured source rect is unavailable` - Covers Returns zero drag compensation when the captured source rect is unavailable.
- Line 194: test `chooses reorder placement from pointer position within target group` - Covers Chooses reorder placement from pointer position within target group.
- Line 199: test `chooses drag release target from centerlines including edge positions` - Covers Chooses drag release target from centerlines including edge positions.
- Line 220: test `chooses rightward drag targets that commit over neighbors and at the end` - Covers Chooses rightward drag targets that commit over neighbors and at the end.
- Line 242: test `uses stable centerlines for multi-step rightward drag placement` - Covers Uses stable centerlines for multi-step rightward drag placement.
- Line 264: test `commits a rightward release at the far edge to the final position` - Covers Commits a rightward release at the far edge to the final position.
- Line 279: test `chooses mirrored one-step drag targets from source displacement` - Covers Chooses mirrored one-step drag targets from source displacement.
- Line 297: test `chooses mirrored multi-step drag targets from source displacement` - Covers Chooses mirrored multi-step drag targets from source displacement.
- Line 317: test `restores original order when rightward displacement returns to source slot` - Covers Restores original order when rightward displacement returns to source slot.
- Line 333: test `keeps a positive visual offset after crossing the first right neighbor boundary` - Covers Keeps a positive visual offset after crossing the first right neighbor boundary.
- Line 346: test `restores original order when leftward displacement returns to source slot` - Covers Restores original order when leftward displacement returns to source slot.
- Line 362: test `chooses mirrored edge release targets from source displacement` - Covers Chooses mirrored edge release targets from source displacement.
- Line 390: test `committed drag order survives open-window refresh reconciliation` - Covers Committed drag order survives open-window refresh reconciliation.
- Line 399: test `uses window handle as fallback identity when process is unavailable` - Covers Uses window handle as fallback identity when process is unavailable.

### tests/taskbarLauncherReliability.test.mjs

Audits taskbar launcher reliability behavior through 14 discovered test entries.

- Line 38: test `Explorer taskbar pin listing does not hide shortcuts just because Resolve fails` - Covers Explorer taskbar pin listing does not hide shortcuts just because Resolve fails.
- Line 45: test `launcher icon extraction never falls back to the .lnk file association icon` - Covers Launcher icon extraction never falls back to the .lnk file association icon.
- Line 55: test `target icon stage only skips exact safe AppsFolder explorer proxies` - Covers Target icon stage only skips exact safe AppsFolder explorer proxies.
- Line 60: test `target icon decision contract stays positive for ordinary shortcuts and negative only for safe AppsFolder explorer proxies` - Covers Target icon decision contract stays positive for ordinary shortcuts and negative only for safe AppsFolder explorer proxies.
- Line 69: test `Explorer taskbar pin launch ShellExecutes the shortcut path without Resolve preflight` - Covers Explorer taskbar pin launch ShellExecutes the shortcut path without Resolve preflight.
- Line 75: test `Explorer taskbar pin launch retries with elevation on access denied` - Covers Explorer taskbar pin launch retries with elevation on access denied.
- Line 83: test `WindowsApps admin launch falls back to Explorer only for AppX targets` - Covers WindowsApps admin launch falls back to Explorer only for AppX targets.
- Line 93: test `explicit admin launch hands code 3 or access denied shortcut results to AppX fallback helper` - Covers Explicit admin launch hands code 3 or access denied shortcut results to AppX fallback helper.
- Line 102: test `elevated launcher helper has no status-file handoff and no helper argv path` - Covers Elevated launcher helper has no status-file handoff and no helper argv path.
- Line 108: test `Explorer launch failure does not remove the visible launcher row` - Covers Explorer launch failure does not remove the visible launcher row.
- Line 115: test `app-managed quick icon backend commands are no longer registered` - Covers App-managed quick icon backend commands are no longer registered.
- Line 120: test `Explorer launcher context menu exposes Windows taskbar unpin only through validated lnk path` - Covers Explorer launcher context menu exposes Windows taskbar unpin only through validated lnk path.
- Line 131: test `active task window context menu exposes PID lookup and taskbar pin actions` - Covers Active task window context menu exposes PID lookup and taskbar pin actions.
- Line 151: test `active taskbar pin creates a taskbar shortcut instead of reviving app-managed quick icons` - Covers Active taskbar pin creates a taskbar shortcut instead of reviving app-managed quick icons.

### tests/taskbarLauncherReorder.test.mjs

Audits taskbar launcher reorder behavior through 4 discovered test entries.

- Line 16: test `reorders pinned taskbar launchers left and right by shortcut path displacement` - Covers Reorders pinned taskbar launchers left and right by shortcut path displacement.
- Line 40: test `keeps launcher order stable for missing or same-position moves` - Covers Keeps launcher order stable for missing or same-position moves.
- Line 52: test `resolves launcher pointer release without launching after a drag` - Covers Resolves launcher pointer release without launching after a drag.
- Line 61: test `BottomBar wires launcher pointer reorder separately from click and native menus` - Covers BottomBar wires launcher pointer reorder separately from click and native menus.

### tests/taskbarPreviewContract.test.mjs

Audits taskbar preview contract behavior through 6 discovered test entries.

- Line 31: test `regular task previews allocate freshness ids from shared native state` - Covers Regular task previews allocate freshness ids from shared native state.
- Line 58: test `task preview payload contract exposes native live thumbnail flag and source` - Covers Task preview payload contract exposes native live thumbnail flag and source.
- Line 93: test `task preview surface gives native DWM thumbnails an unobstructed frame` - Covers Task preview surface gives native DWM thumbnails an unobstructed frame.
- Line 121: test `task preview publish path rechecks request freshness before emitting native state` - Covers Task preview publish path rechecks request freshness before emitting native state.
- Line 142: test `task preview publish path does not hold runtime mutex across Tauri window operations` - Covers Task preview publish path does not hold runtime mutex across Tauri window operations.
- Line 183: test `task window close path skips preview validator while preview capture still uses it` - Covers Task window close path skips preview validator while preview capture still uses.

### tests/taskbarRuntimeDiagnostics.test.mjs

Audits taskbar runtime diagnostics behavior through 3 discovered test entries.

- Line 8: test `normalizes missing taskbar diagnostics safely` - Covers Normalizes missing taskbar diagnostics safely.
- Line 20: test `keeps raw taskbar diagnostics bounded and redacted` - Covers Keeps raw taskbar diagnostics bounded and redacted.
- Line 54: test `preserves unique unresolved count beyond sample cap` - Covers Preserves unique unresolved count beyond sample cap.

### tests/taskbarSmokeScript.test.mjs

Audits taskbar smoke script behavior through 2 discovered test entries.

- Line 7: test `taskbar smoke script exposes deterministic exe, env, and latency contracts` - Covers Taskbar smoke script exposes deterministic exe, env, and latency contracts.
- Line 29: test `taskbar smoke script keeps bounded process launch and cleanup semantics` - Covers Taskbar smoke script keeps bounded process launch and cleanup semantics.

### tests/taskbarTilePointer.test.mjs

Audits taskbar tile pointer behavior through 4 discovered test entries.

- Line 9: test `tracks pending taskbar tile activation only for the primary button` - Covers Tracks pending taskbar tile activation only for the primary button.
- Line 14: test `promotes a non-drag pointer release into taskbar tile activation` - Covers Promotes a non-drag pointer release into taskbar tile activation.
- Line 21: test `suppresses post-drag taskbar tile clicks without activating the window` - Covers Suppresses post-drag taskbar tile clicks without activating the window.
- Line 28: test `suppresses only the matching taskbar tile click after pointer release handling` - Covers Suppresses only the matching taskbar tile click after pointer release handling.

### tests/taskbarUxState.test.mjs

Audits taskbar ux state behavior through 6 discovered test entries.

- Line 20: test `detects taskbar overflow and exposes keyboard guidance` - Covers Detects taskbar overflow and exposes keyboard guidance.
- Line 31: test `moves keyboard focus across taskbar items without wrapping unexpectedly` - Covers Moves keyboard focus across taskbar items without wrapping unexpectedly.
- Line 39: test `summarizes task group state for stronger accessible indicators` - Covers Summarizes task group state for stronger accessible indicators.
- Line 55: test `summarizes inactive attention and toast state with combined labels` - Covers Summarizes inactive attention and toast state with combined labels.
- Line 71: test `renders attentive task groups with toast badge, amber cue, and active suppression` - Covers Renders attentive task groups with toast badge, amber cue, and active suppression.
- Line 88: test `sizes task buttons as equal flex items without content-sized bounds` - Covers Sizes task buttons as equal flex items without content-sized bounds.

### tests/taskbarWindows.test.mjs

Audits taskbar windows behavior through 4 discovered test entries.

- Line 9: test `task window frontend command keeps optimistic activation and minimize intent` - Covers Task window frontend command keeps optimistic activation and minimize intent.
- Line 14: test `taskbar labels expose active-window minimize toggle` - Covers Taskbar labels expose active-window minimize toggle.
- Line 18: test `task window activation updates active highlight before cached snapshot refresh` - Covers Task window activation updates active highlight before cached snapshot refresh.
- Line 25: test `taskbar window frontend payload normalizes attention and toast counts` - Covers Taskbar window frontend payload normalizes attention and toast counts.

### tests/terminalActions.test.mjs

Audits terminal actions behavior through 7 discovered test entries.

- Line 15: test `terminal menus close on outside pointer without eating internal menu clicks` - Covers Terminal menus close on outside pointer without eating internal menu clicks.
- Line 26: test `terminal action gating avoids scanning xterm buffer during reactive state updates` - Covers Terminal action gating avoids scanning xterm buffer during reactive state updates.
- Line 33: test `terminal cwd actions use effective cwd fallback instead of session-only state` - Covers Terminal cwd actions use effective cwd fallback instead of session-only state.
- Line 42: test `terminal action registry defines state-gated Phase 6 actions` - Covers Terminal action registry defines state-gated Phase 6 actions.
- Line 66: test `terminal split actions are always available from the toolbar and bootstrap startup if needed` - Covers Terminal split actions are always available from the toolbar and bootstrap startup if needed.
- Line 78: test `recent terminal history is bounded in memory and not persisted` - Covers Recent terminal history is bounded in memory and not persisted.
- Line 91: test `quick-fix parser remains pure but terminal overlay is hidden for now` - Covers Quick-fix parser remains pure but terminal overlay is hidden for now.

### tests/terminalQuickSelect.test.mjs

Audits terminal quick select behavior through 3 discovered test entries.

- Line 13: test `terminal quick select detects safe daily developer targets` - Covers Terminal quick select detects safe daily developer targets.
- Line 25: test `terminal quick select safety rejects unsafe open targets` - Covers Terminal quick select safety rejects unsafe open targets.
- Line 34: test `persistent terminal wires quick select overlay and keyboard cancellation` - Covers Persistent terminal wires quick select overlay and keyboard cancellation.

### tests/terminalShellIntegration.test.mjs

Audits terminal shell integration behavior through 3 discovered test entries.

- Line 14: test `terminal shell integration parser supports OSC command and cwd markers` - Covers Terminal shell integration parser supports OSC command and cwd markers.
- Line 27: test `visible terminal registers frontend OSC parser and command actions` - Covers Visible terminal registers frontend OSC parser and command actions.
- Line 41: test `PowerShell and Git Bash static integration inject markers without dotfile mutation` - Covers PowerShell and Git Bash static integration inject markers without dotfile mutation.

### tests/terminalTabTitle.test.mjs

Audits terminal tab title behavior through 6 discovered test entries.

- Line 13: test `terminal tab title preserves explicit manual rename above dynamic state` - Covers Terminal tab title preserves explicit manual rename above dynamic state.
- Line 23: test `terminal tab title prefers minimal running command names without output text` - Covers Terminal tab title prefers minimal running command names without output text.
- Line 44: test `terminal tab title formats common developer processes like Windows Terminal` - Covers Terminal tab title formats common developer processes like Windows Terminal.
- Line 51: test `terminal tab title does not treat typed-but-unsubmitted input as a running process` - Covers Terminal tab title does not treat typed-but-unsubmitted input as a running process.
- Line 55: test `terminal tab title uses normalized directory path when idle and profile only as fallback` - Covers Terminal tab title uses normalized directory path when idle and profile only as fallback.
- Line 61: test `terminal tab title strips control noise, collapses whitespace, and truncates` - Covers Terminal tab title strips control noise, collapses whitespace, and truncates.

### tests/terminalWorkbenchState.test.mjs

Audits terminal workbench state behavior through 4 discovered test entries.

- Line 23: test `new header tab from split workbench preserves the old tab workbench and stops no existing panes` - Covers New header tab from split workbench preserves the old tab workbench and stops no existing panes.
- Line 47: test `switching back to a split tab restores its exact pane tree without stopping hidden panes` - Covers Switching back to a split tab restores its exact pane tree without stopping hidden panes.
- Line 64: test `closing a hidden split tab stops only sessions owned by that tab` - Covers Closing a hidden split tab stops only sessions owned by that tab.
- Line 84: test `closing the active split tab immediately activates the neighbor tab workbench` - Covers Closing the active split tab immediately activates the neighbor tab workbench.

### tests/testModernizationPolicy.test.mjs

Audits test modernization policy behavior through 4 discovered test entries.

- Line 22: test `selected brittle visual tests remove exact CSS literals` - Covers Selected brittle visual tests remove exact CSS literals.
- Line 28: test `selected brittle visual tests require behavioral replacement before literal visual assertions are removed` - Covers Selected brittle visual tests require behavioral replacement before literal visual assertions are removed.
- Line 35: test `modernization policy keeps security event and capability invariants out of broad regex purge` - Covers Modernization policy keeps security event and capability invariants out of broad regex purge.
- Line 44: test `modernization policy stays focused on selected visual tests rather than banning all source regex tests` - Covers Modernization policy stays focused on selected visual tests rather than banning all source regex tests.

### tests/themeRegistry.test.mjs

Audits theme registry behavior through 4 discovered test entries.

- Line 163: test `theme registry exposes base themes and popular editor palettes` - Covers Theme registry exposes base themes and popular editor palettes.
- Line 191: test `theme normalization and DOM application are storage-safe` - Covers Theme normalization and DOM application are storage-safe.
- Line 218: test `theme is applied before Svelte mounts to reduce shell paint mismatch` - Covers Theme is applied before Svelte mounts to reduce shell paint mismatch.
- Line 223: test `theme sync listens for cross-webview updates and releases listeners` - Covers Theme sync listens for cross-webview updates and releases listeners.

### tests/topBarCalendar.test.mjs

Audits top bar calendar behavior through 14 discovered test entries.

- Line 25: test `calendar month model builds a Sunday-first six-week grid with adjacent month days` - Covers Calendar month model builds a Sunday-first six-week grid with adjacent month days.
- Line 44: test `calendar month navigation supports other years without losing weekday labels` - Covers Calendar month navigation supports other years without losing weekday labels.
- Line 57: test `calendar labels expose long date and local timezone information` - Covers Calendar labels expose long date and local timezone information.
- Line 72: test `calendar wheel navigation accumulates pixel deltas to a 100px threshold` - Covers Calendar wheel navigation accumulates pixel deltas to a 100px threshold.
- Line 91: test `calendar wheel direction maps down to next month and up to previous month` - Covers Calendar wheel direction maps down to next month and up to previous month.
- Line 102: test `calendar wheel direction reversal clears stale accumulation` - Covers Calendar wheel direction reversal clears stale accumulation.
- Line 111: test `calendar wheel navigation throttles to one month per 500ms including exact boundary` - Covers Calendar wheel navigation throttles to one month per 500ms including exact boundary.
- Line 124: test `calendar wheel idle gap over 700ms clears partial accumulation` - Covers Calendar wheel idle gap over 700ms clears partial accumulation.
- Line 133: test `calendar wheel navigation normalizes line and page delta modes` - Covers Calendar wheel navigation normalizes line and page delta modes.
- Line 141: test `calendar wheel huge delta navigates at most one month without backlog` - Covers Calendar wheel huge delta navigates at most one month without backlog.
- Line 152: test `calendar wheel horizontal-dominant input is ignored and not consumed` - Covers Calendar wheel horizontal-dominant input is ignored and not consumed.
- Line 159: test `calendar wheel consumes vertical input even below threshold or during cooldown` - Covers Calendar wheel consumes vertical input even below threshold or during cooldown.
- Line 173: test `top bar time pill owns an Explorer-like scrollable calendar flyout` - Covers Top bar time pill owns an Explorer-like scrollable calendar flyout.
- Line 196: test `calendar panel is a dedicated top-bar anchored webview so it is not clipped by the compact bar` - Covers Calendar panel is a dedicated top-bar anchored webview so is not clipped by the compact bar.

### tests/topBarFolderReorder.test.mjs

Audits top bar folder reorder behavior through 8 discovered test entries.

- Line 10: test `reorders first pinned folder to the end` - Covers Reorders first pinned folder to the end.
- Line 14: test `reorders last pinned folder to the beginning` - Covers Reorders last pinned folder to the beginning.
- Line 18: test `reorders a middle pinned folder by path not display name` - Covers Reorders a middle pinned folder by path not display name.
- Line 22: test `keeps same array for same slot and missing source` - Covers Keeps same array for same slot and missing source.
- Line 28: test `TopBar pinned folders wire drag reorder through existing persistence path` - Covers TopBar pinned folders wire drag reorder through existing persistence path.
- Line 37: test `TopBar keeps click and context menu behavior below drag threshold` - Covers TopBar keeps click and context menu behavior below drag threshold.
- Line 46: test `TopBar holds stack popup focus while a pinned folder is pressed` - Covers TopBar holds stack popup focus while a pinned folder is pressed.
- Line 59: test `TopBar ordinary pointerdown hides stack popup but pinned-folder pointerdown keeps switch path` - Covers TopBar ordinary pointerdown hides stack popup but pinned-folder pointerdown keeps switch path.

### tests/topBarPins.test.mjs

Audits top bar pins behavior through 6 discovered test entries.

- Line 14: test `does not reveal the last persisted pin during the initial load` - Covers Does not reveal the last persisted pin during the initial load.
- Line 18: test `reveals newly added pins after the initial load` - Covers Reveals newly added pins after the initial load.
- Line 22: test `prefers an explicitly requested visible pin path` - Covers Prefers an explicitly requested visible pin path.
- Line 26: test `detects added pins case-insensitively` - Covers Detects added pins case-insensitively.
- Line 30: test `uses a webview-window scoped event target for top-bar pin updates` - Covers Uses a webview-window scoped event target for top-bar pin updates.
- Line 35: test `TopBar applyStackPins does not recursively reload pins during hydration` - Covers TopBar applyStackPins does not recursively reload pins during hydration.

### tests/topBarTimeoutHygiene.test.mjs

Audits top bar timeout hygiene behavior through 1 discovered test entries.

- Line 7: test `top bar tracks and clears delayed rail scroll updates` - Covers Top bar tracks and clears delayed rail scroll updates.

### tests/trayPanelRetention.test.mjs

Audits tray panel retention behavior through 6 discovered test entries.

- Line 28: test `tray icon invoke has local retention guard and never closes tray panel` - Covers Tray icon invoke has local retention guard and never closes tray panel.
- Line 41: test `tray invoke failure stores inline invoke error and keeps snapshot visible` - Covers Tray invoke failure stores inline invoke error and keeps snapshot visible.
- Line 51: test `rapid left and right tray icon activation share same invoke guard` - Covers Rapid left and right tray icon activation share same invoke guard.
- Line 61: test `top bar only observes tray-panel closed event, never tray icon invoke outcome` - Covers Top bar only observes tray-panel closed event, never tray icon invoke outcome.
- Line 70: test `system tray backend has explicit no-panel-close error contract for stale invoke` - Covers System tray backend has explicit no-panel-close error contract for stale invoke.
- Line 76: test `native tray icon invoke suppresses the next tray focus-loss close only` - Covers Native tray icon invoke suppresses the next tray focus-loss close only.

### tests/trayPanelWiring.test.mjs

Audits tray panel wiring behavior through 7 discovered test entries.

- Line 22: test `tray panel is routed as a dedicated auxiliary shell surface` - Covers Tray panel is routed as a dedicated auxiliary shell surface.
- Line 40: test `tray panel contracts and wrappers use constant-backed IPC/event names` - Covers Tray panel contracts and wrappers use constant-backed IPC/event names.
- Line 57: test `top bar exposes tray button left of sound and keeps popup state mutually exclusive` - Covers Top bar exposes tray button left of sound and keeps popup state mutually exclusive.
- Line 71: test `tray panel surface renders icon-only grid with loading, error, and empty states` - Covers Tray panel surface renders icon-only grid with loading, error, and empty states.
- Line 91: test `tray panel supports keyboard secondary action and Escape close` - Covers Tray panel supports keyboard secondary action and Escape close.
- Line 104: test `tray panel exposes visible close control` - Covers Tray panel exposes visible close control.
- Line 110: test `tray panel Rust placement anchors to right edge and clamps within top bar bounds` - Covers Tray panel Rust placement anchors to right edge and clamps within top bar bounds.

### tests/vscodeFolderOpenPhase4.test.mjs

Audits vscode folder open phase4 behavior through 5 discovered test entries.

- Line 14: test `phase 4 adds VS Code folder-open IPC command wrappers` - Covers Phase 4 adds VS Code folder-open IPC command wrappers.
- Line 20: test `phase 4 stack browser context menus expose folder and current-path Open in VS Code` - Covers Phase 4 stack browser context menus expose folder and current-path Open in VS Code.
- Line 31: test `phase 4 top-bar pin menu includes Open in VS Code and dispatches action payload` - Covers Phase 4 top-bar pin menu includes Open in VS Code and dispatches action payload.
- Line 38: test `phase 4 backend includes shared VS Code resolver with safe missing-install error` - Covers Phase 4 backend includes shared VS Code resolver with safe missing-install error.
- Line 50: test `phase 4 rejects bare executable PATH candidates for VS Code and terminal launch` - Covers Phase 4 rejects bare executable PATH candidates for VS Code and terminal launch.

### tests/windowsKeyOverride.test.mjs

Audits windows key override behavior through 14 discovered test entries.

- Line 9: test `native hook callback only classifies and hands off to bounded queue` - Covers Native hook callback only classifies and hands off to bounded queue.
- Line 21: test `worker owns AppHandle and emit_to for hook events` - Covers Worker owns AppHandle and emit to for hook events.
- Line 32: test `TopBar listens for native Ctrl+Space search toggle through existing centered paths` - Covers TopBar listens for native Ctrl+Space search toggle through existing centered paths.
- Line 43: test `top and bottom shell surfaces catch Ctrl+Space when their webviews have focus` - Covers Top and bottom shell surfaces catch Ctrl+Space when their webviews have focus.
- Line 92: test `Alt+Backquote terminal hotkey emits and shell surfaces toggle terminal panel` - Covers Alt+Backquote terminal hotkey emits and shell surfaces toggle terminal panel.
- Line 110: test `Alt+1 toggles Stack Browser from native hook and top bar wiring` - Covers Alt+1 toggles Stack Browser from native hook and top bar wiring.
- Line 147: test `top bar places Stack Browser toggle button between settings and pinned folders` - Covers Top bar places Stack Browser toggle button between settings and pinned folders.
- Line 160: test `Stack Browser toggle reopens latest request without emitting open event` - Covers Stack Browser toggle reopens latest request without emitting open event.
- Line 172: test `native hook installs during setup and cleans up on exit` - Covers Native hook installs during setup and cleans up on exit.
- Line 179: test `startup fails when required search hotkey hook cannot install` - Covers Startup fails when required search hotkey hook cannot install.
- Line 187: test `native hook passes through when hook state is unavailable` - Covers Native hook passes through when hook state is unavailable.
- Line 198: test `native Ctrl+Space hotkey toggles once and does not capture Windows key` - Covers Native Ctrl+Space hotkey toggles once and does not capture Windows key.
- Line 212: test `fullscreen guard instrumentation tracks duration and wake counts` - Covers Fullscreen guard instrumentation tracks duration and wake counts.
- Line 223: test `legacy taskbar guard refreshes exact owned snapshots through reconcile` - Covers Legacy taskbar guard refreshes exact owned snapshots through reconcile.

### tests/workspaces.test.mjs

Audits workspaces behavior through 4 discovered test entries.

- Line 69: test `workspace IPC wrapper uses centralized command names registered by Rust` - Covers Workspace IPC wrapper uses centralized command names registered by Rust.
- Line 83: test `activation plan exposes top-bar pins and searchable workspace task results` - Covers Activation plan exposes top-bar pins and searchable workspace task results.
- Line 99: test `workspace search bias boosts only results under active workspace roots` - Covers Workspace search bias boosts only results under active workspace roots.
- Line 129: test `startup summary makes non-execution explicit` - Covers Startup summary makes non-execution explicit.

## Rust cargo test cases

### src-tauri/src/appbar.rs

Audits appbar behavior through 42 discovered test entries.

- Line 2101: rust test `reserved_work_area_uses_actual_reserved_edges` - Covers Reserved work area uses actual reserved edges.
- Line 2133: rust test `rollback_removes_appbar_registered_before_later_setup_failure` - Covers Rollback removes appbar registered before later setup failure.
- Line 2215: rust test `failed_appbar_removal_remains_tracked_and_blocks_fresh_restore_registration` - Covers Failed appbar removal remains tracked and blocks fresh restore registration.
- Line 2240: rust test `successful_appbar_removal_clears_all_tracking` - Covers Successful appbar removal clears all tracking.
- Line 2255: rust test `cleanup_runtime_state_with_v2_restores_exact_snapshots_without_replacement_planning` - Covers Cleanup runtime state with v2 restores exact snapshots without replacement planning.
- Line 2304: rust test `legacy_taskbar_guard_cleanup_uses_latest_owned_snapshots` - Covers Legacy taskbar guard cleanup uses latest owned snapshots.
- Line 2350: rust test `legacy_taskbar_guard_starts_even_with_empty_owned_snapshot_list` - Covers Legacy taskbar guard starts even with empty owned snapshot list.
- Line 2371: rust test `visible_primary_taskbar_overrides_dirty_work_area_baseline` - Covers Visible primary taskbar overrides dirty work area baseline.
- Line 2414: rust test `hidden_primary_taskbar_with_dirty_work_area_recovers_taskbar_baseline` - Covers Hidden primary taskbar with dirty work area recovers taskbar baseline.
- Line 2457: rust test `startup_stabilization_reapplies_rect_after_zero_height_poll` - Covers Startup stabilization reapplies rect after zero height poll.
- Line 2520: rust test `parked_rect_stays_below_a_negative_coordinate_virtual_desktop` - Covers Parked rect stays below a negative coordinate virtual desktop.
- Line 2546: rust test `fullscreen_restore_layout_uses_newly_negotiated_bottom_rect` - Covers Fullscreen restore layout uses newly negotiated bottom rect.
- Line 2609: rust test `shell_bar_resize_is_rejected_while_fullscreen_releases_appbars` - Covers Shell bar resize is rejected while fullscreen releases appbars.
- Line 2632: rust test `shell_bar_resize_is_rejected_while_activation_is_in_progress` - Covers Shell bar resize is rejected while activation is in progress.
- Line 2645: rust test `concurrent_activation_cannot_duplicate_guard_startup` - Covers Concurrent activation cannot duplicate guard startup.
- Line 2657: rust test `slow_activation_side_effect_does_not_block_independent_state_lock` - Covers Slow activation side effect does not block independent state lock.
- Line 2669: rust test `partial_appbar_taskbar_rollback_leaves_retryable_state` - Covers Partial appbar taskbar rollback leaves retryable state.
- Line 2719: rust test `cleanup_failure_retains_failed_registration_and_blocks_activation_retry` - Covers Cleanup failure retains failed registration and blocks activation retry.
- Line 2748: rust test `cleanup_during_activation_side_effects_returns_busy_without_state_change` - Covers Cleanup during activation side effects returns busy without state change.
- Line 2769: rust test `cleanup_already_in_progress_rejects_second_cleanup` - Covers Cleanup already in progress rejects second cleanup.
- Line 2782: rust test `cleanup_failure_retains_failed_taskbar_snapshot_and_work_area_baseline` - Covers Cleanup failure retains failed taskbar snapshot and work area baseline.
- Line 2818: rust test `cleanup_success_finalizes_idle_and_cleaned_up` - Covers Cleanup success finalizes idle and cleaned up.
- Line 2841: rust test `activation_rollback_success_resets_runtime_and_clears_ownership` - Covers Activation rollback success resets runtime and clears ownership.
- Line 2877: rust test `guard_startup_blocks_activation_and_cleanup` - Covers Guard startup blocks activation and cleanup.
- Line 2895: rust test `activation_rollback_failure_retains_residue_and_disables_activation` - Covers Activation rollback failure retains residue and disables activation.
- Line 2927: rust test `commit_phase_mismatch_rejects_before_owning_effects` - Covers Commit phase mismatch rejects before owning effects.
- Line 2976: rust test `commit_phase_failure_helper_preserves_local_ownership` - Covers Commit phase failure helper preserves local ownership.
- Line 3024: rust test `guard_handles_extract_under_lock_and_join_after_lock_release` - Covers Guard handles extract under lock and join after lock release.
- Line 3054: rust test `guard_retry_delay_caps_at_four_seconds` - Covers Guard retry delay caps at four seconds.
- Line 3067: rust test `released_state_hides_via_park_action_and_restore_is_eligible_from_released` - Covers Released state hides via park action and restore is eligible from released.
- Line 3085: rust test `work_area_sync_reports_diagnostics_without_failing_transition` - Covers Work area sync reports diagnostics without failing transition.
- Line 3116: rust test `best_effort_work_area_sync_does_not_fail_on_mismatch` - Covers Best effort work area sync does not fail on mismatch.
- Line 3144: rust test `guard_retry_skips_until_deadline_then_retries_and_target_change_bypasses` - Covers Guard retry skips until deadline then retries and target change bypasses.
- Line 3173: rust test `guard_retry_success_and_failure_state_updates_are_deterministic` - Covers Guard retry success and failure state updates are deterministic.
- Line 3187: rust test `foreground_candidate_absent_or_non_hide_restores_shell` - Covers Foreground candidate absent or non hide restores shell.
- Line 3195: rust test `top_appbar_keeps_requested_height_after_query_offset` - Covers Top appbar keeps requested height after query offset.
- Line 3217: rust test `normalized_bottom_appbar_keeps_requested_height` - Covers Normalized bottom appbar keeps requested height.
- Line 3234: rust test `fullscreen_candidate_covering_monitor_hides_shell` - Covers Fullscreen candidate covering monitor hides shell.
- Line 3264: rust test `framed_work_area_candidate_does_not_hide_shell` - Covers Framed work area candidate does not hide shell.
- Line 3294: rust test `borderless_work_area_candidate_hides_shell` - Covers Borderless work area candidate hides shell.
- Line 3324: rust test `shell_or_hidden_candidates_do_not_hide_shell` - Covers Shell or hidden candidates do not hide shell.
- Line 3376: rust test `fullscreen_rect_cover_tolerates_small_dwm_offsets` - Covers Fullscreen rect cover tolerates small dwm offsets.

### src-tauri/src/audio.rs

Audits audio behavior through 2 discovered test entries.

- Line 733: rust test `audio_percent_helpers_use_bounded_scalar_values` - Covers Audio percent helpers use bounded scalar values.
- Line 745: rust test `audio_flow_aliases_normalize_to_supported_directions` - Covers Audio flow aliases normalize to supported directions.

### src-tauri/src/audio_panel.rs

Audits audio panel behavior through 2 discovered test entries.

- Line 106: rust test `anchors_audio_panel_to_sound_button_right_edge` - Covers Anchors audio panel to sound button right edge.
- Line 111: rust test `clamps_audio_panel_inside_top_bar_edges` - Covers Clamps audio panel inside top bar edges.

### src-tauri/src/automation.rs

Audits automation behavior through 6 discovered test entries.

- Line 307: rust test `cli_parser_requires_explicit_local_automation_opt_in` - Covers Cli parser requires explicit local automation opt in.
- Line 316: rust test `cli_parser_accepts_read_only_provider_listing_with_opt_in` - Covers Cli parser accepts read only provider listing with opt in.
- Line 327: rust test `unauthenticated_mutation_requires_user_present_boundary` - Covers Unauthenticated mutation requires user present boundary.
- Line 342: rust test `destructive_action_requires_auth_user_presence_and_confirmation` - Covers Destructive action requires auth user presence and confirmation.
- Line 377: rust test `cli_payload_cannot_request_arbitrary_executable_execution` - Covers Cli payload cannot request arbitrary executable execution.
- Line 395: rust test `forwarding_contract_is_plan_only_and_never_executes_payloads` - Covers Forwarding contract is plan only and never executes payloads.

### src-tauri/src/calendar_panel.rs

Audits calendar panel behavior through 2 discovered test entries.

- Line 106: rust test `anchors_calendar_panel_to_time_button_right_edge` - Covers Anchors calendar panel to time button right edge.
- Line 111: rust test `clamps_calendar_panel_inside_top_bar_edges` - Covers Clamps calendar panel inside top bar edges.

### src-tauri/src/command_panel.rs

Audits command panel behavior through 6 discovered test entries.

- Line 190: rust test `places_command_panel_within_work_area_and_keeps_width_when_it_fits` - Covers Places command panel within work area and keeps width when fits.
- Line 205: rust test `shrinks_command_panel_before_positioning_when_work_area_is_too_narrow` - Covers Shrinks command panel before positioning when work area is too narrow.
- Line 220: rust test `clamps_command_panel_height_to_work_area` - Covers Clamps command panel height to work area.
- Line 235: rust test `clamps_saved_command_panel_size_to_monitor_work_area` - Covers Clamps saved command panel size to monitor work area.
- Line 250: rust test `invalidate_command_panel_focus_loss_nonce_returns_current_nonce_until_next_invalidation` - Covers Invalidate command panel focus loss nonce returns current nonce until next invalidation.
- Line 263: rust test `suppress_next_resize_save_consumes_single_resize_event` - Covers Suppress next resize save consumes single resize event.

### src-tauri/src/contracts.rs

Audits contracts behavior through 4 discovered test entries.

- Line 484: rust test `shell_surface_contract_contains_current_windows` - Covers Shell surface contract contains current windows.
- Line 508: rust test `shell_surface_contract_includes_all_shipped_shell_windows` - Covers Shell surface contract includes all shipped shell windows.
- Line 524: rust test `new_command_contracts_are_unique_and_stable` - Covers New command contracts are unique and stable.
- Line 622: rust test `core_event_contracts_are_stable` - Covers Core event contracts are stable.

### src-tauri/src/control_plane.rs

Audits control plane behavior through 2 discovered test entries.

- Line 86: rust test `control_plane_size_is_clamped_to_monitor_bounds` - Covers Control plane size is clamped to monitor bounds.
- Line 93: rust test `control_plane_position_is_centered_inside_monitor_padding` - Covers Control plane position is centered inside monitor padding.

### src-tauri/src/dev_tools/git_status.rs

Audits git status behavior through 3 discovered test entries.

- Line 196: rust test `parses_clean_dirty_ahead_behind_and_conflicts` - Covers Parses clean dirty ahead behind and conflicts.
- Line 221: rust test `detects_merge_and_rebase_files` - Covers Detects merge and rebase files.
- Line 234: rust test `reads_status_from_temp_git_repo_when_git_is_available` - Covers Reads status from temp git repo when git is available.

### src-tauri/src/dev_tools/task_runner.rs

Audits task runner behavior through 6 discovered test entries.

- Line 546: rust test `rejects_direct_arbitrary_command_request_payloads` - Covers Rejects direct arbitrary command request payloads.
- Line 560: rust test `resolves_only_declared_tasks_from_active_workspace_settings` - Covers Resolves only declared tasks from active workspace settings.
- Line 581: rust test `rejects_inactive_or_undeclared_workspace_task_identity` - Covers Rejects inactive or undeclared workspace task identity.
- Line 601: rust test `bounds_task_history_to_latest_entries` - Covers Bounds task history to latest entries.
- Line 641: rust test `spawn_and_cancel_task_process_without_shell` - Covers Spawn and cancel task process without shell.
- Line 654: rust test `output_sequences_are_task_monotonic_and_chunks_are_bounded` - Covers Output sequences are task monotonic and chunks are bounded.

### src-tauri/src/dev_tools/tool_plans.rs

Audits tool plans behavior through 4 discovered test entries.

- Line 241: rust test `expands_terminal_plan_as_argv_without_shell` - Covers Expands terminal plan as argv without shell.
- Line 258: rust test `expands_editor_file_plan_as_single_goto_argument` - Covers Expands editor file plan as single goto argument.
- Line 280: rust test `preserves_shell_metacharacters_in_paths_as_literal_args` - Covers Preserves shell metacharacters in paths as literal args.
- Line 299: rust test `rejects_template_executables_and_path_traversal` - Covers Rejects template executables and path traversal.

### src-tauri/src/diagnostics.rs

Audits diagnostics behavior through 2 discovered test entries.

- Line 172: rust test `diagnostic_ring_buffer_is_bounded` - Covers Diagnostic ring buffer is bounded.
- Line 187: rust test `diagnostics_redact_secret_fields_and_bearer_tokens` - Covers Diagnostics redact secret fields and bearer tokens.

### src-tauri/src/explorer.rs

Audits explorer behavior through 14 discovered test entries.

- Line 679: rust test `baseline_work_area_uses_original_taskbar_height` - Covers Baseline work area uses original taskbar height.
- Line 707: rust test `validate_identity_rejects_stale_handles` - Covers Validate identity rejects stale handles.
- Line 723: rust test `reconcile_taskbars_returns_fake_hide_ops_for_visible_snapshots` - Covers Reconcile taskbars returns fake hide ops for visible snapshots.
- Line 747: rust test `safe_restore_taskbars_only_keeps_hidden_visible_matches` - Covers Safe restore taskbars only keeps hidden visible matches.
- Line 770: rust test `classify_taskbar_edge_handles_all_edges_with_nonzero_origin` - Covers Classify taskbar edge handles all edges with nonzero origin.
- Line 828: rust test `hide_taskbars_if_needed_excludes_originally_hidden_snapshots` - Covers Hide taskbars if needed excludes originally hidden snapshots.
- Line 862: rust test `restore_safety_only_allows_exact_live_hidden_owned_snapshots` - Covers Restore safety only allows exact live hidden owned snapshots.
- Line 916: rust test `reconcile_owned_taskbars_skips_prehidden_replacement_and_keeps_exact_ownership` - Covers Reconcile owned taskbars skips prehidden replacement and keeps exact ownership.
- Line 963: rust test `reconcile_owned_taskbars_retries_initial_hide_failure_for_originally_visible_owned_taskbars` - Covers Reconcile owned taskbars retries initial hide failure for originally visible owned taskbars.
- Line 1004: rust test `reconcile_owned_taskbars_updates_same_identity_retry_after_successful_hide` - Covers Reconcile owned taskbars updates same identity retry after successful hide.
- Line 1040: rust test `reconcile_owned_taskbars_replaces_same_hwnd_with_new_pid_identity` - Covers Reconcile owned taskbars replaces same hwnd with new pid identity.
- Line 1079: rust test `reconcile_owned_taskbars_replaces_stale_owned_snapshot_for_new_hwnd` - Covers Reconcile owned taskbars replaces stale owned snapshot for new hwnd.
- Line 1119: rust test `primary_ownership_reconcile_hides_late_visible_primary_and_records_ownership` - Covers Primary ownership reconcile hides late visible primary and records ownership.
- Line 1165: rust test `primary_ownership_reconcile_ignores_prehidden_and_other_monitor_primary` - Covers Primary ownership reconcile ignores prehidden and other monitor primary.

### src-tauri/src/launchers.rs

Audits launchers behavior through 10 discovered test entries.

- Line 1062: rust test `detects_only_lnk_shortcuts` - Covers Detects only lnk shortcuts.
- Line 1069: rust test `uses_file_stem_for_launcher_name` - Covers Uses file stem for launcher name.
- Line 1077: rust test `trims_wide_icon_buffers` - Covers Trims wide icon buffers.
- Line 1094: rust test `fallback_launcher_icon_is_image_data_url` - Covers Fallback launcher icon is image data url.
- Line 1099: rust test `sanitizes_shortcut_names` - Covers Sanitizes shortcut names.
- Line 1104: rust test `shell_item_icon_stage_replaces_relaunch_property_path` - Covers Shell item icon stage replaces relaunch property path.
- Line 1112: rust test `apps_folder_arguments_require_exact_safe_parsing_name` - Covers Apps folder arguments require exact safe parsing name.
- Line 1128: rust test `apps_folder_proxy_avoids_explorer_target_icon` - Covers Apps folder proxy avoids explorer target icon.
- Line 1144: rust test `target_icon_stage_only_blocks_exact_apps_folder_explorer_proxies` - Covers Target icon stage only blocks exact apps folder explorer proxies.
- Line 1185: rust test `apps_folder_stage_errors_do_not_produce_icons` - Covers Apps folder stage errors do not produce icons.

### src-tauri/src/layout.rs

Audits layout behavior through 7 discovered test entries.

- Line 244: rust test `preview_rects_span_full_width` - Covers Preview rects span full width.
- Line 254: rust test `preview_rects_leave_center_workspace_gap` - Covers Preview rects leave center workspace gap.
- Line 265: rust test `monitor_shell_layout_uses_per_monitor_dpi_for_mixed_dpi_strips` - Covers Monitor shell layout uses per monitor dpi for mixed dpi strips.
- Line 278: rust test `monitor_shell_layout_assigns_primary_shell_and_secondary_task_strip_ownership` - Covers Monitor shell layout assigns primary shell and secondary task strip ownership.
- Line 295: rust test `popup_anchor_uses_source_monitor_scale_and_clamps_to_monitor_bounds` - Covers Popup anchor uses source monitor scale and clamps to monitor bounds.
- Line 320: rust test `popup_anchor_places_primary_top_popups_below_the_primary_top_bar` - Covers Popup anchor places primary top popups below the primary top bar.
- Line 344: rust test `task_strip_assignment_prefers_current_monitor_then_stable_previous_then_primary` - Covers Task strip assignment prefers current monitor then stable previous then primary.

### src-tauri/src/process_manager.rs

Audits process manager behavior through 24 discovered test entries.

- Line 1241: rust test `process_icon_cache_hit_is_reused_without_extraction` - Covers Process icon cache hit is reused without extraction.
- Line 1272: rust test `process_icon_cache_hit_is_available_while_miss_extracts` - Covers Process icon cache hit is available while miss extracts.
- Line 1324: rust test `process_icon_cache_source_splits_lookup_resolve_and_store` - Covers Process icon cache source splits lookup resolve and store.
- Line 1372: rust test `refuses_to_kill_system_or_current_process` - Covers Refuses to kill system or current process.
- Line 1379: rust test `computes_cpu_percent_from_elapsed_process_ticks` - Covers Computes cpu percent from elapsed process ticks.
- Line 1397: rust test `computes_memory_percent_from_total_memory` - Covers Computes memory percent from total memory.
- Line 1406: rust test `enriches_parent_and_descendant_process_context` - Covers Enriches parent and descendant process context.
- Line 1424: rust test `kill_guardrail_plan_keeps_tree_termination_non_executing` - Covers Kill guardrail plan keeps tree termination non executing.
- Line 1452: rust test `kill_guardrail_plan_refuses_protected_processes` - Covers Kill guardrail plan refuses protected processes.
- Line 1462: rust test `kill_guardrail_execution_requires_explicit_confirmation` - Covers Kill guardrail execution requires explicit confirmation.
- Line 1472: rust test `kill_guardrail_execution_refuses_current_pid_even_with_confirmation` - Covers Kill guardrail execution refuses current pid even with confirmation.
- Line 1483: rust test `kill_guardrail_execution_rejects_stale_descendant_confirmation` - Covers Kill guardrail execution rejects stale descendant confirmation.
- Line 1499: rust test `kill_guardrail_execution_rejects_tree_plan_execution` - Covers Kill guardrail execution rejects tree plan execution.
- Line 1517: rust test `kill_guardrail_execution_accepts_fresh_single_confirmation` - Covers Kill guardrail execution accepts fresh single confirmation.
- Line 1533: rust test `kill_guardrail_rejects_confirmation_with_stale_creation_time` - Covers Kill guardrail rejects confirmation with stale creation time.
- Line 1548: rust test `kill_guardrail_rejects_confirmation_with_stale_image_path` - Covers Kill guardrail rejects confirmation with stale image path.
- Line 1563: rust test `identity_limited_process_kill_fails_closed_without_explicit_policy` - Covers Identity limited process kill fails closed without explicit policy.
- Line 1578: rust test `fake_kill_revalidation_identity_mismatch_does_not_call_terminate` - Covers Fake kill revalidation identity mismatch does not call terminate.
- Line 1600: rust test `fake_kill_revalidation_query_failure_does_not_call_terminate` - Covers Fake kill revalidation query failure does not call terminate.
- Line 1618: rust test `fake_kill_revalidation_match_calls_terminate_once` - Covers Fake kill revalidation match calls terminate once.
- Line 1635: rust test `access_denied_status_matches_raw_and_hresult_only` - Covers Access denied status matches raw and hresult only.
- Line 1643: rust test `path_match_normalization_strips_verbatim_prefixes_slashes_and_case` - Covers Path match normalization strips verbatim prefixes slashes and case.
- Line 1659: rust test `kill_guardrail_execution_requires_workspace_warning_acknowledgement` - Covers Kill guardrail execution requires workspace warning acknowledgement.
- Line 1676: rust test `workspace_hints_use_current_or_dev_paths_without_workspace_crud` - Covers Workspace hints use current or dev paths without workspace crud.

### src-tauri/src/providers.rs

Audits providers behavior through 4 discovered test entries.

- Line 274: rust test `resolves_config_driven_provider_contract_with_bounded_budgets` - Covers Resolves config driven provider contract with bounded budgets.
- Line 303: rust test `rejects_duplicate_provider_ids_and_over_budget_providers` - Covers Rejects duplicate provider ids and over budget providers.
- Line 327: rust test `rejects_secret_like_provider_config_keys_and_values` - Covers Rejects secret like provider config keys and values.
- Line 348: rust test `rejects_arbitrary_executable_or_plugin_provider_config` - Covers Rejects arbitrary executable or plugin provider config.

### src-tauri/src/quick_commands.rs

Audits quick commands behavior through 31 discovered test entries.

- Line 1699: rust test `validate_quick_command_url_accepts_http_and_https` - Covers Validate quick command url accepts http and https.
- Line 1711: rust test `validate_quick_command_url_rejects_unsafe_schemes_whitespace_credentials_and_empty_authority` - Covers Validate quick command url rejects unsafe schemes whitespace credentials and empty authority.
- Line 1720: rust test `stop_marks_run_stopping_before_termination` - Covers Stop marks run stopping before termination.
- Line 1739: rust test `stop_failure_restores_running_state_and_preserves_transcript` - Covers Stop failure restores running state and preserves transcript.
- Line 1758: rust test `stop_rejects_reused_root_pid_before_terminate` - Covers Stop rejects reused root pid before terminate.
- Line 1774: rust test `stop_success_appends_single_stopped_history_entry` - Covers Stop success appends single stopped history entry.
- Line 1784: rust test `stop_state_uses_arc_job_handle_and_checked_resume_thread` - Covers Stop state uses arc job handle and checked resume thread.
- Line 1806: rust test `terminal_run_stays_queryable_until_event_and_persistence` - Covers Terminal run stays queryable until event and persistence.
- Line 1839: rust test `stopped_root_does_not_wait_for_descendant_held_output_pipes` - Covers Stopped root does not wait for descendant held output pipes.
- Line 1850: rust test `windows_job_termination_kills_nested_listener_descendant` - Covers Windows job termination kills nested listener descendant.
- Line 1956: rust test `root_exit_is_claimed_before_late_stop_can_relabel_it` - Covers Root exit is claimed before late stop can relabel.
- Line 1968: rust test `stop_helpers_and_terminator_seam_are_present` - Covers Stop helpers and terminator seam are present.
- Line 1980: rust test `parser_splits_chunks` - Covers Parser splits chunks.
- Line 1990: rust test `malformed_passthrough` - Covers Malformed passthrough.
- Line 2001: rust test `oversize_passthrough` - Covers Oversize passthrough.
- Line 2007: rust test `split_valid_marker_two_chunks` - Covers Split valid marker two chunks.
- Line 2028: rust test `split_utf8_stream_keeps_text_intact_across_chunks` - Covers Split utf8 stream keeps text intact across chunks.
- Line 2051: rust test `sanitize_terminal_text_consumes_common_escape_classes` - Covers Sanitize terminal text consumes common escape classes.
- Line 2057: rust test `split_csi_sequence_is_removed_across_chunks` - Covers Split csi sequence is removed across chunks.
- Line 2066: rust test `split_osc_st_sequence_is_removed_across_chunks` - Covers Split osc st sequence is removed across chunks.
- Line 2077: rust test `split_oem_dbcs_bytes_flush_on_boundary` - Covers Split oem dbcs bytes flush on boundary.
- Line 2085: rust test `split_marker_prefix_across_chunks` - Covers Split marker prefix across chunks.
- Line 2100: rust test `unclosed_marker_carry_bounded_at_prefix_plus_max_encoded` - Covers Unclosed marker carry bounded at prefix plus max encoded.
- Line 2108: rust test `output_event_payload_from_helper` - Covers Output event payload from helper.
- Line 2151: rust test `marker_kind_preserved_in_transcript` - Covers Marker kind preserved in transcript.
- Line 2191: rust test `marker_bounds` - Covers Marker bounds.
- Line 2210: rust test `redaction_helper` - Covers Redaction helper.
- Line 2214: rust test `transcript_bound` - Covers Transcript bound.
- Line 2253: rust test `run_ids_unique` - Covers Run ids unique.
- Line 2257: rust test `history_bounded` - Covers History bounded.
- Line 2280: rust test `state_to_history_uses_finished_fields` - Covers State to history uses finished fields.

### src-tauri/src/search/contracts.rs

Audits contracts behavior through 5 discovered test entries.

- Line 343: rust test `contracts_use_camel_case_json_fields` - Covers Contracts use camel case json fields.
- Line 354: rust test `progress_payload_represents_pending_before_providers` - Covers Progress payload represents pending before providers.
- Line 371: rust test `health_and_timing_contracts_keep_provider_diagnostics` - Covers Health and timing contracts keep provider diagnostics.
- Line 397: rust test `provider_signal_is_internal_and_defaults_from_old_json` - Covers Provider signal is internal and defaults from old json.
- Line 417: rust test `action_safety_rejects_empty_command_ids` - Covers Action safety rejects empty command ids.

### src-tauri/src/search/matcher.rs

Audits matcher behavior through 3 discovered test entries.

- Line 497: rust test `token_prefix_matches_multi_word_abbreviation_groups` - Covers Token prefix matches multi word abbreviation groups.
- Line 507: rust test `subsequence_matches_short_launcher_style_abbreviation` - Covers Subsequence matches short launcher style abbreviation.
- Line 517: rust test `small_edit_distance_is_bounded` - Covers Small edit distance is bounded.

### src-tauri/src/search/mod.rs

Audits mod behavior through 9 discovered test entries.

- Line 378: rust test `phase3_latest_registry_is_monotonic_and_older_sequences_become_stale` - Covers Phase3 latest registry is monotonic and older sequences become stale.
- Line 390: rust test `empty_response_preserves_query_identity` - Covers Empty response preserves query identity.
- Line 402: rust test `command_returns_settings_results_without_legacy_search_sources` - Covers Command returns settings results without legacy search sources.
- Line 415: rust test `command_reports_phase_four_providers_and_timings` - Covers Command reports phase four providers and timings.
- Line 436: rust test `command_keeps_legacy_hot_paths_out_of_visible_engine` - Covers Command keeps legacy hot paths out of visible engine.
- Line 449: rust test `command_returns_open_window_context_results` - Covers Command returns open window context results.
- Line 475: rust test `progressive_search_emits_local_then_provider_then_complete` - Covers Progressive search emits local then provider then complete.
- Line 534: rust test `search_engine_preserves_local_results_when_everything_is_unavailable` - Covers Search engine preserves local results when everything is unavailable.
- Line 577: rust test `stable_result_key_prefers_record_key` - Covers Stable result key prefers record key.

### src-tauri/src/search/phase0_harness.rs

Audits phase0 harness behavior through 1 discovered test entries.

- Line 490: rust test `phase0_harness_produces_progress_and_health_traces` - Covers Phase0 harness produces progress and health traces.

### src-tauri/src/search/phase3_harness.rs

Audits phase3 harness behavior through 3 discovered test entries.

- Line 263: rust test `phase3_latest_only_harness_meets_user_targets` - Covers Phase3 latest only harness meets user targets.
- Line 301: rust test `phase3_artifact_metrics_are_derived_from_observed_events` - Covers Phase3 artifact metrics are derived from observed events.
- Line 336: rust test `phase3_latest_only_harness_writes_artifact_bundle` - Covers Phase3 latest only harness writes artifact bundle.

### src-tauri/src/search/phase4_harness.rs

Audits phase4 harness behavior through 3 discovered test entries.

- Line 158: rust test `phase4_everything_overfetch_artifact_meets_targets` - Covers Phase4 everything overfetch artifact meets targets.
- Line 173: rust test `phase4_artifact_rejects_broken_phase3_regression_observations` - Covers Phase4 artifact rejects broken phase3 regression observations.
- Line 204: rust test `phase4_everything_overfetch_writes_artifact_bundle` - Covers Phase4 everything overfetch writes artifact bundle.

### src-tauri/src/search/providers/apps.rs

Audits apps behavior through 14 discovered test entries.

- Line 853: rust test `phase2_app_source_priority_signals_match_approved_cap_scale` - Covers Phase2 app source priority signals match approved cap scale.
- Line 863: rust test `app_index_cache_freshness_is_bounded_by_ttl` - Covers App index cache freshness is bounded by ttl.
- Line 874: rust test `indexes_start_menu_shortcuts_once_then_searches_in_memory` - Covers Indexes start menu shortcuts once then searches in memory.
- Line 901: rust test `cold_query_path_returns_cache_miss_without_scanning` - Covers Cold query path returns cache miss without scanning.
- Line 910: rust test `stale_app_cache_returns_existing_rows_while_refresh_is_deferred` - Covers Stale app cache returns existing rows while refresh is deferred.
- Line 930: rust test `empty_cache_reports_indexing_while_refresh_is_running` - Covers Empty cache reports indexing while refresh is running.
- Line 939: rust test `fresh_cache_reports_refresh_while_startup_warm_is_running` - Covers Fresh cache reports refresh while startup warm is running.
- Line 959: rust test `persisted_cache_round_trips_non_secret_metadata` - Covers Persisted cache round trips non secret metadata.
- Line 985: rust test `corrupt_persisted_cache_is_ignored` - Covers Corrupt persisted cache is ignored.
- Line 998: rust test `app_results_outrank_incidental_everything_scores` - Covers App results outrank incidental everything scores.
- Line 1015: rust test `launcher_style_vs_code_query_matches_visual_studio_code` - Covers Launcher style vs code query matches visual studio code.
- Line 1031: rust test `alias_only_app_match_gets_visible_highlight_fallback` - Covers Alias only app match gets visible highlight fallback.
- Line 1047: rust test `app_index_collapses_duplicate_launch_paths_to_single_entry` - Covers App index collapses duplicate launch paths to single entry.
- Line 1075: rust test `identity_collapse_prefers_non_windows_apps_path` - Covers Identity collapse prefers non windows apps path.

### src-tauri/src/search/providers/everything.rs

Audits everything behavior through 18 discovered test entries.

- Line 766: rust test `phase3_stale_sequence_does_not_enter_everything_sdk_boundary` - Covers Phase3 stale sequence does not enter everything sdk boundary.
- Line 792: rust test `phase3_sequence_is_checked_again_after_everything_lock_wait` - Covers Phase3 sequence is checked again after everything lock wait.
- Line 837: rust test `phase2_everything_run_count_signal_matches_approved_cap_scale` - Covers Phase2 everything run count signal matches approved cap scale.
- Line 845: rust test `cached_health_has_explicit_ttl` - Covers Cached health has explicit ttl.
- Line 862: rust test `simple_name_request_keeps_full_path_and_content_search_off` - Covers Simple name request keeps full path and content search off.
- Line 877: rust test `path_like_request_enables_full_path_search_only_for_paths` - Covers Path like request enables full path search only for paths.
- Line 898: rust test `folder_navigation_queries_stay_name_fast_but_boost_folders` - Covers Folder navigation queries stay name fast but boost folders.
- Line 929: rust test `app_like_queries_use_run_count_sort_without_content_search` - Covers App like queries use run count sort without content search.
- Line 944: rust test `cached_health_reuses_fresh_state_without_redetecting` - Covers Cached health reuses fresh state without redetecting.
- Line 981: rust test `search_everything_records_direct_sdk_latency_with_injected_runner` - Covers Search everything records direct sdk latency with injected runner.
- Line 1014: rust test `simple_name_query_returns_rows_without_path_mode` - Covers Simple name query returns rows without path mode.
- Line 1047: rust test `everything_request_limit_is_bounded_to_display_plus_overfetch` - Covers Everything request limit is bounded to display plus overfetch.
- Line 1054: rust test `detection_health_prefers_cached_sdk_ready_path` - Covers Detection health prefers cached sdk ready path.
- Line 1067: rust test `maps_everything_sdk_rows_to_new_contract_rows` - Covers Maps everything sdk rows to new contract rows.
- Line 1089: rust test `phase4_everything_keeps_overfetch_rows_for_canonical_rerank` - Covers Phase4 everything keeps overfetch rows for canonical rerank.
- Line 1124: rust test `phase4_everything_mapping_stays_bounded_to_approved_overfetch_cap` - Covers Phase4 everything mapping stays bounded to approved overfetch cap.
- Line 1148: rust test `compresses_everything_highlight_indexes_into_span_pairs` - Covers Compresses everything highlight indexes into span pairs.
- Line 1156: rust test `highlighted_file_name_markers_become_span_pairs` - Covers Highlighted file name markers become span pairs.

### src-tauri/src/search/providers/local.rs

Audits local behavior through 3 discovered test entries.

- Line 309: rust test `important_folder_queries_match_existing_dev_root_only` - Covers Important folder queries match existing dev root only.
- Line 341: rust test `missing_important_roots_emit_no_fake_rows` - Covers Missing important roots emit no fake rows.
- Line 356: rust test `bounded_command_rows_match_without_filesystem_scan` - Covers Bounded command rows match without filesystem scan.

### src-tauri/src/search/providers/open_windows.rs

Audits open windows behavior through 1 discovered test entries.

- Line 193: rust test `matches_open_window_title_and_app_name` - Covers Matches open window title and app name.

### src-tauri/src/search/providers/settings.rs

Audits settings behavior through 21 discovered test entries.

- Line 589: rust test `dataset_contains_required_windows_settings_rows` - Covers Dataset contains required windows settings rows.
- Line 612: rust test `settings_catalog_schema_and_action_safety_are_valid` - Covers Settings catalog schema and action safety are valid.
- Line 648: rust test `display_settings_matches_display_screen_and_monitor_aliases` - Covers Display settings matches display screen and monitor aliases.
- Line 664: rust test `sound_settings_matches_sound_audio_and_volume_aliases` - Covers Sound settings matches sound audio and volume aliases.
- Line 674: rust test `control_panel_matches_classic_control_intents` - Covers Control panel matches classic control intents.
- Line 690: rust test `windows_settings_matches_settings_app_intents` - Covers Windows settings matches settings app intents.
- Line 702: rust test `settings_actions_use_shell_uri_for_ms_settings` - Covers Settings actions use shell uri for ms settings.
- Line 716: rust test `all_settings_actions_are_rust_side_safe` - Covers All settings actions are rust side safe.
- Line 749: rust test `c_dev_query_does_not_emit_fake_settings_result` - Covers C dev query does not emit fake settings result.
- Line 755: rust test `settings_provider_reports_ready_health` - Covers Settings provider reports ready health.
- Line 763: rust test `display_settings_matches_disp_set_abbreviation` - Covers Display settings matches disp set abbreviation.
- Line 773: rust test `alias_only_setting_match_gets_visible_highlight_fallback` - Covers Alias only setting match gets visible highlight fallback.
- Line 785: rust test `build_guard_excludes_rows_above_current_build` - Covers Build guard excludes rows above current build.
- Line 794: rust test `expanded_catalog_includes_common_windows_settings_intents` - Covers Expanded catalog includes common windows settings intents.
- Line 830: rust test `installed_apps_row_covers_add_or_remove_programs_and_appsfeatures_uri` - Covers Installed apps row covers add or remove programs and appsfeatures uri.
- Line 849: rust test `new_catalog_rows_rank_intended_settings_top_for_exact_and_alias_queries` - Covers New catalog rows rank intended settings top for exact and alias queries.
- Line 863: rust test `phase1_short_prefix_corpus_returns_expected_top_settings_without_flooding` - Covers Phase1 short prefix corpus returns expected top settings without flooding.
- Line 885: rust test `phase1_short_prefix_corpus_rejects_broad_or_pathlike_settings_noise` - Covers Phase1 short prefix corpus rejects broad or pathlike settings noise.
- Line 895: rust test `phase1_short_token_policy_is_static_row_ordered_prefix_not_exact_query_allowlist` - Covers Phase1 short token policy is static row ordered prefix not exact query allowlist.
- Line 912: rust test `phase1_one_character_policy_is_limited_to_canonical_windows_root_intent` - Covers Phase1 one character policy is limited to canonical windows root intent.
- Line 923: rust test `control_panel_subtasks_use_control_exe_with_safe_applet_args` - Covers Control panel subtasks use control exe with safe applet args.

### src-tauri/src/search/scoring.rs

Audits scoring behavior through 15 discovered test entries.

- Line 933: rust test `phase2_cap_recommendation_requires_mrr_gain_before_tie_breaks` - Covers Phase2 cap recommendation requires mrr gain before tie breaks.
- Line 946: rust test `phase2_provider_signal_experiment_preserves_invariants_and_recommends_cap` - Covers Phase2 provider signal experiment preserves invariants and recommends cap.
- Line 1009: rust test `phase2_provider_signal_cap50_acquires_source_priority_close_call` - Covers Phase2 provider signal cap50 acquires source priority close call.
- Line 1035: rust test `phase2_provider_signal_is_clamped_and_never_resurrects_nonmatches` - Covers Phase2 provider signal is clamped and never resurrects nonmatches.
- Line 1077: rust test `provider_type_boosts_do_not_surface_non_matches` - Covers Provider type boosts do not surface non matches.
- Line 1090: rust test `settings_intent_wins_settings_query` - Covers Settings intent wins settings query.
- Line 1115: rust test `windows_settings_intent_beats_incidental_everything_rows` - Covers Windows settings intent beats incidental everything rows.
- Line 1140: rust test `control_panel_intent_beats_incidental_everything_rows` - Covers Control panel intent beats incidental everything rows.
- Line 1179: rust test `app_intent_wins_app_query` - Covers App intent wins app query.
- Line 1204: rust test `folder_intent_wins_path_query` - Covers Folder intent wins path query.
- Line 1230: rust test `duplicate_paths_collapse_once_with_best_provider_row` - Covers Duplicate paths collapse once with best provider row.
- Line 1255: rust test `exact_open_window_hit_outranks_incidental_everything_folder` - Covers Exact open window hit outranks incidental everything folder.
- Line 1280: rust test `fuzzy_apps_and_settings_match_abbreviations_without_helping_everything` - Covers Fuzzy apps and settings match abbreviations without helping everything.
- Line 1316: rust test `token_prefix_fuzzy_keeps_exact_setting_above_random_file` - Covers Token prefix fuzzy keeps exact setting above random file.
- Line 1342: rust test `alias_only_app_match_gets_visible_highlight_fallback` - Covers Alias only app match gets visible highlight fallback.

### src-tauri/src/search_panel.rs

Audits search panel behavior through 10 discovered test entries.

- Line 294: rust test `anchors_panel_to_search_control_right_edge` - Covers Anchors panel to search control right edge.
- Line 299: rust test `clamps_panel_inside_host_edges` - Covers Clamps panel inside host edges.
- Line 305: rust test `centered_search_uses_larger_keyboard_launcher_size` - Covers Centered search uses larger keyboard launcher size.
- Line 313: rust test `centered_search_resize_clamps_to_supported_bounds` - Covers Centered search resize clamps to supported bounds.
- Line 343: rust test `stores_latest_payload_with_visible_results_for_panel_fetch` - Covers Stores latest payload with visible results for panel fetch.
- Line 368: rust test `rejects_stale_search_payload_sequences` - Covers Rejects stale search payload sequences.
- Line 396: rust test `rejects_same_sequence_for_different_query` - Covers Rejects same sequence for different query.
- Line 423: rust test `accepts_same_sequence_when_only_trailing_space_differs` - Covers Accepts same sequence when only trailing space differs.
- Line 461: rust test `rejects_phase_regression_for_same_query_and_sequence` - Covers Rejects phase regression for same query and sequence.
- Line 488: rust test `allows_complete_after_recoverable_provider_error` - Covers Allows complete after recoverable provider error.

### src-tauri/src/search_sources/apps.rs

Audits apps behavior through 2 discovered test entries.

- Line 230: rust test `finds_unpinned_start_menu_app_shortcut` - Covers Finds unpinned start menu app shortcut.
- Line 254: rust test `cached_app_index_freshness_is_bounded` - Covers Cached app index freshness is bounded.

### src-tauri/src/search_sources/everything.rs

Audits everything behavior through 8 discovered test entries.

- Line 519: rust test `parses_everything_highlight_markers_to_zero_based_indexes` - Covers Parses everything highlight markers to zero based indexes.
- Line 528: rust test `maps_file_folder_and_volume_results_to_system_results` - Covers Maps file folder and volume results to system results.
- Line 559: rust test `maps_installed_app_candidates_from_everything_as_apps` - Covers Maps installed app candidates from everything as apps.
- Line 575: rust test `request_limit_allows_larger_everything_result_sets` - Covers Request limit allows larger everything result sets.
- Line 582: rust test `provider_resets_sdk_after_successful_query` - Covers Provider resets sdk after successful query.
- Line 601: rust test `provider_allows_single_character_everything_queries` - Covers Provider allows single character everything queries.
- Line 620: rust test `health_accepts_portable_running_everything_with_repo_sdk` - Covers Health accepts portable running everything with repo sdk.
- Line 634: rust test `provider_resets_sdk_after_failed_query` - Covers Provider resets sdk after failed query.

### src-tauri/src/search_sources/everything_ffi.rs

Audits everything ffi behavior through 4 discovered test entries.

- Line 268: rust test `sdk_candidates_include_installed_everything_directory_without_recursive_scan` - Covers Sdk candidates include installed everything directory without recursive scan.
- Line 282: rust test `sdk_candidates_include_repo_local_everything_sdk_without_env_dependency` - Covers Sdk candidates include repo local everything sdk without env dependency.
- Line 291: rust test `maps_everything_sort_modes_to_sdk_constants` - Covers Maps everything sort modes to sdk constants.
- Line 299: rust test `sdk_access_lock_serializes_overlapping_queries` - Covers Sdk access lock serializes overlapping queries.

### src-tauri/src/search_sources/everything_install.rs

Audits everything install behavior through 4 discovered test entries.

- Line 291: rust test `missing_supply_chain_gate_blocks_setup_execution` - Covers Missing supply chain gate blocks setup execution.
- Line 305: rust test `no_consent_never_launches_everything` - Covers No consent never launches everything.
- Line 315: rust test `consent_requires_checksum_provenance_license_and_privacy_notice` - Covers Consent requires checksum provenance license and privacy notice.
- Line 344: rust test `official_download_open_does_not_require_artifact_metadata` - Covers Official download open does not require artifact metadata.

### src-tauri/src/search_sources/files.rs

Audits files behavior through 1 discovered test entries.

- Line 165: rust test `finds_matching_file_in_user_roots` - Covers Finds matching file in user roots.

### src-tauri/src/search_sources/index.rs

Audits index behavior through 4 discovered test entries.

- Line 227: rust test `cached_index_round_trips_file_and_app_results` - Covers Cached index round trips file and app results.
- Line 270: rust test `query_uses_cached_entries` - Covers Query uses cached entries.
- Line 292: rust test `provider_query_key_matches_cairo_style_normalization` - Covers Provider query key matches cairo style normalization.
- Line 300: rust test `refreshed_event_payload_reports_cache_generation` - Covers Refreshed event payload reports cache generation.

### src-tauri/src/search_sources/provider.rs

Audits provider behavior through 1 discovered test entries.

- Line 192: rust test `checked_at_uses_iso_utc_shape` - Covers Checked at uses iso utc shape.

### src-tauri/src/search_sources/query.rs

Audits query behavior through 2 discovered test entries.

- Line 62: rust test `parses_flow_style_action_keyword_and_terms` - Covers Parses flow style action keyword and terms.
- Line 75: rust test `parses_home_query_without_action_keyword` - Covers Parses home query without action keyword.

### src-tauri/src/search_sources/scoring.rs

Audits scoring behavior through 7 discovered test entries.

- Line 256: rust test `scores_spotify_shortcut_as_exact_match` - Covers Scores spotify shortcut as exact match.
- Line 267: rust test `ranks_cached_results_without_touching_filesystem` - Covers Ranks cached results without touching filesystem.
- Line 289: rust test `everything_provider_and_type_boosts_are_saturating` - Covers Everything provider and type boosts are saturating.
- Line 309: rust test `exact_app_intent_outranks_high_priority_folder_match` - Covers Exact app intent outranks high priority folder match.
- Line 345: rust test `fuzzy_app_token_matches_launcher_intent` - Covers Fuzzy app token matches launcher intent.
- Line 368: rust test `control_panel_query_matches_control_plane_command_alias` - Covers Control panel query matches control plane command alias.
- Line 404: rust test `bare_settings_query_outranks_exact_incidental_folder` - Covers Bare settings query outranks exact incidental folder.

### src-tauri/src/search_sources/windows_search.rs

Audits windows search behavior through 7 discovered test entries.

- Line 681: rust test `windows_search_limit_is_bounded_for_query_helper` - Covers Windows search limit is bounded for query helper.
- Line 688: rust test `windows_search_result_limit_is_bounded_for_row_retrieval` - Covers Windows search result limit is bounded for row retrieval.
- Line 695: rust test `windows_search_provider_requests_displayable_columns` - Covers Windows search provider requests displayable columns.
- Line 705: rust test `maps_windows_search_file_row_to_system_result` - Covers Maps windows search file row to system result.
- Line 725: rust test `maps_windows_search_folder_row_to_system_result` - Covers Maps windows search folder row to system result.
- Line 745: rust test `maps_windows_search_program_row_to_app_result` - Covers Maps windows search program row to app result.
- Line 769: rust test `maps_file_url_when_display_path_is_missing` - Covers Maps file url when display path is missing.

### src-tauri/src/settings.rs

Audits settings behavior through 17 discovered test entries.

- Line 951: rust test `loads_default_settings_when_file_is_missing` - Covers Loads default settings when file is missing.
- Line 961: rust test `saves_and_loads_versioned_settings` - Covers Saves and loads versioned settings.
- Line 972: rust test `migrates_unversioned_settings_to_v1_defaults` - Covers Migrates unversioned settings to v1 defaults.
- Line 996: rust test `default_settings_include_search_settings_without_bumping_v1` - Covers Default settings include search settings without bumping v1.
- Line 1021: rust test `clamps_search_result_limits_and_forces_content_search_off` - Covers Clamps search result limits and forces content search off.
- Line 1035: rust test `clamps_shell_bar_heights_while_locks_default_on` - Covers Clamps shell bar heights while locks default on.
- Line 1051: rust test `shell_bar_height_save_preserves_unrelated_settings` - Covers Shell bar height save preserves unrelated settings.
- Line 1079: rust test `shell_bar_lock_save_preserves_current_heights` - Covers Shell bar lock save preserves current heights.
- Line 1105: rust test `partial_nested_search_settings_default_missing_fields` - Covers Partial nested search settings default missing fields.
- Line 1130: rust test `backs_up_corrupt_settings_and_recovers_defaults` - Covers Backs up corrupt settings and recovers defaults.
- Line 1146: rust test `rejects_secret_like_settings_keys` - Covers Rejects secret like settings keys.
- Line 1155: rust test `allows_quick_command_transcript_secret_field` - Covers Allows quick command transcript secret field.
- Line 1183: rust test `validates_quick_command_entries_and_rejects_secret_like_args` - Covers Validates quick command entries and rejects secret like args.
- Line 1215: rust test `validates_quick_command_unique_ids_and_command_blocks` - Covers Validates quick command unique ids and command blocks.
- Line 1260: rust test `generic_settings_save_preserves_legacy_quick_command_order_and_history` - Covers Generic settings save preserves legacy quick command order and history.
- Line 1343: rust test `quick_command_specific_update_promotes_legacy_order_version` - Covers Quick command specific update promotes legacy order version.
- Line 1391: rust test `rejects_unknown_quick_command_order_version` - Covers Rejects unknown quick command order version.

### src-tauri/src/settings_panel.rs

Audits settings panel behavior through 2 discovered test entries.

- Line 87: rust test `anchors_settings_panel_to_button_left_edge` - Covers Anchors settings panel to button left edge.
- Line 92: rust test `clamps_settings_panel_inside_top_bar_edges` - Covers Clamps settings panel inside top bar edges.

### src-tauri/src/shell_paths.rs

Audits shell paths behavior through 13 discovered test entries.

- Line 426: rust test `control_panel_args_allow_applets_and_block_shell_metacharacters` - Covers Control panel args allow applets and block shell metacharacters.
- Line 437: rust test `vscode_resolver_uses_standard_candidate_order` - Covers Vscode resolver uses standard candidate order.
- Line 465: rust test `vscode_resolver_returns_none_when_missing` - Covers Vscode resolver returns none when missing.
- Line 470: rust test `shell_open_boundary_allows_existing_local_files_and_folders` - Covers Shell open boundary allows existing local files and folders.
- Line 490: rust test `shell_open_boundary_rejects_execution_and_protocol_inputs` - Covers Shell open boundary rejects execution and protocol inputs.
- Line 523: rust test `shell_open_boundary_allows_ms_settings_only_as_vetted_protocol` - Covers Shell open boundary allows ms settings only as vetted protocol.
- Line 533: rust test `open_with_picker_rejects_protocol_targets` - Covers Open with picker rejects protocol targets.
- Line 551: rust test `open_with_picker_rejects_missing_targets` - Covers Open with picker rejects missing targets.
- Line 560: rust test `open_with_picker_rejects_directories` - Covers Open with picker rejects directories.
- Line 573: rust test `open_with_picker_rejects_executable_or_script_targets` - Covers Open with picker rejects executable or script targets.
- Line 598: rust test `open_with_picker_accepts_existing_document_file` - Covers Open with picker accepts existing document file.
- Line 613: rust test `audited_app_launch_boundary_accepts_executables_for_stack_browser_activation` - Covers Audited app launch boundary accepts executables for stack browser activation.
- Line 635: rust test `audited_app_launch_boundary_allows_apps_without_weakening_generic_shell_open` - Covers Audited app launch boundary allows apps without weakening generic shell open.

### src-tauri/src/shell_windows.rs

Audits shell windows behavior through 3 discovered test entries.

- Line 570: rust test `desired_shell_ex_style_hides_from_alt_tab` - Covers Desired shell ex style hides from alt tab.
- Line 579: rust test `desired_shell_ex_style_preserves_unrelated_bits` - Covers Desired shell ex style preserves unrelated bits.
- Line 590: rust test `alt_tab_exclusion_rejects_task_switcher_bits` - Covers Alt tab exclusion rejects task switcher bits.

### src-tauri/src/stack_popup.rs

Audits stack popup behavior through 53 discovered test entries.

- Line 1747: rust test `stack_git_remote_url_validation_allows_only_safe_browser_urls` - Covers Stack git remote url validation allows only safe browser urls.
- Line 1765: rust test `rejects_invalid_rename_child_names` - Covers Rejects invalid rename child names.
- Line 1786: rust test `chooses_next_new_text_document_name_without_overwrite` - Covers Chooses next new text document name without overwrite.
- Line 1799: rust test `suggests_developer_open_with_apps_for_text_extensions` - Covers Suggests developer open with apps for text extensions.
- Line 1825: rust test `stack_file_drag_uses_native_drag_mechanism` - Covers Stack file drag uses native drag mechanism.
- Line 1834: rust test `reads_folder_details_with_folders_first` - Covers Reads folder details with folders first.
- Line 1849: rust test `serializes_stack_item_icon_data_url_for_frontend` - Covers Serializes stack item icon data url for frontend.
- Line 1871: rust test `deserializes_stack_item_without_icon_payload` - Covers Deserializes stack item without icon payload.
- Line 1891: rust test `normalizes_show_stack_popup_request_before_delivery` - Covers Normalizes show stack popup request before delivery.
- Line 1914: rust test `deserializes_legacy_show_stack_popup_request_without_request_id` - Covers Deserializes legacy show stack popup request without request id.
- Line 1926: rust test `paginates_large_stack_folders_without_truncating_metadata` - Covers Paginates large stack folders without truncating metadata.
- Line 1962: rust test `preserves_original_symlink_paths_when_materializing_page_items` - Covers Preserves original symlink paths when materializing page items.
- Line 2002: rust test `builds_partial_listing_warnings_with_optional_paths` - Covers Builds partial listing warnings with optional paths.
- Line 2013: rust test `suggests_stack_paths_with_directories_only_sorted_and_bounded` - Covers Suggests stack paths with directories only sorted and bounded.
- Line 2030: rust test `suggests_stack_paths_returns_structured_errors_for_invalid_parent` - Covers Suggests stack paths returns structured errors for invalid parent.
- Line 2045: rust test `chooses_copy_destination_when_name_exists` - Covers Chooses copy destination when name exists.
- Line 2056: rust test `rejects_copying_folder_into_itself_or_descendant` - Covers Rejects copying folder into itself or descendant.
- Line 2069: rust test `refuses_to_copy_symlink_directories_when_supported` - Covers Refuses to copy symlink directories when supported.
- Line 2092: rust test `clipboard_mode_debug_labels_remain_stable` - Covers Clipboard mode debug labels remain stable.
- Line 2098: rust test `maps_preferred_drop_effect_to_clipboard_mode` - Covers Maps preferred drop effect to clipboard mode.
- Line 2106: rust test `move_fallback_copies_then_deletes_source` - Covers Move fallback copies then deletes source.
- Line 2127: rust test `paste_result_preserves_successes_and_reports_failures` - Covers Paste result preserves successes and reports failures.
- Line 2153: rust test `resolves_supported_shell_aliases_for_pinning` - Covers Resolves supported shell aliases for pinning.
- Line 2175: rust test `maps_windows_file_attribute_bits` - Covers Maps windows file attribute bits.
- Line 2185: rust test `stack_item_reports_readonly_metadata` - Covers Stack item reports readonly metadata.
- Line 2204: rust test `reorders_pins_by_requested_paths_and_keeps_unspecified_tail` - Covers Reorders pins by requested paths and keeps unspecified tail.
- Line 2226: rust test `backs_up_corrupt_pin_store_file` - Covers Backs up corrupt pin store file.
- Line 2286: rust test `normalize_file_uri_paths` - Covers Normalize file uri paths.
- Line 2298: rust test `normalizes_file_uri_candidate_forms` - Covers Normalizes file uri candidate forms.
- Line 2314: rust test `strips_extended_windows_prefixes_from_stack_paths` - Covers Strips extended windows prefixes from stack paths.
- Line 2330: rust test `matches_stale_pins_by_raw_normalized_path` - Covers Matches stale pins by raw normalized path.
- Line 2346: rust test `creates_new_folder` - Covers Creates new folder.
- Line 2361: rust test `deletes_file_and_folder` - Covers Deletes file and folder.
- Line 2379: rust test `async_delete_deletes_nested_folder_and_preserves_missing_error` - Covers Async delete deletes nested folder and preserves missing error.
- Line 2400: rust test `archive_extraction_runner_preserves_process_error_contract` - Covers Archive extraction runner preserves process error contract.
- Line 2417: rust test `archive_extraction_timeout_is_clamped_and_source_avoids_raw_status` - Covers Archive extraction timeout is clamped and source avoids raw status.
- Line 2448: rust test `stack_long_running_file_op_commands_use_spawn_blocking_boundaries` - Covers Stack long running file op commands use spawn blocking boundaries.
- Line 2483: rust test `stack_phase1_matrix_commands_are_guarded_in_source` - Covers Stack phase1 matrix commands are guarded in source.
- Line 2505: rust test `stack_popup_setwindowpos_uses_noactivate_for_z_order_changes` - Covers Stack popup setwindowpos uses noactivate for z order changes.
- Line 2513: rust test `stack_icon_resolution_cache_reuses_cached_path_icons` - Covers Stack icon resolution cache reuses cached path icons.
- Line 2531: rust test `stack_icon_resolution_returns_none_for_missing_paths_without_failing_batch` - Covers Stack icon resolution returns none for missing paths without failing batch.
- Line 2544: rust test `stack_icon_resolution_batch_limit_stays_bounded` - Covers Stack icon resolution batch limit stays bounded.
- Line 2554: rust test `async_stack_icon_resolution_preserves_batch_contract` - Covers Async stack icon resolution preserves batch contract.
- Line 2571: rust test `archive_kind_from_path_accepts_zip_rar_files_only` - Covers Archive kind from path accepts zip rar files only.
- Line 2603: rust test `archive_7zip_candidates_include_program_files_and_path_fallback` - Covers Archive 7zip candidates include program files and path fallback.
- Line 2614: rust test `archive_extraction_plan_vectorizes_paths_with_spaces` - Covers Archive extraction plan vectorizes paths with spaces.
- Line 2643: rust test `archive_extraction_plan_keeps_builtin_and_7zip_zip_modes_distinct` - Covers Archive extraction plan keeps builtin and 7zip zip modes distinct.
- Line 2689: rust test `read_stack_folder_page_lists_zip_contents_as_stack_rows` - Covers Read stack folder page lists zip contents as stack rows.
- Line 2736: rust test `read_stack_folder_command_accepts_zip_paths` - Covers Read stack folder command accepts zip paths.
- Line 2763: rust test `stack_item_properties_plan_validates_existing_paths` - Covers Stack item properties plan validates existing paths.
- Line 2779: rust test `windows_explorer_reveal_select_arg_requests_new_window_and_keeps_select_path_together` - Covers Windows explorer reveal select arg requests new window and keeps select path together.
- Line 2789: rust test `windows_explorer_reveal_show_mode_maximizes_only_hidden_directories` - Covers Windows explorer reveal show mode maximizes only hidden directories.
- Line 2805: rust test `windows_explorer_reveal_launch_plan_preserves_fixed_executable_and_single_parameter` - Covers Windows explorer reveal launch plan preserves fixed executable and single parameter.

### src-tauri/src/stack_popup/git_status.rs

Audits git status behavior through 37 discovered test entries.

- Line 2086: rust test `commit_file_parser_handles_nul_status_paths_and_renames` - Covers Commit file parser handles nul status paths and renames.
- Line 2103: rust test `numstat_parser_handles_tab_records_binary_and_rename_nuls` - Covers Numstat parser handles tab records binary and rename nuls.
- Line 2116: rust test `untracked_line_counter_handles_trailing_newline_and_nonterminated_bytes` - Covers Untracked line counter handles trailing newline and nonterminated bytes.
- Line 2137: rust test `untracked_line_counter_rejects_symlink_outside_repo` - Covers Untracked line counter rejects symlink outside repo.
- Line 2161: rust test `untracked_line_counter_stops_after_budget` - Covers Untracked line counter stops after budget.
- Line 2178: rust test `porcelain_status_kinds_cover_stack_badges` - Covers Porcelain status kinds cover stack badges.
- Line 2197: rust test `git_remote_url_normalizer_handles_common_browser_remotes` - Covers Git remote url normalizer handles common browser remotes.
- Line 2231: rust test `git_branch_path_encoder_preserves_slashes_and_escapes_specials` - Covers Git branch path encoder preserves slashes and escapes specials.
- Line 2248: rust test `browser_remote_repository_url_uses_provider_routes_and_root_fallback` - Covers Browser remote repository url uses provider routes and root fallback.
- Line 2264: rust test `porcelain_parser_returns_counts_and_absolute_paths` - Covers Porcelain parser returns counts and absolute paths.
- Line 2294: rust test `porcelain_parser_tracks_independent_staged_and_unstaged_sides` - Covers Porcelain parser tracks independent staged and unstaged sides.
- Line 2332: rust test `porcelain_parser_keeps_rename_old_and_new_paths` - Covers Porcelain parser keeps rename old and new paths.
- Line 2352: rust test `branch_parser_marks_remote_refs_and_checkout_tracks_remote` - Covers Branch parser marks remote refs and checkout tracks remote.
- Line 2361: rust test `classify_git_run_mode_treats_restore_and_stash_as_local_mutation_and_diff_as_read` - Covers Classify git run mode treats restore and stash as local mutation and diff as read.
- Line 2374: rust test `git_pathspecs_reject_paths_outside_repo_and_use_nul_input` - Covers Git pathspecs reject paths outside repo and use nul input.
- Line 2406: rust test `git_stage_allows_selected_nested_changed_file_with_absolute_path` - Covers Git stage allows selected nested changed file with absolute path.
- Line 2447: rust test `git_stage_deleted_file_uses_cached_diff_and_stack_add` - Covers Git stage deleted file uses cached diff and stack add.
- Line 2535: rust test `commit_files_git_args_use_first_parent_and_root_mode` - Covers Commit files git args use first parent and root mode.
- Line 2565: rust test `commit_files_backend_includes_merge_commit_changes_against_first_parent` - Covers Commit files backend includes merge commit changes against first parent.
- Line 2829: rust test `missing_path_validation_rejects_namespace_and_dot_tricks` - Covers Missing path validation rejects namespace and dot tricks.
- Line 2838: rust test `missing_absolute_path_outside_repo_is_rejected` - Covers Missing absolute path outside repo is rejected.
- Line 2860: rust test `git_log_parser_uses_delimited_records` - Covers Git log parser uses delimited records.
- Line 2876: rust test `git_run_mode_classifies_remote_and_read_commands` - Covers Git run mode classifies remote and read commands.
- Line 2886: rust test `git_timeout_override_clamps_to_bounds` - Covers Git timeout override clamps to bounds.
- Line 2895: rust test `git_nonzero_classifier_maps_repository_and_auth_errors` - Covers Git nonzero classifier maps repository and auth errors.
- Line 2915: rust test `git_nonzero_classifier_keeps_labeled_stdout_and_stderr` - Covers Git nonzero classifier keeps labeled stdout and stderr.
- Line 2923: rust test `git_operation_output_includes_stdout_stderr_and_truncation_markers` - Covers Git operation output includes stdout stderr and truncation markers.
- Line 2947: rust test `trusted_git_path_requires_known_location` - Covers Trusted git path requires known location.
- Line 2953: rust test `git_tree_parser_handles_files_and_directories` - Covers Git tree parser handles files and directories.
- Line 2967: rust test `git_branch_parser_marks_current_and_remote` - Covers Git branch parser marks current and remote.
- Line 2979: rust test `git_branch_parser_skips_remote_head_symbolic_ref` - Covers Git branch parser skips remote head symbolic ref.
- Line 2990: rust test `worktree_parser_and_annotation_mark_only_other_local_worktrees` - Covers Worktree parser and annotation mark only other local worktrees.
- Line 3011: rust test `git_request_validation_rejects_option_injection_and_parent_paths` - Covers Git request validation rejects option injection and parent paths.
- Line 3029: rust test `branch_create_args_use_fixed_argv_with_optional_source` - Covers Branch create args use fixed argv with optional source.
- Line 3052: rust test `branch_delete_args_use_safe_fixed_argv` - Covers Branch delete args use safe fixed argv.
- Line 3101: rust test `git_stash_ref_validation_only_accepts_conservative_stack_refs` - Covers Git stash ref validation only accepts conservative stack refs.
- Line 3124: rust test `git_stash_list_parser_returns_typed_entries` - Covers Git stash list parser returns typed entries.

### src-tauri/src/stack_popup/open_with.rs

Audits open with behavior through 2 discovered test entries.

- Line 171: rust test `open_with_candidate_list_is_allowlist_only` - Covers Open with candidate list is allowlist only.
- Line 187: rust test `open_with_candidate_specs_are_selected_from_known_extensions` - Covers Open with candidate specs are selected from known extensions.

### src-tauri/src/stack_popup/paging.rs

Audits paging behavior through 21 discovered test entries.

- Line 735: rust test `stack_folder_page_diagnostics_defaults_to_zeroed_metrics` - Covers Stack folder page diagnostics defaults to zeroed metrics.
- Line 750: rust test `stack_folder_session_paging_returns_stable_unique_rows_across_pages` - Covers Stack folder session paging returns stable unique rows across pages.
- Line 790: rust test `stale_stack_folder_session_is_rejected_after_new_session_starts` - Covers Stale stack folder session is rejected after new session starts.
- Line 810: rust test `session_continuation_uses_stable_snapshot_without_rereading_directory` - Covers Session continuation uses stable snapshot without rereading directory.
- Line 836: rust test `mixed_large_folder_listing_completes_across_pages_without_duplicates_or_skips` - Covers Mixed large folder listing completes across pages without duplicates or skips.
- Line 876: rust test `continuation_diagnostics_keep_session_elapsed_time_for_metadata_completion_timing` - Covers Continuation diagnostics keep session elapsed time for metadata completion timing.
- Line 909: rust test `downloads_folder_page_one_contains_newest_entries_before_paging` - Covers Downloads folder page one contains newest entries before paging.
- Line 947: rust test `ordinary_folder_pages_remain_name_sorted` - Covers Ordinary folder pages remain name sorted.
- Line 979: rust test `downloads_folder_entries_sort_modified_desc_with_missing_times_last` - Covers Downloads folder entries sort modified desc with missing times last.
- Line 1011: rust test `read_stack_folder_page_clamps_extreme_page_limit_and_preserves_limit_zero_semantics` - Covers Read stack folder page clamps extreme page limit and preserves limit zero semantics.
- Line 1046: rust test `filesystem_listing_stops_at_entry_bound_with_warning` - Covers Filesystem listing stops at entry bound with warning.
- Line 1054: rust test `zip_listing_stops_at_raw_scan_bound_with_warning` - Covers Zip listing stops at raw scan bound with warning.
- Line 1080: rust test `zip_unique_children_stop_at_first_retained_cap_overflow` - Covers Zip unique children stop at first retained cap overflow.
- Line 1104: rust test `zip_entry_names_over_byte_bound_are_skipped_with_bounded_warning` - Covers Zip entry names over byte bound are skipped with bounded warning.
- Line 1119: rust test `continuation_clones_only_requested_page_slice` - Covers Continuation clones only requested page slice.
- Line 1135: rust test `session_store_evicts_by_total_entry_budget_before_count_cap` - Covers Session store evicts by total entry budget before count cap.
- Line 1162: rust test `session_store_evicts_by_estimated_byte_budget_before_count_cap` - Covers Session store evicts by estimated byte budget before count cap.
- Line 1184: rust test `oversized_single_session_is_retained_within_byte_budget` - Covers Oversized single session is retained within byte budget.
- Line 1201: rust test `offset_overflow_and_beyond_total_return_empty_without_has_more` - Covers Offset overflow and beyond total return empty without has more.
- Line 1222: rust test `session_path_mismatch_error_remains_unchanged` - Covers Session path mismatch error remains unchanged.
- Line 1242: rust test `active_by_path_tracks_oldest_tie_break_and_eviction` - Covers Active by path tracks oldest tie break and eviction.

### src-tauri/src/stack_popup/popup_window.rs

Audits popup window behavior through 11 discovered test entries.

- Line 561: rust test `stack_popup_size_clamps_to_sane_monitor_bounds` - Covers Stack popup size clamps to sane monitor bounds.
- Line 595: rust test `stack_popup_size_roundtrips_through_geometry_file` - Covers Stack popup size roundtrips through geometry file.
- Line 609: rust test `one_shot_focus_loss_suppression_is_consumed_without_focus_restore_hold` - Covers One shot focus loss suppression is consumed without focus restore hold.
- Line 622: rust test `cleared_one_shot_focus_loss_suppression_is_not_consumed_later` - Covers Cleared one shot focus loss suppression is not consumed later.
- Line 636: rust test `repeated_focus_loss_suppression_calls_do_not_stack` - Covers Repeated focus loss suppression calls do not stack.
- Line 651: rust test `expired_focus_loss_suppression_does_not_hide_future_focus_loss` - Covers Expired focus loss suppression does not hide future focus loss.
- Line 663: rust test `topmost_restore_suppression_ttl_is_longer_than_focus_loss_ttl` - Covers Topmost restore suppression ttl is longer than focus loss ttl.
- Line 673: rust test `active_topmost_restore_suppression_blocks_restore_without_consuming` - Covers Active topmost restore suppression blocks restore without consuming.
- Line 692: rust test `expired_topmost_restore_suppression_allows_restore_and_clears` - Covers Expired topmost restore suppression allows restore and clears.
- Line 712: rust test `cleared_topmost_restore_suppression_allows_restore` - Covers Cleared topmost restore suppression allows restore.
- Line 731: rust test `repeated_topmost_restore_suppression_calls_do_not_stack` - Covers Repeated topmost restore suppression calls do not stack.

### src-tauri/src/stack_popup/process_runner.rs

Audits process runner behavior through 6 discovered test entries.

- Line 466: rust test `caps_output_and_counts_total_bytes` - Covers Caps output and counts total bytes.
- Line 475: rust test `timeout_returns_error` - Covers Timeout returns error.
- Line 482: rust test `nonzero_exit_returns_metadata` - Covers Nonzero exit returns metadata.
- Line 505: rust test `spec_exposes_frozen_generic_runner_api` - Covers Spec exposes frozen generic runner api.
- Line 515: rust test `trusted_taskkill_lookup_is_not_path_based` - Covers Trusted taskkill lookup is not path based.
- Line 526: rust test `timeout_kill_tree_runs_before_direct_child_kill` - Covers Timeout kill tree runs before direct child kill.

### src-tauri/src/stack_popup/recovery_journal.rs

Audits recovery journal behavior through 6 discovered test entries.

- Line 353: rust test `journal_transitions_and_interrupts_stale_running` - Covers Journal transitions and interrupts stale running.
- Line 367: rust test `journal_schema_includes_required_fields` - Covers Journal schema includes required fields.
- Line 390: rust test `atomic_write_replaces_existing_file` - Covers Atomic write replaces existing file.
- Line 402: rust test `cleanup_skips_unknown_artifacts_and_respects_emergency_disable` - Covers Cleanup skips unknown artifacts and respects emergency disable.
- Line 432: rust test `unique_temp_path_stays_in_dir_and_uses_unique_name` - Covers Unique temp path stays in dir and uses unique name.
- Line 447: rust test `operation_id_is_uniqueish` - Covers Operation id is uniqueish.

### src-tauri/src/stack_popup/terminal.rs

Audits terminal behavior through 20 discovered test entries.

- Line 1437: rust test `terminal_profiles_have_fixed_process_plans` - Covers Terminal profiles have fixed process plans.
- Line 1491: rust test `powershell_startup_is_hidden_and_preserves_tab_completion_cycling` - Covers Powershell startup is hidden and preserves tab completion cycling.
- Line 1555: rust test `powershell_conpty_session_stays_running_after_start` - Covers Powershell conpty session stays running after start.
- Line 1585: rust test `cmd_conpty_spawn_smoke` - Covers Cmd conpty spawn smoke.
- Line 1607: rust test `cmd_launches_pwsh_in_conpty` - Covers Cmd launches pwsh in conpty.
- Line 1633: rust test `terminal_output_decoder_preserves_split_utf8_sequences` - Covers Terminal output decoder preserves split utf8 sequences.
- Line 1655: rust test `display_paths_strip_extended_windows_prefix` - Covers Display paths strip extended windows prefix.
- Line 1668: rust test `write_stack_terminal_session_authorizes_before_writer_clone` - Covers Write stack terminal session authorizes before writer clone.
- Line 1682: rust test `generated_terminal_session_ids_are_bounded` - Covers Generated terminal session ids are bounded.
- Line 1694: rust test `cd_commands_update_tracked_terminal_cwd` - Covers Cd commands update tracked terminal cwd.
- Line 1717: rust test `terminal_runtime_state_keeps_more_than_four_active_sessions` - Covers Terminal runtime state keeps more than four active sessions.
- Line 1727: rust test `terminal_stop_request_blocks_late_reinsert` - Covers Terminal stop request blocks late reinsert.
- Line 1737: rust test `terminal_registry_lists_and_renames_target_scoped_sessions` - Covers Terminal registry lists and renames target scoped sessions.
- Line 1767: rust test `terminal_registry_stops_only_requested_target_sessions` - Covers Terminal registry stops only requested target sessions.
- Line 1802: rust test `terminal_resize_request_rejects_invalid_session_and_bounds` - Covers Terminal resize request rejects invalid session and bounds.
- Line 1844: rust test `terminal_resize_test_session_records_requested_size` - Covers Terminal resize test session records requested size.
- Line 1864: rust test `terminal_registry_keeps_live_session_visible_during_operations` - Covers Terminal registry keeps live session visible during operations.
- Line 1877: rust test `terminal_session_target_defaults_to_caller_and_rejects_foreign_target` - Covers Terminal session target defaults to caller and rejects foreign target.
- Line 1898: rust test `terminal_session_target_auth_rejects_mismatched_caller` - Covers Terminal session target auth rejects mismatched caller.
- Line 1914: rust test `terminal_list_target_defaults_to_caller_and_rejects_other_target` - Covers Terminal list target defaults to caller and rejects other target.

### src-tauri/src/stack_popup/text_document/feasibility/contract.rs

Audits contract behavior through 12 discovered test entries.

- Line 1936: rust test `decimal_u64_is_string_only_and_lossless` - Covers Decimal u64 is string only and lossless.
- Line 1947: rust test `crlf_maps_to_one_local_unit_and_two_source_bytes` - Covers Crlf maps to one local unit and two source bytes.
- Line 2001: rust test `ledger_is_exact_once_and_never_reapplies_retired_ids` - Covers Ledger is exact once and never reapplies retired ids.
- Line 2036: rust test `selection_remap_preserves_affinity_and_marks_deleted_anchor` - Covers Selection remap preserves affinity and marks deleted anchor.
- Line 2125: rust test `lease_row_ceiling_is_global_across_segments_and_includes_base_row` - Covers Lease row ceiling is global across segments and includes base row.
- Line 2138: rust test `shared_wire_maps_and_forged_boundaries` - Covers Shared wire maps and forged boundaries.
- Line 2177: rust test `selection_affinity_and_overflow_are_checked` - Covers Selection affinity and overflow are checked.
- Line 2209: rust test `actual_edit_wire_deserializes_and_rejects_unknown_fields` - Covers Actual edit wire deserializes and rejects unknown fields.
- Line 2224: rust test `insertion_wire_rejects_unpaired_surrogates_and_opposite_variant_fields` - Covers Insertion wire rejects unpaired surrogates and opposite variant fields.
- Line 2255: rust test `close_and_reload_require_preceding_input_before_disposition` - Covers Close and reload require preceding input before disposition.
- Line 2311: rust test `payload_receipts_and_retirement_stay_bounded` - Covers Payload receipts and retirement stay bounded.
- Line 2360: rust test `authoritative_ownership_and_input_barrier_reject_stale_requests` - Covers Authoritative ownership and input barrier reject stale requests.

### src-tauri/src/stack_popup/text_document/feasibility/hash.rs

Audits hash behavior through 1 discovered test entries.

- Line 54: rust test `sha256_known_vectors_and_chunking` - Covers Sha256 known vectors and chunking.

### src-tauri/src/stack_popup/text_document/feasibility/tests.rs

Audits tests behavior through 21 discovered test entries.

- Line 86: rust test `t02_01_authoritative_identity_rejection_and_races` - Covers T02 01 authoritative identity rejection and races.
- Line 174: rust test `t02_01_identity_captures_reparse_metadata_and_rejects_named_streams` - Covers T02 01 identity captures reparse metadata and rejects named streams.
- Line 191: rust test `t02_04_invalid_prefix_is_retained_and_exposed_as_decision_required` - Covers T02 04 invalid prefix is retained and exposed as decision required.
- Line 219: rust test `t02_04_far_invalid_range_is_exposed_not_private_state` - Covers T02 04 far invalid range is exposed not private state.
- Line 247: rust test `t02_01_reparse_ancestor_and_final_refuse` - Covers T02 01 reparse ancestor and final refuse.
- Line 276: rust test `t02_02_ordinary_and_preexisting_mapped_writer_native` - Covers T02 02 ordinary and preexisting mapped writer native.
- Line 358: rust test `t02_02_prefix_edit_copy_handoff_and_cancellation` - Covers T02 02 prefix edit copy handoff and cancellation.
- Line 455: rust test `t02_03_paged_edits_history_eviction_and_quota_keep_dirty_root` - Covers T02 03 paged edits history eviction and quota keep dirty root.
- Line 508: rust test `t02_03_authenticated_reopen_and_tamper_refusal` - Covers T02 03 authenticated reopen and tamper refusal.
- Line 546: rust test `t02_04_every_split_codec_map_crlf_and_invalid_tail` - Covers T02 04 every split codec map crlf and invalid tail.
- Line 662: rust test `t02_04_disk_checkpoints_and_distant_invalid` - Covers T02 04 disk checkpoints and distant invalid.
- Line 693: rust test `t02_05_blocked_scan_urgent_priority_latest_and_cancellation` - Covers T02 05 blocked scan urgent priority latest and cancellation.
- Line 778: rust test `t02_05_priority_lanes_bound_background_work_and_keep_actor_lock_io_free` - Covers T02 05 priority lanes bound background work and keep actor lock io free.
- Line 959: rust test `t02_05_prefix_caret_race_preempts_background_and_keeps_latest_seek` - Covers T02 05 prefix caret race preempts background and keeps latest seek.
- Line 1150: rust test `t02_05_public_source_failure_states_are_distinct` - Covers T02 05 public source failure states are distinct.
- Line 1173: rust test `t02_large_populated_storage_experiment` - Covers T02 large populated storage experiment.
- Line 1310: rust test `t02_external_byte_oracle_cases` - Covers T02 external byte oracle cases.
- Line 1393: rust test `t02_resource_control` - Covers T02 resource control.
- Line 1401: rust test `t02_01_symlink_refusal_or_explicit_fixture_block` - Covers T02 01 symlink refusal or explicit fixture block.
- Line 1421: rust test `t02_02_copy_quota_failure_retains_protected_source` - Covers T02 02 copy quota failure retains protected source.
- Line 1453: rust test `t02_01_native_probe_canonical_root_opens_ordinary_source` - Covers T02 01 native probe canonical root opens ordinary source.

### src-tauri/src/stack_popup/text_document/protocol.rs

Audits protocol behavior through 12 discovered test entries.

- Line 969: rust test `shared_canonical_request_bytes` - Covers Shared canonical request bytes.
- Line 999: rust test `shared_lossless_integers` - Covers Shared lossless integers.
- Line 1009: rust test `shared_scalar_crlf_utf16_maps` - Covers Shared scalar crlf utf16 maps.
- Line 1030: rust test `newline_forgery_overflow_and_unknown_fields_reject` - Covers Newline forgery overflow and unknown fields reject.
- Line 1051: rust test `global_rows_and_projection_are_bounded` - Covers Global rows and projection are bounded.
- Line 1073: rust test `combined_cross_lease_selection_checks_ownership_revision_direction` - Covers Combined cross lease selection checks ownership revision direction.
- Line 1097: rust test `affinity_remapping_is_checked` - Covers Affinity remapping is checked.
- Line 1106: rust test `incomplete_context_never_certifies_grapheme_boundary` - Covers Incomplete context never certifies grapheme boundary.
- Line 1118: rust test `exact_once_retirement_and_barriers` - Covers Exact once retirement and barriers.
- Line 1145: rust test `camel_case_requests_reject_missing_barrier_and_extra_insertion_fields` - Covers Camel case requests reject missing barrier and extra insertion fields.
- Line 1156: rust test `shared_result_wire_vectors_and_explicit_nulls` - Covers Shared result wire vectors and explicit nulls.
- Line 1176: rust test `open_validation_and_selection_lifecycle` - Covers Open validation and selection lifecycle.

### src-tauri/src/system_power.rs

Audits system power behavior through 2 discovered test entries.

- Line 76: rust test `deserializes_only_known_power_actions` - Covers Deserializes only known power actions.
- Line 91: rust test `restart_and_shutdown_use_argument_vector_plans` - Covers Restart and shutdown use argument vector plans.

### src-tauri/src/system_tray.rs

Audits system tray behavior through 17 discovered test entries.

- Line 991: rust test `snapshot_id_round_trips_toolbar_and_command` - Covers Snapshot id round trips toolbar and command.
- Line 1004: rust test `invalid_snapshot_ids_are_rejected_before_relay` - Covers Invalid snapshot ids are rejected before relay.
- Line 1011: rust test `stale_tray_icon_invoke_returns_error_without_panel_close_payload` - Covers Stale tray icon invoke returns error without panel close payload.
- Line 1020: rust test `tray_discovery_collects_visible_and_overflow_toolbar_candidates` - Covers Tray discovery collects visible and overflow toolbar candidates.
- Line 1069: rust test `tray_discovery_does_not_drop_toolbar_source_for_synthetic_visibility` - Covers Tray discovery does not drop toolbar source for synthetic visibility.
- Line 1100: rust test `tray_discovery_includes_secondary_taskbar_sources` - Covers Tray discovery includes secondary taskbar sources.
- Line 1141: rust test `live_tray_snapshot_is_not_empty_when_explorer_exposes_toolbar_buttons` - Covers Live tray snapshot is not empty when explorer exposes toolbar buttons.
- Line 1157: rust test `tray_snapshot_ids_include_source_and_toolbar_identity` - Covers Tray snapshot ids include source and toolbar identity.
- Line 1178: rust test `tray_snapshot_merge_preserves_distinct_sources_with_similar_labels` - Covers Tray snapshot merge preserves distinct sources with similar labels.
- Line 1209: rust test `tray_snapshots_serialize_absent_native_icons_explicitly` - Covers Tray snapshots serialize absent native icons explicitly.
- Line 1228: rust test `tray_snapshots_serialize_present_native_icons_explicitly` - Covers Tray snapshots serialize present native icons explicitly.
- Line 1247: rust test `tray_label_resolution_prefers_real_explorer_text_and_sanitizes_it` - Covers Tray label resolution prefers real explorer text and sanitizes.
- Line 1260: rust test `tray_icon_payload_resolution_falls_back_when_native_icon_absent` - Covers Tray icon payload resolution falls back when native icon absent.
- Line 1272: rust test `tray_button_normalization_excludes_hidden_or_invalid_commands` - Covers Tray button normalization excludes hidden or invalid commands.
- Line 1294: rust test `tray_metadata_parser_extracts_bounded_icon_candidates` - Covers Tray metadata parser extracts bounded icon candidates.
- Line 1306: rust test `tray_metadata_read_size_covers_candidates_without_unbounded_reads` - Covers Tray metadata read size covers candidates without unbounded reads.
- Line 1318: rust test `tray_metadata_parser_rejects_empty_or_oversized_input` - Covers Tray metadata parser rejects empty or oversized input.

### src-tauri/src/task_gallery.rs

Audits task gallery behavior through 1 discovered test entries.

- Line 570: rust test `task_gallery_width_scales_from_tab_count_and_clamps_to_monitor` - Covers Task gallery width scales from tab count and clamps to monitor.

### src-tauri/src/task_preview.rs

Audits task preview behavior through 8 discovered test entries.

- Line 543: rust test `stale_preview_request_does_not_match_latest_request_id` - Covers Stale preview request does not match latest request id.
- Line 553: rust test `request_hide_request_sequence_rejects_stale_hover_and_hide_generations` - Covers Request hide request sequence rejects stale hover and hide generations.
- Line 568: rust test `clear_active_live_thumbnail_drops_stale_handle_from_state` - Covers Clear active live thumbnail drops stale handle from state.
- Line 578: rust test `shared_allocator_advances_past_latest_and_prior_allocations` - Covers Shared allocator advances past latest and prior allocations.
- Line 589: rust test `live_thumbnail_frame_scales_with_preview_window` - Covers Live thumbnail frame scales with preview window.
- Line 602: rust test `live_thumbnail_destination_preserves_source_aspect_ratio` - Covers Live thumbnail destination preserves source aspect ratio.
- Line 621: rust test `preview_position_anchors_to_host_and_clamps_to_screen` - Covers Preview position anchors to host and clamps to screen.
- Line 636: rust test `live_thumbnail_properties_make_destination_visible` - Covers Live thumbnail properties make destination visible.

### src-tauri/src/task_windows/actions.rs

Audits actions behavior through 6 discovered test entries.

- Line 520: rust test `task_window_identity_matches_requires_exact_pid_time_and_path` - Covers Task window identity matches requires exact pid time and path.
- Line 553: rust test `capture_identity_requires_matching_live_pid_only` - Covers Capture identity requires matching live pid only.
- Line 564: rust test `revalidation_requires_exact_captured_identity_match` - Covers Revalidation requires exact captured identity match.
- Line 584: rust test `access_denied_win32_code_matches_raw_and_hresult_from_win32_only` - Covers Access denied win32 code matches raw and hresult from win32 only.
- Line 592: rust test `access_denied_helper_recognizes_terminate_and_open_denied_forms` - Covers Access denied helper recognizes terminate and open denied forms.
- Line 599: rust test `elevate_gate_triggers_only_for_access_denied_forms` - Covers Elevate gate triggers only for access denied forms.

### src-tauri/src/task_windows/attention.rs

Audits attention behavior through 8 discovered test entries.

- Line 213: rust test `request_sets_requested_and_is_idempotent` - Covers Request sets requested and is idempotent.
- Line 222: rust test `clear_removes_record` - Covers Clear removes record.
- Line 231: rust test `reconcile_removes_stale_entries` - Covers Reconcile removes stale entries.
- Line 246: rust test `clear_only_matches_exact_identity` - Covers Clear only matches exact identity.
- Line 261: rust test `remove_root_owner_is_idempotent` - Covers Remove root owner is idempotent.
- Line 277: rust test `reconcile_keeps_none_creation_time_request_when_visible_snapshot_gains_creation_time` - Covers Reconcile keeps none creation time request when visible snapshot gains creation time.
- Line 296: rust test `unequal_known_creation_times_do_not_match` - Covers Unequal known creation times do not match.
- Line 308: rust test `clear_with_known_creation_time_clears_provisional_request` - Covers Clear with known creation time clears provisional request.

### src-tauri/src/task_windows/bounded_string_cache.rs

Audits bounded string cache behavior through 5 discovered test entries.

- Line 112: rust test `evicts_oldest_entry_at_capacity` - Covers Evicts oldest entry at capacity.
- Line 130: rust test `reinserting_key_updates_recency` - Covers Reinserting key updates recency.
- Line 149: rust test `caches_positive_and_negative_values` - Covers Caches positive and negative values.
- Line 162: rust test `expires_positive_entries_after_ttl` - Covers Expires positive entries after ttl.
- Line 172: rust test `expires_negative_entries_after_ttl` - Covers Expires negative entries after ttl.

### src-tauri/src/task_windows/diagnostics.rs

Audits diagnostics behavior through 2 discovered test entries.

- Line 470: rust test `redacts_complete_path_tokens` - Covers Redacts complete path tokens.
- Line 480: rust test `unresolved_count_is_independent_from_sample_capacity` - Covers Unresolved count is independent from sample capacity.

### src-tauri/src/task_windows/flash_fixture.rs

Audits flash fixture behavior through 2 discovered test entries.

- Line 388: rust test `parses_fixture_args_with_bounds` - Covers Parses fixture args with bounds.
- Line 408: rust test `rejects_duplicate_flags_marker_required_boundaries_and_bad_args` - Covers Rejects duplicate flags marker required boundaries and bad args.

### src-tauri/src/task_windows/helper.rs

Audits helper behavior through 6 discovered test entries.

- Line 285: rust test `canonical_path_roundtrip_handles_spaces_and_unicode` - Covers Canonical path roundtrip handles spaces and unicode.
- Line 298: rust test `helper_failure_codes_are_stable` - Covers Helper failure codes are stable.
- Line 304: rust test `access_denied_escalation_predicate_is_only_access_denied` - Covers Access denied escalation predicate is only access denied.
- Line 319: rust test `helper_exit_code_mapping_handles_uac_cancel_exactly` - Covers Helper exit code mapping handles uac cancel exactly.
- Line 339: rust test `helper_argv_decode_rejects_bad_descriptors` - Covers Helper argv decode rejects bad descriptors.
- Line 346: rust test `descriptor_validation_requires_exact_hwnd_pid_time_and_path_match` - Covers Descriptor validation requires exact hwnd pid time and path match.

### src-tauri/src/task_windows/icons.rs

Audits icons behavior through 3 discovered test entries.

- Line 350: rust test `source_helper_keeps_resolution_outside_lock` - Covers Source helper keeps resolution outside lock.
- Line 367: rust test `cache_key_changes_with_icon_identity` - Covers Cache key changes with icon identity.
- Line 381: rust test `cache_hit_skips_resolution_even_with_icon_identity` - Covers Cache hit skips resolution even with icon identity.

### src-tauri/src/task_windows/native_hooks.rs

Audits native hooks behavior through 6 discovered test entries.

- Line 654: rust test `native_attention_defaults_on_with_exact_zero_kill_switch` - Covers Native attention defaults on with exact zero kill switch.
- Line 674: rust test `json_line_formatting_stable` - Covers Json line formatting stable.
- Line 686: rust test `coalesce_hook_wake_timeout_blocks_duplicate_wakes` - Covers Coalesce hook wake timeout blocks duplicate wakes.
- Line 701: rust test `real_worker_stops_and_clears_native_resources` - Covers Real worker stops and clears native resources.
- Line 720: rust test `post_taskbar_flash_or_fallback_uses_fallback_exactly_once_on_failure` - Covers Post taskbar flash or fallback uses fallback exactly once on failure.
- Line 739: rust test `post_taskbar_flash_or_fallback_skips_fallback_on_success` - Covers Post taskbar flash or fallback skips fallback on success.

### src-tauri/src/task_windows/previews.rs

Audits previews behavior through 6 discovered test entries.

- Line 412: rust test `normalizes_gdi_bgra_pixels_for_png_output` - Covers Normalizes gdi bgra pixels for png output.
- Line 421: rust test `validates_non_empty_preview_bounds` - Covers Validates non empty preview bounds.
- Line 435: rust test `capture_rop_includes_layered_windows` - Covers Capture rop includes layered windows.
- Line 442: rust test `preview_bounds_prefers_sane_extended_frame` - Covers Preview bounds prefers sane extended frame.
- Line 463: rust test `preview_bounds_falls_back_when_extended_frame_is_empty` - Covers Preview bounds falls back when extended frame is empty.
- Line 484: rust test `preview_bounds_rejects_unsane_whole_virtual_desktop_bounds` - Covers Preview bounds rejects unsane whole virtual desktop bounds.

### src-tauri/src/task_windows/tests.rs

Audits tests behavior through 27 discovered test entries.

- Line 36: rust test `excludes_tool_windows` - Covers Excludes tool windows.
- Line 44: rust test `excludes_no_activate_windows` - Covers Excludes no activate windows.
- Line 52: rust test `excludes_shell_tray_windows` - Covers Excludes shell tray windows.
- Line 60: rust test `excludes_dwm_process_windows` - Covers Excludes dwm process windows.
- Line 69: rust test `excludes_dwm_notification_window_even_when_process_name_is_missing` - Covers Excludes dwm notification window even when process name is missing.
- Line 80: rust test `includes_minimized_windows_with_identity` - Covers Includes minimized windows with identity.
- Line 89: rust test `includes_non_primary_visible_windows_with_identity` - Covers Includes non primary visible windows with identity.
- Line 96: rust test `excludes_empty_title_helper_windows_without_explicit_taskbar_style` - Covers Excludes empty title helper windows without explicit taskbar style.
- Line 105: rust test `allows_empty_title_windows_that_force_taskbar_presence` - Covers Allows empty title windows that force taskbar presence.
- Line 115: rust test `sorts_windows_by_handle_for_stable_taskbar_order` - Covers Sorts windows by handle for stable taskbar order.
- Line 170: rust test `notification_count_tracks_per_app_identity_until_focus_reset` - Covers Notification count tracks per app identity until focus reset.
- Line 184: rust test `minimizes_only_from_live_foreground_state` - Covers Minimizes only from live foreground state.
- Line 191: rust test `does_not_minimize_already_minimized_window` - Covers Does not minimize already minimized window.
- Line 196: rust test `retries_close_with_post_message_when_window_remains` - Covers Retries close with post message when window remains.
- Line 203: rust test `uses_foreground_handoff_when_set_foreground_is_denied_for_non_minimized_windows` - Covers Uses foreground handoff when set foreground is denied for non minimized windows.
- Line 208: rust test `skips_foreground_handoff_when_set_foreground_succeeds_or_window_is_minimized` - Covers Skips foreground handoff when set foreground succeeds or window is minimized.
- Line 213: rust test `resolves_activation_target_to_visible_last_active_popup_or_root_owner` - Covers Resolves activation target to visible last active popup or root owner.
- Line 220: rust test `marks_window_busy_when_title_changes_between_refreshes` - Covers Marks window busy when title changes between refreshes.
- Line 240: rust test `marks_window_busy_when_process_cpu_advances_enough_between_refreshes` - Covers Marks window busy when process cpu advances enough between refreshes.
- Line 260: rust test `keeps_window_idle_without_previous_activity_delta` - Covers Keeps window idle without previous activity delta.
- Line 280: rust test `suppresses_generic_window_activity_indicator_even_when_title_changes` - Covers Suppresses generic window activity indicator even when title changes.
- Line 300: rust test `allows_llm_and_terminal_activity_indicators` - Covers Allows llm and terminal activity indicators.
- Line 313: rust test `suppresses_generic_windows_with_llm_text_in_title` - Covers Suppresses generic windows with llm text in title.
- Line 325: rust test `rejects_current_exe_as_internal_shell_close_target` - Covers Rejects current exe as internal shell close target.
- Line 331: rust test `allows_browser_activity_indicators_only_for_downloads` - Covers Allows browser activity indicators only for downloads.
- Line 348: rust test `attention_state_defaults_to_idle_and_stays_idempotent` - Covers Attention state defaults to idle and stays idempotent.
- Line 367: rust test `clear_taskbar_attention_if_matches_leaves_unrelated_identity_requested` - Covers Clear taskbar attention if matches leaves unrelated identity requested.

### src-tauri/src/task_windows/windows.rs

Audits windows behavior through 5 discovered test entries.

- Line 537: rust test `task_window_helper_arg_uses_plain_decimal_hwnd_value` - Covers Task window helper arg uses plain decimal hwnd value.
- Line 543: rust test `shell_execute_error_maps_uac_cancel_before_context` - Covers Shell execute error maps uac cancel before context.
- Line 552: rust test `shell_execute_error_maps_hresult_from_win32_uac_cancel` - Covers Shell execute error maps hresult from win32 uac cancel.
- Line 561: rust test `shell_execute_error_does_not_map_arbitrary_hresult` - Covers Shell execute error does not map arbitrary hresult.
- Line 570: rust test `native_and_manual_refresh_due_rules_hold` - Covers Native and manual refresh due rules hold.

### src-tauri/src/taskbar_menu.rs

Audits taskbar menu behavior through 3 discovered test entries.

- Line 476: rust test `parses_known_menu_prefix_payloads` - Covers Parses known menu prefix payloads.
- Line 492: rust test `rejects_wrong_prefix_payloads` - Covers Rejects wrong prefix payloads.
- Line 504: rust test `decodes_menu_payload_values` - Covers Decodes menu payload values.

### src-tauri/src/terminal_panel.rs

Audits terminal panel behavior through 3 discovered test entries.

- Line 112: rust test `anchors_terminal_panel_to_button_right_edge` - Covers Anchors terminal panel to button right edge.
- Line 120: rust test `clamps_terminal_panel_inside_top_bar_edges` - Covers Clamps terminal panel inside top bar edges.
- Line 126: rust test `bounds_terminal_panel_width_to_host_edges` - Covers Bounds terminal panel width to host edges.

### src-tauri/src/tray_panel.rs

Audits tray panel behavior through 4 discovered test entries.

- Line 125: rust test `anchors_tray_panel_to_button_right_edge` - Covers Anchors tray panel to button right edge.
- Line 130: rust test `clamps_tray_panel_inside_top_bar_edges` - Covers Clamps tray panel inside top bar edges.
- Line 136: rust test `tray_focus_loss_suppression_is_one_shot` - Covers Tray focus loss suppression is one shot.
- Line 145: rust test `expired_tray_focus_loss_suppression_does_not_hide_future_focus_loss` - Covers Expired tray focus loss suppression does not hide future focus loss.

### src-tauri/src/windows_key_hook.rs

Audits windows key hook behavior through 16 discovered test entries.

- Line 522: rust test `ctrl_space_toggles_search_and_suppresses_space` - Covers Ctrl space toggles search and suppresses space.
- Line 544: rust test `right_ctrl_space_toggles_search` - Covers Right ctrl space toggles search.
- Line 558: rust test `alt_backquote_toggles_terminal_and_suppresses_backquote` - Covers Alt backquote toggles terminal and suppresses backquote.
- Line 580: rust test `repeated_alt_backquote_does_not_duplicate_terminal_toggle` - Covers Repeated alt backquote does not duplicate terminal toggle.
- Line 598: rust test `alt_1_toggles_stack_browser_and_suppresses_repeat` - Covers Alt 1 toggles stack browser and suppresses repeat.
- Line 622: rust test `ctrl_alt_1_passes_through` - Covers Ctrl alt 1 passes through.
- Line 640: rust test `async_alt_state_toggles_when_alt_down_was_not_observed` - Covers Async alt state toggles when alt down was not observed.
- Line 654: rust test `released_alt_state_passes_through_stale_classifier_alt` - Covers Released alt state passes through stale classifier alt.
- Line 672: rust test `bare_space_and_other_ctrl_chords_pass_through` - Covers Bare space and other ctrl chords pass through.
- Line 694: rust test `repeated_space_down_does_not_duplicate_open_search` - Covers Repeated space down does not duplicate open search.
- Line 720: rust test `ctrl_release_resets_chord` - Covers Ctrl release resets chord.
- Line 750: rust test `async_control_state_opens_when_control_down_was_not_observed` - Covers Async control state opens when control down was not observed.
- Line 765: rust test `released_control_state_passes_through_stale_classifier_control` - Covers Released control state passes through stale classifier control.
- Line 780: rust test `unavailable_hook_state_passes_through` - Covers Unavailable hook state passes through.
- Line 800: rust test `lifecycle_install_once_and_uninstall_are_idempotent` - Covers Lifecycle install once and uninstall are idempotent.
- Line 812: rust test `emitted_event_targets_top_bar_existing_open_path` - Covers Emitted event targets top bar existing open path.

### src-tauri/src/workspaces.rs

Audits workspaces behavior through 6 discovered test entries.

- Line 710: rust test `validates_workspace_schema_paths_and_startup_metadata` - Covers Validates workspace schema paths and startup metadata.
- Line 718: rust test `rejects_relative_root_and_pin_paths` - Covers Rejects relative root and pin paths.
- Line 728: rust test `rejects_secret_like_workspace_env_names_and_values` - Covers Rejects secret like workspace env names and values.
- Line 751: rust test `rejects_secret_like_workspace_task_and_startup_args` - Covers Rejects secret like workspace task and startup args.
- Line 778: rust test `activation_plan_never_executes_startup_or_restores_windows` - Covers Activation plan never executes startup or restores windows.
- Line 792: rust test `activation_plan_uses_root_pin_when_workspace_has_no_pins` - Covers Activation plan uses root pin when workspace has no pins.

## Smoke/test scripts outside tests/

### scripts/runtime-smoke.ps1

Plan 13 runtime smoke harness.

- Script-level test/smoke entry - validates runtime behavior by executing documented smoke scenario or dry-run contract.

### scripts/smoke-fullscreen-appbar.ps1

Runs the live Windows fullscreen appbar smoke checklist for JasonShell.

- Script-level test/smoke entry - validates runtime behavior by executing documented smoke scenario or dry-run contract.

### scripts/smoke-taskbar-attention.ps1

Runs deterministic taskbar attention smoke checks against the built JasonShell exe.

- Script-level test/smoke entry - validates runtime behavior by executing documented smoke scenario or dry-run contract.

## Coverage check

- Node test files documented: 124.
- Rust source files with test functions documented: 77.
- Smoke/test scripts documented: 3.
- Individual discovered test/describe entries documented: 1620.
- Discovery basis: static scan of `tests/*.test.mjs`, Rust `#[test]` / `#[tokio::test]`, package scripts, and clearly named smoke/test scripts.