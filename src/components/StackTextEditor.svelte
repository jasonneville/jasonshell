<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import {
    createStackTextEditorAdapter,
    type StackTextEditorAdapter
  } from '../features/stack-browser/stackTextEditorAdapter';
  import { isStackBasicTextEditorDirty } from '../features/stack-browser/basicTextEditorDirtyState';
  import { isStackBasicTextSaveConflict } from '../features/stack-browser/basicTextEditorSaveConflict';
  import { readStackBasicTextFile, saveStackBasicTextFile } from '../lib/stackPopup';
  import {
    addShellPreferencesChangeListener,
    getInitialShellPreferences,
    stackEditorFontById
  } from '../lib/shellPreferences';
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import StackMarkdownPreview from './StackMarkdownPreview.svelte';
  import StackConfirmDialog from './StackConfirmDialog.svelte';

  export let path: string;
  export let onDirtyChange: (dirty: boolean) => void;
  export let onDismiss: (dirty: boolean) => void;

  let editorHost: HTMLDivElement | null = null;
  let editorAdapter: StackTextEditorAdapter | null = null;
  let loading = true;
  let errorMessage = '';
  let saveErrorMessage = '';
  let reloadErrorMessage = '';
  let saveConflict = false;
  let reloadConfirmation = false;
  let reloadDialog: StackConfirmDialog | undefined;
  let initialContent = '';
  let fileIdentity = '';
  let draft = '';
  let saving = false;
  let markdown = false;
  let editorMode: 'preview' | 'edit' = 'edit';
  let requestSequence = 0;
  let disposed = false;
  let stackEditorFontStack = stackEditorFontById(getInitialShellPreferences().stackEditorFontId).stack;
  const removePreferencesListener = addShellPreferencesChangeListener((preferences) => {
    stackEditorFontStack = stackEditorFontById(preferences.stackEditorFontId).stack;
    editorAdapter?.setFont(stackEditorFontStack);
  });

  $: filename = path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
  $: dirty = isStackBasicTextEditorDirty(draft, initialContent);
  $: onDirtyChange(dirty);
  $: void loadFile(path);

  function isMarkdownPath(filePath: string) {
    return /\.md$/i.test(filePath);
  }

  function destroyEditor() {
    editorAdapter?.destroy();
    editorAdapter = null;
  }

  async function mountDraftEditor(filePath: string) {
    await tick();
    if (disposed || filePath !== path || !editorHost) return;
    const mountedAdapter = createStackTextEditorAdapter({
      parent: editorHost,
      content: draft,
      path: filePath,
      fontStack: stackEditorFontStack,
      onChange: (currentDraft) => { draft = currentDraft; },
      onDismiss: () => onDismiss(dirty)
    });
    editorAdapter = mountedAdapter;
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
    if (!disposed && filePath === path && editorAdapter === mountedAdapter) mountedAdapter.focusAtStart();
  }

  function setEditorMode(mode: 'preview' | 'edit') {
    if (editorMode === mode) return;
    editorMode = mode;
    destroyEditor();
    if (mode === 'edit') void mountDraftEditor(path);
  }

  async function saveDraft() {
    if (!dirty || saving || loading || errorMessage || !fileIdentity || saveConflict) return;

    const sequence = requestSequence;
    const expectedContent = initialContent;
    const expectedIdentity = fileIdentity;
    const contentToSave = draft;
    saving = true;
    saveErrorMessage = '';
    try {
      const result = await saveStackBasicTextFile(path, contentToSave, expectedContent, expectedIdentity);
      if (disposed || sequence !== requestSequence) return;
      initialContent = result.content;
      fileIdentity = result.identity;
      saveConflict = false;
    } catch (error) {
      if (disposed || sequence !== requestSequence) return;
      const message = error instanceof Error && error.message ? error.message : String(error);
      saveErrorMessage = message;
      saveConflict = isStackBasicTextSaveConflict(message);
    } finally {
      if (!disposed && sequence === requestSequence) saving = false;
    }
  }

  function isSaveShortcutBlockedByActiveModal(event: KeyboardEvent) {
    const target = event.target;
    return target instanceof Element
      && target.closest('[role="alertdialog"][aria-modal="true"], [role="dialog"][aria-modal="true"]') !== null;
  }

  function handleSaveShortcut(event: KeyboardEvent) {
    if (event.key.toLowerCase() !== 's' || !(event.ctrlKey || event.metaKey)) return;
    event.preventDefault();
    if (isSaveShortcutBlockedByActiveModal(event)) return;
    void saveDraft();
  }

  function requestConflictReload() {
    if (saveConflict && !loading && !saving) reloadConfirmation = true;
  }

  function cancelConflictReload() {
    reloadConfirmation = false;
  }

  export function handleMouseNavigation(direction: -1 | 1): 'consumed' | 'unhandled' {
    if (reloadConfirmation) {
      reloadDialog?.handleMouseNavigation(direction);
    } else if (direction === -1 && !saving) {
      onDismiss(isStackBasicTextEditorDirty(draft, initialContent));
    }
    return 'consumed';
  }

  async function confirmConflictReload() {
    if (!saveConflict) return;
    reloadConfirmation = false;
    await reloadDiskVersion(path);
  }

  async function commitLoadedFile(filePath: string, sequence: number, result: { content: string; identity: string }) {
    destroyEditor();
    initialContent = result.content;
    fileIdentity = result.identity;
    draft = result.content;
    markdown = isMarkdownPath(filePath);
    editorMode = markdown ? 'preview' : 'edit';
    errorMessage = '';
    saveErrorMessage = '';
    reloadErrorMessage = '';
    saveConflict = false;
    loading = false;
    if (markdown) return;
    await tick();
    if (disposed || sequence !== requestSequence || filePath !== path) return;
    if (!editorHost) return;
    const mountedAdapter = createStackTextEditorAdapter({
      parent: editorHost,
      content: result.content,
      path: filePath,
      fontStack: stackEditorFontStack,
      onChange: (currentDraft) => {
        draft = currentDraft;
      },
      onDismiss: () => onDismiss(dirty)
    });
    if (disposed || sequence !== requestSequence || filePath !== path) {
      mountedAdapter.destroy();
      return;
    }
    editorAdapter = mountedAdapter;
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
    if (disposed || sequence !== requestSequence || filePath !== path || editorAdapter !== mountedAdapter) return;
    mountedAdapter.focusAtStart();
  }

  async function reloadDiskVersion(filePath: string) {
    const sequence = ++requestSequence;
    reloadErrorMessage = '';
    try {
      const result = await readStackBasicTextFile(filePath);
      if (disposed || sequence !== requestSequence || filePath !== path) return;
      await commitLoadedFile(filePath, sequence, result);
    } catch (error) {
      if (disposed || sequence !== requestSequence || filePath !== path) return;
      reloadErrorMessage = error instanceof Error && error.message ? error.message : String(error);
    }
  }

  async function loadFile(filePath: string) {
    const sequence = ++requestSequence;
    destroyEditor();
    loading = true;
    saving = false;
    errorMessage = '';
    saveErrorMessage = '';
    reloadErrorMessage = '';
    saveConflict = false;
    reloadConfirmation = false;
    initialContent = '';
    fileIdentity = '';
    draft = '';
    markdown = isMarkdownPath(filePath);
    editorMode = markdown ? 'preview' : 'edit';

    try {
      const result = await readStackBasicTextFile(filePath);
      if (disposed || sequence !== requestSequence || filePath !== path) return;
      await commitLoadedFile(filePath, sequence, result);
    } catch (error) {
      if (disposed || sequence !== requestSequence || filePath !== path) return;
      loading = false;
      errorMessage = error instanceof Error && error.message ? error.message : String(error);
    }
  }

  onDestroy(() => {
    disposed = true;
    requestSequence += 1;
    removePreferencesListener();
    destroyEditor();
  });
