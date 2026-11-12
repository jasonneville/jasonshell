<script lang="ts">
  import { onMount, tick } from 'svelte';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import MeltSelect from './melt/MeltSelect.svelte';
  import StackConfirmDialog from './StackConfirmDialog.svelte';
  import ThemeColorField from './ThemeColorField.svelte';
  import {
    getSavedThemeState, requestShellThemeSelection, shellThemeOptions, subscribeShellThemes
  } from '../lib/themes';
  import {
    createCustomTheme, getThemePaletteColorLayers, resetThemePaletteColorLayer,
    resolveThemePaletteColor, setThemePaletteColor, THEME_PALETTE_TOKENS,
    type CustomTheme
  } from '../lib/themePalette';
  import { createThemeEditor, deleteCustomTheme, subscribeThemePreview } from '../lib/themePreview';

  let saved = $state.raw(getSavedThemeState());
  let options = $state(shellThemeOptions().map((theme) => ({ value: theme.id, label: theme.label })));
  let draft = $state.raw<CustomTheme | null>(null);
  let editor: ReturnType<typeof createThemeEditor> | null = null;
  let original = '';
  let error = $state('');
  let status = $state('');
  let invalidFields = $state<string[]>([]);
  let fieldGeneration = $state(0);
  let nameInput = $state<HTMLInputElement>();
  let editorRoot = $state<HTMLElement>();
  let confirmation = $state<
    | { kind: 'discard'; id: string; origin: HTMLElement | null }
    | { kind: 'delete'; id: string; expectedRevision: number; label: string; origin: HTMLElement | null }
    | null
  >(null);
  let draftOrigin: { element: HTMLElement; label: string | null; text: string } | null = null;
  let selectionAnswer: ((answer: boolean) => void) | null = null;
  const core = [
    ['--js-color-bg', 'Background'], ['--js-color-surface', 'Surface'],
    ['--js-color-surface-raised', 'Raised surface'], ['--js-color-text', 'Text'],
    ['--js-color-text-muted', 'Muted text'], ['--js-color-accent', 'Accent'], ['--js-color-border', 'Border']
  ] as const;
  const advanced = THEME_PALETTE_TOKENS.filter((token) => !core.some(([coreToken]) => coreToken === token));
  const selectedCustom = $derived(saved.customThemes.find((theme) => theme.id === saved.selectedId));
  const dirty = $derived(draft !== null && JSON.stringify(draft) !== original);
  const nameValid = $derived(Boolean(draft?.label.trim()) && (draft?.label.trim().length ?? 0) <= 80);
  const contrast = $derived(draft ? contrastWarnings(draft) : []);

  onMount(() => {
    const unsubscribe = subscribeShellThemes((state) => {
      saved = state;
      options = shellThemeOptions().map((theme) => ({ value: theme.id, label: theme.label }));
    });
    const unsubscribePreview = subscribeThemePreview((state) => {
      if (state.error) error = state.error;
      if (!state.active && editor) {
        editor = null;
        draft = null;
        invalidFields = [];
      }
    });
    return () => {
      unsubscribe();
      unsubscribePreview();
      selectionAnswer?.(false);
      editor?.dispose();
    };
  });

  function currentOrigin() {
    return document.activeElement instanceof HTMLElement ? document.activeElement : null;
  }

  function selectionRequested(id: string): Promise<boolean> {
    if (!draft) return Promise.resolve(true);
    if (confirmation) return Promise.resolve(false);
    return new Promise((resolve) => {
      selectionAnswer = resolve;
      confirmation = {
        kind: 'discard', id,
        // The listbox is hidden as soon as its option is selected; restoring
        // focus there loses it to body. Return to the persistent theme trigger.
        origin: editorRoot?.querySelector<HTMLButtonElement>('.melt-select-trigger') ?? currentOrigin()
      };
    });
  }

  async function selectTheme(id: string) {
    error = '';
    status = '';
    try { await requestShellThemeSelection(id); }
    catch (reason) { showError(reason, 'Could not select theme. Save or cancel the draft in Settings, then retry.'); }
  }

  function startDraft(copy: boolean, custom = selectedCustom, origin = currentOrigin()) {
    if (draft) return;
    error = '';
    status = '';
    try {
      const source = custom ?? saved.selectedId;
      const sourceLabel = custom?.label ?? options.find((option) => option.value === saved.selectedId)?.label ?? 'Theme';
      const sourceDraft = copy ? createCustomTheme(source, `${sourceLabel.slice(0, 75)} copy`) : source;
      editor = createThemeEditor(sourceDraft, { onSelectionRequest: selectionRequested });
      draft = editor.getDraft();
      draftOrigin = origin ? { element: origin, label: origin.getAttribute('aria-label'), text: origin.textContent?.trim() ?? '' } : null;
      original = JSON.stringify(draft);
      invalidFields = [];
      fieldGeneration += 1;
      void tick().then(() => { nameInput?.focus(); nameInput?.select(); });
    } catch (reason) { showError(reason, 'Could not open a theme draft. Try again.'); }
  }

  function updateDraft(next: CustomTheme) {
    if (!editor) return;
    error = '';
    draft = next;
    // Name input may be temporarily empty. Keep the last valid preview until
    // the whole draft validates, without losing the user's intermediate name.
    if (!next.label.trim() || next.label.trim().length > 80) return;
    try { editor.update({ ...next, label: next.label.trim() }); }
    catch (reason) { showError(reason, 'Preview could not update. Correct the color and retry.'); }
  }

  function changeColor(token: string, value: string, layerIndex = 0) {
    if (!draft) return;
    try { updateDraft({ ...draft, palette: setThemePaletteColor(draft.palette, token, value, undefined, layerIndex) }); }
    catch (reason) { showError(reason, 'Color could not update. Use a valid hexadecimal color.'); }
  }

  function resetColor(token: string, layerIndex?: number) {
    if (!draft) return;
    updateDraft({ ...draft, palette: layerIndex === undefined
      ? { ...draft.palette, [token]: draft.startingPalette[token] }
      : resetThemePaletteColorLayer(draft.palette, draft.startingPalette, token, layerIndex) });
  }

  function resetColors() {
    if (!draft) return;
    updateDraft({ ...draft, palette: { ...draft.startingPalette } });
    invalidFields = [];
    fieldGeneration += 1;
  }

  function cancelDraft() {
    try {
      editor?.cancel();
      editor?.dispose();
      editor = null;
      draft = null;
      invalidFields = [];
      error = '';
      status = 'Draft discarded. Current saved theme restored.';
      restoreDraftFocus();
    } catch (reason) { showError(reason, 'Could not restore the saved theme. Retry Cancel.'); }
  }

  function restoreDraftFocus() {
    const origin = draftOrigin;
    draftOrigin = null;
    void tick().then(() => {
      const buttons = Array.from(editorRoot?.querySelectorAll<HTMLButtonElement>('button') ?? []);
      const replacement = buttons.find((button) => origin && (origin.label
        ? button.getAttribute('aria-label') === origin.label
        : button.textContent?.trim() === origin.text));
      const target = origin?.element.isConnected ? origin.element : replacement;
      if (target && !target.closest('details:not([open])')) target.focus();
      else editorRoot?.querySelector<HTMLButtonElement>('.melt-select-trigger')?.focus();
    });
  }

  function saveDraft() {
    if (!editor || !draft || !nameValid || invalidFields.length) return;
    try {
      const currentEditor = editor;
      currentEditor.update({ ...draft, label: draft.label.trim() });
      const result = currentEditor.save();
      currentEditor.dispose();
      editor = null;
      draft = null;
      status = `Saved and applied ${result.label}.`;
      error = '';
    } catch (reason) { showError(reason, 'Theme was not saved. Draft and preview retained. Free storage space or delete an unused theme, then retry.'); }
  }

  function requestDelete(id = selectedCustom?.id) {
    const target = saved.customThemes.find((theme) => theme.id === id);
    if (!target) return;
    confirmation = { kind: 'delete', id: target.id, expectedRevision: target.revision, label: target.label, origin: currentOrigin() };
  }

  function closeConfirmation(accepted: boolean) {
    const pending = confirmation;
    if (!pending) return;
    confirmation = null;
    if (pending.kind === 'discard') {
      selectionAnswer?.(accepted);
      selectionAnswer = null;
    } else if (accepted) {
      try {
        deleteCustomTheme(pending.id, pending.expectedRevision);
        status = `Deleted ${pending.label}.`;
        error = '';
      } catch (reason) { showError(reason, 'Theme could not be deleted. Retry after saving or cancelling any draft.'); }
    }
    void tick().then(() => {
      if (!pending.origin?.isConnected) editorRoot?.querySelector<HTMLButtonElement>('.melt-select-trigger')?.focus();
    });
  }

  export function resetPresentationTheme() {
    if (confirmation) closeConfirmation(false);
    cancelDraft();
    void selectTheme('base-dark');
  }

  function showError(reason: unknown, fallback: string) {
    error = reason instanceof Error ? reason.message : fallback;
  }

  function fieldValidity(token: string, valid: boolean) {
    invalidFields = valid ? invalidFields.filter((field) => field !== token) : [...new Set([...invalidFields, token])];
  }

  function containConfirmationFocus(event: FocusEvent) {
    if (!confirmation) return;
    const dialog = editorRoot?.querySelector<HTMLElement>('[role="alertdialog"]');
    // A closing Melt picker restores its trigger after the shared dialog's
    // initial-focus tick. Keep that restoration inside the active modal too.
    if (dialog && event.target instanceof Node && !dialog.contains(event.target)) {
      dialog.querySelector<HTMLButtonElement>('.stack-confirm-actions button')?.focus();
    }
  }

  function rgba(theme: CustomTheme, token: string, layerIndex = 0): [number, number, number, number] {
    const value = resolveThemePaletteColor(theme.palette, token, layerIndex);
    if (value === 'transparent') return [0, 0, 0, 0];
    if (value.startsWith('#')) {
      let digits = value.slice(1);
      if (digits.length <= 4) digits = [...digits].map((digit) => digit + digit).join('');
      return [parseInt(digits.slice(0, 2), 16), parseInt(digits.slice(2, 4), 16), parseInt(digits.slice(4, 6), 16), digits.length === 8 ? parseInt(digits.slice(6), 16) / 255 : 1];
    }
    const rgb = value.match(/^rgba?\(([^()]*)\)$/);
    if (rgb) {
      const channels = rgb[1].split(',').map(Number);
      return [channels[0], channels[1], channels[2], channels[3] ?? 1];
    }
    if (typeof document === 'undefined') return [0, 0, 0, 1];
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 1;
    const context = canvas.getContext('2d');
    if (!context) return [0, 0, 0, 1];
    context.fillStyle = value;
    context.fillRect(0, 0, 1, 1);
    const pixel = context.getImageData(0, 0, 1, 1).data;
    return [pixel[0], pixel[1], pixel[2], pixel[3] / 255];
  }

  function fieldColor(theme: CustomTheme, token: string, layerIndex = 0) {
    const [r, g, b, a] = rgba(theme, token, layerIndex);
    return `rgba(${r}, ${g}, ${b}, ${a})`;
  }

  function over(foreground: number[], background: number[]): number[] {
    return foreground.slice(0, 3).map((channel, index) => channel * foreground[3] + background[index] * (1 - foreground[3]));
  }

  function luminance(color: number[]) {
    return color.map((channel) => channel / 255).map((channel) => channel <= .04045 ? channel / 12.92 : ((channel + .055) / 1.055) ** 2.4)
      .reduce((sum, channel, index) => sum + channel * [.2126, .7152, .0722][index], 0);
  }

  function contrastWarnings(theme: CustomTheme): string[] {
    const backdrop = rgba(theme, '--js-color-bg');
    const background = over(backdrop, [0, 0, 0]);
    const pairs = [
      ['Text / surface', '--js-color-text', '--js-color-surface', 4.5],
      ['Muted text / surface', '--js-color-text-muted', '--js-color-surface', 4.5],
      ['Text / control', '--js-color-text', '--js-color-control', 4.5],
      ['Accent / surface', '--js-color-accent', '--js-color-surface', 3],
      ['Border / surface', '--js-color-border', '--js-color-surface', 3]
    ] as const;
    const warnings: string[] = [];
    if (backdrop[3] < 1) warnings.push('Background is transparent: desktop backdrop is unknown; contrast estimates cannot certify readability.');
    for (const [label, foregroundToken, surfaceToken, target] of pairs) {
      const surface = over(rgba(theme, surfaceToken), background);
      const foreground = over(rgba(theme, foregroundToken), surface);
      const light = luminance(foreground), dark = luminance(surface);
      const ratio = (Math.max(light, dark) + .05) / (Math.min(light, dark) + .05);
      if (ratio < target) warnings.push(`${label}: ${ratio.toFixed(2)}:1 (suggested ${target}:1).`);
    }
    return warnings;
  }

  function tokenLabel(token: string) {
    return token.replace(/^--js-/, '').replaceAll('-', ' ').replace(/^./, (letter) => letter.toUpperCase()) + (/shadow|focus-ring|inset-highlight/.test(token) ? ' tint' : '');
  }
