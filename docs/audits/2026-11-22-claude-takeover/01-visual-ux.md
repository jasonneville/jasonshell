## Visual & UX audit

Scope: static read-only review of `src/app.css`, every `src/components/*.css`, the component `<style>` blocks, and the markup of the main surfaces: TopBar, BottomBar, Search, Stack Browser, Git panel, the two terminals, Command panel, Settings, ThemeEditor, Process Manager/System Info, Task Preview/Gallery, Calendar, Audio, Tray, Quick Launch, Snip, notification banner, speech orb, and context menus. Nothing was rendered: no Tauri launch, no browser fixture. The contrast numbers below come from a scratch Node script. It parsed `src/lib/builtinThemePalettes.generated.ts`, composited each RGBA token over `--js-color-surface-raised`, which itself was composited over `--js-color-bg`, and computed WCAG 2.x ratios. Root font size is the browser default 16px, since nothing in `app.css` or `main.ts` changes it. So `0.62rem` means 9.9px.

Severity: **High** = users notice it or it breaks a11y or coherence across the shell. **Med** = clear polish or consistency debt. **Low** = nice to fix.
Effort: **S** < half a day, **M** 1–3 days, **L** a week or more, or cross-cutting.

### Quick-win shortlist (top 8)
1. VIS-01: the default `base-dark` palette doesn't hang together (accent-border is black, selected is invisible, the accent hue doesn't match accent-soft).
2. VIS-05: the type scale is microscopic (about 60 distinct font sizes; the most common is 9.9px).
3. VIS-03: muted/subtle text and accent borders fail WCAG in most built-in themes.
4. VIS-10: focus rings are drawn with `box-shadow`, which disappears in Windows forced-colors (High Contrast) mode.
5. VIS-14: both terminals, plus the Stack terminal, hard-code a navy palette that ignores all 34 themes.
6. VIS-08: there's no shared button, menu, or field primitive, so each surface hand-rolls them (36 raw `<button>`s in the Git panel, 20 in the terminal).
7. VIS-20b: the notification banner's dismiss target is 14px, the banner isn't announced to screen readers, and the timer can't be paused.
8. VIS-24: Stack Browser rows are `<button role="row">` with gridcells inside, which is invalid ARIA and leaves the grid unreadable to screen readers.

---

### A. Design tokens & theming

- **VIS-01 — The default theme (`base-dark`) palette doesn't hang together** (High impact, S effort).
  Evidence: `src/app.css:74-77`.
  - `--js-color-accent: #c0d8e7` is a pale grey-blue.
  - `--js-color-accent-soft: rgba(91,130,255,.18)` is a saturated royal blue.
  - `--js-color-accent-border: rgba(0,0,0,.44)` is translucent **black**.
  - `--js-color-selected: rgba(45,45,45,.2)` is near-invisible on a `#181e2b` surface.
  - The same values are copied into `src/lib/builtinThemePalettes.generated.ts` (the `"base-dark"` block).
  `--js-color-accent-border` is used 106 times across components for hover, selected, and drop-target borders, and it's the outline colour for some focus states (`BottomBar.css:173`, `SearchPanelSurface.css:171`, `StackPopupSurface.svelte:3286`). In the default theme all of those cues are effectively invisible: computed contrast of accent-border against surface is **1.13:1**. Hovers come out blue (accent-soft) while "accent" text and rings come out grey, so the shell looks like it has two accent colours.
  Recommendation: derive the whole accent family from one source hue in OKLCH. For example:
  - `--js-accent-h`
  - accent = `oklch(75% .12 h)`
  - accent-soft = accent at 16% alpha
  - accent-border = accent at 55% alpha
  - selected = accent at 14–18%
  Generate every theme's `accent-*` from its accent with `color-mix()` rather than hand-typed RGBA. Fluent 2 does this: one brand ramp, with stroke and fill tokens derived from it.

- **VIS-02 — Token names describe colours, not roles; there are no state or semantic layers** (Med, M).
  Evidence: `src/app.css:61-109`.
  - Tokens: `--js-color-success`, `-warning`, `-error`, `-info` are *fills* only. There's no `--js-color-success-text`, no `--js-color-warning-text`, and no `--js-color-danger`. `ContextMenu.svelte:82` references `var(--js-color-danger, #f08080)`, which is never defined, so it always falls back.
  - Aliases: `--js-bg-surface`, `--js-bg-bar`, `--js-bg-control`, and `--js-bg-active` duplicate `--js-color-surface-raised` and the others. Components mix both names: `TaskPreviewSurface.css` uses `--js-bg-surface`, while `SearchPanelSurface.css:66` uses `--js-bg-control`.
  - Mono font: `--js-font-mono` is referenced in `StackPopupSurface.css:455,700`, `SettingsPanelSurface.css:251`, and `ThemeColorField.svelte:96`, but never defined anywhere. Each call site supplies its own fallback stack.
  Recommendation: a three-tier system.
  - Primitives, e.g. `--js-blue-60`.
  - Semantic tokens: `--js-fill-subtle`, `--js-fill-subtle-hover`, `--js-fill-subtle-pressed`, `--js-stroke-focus`, `--js-fg-danger`, `--js-fg-on-accent`.
  - Component tokens: `--js-taskbar-tile-bg`.
  Fluent 2's `colorNeutralBackground1/1Hover/1Pressed` and `colorNeutralStroke1`/`colorStrokeFocus2` are a good naming template. Add `--js-font-mono` to `:root` and to the font preference system.

- **VIS-03 — Muted/subtle text and state borders fail WCAG in most themes** (High, M).
  Evidence: computed against `surface-raised` from `builtinThemePalettes.generated.ts`.
  - `--js-color-text-muted` is **below 4.5:1 in 14 of 34 themes**. Examples: `solarized-light` 3.09, `github-dark-dimmed` 3.30, `solarized-dark` 3.78, `github-light` 3.84, `one-dark-pro` 3.93, `github-dark` 4.01, `atom-one-dark` 4.34.
  - `--js-color-text-subtle` is **below 4.5:1 in all 34 themes**, ranging from 2.10 in `solarized-light` to 4.33 in `base-dark`. It's used for placeholders (`app.css:1259`) and disabled menu text (`StackPopupSurface.css:1461`).
  - `--js-color-accent-border` is **below 3:1 in all 34 themes** (1.13–3.06). That fails WCAG 1.4.11 non-text contrast wherever it's the only state indicator: selected tabs (`ProcessManagerSurface.css` tab `[aria-selected='true']`), hover borders, and drop targets.
  - Accent used as text is below 4.5:1 in `solarized-*` (3.5), `gruvbox-light` (3.52), `horizon` (3.75), and `atom-one-light` (4.04).
  This is worse because the type is tiny (VIS-05). At 9–10px, 4.5:1 is the floor, not the target.
  Recommendation:
  - Add a contrast gate to `scripts/generate-theme-palettes.mjs` that fails generation when muted < 4.5, subtle < 3.0 (and is only allowed for non-essential text), or accent-border < 3.0.
  - Auto-nudge lightness in OKLCH until each threshold passes.
  - `ThemeEditor.svelte:297` already has `contrastWarnings` for custom themes. Apply the same check to built-ins, and surface the computed ratios in the editor UI.

- **VIS-04 — Themes are hand-written in two places, and their overrides are inconsistent** (Med, M).
  Evidence:
  - `app.css:113-1121` holds 34 `:root[data-theme=…]` blocks (≈1000 lines), and `builtinThemePalettes.generated.ts` holds the same data again.
  - Early themes (`monokai` `app.css:~170`, `atom-one-dark`, `nord`, `dracula`…) override only bg, text, surface, and accent, and *inherit* base-dark's blue `control-hover rgba(138,167,255,.16)`, `scrollbar-thumb`, `info`, and `focus-ring rgba(160,190,255,.7)`. Monokai therefore gets blue hovers and blue focus rings next to a green accent.
  - Later themes (`night-owl` at `app.css:402` onward) define the full set.
  - `nord` alone sets `text-shadow: 0 0 .1px rgba(0,0,0,.3)` and re-declares font smoothing (`app.css:229-233`). That's a rendering hack leaking into a palette.
  Recommendation: make the generator emit the **full** token set for every theme from a small seed (bg, fg, accent, plus optional semantic hues). Delete the hand-written CSS blocks and have `themes.ts` apply generated values, or generate the CSS too. Remove the nord text-shadow.

