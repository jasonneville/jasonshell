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
    buildVisibleSearchGroupOverflows,
    DEFAULT_VISIBLE_GROUP_LIMIT,
    nextSearchPanelFallbackDelay,
    nextVisibleRowIndex,
    type SearchExpandableGroupId,
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
  let expandedVisibleGroups = new Set<SearchExpandableGroupId>();
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
  $: visibleRows = buildVisibleSearchRows(results, {
    expandedGroups: expandedVisibleGroups,
    perGroupLimit: DEFAULT_VISIBLE_GROUP_LIMIT
  });
  $: visibleGroupOverflows = buildVisibleSearchGroupOverflows(results, {
    expandedGroups: expandedVisibleGroups,
    perGroupLimit: DEFAULT_VISIBLE_GROUP_LIMIT
  });
  $: overflowByGroup = new Map(visibleGroupOverflows.map((overflow) => [overflow.groupId, overflow]));
  $: lastVisibleIndexByGroup = visibleRows.reduce((lookup, row, index) => {
    lookup.set(row.groupId, index);
    return lookup;
  }, new Map<string, number>());
  $: selectedRowIndex = selectedVisibleRowIndex(visibleRows, selectedIndex);
  $: selectedRow = selectedRowIndex >= 0 ? visibleRows[selectedRowIndex] : null;
  $: void revealSelectedResult(selectedRowIndex, visibleRows.length);
  const topBarTarget = topBarWebviewWindowEventTarget();

  function applyPayload(payload: SearchPanelPayload | null) {
    const previousQuery = panelState.query;
    panelState = applySearchPanelPayload(panelState, payload);
    if (panelState.query !== previousQuery) {
      expandedVisibleGroups = new Set<SearchExpandableGroupId>();
    }
    if (panelState.query === '' || optimisticQueryDraft === panelState.query) {
      optimisticQueryDraft = null;
    }
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    void getCurrentWindow().listen<SearchPanelPayload>(SEARCH_PANEL_UPDATE_EVENT, (event) => {
      fallbackGeneration += 1;
      stopFallbackPolling();
      fallbackAttempt = 0;
      applyPayload(event.payload);
      if (shouldFocusCenteredQueryInput()) {
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
  }

  function markPanelInteraction() {
    announcePanelInteraction();
  }

  function announcePanelInteraction() {
    void emitTo(topBarTarget, SEARCH_PANEL_INTERACTION_EVENT, null);
  }

  function pinFolderResult(event: MouseEvent, result: SearchPanelResult) {
    event.preventDefault();
    event.stopPropagation();
    markPanelInteraction();
    if (result.kind === 'folder' && result.path) {
      void emitTo(topBarTarget, SEARCH_PANEL_PIN_FOLDER_EVENT, result.path);
    }
  }

  function expandGroup(groupId: SearchExpandableGroupId) {
    markPanelInteraction();
    expandedVisibleGroups = new Set([...expandedVisibleGroups, groupId]);
    void emitTo(topBarTarget, SEARCH_PANEL_EXPAND_GROUP_EVENT, groupId);
  }

  function handleResultKeydown(event: KeyboardEvent, row: SearchVisibleRow) {
    if (event.key === 'Enter') {
      event.preventDefault();
      activateRow(row);
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      selectVisibleOffset(1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      selectVisibleOffset(-1);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      if (presentation === 'centered') {
        hideCenteredPanelImmediately();
        return;
      }
      announcePanelInteraction();
      void emitTo(topBarTarget, SEARCH_PANEL_KEY_EVENT, event.key);
    } else if (event.key === ' ') {
      event.preventDefault();
      selectRow(row);
    }
  }

  function selectVisibleOffset(direction: -1 | 1) {
    const nextIndex = nextVisibleRowIndex(visibleRows, selectedRowIndex, direction);
    if (nextIndex < 0) {
      return;
    }
    selectRow(visibleRows[nextIndex]);
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
          on:click={() => selectRow(row)}
          on:dblclick={() => activateRow(row)}
          on:keydown={(event) => handleResultKeydown(event, row)}
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
        {@const overflow = overflowByGroup.get(row.groupId as SearchExpandableGroupId)}
        {#if overflow && lastVisibleIndexByGroup.get(row.groupId) === index}
          <button
            type="button"
            class="result-show-more"
            on:click={() => expandGroup(overflow.groupId)}
          >
            Show {overflow.hiddenCount} more {overflow.groupLabel.toLowerCase()}
          </button>
        {/if}
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
