# Search scope browser acceptance

Current approved visual contract supersedes earlier 84px-trigger/0.7rem notes below: the combined lens-plus-picker trigger is **112px** (old lens18 + gap10 + picker84), its divider/query X stays fixed, and trigger/options use **0.8rem/600** with the configured UI family. The decorative lens center must hit-test to the trigger and open it. The real rounded popup shares the 112px capsule's left/right edges and must fit all labels plus selected checkmark without horizontal scrolling at 720/420/320px.

Shared focus contract: `restoreFocusOnClose` defaults true for existing consumers; Search opts out of the extra generic close restoration. Selection and Escape still restore the trigger, while Tab/outside focus remain natural. The trusted runner temporarily unmounts Search while mounting isolated default/opt-out shared controls, preventing Search's unrelated polling/autofocus lifecycle from contaminating the shared-default oracle; Search is remounted afterward. It checks default outside-dismissal restoration, opt-out outside-focus preservation, and opt-out Escape/selection restoration with actual trusted inputs.

The isolated outside input sits clear of both shared popups and is explicitly hit-tested before each click. Without that fixture spacing a click could select an overlaid option and falsely look like outside-dismissal focus theft. The trusted runner also clicks the lens center using real pointer coordinates, not only the trigger label. At .8rem/600, loaded Everything metrics are 65.72px Open Sans and 66.55px Google Sans; acceptance measures the actual label slot, popup/checkmark overflow, and unchanged divider X rather than assuming a native-arrow allowance.

Caret/hover regression: after setting All and moving the real pointer away, call `runPickerAppearance('idle',false)` with the panel region focused (not query). Then trusted CLI `hover "button[aria-label='Search scope']"` followed by `runPickerAppearance('hover',false)`. Repeat after clicking query with `runPickerAppearance('query-focused-hover',true)` and after opening with `runPickerAppearance('open',false)`. Expected no failures: caret visible-ink center within 1px of lens/All ink centers; transparent or achromatic trigger fill (not theme-accent tint); unchanged original 8% divider; actual query focus/header focus treatment retained. Geometry uses SVG `getBBox`/screen transforms when present, otherwise DOM text-range baseline plus loaded-font actual ink metrics, not just span boxes. This is a bounded geometry/font-metric oracle, not pixel-perfect raster certification. Keep the separate keyboard-focus/contrast gates.

For the current CSS `::before` vector mask, the oracle loads the actual computed mask image and rasterizes its SVG alpha at 32x, including the real stroke/caps/joins, with `contain` sizing. It places those alpha bounds using the actual centered flex geometry of the 8x5px pseudo-element inside the caret span and verifies the legacy Unicode font is zero-size. Unsupported pseudo layouts fail explicitly. This measures visible mask ink rather than the hidden glyph or just the 8x18px span; inspect the real-page screenshots independently as well.

Production `SearchPanelSurface.svelte` and global CSS, mounted with rejecting mocked IPC. No native shell calls or dependencies added.

```powershell
npx vite --config tests/browser/search-scope/vite.config.ts
agent-browser --session search-scope open http://127.0.0.1:4186
agent-browser --session search-scope wait --fn "window.searchScopeReady === true"
agent-browser --session search-scope eval "window.searchScopeFixture.run()"
```

Expected `failures: []`. Covers all scopes, unchanged query emission, original activation/selection identities, hidden-result safety, scope-trigger focus on refresh, type/clear and 720/420/320px geometry with long result titles. For trusted input smoke use snapshot/click/fill/press on this same mounted fixture. Browser evidence does not certify WebView2/native actions or live shell placement.

If 4186 is occupied, use `npx vite --config tests/browser/search-scope/vite.config.ts --port 4196` and open that port instead; do not stop unrelated servers.

The rounded-open-popup requirement explicitly supersedes the former native-select contract: use the established project `MeltSelect` button combobox / real HTML-popover listbox. Tests target its accessible trigger and controlled listbox, not a second menu implementation. Result-list queries are separately scoped to `#search-results` so popup options cannot masquerade as search results.

