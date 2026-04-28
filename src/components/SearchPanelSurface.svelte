<script lang="ts">
  import './SearchPanelSurface.css';
  import { emit } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount, tick } from 'svelte';
  import {
    getSearchPanelPayload,
    SEARCH_PANEL_ACTIVATE_EVENT,
    SEARCH_PANEL_INTERACTION_EVENT,
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
  import { setFolderDragPayload } from '../lib/folderDrag';
  import {
    groupSearchResults,
    searchResultActionHints
  } from '../features/search/searchUxState';

  let panelState = defaultSearchPanelViewState;
  let resultRows: Array<HTMLDivElement | undefined> = [];
  let panelElement: HTMLElement | null = null;
  let lastRevealedSelection = '';
  $: query = panelState.query;
  $: results = panelState.results;
  $: selectedIndex = panelState.selectedIndex;
  $: statusMessage = panelState.statusMessage;
  $: resultGroups = groupSearchResults(results);
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
    markPanelInteraction();
    void emit(SEARCH_PANEL_ACTIVATE_EVENT, result.id);
  }

  function selectResult(result: SearchPanelResult) {
    markPanelInteraction();
    void emit(SEARCH_PANEL_SELECT_EVENT, result.id);
  }

  function markPanelInteraction() {
    panelElement?.focus({ preventScroll: true });
    void emit(SEARCH_PANEL_INTERACTION_EVENT, null);
  }

  function pinFolderResult(event: MouseEvent, result: SearchPanelResult) {
    event.preventDefault();
    event.stopPropagation();
    markPanelInteraction();
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

    setFolderDragPayload(event.dataTransfer, [result.path], 'copy');
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

<svelte:window on:mousedown={markPanelInteraction} />

<section
  class="search-panel"
  aria-label="Search results"
  tabindex="-1"
  bind:this={panelElement}
>
  <header class="search-panel-header">
    <strong>Search</strong>
    <span>{query || 'Apps, windows, places, files, commands'}</span>
    <kbd>Enter opens</kbd>
  </header>

  {#if results.length}
    <div class="result-list" role="listbox" tabindex="0" aria-label="Search results" aria-activedescendant={results[selectedIndex]?.id ? `search-result-${selectedIndex}` : undefined}>
      {#each resultGroups as group (group.id)}
        <div class="result-group" role="group" aria-label={group.label}>
          <div class="result-group-label">{group.label}</div>
          {#each group.items as item (item.result.id)}
            {@const result = item.result}
            {@const index = item.index}
            {@const hints = searchResultActionHints(result)}
            <div
              id={`search-result-${index}`}
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
              <span class="result-actions">
                <span class="result-action-primary">{hints.primary}</span>
                {#if hints.secondary === 'Pin' && result.kind === 'folder' && result.path}
                  <button
                    class="pin-folder"
                    type="button"
                    aria-label={`Pin ${result.title} to the top bar`}
                    on:click={(event) => pinFolderResult(event, result)}
                  >
                    Pin
                  </button>
                {:else if hints.secondary}
                  <span class="result-kind">{hints.secondary}</span>
                {:else}
                  <span class="result-kind">{result.kind}</span>
                {/if}
              </span>
            </div>
          {/each}
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-state surface-state info" role="status">{statusMessage}</div>
  {/if}
</section>
