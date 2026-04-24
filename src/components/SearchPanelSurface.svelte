<script lang="ts">
  import { emit } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount, tick } from 'svelte';
  import {
    getSearchPanelPayload,
    SEARCH_PANEL_ACTIVATE_EVENT,
    SEARCH_PANEL_PIN_FOLDER_EVENT,
    SEARCH_PANEL_SELECT_EVENT,
    SEARCH_PANEL_UPDATE_EVENT,
    type SearchPanelPayload,
    type SearchPanelResult
  } from '../lib/searchPanel';
  import {
    applySearchPanelPayload,
    defaultSearchPanelViewState,
    shouldRevealSelectedResult
  } from '../lib/searchPanelState';
  import { folderPathToUri, JASONSHELL_FOLDER_DRAG_TYPE } from '../lib/folderDrag';

  let panelState = defaultSearchPanelViewState;
  let resultRows: Array<HTMLDivElement | undefined> = [];
  let lastRevealedSelection = '';
  $: query = panelState.query;
  $: results = panelState.results;
  $: selectedIndex = panelState.selectedIndex;
  $: statusMessage = panelState.statusMessage;
  $: void revealSelectedResult(selectedIndex, results.length);

  function applyPayload(payload: SearchPanelPayload | null) {
    panelState = applySearchPanelPayload(panelState, payload);
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    void getCurrentWindow().listen<SearchPanelPayload>(SEARCH_PANEL_UPDATE_EVENT, (event) => {
      applyPayload(event.payload);
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void getSearchPanelPayload().then(applyPayload);
    const refreshTimer = window.setInterval(() => {
      void getSearchPanelPayload().then(applyPayload);
    }, 120);

    return () => {
      window.clearInterval(refreshTimer);
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });

  function activateResult(result: SearchPanelResult) {
    void emit(SEARCH_PANEL_ACTIVATE_EVENT, result.id);
  }

  function selectResult(result: SearchPanelResult) {
    void emit(SEARCH_PANEL_SELECT_EVENT, result.id);
  }

  function pinFolderResult(event: MouseEvent, result: SearchPanelResult) {
    event.preventDefault();
    event.stopPropagation();
    if (result.kind === 'folder' && result.path) {
      void emit(SEARCH_PANEL_PIN_FOLDER_EVENT, result.path);
    }
  }

  function handleResultKeydown(event: KeyboardEvent, result: SearchPanelResult) {
    if (event.key === 'Enter') {
      event.preventDefault();
      activateResult(result);
    } else if (event.key === ' ') {
      event.preventDefault();
      selectResult(result);
    }
  }

  function startFolderDrag(event: DragEvent, result: SearchPanelResult) {
    if (result.kind !== 'folder' || !result.path || !event.dataTransfer) {
      return;
    }

    event.dataTransfer.effectAllowed = 'copy';
    event.dataTransfer.setData(JASONSHELL_FOLDER_DRAG_TYPE, result.path);
    event.dataTransfer.setData('text/plain', result.path);
    event.dataTransfer.setData('text/uri-list', folderPathToUri(result.path));
  }

  function trackResultRow(node: HTMLDivElement, index: number) {
    resultRows[index] = node;

    return {
      update(nextIndex: number) {
        if (resultRows[index] === node) {
          resultRows[index] = undefined;
        }
        index = nextIndex;
        resultRows[index] = node;
      },
      destroy() {
        if (resultRows[index] === node) {
          resultRows[index] = undefined;
        }
      }
    };
  }

  async function revealSelectedResult(index: number, resultCount: number) {
    const revealKey = `${index}:${resultCount}`;
    if (lastRevealedSelection === revealKey || !shouldRevealSelectedResult(index, resultCount)) {
      return;
    }

    lastRevealedSelection = revealKey;
    await tick();
    resultRows[index]?.scrollIntoView({ block: 'nearest' });
  }
</script>

<section class="search-panel" aria-label="Search results">
  <header class="search-panel-header">
    <strong>Search</strong>
    <span>{query || 'Apps, windows, files, folders, commands'}</span>
  </header>

  {#if results.length}
    <div class="result-list" role="listbox" aria-label="Search results">
      {#each results as result, index (result.id)}
        <div
          class:result-selected={index === selectedIndex}
          class="result-row"
          role="option"
          tabindex="0"
          draggable={result.kind === 'folder' && !!result.path}
          aria-selected={index === selectedIndex}
          use:trackResultRow={index}
          on:click={() => selectResult(result)}
          on:dblclick={() => activateResult(result)}
          on:keydown={(event) => handleResultKeydown(event, result)}
          on:dragstart={(event) => startFolderDrag(event, result)}
        >
          <span class="result-icon" aria-hidden="true">
            {#if result.iconDataUrl}
              <img src={result.iconDataUrl} alt="" draggable="false" />
            {:else}
              {result.kind.slice(0, 1).toUpperCase()}
            {/if}
          </span>
          <span class="result-copy">
            <strong>{result.title}</strong>
            <small>{result.subtitle}</small>
          </span>
          {#if result.kind === 'folder' && result.path}
            <span class="result-actions">
              <span class="result-kind">{result.kind}</span>
              <button
                class="pin-folder"
                type="button"
                on:click={(event) => pinFolderResult(event, result)}
              >
                Pin
              </button>
            </span>
          {:else}
            <span class="result-kind">{result.kind}</span>
          {/if}
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-state">{statusMessage}</div>
  {/if}
</section>

<style>
  .search-panel {
    background:
      linear-gradient(180deg, rgba(23, 28, 39, 0.98), rgba(11, 14, 22, 0.98));
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 0.45rem;
    box-shadow: 0 22px 46px rgba(0, 0, 0, 0.44);
    color: #f0f4ff;
    height: 100%;
    overflow: hidden;
    padding: 0.7rem;
    width: 100%;
  }

  .search-panel-header {
    align-items: baseline;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    gap: 0.55rem;
    padding: 0 0.15rem 0.55rem;
  }

  .search-panel-header strong {
    font-size: 0.9rem;
    font-weight: 800;
  }

  .search-panel-header span {
    color: rgba(221, 229, 250, 0.62);
    font-size: 0.72rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result-list {
    display: grid;
    gap: 0.35rem;
    max-height: calc(100% - 2rem);
    overflow-y: auto;
    padding-top: 0.55rem;
    scrollbar-color: rgba(124, 160, 255, 0.45) rgba(255, 255, 255, 0.06);
  }

  .result-row {
    align-items: center;
    border: 1px solid transparent;
    border-radius: 0.38rem;
    display: grid;
    gap: 0.55rem;
    grid-template-columns: 1.8rem minmax(0, 1fr) auto;
    min-height: 2.55rem;
    padding: 0.34rem 0.45rem;
    text-align: left;
    user-select: none;
  }

  .result-row:hover,
  .result-selected {
    background: rgba(77, 124, 254, 0.15);
    border-color: rgba(124, 160, 255, 0.3);
  }

  .result-icon {
    align-items: center;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 0.3rem;
    color: rgba(228, 235, 255, 0.74);
    display: grid;
    font-size: 0.72rem;
    font-weight: 800;
    height: 1.8rem;
    justify-items: center;
    width: 1.8rem;
  }

  .result-icon img {
    height: 1.1rem;
    width: 1.1rem;
  }

  .result-copy {
    display: grid;
    min-width: 0;
  }

  .result-copy strong,
  .result-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result-copy strong {
    font-size: 0.76rem;
    font-weight: 750;
  }

  .result-copy small,
  .result-kind,
  .empty-state {
    color: rgba(218, 226, 248, 0.58);
    font-size: 0.62rem;
  }

  .result-kind {
    text-transform: uppercase;
  }

  .result-actions {
    align-items: center;
    display: flex;
    gap: 0.35rem;
  }

  .pin-folder {
    background: rgba(255, 255, 255, 0.07);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 0.25rem;
    color: #eef3ff;
    font-size: 0.58rem;
    font-weight: 800;
    padding: 0.12rem 0.36rem;
  }

  .pin-folder:hover {
    background: rgba(77, 124, 254, 0.18);
    border-color: rgba(124, 160, 255, 0.36);
  }

  .empty-state {
    display: grid;
    height: calc(100% - 2rem);
    place-items: center;
  }
</style>