</script>

<svelte:window on:keydown={handleSaveShortcut} />

<section class="stack-text-editor" aria-labelledby="stack-text-editor-title" aria-busy={loading || saving}>
  <header class="stack-text-editor-header">
    <MeltActionButton class="stack-text-editor-close" ariaLabel="Close editor and return to folder" onClick={() => onDismiss(dirty)}>
      <MaterialSymbolIcon name="arrow_back" />
    </MeltActionButton>
    <div class="stack-text-editor-context">
      <strong id="stack-text-editor-title">{filename}</strong>
      <span title={path}>{path}</span>
    </div>
    <span class:dirty class="stack-text-editor-draft-state" role="status" aria-live="polite">
      {saving ? 'Saving…' : dirty ? 'Draft changed' : 'Saved'}
    </span>
    <MeltActionButton
      class="stack-text-editor-save"
      ariaLabel="Save file"
      disabled={!dirty || saving || loading || Boolean(errorMessage) || saveConflict}
      onClick={() => void saveDraft()}
    >
      <MaterialSymbolIcon name="save" />
    </MeltActionButton>
    {#if markdown && !loading && !errorMessage}
      <div class="stack-text-editor-mode" aria-label="Markdown view">
        <button type="button" aria-pressed={editorMode === 'preview'} on:click={() => setEditorMode('preview')}>Preview</button>
        <button type="button" aria-pressed={editorMode === 'edit'} on:click={() => setEditorMode('edit')}>Edit</button>
      </div>
    {/if}
  </header>

  <div class="stack-text-editor-body">
    {#if saveConflict || saveErrorMessage || reloadErrorMessage}
      <div class="stack-text-editor-feedback" role="alert">
        {#if saveConflict}
          <span>The file changed on disk. Your draft is preserved; reload the disk version only if you want to discard it.</span>
        {:else if saveErrorMessage}
          <span>{saveErrorMessage}</span>
        {/if}
        {#if reloadErrorMessage}<span>{reloadErrorMessage}</span>{/if}
        {#if saveConflict}
          <MeltActionButton class="stack-text-editor-reload" ariaLabel="Reload disk version and discard draft" onClick={requestConflictReload}>
            Reload disk version
          </MeltActionButton>
        {/if}
      </div>
    {/if}

    {#if loading}
      <div class="stack-text-editor-state surface-state info" role="status">Loading file…</div>
    {:else if errorMessage}
      <div class="stack-text-editor-state surface-state error" role="alert">{errorMessage}</div>
    {:else}
      {#if markdown && editorMode === 'preview'}
        <div class="stack-markdown-scroll"><StackMarkdownPreview source={draft} /></div>
      {:else}<div class="stack-text-editor-field">
        <span class="stack-text-editor-label">File contents</span>
        <div
          bind:this={editorHost}
          class="stack-text-editor-host"
          role="group"
          aria-label="File contents"
        ></div>
      </div>{/if}
    {/if}
  </div>

  {#if reloadConfirmation}
    <StackConfirmDialog bind:this={reloadDialog} title="Discard draft and reload?" message="Your in-memory draft will be discarded and replaced with the current disk version." confirmLabel="Discard and reload" tone="danger" initialFocus="cancel" dismissOnBackdrop={false} onCancel={cancelConflictReload} onConfirm={() => void confirmConflictReload()} />
  {/if}
</section>

<style>
  .stack-text-editor {
    background: var(--js-color-surface-sunken);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-md);
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-height: 0;
    overflow: hidden;
  }

  .stack-text-editor-header {
    align-items: center;
    background: var(--js-bg-surface);
    border-bottom: 1px solid var(--js-color-border);
    display: grid;
    gap: var(--js-space-2);
    grid-template-columns: auto minmax(0, 1fr) auto auto auto;
    padding: var(--js-space-2) var(--js-space-3);
  }

  :global(.stack-text-editor-close) {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: var(--js-radius-xs);
    color: var(--js-color-text);
    display: inline-flex;
    height: 1.75rem;
    justify-content: center;
    width: 1.75rem;
  }

  :global(.stack-text-editor-close:hover),
  :global(.stack-text-editor-close:focus-visible) {
    background: var(--js-color-accent-soft);
    box-shadow: var(--js-focus-ring);
  }

  :global(.stack-text-editor-save) {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: var(--js-radius-xs);
    color: var(--js-color-text);
    display: inline-flex;
    height: 1.75rem;
    justify-content: center;
    width: 1.75rem;
  }

  :global(.stack-text-editor-save:hover:not(:disabled)),
  :global(.stack-text-editor-save:focus-visible) {
    background: var(--js-color-accent-soft);
    box-shadow: var(--js-focus-ring);
  }

  :global(.stack-text-editor-save:disabled) { color: var(--js-color-text-muted); cursor: not-allowed; }

  .stack-text-editor-context { display: grid; min-width: 0; }
  .stack-text-editor-context strong { color: var(--js-color-text-strong); font-size: 0.82rem; }
  .stack-text-editor-context span { color: var(--js-color-text-muted); font-size: 0.66rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .stack-text-editor-draft-state {
    border: 1px solid var(--js-color-border);
    border-radius: 999px;
    color: var(--js-color-text-muted);
    font-size: 0.64rem;
    padding: 0.18rem 0.45rem;
  }
  .stack-text-editor-draft-state.dirty { border-color: var(--js-color-accent-border); color: var(--js-color-text-strong); }
  .stack-text-editor-mode { background: var(--js-color-surface-sunken); border: 1px solid var(--js-color-border); border-radius: 999px; display: flex; padding: 2px; }
  .stack-text-editor-mode button { background: transparent; border: 0; border-radius: 999px; color: var(--js-color-text-muted); cursor: pointer; font: inherit; font-size: 0.68rem; padding: 0.28rem 0.65rem; }
  .stack-text-editor-mode button[aria-pressed='true'] { background: var(--js-color-accent-soft); box-shadow: inset 0 0 0 1px var(--js-color-accent-border); color: var(--js-color-text-strong); }
  .stack-text-editor-mode button:focus-visible { box-shadow: var(--js-focus-ring); outline: none; }

  .stack-text-editor-body {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-height: 0;
    overflow: hidden;
  }

  .stack-text-editor-feedback {
    align-items: center;
    background: var(--js-color-accent-soft);
    border-bottom: 1px solid var(--js-color-accent-border);
    color: var(--js-color-text);
    display: flex;
    font-size: 0.7rem;
    gap: var(--js-space-2);
    justify-content: space-between;
    margin: 0;
    padding: 0.38rem var(--js-space-3);
  }

  :global(.stack-text-editor-reload) {
    background: var(--js-bg-control);
    border: 1px solid var(--js-color-accent-border);
    border-radius: var(--js-radius-xs);
    color: var(--js-color-text);
    flex: none;
    font: inherit;
    padding: 0.2rem 0.45rem;
  }

  :global(.stack-text-editor-reload:hover),
  :global(.stack-text-editor-reload:focus-visible) {
    box-shadow: var(--js-focus-ring);
    outline: none;
  }

  .stack-text-editor-field { display: grid; min-height: 0; overflow: hidden; }
  .stack-text-editor-label { height: 1px; overflow: hidden; position: absolute; width: 1px; clip: rect(0 0 0 0); }
  .stack-text-editor-host {
    box-sizing: border-box;
    height: 100%;
    min-height: 0;
    overflow: hidden;
    width: 100%;
  }
  .stack-text-editor-state { align-self: center; justify-self: center; }
  .stack-markdown-scroll { background: var(--js-bg-surface); min-height: 0; overflow: auto; }
  @media (max-width: 42rem) { .stack-text-editor-header { grid-template-columns: auto minmax(0, 1fr) auto; } .stack-text-editor-draft-state { display: none; } }
</style>