- **VIS-05 — Type scale: about 60 distinct font sizes, centred on about 10px** (High, M).
  Evidence: a grep across components finds `0.62rem` ×42, `0.68rem` ×20, `0.72rem` ×18, `0.58rem` ×17, `0.54rem` ×8, `0.52rem` ×4, `0.5rem` ×3, `0.48rem` ×1, and raw `9px`, `10px`, `11px`, `13px` values.
  - Taskbar labels: `BottomBar.css:414` uses `0.62rem` (9.9px).
  - Group counts: `BottomBar.css:358` uses `0.54rem` (8.6px).
  - Tooltips: `MeltActionButton.svelte` style uses `0.62rem`.
  - The global `.surface-state` uses `0.72rem` (11.5px).
  For comparison, Windows 11 / Fluent 2 uses Caption 12px, Body 14px, Subtitle 20px. macOS uses 13px body with an 11px minimum. 8.6–10px is below every desktop OS minimum and gets worse at 100% scaling.
  Recommendation: replace the scale with about 6 tokens: `--js-text-caption: 12px`, `--js-text-body: 13px` (dense) or 14px, `--js-text-body-strong`, `--js-text-subtitle: 16px`, `--js-text-title: 20px`, `--js-text-display: 28px`. Each gets a paired line-height. Then do a codemod to map every `font-size: 0.5x–0.8xrem` onto them. Density (VIS-07) should change spacing, not shrink text below 12px.

- **VIS-06 — Font weights are chaotic, and Google Sans weights are faked** (Med, S).
  Evidence:
  - Weights in use: 800 ×47, 700 ×26, 750 ×20, 600 ×15, 760 ×6, 850 ×5, 650 ×5, 900 ×3, and one-offs 820, 799 (`BottomBar.css:121`), 780, and 640.
  - `app.css:19-44` maps the *same* static file `google-sans-latin.woff2` to weights 400, 500, and 700, and `:root` sets `font-synthesis: none` (`app.css:110`). When a user picks Google Sans in Settings (`shellPreferences.ts:55`), every "bold" label renders at regular weight.
  - Heavy 750–900 weights at 9–10px also fill in the counters of Open Sans and hurt legibility.
  Recommendation: use 3 weights only (400 regular, 600 semibold, 700 bold) as tokens `--js-weight-*`. Ship the real Google Sans variable font, or declare only 400. Fluent uses Regular and Semibold almost exclusively.

- **VIS-07 — The spacing and radius scales are odd and too small** (Med, M).
  Evidence:
  - Spacing: `app.css:92-95` defines `--js-space-1..4 = .18/.35/.55/.72rem` (2.9/5.6/8.8/11.5px). There's no 4px grid and no tokens above 12px, so components use raw `gap: 10px`, `padding: 4px 10px 4px 6px` (`SearchPanelSurface.css:66-72`), `gap: 3px` (`BottomBar.css:11`), and so on.
  - Radii: `app.css:88-91` defines `.18/.28/.42/.56rem` (2.9/4.5/6.7/9px). Raw values are also everywhere: `12px` and `8px` (ContextMenu), `8px` (TaskPreview/Gallery), `2px` (taskbar tiles, `BottomBar.css:67,113,424`), `4px` and `6px` (terminal), plus `0.55rem`, `0.4rem`, `0.35rem`, `14px`. That's 24 distinct radius declarations.
  - Compact density shrinks spacing from an already tiny base.
  Recommendation: a 4px grid: `--js-space-1:4px`, `2:8px`, `3:12px`, `4:16px`, `5:20px`, `6:24px`, `8:32px`. Radii matching Fluent 2: `--js-radius-sm:4px` (controls), `md:8px` (cards, flyouts, menus), `lg:12px` (top-level windows/panels), `full:999px`. Nested radius = outer − padding. Ban raw px radii in lint (stylelint `declaration-property-value-allowed-list`).

- **VIS-08 — No shared visual primitives; every surface restyles buttons, menus, and inputs** (High, L).
  Evidence:
  - `MeltActionButton.svelte` is only a `<button>` with a tooltip. It has no variant, size, or tone props, so every consumer restyles it.
  - `TopBar.css` has near-identical blocks for `.command-button`, `.terminal-button`, `.tray-button`, `.sound-button`, `.search-button`, and `.mic-button` (`TopBar.css:316-420, 586-603`), each with the same 9 declarations.
  - Raw `<button>` counts: `StackGitPanel.svelte` 36, `TerminalPanelSurface.svelte` 20, `StackPopupSurface.svelte` 8.
  - Context menus exist three times: `ContextMenu.svelte`, `.context-menu button` inside `StackPopupSurface.css:~1455`, and `.terminal-toolbar-menu` (`TerminalPanelSurface.css:199`).
  - Spinners exist four times: `mic-progress-spin` (TopBar.css:520), `command-spin` (CommandPanelSurface.css:125), `stack-git-spin` (StackGitPanel.svelte:2479), and `stack-operation-spin` (StackPopupSurface.css:933).
  - Close buttons exist at least four times: `AnchoredPanelCloseButton`, `.stack-browser-close-button`, `.preview-close-button`, and `TerminalSearchCloseButton`.
  - Native `<select>` is used in AudioPanel (2) and the Git panel (2) while Settings and Search use `MeltSelect`.
  Recommendation: build `src/components/ui/`.
  - `Button`: variant `subtle | standard | accent | danger`, size `sm | md`, plus `icon` mode.
  - `IconButton`, `Menu`/`MenuItem`, `TextField`, `Select`, `Toggle`, `Spinner`, `Badge`, `Kbd`, `Tooltip`, `PanelHeader`, `EmptyState`, `Skeleton`.
  - Each primitive owns hover, pressed, focus, disabled, and forced-colors styling once.
  Migrate surface by surface. This is the single biggest lever for "Linear-level" consistency.

- **VIS-09 — Elevation: one huge shadow everywhere, and none of it tuned for small native windows** (Med, S).
  Evidence:
  - `--js-shadow-raised: 0 18px 46px rgba(0,0,0,.42)` (`app.css:96`) is applied to the 23px-tall top bar (`TopBar.css:5`), the 32px bottom bar (`BottomBar.css:6`), tooltips, notification banners, snip preview, task previews, and context menus.
  - Each surface is its own transparent WebView window sized to its content, so a 46px blur is mostly clipped at the window edge and shows up as a hard-cut dark band, or not at all.
  - There's no elevation scale (2/4/8/16/28/64 in Fluent).
  Recommendation:
  - Tokens `--js-elev-2` (controls), `--js-elev-8` (tooltips, menus), `--js-elev-16` (flyouts), `--js-elev-28` (dialogs). Each is a two-layer shadow: a tight key shadow plus a soft ambient one.
  - For native-window surfaces, let DWM draw the shadow (`DWMWA_WINDOW_CORNER_PREFERENCE` plus a system shadow on Win11), or pad the window by the blur radius.
  - Bars should have no drop shadow, only a 1px hairline stroke, like the Win11 taskbar.

### B. Accessibility

