# Broad-gate diagnosis after custom-theme acceptance

Baseline: `HEAD 43af99a6009bd2dee5b7f27d7b425832e307841b`.
Read-only `git show HEAD:<path>` compared with current source/tests after CRLF→LF
normalization. Every unrelated failing test and its decisive production source
was identical to HEAD. No dirty production work was reverted. Four safe test
maintenance repairs below preserve all behavior/security assertions.

## Original eleven Node failures

1. **`tests/changelogPolicy.test.mjs:104` — README validation scripts match package
   scripts without stale copies.** Assertion line 114 compares the literal
   `Exact package scripts` block in `README.md` against
   `Object.keys(packageJson.scripts)`. Missing now: `themes:generate`,
   `themes:check`, `precargo:check`, `precargo:test`, `pretauri`.
   **Mixed**: HEAD already omitted the three lifecycle scripts; this feature adds
   the two theme scripts. Test itself unchanged. Smallest repair: documentation
   owner updates that exact README block, in package.json insertion order, with
   every actual script. No inventory exclusions should be added to the test.

2. **`tests/contextMenuSpeechAcceptance.test.mjs:8` — shared app-owned menu provides
   rounded glass motion, icons, and disabled/destructive items.** Line 19 expects
   `/(?:backdrop-filter|backdropFilter)/` in
   `src/components/ContextMenu.svelte`; actual component has neither.
   **Baseline**, test and source HEAD-identical. Smallest source repair if the
   existing glass contract remains intended: restore the shared menu blur;
   coordinate with items 3–4 rather than deleting assertions.

3. **`tests/contextMenuVisualRegression.test.mjs:47` — shared menu uses a stable
   translucent theme backplate throughout opening animation.** Line 50 expects
   `color-mix(in srgb, var(--js-color-surface-overlay) 73%, transparent)`;
   actual shared menu background is `var(--js-color-surface)`.
   **Baseline**, decisive shared component and test HEAD-identical. Smallest
   source repair under the current test contract: restore that backplate without
   introducing an opacity animation or changing the separate bar-overlay rule.

4. **`tests/contextMenuVisualRegression.test.mjs:71` — shared menu uses the
   established 8px backdrop blur without vendor override.** Line 73 expects
   `backdrop-filter: blur(8px);`; actual shared panel has no backdrop-filter.
   **Baseline**, same HEAD-identical source/test as item 3. Same scoped production
   repair as item 2; no test disabling.

5. **`tests/frontendUiPolicy.test.mjs:30` — app and component UI sources do not use
   active gradients.** Line 39 fails first on `src/components/BottomBar.css`,
   whose `.bottom-bar.surface` and several controls use
   `background-image: linear-gradient(...)`.
   **Baseline**, that CSS and the policy test HEAD-identical. Baseline also
   explicitly requires a preview gradient in tracked
   `tests/taskPreviewBaseline.test.mjs:19`, so removing all gradients or weakening
   either contract blindly is not justified. Requires source/policy owner to
   reconcile the already-conflicting approved visual scope with behavior/visual
   evidence; custom-theme tests must not silently erase the conflict.

6. **`tests/meltMigrationWiring.test.mjs:199` — bottom-bar command buttons use
   Melt-backed action buttons without changing taskbar semantics.** Original
   line 235 expected direct `<span class="task-label">{taskWindowLabel(taskWindow)}</span>`.
   Actual HEAD/current production uses the same host and text through
   `<OverflowTicker text={taskWindowLabel(taskWindow)} />`.
   **Baseline stale syntax guard. Repaired test only**: assert the production
   OverflowTicker import plus same host and exact label expression. All adjacent
   Melt/taskbar/menu assertions retained. Existing rendered ticker coverage is
   `tests/components/overflow-ticker.test.ts`; full component gate passed.

7. **`tests/persistentTerminalPanel.test.mjs:227` — material symbol icon set keeps
   official 960-grid paths and equal inline sizing.** Line 228 expected name
   inventory omitted `crop` between `content_cut` and `content_paste`; actual
   `src/components/icons/materialSymbolIcons.ts` includes it.
   **Baseline**, source/test HEAD-identical, shipped snipping control documented
   in README. **Repaired expected inventory only** by adding `crop`; retained
   official-path and sizing assertions.

