# Custom theme acceptance evidence

## Intake boundary

Acceptance criteria 1–15 are now supplied. Canonical storage is
`jasonshell.themes.v1`: version 1, string revision, selectedId and customThemes.
Custom records carry custom-prefixed ID, trimmed label (80 characters), builtin
base, mode, numeric revision, startingPalette and palette. Limits: 100 records,
512KiB. Future schemas remain untouched. Existing ID-only storage is migration
input only when canonical state is absent. No production edits belong here.

## Initial RED contract proposal

`themePalette.getPresetThemePalette(id: unknown): Record<string, string>` returns
a fresh, complete palette for an existing preset, with unknown IDs falling back
to Base Dark. Keys are the existing color/background/shadow/focus/scrollbar and
editor-selection tokens, not font/spacing/radius/motion tokens. Values retain
existing aliases. The CSS cascade is an independent initial oracle.

Production owner accepted this API. Existing `themes` exports stay compatible.
`themePersistence` and `themePreview` mutation/protocol contracts use their actual
exported APIs. Deterministic protocol tests control time and message delivery;
browser tests independently use real same-origin transport between pages.

## Acceptance mapping

1. All 34 preset clones: raw token contracts plus browser computed-paint equality.
2. Immutable builtins / independent custom copies: Node clone and revision cases.
3. Reset baseline: Node draft and editor reset, with presentation preferences on.
4. Valid edits synchronize; invalid edits keep last valid appearance: component
   input plus two-window browser assertions.
5. Hide/reopen retains draft/preview: mounted Settings and browser journey.
6. Cancel restores current saved selection in every window: browser journey.
7. Save collection+selection atomically; fresh pre-mount hydration: persistence
   failure contracts plus browser reload/new-page assertions.
8. Restart restores saved only: browser saved-state reload with unsaved preview.
9. Active deletion confirmation: Cancel byte-identical; confirm Base Dark in same
   canonical write and all live windows.
10. Presentation Reset: defaults, preview discarded, saved collection preserved.
11. Late join, stale packets, owner loss, disposal/resume: deterministic protocol
    tests plus browser late-window and unexpected-owner-close journey.
12. Quota/storage/revision conflicts: draft retained, accessible truthful failure,
    no applied success before durable write.
13. Strong focus and reduced transparency win over custom palette: browser paint.
14. Recovery controls remain readable with hostile palettes: browser contrast /
    independently resolved recovery colors; no claim of full a11y certification.
15. Terminal canvas/ANSI and all layout/control ordering unchanged: scoped source
    regressions plus representative browser geometry before/after theme edits.

## Evidence ownership and minimum gates

- Node palette contracts: every preset's full inherited token map, independent
  clones/default/reset; schema allows only editable tokens and safe values;
  alias references, unknown names, self/indirect cycles and invalid input.
- Node persistence contracts: schema migration, future-version preservation,
  malformed/oversized input, read and quota failure, safe limits, revision
  conflicts, active-custom deletion, and no persistence on preview/cancel.
- Node preview protocol: latest-session/revision ownership, stale or late packets,
  late joiners, owner disappearance, cancellation, disposal and listener cleanup.
- Production-mounted component tests: editor opening/cloning, color and text
  input, validation errors and recovery, name/save/reset/delete confirmation,
  dirty-exit cancel/discard, focus restoration and keyboard operation. Use nearby
  Settings/Control Plane test conventions; do not replace production UI with a
  test facsimile.
- Browser E2E (required): actual production components with mocked native IPC;
  real same-origin localStorage and BroadcastChannel in separate tabs/windows.
  Preview must propagate without persisting, cancel must restore, saved custom
  palettes must survive page/browser context restart, and deletion must converge
  across windows. Open a late window during preview; close the owner unexpectedly.
  Verify preferences (strong focus/reduced transparency) remain authoritative.
- Browser preset regression: compare resolved values of every palette token for
  all 34 presets, including aliases and representative painted properties. Keep
  raw-palette equality separate from computed-style equivalence.
- Final gates: focused Node/component/browser journeys, then `npm run validate`,
  followed by independent review. Never weaken old tests to accommodate defects.

## Limits

Mocked IPC browser tests establish renderer integration only. They do not certify
WebView2/native Tauri cross-window transport, native startup or live Windows shell
behavior. No live Tauri/AppBar/Windows smoke without explicit human consent.
No numeric coverage claim without measured instrumentation.

