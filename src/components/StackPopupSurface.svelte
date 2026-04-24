<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import {
    copyStackItems,
    getStackPopupRequest,
    hideStackPopup,
    listStackFolder,
    openStackItem,
    pasteStackItems,
    renameStackItem,
    STACK_POPUP_OPEN_EVENT,
    type StackEntry
  } from '../lib/stackPopup';
  import {
    applyStackEntries,
    canNavigateStackBack,
    canNavigateStackForward,
    defaultStackPopupViewState,
    formatStackSize,
    navigateStackHistory,
    openStackFolder,
    selectedStackEntry,
    selectStackEntry
  } from '../lib/stackPopupState';

  let stackState = defaultStackPopupViewState;
  let loadingPath: string | null = null;
  let errorMessage = '';

  $: currentPath = stackState.currentPath;
  $: entries = stackState.entries;
  $: selectedEntry = selectedStackEntry(stackState);
  $: canGoBack = canNavigateStackBack(stackState);
  $: canGoForward = canNavigateStackForward(stackState);

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    void getCurrentWindow().listen<string | { path: string }>(STACK_POPUP_OPEN_EVENT, (event) => {
      const path = typeof event.payload === 'string' ? event.payload : event.payload.path;
      void openFolder(path);
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void getStackPopupRequest().then((request) => {
      if (request?.path) {
        void openFolder(request.path);
      }
    });

    return () => {
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });

  async function openFolder(folderPath: string) {
    stackState = openStackFolder(stackState, folderPath);
    await loadFolder(stackState.currentPath);
  }

  async function loadFolder(folderPath: string) {
    if (!folderPath) {
      return;
    }

    loadingPath = folderPath;
    errorMessage = '';
    try {
      const loadedEntries = await listStackFolder(folderPath);
      stackState = applyStackEntries(stackState, folderPath, loadedEntries);
    } catch (error) {
      console.error('Failed to load stack folder', error);
      if (loadingPath === folderPath) {
        errorMessage = 'Folder unavailable';
      }
    } finally {
      if (loadingPath === folderPath) {
        loadingPath = null;
      }
    }
  }

  async function navigateHistory(direction: -1 | 1) {
    stackState = navigateStackHistory(stackState, direction);
    await loadFolder(stackState.currentPath);
  }

  async function activateEntry(entry: StackEntry) {
    if (entry.entryType === 'Folder') {
      await openFolder(entry.path);
    } else {
      await openStackItem(entry.path);
    }
  }

  async function copySelected(cut: boolean) {
    if (!selectedEntry) {
      return;
    }

    await copyStackItems([selectedEntry.path], cut);
  }

  async function pasteIntoCurrentFolder() {
    if (!currentPath) {
      return;
    }

    try {
      const loadedEntries = await pasteStackItems(currentPath);
      stackState = applyStackEntries(stackState, currentPath, loadedEntries);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to paste stack items', error);
      errorMessage = 'Paste unavailable';
    }
  }

  async function renameSelected() {
    if (!selectedEntry) {
      return;
    }

    const nextName = window.prompt('Rename stack item', selectedEntry.name);
    if (!nextName || nextName === selectedEntry.name) {
      return;
    }

    try {
      const renamedEntry = await renameStackItem(selectedEntry.path, nextName);
      const loadedEntries = await listStackFolder(currentPath);
      stackState = selectStackEntry(
        applyStackEntries(stackState, currentPath, loadedEntries),
        renamedEntry.path
      );
      errorMessage = '';
    } catch (error) {
      console.error('Failed to rename stack item', error);
      errorMessage = 'Rename unavailable';
    }
  }

  function formatModified(modifiedMs: number | null | undefined) {
    if (!modifiedMs) {
      return '';
    }

    return new Intl.DateTimeFormat(undefined, {
      dateStyle: 'short',
      timeStyle: 'short'
    }).format(new Date(modifiedMs));
  }

  function breadcrumbSegments(path: string) {
    if (!path) return [];
    const normalized = path.replace(/\//g, '\\\\');
    const segments: { name: string; path: string }[] = [];
    if (/^[a-zA-Z]:/.test(normalized)) {
      const drive = normalized.slice(0, 2);
      let acc = drive + '\\\\';
      segments.push({ name: drive, path: acc });
      const rest = normalized.slice(2).split(/\\\\+/).filter(Boolean);
      for (const seg of rest) {
        if (acc.endsWith('\\\\')) acc = acc + seg;
        else acc = acc + '\\\\' + seg;
        segments.push({ name: seg, path: acc });
      }
    } else {
      const parts = path.split(/[\\\\/]+/).filter(Boolean);
      let acc = '';
      for (const seg of parts) {
        acc = acc ? `${acc}/${seg}` : `/${seg}`;
        segments.push({ name: seg, path: acc });
      }
    }
    return segments;
  }

  function selectAdjacentEntry(direction: 1 | -1) {
    const idx = stackState.entries.findIndex((e) => e.path === stackState.selectedPath);
    let next = idx;
    if (next === -1) {
      next = direction === 1 ? 0 : stackState.entries.length - 1;
    } else {
      next = Math.max(0, Math.min(stackState.entries.length - 1, idx + direction));
    }
    const entry = stackState.entries[next];
    if (entry) {
      stackState = selectStackEntry(stackState, entry.path);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      void hideStackPopup();
    } else if (event.key === 'Enter' && selectedEntry) {
      event.preventDefault();
      void activateEntry(selectedEntry);
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      selectAdjacentEntry(1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      selectAdjacentEntry(-1);
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'c') {
      event.preventDefault();
      void copySelected(false);
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'x') {
      event.preventDefault();
      void copySelected(true);
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'v') {
      event.preventDefault();
      void pasteIntoCurrentFolder();
    } else if (event.key === 'F2') {
      event.preventDefault();
      void renameSelected();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<section class="stack-popup" aria-label="Stack browser">
  <header class="stack-toolbar">
    <div class="stack-path" title={currentPath}>
      {#if currentPath}
        <nav class="breadcrumbs" aria-label="Path breadcrumbs">
          {#each breadcrumbSegments(currentPath) as crumb, i (crumb.path)}
            <button type="button" class="crumb" on:click={() => void openFolder(crumb.path)}>{crumb.name}</button>
            {#if i < breadcrumbSegments(currentPath).length - 1}
              <span class="crumb-sep">/</span>
            {/if}
          {/each}
        </nav>
      {:else}
        Stack Browser
      {/if}
    </div>
    <div class="stack-actions">
      <button type="button" disabled={!canGoBack} on:click={() => void navigateHistory(-1)}>Back</button>
      <button type="button" disabled={!canGoForward} on:click={() => void navigateHistory(1)}>Forward</button>
      <button type="button" on:click={() => void loadFolder(currentPath)}>Refresh</button>
      <button type="button" disabled={!selectedEntry} on:click={() => void copySelected(false)}>Copy</button>
      <button type="button" disabled={!selectedEntry} on:click={() => void copySelected(true)}>Cut</button>
      <button type="button" disabled={!currentPath} on:click={() => void pasteIntoCurrentFolder()}>Paste</button>
      <button type="button" disabled={!selectedEntry} on:click={() => void renameSelected()}>Rename</button>
    </div>
  </header>

  <div class="stack-status">
    <span>{errorMessage || stackState.statusMessage}</span>
    {#if loadingPath}
      <span>Loading...</span>
    {/if}
  </div>

  <div class="details-table" role="table" aria-label="Folder details">
    <div class="details-header" role="row">
      <span role="columnheader">Name</span>
      <span role="columnheader">Type</span>
      <span role="columnheader">Size</span>
      <span role="columnheader">Modified</span>
    </div>

    {#if entries.length}
      <div class="details-body">
        {#each entries as entry (entry.id)}
          <button
            class:selected={entry.path === stackState.selectedPath}
            type="button"
            role="row"
            on:click={() => stackState = selectStackEntry(stackState, entry.path)}
            on:dblclick={() => void activateEntry(entry)}
          >
            <span role="cell" title={entry.path}>{entry.name}</span>
            <span role="cell">{entry.typeLabel}</span>
            <span role="cell">{formatStackSize(entry.size)}</span>
            <span role="cell">{formatModified(entry.modifiedMs)}</span>
          </button>
        {/each}
      </div>
    {:else}
      <div class="empty-stack">{loadingPath ? 'Loading folder...' : stackState.statusMessage}</div>
    {/if}
  </div>
</section>

<style>
  .stack-popup {
    background: linear-gradient(180deg, rgba(23, 28, 39, 0.98), rgba(10, 13, 21, 0.98));
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 0.45rem;
    box-shadow: 0 22px 46px rgba(0, 0, 0, 0.44);
    color: #f0f4ff;
    display: grid;
    grid-template-rows: auto auto 1fr;
    height: 100%;
    overflow: hidden;
    padding: 0.65rem;
    width: 100%;
  }

  .stack-toolbar {
    align-items: center;
    display: grid;
    gap: 0.65rem;
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .stack-path {
    color: rgba(240, 244, 255, 0.92);
    font-size: 0.82rem;
    font-weight: 750;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stack-actions {
    display: flex;
    gap: 0.3rem;
  }

  .stack-actions button {
    background: rgba(255, 255, 255, 0.07);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 0.28rem;
    color: #eef3ff;
    font-size: 0.62rem;
    font-weight: 750;
    min-height: 1.45rem;
    padding: 0 0.42rem;
  }

  .stack-actions button:disabled {
    color: rgba(218, 226, 248, 0.32);
  }

  .stack-actions button:not(:disabled):hover {
    background: rgba(77, 124, 254, 0.18);
    border-color: rgba(124, 160, 255, 0.35);
  }

  .stack-status {
    color: rgba(218, 226, 248, 0.58);
    display: flex;
    font-size: 0.62rem;
    justify-content: space-between;
    min-height: 1.45rem;
    padding: 0.35rem 0.05rem 0.4rem;
  }

  .details-table {
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 0.38rem;
    display: grid;
    grid-template-rows: auto 1fr;
    min-height: 0;
    overflow: hidden;
  }

  .details-header,
  .details-body button {
    display: grid;
    grid-template-columns: minmax(10rem, 1fr) 5.5rem 5rem 8.5rem;
  }

  .details-header {
    background: rgba(255, 255, 255, 0.05);
    color: rgba(218, 226, 248, 0.58);
    font-size: 0.58rem;
    font-weight: 800;
    min-height: 1.65rem;
    text-transform: uppercase;
  }

  .details-header span,
  .details-body button span {
    align-content: center;
    min-width: 0;
    overflow: hidden;
    padding: 0 0.55rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .details-body {
    min-height: 0;
    overflow-y: auto;
    scrollbar-color: rgba(124, 160, 255, 0.45) rgba(255, 255, 255, 0.06);
  }

  .details-body button {
    background: transparent;
    border: 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.045);
    color: rgba(240, 244, 255, 0.9);
    font: inherit;
    font-size: 0.7rem;
    min-height: 1.9rem;
    text-align: left;
    width: 100%;
  }

  .details-body button:hover,
  .details-body button.selected {
    background: rgba(77, 124, 254, 0.16);
  }

  .empty-stack {
    align-items: center;
    color: rgba(218, 226, 248, 0.58);
    display: grid;
    font-size: 0.72rem;
    justify-items: center;
  }

  .breadcrumbs {
    display: flex;
    gap: 0.25rem;
    align-items: center;
    flex-wrap: nowrap;
    overflow: hidden;
  }

  .breadcrumbs .crumb {
    background: transparent;
    border: 0;
    color: rgba(240, 244, 255, 0.9);
    font-weight: 700;
    padding: 0 0.25rem;
    text-overflow: ellipsis;
    white-space: nowrap;
    overflow: hidden;
  }

  .breadcrumbs .crumb-sep {
    color: rgba(218, 226, 248, 0.46);
    padding: 0 0.12rem;
  }

  .stack-actions button:nth-child(3) {
    background: rgba(255, 255, 255, 0.04);
  }
</style>
