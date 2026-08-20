<script lang="ts">
  import './SearchPanelSurface.css';
  import { emitTo } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount, tick } from 'svelte';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import {
    getSearchPanelPayload,
    hideSearchPanel,
    readCenteredSearchPanelSize,
    resizeSearchPanel,
    SEARCH_PANEL_ACTIVATE_EVENT,
    SEARCH_PANEL_INTERACTION_EVENT,
    SEARCH_PANEL_KEY_EVENT,
    SEARCH_PANEL_EXPAND_GROUP_EVENT,
    SEARCH_PANEL_PIN_FOLDER_EVENT,
    SEARCH_PANEL_QUERY_EVENT,
    SEARCH_PANEL_SELECT_EVENT,
    SEARCH_PANEL_UPDATE_EVENT,
    writeCenteredSearchPanelSize,
    type SearchPanelPayload,
    type SearchPanelQueryPayload,
    type SearchPanelResult
  } from '../lib/searchPanel';
  import {
    applySearchPanelPayload,
    defaultSearchPanelViewState,
    shouldRevealSelectedResult
  } from '../lib/searchPanelState';
  import { setFolderDragPayload } from '../lib/folderDrag';
  import { topBarWebviewWindowEventTarget } from '../lib/topBarPins';
  import {
    buildVisibleSearchRows,
    nextSearchPanelFallbackDelay,
    nextVisibleRowIndex,
    selectedVisibleRowIndex,
    searchVisibleRowIdentity,
    searchResultActionHints,
    type SearchVisibleRow,
    shouldContinueSearchPanelFallbackPolling
  } from '../features/search/searchUxState';

  let panelState = defaultSearchPanelViewState;
  let optimisticQueryDraft: string | null = null;
  let resultRows: Array<HTMLDivElement | undefined> = [];
  let queryInput: HTMLInputElement | null = null;
  let lastRevealedSelection = '';
  let fallbackTimer: number | null = null;
  let fallbackAttempt = 0;
  let fallbackGeneration = 0;
  let queryInputSequence = 0;
  let resizeDrag: {
    pointerId: number;
    startX: number;
    startY: number;
    startWidth: number;
    startHeight: number;
  } | null = null;
  $: query = panelState.query;
  $: displayedQuery = optimisticQueryDraft ?? query;
  $: results = panelState.results;
  $: selectedIndex = panelState.selectedIndex;
  $: statusMessage = panelState.statusMessage;
  $: presentation = panelState.presentation;
  $: visibleRows = buildVisibleSearchRows(results);
  $: selectedRowIndex = selectedVisibleRowIndex(visibleRows, selectedIndex);
  $: selectedRow = selectedRowIndex >= 0 ? visibleRows[selectedRowIndex] : null;
  $: void revealSelectedResult(selectedRowIndex, visibleRows.length);
  $: visibleStatusMessage = searchPanelStatusMessage(statusMessage, results.length);
  const topBarTarget = topBarWebviewWindowEventTarget();

  function applyPayload(payload: SearchPanelPayload | null) {
    const previousQuery = panelState.query;
    panelState = applySearchPanelPayload(panelState, payload);
    if (panelState.query === '' || optimisticQueryDraft === panelState.query) {
      optimisticQueryDraft = null;
    }
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    const registerAsyncUnlistener = (registration: Promise<() => void>) => {
      void registration.then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlisteners.push(unlisten);
      });
    };
    registerAsyncUnlistener(getCurrentWindow().listen<SearchPanelPayload>(SEARCH_PANEL_UPDATE_EVENT, (event) => {
      fallbackGeneration += 1;
      stopFallbackPolling();
      fallbackAttempt = 0;
      applyPayload(event.payload);
      if (shouldFocusCenteredQueryInput()) {
        void focusQueryInput();
      }
      scheduleFallbackPoll();
    }));
    scheduleFallbackPoll(0);

    return () => {
      disposed = true;
      fallbackGeneration += 1;
      stopFallbackPolling();
      cancelQueuedQueryEmit();
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
        if (shouldFocusCenteredQueryInput()) {
          void focusQueryInput();
        }
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

  function activateRow(row: SearchVisibleRow) {
    markPanelInteraction();
    void emitTo(topBarTarget, SEARCH_PANEL_ACTIVATE_EVENT, searchVisibleRowIdentity(row));
  }

  function updateQuery(event: Event) {
    const value = (event.currentTarget as HTMLInputElement).value;
    optimisticQueryDraft = value;
    announcePanelInteraction();
    queueQueryEmit(value);
  }

  function hasCenteredSearchClearValue() {
    return Boolean(displayedQuery);
  }

  async function clearCenteredSearch() {
    optimisticQueryDraft = '';
    announcePanelInteraction();
    queueQueryEmit('');
    await tick();
    queryInput?.focus({ preventScroll: true });
  }

  function isCtrlSpaceHotkey(event: KeyboardEvent) {
    return event.code === 'Space' && event.ctrlKey && !event.altKey && !event.metaKey;
  }

  function closeCenteredPanelFromHotkey() {
    hideCenteredPanelImmediately();
  }

  function handleQueryKeydown(event: KeyboardEvent) {
    if (isCtrlSpaceHotkey(event)) {
      event.preventDefault();
      closeCenteredPanelFromHotkey();
      return;
    }
    if (isCtrlEnterHotkey(event)) {
      event.preventDefault();
      pinSelectedFolder();
      return;
    }
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
        activateRow(selectedRow);
        return;
      }
    }
    if (event.key === 'Escape' && presentation === 'centered') {
      hideCenteredPanelImmediately();
      return;
    }
    void emitTo(topBarTarget, SEARCH_PANEL_KEY_EVENT, event.key);
  }

  function selectRow(row: SearchVisibleRow) {
    markPanelInteraction();
    void emitTo(topBarTarget, SEARCH_PANEL_SELECT_EVENT, searchVisibleRowIdentity(row));
    void focusQueryInput();
  }

  function markPanelInteraction() {
    announcePanelInteraction();
  }

  function announcePanelInteraction() {
    void emitTo(topBarTarget, SEARCH_PANEL_INTERACTION_EVENT, null);
  }

  function isCtrlEnterHotkey(event: KeyboardEvent) {
    return event.key === 'Enter' && event.ctrlKey && !event.altKey && !event.metaKey;
  }

  function pinSelectedFolder() {
    if (!selectedRow?.result.path || selectedRow.result.kind !== 'folder') {
      return;
    }
    markPanelInteraction();
    void emitTo(topBarTarget, SEARCH_PANEL_PIN_FOLDER_EVENT, selectedRow.result.path);
  }

  function selectVisibleOffset(direction: -1 | 1) {
    const nextIndex = nextVisibleRowIndex(visibleRows, selectedRowIndex, direction);
    if (nextIndex < 0) {
      return;
    }
    selectRow(visibleRows[nextIndex]);
  }

  function handleOptionKeydown(event: KeyboardEvent, row: SearchVisibleRow) {
    if (event.key === 'Enter') {
      event.preventDefault();
      activateRow(row);
    }
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

  function shouldFocusCenteredQueryInput() {
    if (presentation !== 'centered' || !queryInput || document.activeElement === queryInput) {
      return false;
    }
    return !(document.activeElement instanceof HTMLElement && document.activeElement.closest('.search-panel'));
  }

  function hideCenteredPanelImmediately() {
    optimisticQueryDraft = null;
    cancelQueuedQueryEmit();
    queryInput?.blur();
    fallbackGeneration += 1;
    stopFallbackPolling();
    void hideSearchPanel().catch(() => undefined);
    announcePanelInteraction();
    void emitTo(topBarTarget, SEARCH_PANEL_KEY_EVENT, 'Escape');
  }

  function queueQueryEmit(value: string) {
    queryInputSequence += 1;
    const payload: SearchPanelQueryPayload = {
      query: value,
      inputSequence: queryInputSequence
    };
    void emitTo(topBarTarget, SEARCH_PANEL_QUERY_EVENT, payload);
  }

  function cancelQueuedQueryEmit() {
    // Query emits are per-event; cleanup hook stays for lifecycle symmetry.
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

  function highlightParts(text: string, indexes?: number[]) {
    const selected = new Set<number>();
    for (let index = 0; index < (indexes?.length ?? 0); index += 2) {
      const start = indexes?.[index] ?? 0;
      const length = indexes?.[index + 1] ?? 0;
      for (let offset = 0; offset < length; offset += 1) {
        selected.add(start + offset);
      }
    }
    return Array.from(text).map((character, index) => ({
      character,
      highlighted: selected.has(index)
    }));
  }

  function searchPanelStatusMessage(message: string, resultCount: number) {
    const trimmed = message.trim();
    if (!trimmed) {
      return '';
    }
    if (resultCount > 0 && (trimmed === 'Showing search results' || trimmed === 'Search is ready')) {
      return '';
    }
    return trimmed;
  }
</script>

<svelte:window on:mousedown={markPanelInteraction} />

<section
  class="search-panel"
  aria-label="Search results"
  tabindex="-1"
>
  <header class="search-panel-header">
    {#if presentation === 'centered'}
      <input
        bind:this={queryInput}
        aria-activedescendant={selectedRow?.domId}
        aria-autocomplete="list"
        aria-controls="search-results"
        aria-describedby="search-status"
        aria-expanded={true}
        role="combobox"
        aria-label="Search Everything"
        autocomplete="off"
        class="search-panel-query"
        placeholder="Search Everything"
        value={displayedQuery}
        on:input={updateQuery}
        on:keydown={handleQueryKeydown}
      />
      {#if presentation === 'centered' && hasCenteredSearchClearValue()}
        <MeltActionButton
          class="search-panel-clear-button"
          ariaLabel="Clear search"
          tooltip="Clear search"
          onClick={() => void clearCenteredSearch()}
        >
          ×
        </MeltActionButton>
      {/if}
    {:else}
      <strong>Search</strong>
      <span>{query || 'Everything files and folders'}</span>
    {/if}
    <kbd>Enter opens</kbd>
  </header>

  <div
    id="search-status"
    class="search-panel-status"
    role="status"
    aria-live="polite"
    aria-atomic="true"
  >
    {visibleStatusMessage}
  </div>

  <div
    id="search-results"
    class="result-list"
    role="listbox"
    tabindex="-1"
    aria-label="Search results"
  >
    {#if results.length}
      {#each visibleRows as row, index (row.rowKey)}
        {@const result = row.result}
        {@const hints = searchResultActionHints(result)}
        <div
          id={row.domId}
          class:result-selected={index === selectedRowIndex}
          class="result-row"
          role="option"
          tabindex="-1"
          draggable={result.kind === 'folder' && !!result.path}
          aria-selected={index === selectedRowIndex}
          use:trackVisibleRow={index}
          on:click={() => selectRow(row)}
          on:dblclick={() => activateRow(row)}
          on:keydown={(event) => handleOptionKeydown(event, row)}
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
            <strong>
              {#each highlightParts(result.title, result.titleHighlightData) as part}
                <span class:result-highlight={part.highlighted}>{part.character}</span>
              {/each}
            </strong>
            <small>
              {#each highlightParts(result.subtitle, result.subtitleHighlightData) as part}
                <span class:result-highlight={part.highlighted}>{part.character}</span>
              {/each}
            </small>
          </span>
          <span class="result-actions">
            <span class="result-action-primary">{hints.primary}</span>
            {#if hints.secondary === 'Pin' && result.kind === 'folder' && result.path}
              <span class="pin-folder" aria-hidden="true">Pin</span>
              <span class="pin-folder-shortcut">Ctrl+Enter</span>
            {:else if hints.secondary}
              <span class="result-kind">{hints.secondary}</span>
            {:else}
              <span class="result-kind">{result.kind}</span>
            {/if}
          </span>
        </div>
      {/each}
    {:else}
      <div class="empty-state surface-state info">{visibleStatusMessage ? '' : 'No search results matched'}</div>
    {/if}
  </div>
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