8. **`tests/sourceContractIntentRegistry.test.mjs:150` — source-text tests have
   explicit intent tags in the registry.** Expected no unregistered files;
   actual remaining file was `taskPreviewBaseline.test.mjs`.
   **Baseline**: `git ls-tree HEAD tests/taskPreviewBaseline.test.mjs` confirms
   tracked blob `b9c0747d9d4978e1105330cf21e868fdd4444b6d`; source is unchanged;
   HEAD registry omits it. **Repaired registry only**, adding the existing inset
   lens source contract under `architectureContract`. No test bypass added.

9. **`tests/speechIndicator.test.mjs:49` — mic shell stays a fixed opaque 40px
   circle in quiet and speaking states.** Lines 56–57 expect width/height 40px;
   `.mic-shell` in `src/components/SpeechIndicatorSurface.svelte` has 35px each.
   **Baseline**, source/test HEAD-identical. Requires speech UI owner to reconcile
   intended geometry; smallest production fix under the current spec/test is
   restoring 40px without changing meter/artwork behavior. Not changed in this
   test-only theme lane.

10. **`tests/taskbarAttentionModel.test.mjs:32` — nativeHook printing and flush are
    gated without gating signal processing or reconciliation.** Line 34 said
    `handle_event must remain present`. Function actually exists at
    `src-tauri/src/task_windows/native_hooks.rs:435`. Parser required `\n}\n`;
    Windows checkout has `\n}\r\n`. Proof: original regex false on raw source,
    true after CRLF→LF. **Baseline checkout/parser issue**, source/test otherwise
    HEAD-identical. **Repaired read helper only** to normalize CRLF. All printing,
    flushing, signal/reconciliation and opt-in assertions now execute unchanged.

11. **`tests/topBarMicControl.test.mjs:276` — mic hover matches adjacent control
    and successful paste check overlays visible mic glyph.** Line 281 compares
    hover background/border with neighboring terminal/command controls. Actual
    mic `border-color: transparent`; neighbor
    `border-color: var(--js-color-accent-border)` in `src/components/TopBar.css`.
    **Baseline**, decisive CSS/component/test HEAD-identical. Smallest source
    repair under this contract: match the mic hover border to its neighbors,
    preserving the independent green copied-glyph overlay. Needs UI owner, not
    arbitrary assertion removal.

## Six non-passing cases: three skips, three TODOs

Earlier shorthand “six skips” was inaccurate. TAP reports **3 skipped + 3 TODO**.
All following test files and annotations were checked HEAD-identical; none were
introduced, disabled or broadened by this feature.

- `tests/snippingNativeJourney.test.mjs:61`: **actual Tauri synthetic offscreen
  journey and independent saved PNG pixels** — opt-in native test skipped without
  its enable flag. No consent supplied; keep skipped.
- `tests/topBarMicControl.rendered.test.mjs:3`: **rendered mic control browser
  qualification** — existing skip: “Known limitation: agent-browser open times
  out on this host; native-driven behavior covered by focused compiled Node
  tests.” Theme CDP evidence does not certify this unrelated journey.
- `tests/windowsExecutableManifest.test.mjs:7`: **built Windows shell embeds its
  Common Controls v6 activation manifest** — existing skip without
  `JASONSHELL_TEST_EXE` pointing at a built PE artifact.
- `tests/searchOverhaulPhase0.test.mjs:279`: **phase 7 audit expectations are
  explicit and validated** — existing TODO: historical `search_overhaul.md`
  superseded by `search_upgrade_plan.md` and removed.
- Same file line 308: **phase 8 QA and performance expectations are testable from
  the overhaul plan** — existing TODO: historical QA checks superseded by active
  upgrade-plan validation.
- Same file line 499: **phase 3 everything provider has cached health, bounded
  simple-name request, and timings** — existing TODO: old literal check predates
  Phase 5 query modes; Rust Everything provider tests cover it.

## Rust missing frontend bundles: resolved safely

Original `npm run cargo:test` errors:

- `src-tauri/examples/stack_text_probe.rs:148`, `tauri::generate_context!`:
  `frontendDist` is `../../../dist-text-probe`, path does not exist.
- `src-tauri/examples/stack_text_p03.rs:130`, same macro:
  `frontendDist` is `../../../dist-p03-experiment`, path does not exist.