`window.searchScopeFixture.runDropdown()` covers ArrowDown opening, Home/End/arrows/typeahead, Enter selection without backend query, Escape popup-only dismissal/restoration, open-refresh focus, positive popup radius equal to the header and contained popup geometry at 720/420/320px. It uses programmatic events against a real popover. Separate trusted CLI smoke must open with click/Enter/Space/ArrowDown, choose options, verify Escape does not close search, Tab closes and advances to query, and outside click closes without stealing clicked-target focus. Capture `isTrusted` for the trusted journey. Existing `run()` retains identity/hidden-folder safety checks.

Run the trusted journey with `pwsh -NoProfile -File tests/browser/search-scope/run-trusted.ps1`; `-Url`, `-Session`, and `-OutputDirectory` are configurable. It records actual open/inert/focus states and trusted events, exits 1 on failures, and never invokes native commands. Synthetic setup/refresh is explicitly distinct from trusted CLI input. Popup geometry also rejects horizontal overflow with selected Everything and its checkmark; checking only label width or outer viewport containment misses the narrow-popup scrollbar regression.

Component harness limitation: jsdom 27 supplies `[popover]` UA `display:none`, but no `HTMLElement.showPopover`/`hidePopover`. Search tests provide a local, restored visibility-only shim so accessible option queries use the real open/closed state instead of `hidden:true`. Real browser inspection confirmed open popover `inert === false`, no inert attribute, `:popover-open === true`; the jsdom string `inert="false"` is not evidence of production inertness. The shim does not claim top-layer geometry/default-keyboard/focus-dismissal coverage; those remain browser-only.

## Focus-cascade regression

The fixture imports production `src/app.css`, including real theme selectors and global `:focus-visible` rules. `runFocus` waits for CSS transitions to settle and reports actual computed styles, not source-string expectations.

```powershell
agent-browser --session search-scope set viewport 320 620
agent-browser --session search-scope press Tab
agent-browser --session search-scope eval "window.searchScopeFixture.runFocus('vitesse-dark',320,true)"
agent-browser --session search-scope set viewport 720 620
agent-browser --session search-scope eval "window.searchScopeFixture.runFocus('base-dark',720,false)"
```

Expected `failures: []`: no query box-shadow/outline, a single positive header focus outline at most 1px, at least 14px from unchanged 84px scope divider to query text, visible trigger/clear keyboard focus, and nonoverlapping contained controls. Vitesse Dark with strong focus is an analogous warm/yellow reproduction, not a claim about the user's exact theme. Setup input events and focus calls inside `runFocus` are synthetic/programmatic; the initial CLI Tab establishes keyboard modality. Repeat with real CLI Tab/fill to corroborate trusted input if needed. No theme tokens or global focus rules are overridden by the fixture.

Focus-color regression also requires at least **3:1 composited sRGB contrast** against both panel and header flat fills. It resolves real computed colors, alpha-composites ancestor layers over black/white backing extremes (the fixture document is transparent), then composites the outline on each background before comparing relative luminance. Background images/gradients fail explicitly rather than producing a false pass. This is bounded Base Dark / Vitesse Dark visibility coverage, not whole-app WCAG certification, native translucency certification, or screenshot pixel sampling.

## Picker typography

`window.searchScopeFixture.runTypography()` compares the trigger and every popup option's computed font family with the actual body family. It applies the production Open Sans and Google Sans preferences with storage/broadcast disabled, waits for loaded fonts, measures Everything against the actual label slot inside the unchanged 84px picker, and checks 320/420px containment. Original root styles/preferences are restored afterward. Approved typography remains 0.7rem/600. This does not certify OS-native popup rasterization.

Baseline before typography production edits: both families already match, including all options, at 0.7rem/400. The query is separately styled 0.95rem/650, so visibly different size/weight alone is not evidence of a font-family mismatch. Global `app.css` and Audio Panel's existing native selects use `font: inherit`; the search follows that precedent. Keep computed-style evidence distinct from Windows native dropdown font selection.

Approved typography followup adds weight 600 while retaining 0.7rem on picker and options. Loaded-font measurement ruled out 0.72rem/600 at the unchanged 59px allowance: Everything measures 59.15px Open Sans / 59.89px Google Sans. At 0.7rem/600 it measures 57.50px / 58.23px. `runTypography()` asserts the chosen size, weight, family inheritance, loaded-font state and actual fit; pre-edit weight 400 must fail.
