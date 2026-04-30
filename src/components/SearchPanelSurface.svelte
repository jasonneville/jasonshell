<script lang="ts">
  import './SearchPanelSurface.css';
  import { emit } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount, tick } from 'svelte';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import {
    getSearchPanelPayload,
    readCenteredSearchPanelSize,
    resizeSearchPanel,
    SEARCH_PANEL_ACTIVATE_EVENT,
    SEARCH_PANEL_INTERACTION_EVENT,
    SEARCH_PANEL_KEY_EVENT,
    SEARCH_PANEL_PIN_FOLDER_EVENT,
    SEARCH_PANEL_QUERY_EVENT,
    SEARCH_PANEL_SELECT_EVENT,
    SEARCH_PANEL_UPDATE_EVENT,
    writeCenteredSearchPanelSize,
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
    buildVisibleSearchRows,
    nextSearchPanelFallbackDelay,
    nextVisibleRowIndex,
    selectedVisibleRowIndex,
    searchResultActionHints,
    shouldContinueSearchPanelFallbackPolling
  } from '../features/search/searchUxState';

  let panelState = defaultSearchPanelViewState;
  let resultRows: Array<HTMLDivElement | undefined> = [];
  let panelElement: HTMLElement | null = null;
  let queryInput: HTMLInputElement | null = null;
  let lastRevealedSelection = '';
  let fallbackTimer: number | null = null;
  let fallbackAttempt = 0;
  let fallbackGeneration = 0;
  let resizeDrag: {
    pointerId: number;
    startX: number;
    startY: number;
    startWidth: number;
    startHeight: number;
  } | null = null;
  $: query = panelState.query;
  $: results = panelState.results;
  $: selectedIndex = panelState.selectedIndex;
  $: statusMessage = panelState.statusMessage;
  $: presentation = panelState.presentation;
  $: visibleRows = buildVisibleSearchRows(results);
  $: selectedRowIndex = selectedVisibleRowIndex(visibleRows, selectedIndex);
  $: selectedRow = selectedRowIndex >= 0 ? visibleRows[selectedRowIndex] : null;
  $: void revealSelectedResult(selectedRowIndex, visibleRows.length);

  function applyPayload(payload: SearchPanelPayload | null) {
    panelState = applySearchPanelPayload(panelState, payload);
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    void getCurrentWindow().listen<SearchPanelPayload>(SEARCH_PANEL_UPDATE_EVENT, (event) => {
      fallbackGeneration += 1;
      stopFallbackPolling();
      fallbackAttempt = 0;
      applyPayload(event.payload);
      if (event.payload.presentation === 'centered') {
        void focusQueryInput();
      }
      scheduleFallbackPoll();
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    scheduleFallbackPoll(0);

    return () => {
      fallbackGeneration += 1;
      stopFallbackPolling();
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });

  function scheduleFallbackPoll(delay: number | null = nextSearchPanelFallbackDelay(fallbackAttempt)) {
    if (delay === null || fallbackTimer !== null) {
      return;
    }
    fallbackTimer = window.setTimeout(() => {
      fallbackTimer = null;
      const attempt = fallbackAttempt;
      const generation = fallbackGeneration;
      fallbackAttempt += 1;
      void getSearchPanelPayload().then((payload) => {
        if (generation !== fallbackGeneration) {
          return;
        }
        applyPayload(payload);
        if (
          shouldContinueSearchPanelFallbackPolling(
            attempt,
            Boolean(payload),
            Boolean(panelState.query || panelState.results.length)
          )
        ) {
          scheduleFallbackPoll();
        }
      }).catch(() => {
        if (generation !== fallbackGeneration) {
          return;
        }
        if (
          shouldContinueSearchPanelFallbackPolling(
            attempt,
            false,
            Boolean(panelState.query || panelState.results.length)
          )
        ) {
          scheduleFallbackPoll();
        }
      });
    }, delay);
  }

  function stopFallbackPolling() {
    if (fallbackTimer !== null) {
      window.clearTimeout(fallbackTimer);
      fallbackTimer = null;
    }
  }

  function activateResult(result: SearchPanelResult) {
    markPanelInteraction();
    void emit(SEARCH_PANEL_ACTIVATE_EVENT, result.id);
  }

  function updateQuery(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value;
    announcePanelInteraction();
    void emit(SEARCH_PANEL_QUERY_EVENT, value);
  }

  function handleQueryKeydown(event: KeyboardEvent) {
    if (!['ArrowDown', 'ArrowUp', 'Enter', 'Escape'].includes(event.key)) {
      return;
    }
    event.preventDefault();
    announcePanelInteraction();
    if (event.key === 'ArrowDown') {
      selectVisibleOffset(1);
      return;
    }
    if (event.key === 'ArrowUp') {
      selectVisibleOffset(-1);
      return;
    }
    if (event.key === 'Enter') {
      if (selectedRow) {
        activateResult(selectedRow.result);
        return;
      }
    }
    void emit(SEARCH_PANEL_KEY_EVENT, event.key);
  }

  function selectResult(result: SearchPanelResult) {
    markPanelInteraction();
    void emit(SEARCH_PANEL_SELECT_EVENT, result.id);
  }

  function markPanelInteraction() {
    panelElement?.focus({ preventScroll: true });
    announcePanelInteraction();
  }

  function announcePanelInteraction() {
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

  function selectVisibleOffset(direction: -1 | 1) {
    const nextIndex = nextVisibleRowIndex(visibleRows, selectedRowIndex, direction);
    if (nextIndex < 0) {
      return;
    }
    selectResult(visibleRows[nextIndex].result);
  }

  function startFolderDrag(event: DragEvent, result: SearchPanelResult) {
    if (result.kind !== 'folder' || !result.path || !event.dataTransfer) {
      return;
    }

    setFolderDragPayload(event.dataTransfer, [result.path], 'copy');
  }

  function trackVisibleRow(node: HTMLDivElement, index: number) {
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

  async function focusQueryInput() {
    await tick();
    queryInput?.focus({ preventScroll: true });
  }

  function beginResize(event: PointerEvent) {
    if (presentation !== 'centered') {
      return;
    }
    event.preventDefault();
    const size = readCenteredSearchPanelSize();
    resizeDrag = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      startWidth: size.width,
      startHeight: size.height
    };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function updateResize(event: PointerEvent) {
    if (!resizeDrag || resizeDrag.pointerId !== event.pointerId) {
      return;
    }
    const size = writeCenteredSearchPanelSize({
      width: resizeDrag.startWidth + event.clientX - resizeDrag.startX,
      height: resizeDrag.startHeight + event.clientY - resizeDrag.startY
    });
    void resizeSearchPanel(size).catch(() => undefined);
  }

  function endResize(event: PointerEvent) {
    if (!resizeDrag || resizeDrag.pointerId !== event.pointerId) {
      return;
    }
    updateResize(event);
    resizeDrag = null;
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
    {#if presentation === 'centered'}
      <input
        bind:this={queryInput}
        aria-label="Search Everything"
        autocomplete="off"
        class="search-panel-query"
        placeholder="Search Everything"
        value={query}
        on:input={updateQuery}
        on:keydown={handleQueryKeydown}
      />
    {:else}
      <strong>Search</strong>
      <span>{query || 'Everything files and folders'}</span>
    {/if}
    <kbd>Enter opens</kbd>
  </header>

  {#if results.length}
    <div class="result-list" role="listbox" tabindex="0" aria-label="Search results" aria-activedescendant={selectedRow?.domId}>
      {#each visibleRows as row, index (row.rowKey)}
        {@const result = row.result}
        {@const hints = searchResultActionHints(result)}
        {#if row.showGroupLabel}
          <div class="result-group-label">{row.groupLabel}</div>
        {/if}
        <div
          id={row.domId}
          class:result-selected={index === selectedRowIndex}
          class="result-row"
          role="option"
          tabindex="0"
          draggable={result.kind === 'folder' && !!result.path}
          aria-selected={index === selectedRowIndex}
          use:trackVisibleRow={index}
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
              <MeltActionButton
                class="pin-folder"
                ariaLabel={`Pin ${result.title} to the top bar`}
                onClick={(event) => pinFolderResult(event, result)}
              >
                Pin
              </MeltActionButton>
            {:else if hints.secondary}
              <span class="result-kind">{hints.secondary}</span>
            {:else}
              <span class="result-kind">{result.kind}</span>
            {/if}
          </span>
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-state surface-state info" role="status">{statusMessage}</div>
  {/if}
  {#if presentation === 'centered'}
    <button
      type="button"
      class="search-resize-grip"
      aria-label="Resize search panel"
      on:pointerdown={beginResize}
      on:pointermove={updateResize}
      on:pointerup={endResize}
      on:pointercancel={endResize}
    ></button>
  {/if}
</section>