- **VIS-10 — Focus indicators vanish in forced-colors (High Contrast) mode** (High, S).
  Evidence:
  - The global focus style is `:where(...):focus-visible { outline: 0; box-shadow: var(--js-focus-ring) }` (`app.css:1242-1245`). Many components also use `box-shadow: var(--js-focus-ring)` with `outline: none`: `ProcessManagerSurface.css` tabs, `TaskPreviewSurface.css` `.preview-surface:focus-visible`, `StackConfirmDialog.svelte`, `ContextMenu.svelte:80`, and others. There are 47 `outline: 0|none` declarations in components.
  - In forced-colors mode Chromium forces `box-shadow: none`, so every focus ring disappears.
  - The forced-colors block (`app.css:1304-1334`) only remaps tokens, which doesn't help. Only `AnchoredPanelCloseButton` and a TopBar drag state add forced-colors outlines.
  Recommendation: use `outline: 2px solid var(--js-stroke-focus); outline-offset: 2px` as the base focus style. `outline` survives forced colors and follows border-radius in Chromium. Keep box-shadow only as an extra halo. Add `@media (forced-colors: active) { :focus-visible { outline-color: Highlight } }`. Fluent uses a two-ring focus (2px dark inner, 1px light outer), which is a good model.

- **VIS-11 — The `prefers-contrast: more` branch is merged with forced-colors and uses the wrong system-colour pairs** (Med, S).
  Evidence: `app.css:1304` reads `@media (prefers-contrast: more), (forced-colors: active)`. It sets `--js-color-selected`, `--js-color-control-hover`, and `--js-color-accent-soft` to `Highlight` but leaves text as `CanvasText`. Selected and hovered rows therefore show CanvasText on Highlight, which can be unreadable. The correct pair is `HighlightText`. In `prefers-contrast: more` *without* forced colors, the whole branded palette is replaced by `Canvas`/`CanvasText`, which is heavier than needed.
  Recommendation: split the two media queries.
  - `prefers-contrast: more`: raise border alpha, set muted text = text, and thicken focus rings.
  - `forced-colors`: rely on system colours, add `--js-fg-on-selected: HighlightText`, and use `forced-color-adjust: none` only where branded glyphs need it.

- **VIS-12 — Hit targets are below WCAG 2.5.8 (24×24) and Fluent's 32px guidance** (High, M).
  Evidence:
  - Notification dismiss is 14×14 px (`QuickCommandNotificationSurface.svelte`, `--dismiss-diameter: 14px`).
  - Bar resize handles are 5px tall (`TopBar.css:26`, `BottomBar.css:21`), and they're focusable `MeltActionButton`s with no keyboard behaviour.
  - Stack search clear is 1.2rem × 1.35rem (19px; `StackPopupSurface.css:831`).
  - Search and Stack resize grips are 1.1–1.2rem (`SearchPanelSurface.css:402`, `StackPopupSurface.css:1471`).
  - The top bar's pin buttons and rail scroll arrows are 1.4rem (22px; `TopBar.css:145,212`).
  - The tray button is 1.45rem wide (`TopBar.css:375`).
  - Terminal tab close is `width: 1rem` (`TerminalPanelSurface.css:124`).
  - Quick-launch launcher buttons are `1.46rem` wide with a 0.86rem icon (`BottomBar.css:155,190`).
  Recommendation: 24px minimum for every interactive target, and 32px for primary bar controls. Where the visual must stay small, extend the hit area with a `::before` pseudo-element. BottomBar already does this for the bottom inset (`BottomBar.css:56-63`), so generalise it into the Button primitive. Give resize handles keyboard support (arrow keys with `aria-valuenow`, `role="separator"`, `aria-orientation`).

- **VIS-13 — Draggable and reorderable controls have no keyboard path** (Med, M).
  Evidence:
  - The TopBar control items reorder by pointer drag only (`TopBar.svelte` `.top-bar-control-item` with `on:pointerdown` and a11y svelte-ignore comments). Pinned folders likewise reorder only by pointer.
  - Task groups in BottomBar are pointer-drag only.
  - CommandPanel *does* have an order live region (`CommandPanelSurface.svelte:1023`), which is good, so the pattern exists but isn't shared.
  Recommendation: Alt+Shift+←/→ to move the focused item, announced via a shared `aria-live` "moved X to position 3 of 7" helper. Show a visible drag affordance on hover (a grip, or a lift with scale 1.02 and elevation).

- **VIS-14b — The terminals force `screenReaderMode: false`** (Med, S).
  Evidence: `TerminalPanelSurface.svelte:986` and `StackTerminalPane.svelte:241`. In addition, `.xterm-helper-textarea` is forced to `left:-10000px; width:0; height:0 !important` (`StackPopupSurface.css:718-733`, `StackTerminalPane.css:50-65`). xterm uses that textarea to position IME composition windows (CJK input) and for accessibility. Moving it off-screen can misplace the IME candidate window.
  Recommendation: follow the OS screen reader setting, or expose a setting. Stop overriding the helper textarea's position; hide it visually with `opacity:0` only and keep xterm's own placement.

- **VIS-15 — The Calendar grid has invalid ARIA, and day buttons lack full names** (Med, S).
  Evidence:
  - `CalendarPanelSurface.svelte:93-112` has `role="grid"` with direct `columnheader` children and `<button>` days, and no `role="row"` wrappers.
  - Day buttons are labelled only by the number ("14").
  - Navigation uses text glyphs `«‹›»` (lines 86-90).
  - The container is `tabindex="0"` with a wheel handler, which hijacks scrolling.
  Recommendation: use `role="grid"` → `row` → `gridcell`, each containing a button with `aria-label="Friday, November 14, 2026"`, and roving tabindex with arrow, PageUp/PageDown, and Home/End keys (the WAI-ARIA date-picker pattern). Use real chevron icons.

- **VIS-16 — Sortable column headers lose their button semantics** (Low, S).
  Evidence: `ProcessManagerSurface.svelte:487-508` and `StackPopupSurface.svelte:3544-3547` pass `role="columnheader"` to `MeltActionButton`, which renders a `<button role="columnheader">`. The role override strips the button role, so screen readers don't announce it as activatable. Sort state is also shown with text glyphs via `sortIndicator()`.
  Recommendation: `<div role="columnheader" aria-sort>` wrapping a real `<button>`, with a sort chevron icon that rotates over 150ms.

- **VIS-17 — Selection and active state shown only by colour or border** (Med, S).
  Evidence:
  - Terminal active tab: `TerminalPanelSurface.css:68-72` changes only the border colour.
  - Taskbar active window: `BottomBar.css:432-437` uses a background tint plus accent-border, which is 1.13:1 in the default theme.
  - Task buttons don't expose `aria-pressed` or `aria-current` for the active window (`BottomBar.svelte` task-button markup).
  - Git status uses a colour-only 5px inset bar (`StackPopupSurface.css:1261-1277`).
  Recommendation: give active items a shape cue. Win11 uses a 3px pill indicator under active taskbar apps (short for open, long for active) and a 3px accent bar for selected list items. Add `aria-current="true"` (or `aria-pressed`) on the active task button.

- **VIS-18 — Icons announce internal ids, and some decorative glyphs are text** (Low, S).
  Evidence:
  - `MaterialSymbolIcon.svelte` sets `aria-label={name}` when `decorative=false`, so a screen reader would hear "monitor_heart" or "code_blocks".
  - Text glyphs stand in for icons: tray `▾` (`TopBar.svelte:2287`), rail scroll `&lsaquo;`/`&rsaquo;` (`TopBar.svelte:2198,2247`), git refresh `↻` (`StackGitPanel.svelte:1668`), remote `↗` (`StackPopupSurface.svelte:3411`), terminal overflow `⋯` (`TerminalPanelSurface.svelte:2583`), mic copied `✓` (`TopBarMicControl.svelte:160`), search clear `×`, and calendar `«‹›»`. Their baselines and weights depend on the active UI font, and Google Sans Code or Cascadia UI will shift them.
  Recommendation: require a `label` prop for non-decorative icons. Add `expand_more`, `chevron_left/right`, `refresh`, `open_in_new`, `more_horiz`, `check`, `close`, and `first_page/last_page` to `materialSymbolIcons.ts`, and ban text glyphs as icons.

