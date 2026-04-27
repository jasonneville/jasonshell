<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount } from 'svelte';
  import {
    copyStackItems,
    deleteStackItem,
    getStackPopupRequest,
    hideStackPopup,
    listStackFolder,
    newStackFolder,
    openStackItem,
    pinStackFolder,
    pasteStackItems,
    renameStackItem,
    revealStackItem,
    STACK_POPUP_OPEN_EVENT,
    type StackEntry,
    type StackFolderListing
  } from '../lib/stackPopup';
  import { folderPathsFromTransfer, normalizeDroppedPath, setFolderDragPayload } from '../lib/folderDrag';
  import {
    applyStackFolderListing,
    canNavigateStackBack,
    canNavigateStackForward,
    mergeStackFolderListings,
    defaultStackPopupViewState,
    findTypeToSelectPath,
    formatStackSize,
    navigateStackHistory,
    openStackFolder,
    parentStackPath,
    selectAllStackEntries,
    selectedStackEntry,
    selectedStackPaths,
    selectStackEntry,
    stackPopupHasRetainedRows,
    stackBreadcrumbSegments,
    stackPopupOpenPath,
    stackPopupRequestKey,
    updateStackSort,
    type StackPopupOpenPayload,
    type StackSortColumn
  } from '../lib/stackPopupState';
  import { stackFileIconForEntry } from '../lib/stackFileIcons';

  const STACK_PATHS_DRAG_TYPE = 'application/x-jasonshell-stack-paths';

  let stackState = defaultStackPopupViewState;
  let loadingPath: string | null = null;
  let errorMessage = '';
  let rowMenu: { x: number; y: number; path: string } | null = null;
  let backgroundMenu: { x: number; y: number } | null = null;
  let typeToSelectBuffer = '';
  let typeToSelectTimer: number | null = null;
  let lastHtmlDropAt = 0;
  let detailsGrid: HTMLDivElement | null = null;
  let lastHandledOpenRequestKey: string | null = null;
  let pendingOpenRequestKey: string | null = null;
  let folderLoadSequence = 0;

  $: currentPath = stackState.currentPath;
  $: entries = stackState.entries;
  $: selectedEntry = selectedStackEntry(stackState);
  $: selectedPaths = selectedStackPaths(stackState);
  $: hasSelection = selectedPaths.length > 0;
  $: canGoBack = canNavigateStackBack(stackState);
  $: canGoForward = canNavigateStackForward(stackState);
  $: breadcrumbs = stackBreadcrumbSegments(currentPath);
  $: hasRetainedRows = stackPopupHasRetainedRows(stackState);

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    const latestRequestTimer = window.setInterval(() => {
      void reconcileLatestStackPopupRequest();
    }, 250);

    void initializeOpenRequestDelivery(unlisteners, () => disposed);
    void getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === 'drop' && currentPath && Date.now() - lastHtmlDropAt > 500) {
        void pasteDroppedPaths(event.payload.paths, currentPath, false);
      }
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlisteners.push(unlisten);
      }
    });

    return () => {
      disposed = true;
      if (typeToSelectTimer !== null) {
        window.clearTimeout(typeToSelectTimer);
      }
      window.clearInterval(latestRequestTimer);
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });

  async function initializeOpenRequestDelivery(unlisteners: Array<() => void>, isDisposed: () => boolean) {
    try {
      const unlisten = await getCurrentWindow().listen<StackPopupOpenPayload>(STACK_POPUP_OPEN_EVENT, (event) => {
        void handleOpenRequest(event.payload);
      });
      if (isDisposed()) {
        unlisten();
        return;
      }
      unlisteners.push(unlisten);
      await openLatestStackPopupRequest();
    } catch (error) {
      console.error('Failed to initialize stack popup open listener', error);
      await openLatestStackPopupRequest();
    }
  }

  async function openLatestStackPopupRequest() {
    try {
      await handleOpenRequest(await getStackPopupRequest());
    } catch (error) {
      console.error('Failed to load latest stack popup request', error);
    }
  }

  async function reconcileLatestStackPopupRequest() {
    try {
      await handleOpenRequest(await getStackPopupRequest());
    } catch (error) {
      console.error('Failed to reconcile latest stack popup request', error);
    }
  }

  async function handleOpenRequest(payload: StackPopupOpenPayload) {
    const path = stackPopupOpenPath(payload);
    const requestKey = stackPopupRequestKey(payload);
    if (!path || !requestKey || requestKey === lastHandledOpenRequestKey || requestKey === pendingOpenRequestKey) {
      return;
    }

    pendingOpenRequestKey = requestKey;
    try {
      await openFolder(path);
      lastHandledOpenRequestKey = requestKey;
    } finally {
      if (pendingOpenRequestKey === requestKey) {
        pendingOpenRequestKey = null;
      }
    }
  }

  async function openFolder(folderPath: string) {
    closeMenus();
    stackState = openStackFolder(stackState, folderPath);
    await loadFolder(stackState.currentPath);
  }

  async function loadFolder(folderPath: string) {
    if (!folderPath) {
      return;
    }

    const loadSequence = ++folderLoadSequence;
    let mergedListing: StackFolderListing | null = null;
    loadingPath = folderPath;
    errorMessage = '';
    try {
      const listing = await listStackFolder(folderPath, async (page) => {
        if (loadSequence !== folderLoadSequence) {
          return;
        }

        mergedListing = mergeStackFolderListings(mergedListing, page);
        stackState = applyStackFolderListing(stackState, folderPath, mergedListing);
        focusDetailsGrid();
      });
      if (loadSequence !== folderLoadSequence) {
        return;
      }
      stackState = applyStackFolderListing(stackState, folderPath, mergedListing ?? listing);
      focusDetailsGrid();
    } catch (error) {
      console.error('Failed to load stack folder', error);
      if (loadSequence === folderLoadSequence && loadingPath === folderPath) {
        errorMessage = error instanceof Error ? error.message : String(error || 'Folder unavailable');
      }
    } finally {
      if (loadSequence === folderLoadSequence && loadingPath === folderPath) {
        loadingPath = null;
      }
    }
  }

  async function navigateHistory(direction: -1 | 1) {
    stackState = navigateStackHistory(stackState, direction);
    await loadFolder(stackState.currentPath);
  }

  async function activateEntry(entry: StackEntry) {
    closeMenus();
    if (entry.entryType === 'Folder') {
      await openFolder(entry.path);
    } else {
      await openStackItem(entry.path);
    }
  }

  async function copySelected(cut: boolean) {
    closeMenus();
    if (!selectedPaths.length) {
      return;
    }

    try {
      await copyStackItems(selectedPaths, cut);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to copy stack items', error);
      errorMessage = operationErrorMessage(error, cut ? 'Cut unavailable' : 'Copy unavailable');
    }
  }

  async function pasteIntoCurrentFolder() {
    closeMenus();
    if (!currentPath) {
      return;
    }

    try {
      const listing = await pasteStackItems(currentPath);
      stackState = applyStackFolderListing(stackState, currentPath, listing);
      errorMessage = pasteFailureSummary(listing.pasteFailures);
    } catch (error) {
      console.error('Failed to paste stack items', error);
      errorMessage = operationErrorMessage(error, 'Paste unavailable');
    }
  }

  async function deleteSelected() {
    closeMenus();
    if (!selectedPaths.length || !currentPath) {
      return;
    }

    const label = selectedPaths.length === 1 && selectedEntry ? selectedEntry.name : `${selectedPaths.length} items`;
    if (!window.confirm(`Delete ${label}?`)) {
      return;
    }

    try {
      const failures: string[] = [];
      for (const path of selectedPaths) {
        try {
          await deleteStackItem(path);
        } catch (error) {
          failures.push(operationErrorMessage(error, `Failed to delete ${path}`));
        }
      }
      const listing = await listStackFolder(currentPath);
      stackState = applyStackFolderListing(stackState, currentPath, listing);
      errorMessage = failures.length
        ? `Delete completed with ${failures.length} failure${failures.length === 1 ? '' : 's'}: ${failures[0]}`
        : '';
    } catch (error) {
      console.error('Failed to delete stack item', error);
      errorMessage = operationErrorMessage(error, 'Delete unavailable');
    }
  }

  async function createFolder() {
    closeMenus();
    if (!currentPath) {
      return;
    }

    const name = window.prompt('New folder name', 'New Folder');
    if (!name) {
      return;
    }

    try {
      const createdEntry = await newStackFolder(currentPath, name);
      const listing = await listStackFolder(currentPath);
      stackState = selectStackEntry(
        applyStackFolderListing(stackState, currentPath, listing),
        createdEntry.path
      );
      errorMessage = '';
    } catch (error) {
      console.error('Failed to create stack folder', error);
      errorMessage = operationErrorMessage(error, 'New folder unavailable');
    }
  }

  async function revealSelected() {
    closeMenus();
    if (!selectedEntry) {
      return;
    }

    try {
      await revealStackItem(selectedEntry.path);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to reveal stack item', error);
      errorMessage = operationErrorMessage(error, 'Reveal unavailable');
    }
  }

  async function pinSelectedFolderToTopBar() {
    closeMenus();
    if (!selectedEntry || selectedEntry.entryType !== 'Folder') {
      return;
    }

    try {
      await pinStackFolder(selectedEntry.path);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to pin stack folder', error);
      errorMessage = operationErrorMessage(error, 'Pin unavailable');
    }
  }

  async function renameSelected() {
    closeMenus();
    if (!selectedEntry) {
      return;
    }

    const nextName = window.prompt('Rename stack item', selectedEntry.name);
    if (!nextName || nextName === selectedEntry.name) {
      return;
    }

    try {
      const renamedEntry = await renameStackItem(selectedEntry.path, nextName);
      const listing = await listStackFolder(currentPath);
      stackState = selectStackEntry(
        applyStackFolderListing(stackState, currentPath, listing),
        renamedEntry.path
      );
      errorMessage = '';
    } catch (error) {
      console.error('Failed to rename stack item', error);
      errorMessage = operationErrorMessage(error, 'Rename unavailable');
    }
  }

  function operationErrorMessage(error: unknown, fallback: string) {
    if (typeof error === 'string' && error.trim()) {
      return error;
    }
    if (error instanceof Error && error.message) {
      return error.message;
    }
    return fallback;
  }

  function pasteFailureSummary(failures: { path: string; message: string }[]) {
    if (!failures.length) {
      return '';
    }
    const first = failures[0];
    const suffix = failures.length === 1 ? '' : ` and ${failures.length - 1} more`;
    return `Paste completed with ${failures.length} failure${failures.length === 1 ? '' : 's'}: ${first.message}${suffix}`;
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

  function closeMenus() {
    rowMenu = null;
    backgroundMenu = null;
  }

  function focusDetailsGrid() {
    window.requestAnimationFrame(() => detailsGrid?.focus());
  }

  function sortBy(column: StackSortColumn) {
    stackState = updateStackSort(stackState, column);
    focusDetailsGrid();
  }

  function ariaSort(column: StackSortColumn) {
    if (stackState.sortColumn !== column) {
      return 'none';
    }
    return stackState.sortDirection === 'asc' ? 'ascending' : 'descending';
  }

  function sortIndicator(column: StackSortColumn) {
    if (stackState.sortColumn !== column) {
      return '';
    }
    return stackState.sortDirection === 'asc' ? ' ▲' : ' ▼';
  }

  function stackAttributeLabels(entry: StackEntry) {
    const labels = [];
    if (entry.isHidden) labels.push('Hidden');
    if (entry.isSystem) labels.push('System');
    if (entry.isReadonly) labels.push('Read-only');
    if (entry.isSymlink || entry.isReparsePoint) labels.push('Link');
    return labels;
  }

  function selectEntryFromMouse(event: MouseEvent, entry: StackEntry) {
    if (hasRetainedRows) {
      return;
    }
    const mode = event.shiftKey ? 'range' : event.ctrlKey || event.metaKey ? 'toggle' : 'single';
    stackState = selectStackEntry(stackState, entry.path, mode);
  }

  function selectEntryByIndex(index: number, range = false) {
    if (hasRetainedRows) {
      return;
    }
    const entry = entries[Math.max(0, Math.min(entries.length - 1, index))];
    if (entry) {
      stackState = selectStackEntry(stackState, entry.path, range ? 'range' : 'single');
    }
  }

  function selectedIndex() {
    return entries.findIndex((entry) => entry.path === stackState.selectedPath);
  }

  async function navigateParent() {
    const parent = parentStackPath(currentPath);
    if (parent && parent !== currentPath) {
      await openFolder(parent);
    }
  }

  function handleRowContextMenu(event: MouseEvent, entry: StackEntry) {
    if (hasRetainedRows) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (!stackState.selectedPaths.includes(entry.path)) {
      stackState = selectStackEntry(stackState, entry.path);
    }
    rowMenu = { x: event.clientX, y: event.clientY, path: entry.path };
    backgroundMenu = null;
  }

  function handleBackgroundContextMenu(event: MouseEvent) {
    event.preventDefault();
    rowMenu = null;
    backgroundMenu = { x: event.clientX, y: event.clientY };
  }

  function selectedDragPaths(entry: StackEntry) {
    return stackState.selectedPaths.includes(entry.path) ? selectedPaths : [entry.path];
  }

  function handleRowDragStart(event: DragEvent, entry: StackEntry) {
    if (hasRetainedRows) {
      event.preventDefault();
      return;
    }
    const paths = selectedDragPaths(entry);
    if (!stackState.selectedPaths.includes(entry.path)) {
      stackState = selectStackEntry(stackState, entry.path);
    }
    event.dataTransfer?.setData(STACK_PATHS_DRAG_TYPE, JSON.stringify(paths));
    event.dataTransfer?.setData('text/plain', paths.join('\n'));
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'copyMove';
      const folderPaths = paths.filter((path) => entries.some((item) => item.path === path && item.entryType === 'Folder'));
      if (folderPaths.length) {
        setFolderDragPayload(event.dataTransfer, folderPaths, 'copyMove');
      }
    }
  }

  function pathsFromDrop(event: DragEvent) {
    const transfer = event.dataTransfer;
    if (!transfer) {
      return [];
    }
    const custom = transfer.getData(STACK_PATHS_DRAG_TYPE);
    if (custom) {
      try {
        const parsed = JSON.parse(custom);
        if (Array.isArray(parsed)) {
          return parsed.map((path) => normalizeDroppedPath(String(path))).filter(Boolean) as string[];
        }
      } catch {
        // Fall through to shared drag parsers.
      }
    }
    return folderPathsFromTransfer(transfer);
  }

  function handleDropOver(event: DragEvent, destinationEntry?: StackEntry) {
    if (destinationEntry && destinationEntry.entryType !== 'Folder') {
      return;
    }
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = event.shiftKey ? 'move' : 'copy';
    }
  }

  async function handleDrop(event: DragEvent, destinationPath: string) {
    event.preventDefault();
    event.stopPropagation();
    lastHtmlDropAt = Date.now();
    closeMenus();
    const paths = pathsFromDrop(event);
    await pasteDroppedPaths(paths, destinationPath, event.shiftKey);
  }

  async function pasteDroppedPaths(paths: string[], destinationPath: string, move: boolean) {
    if (!paths.length || !destinationPath) {
      return;
    }
    try {
      await copyStackItems(paths, move);
      const listing = await pasteStackItems(destinationPath);
      if (destinationPath === currentPath) {
        stackState = applyStackFolderListing(stackState, currentPath, listing);
      } else {
        await loadFolder(currentPath);
      }
      errorMessage = pasteFailureSummary(listing.pasteFailures);
    } catch (error) {
      console.error('Failed to drop stack items', error);
      errorMessage = operationErrorMessage(error, 'Drop unavailable');
      await loadFolder(currentPath);
    }
  }

  function typeToSelect(key: string) {
    if (hasRetainedRows) {
      return;
    }
    if (typeToSelectTimer !== null) {
      window.clearTimeout(typeToSelectTimer);
    }
    typeToSelectBuffer += key;
    typeToSelectTimer = window.setTimeout(() => {
      typeToSelectBuffer = '';
      typeToSelectTimer = null;
    }, 700);
    const path = findTypeToSelectPath(entries, typeToSelectBuffer, stackState.selectedPath);
    if (path) {
      stackState = selectStackEntry(stackState, path);
    }
  }

  function selectAdjacentEntry(direction: 1 | -1) {
    if (hasRetainedRows) {
      return;
    }
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
      if (rowMenu || backgroundMenu) {
        closeMenus();
      } else {
        void hideStackPopup();
      }
    } else if (event.key === 'Enter' && selectedEntry) {
      event.preventDefault();
      void activateEntry(selectedEntry);
    } else if (event.key === 'Delete') {
      event.preventDefault();
      void deleteSelected();
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      selectAdjacentEntry(1);
    } else if (event.altKey && event.key === 'ArrowUp') {
      event.preventDefault();
      void navigateParent();
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
    } else if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === 'n') {
      event.preventDefault();
      void createFolder();
    } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') {
      event.preventDefault();
      stackState = selectAllStackEntries(stackState);
    } else if (event.key === 'Backspace') {
      event.preventDefault();
      void navigateParent();
    } else if (event.key === 'Home') {
      event.preventDefault();
      selectEntryByIndex(0, event.shiftKey);
    } else if (event.key === 'End') {
      event.preventDefault();
      selectEntryByIndex(entries.length - 1, event.shiftKey);
    } else if (event.key === 'PageDown') {
      event.preventDefault();
      selectEntryByIndex((selectedIndex() < 0 ? 0 : selectedIndex()) + 10, event.shiftKey);
    } else if (event.key === 'PageUp') {
      event.preventDefault();
      selectEntryByIndex((selectedIndex() < 0 ? entries.length - 1 : selectedIndex()) - 10, event.shiftKey);
    } else if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      const entry = selectedEntry ?? entries[0];
      if (entry) {
        rowMenu = { x: 24, y: 72, path: entry.path };
        backgroundMenu = null;
      }
    } else if (event.key === 'F2') {
      event.preventDefault();
      void renameSelected();
    } else if (!event.ctrlKey && !event.metaKey && !event.altKey && event.key.length === 1) {
      typeToSelect(event.key);
    }
  }

  function handleMouseNavigation(event: MouseEvent) {
    if (event.button === 3 && canGoBack) {
      event.preventDefault();
      void navigateHistory(-1);
    } else if (event.button === 4 && canGoForward) {
      event.preventDefault();
      void navigateHistory(1);
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} on:click={closeMenus} on:mousedown={handleMouseNavigation} />

<section class="stack-popup" aria-label="Stack browser">
  <header class="stack-toolbar">
    <div class="stack-path" title={currentPath}>
      {#if currentPath}
        <nav class="breadcrumbs" aria-label="Path breadcrumbs">
          {#each breadcrumbs as crumb, i (crumb.path)}
            <button type="button" class="crumb" on:click={() => void openFolder(crumb.path)}>{crumb.name}</button>
            {#if i < breadcrumbs.length - 1}
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
      <button type="button" disabled={!hasSelection} on:click={() => void copySelected(false)}>Copy</button>
      <button type="button" disabled={!hasSelection} on:click={() => void copySelected(true)}>Cut</button>
      <button type="button" disabled={!currentPath} on:click={() => void pasteIntoCurrentFolder()}>Paste</button>
      <button type="button" disabled={!selectedEntry} on:click={() => void renameSelected()}>Rename</button>
      <button type="button" disabled={!hasSelection} on:click={() => void deleteSelected()}>Delete</button>
      <button type="button" disabled={!currentPath} on:click={() => void createFolder()}>New Folder</button>
      <button type="button" disabled={!selectedEntry} on:click={() => void revealSelected()}>Reveal</button>
    </div>
  </header>

  <div class="stack-status">
    <span>{errorMessage || stackState.statusMessage}</span>
    {#if loadingPath}
      <span>Loading...</span>
    {/if}
  </div>

  <div
    class="details-table"
    role="grid"
    aria-label="Folder details"
    aria-rowcount={entries.length + 1}
    aria-colcount="4"
    tabindex="0"
    bind:this={detailsGrid}
    on:contextmenu={handleBackgroundContextMenu}
    on:dragover={(event) => handleDropOver(event)}
    on:drop={(event) => void handleDrop(event, currentPath)}
  >
    <div class="details-header" role="row" aria-rowindex="1">
      <button type="button" class="details-sort" role="columnheader" aria-colindex="1" aria-sort={ariaSort('name')} on:click={() => sortBy('name')}>Name{sortIndicator('name')}</button>
      <button type="button" class="details-sort" role="columnheader" aria-colindex="2" aria-sort={ariaSort('type')} on:click={() => sortBy('type')}>Type{sortIndicator('type')}</button>
      <button type="button" class="details-sort" role="columnheader" aria-colindex="3" aria-sort={ariaSort('size')} on:click={() => sortBy('size')}>Size{sortIndicator('size')}</button>
      <button type="button" class="details-sort" role="columnheader" aria-colindex="4" aria-sort={ariaSort('modified')} on:click={() => sortBy('modified')}>Modified{sortIndicator('modified')}</button>
    </div>

    {#if entries.length}
      <div class="details-body">
        {#each entries as entry, index (entry.id)}
          {@const fileIcon = stackFileIconForEntry(entry)}
          <button
            class:selected={stackState.selectedPaths.includes(entry.path)}
            class:subdued={entry.isHidden || entry.isSystem}
            class:readonly={entry.isReadonly}
            class:linked={entry.isSymlink || entry.isReparsePoint}
            class:retained={hasRetainedRows}
            type="button"
            role="row"
            aria-rowindex={index + 2}
            aria-selected={stackState.selectedPaths.includes(entry.path)}
            aria-disabled={hasRetainedRows}
            disabled={hasRetainedRows}
            draggable={!hasRetainedRows}
            on:click={(event) => selectEntryFromMouse(event, entry)}
            on:dblclick={() => void activateEntry(entry)}
            on:contextmenu={(event) => handleRowContextMenu(event, entry)}
            on:dragstart={(event) => handleRowDragStart(event, entry)}
            on:dragover={(event) => handleDropOver(event, entry)}
            on:drop={(event) => entry.entryType === 'Folder' && void handleDrop(event, entry.path)}
          >
            <span role="gridcell" aria-colindex="1" title={entry.path}>
              <span
                class={`stack-entry-icon stack-entry-icon-${fileIcon.kind}`}
                aria-label={fileIcon.label}
                title={fileIcon.label}
              >
                {#if entry.iconDataUrl}
                  <img src={entry.iconDataUrl} alt="" aria-hidden="true" draggable="false" />
                {:else}
                  <span class="stack-entry-icon-shape" aria-hidden="true"></span>
                {/if}
              </span>
              <span>{entry.name}</span>
              {#if stackAttributeLabels(entry).length}
                <span class="item-badges" aria-label={stackAttributeLabels(entry).join(', ')}>
                  {#each stackAttributeLabels(entry) as label}
                    <span>{label}</span>
                  {/each}
                </span>
              {/if}
            </span>
            <span role="gridcell" aria-colindex="2">{entry.typeLabel}</span>
            <span role="gridcell" aria-colindex="3">{formatStackSize(entry.size)}</span>
            <span role="gridcell" aria-colindex="4">{formatModified(entry.modifiedMs)}</span>
          </button>
        {/each}
      </div>
    {:else}
      <div class="empty-stack">{loadingPath ? 'Loading folder...' : stackState.statusMessage}</div>
    {/if}
  </div>

  {#if rowMenu}
    <div
      class="context-menu"
      style={`left:${rowMenu.x}px;top:${rowMenu.y}px`}
      role="menu"
      tabindex="-1"
      on:click|stopPropagation
      on:keydown={(event) => event.key === 'Escape' && closeMenus()}
    >
      <button type="button" role="menuitem" disabled={!selectedEntry} on:click={() => selectedEntry && void activateEntry(selectedEntry)}>Open</button>
      <button type="button" role="menuitem" disabled={!hasSelection} on:click={() => void copySelected(false)}>Copy</button>
      <button type="button" role="menuitem" disabled={!hasSelection} on:click={() => void copySelected(true)}>Cut</button>
      <button type="button" role="menuitem" disabled={selectedEntry?.entryType !== 'Folder'} on:click={() => void pinSelectedFolderToTopBar()}>Pin to Top Bar</button>
      <button type="button" role="menuitem" disabled={!selectedEntry} on:click={() => void renameSelected()}>Rename</button>
      <button type="button" role="menuitem" disabled={!hasSelection} on:click={() => void deleteSelected()}>Delete</button>
      <button type="button" role="menuitem" disabled={!selectedEntry} on:click={() => void revealSelected()}>Reveal</button>
    </div>
  {/if}

  {#if backgroundMenu}
    <div
      class="context-menu"
      style={`left:${backgroundMenu.x}px;top:${backgroundMenu.y}px`}
      role="menu"
      tabindex="-1"
      on:click|stopPropagation
      on:keydown={(event) => event.key === 'Escape' && closeMenus()}
    >
      <button type="button" role="menuitem" disabled={!currentPath} on:click={() => void pasteIntoCurrentFolder()}>Paste</button>
      <button type="button" role="menuitem" disabled={!currentPath} on:click={() => void createFolder()}>New Folder</button>
    </div>
  {/if}
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
    display: grid;
    gap: 0.65rem;
  }

  .stack-path {
    color: rgba(240, 244, 255, 0.92);
    font-size: 0.82rem;
    font-weight: 750;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stack-actions {
    display: flex;
    flex-wrap: wrap;
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

  .details-table:focus-visible {
    outline: 2px solid rgba(150, 184, 255, 0.62);
    outline-offset: -2px;
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

  .details-header .details-sort,
  .details-body button > span {
    align-content: center;
    min-width: 0;
    overflow: hidden;
    padding: 0 0.55rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .details-body button > span:first-child {
    align-items: center;
    display: flex;
    gap: 0.4rem;
  }

  .stack-entry-icon {
    align-items: center;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.12), rgba(255, 255, 255, 0.04));
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 0.18rem;
    display: inline-flex;
    flex: 0 0 auto;
    height: 1rem;
    justify-content: center;
    min-width: 1rem;
    overflow: hidden;
    position: relative;
    width: 1rem;
  }

  .stack-entry-icon img {
    height: 1rem;
    width: 1rem;
  }

  .stack-entry-icon-shape,
  .stack-entry-icon-shape::before,
  .stack-entry-icon-shape::after {
    box-sizing: border-box;
    display: block;
    position: absolute;
  }

  .stack-entry-icon-shape {
    inset: 0;
  }

  .stack-entry-icon-folder .stack-entry-icon-shape::before {
    background: linear-gradient(180deg, #ffd67a, #d79932);
    border-radius: 0.1rem 0.1rem 0 0;
    content: '';
    height: 0.28rem;
    left: 0.12rem;
    top: 0.2rem;
    width: 0.42rem;
  }

  .stack-entry-icon-folder .stack-entry-icon-shape::after {
    background: linear-gradient(180deg, #ffd978, #b97920);
    border: 1px solid rgba(255, 239, 177, 0.55);
    border-radius: 0.12rem;
    content: '';
    height: 0.55rem;
    inset: 0.34rem 0.1rem 0.11rem;
  }

  .stack-entry-icon-file .stack-entry-icon-shape::before,
  .stack-entry-icon-document .stack-entry-icon-shape::before,
  .stack-entry-icon-code .stack-entry-icon-shape::before,
  .stack-entry-icon-image .stack-entry-icon-shape::before,
  .stack-entry-icon-audio .stack-entry-icon-shape::before,
  .stack-entry-icon-video .stack-entry-icon-shape::before,
  .stack-entry-icon-archive .stack-entry-icon-shape::before,
  .stack-entry-icon-app .stack-entry-icon-shape::before {
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(184, 203, 242, 0.86));
    border: 1px solid rgba(255, 255, 255, 0.5);
    border-radius: 0.12rem;
    content: '';
    inset: 0.13rem 0.2rem 0.12rem;
  }

  .stack-entry-icon-file .stack-entry-icon-shape::after,
  .stack-entry-icon-document .stack-entry-icon-shape::after,
  .stack-entry-icon-code .stack-entry-icon-shape::after,
  .stack-entry-icon-image .stack-entry-icon-shape::after,
  .stack-entry-icon-audio .stack-entry-icon-shape::after,
  .stack-entry-icon-video .stack-entry-icon-shape::after,
  .stack-entry-icon-archive .stack-entry-icon-shape::after,
  .stack-entry-icon-app .stack-entry-icon-shape::after {
    border-left: 0.2rem solid transparent;
    border-top: 0.2rem solid rgba(105, 133, 190, 0.75);
    content: '';
    right: 0.2rem;
    top: 0.13rem;
  }

  .stack-entry-icon-app .stack-entry-icon-shape::before {
    background: linear-gradient(135deg, #8fc7ff, #3767d5);
    border-radius: 0.18rem;
    inset: 0.16rem;
  }

  .stack-entry-icon-app .stack-entry-icon-shape::after {
    background: rgba(255, 255, 255, 0.72);
    border: 0;
    border-radius: 999px;
    content: '';
    height: 0.26rem;
    inset: 0.37rem;
  }

  .stack-entry-icon-image .stack-entry-icon-shape::before {
    background: linear-gradient(180deg, #d9fff5, #6bd8bd);
  }

  .stack-entry-icon-archive .stack-entry-icon-shape::before {
    background: linear-gradient(180deg, #f0d6ff, #ac75df);
  }

  .stack-entry-icon-code .stack-entry-icon-shape::before {
    background: linear-gradient(180deg, #e5edff, #83a3ff);
  }

  .stack-entry-icon-folder {
    background: linear-gradient(180deg, rgba(245, 191, 92, 0.42), rgba(164, 113, 35, 0.32));
    border-color: rgba(245, 191, 92, 0.38);
    color: #fff0c7;
  }

  .stack-entry-icon-app {
    background: linear-gradient(180deg, rgba(114, 178, 255, 0.42), rgba(45, 86, 177, 0.34));
    border-color: rgba(132, 196, 255, 0.42);
    color: #e9f3ff;
  }

  .stack-entry-icon-image,
  .stack-entry-icon-video {
    background: linear-gradient(180deg, rgba(100, 211, 181, 0.34), rgba(35, 112, 97, 0.28));
    border-color: rgba(111, 230, 194, 0.32);
  }

  .stack-entry-icon-archive {
    background: linear-gradient(180deg, rgba(204, 151, 255, 0.34), rgba(91, 51, 137, 0.32));
    border-color: rgba(204, 151, 255, 0.32);
  }

  .stack-entry-icon-code {
    background: linear-gradient(180deg, rgba(144, 181, 255, 0.32), rgba(57, 84, 158, 0.3));
    border-color: rgba(144, 181, 255, 0.34);
  }

  .details-header .details-sort {
    background: transparent;
    border: 0;
    color: inherit;
    font: inherit;
    text-align: left;
    text-transform: inherit;
  }

  .details-header .details-sort:hover,
  .details-header .details-sort:focus-visible {
    background: rgba(124, 160, 255, 0.14);
    color: rgba(240, 244, 255, 0.86);
    outline: 0;
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

  .details-body button.selected {
    outline: 1px solid rgba(124, 160, 255, 0.34);
    outline-offset: -1px;
  }

  .details-body button.subdued {
    color: rgba(218, 226, 248, 0.58);
  }

  .details-body button.readonly {
    box-shadow: inset 3px 0 0 rgba(245, 191, 92, 0.42);
  }

  .details-body button.linked {
    box-shadow: inset 3px 0 0 rgba(132, 196, 255, 0.48);
  }

  .details-body button.readonly.linked {
    box-shadow:
      inset 3px 0 0 rgba(245, 191, 92, 0.42),
      inset 6px 0 0 rgba(132, 196, 255, 0.36);
  }

  .details-body button.retained {
    cursor: progress;
  }

  .item-badges {
    display: inline-flex;
    gap: 0.18rem;
    margin-left: 0.35rem;
    padding: 0;
    vertical-align: middle;
  }

  .item-badges span {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 999px;
    color: rgba(240, 244, 255, 0.7);
    font-size: 0.5rem;
    font-weight: 800;
    line-height: 1;
    padding: 0.12rem 0.25rem;
  }

  .details-body button:focus-visible {
    outline: 2px solid rgba(150, 184, 255, 0.72);
    outline-offset: -2px;
  }

  .context-menu {
    background: rgba(15, 20, 32, 0.98);
    border: 1px solid rgba(255, 255, 255, 0.12);
    border-radius: 0.35rem;
    box-shadow: 0 16px 34px rgba(0, 0, 0, 0.42);
    display: grid;
    min-width: 8.5rem;
    padding: 0.25rem;
    position: fixed;
    z-index: 50;
  }

  .context-menu button {
    background: transparent;
    border: 0;
    border-radius: 0.24rem;
    color: rgba(240, 244, 255, 0.92);
    font: inherit;
    font-size: 0.68rem;
    min-height: 1.55rem;
    padding: 0 0.55rem;
    text-align: left;
  }

  .context-menu button:hover:not(:disabled),
  .context-menu button:focus-visible {
    background: rgba(77, 124, 254, 0.22);
  }

  .context-menu button:disabled {
    color: rgba(218, 226, 248, 0.32);
  }

  .empty-stack {
    align-items: center;
    color: rgba(218, 226, 248, 0.58);
    display: grid;
    font-size: 0.72rem;
    justify-items: center;
  }

  .breadcrumbs {
    width: 100%;
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
    min-width: 0;
    padding: 0 0.25rem;
    text-overflow: ellipsis;
    white-space: nowrap;
    overflow: hidden;
  }

  .breadcrumbs .crumb:last-child {
    flex: 1 1 auto;
    text-align: left;
  }

  .breadcrumbs .crumb-sep {
    color: rgba(218, 226, 248, 0.46);
    padding: 0 0.12rem;
  }

  .stack-actions button:nth-child(3) {
    background: rgba(255, 255, 255, 0.04);
  }
</style>
