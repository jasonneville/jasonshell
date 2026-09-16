<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import {
    createStackTextEditorAdapter,
    type StackTextEditorAdapter
  } from '../features/stack-browser/stackTextEditorAdapter';
  import { readStackBasicTextFile } from '../lib/stackPopup';
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import MeltActionButton from './melt/MeltActionButton.svelte';

  export let path: string;
  export let onDirtyChange: (dirty: boolean) => void;
  export let onDismiss: (dirty: boolean) => void;

  let editorHost: HTMLDivElement | null = null;
  let editorAdapter: StackTextEditorAdapter | null = null;
  let loading = true;
  let errorMessage = '';
  let initialContent = '';
  let draft = '';
  let requestSequence = 0;
  let disposed = false;

  $: filename = path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
  $: dirty = draft !== initialContent;
  $: onDirtyChange(dirty);
  $: void loadFile(path);

  function destroyEditor() {
    editorAdapter?.destroy();
    editorAdapter = null;
  }

  async function loadFile(filePath: string) {
    const sequence = ++requestSequence;
    destroyEditor();
    loading = true;
    errorMessage = '';
    initialContent = '';
    draft = '';

    try {
      const result = await readStackBasicTextFile(filePath);
      if (disposed || sequence !== requestSequence || filePath !== path) return;
      initialContent = result.content;
      draft = result.content;
      loading = false;
      await tick();
      if (disposed || sequence !== requestSequence || filePath !== path) return;
      if (!editorHost) return;
      const mountedAdapter = createStackTextEditorAdapter({
        parent: editorHost,
        content: result.content,
        onChange: (currentDraft, currentDirty) => {
          draft = currentDraft;
          dirty = currentDirty;
        },
        onDismiss: (currentDirty) => onDismiss(currentDirty)
      });
      if (disposed || sequence !== requestSequence || filePath !== path) {
        mountedAdapter.destroy();
        return;
      }
      editorAdapter = mountedAdapter;
      await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
      if (disposed || sequence !== requestSequence || filePath !== path || editorAdapter !== mountedAdapter) return;
      mountedAdapter.focusAtStart();
    } catch (error) {
      if (disposed || sequence !== requestSequence || filePath !== path) return;
      loading = false;
      errorMessage = error instanceof Error && error.message ? error.message : String(error);
    }
  }

  onDestroy(() => {
    disposed = true;
    requestSequence += 1;
    destroyEditor();
  });
</script>

<section class="stack-text-editor" aria-labelledby="stack-text-editor-title" aria-busy={loading}>
  <header class="stack-text-editor-header">
    <MeltActionButton class="stack-text-editor-close" ariaLabel="Close editor and return to folder" onClick={() => onDismiss(dirty)}>
      <MaterialSymbolIcon name="arrow_back" />
    </MeltActionButton>
    <div class="stack-text-editor-context">
      <strong id="stack-text-editor-title">{filename}</strong>
      <span title={path}>{path}</span>
    </div>
    <span class:dirty class="stack-text-editor-draft-state" role="status" aria-live="polite">
      {dirty ? 'Draft changed' : 'Draft unchanged'}
    </span>
  </header>

  <p class="stack-text-editor-notice">Draft only — saving is not available yet</p>

  {#if loading}
    <div class="stack-text-editor-state surface-state info" role="status">Loading file…</div>
  {:else if errorMessage}
    <div class="stack-text-editor-state surface-state error" role="alert">{errorMessage}</div>
  {:else}
    <div class="stack-text-editor-field">
      <span class="stack-text-editor-label">File contents</span>
      <div
        bind:this={editorHost}
        class="stack-text-editor-host"
        role="group"
        aria-label="File contents"
      ></div>
    </div>
  {/if}
</section>

<style>
  .stack-text-editor {
    background: var(--js-color-surface-sunken);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-md);
    display: grid;
    grid-template-rows: auto auto minmax(0, 1fr);
    min-height: 0;
    overflow: hidden;
  }

  .stack-text-editor-header {
    align-items: center;
    background: var(--js-bg-surface);
    border-bottom: 1px solid var(--js-color-border);
    display: grid;
    gap: var(--js-space-2);
    grid-template-columns: auto minmax(0, 1fr) auto;
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

  .stack-text-editor-notice {
    background: var(--js-color-accent-soft);
    border-bottom: 1px solid var(--js-color-accent-border);
    color: var(--js-color-text);
    font-size: 0.7rem;
    margin: 0;
    padding: 0.38rem var(--js-space-3);
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
</style>