### C. Surface-by-surface

#### Top bar
- **VIS-19 — The top bar has inconsistent control heights and redundant hover rules** (Med, S).
  Evidence:
  - The bar's default logical height is 23.4px (`TopBar.css:12`), but the icon buttons are `height:1.7rem` = 27px (`TopBar.css:326,339,374,386,400,592`). Pins and the home button are 1.4rem (`TopBar.css:62,145,212`).
  - Every hover sets `border-color: var(--js-color-accent-border)` on buttons that have `border: none`, which does nothing (for example `TopBar.css:46-49, 66-69, 216-219`).
  - None of the top bar buttons have an `:active` (pressed) state.
  - `.time-pill span` styles (`TopBar.css:256-260`) and the whole `.top-bar-identity` block (`TopBar.css:72-103`) are dead: the markup renders `<strong>{shellTime} {shellDate}</strong>` only, and has no identity element.
  Recommendation:
  - Define `--js-bar-control-size` from the bar height (bar − 2×4px) so controls scale when users resize the bar.
  - One `.bar-button` class with `:hover` (fill-subtle-hover), `:active` (fill-subtle-pressed, plus `scale(.96)` over 80ms), `[aria-expanded=true]` (fill-subtle-selected plus a 2px accent underline, like the active flyout button in the Win11 tray), and `:focus-visible`.
  - Delete the dead CSS.

- **VIS-20 — Top-bar status icons are static** (Med, M).
  Evidence: `TopBar.svelte` renders the sound control as `<MaterialSymbolIcon name="speaker" />` regardless of volume or mute. The tray is a bare `▾`, and the clock shows time and date in a single bold line.
  Recommendation: reflect live state, the way the macOS menu bar and Win11 quick settings do.
  - Sound: volume_off / volume_mute / volume_down / volume_up glyphs by level. Scroll-wheel over the icon changes volume and shows a transient value bubble.
  - Clock: two-line stacked time and date when the bar is tall enough, single line when short.
  - Add a notification count badge.
  - Show network and battery status if those providers exist.

- **VIS-21 — Pinned-folder rail feedback is functional but plain** (Low, S).
  Evidence: `TopBar.css:110-200`. Overflow uses `‹ ›` scroll buttons and the drop state is a flat accent-soft overlay with the text "Drop to pin folder". Pins render as text-only chips (`min-width:3.5rem; max-width:9.5rem`) with no folder icon.
  Recommendation:
  - Add folder icons from `stackIconCache`, or a coloured dot per pin.
  - Fade the scroll edges with a mask instead of buttons (`mask-image: linear-gradient(90deg, transparent, #000 16px, #000 calc(100% - 16px), transparent)`).
  - When dragging, show an insertion caret between chips rather than a full overlay.

#### Bottom bar (taskbar)
- **VIS-22 — Taskbar tiles feel dated: 2px boxes, 12px icons, 10px text** (High, M).
  Evidence:
  - Task buttons have `border-radius: 2px`, a full 1px border, an inset highlight, `font-size: .62rem`, and `font-weight: 600` (`BottomBar.css:415-440`).
  - The icon is `.74rem` = 11.8px (`BottomBar.css:458-464`). Launcher icons are `.86rem` = 13.8px (`BottomBar.css:188-193`).
  - "Quick Launch" is a literal text button (`BottomBar.svelte` quick-launch button) rather than a logo or Start affordance.
  - Hard-coded colours bypass the theme: `#ffd54f` attention (`BottomBar.css:279,450`) and `rgba(147,202,249,.96)` busy bar (`BottomBar.css:306`).
  - Overflow is shown by covering the end with a *solid* `--js-color-surface` block (`BottomBar.css:217-226`) rather than a fade.
  Recommendation: a modern tile.
  - 16px icon at the default bar height (24px when the bar is ≥ 40px), label at 12px, radius 4px.
  - Borderless fill-subtle background, with fill on hover and pressed.
  - A 3px rounded indicator pill at the bottom: 6px wide when open, 16px wide in accent when active, warning colour when flashing.
  - Overflow fade with `mask-image`.
  - Replace `#ffd54f` with `--js-color-warning-fg`.
  - Option for "icons only, combine" (Win11) vs "labels" (Win10/KDE).

- **VIS-23 — Window previews and the gallery are good structurally but lack motion and hierarchy** (Med, M).
  Evidence:
  - `TaskPreviewSurface.css:7-28` uses an accent-tinted gradient chrome, `border-radius: 8px`, and `padding: 10px`.
  - The gallery header uses `font-size: 11px`/`10px` px literals (`TaskGallerySurface.css:24-40`).
  - The close button is a 25px-tall text box with a `min-width: 2.1rem` (`TaskPreviewSurface.css:126-145`).
  - Previews just appear, with no enter animation or hover-to-highlight on the actual window.
  Recommendation:
  - Fade + 4px rise on enter (120ms, `cubic-bezier(0,0,0,1)`).
  - Crossfade when moving between adjacent task previews instead of hiding and re-showing.
  - Hovering a preview should trigger Aero-Peek-style dimming of the other windows (needs the native side).
  - Make the close button a 24px icon button that only appears on hover or focus.
  - Use the radius and type tokens.

#### Stack Browser (file popup)
- **VIS-24 — Grid rows are `<button role="row">` containing gridcells** (High, M).
  Evidence:
  - `StackPopupSurface.svelte:3580-3620`: `type="button" role="row" aria-selected … <span role="gridcell">`.
  - A `<button>`'s children are presentational, so assistive tech flattens the cells.
  - The `role="grid"` container (`:3532`) has no `aria-multiselectable` even though marquee and multi-select exist.
  - `disabled={hasRetainedRows}` removes rows from the tab and AT tree while retained.
  Recommendation: `div role="row" tabindex=-1` with roving focus managed by the grid, `aria-multiselectable="true"`, `aria-selected` per row, and gridcells as children. Keep the click and drag handlers on the row div.

- **VIS-25 — File-type icons are CSS-drawn 16px glyphs with fixed colours** (Med, M).
  Evidence: `StackPopupSurface.css:1040-1175`. Folder, file, app, image, archive, and code icons are built from `::before`/`::after` boxes with literal colours (`#ffd67a`, `#d79932`, `#4f8dff`, `#6bd8bd`, `#ac75df`, `#83a3ff`, and others) and `rgba(255,255,255,.12)` borders. They ignore light themes and look crude next to the real shell icons used elsewhere (`stackIconCache.ts`, `iconDataUrl` in search and taskbar).
  Recommendation: use real Windows shell icons (SHGetFileInfo, which the code already caches) at 16 and 20px, with an SVG icon set as fallback (Fluent UI System Icons or Material Symbols, already partly vendored). Tint fallbacks with theme tokens.

- **VIS-26 — Breadcrumbs and path bar** (Med, S).
  Evidence:
  - The separator is `/` on Windows paths (`StackPopupSurface.svelte` `.crumb-sep`).
  - The collapsed segments `...` is a non-interactive `<span>` with a `title` (`:3392-3393`), so hidden ancestors can't be reached.
  - Inline-completion ghost text duplicates input styling with hard-coded `.78rem`/700 (`StackPopupSurface.css:90-118`).
  Recommendation: chevron separators, which also act as dropdown buttons listing sibling folders (Explorer pattern). Make `…` a menu button listing the hidden segments. Click on empty path space to switch to edit mode, as Explorer does.