## Executed evidence

- Focused Node command: `node --test tests/settingsPanelWiring.test.mjs
  tests/themePalette.test.mjs tests/themePersistence.test.mjs
  tests/themePreview.test.mjs tests/themeRegistry.test.mjs
  tests/themeTerminalBoundary.test.mjs`: 105/105 passed.
- `npm run test:component -- tests/components/theme-editor.test.ts`: 14/14
  passed, including conflict-at-delete-confirm, ordinary Cancel focus/error
  association, management/duplicate independence and individual composite reset.
- Browser `run.mjs`: 16/16 renderer journeys passed; all 3,876 computed-style
  comparisons passed. Includes real keyboard focus containment/Escape return,
  320/360px reflow, cross-window picker decline/confirm, presentation Reset,
  canonical/all-key preview isolation, quota recovery and lost-end owner recovery.
- Browser `restart.mjs`: actual Chrome-process teardown/relaunch of an isolated
  profile passed; pre-mount saved-only hydration, no surviving draft and unchanged
  storage. This is not native-shell restart certification.
- Scoped built-in Node V8 coverage on compiled `themePalette`, `themePersistence`,
  `themePreview`, `themes`: 93.09% lines, 87.28% branches, 90.98% functions.
  Individual line coverage: 100%, 98%, 96.14%, 79.31%. No Svelte/browser/native
  coverage percentage is inferred from this measurement.
- Full `npm run validate` passed check (zero errors/warnings), build, and 471
  component tests, then stopped at Node. After updating the owned Settings source
  contract and registering owned source tests, final `npm run test:node` reported
  1,358 passed / 11 failed / 6 skipped (1,375 total). Remaining failures are
  changelog/script parity, unrelated context-menu/BottomBar/speech/taskbar/terminal
  source contracts, and the unregistered unrelated `taskPreviewBaseline.test.mjs`.
  They were not weakened or silently excluded.
- Separate `npm run cargo:check` passed. Initial `npm run cargo:test` failed
  compiling examples with missing frontendDist directories. Existing build-only
  Vite commands prepared ignored `dist-text-probe` and `dist-p03-experiment`;
  rerun passed **1,152 Rust tests, 30 ignored, 26 binaries**, no native smoke.
- Broad-gate baseline audit and four nonweakening test repairs are recorded in
  `custom-theme-broad-gates.md`. Latest full Node rerun: **1,362 passed / 7 failed /
  3 skipped / 3 TODO**. Earlier “six skipped” shorthand meant three skips plus
  three historical TODOs; all six annotations predate this feature.

Browser setup/commands and screenshot/report locations are documented in
`browser/custom-themes/README.md`. Detailed external logs live under the approved
`C:/Users/jnev1/AppData/Local/Temp/opencode/` directory, notably
`custom-theme-validate.log`, `custom-theme-final-node.log`,
`theme-engine-coverage.log`, `custom-theme-cargo-test.log`, and
`custom-theme-cargo-check.log`. At that checkpoint required broad gates remained
blocked outside the custom-theme acceptance lane; no native smoke was run.

### Final gate closure

Bounded Leader-approved baseline source/policy repairs resolved the preceding
gate failures; see `custom-theme-broad-gates.md` for historical diagnosis and
exact nonweakening repairs. Final `npm run validate` **exit0**:

- check0 errors/warnings, build passed;
- component471/471;
- Node1,370 passed/zero failed/3 existing skips/3 historical TODOs (1,376 total);
- Rust1,152 passed/30 ignored across26 binaries; cargo check passed.

Final original browser rerun passed16/16 and3,876/3,876 paint comparisons. Added
production-rendered baseline checks passed menu alpha/blur preference precedence,
speech40×40 quiet/speaking, and flat BottomBar/gallery tiles with preserved
gallery180×136 geometry/bevel and approved outer gradient. New test-only Svelte
fixture autofixer reported no issues. No native shell smoke was performed.
Latest artifacts: `custom-theme-validate-final.log`,
`custom-theme-baseline-render-report.json`, and `theme-browser-final.log` under
approved Temp/opencode. All Cargo work is finished; tracked generated permission
churn is untouched for the Leader's git-specialist cleanup.
