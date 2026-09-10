# Adding standalone SVG icon assets

Use this when adding an external `.svg` file to a Svelte 5/Vite/Tauri component. Current working implementation: `src/components/CommandPanelSurface.svelte` saved-command create icon uses `src/assets/icons/add_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg`.

## Prefer shared icons first

- Use `src/components/icons/MaterialSymbolIcon.svelte` when the icon is a Material Symbol or should inherit theme color with `currentColor`.
- Use a standalone asset only when the SVG is copied from an external source, must stay byte-stable, or is not in the shared registry.

## Steps

1. Copy SVG into `src/assets/icons/`.
2. Use safe filename:
   - lowercase words, descriptive noun/action.
   - keep `.svg` extension.
   - avoid spaces, uppercase-only abbreviations, user/private names, temp names.
   - Material export names are OK, but copy exact case from disk when importing.
3. In the component, create a Vite asset URL. Adjust `../assets/...` relative to that component file:

   ```svelte
   <script lang="ts">
     const addIconUrl = new URL('../assets/icons/add.svg', import.meta.url).href;
   </script>
   ```

4. Render as decorative image inside `MeltActionButton`. Keep the button accessible name on `ariaLabel`:

   ```svelte
   <MeltActionButton class="command-text-button command-create-button" ariaLabel="Create command" onClick={startNewEntry}>
     <img class="command-new-icon" src={addIconUrl} alt="" aria-hidden="true" draggable="false" />
   </MeltActionButton>
   ```

5. Size icon and button hit target in CSS:

   ```css
   .command-create-button { align-items: center; display: inline-flex; justify-content: center; min-height: 24px; min-width: 24px; padding: 0; }
   .command-new-icon { display: block; height: 16px; width: 16px; }
   ```

## Theme/color warning

- SVG loaded through `<img>` does **not** inherit parent `color` or `currentColor` from CSS.
- Hardcoded `fill="#..."` stays fixed across themes.
- If theme-aware color is required, prefer `MaterialSymbolIcon.svelte` or inline SVG with `fill="currentColor"`.

## Focused source-contract test example

Keep source-contract tests essential, not a brittle copy of the whole component. Assert the asset exists, the component imports the exact file, the image is decorative, and the button keeps an accessible name + handler. Current implementation example:

```js
test('quick command create action uses copied add icon while retaining accessible name and behavior', () => {
  const addIconSource = readFileSync(commandPanelNewIconPath, 'utf8');

  assert.match(addIconSource, /<svg[^>]*viewBox="0 -960 960 960"/);
  assert.match(commandPanelSource, /new URL\('\.\.\/assets\/icons\/add_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24\.svg', import\.meta\.url\)\.href/);
  assert.match(commandPanelSource, /ariaLabel="Create command"/);
  assert.match(commandPanelSource, /onClick=\{startNewEntry\}/);
  assert.match(commandPanelSource, /<img[^>]*alt=""[^>]*aria-hidden="true"[^>]*draggable="false"/);
});
```

## Validation

Run focused + repo gates for touched surface:

```bash
npm run check
node --test tests/commandPanelWiring.test.mjs
npm run build
git diff --check
```

## Checklist

- [ ] Preserve unrelated dirty worktree changes.
- [ ] SVG copied under `src/assets/icons/`.
- [ ] Filename safe, lowercase, descriptive.
- [ ] `new URL(..., import.meta.url).href` path correct from component.
- [ ] `<img>` decorative: `alt=""`, `aria-hidden="true"`, `draggable="false"`.
- [ ] `MeltActionButton` keeps usable `ariaLabel`.
- [ ] CSS gives stable icon size and button target `>= 24px` by `>= 24px`.
- [ ] Color/theme choice intentional: hardcoded asset fill vs shared `currentColor` icon.
- [ ] Focused source-contract test added/updated.
- [ ] Validation commands run or documented if skipped.