- **VIS-27 — Stack popup hard-codes dark colours that break light themes** (Med, S).
  Evidence:
  - Terminal mode uses `#06080b`, `#d7e2f5`, `#aebbd0`, and `#4e8cc8` (`StackPopupSurface.css:686-757`).
  - Git status colours are `#76ad4f`, `#c67f13`, `#da5b4a`, `#479fe6` (`:1261-1357`). On `github-light`, `#c67f13` on white is about 3:1 and fails as text.
  - `StackConfirmDialog.svelte` mixes `#05080e`/`#101827` into the backdrop and surface.
  - `ThemeEditor.svelte:434` `.theme-contrast` uses `background:#222f42`.
  Recommendation: add `--js-git-added|modified|deleted|untracked|conflict` semantic tokens per theme (with light and dark variants), plus a terminal palette (VIS-14). Use `color-mix(... var(--js-color-bg))` instead of literal navy.

- **VIS-28 — Stack popup density and chrome** (Low, M).
  Evidence: `.stack-popup` uses `padding: var(--js-space-4)` (11.5px), `border-radius: var(--js-radius-lg)` (9px), and an absolutely positioned close button with `padding-right: 3rem` reserved on the toolbar (`StackPopupSurface.css:1-45`). The status bar and git summary use 9–10px text.
  Recommendation: an Explorer- or Finder-grade layout.
  - 32px toolbar row: back, forward, up, breadcrumb, search, view toggle.
  - Optional sidebar with pins and recent folders.
  - 28px rows with 16px icons.
  - Column header at 12px semibold muted.
  - Status bar at 12px.
  - Close in the top-right as a standard window caption button (46×32 like Win11), or an Esc hint.

#### Git panel
- **VIS-29 — The Git panel is a 3,294-line component with 36 bespoke buttons** (Med, L).
  Evidence: `StackGitPanel.svelte` (style block from line 2075, around 1,200 lines of CSS). It has its own spinner (`:2479`), drawer animation (`:3283`), `backdrop-filter: blur(8px)` (`:2513`), and literal colours `#ff8a8a`, `#f4c56a`, `#101827`.
  Recommendation: split it into `GitChangesList`, `GitDiffView`, `GitCommitBox`, and `GitBranchPicker` built on the shared primitives. Visually, aim for the VS Code SCM / GitHub Desktop / Tower conventions: a sticky commit box on top, grouped Staged and Changes sections with counts, and per-row hover actions (stage, discard, open). Use the M/A/D/U letter badges in semantic colours with tabular alignment.

#### Terminals
- **VIS-14 — Both terminal surfaces ignore the theme system** (High, M).
  Evidence:
  - `TerminalPanelSurface.css` has **54** literal colours (`#05070a`, `#111722`, `#263246`, `#67a3ff`, `#69d68b`, …) and 17 `!important`s.
  - `StackTerminalPane.css` has 15 literal colours.
  - xterm `theme` is unset in `TerminalPanelSurface.svelte:977-987`, so it uses the default ANSI palette. It's hard-coded to `#06080b` in `StackTerminalPane.svelte:242-248`.
  - The two terminals also diverge in behaviour and font size. The Stack terminal has `fontSize: 13`, Unicode11, WebLinks, and 5000 scrollback; the panel terminal has a configurable size, no Unicode11, and 8000 scrollback.
  - The xterm helper and screen overrides are copy-pasted three times (`StackPopupSurface.css:710-733`, `StackTerminalPane.css:36-65`, `TerminalPanelSurface.css:362-390`).
  Recommendation:
  - Add `--js-term-bg/fg/cursor/selection` plus 16 `--js-ansi-*` tokens per theme. Most of the 34 built-in themes (Dracula, Nord, Tokyo Night, Catppuccin, Solarized…) have canonical terminal palettes, so this is cheap and high-delight.
  - Build the xterm `ITheme` from computed CSS variables and re-apply it on theme change.
  - Share one `createShellTerminal()` factory and one CSS partial.
  - Default to `minimumContrastRatio: 4.5` (an xterm option) for safety.

- **VIS-30 — Terminal chrome: tabs and toolbar are square, colour-only, and hide close until hover** (Med, M).
  Evidence:
  - Tabs are `border-radius:0` boxes with full borders (`TerminalPanelSurface.css:53-66`).
  - The status indicator is a green glyph (`#69d68b`, `:112`) that swaps to a hidden close button on hover (`:126-135`).
  - Toolbar buttons are 4px-radius bordered boxes (`:166-178`).
  - Panes use an 8px radius while tabs use 0.
  Recommendation: Windows Terminal / Warp-style tabs.
  - Rounded-top tabs flush with the content.
  - The active tab takes the content background, inactive tabs are transparent with muted text.
  - Always reserve close-button space and show it at 40% opacity when inactive.
  - Running, done, and error states as small coloured dots with a tooltip.
  - Group toolbar actions into a "+ ▾" split button and an overflow menu.
  - Warp-style "blocks" (command, output, exit status as cards) would be a standout, and OSC 133 shell integration already appears to exist (`features/stack-browser/terminalShellIntegration.ts`).

#### Search (centered)
- **VIS-31 — Search lacks Spotlight/Raycast-class hierarchy** (High, M).
  Evidence: `SearchPanelSurface.svelte:440-555`.
  - It's a flat listbox of rows. The icon fallback is the first letter of `result.kind` (`:505-511`).
  - There are no section headers (Apps, Files, Folders, Commands, Settings) and no preview pane.
  - Highlighting renders one `<span>` per character (`highlightParts` loop, `:513-521`), which inflates the DOM and can make some screen readers read character by character.
  - The kbd hint is a bare `↵`.
  - `aria-expanded={true}` is constant.
  - Sizing mixes px and rem (`SearchPanelSurface.css:66-72, 85-95`).
  Recommendation:
  - 56px input row with 18px text and a large lens.
  - Grouped sections with 11px uppercase muted headers.
  - 40px rows with 24px icons, title at 13px, subtitle at 12px muted.
  - Right-aligned action hint (`↵ Open`, `Ctrl ↵ Pin`) in Kbd chips.
  - A right-hand preview pane for files: thumbnail, path, size, modified date.
  - A Raycast-style `Ctrl+K` action panel for secondary actions.
  - Merge adjacent highlighted characters into runs.
  - Animate the selection indicator by sliding a background element between rows (`transform`, 90ms) instead of jumping.

- **VIS-32 — Search empty, loading, and no-results states are thin** (Low, S).
  Evidence: there's a single `.empty-state surface-state info` "No search results matched" (`SearchPanelSurface.svelte:~548`), and the status line is 0.62rem (`SearchPanelSurface.css:27-35`).
  Recommendation: a zero-query state with recent items, pinned folders, and suggested commands. A loading state with 3–5 skeleton rows after 150ms. A no-results state that offers "Search the web / Search in Everything / Run as command".

#### Settings
- **VIS-33 — Settings is one long scroll with mixed save models and implementation jargon** (Med, M).
  Evidence: `SettingsPanelSurface.svelte`.
  - Sections in a single `<main>`: Appearance, Clock, "JSON shell settings", Speech, Keyboard shortcuts, Shell bars, Power.
  - Most toggles apply instantly, but hotkeys need an explicit Save/Cancel.
  - "JSON shell settings" and long technical help text ("Parakeet v2 int8 .tar, .tar.gz, .tgz…") are exposed.
  - Power actions (Sleep, Restart, Turn Off) sit at the bottom of Settings rather than in a power menu.
  Recommendation: the Win11 Settings layout.
  - Left nav (Personalization, Taskbar & bars, Search, Keyboard, Speech, Advanced) with a search box on top.
  - Settings cards: title, description, and control on the right.
  - Instant apply everywhere, with an undo toast for destructive changes. Hotkeys apply instantly after conflict validation.
  - Move power into a dedicated power flyout from the top-bar home button.