Both configs/scripts are HEAD-identical. `.gitignore` explicitly ignores both
output directories. Existing source-grounded build-only preparation was executed:

```powershell
npm exec vite build -- --config scripts/stack-text-editor/vite.config.mjs
npm exec vite build -- --config vite.p03-experiment.config.ts --outDir dist-p03-experiment --emptyOutDir
npm run cargo:test
```

The P03 Vite command matches the build step in
`scripts/stack-text-editor/launch-p03-experiment.ps1:15`. **Do not run that launcher
for this task**: its next command is `cargo run` of a native experiment and
requires a RunId. Only its harmless frontend build step was used.

Prepared `cargo:test` now passes: **1,152 passed, zero failed, 30 ignored across
26 test binaries**. `npm run cargo:check` also passed. Neither command launched
live shell smoke. Ignored native journeys remain ignored.

## Logs and current boundary

All logs are under `C:/Users/jnev1/AppData/Local/Temp/opencode/`:

- `custom-theme-final-node.log`: original exact 11 failures, 1,358 passes.
- `custom-theme-gate-tap.log`: exact skip/TODO names/reasons.
- `custom-theme-baseline-hygiene.log`: registry + CRLF parser repair, 12/12 pass.
- `custom-theme-baseline-inventories.log`: ticker + icon inventory repair, 25/25 pass.
- `custom-theme-final-node-after-baseline-repairs.log`: latest full Node rerun,
  **1,362 passed / 7 failed / 3 skipped / 3 TODO** (1,375 total).
- `custom-theme-cargo-test.log`: original missing frontendDist failures.
- `custom-theme-text-probe-build.log`, `custom-theme-p03-build.log`: prepared assets.
- `custom-theme-cargo-test-prepared.log`: all Rust tests pass after preparation.
- `custom-theme-cargo-check.log`: successful cargo check.

At that checkpoint the seven remaining failures were items 1–5, 9 and 11 above.
No contract was disabled, and this test-only lane did not silently change product
appearance. The explicit Leader-approved closure follows; the historical evidence
above describes the original failures, not current unresolved defects.

## Final closure after bounded Leader approval

The source owner restored generic menu 73%-overlay backplate/8px blur and the
documented 40×40 speech shell; reduced-transparency menus now retain opaque raised
RGB with no blur. Exactly five unapproved BottomBar gradient declarations and
the gallery tile gradient were removed. Approved outer-gallery, preview and
speech-artwork gradients remain. The docs owner repaired all actual README
script inventory entries.

Runner-owned policy now permits only the **exact declaration, property and
selector** for the three approved gradients—not entire files. Negative cases
prove unknown gradients in other selectors, inside approved selectors, and
altered approved values still fail. The mic test now checks the actual cascade:
equal hover backgrounds, mic 1px transparent base/transparent hover, neighboring
buttons inheriting the global zero-width border; copied-check assertions remain.

Rendered production/mocked-IPC evidence:

- Four menu samples (dark/light × normal/reduced): normal generic alpha<1 and
  blur8px; bar alpha1; reduced generic/bar alpha1 and no blur.
- Quiet/speaking shell 40×40, circular, artwork24×24 in both states.
- Bottom controls/task tiles and gallery tiles `background-image:none`;
  gallery180×136/radius2px/inset bevel unchanged; approved outer gradient retained.
- Original theme suite rerun: 16/16 journeys, all3,876 computed-style comparisons.

**Final `npm run validate` exit0**: check0 errors/0 warnings, build pass,
471 component tests pass, Node1,370 pass/0 fail/3 pre-existing skips/3 historical
TODOs (1,376 total), Rust1,152 pass/30 ignored across26 binaries, cargo check pass.
No new skips or test disabling. No live native smoke. The existing Chrome-process
restart evidence remains valid; neither palette engine nor persistence changed.

Final logs under approved Temp/opencode:

- `custom-theme-validate-final.log`: complete exit0 gate output.
- `custom-theme-baseline-targeted.log`: 65/65 focused baseline assertions.
- `custom-theme-baseline-render-report.json`: rendered metrics and screenshot paths.
- `custom-theme-baseline-render.log`: browser execution output.
- `theme-browser-final.log`: final original16-journey rerun.

All Rust commands are now finished. Generated tracked permission-file churn was
left untouched for Leader/git-specialist inspection after the final Cargo run.