</script>

<svelte:document onfocusin={containConfirmationFocus} />

<section class="theme-editor" aria-label="Custom theme editor" bind:this={editorRoot}>
  <MeltSelect label="Theme" value={saved.selectedId} {options} onChange={selectTheme} />
  {#if !draft}
    <div class="theme-editor-actions">
      <MeltActionButton onClick={(event) => startDraft(true, selectedCustom, event.currentTarget as HTMLElement)}>Create copy</MeltActionButton>
      {#if selectedCustom}
        <MeltActionButton onClick={(event) => startDraft(false, selectedCustom, event.currentTarget as HTMLElement)}>Edit theme</MeltActionButton>
        <MeltActionButton onClick={(event) => startDraft(true, selectedCustom, event.currentTarget as HTMLElement)}>Duplicate theme</MeltActionButton>
        <MeltActionButton onClick={() => requestDelete()}>Delete theme</MeltActionButton>
      {/if}
    </div>
    <p class="theme-editor-help">Built-in themes stay unchanged. Create a named copy to customize shared shell colors.</p>
    {#if saved.customThemes.length}
      <details class="theme-saved-list"><summary>Manage saved themes ({saved.customThemes.length})</summary>
        <ul>{#each saved.customThemes as custom (custom.id)}<li>
          <strong>{custom.label}{custom.id === saved.selectedId ? ' · active' : ''}</strong>
          <div class="theme-editor-actions">
            <MeltActionButton ariaLabel={`Edit ${custom.label}`} onClick={(event) => startDraft(false, custom, event.currentTarget as HTMLElement)}>Edit</MeltActionButton>
            <MeltActionButton ariaLabel={`Duplicate ${custom.label}`} onClick={(event) => startDraft(true, custom, event.currentTarget as HTMLElement)}>Duplicate</MeltActionButton>
            <MeltActionButton ariaLabel={`Delete ${custom.label}`} onClick={() => requestDelete(custom.id)}>Delete</MeltActionButton>
          </div>
        </li>{/each}</ul>
      </details>
    {/if}
  {:else}
    <div class="theme-editor-heading"><h3>Theme colors</h3><span>{dirty ? 'Unsaved changes' : 'Draft · not saved'}</span></div>
    <p class="theme-editor-help">Base: {options.find((option) => option.value === draft?.baseBuiltinId)?.label ?? draft.baseBuiltinId} · {draft.mode}. Changes preview immediately. Hiding Settings keeps this draft; restart loads only saved themes.</p>
    <label class="theme-name"><span>Theme name</span><input bind:this={nameInput} type="text" value={draft.label} maxlength="80" aria-invalid={!nameValid} oninput={(event) => draft && updateDraft({ ...draft, label: event.currentTarget.value })} /></label>
    {#if !nameValid}<p class="theme-editor-error">Enter a name of 1–80 characters.</p>{/if}
    {#key fieldGeneration}
      <div class="theme-color-list">
        {#each core as [token, label] (token)}
          <ThemeColorField {label} value={draft.palette[token]} resolved={fieldColor(draft, token)} onChange={(value) => changeColor(token, value)} onReset={() => resetColor(token)} onValidityChange={(valid) => fieldValidity(token, valid)} />
        {/each}
      </div>
      <details class="theme-advanced"><summary>Advanced colors & tints</summary><p class="theme-editor-help">Aliases stay linked until overridden. Composite fields change tint only; shadow and focus geometry stay unchanged.</p>
        <div class="theme-color-list">
          {#each advanced as token (token)}
            {@const layers = getThemePaletteColorLayers(draft.palette, token)}
            {#each layers as layer, layerIndex (layerIndex)}
              <ThemeColorField
                label={tokenLabel(token) + (layers.length > 1 ? ` layer ${layerIndex + 1}` : '')}
                value={layer} resolved={fieldColor(draft, token, layerIndex)}
                onChange={(value) => changeColor(token, value, layerIndex)}
                onReset={() => resetColor(token, layers.length > 1 ? layerIndex : undefined)}
                onValidityChange={(valid) => fieldValidity(`${token}:${layerIndex}`, valid)}
              />
            {/each}
          {/each}
        </div>
      </details>
    {/key}
    <details class="theme-contrast" open={contrast.length > 0}><summary>Contrast guidance {contrast.length ? `· ${contrast.length} warnings` : '· no flagged pairs'}</summary>
      <p class="theme-editor-help">Nonblocking estimates composite alpha over the configured background (black when its desktop backdrop is unknown). Actual native translucency may differ.</p>
      {#if contrast.length}<ul>{#each contrast as warning (warning)}<li>{warning}</li>{/each}</ul>{/if}
    </details>
    <div class="theme-editor-actions theme-save-actions">
      <MeltActionButton disabled={!nameValid || invalidFields.length > 0} onClick={saveDraft}>Save & apply</MeltActionButton>
      <MeltActionButton onClick={cancelDraft}>Cancel</MeltActionButton>
      <MeltActionButton onClick={resetColors}>Reset colors</MeltActionButton>
    </div>
    <p class="theme-editor-help">Reset colors restores the source-copy baseline, not presentation defaults. Save is required even for name-only edits.</p>
  {/if}
  {#if error}<p class="theme-editor-error" role="alert">{error}</p>{/if}
  {#if status}<p class="theme-editor-status" role="status">{status}</p>{/if}
  {#if confirmation}
    <StackConfirmDialog
      title={confirmation.kind === 'delete' ? 'Delete custom theme?' : 'Discard theme draft?'}
      message={confirmation.kind === 'delete' ? 'This removes the saved theme. If active, Base Dark will be applied. This cannot be undone.' : 'Selecting another theme discards this draft and its preview. Keep editing to preserve your changes.'}
      confirmLabel={confirmation.kind === 'delete' ? 'Delete theme' : 'Discard & select'}
      cancelLabel={confirmation.kind === 'delete' ? 'Cancel' : 'Keep editing'}
      tone="danger" returnFocus={confirmation.origin}
      onConfirm={() => closeConfirmation(true)} onCancel={() => closeConfirmation(false)}
    />
  {/if}
</section>

<style>
  /* A deliberately independent recovery surface: any custom palette, including
     transparent or same-color foreground/background, must remain reversible. */
  .theme-editor {
    --js-color-text: #f1f5ff; --js-color-text-strong: #ffffff; --js-color-text-muted: #b9c7dc;
    --js-color-surface-raised: #202c3d; --js-color-surface-sunken: #101722;
    --js-color-border: #58677e; --js-color-accent: #b5d0ff; --js-color-accent-border: #9dbdf2;
    --js-color-control-hover: #33455d; --js-color-selected: #344966; --js-bg-control: #27364b;
    --js-color-error: #702d3b; --js-color-error-border: #ffb5bf;
    --js-focus-ring: 0 0 0 2px #b5d0ff;
    --js-scrollbar-thumb: #8294b0; --js-scrollbar-track: #182231;
    background: #182231; border: 1px solid #58677e; border-radius: var(--js-radius-md);
    color: #f1f5ff; display: grid; gap: .65rem; padding: .65rem; min-width: 0;
  }
  .theme-editor-heading { display: flex; flex-wrap: wrap; justify-content: space-between; gap: .3rem; align-items: center; }
  h3 { font-size: .78rem; margin: 0; }
  .theme-editor-heading span { font-size: .6rem; color: #ffdd9b; }
  .theme-editor-help { color: #b9c7dc; font-size: .63rem; line-height: 1.5; margin: 0; }
  .theme-editor-actions { display: flex; flex-wrap: wrap; gap: .35rem; }
  .theme-editor :global(button:not(.stack-confirm-hitbox)) { min-height: 1.9rem; }
  .theme-editor :global(.theme-editor-actions button), .theme-editor :global(.theme-color-controls button) { background: #27364b; border: 1px solid #58677e; border-radius: var(--js-radius-sm); color: #f1f5ff; font: inherit; font-size: .65rem; padding: .25rem .55rem; }
  .theme-editor :global(button:focus-visible), summary:focus-visible { outline: 2px solid #b5d0ff; outline-offset: 2px; }
  .theme-editor :global(button:disabled) { opacity: .55; cursor: not-allowed; }
  .theme-editor :global(.theme-save-actions button:first-child) { background: #b5d0ff; color: #142137; font-weight: 800; }
  .theme-color-list { display: grid; gap: .65rem; }
  .theme-name { display: grid; gap: .25rem; }
  .theme-name span { color: #b9c7dc; font-size: .65rem; }
  .theme-name input { background: #101722; border: 1px solid #58677e; border-radius: var(--js-radius-sm); color: #f1f5ff; min-height: 1.9rem; min-width: 0; width: 100%; font: inherit; font-size: .7rem; padding: .2rem .4rem; }
  .theme-name input:focus-visible { outline: 2px solid #b5d0ff; outline-offset: 2px; }
  summary { cursor: pointer; font-size: .68rem; padding: .25rem 0; }
  details[open] > .theme-editor-help { margin: .3rem 0 .6rem; }
  .theme-contrast { background: #222f42; border-radius: var(--js-radius-sm); padding: .35rem .5rem; }
  ul { margin: .35rem 0; padding-left: 1rem; color: #ffdd9b; font-size: .62rem; }
  li + li { margin-top: .25rem; }
  .theme-saved-list ul { list-style: none; padding: 0; color: #f1f5ff; display: grid; gap: .5rem; }
  .theme-saved-list li { display: grid; gap: .25rem; }
  .theme-editor-error { color: #ffc8cf; font-size: .66rem; margin: 0; }
  .theme-editor-status { color: #bce7cd; font-size: .66rem; margin: 0; }
</style>