- **VIS-34 — The theme editor is powerful but raw** (Low, M).
  Evidence: `ThemeEditor.svelte` (441 lines). Contrast guidance is a `<details>` list of text warnings (`:378-380`) on a hard-coded `#222f42` panel (`:434`).
  Recommendation:
  - A live preview card showing a mini top bar, a taskbar tile, a menu, and a terminal.
  - Show contrast ratios next to each pair with pass/fail chips.
  - One-click "fix" that nudges lightness.
  - Generate from wallpaper (sample dominant colours, as Material You / KDE do).
  - Import/export of VS Code and Windows Terminal themes.

#### Process Manager / System Info
- **VIS-35 — Process Manager is a functional table without data visualisation** (Med, M).
  Evidence: `ProcessManagerSurface.css` has 22 font-size declarations, mostly 0.58–0.78rem. The header is an uppercase title at `0.78rem` (`:60-64`). Meters are `MeltProgress` bars per row. There are no history graphs, and System Info is a definition list (`SystemInfoView.css`).
  Recommendation, Task Manager / iStat / btop style:
  - Sparkline CPU, memory, and GPU history (60s) in the header.
  - Heat-tinted cells (background opacity proportional to value) instead of per-row bars.
  - Sticky header.
  - Grouped apps / background / Windows processes with collapse chevrons.
  - Per-row hover actions (end task, open location).
  - Monospace tabular numbers (already partially used, `:288,509`).

#### Command panel
- **VIS-36 — Command panel markup and CSS are dense and hard to maintain** (Low, M).
  Evidence:
  - `CommandPanelSurface.svelte:1019-1067` packs entire layouts into single 300–600-character lines.
  - `CommandPanelSurface.css` has 18 font-size values and uses uppercase 0.62rem labels with `letter-spacing:.08em` (`:90,144`).
  - It has its own `command-spin`.
  Recommendation: reformat the markup and move it onto the primitives. Visually, use a two-pane layout: a list on the left with run status dots, and a details/editor pane on the right with tabs (Configure / Runs). Show runs as a timeline with duration bars and exit-code chips.

#### Calendar, Audio, Tray, Quick Launch
- **VIS-37 — The Audio flyout lacks basics** (Med, S).
  Evidence: `AudioPanelSurface.svelte:228-300`.
  - There's no mute toggle.
  - The device pickers are unstyled native `<select>`s while Settings uses `MeltSelect`.
  - There's a manual "Refresh" text button.
  - App sessions show no icons.
  - The header is an uppercase 0.68rem `<strong>`.
  - Loading text uses "..." instead of "…".
  Recommendation, Win11 quick settings volume style:
  - Speaker icon button (mute) + slider + percentage.
  - Device list as a radio list with device icons.
  - Per-app rows with app icons and mini sliders.
  - Live updates without a manual refresh.
  - Peak meter bars.

- **VIS-38 — Calendar is a bare month grid with filler copy** (Low, M).
  Evidence: the footer says "Weekdays shown for local calendar" (`CalendarPanelSurface.svelte:114-117`). There's no agenda, no week numbers option, no "Today" button, and it ticks every second (`:61-64`) to drive a seconds display.
  Recommendation:
  - Today button, ISO week numbers option, world clocks.
  - Agenda from the Windows calendar if a provider exists.
  - Animated month transitions: slide 8px + fade, 160ms.
  - Today as an accent-filled circle, selected as an accent ring.

- **VIS-39 — Quick Launch is a plain list with no search** (Med, M).
  Evidence: `QuickLaunchPanelSurface.svelte`. It's a listbox of launcher rows with a `kbd` hint "↑↓ Enter Esc". There's no type-to-filter, and the empty state is plain text "No pinned Explorer launchers." `font-family` is re-declared at `:242`.
  Recommendation: Start-menu-grade.
  - Type-to-filter immediately (focus lands in a filter field).
  - Pinned grid (6 columns, 32px icons) plus a Recent list.
  - Right-click menu with Pin/Unpin, Open location, Run as admin.
  - Empty state with an "Add launcher" action.

- **VIS-40 — The tray flyout** (Low, S).
  Evidence: `TrayPanelSurface.css` (125 lines) uses `1rem` icons (`:107`).
  Recommendation: a 4-column grid of 24px hit targets with 16px icons, a hover tooltip with the app name, and drag to promote an item to the top bar.

#### Snipping
- **VIS-41 — The snip overlay is minimal compared with Snipping Tool, ShareX, or CleanShot** (Med, M).
  Evidence:
  - `SnipOverlaySurface.svelte`: the hint is an unpadded, unstyled `<span>` "Esc to cancel" (`.hint`).
  - Dim is a fixed `rgba(0,0,0,.48)`.
  - There's no live size readout, no crosshair guides, no magnifier loupe, and no mode toolbar (rectangle / window / fullscreen / freeform).
  - Inline `style=` strings are rebuilt for 5 elements on every pointermove.
  Recommendation:
  - A floating mode toolbar top-centre (Fluent pill: 40px tall, radius 8).
  - A W×H badge following the cursor.
  - Crosshair lines and an 8× loupe with a pixel grid and colour value.
  - Window-snapping highlight.
  - Use a single SVG mask or `clip-path` for the dim instead of 4 divs.
  - Style the hint as a pill using the same `Kbd` chip.

- **VIS-42 — The snip preview's actions are icon-only without visible labels or shortcuts** (Low, S).
  Evidence: `SnipPreviewSurface.svelte`. Copy, Save, and Dismiss are 2rem icon buttons, and Dismiss uses a separate `<img>` delete icon instead of `MaterialSymbolIcon`. The title is 0.72rem and the dimensions 0.58rem.
  Recommendation: a labelled split toolbar (Copy `Ctrl+C`, Save `Ctrl+S`, Annotate, Close `Esc`), quick markup tools (pen, highlighter, crop), auto-copy-to-clipboard on capture with a toast, and drag-out of the image.

#### Notifications, speech, context menus
- **VIS-20b — The notification banner** (High, S). Evidence: `QuickCommandNotificationSurface.svelte`.
  - The dismiss button is 14px, sitting half outside the banner.
  - The `<section>` has no `role="status"` or `aria-live`, so screen readers never hear it.
  - The 6s countdown ring (`dismiss-countdown 6s`) doesn't pause on hover or focus. Under reduced motion only the ring animation is removed.
  - The verdict is shown as text + icon (good), but the success colour is `--js-color-success-border`, a 42%-alpha colour.
  Recommendation, Win11 toast style:
  - 24–32px close button inside the card.
  - `role="status"` (or `alert` for failures).
  - Pause the timer on hover and focus (WCAG 2.2.1).
  - Solid semantic icon colours.
  - Slide-in from the edge (200ms decelerate) and fade-out.
  - Stack multiple banners with a "+2 more" collapse.

- **VIS-43 — Context menus are translucent over nothing** (High, S).
  Evidence:
  - `ContextMenu.svelte:33-34`: `background: color-mix(in srgb, var(--js-color-surface-overlay) 73%, transparent); backdrop-filter: blur(8px)`. In `base-dark`, `surface-overlay` is `rgba(255,255,255,.055)`, so the menu background is about 4% white.
  - `backdrop-filter` inside a transparent WebView only blurs that WebView's own pixels, not the desktop. Any in-surface menu over a sparse area is close to transparent text on whatever sits behind it. The overlay variant (`.context-menu-overlay`) and reduced transparency switch to opaque, which shows the problem was noticed.
  - Radius 12px for the menu and 8px for items doesn't match the rest of the shell (VIS-07).
  - The entrance is `scale(.95)` with no opacity fade (`:89-92`).
  Recommendation:
  - Default to an opaque `surface-raised` at 96–98% alpha.
  - Use real acrylic via the native window effects (Tauri `window-vibrancy`, `apply_acrylic` on Win10/11, `apply_mica` on Win11) for menu windows, so translucency samples the desktop.
  - Entrance: opacity 0→1 plus `translateY(-4px)` → 0 over 150ms, `cubic-bezier(0,0,0,1)` (Fluent "decelerate max").
  - Radius `--js-radius-md` (8px) for the menu and 4px for items.
  - Add keyboard-shortcut hint columns and submenu chevrons.

- **VIS-44 — The speech orb hard-codes a navy glyph on the theme accent** (Low, S).
  Evidence: `SpeechIndicatorSurface.svelte` uses `.mic-keyline { stroke:#162d49 }` and a gradient `#162d49 → #335371 → #a3c2e0` over `background: var(--js-color-accent)`. On dark or saturated accents (github-dark `#2f81f7`, solarized `#268bd2`, gruvbox-light `#b57614`) navy-on-accent contrast drops sharply. The surface is `aria-hidden="true"` with no live announcement that recording started. TopBar does have an sr-only `.mic-status`, which is good.
  Recommendation: compute the ink as `--js-fg-on-accent` (pick black or white by accent luminance), and keep the gradient hue-relative using `color-mix` with the accent.

### D. Motion & micro-interaction

- **VIS-45 — No motion system: one hard-coded easing, scattered durations, no surface enter/exit** (Med, M).
  Evidence:
  - Tokens `--js-motion-fast/medium` (`app.css:104-105`) exist, but components hard-code `140ms` (×8) and `120ms` (×5).
  - Easing is `ease` 13 times and one `cubic-bezier(0.22,1,0.36,1)`.
  - None of the top-level panels (Search, Stack, Audio, Calendar, Command, Settings, Tray) have an enter or exit transition. Only the Speech History (`speech-history-enter`) and Git drawer (`stack-git-drawer-in`) animate.
  - The global `transition` on `:where(button, input, select, [role=button], [tabindex])` (`app.css:1233-1240`) animates `background` (not `background-color`) on every tabindex element, including large grids.
  Recommendation, Fluent motion tokens:
  - Durations: `--js-dur-ultra-fast:50ms`, `faster:100ms`, `fast:150ms`, `normal:200ms`, `slow:300ms`.
  - Curves: `--js-ease-decelerate: cubic-bezier(0,0,0,1)`, `--js-ease-accelerate: cubic-bezier(1,0,1,1)`, `--js-ease-standard: cubic-bezier(.33,0,.67,1)`.
  - Panels: fade + 8px slide from their anchor edge (the top bar flyouts slide down), 200ms decelerate on enter, 100ms accelerate on exit.
  - Pressed: `scale(.97)` over 50ms.
  - Narrow the global transition to `background-color, border-color, color, box-shadow, outline-color`.

- **VIS-46 — Reduced motion is blunt and global** (Low, S).
  Evidence: `app.css:1288-1302` forces `animation-duration:1ms !important; animation-iteration-count:1 !important; transition-duration:1ms !important` on everything. Busy spinners then freeze at a random frame instead of switching to a non-motion indicator. Only the BottomBar busy bar has a designed fallback (`BottomBar.css:318-326`). `StackPopupSurface.css:938` slows the spinner instead of stopping it.
  Recommendation: under reduced motion, keep opacity crossfades (they're vestibular-safe) and replace spinners with a pulsing-opacity dot or static "Working…" text, in the shared Spinner primitive.

- **VIS-47 — No pressed states, inconsistent hover fills** (Med, S).
  Evidence:
  - Only BottomBar (`:active` at `BottomBar.css:166,429`) and the Stack close button (`StackPopupSurface.css:53-56`) have pressed states.
  - Hover uses `--js-color-accent-soft` in TopBar, Notification, and SnipPreview, but `--js-color-control-hover` in BottomBar, ContextMenu, and AnchoredPanelCloseButton. Those are different hues in most themes.
  Recommendation: one hover/pressed ladder for neutral controls: `fill-subtle` → `fill-subtle-hover` (+4% fg) → `fill-subtle-pressed` (+2% fg). Accent fills only for selected or primary.

- **VIS-48 — No skeletons anywhere; loading is plain text** (Low, M).
  Evidence: "Loading processes…" (`ProcessManagerSurface.svelte:212,627`), "Loading commands…" (`CommandPanelSurface.svelte:1027`), "Loading audio controls..." (`AudioPanelSurface.svelte:239`), and "Loading system information…" (`SystemInfoView.svelte:50`). Grep finds no skeleton component.
  Recommendation: a `Skeleton` primitive (shimmer gradient, static under reduced motion) shown after a 150–200ms delay to avoid flashing. Keep the text in an `aria-live` region for AT.

### E. Typography & fonts

- **VIS-49 — The default UI font is Open Sans, not the platform font** (Med, S).
  Evidence:
  - `app.css:61-62`: `--js-font-sans: 'Open Sans', 'Segoe UI Variable Text', …`.
  - Only Latin and Latin-ext subsets are declared (`app.css:1-9, 47-54`), but `assets/fonts/open-sans/open-sans-1..8.woff2` are shipped and unreferenced. Cyrillic, Greek, and Vietnamese glyphs fall back to Segoe mid-word.
  - Markdown preview uses Georgia for body and Trebuchet MS for headings (`StackMarkdownPreview.svelte:23-24`), a third and fourth typographic voice.
  - `TextEditorProjectionExperiment.svelte:168` sets `:global(body)` font and background, which leaks globally if it's ever mounted.
  Recommendation:
  - Default to `'Segoe UI Variable Text', 'Segoe UI', system-ui` with optical sizes: Segoe UI Variable has Small, Text, and Display axes, and should use `font-variation-settings: 'opsz'` by size. Keep Open Sans as an option.
  - Declare the remaining subsets or remove them.
  - Markdown preview should use the UI sans at 14–15px with 1.6 line-height, mono for code, and the same heading scale.

- **VIS-50 — Overuse of uppercase micro-labels** (Low, S).
  Evidence: `text-transform: uppercase` appears 42 times, usually combined with 0.54–0.68rem and weight 800. Examples: `TopBar.css:89`, `BottomBar.css:500`, `ProcessManagerSurface.css:63`, `AudioPanelSurface.svelte` header, `CommandPanelSurface.css:90,144`. Uppercase at 9px is hard to read and reads as "admin panel", not "OS".
  Recommendation: sentence case at 12px semibold muted for section labels (Fluent / macOS), with uppercase reserved for 1–2 overline contexts.

- **VIS-51 — Monospace stacks are duplicated and inconsistent** (Low, S).
  Evidence:
  - `TERMINAL_PANEL_FONT_FAMILY` (`TerminalPanelSurface.svelte:114`), `STACK_TERMINAL_FONT_FAMILY` (`StackTerminalPane.svelte:42`), and CSS stacks at `TerminalPanelSurface.css:339` and `StackTerminalPane.css:16`.
  - The terminal font doesn't follow the "Stack text editor font" preference (`shellPreferences.ts:89-95`).
  - `--js-font-mono` is undefined (VIS-02).
  Recommendation: `--js-font-mono` set from one preference ("Code font") that applies to the editor, both terminals, kbd, and hex fields. Enable ligatures for Cascadia Code optionally.

### F. Layout, CSS architecture & consistency

- **VIS-52 — Inline styles and the `!important` footprint** (Low, M).
  Evidence:
  - `!important`: 17 in `TerminalPanelSurface.css`, 17 in `StackTerminalPane.css`, 16 in `StackPopupSurface.css`, 1 each in `TopBar.css:18` (hides tooltips during snip) and `SettingsPanelSurface.css`.
  - Inline `style=` attributes: 6 in `StackPopupSurface.svelte`, 5 in `StackGitPanel.svelte`, 5 in `SnipOverlaySurface.svelte`, 3 in `TopBar.svelte`, and more.
  - The `StackGitPanel` transcript element carries a template-literal `font-size: ${transcriptFontSize}px` inline, alongside 10+ event handlers on one line.
  Recommendation: inline styles only for dynamic custom properties (`style:--x={…}`). Static styling goes in CSS. Scope xterm overrides in one `xterm-overrides.css` using `@layer vendor` so `!important` isn't needed.

- **VIS-53 — CSS split between global files and `<style>` blocks, with `:global` everywhere** (Low, M).
  Evidence: 16 component CSS files are imported globally. They use Svelte `:global(...)` selectors (`StackPopupSurface.css:704`), while `<style>` blocks use `:global(.js-context-menu …)` for every rule (`ContextMenu.svelte`). In practice there's no scoping, which invites cross-surface leaks such as `.preview-surface`, used by both `TaskPreviewSurface.css` and `SnipPreviewSurface.svelte`.
  Recommendation: adopt `@layer reset, tokens, base, primitives, surfaces, overrides;`. Namespace surface classes (`.js-taskbar__tile`) or rely on Svelte scoping. Add stylelint with `color-no-hex` (outside token files), `declaration-strict-value` for color, radius, and font-size, and `selector-max-specificity`.

- **VIS-54 — Global `user-select: none`** (Low, S).
  Evidence: `app.css:1187-1190` sets `body { user-select: none }`. Error messages, paths in Process Manager, System Info values, and notification text can't be selected or copied unless a surface opts back in. Only terminal output does (`StackPopupSurface.css:704`).
  Recommendation: keep `none` on chrome (bars, buttons), but set `user-select: text` on `.surface-state`, error text, table cells, System Info `dd`, and path displays.

- **VIS-55 — Scrollbars are accent-tinted, always visible, and inconsistent** (Low, S).
  Evidence: the global thumb is `--js-scrollbar-thumb` (accent at 42–48% alpha, `app.css:106`). The Task gallery overrides it to a 12px native-styled rail (`TaskGallerySurface.css:60-74`), and `.stack-pins` re-declares it (`TopBar.css:116-118`).
  Recommendation: Win11-style overlay scrollbars. A thin 2px neutral indicator that expands to 6px on hover, appears on scroll, and fades out after 1s. Neutral colour (fg at 30%), not accent.

- **VIS-56 — Translucency, Mica, and acrylic are absent despite the prompt and setting** (Med, M).
  Evidence:
  - Surfaces are 98.5–99% opaque (`--js-color-surface*` alpha .985/.99 in every theme).
  - Grep finds no `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)`, `window_effects`, or vibrancy call in `src-tauri/src`.
  - The "Reduce transparency" toggle (`SettingsPanelSurface.svelte`) therefore only changes `--js-color-surface-overlay` and context menus.
  Recommendation:
  - Bars and flyouts: Acrylic on Win10 (the dev machine is 10.0.19045) and Mica/Mica Alt on Win11, via `window-vibrancy` or direct DWM calls.
  - Pair with semi-transparent surface tokens (`surface` at 70–80% alpha in "transparent" mode).
  - Keep the opaque path for reduced transparency, battery saver, and forced colors.
  This single change does the most to make the shell look native.

- **VIS-57 — Bar resize handle discoverability** (Low, S).
  Evidence: the 5px transparent strip shows accent fill only on hover (`TopBar.css:20-38`, `BottomBar.css:17-34`). There's no cursor-adjacent size tooltip and no snap feedback.
  Recommendation: on hover, after a 300ms delay, show a centred 32×4px grip pill. While dragging, show a "32 px" tooltip and snap to 24/32/40/48 with a subtle tick.

- **VIS-58 — Density setting changes only spacing, not control height** (Low, S).
  Evidence: `app.css:118-122` changes only `--js-space-2..4`. Control heights are hard-coded in rem per component.
  Recommendation: `--js-control-h: 32px` (standard) / `28px` (compact) / `24px` (dense), driven by `data-shell-density` and consumed by all primitives.

- **VIS-59 — Copy and microcopy polish** (Low, S).
  Evidence:
  - Mixed ellipsis: "Loading audio controls..." (`AudioPanelSurface.svelte:239`) vs "Loading commands…".
  - "Turn Off" vs "Shut down" (Windows uses "Shut down").
  - Placeholders: "What are you looking for?" (search) and "Stack Browser" (path input placeholder, `StackPopupSurface.svelte:3354`).
  - The terminal tab "+" button is text.
  - Calendar footer filler text.
  Recommendation: a short voice guide: sentence case, real ellipsis (…), Windows verbs, and placeholders that teach ("Search apps, files, commands…", "Type a path or press Ctrl+L").

---

### Big bets / creative ideas

1. **Native materials pass ("feels like Windows 11, but better")**
   - Acrylic on bars and flyouts (Mica on Win11).
   - Rounded 8px flyouts with DWM shadows.
   - Segoe UI Variable with optical sizing and Fluent motion curves.
   - A 1px "light stroke" top highlight on surfaces (already exists as `--js-inset-highlight`; tune it per material).
   - Combined with VIS-01, VIS-05, VIS-07, and VIS-09, this makes JasonShell look native at first glance.

2. **Command-centre search (Raycast × PowerToys Run)**
   - One `Alt+Space` entry point with grouped results, a preview pane, and a `Ctrl+K` action palette.
   - Calculator, unit conversion, clipboard history, window switcher, and Quick Commands as first-class "extensions".
   - Fuzzy match highlighting in runs, and a sliding selection pill.
   - Recent and frecency zero-state.

3. **Living top bar (macOS menu bar meets the iOS Dynamic Island)**
   - A centre "activity capsule" that morphs to show what's happening now: recording (waveform from the existing speech meter), quick command running (progress ring then a ✓/✗ morph), a snip just taken (thumbnail, click to open), a terminal job finished.
   - Uses FLIP width animation (200ms decelerate), and replaces the separate notification banner window for most cases.

4. **Taskbar 2.0**
   - Pill indicators.
   - Smooth reflow when windows open or close (FLIP on `.task-group` with `transform` + 150ms).
   - Hover-intent previews that crossfade between siblings.
   - Optional Dock-style subtle magnification (1.0→1.15, falloff over 2 neighbours, disabled under reduced motion).
   - Drag-to-reorder with a spring settle.
   - Progress shown as a fill behind the tile (from `ITaskbarList3` progress).
   - Badge counts.

5. **Theme engine upgrades**
   - OKLCH seed-based generation with guaranteed contrast.
   - "Match wallpaper" (sample dominant hue).
   - Time-of-day auto light/dark following `prefers-color-scheme` and Windows' `AppsUseLightTheme`.
   - Per-theme ANSI palettes for the terminals.
   - Live preview diorama in the editor.
   - Import from VS Code / Windows Terminal JSON.

6. **Warp-style terminal blocks**
   - Using the existing shell-integration parser: each command becomes a collapsible card (prompt, command, output, duration, exit badge).
   - Hover actions: copy output, rerun, share to Quick Commands, open in Stack Browser.
   - Sticky command header while scrolling long output.

7. **Stack Browser as a Finder/Files-class mini file manager**
   - Real shell icons and thumbnails, plus a Quick Look preview (Space) for images, text, markdown, and PDFs.
   - Column view option.
   - Tag colours.
   - Breadcrumb dropdowns.
   - Git status as a subtle gutter with an inline diff peek.
   - Drag ghosts showing a count badge and stacked thumbnails.

8. **Snipping studio**
   - Mode toolbar, loupe, smart window and element detection, and post-capture annotation (arrow, blur, highlight, text).
   - OCR "copy text" via Windows.Media.Ocr.
   - Pin-to-screen floating snip.
   - All rendered with the same primitives and materials.

9. **Unified notification centre**
   - Toasts stack bottom-right (or under the activity capsule), with a history panel from the clock (Win11 pattern).
   - Grouped by source, with inline actions (Open output, Rerun) and a do-not-disturb toggle.

10. **Design-system site inside the repo**
    - A `/ui` gallery page (vite route, mocked IPC) rendering every primitive in every theme, density, and contrast mode.
    - Playwright screenshot diffing per theme.
    - This prevents the drift documented above from coming back.
