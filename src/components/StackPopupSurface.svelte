<script lang="ts">
  import './StackPopupSurface.css';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { onMount, tick } from 'svelte';
  import {
    beginStackPopupFocusLossHold,
    copyStackItems,
    deleteStackItem,
    endStackPopupFocusLossHold,
    getStackPopupRequest,
    hideStackPopup,
    listStackFolder,
    newStackFolder,
    openStackItem,
    openStackItemWithPicker,
    pinStackFolder,
    pasteStackItems,
    renameStackItem,
    resizeStackPopup,
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
  import { positionContextMenuInViewport } from '../lib/contextMenuPosition';
  import {
    stackBrowserBreadcrumbOverflow,
    stackBrowserDeletePrompt,
    stackBrowserScrollTopForIndex,
    stackBrowserVirtualWindow
  } from '../features/stack-browser/viewModel';

  const STACK_PATHS_DRAG_TYPE = 'application/x-jasonshell-stack-paths';
  const STACK_POPUP_MIN_WIDTH = 560;
  const STACK_POPUP_MIN_HEIGHT = 280;

  let stackState = defaultStackPopupViewState;
  let loadingPath: string | null = null;
  let errorMessage = '';
  let rowMenu: { x: number; y: number; path: string } | null = null;
  let backgroundMenu: { x: number; y: number } | null = null;
  let rowMenuElement: HTMLDivElement | null = null;
  let backgroundMenuElement: HTMLDivElement | null = null;
  let deleteConfirmation: { title: string; message: string; paths: string[]; folderPath: string } | null = null;
  let deleteCancelButton: HTMLButtonElement | null = null;
  let rowSubmenuOpensLeft = false;
  let createFolderDraft: string | null = null;
  let renameDraft: string | null = null;
  let editorInput: HTMLInputElement | null = null;
  let typeToSelectBuffer = '';
  let typeToSelectTimer: number | null = null;
  let lastHtmlDropAt = 0;
  let detailsGrid: HTMLDivElement | null = null;
  let detailsBody: HTMLDivElement | null = null;
  let detailsBodyScrollTop = 0;
  let detailsBodyHeight = 0;
  let lastHandledOpenRequestKey: string | null = null;
  let pendingOpenRequestKey: string | null = null;
  let folderLoadSequence = 0;
  let resizeGrip: HTMLButtonElement | null = null;
  let resizeDrag:
    | {
        pointerId: number;
        startX: number;
        startY: number;
        startWidth: number;
        startHeight: number;
      }
    | null = null;
  let pendingResize: { width: number; height: number; persist: boolean } | null = null;
  let resizeFrame: number | null = null;
  let resizeRequestChain: Promise<void> = Promise.resolve();

  $: currentPath = stackState.currentPath;
  $: entries = stackState.entries;
  $: selectedEntry = selectedStackEntry(stackState);
  $: selectedPaths = selectedStackPaths(stackState);
  $: hasSelection = selectedPaths.length > 0;
  $: canGoBack = canNavigateStackBack(stackState);
  $: canGoForward = canNavigateStackForward(stackState);
  $: breadcrumbs = stackBreadcrumbSegments(currentPath);
  $: breadcrumbOverflow = stackBrowserBreadcrumbOverflow(breadcrumbs, 5);
  $: hasRetainedRows = stackPopupHasRetainedRows(stackState);
  $: virtualEntries = stackBrowserVirtualWindow(entries, detailsBodyScrollTop, detailsBodyHeight);

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
      if (resizeFrame !== null) {
        window.cancelAnimationFrame(resizeFrame);
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
        updateDetailsViewport();
        focusDetailsGrid();
      });
      if (loadSequence !== folderLoadSequence) {
        return;
      }
      stackState = applyStackFolderListing(stackState, folderPath, mergedListing ?? listing);
      updateDetailsViewport();
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

  async function openSelectedWithPicker() {
    closeMenus();
    if (!selectedEntry || selectedEntry.entryType !== 'File') {
      return;
    }

    try {
      await openStackItemWithPicker(selectedEntry.path);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to open stack item with picker', error);
      errorMessage = operationErrorMessage(error, 'Open with unavailable');
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

    const deletePrompt = stackBrowserDeletePrompt(entries, selectedPaths, stackState.selectedPath);
    if (!deletePrompt.canDelete) {
      return;
    }

    deleteConfirmation = {
      title: deletePrompt.title,
      message: deletePrompt.message,
      paths: deletePrompt.paths,
      folderPath: currentPath
    };
    await tick();
    deleteCancelButton?.focus();
  }

  function cancelDeleteConfirmation() {
    deleteConfirmation = null;
    focusDetailsGrid();
  }

  async function confirmDeleteSelection() {
    const pendingDelete = deleteConfirmation;
    if (!pendingDelete) {
      return;
    }

    deleteConfirmation = null;
    let focusHoldStarted = false;
    try {
      await beginStackPopupFocusLossHold();
      focusHoldStarted = true;
      const failures: string[] = [];
      for (const path of pendingDelete.paths) {
        try {
          await deleteStackItem(path);
        } catch (error) {
          failures.push(operationErrorMessage(error, `Failed to delete ${path}`));
        }
      }
      const listing = await listStackFolder(pendingDelete.folderPath);
      if (currentPath === pendingDelete.folderPath) {
        stackState = applyStackFolderListing(stackState, pendingDelete.folderPath, listing);
      } else {
        await loadFolder(currentPath);
      }
      focusDetailsGrid();
      errorMessage = failures.length
        ? `Delete completed with ${failures.length} failure${failures.length === 1 ? '' : 's'}: ${failures[0]}`
        : '';
    } catch (error) {
      console.error('Failed to delete stack item', error);
      errorMessage = operationErrorMessage(error, 'Delete unavailable');
    } finally {
      if (focusHoldStarted) {
        try {
          await endStackPopupFocusLossHold();
        } catch (error) {
          console.error('Failed to release stack popup focus hold', error);
          errorMessage ||= operationErrorMessage(error, 'Stack Browser focus restore failed');
        }
      }
    }
  }

  function beginCreateFolder() {
    closeMenus();
    if (!currentPath) {
      return;
    }
    renameDraft = null;
    createFolderDraft = 'New Folder';
    focusEditorInput();
  }

  async function createFolder() {
    closeMenus();
    if (!currentPath || createFolderDraft === null) {
      return;
    }

    const name = createFolderDraft.trim();
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
      createFolderDraft = null;
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

  function beginRenameSelected() {
    closeMenus();
    if (!selectedEntry) {
      return;
    }
    createFolderDraft = null;
    renameDraft = selectedEntry.name;
    focusEditorInput();
  }

  async function renameSelected() {
    closeMenus();
    if (!selectedEntry || renameDraft === null) {
      return;
    }

    const nextName = renameDraft.trim();
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
      renameDraft = null;
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

  function cancelInlineEditor() {
    createFolderDraft = null;
    renameDraft = null;
  }

  function focusEditorInput() {
    window.requestAnimationFrame(() => {
      editorInput?.focus();
      editorInput?.select();
    });
  }

  async function positionOpenMenus() {
    await tick();
    if (rowMenu && rowMenuElement) {
      rowMenu = positionedMenu(rowMenu, rowMenuElement);
      rowSubmenuOpensLeft = rowMenu.x + rowMenuElement.getBoundingClientRect().width + 154 > window.innerWidth;
    }
    if (backgroundMenu && backgroundMenuElement) {
      backgroundMenu = positionedMenu(backgroundMenu, backgroundMenuElement);
    }
  }

  function positionedMenu<T extends { x: number; y: number }>(menu: T, element: HTMLElement): T {
    const rect = element.getBoundingClientRect();
    return {
      ...menu,
      ...positionContextMenuInViewport(
        menu,
        { width: rect.width, height: rect.height },
        { width: window.innerWidth, height: window.innerHeight }
      )
    };
  }

  function focusDetailsGrid() {
    window.requestAnimationFrame(() => detailsGrid?.focus());
  }

  function updateDetailsViewport() {
    window.requestAnimationFrame(() => {
      if (!detailsBody) {
        detailsBodyScrollTop = 0;
        detailsBodyHeight = 0;
        return;
      }
      detailsBodyScrollTop = detailsBody.scrollTop;
      detailsBodyHeight = detailsBody.getBoundingClientRect().height;
    });
  }

  function handleDetailsBodyScroll() {
    detailsBodyScrollTop = detailsBody?.scrollTop ?? 0;
    detailsBodyHeight = detailsBody?.getBoundingClientRect().height ?? 0;
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
    const normalizedIndex = Math.max(0, Math.min(entries.length - 1, index));
    const entry = entries[normalizedIndex];
    if (entry) {
      stackState = selectStackEntry(stackState, entry.path, range ? 'range' : 'single');
      scrollEntryIndexIntoView(normalizedIndex);
    }
  }

  function selectedIndex() {
    return entries.findIndex((entry) => entry.path === stackState.selectedPath);
  }

  function scrollEntryIndexIntoView(index: number) {
    if (!detailsBody || index < 0 || !entries.length) {
      return;
    }
    const viewportHeight = detailsBodyHeight || detailsBody.getBoundingClientRect().height;
    const nextScrollTop = stackBrowserScrollTopForIndex(
      index,
      detailsBody.scrollTop,
      viewportHeight,
      entries.length
    );
    if (nextScrollTop !== detailsBody.scrollTop) {
      detailsBody.scrollTop = nextScrollTop;
      detailsBodyScrollTop = nextScrollTop;
      detailsBodyHeight = viewportHeight;
    }
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
    void positionOpenMenus();
  }

  function handleBackgroundContextMenu(event: MouseEvent) {
    event.preventDefault();
    rowMenu = null;
    backgroundMenu = { x: event.clientX, y: event.clientY };
    void positionOpenMenus();
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
      scrollEntryIndexIntoView(entries.findIndex((entry) => entry.path === path));
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
      scrollEntryIndexIntoView(next);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    const target = event.target instanceof HTMLElement ? event.target : null;
    if (target?.closest('.inline-editor') && event.key !== 'Escape') {
      return;
    }

    if (deleteConfirmation) {
      if (event.key === 'Escape') {
        event.preventDefault();
        cancelDeleteConfirmation();
      }
      return;
    }

    if (event.key === 'Escape') {
      event.preventDefault();
      if (createFolderDraft !== null || renameDraft !== null) {
        cancelInlineEditor();
      } else if (rowMenu || backgroundMenu) {
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
      beginCreateFolder();
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
      beginRenameSelected();
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

  function beginResize(event: PointerEvent) {
    event.preventDefault();
    event.stopPropagation();
    resizeDrag = {
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      startWidth: window.innerWidth,
      startHeight: window.innerHeight
    };
    resizeGrip?.setPointerCapture(event.pointerId);
  }

  function handleResizePointerMove(event: PointerEvent) {
    if (!resizeDrag || event.pointerId !== resizeDrag.pointerId) {
      return;
    }

    event.preventDefault();
    const width = Math.max(
      STACK_POPUP_MIN_WIDTH,
      Math.round(resizeDrag.startWidth + event.clientX - resizeDrag.startX)
    );
    const height = Math.max(
      STACK_POPUP_MIN_HEIGHT,
      Math.round(resizeDrag.startHeight + event.clientY - resizeDrag.startY)
    );
    scheduleResize(width, height, false);
  }

  function endResize(event: PointerEvent) {
    if (!resizeDrag || event.pointerId !== resizeDrag.pointerId) {
      return;
    }

    event.preventDefault();
    resizeGrip?.releasePointerCapture(event.pointerId);
    const width = Math.max(
      STACK_POPUP_MIN_WIDTH,
      Math.round(resizeDrag.startWidth + event.clientX - resizeDrag.startX)
    );
    const height = Math.max(
      STACK_POPUP_MIN_HEIGHT,
      Math.round(resizeDrag.startHeight + event.clientY - resizeDrag.startY)
    );
    resizeDrag = null;
    scheduleResize(width, height, true);
  }

  function scheduleResize(width: number, height: number, persist: boolean) {
    pendingResize = { width, height, persist: pendingResize?.persist || persist };
    if (resizeFrame !== null) {
      return;
    }

    resizeFrame = window.requestAnimationFrame(() => {
      resizeFrame = null;
      const request = pendingResize;
      pendingResize = null;
      if (!request) {
        return;
      }
      resizeRequestChain = resizeRequestChain
        .catch(() => undefined)
        .then(() => resizeStackPopup(request.width, request.height, request.persist))
        .then(() => updateDetailsViewport())
        .catch((error) => {
          console.error('Failed to resize stack popup', error);
          errorMessage = operationErrorMessage(error, 'Resize unavailable');
        });
    });
  }
</script>

<svelte:window
  on:keydown={handleKeydown}
  on:click={closeMenus}
  on:mousedown={handleMouseNavigation}
  on:pointermove={handleResizePointerMove}
  on:pointerup={endResize}
  on:pointercancel={endResize}
  on:resize={updateDetailsViewport}
/>

<section class:resizing={!!resizeDrag} class="stack-popup" aria-label="Stack browser" aria-busy={loadingPath ? 'true' : 'false'}>
  <header class="stack-toolbar">
    <div class="stack-path" title={currentPath}>
      {#if currentPath}
        <nav class="breadcrumbs" aria-label="Path breadcrumbs">
          {#each breadcrumbOverflow.visibleSegments as crumb, i (crumb.path)}
            <button type="button" class="crumb" aria-current={crumb.path === currentPath ? 'page' : undefined} on:click={() => void openFolder(crumb.path)}>{crumb.name}</button>
            {#if i === 0 && breadcrumbOverflow.hiddenCount}
              <span class="crumb-sep">/</span>
              <span class="crumb-overflow" title={breadcrumbOverflow.hiddenTitle} aria-label={`${breadcrumbOverflow.hiddenCount} collapsed path segments`}>...</span>
            {/if}
            {#if i < breadcrumbOverflow.visibleSegments.length - 1}
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
      <button type="button" disabled={!selectedEntry} on:click={beginRenameSelected}>Rename</button>
      <button type="button" disabled={!hasSelection} on:click={() => void deleteSelected()}>Delete</button>
      <button type="button" disabled={!currentPath} on:click={beginCreateFolder}>New Folder</button>
      <button type="button" disabled={!selectedEntry} on:click={() => void revealSelected()}>Reveal</button>
    </div>
  </header>

  <div class="stack-status surface-state" class:error={!!errorMessage} class:info={!errorMessage} role="status" aria-live="polite">
    <span>{errorMessage || stackState.statusMessage}</span>
    {#if loadingPath}
      <span>Loading...</span>
    {/if}
  </div>

  {#if createFolderDraft !== null || renameDraft !== null}
    <form
      class="inline-editor"
      on:submit|preventDefault={() => createFolderDraft !== null ? void createFolder() : void renameSelected()}
    >
      <label for="stack-inline-editor">{createFolderDraft !== null ? 'New folder name' : 'Rename item'}</label>
      <input
        id="stack-inline-editor"
        bind:this={editorInput}
        value={createFolderDraft ?? renameDraft ?? ''}
        on:input={(event) => {
          const value = event.currentTarget.value;
          if (createFolderDraft !== null) {
            createFolderDraft = value;
          } else {
            renameDraft = value;
          }
        }}
        on:click|stopPropagation
        on:mousedown|stopPropagation
      />
      <button type="submit">OK</button>
      <button type="button" on:click={cancelInlineEditor}>Cancel</button>
    </form>
  {/if}

  <div
    class="details-table"
    role="grid"
    aria-label="Folder details"
    aria-busy={loadingPath ? 'true' : 'false'}
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
      <div
        class="details-body"
        bind:this={detailsBody}
        on:scroll={handleDetailsBodyScroll}
      >
        {#if virtualEntries.beforeHeight}
          <div class="virtual-spacer" style={`height:${virtualEntries.beforeHeight}px`} aria-hidden="true"></div>
        {/if}
        {#each virtualEntries.rows as virtualRow (virtualRow.item.id)}
          {@const entry = virtualRow.item}
          {@const fileIcon = stackFileIconForEntry(entry)}
          <button
            class:selected={stackState.selectedPaths.includes(entry.path)}
            class:subdued={entry.isHidden || entry.isSystem}
            class:readonly={entry.isReadonly}
            class:linked={entry.isSymlink || entry.isReparsePoint}
            class:retained={hasRetainedRows}
            type="button"
            role="row"
            aria-rowindex={virtualRow.index + 2}
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
        {#if virtualEntries.afterHeight}
          <div class="virtual-spacer" style={`height:${virtualEntries.afterHeight}px`} aria-hidden="true"></div>
        {/if}
      </div>
    {:else}
      <div class="empty-stack surface-state" class:loading={!!loadingPath} class:info={!loadingPath} role="status">{loadingPath ? 'Loading folder...' : stackState.statusMessage}</div>
    {/if}
  </div>

  {#if rowMenu}
    <div
      class="context-menu"
      style={`left:${rowMenu.x}px;top:${rowMenu.y}px`}
      role="menu"
      tabindex="-1"
      bind:this={rowMenuElement}
      on:click|stopPropagation
      on:keydown={(event) => event.key === 'Escape' && closeMenus()}
    >
      <button type="button" role="menuitem" disabled={!selectedEntry} on:click={() => selectedEntry && void activateEntry(selectedEntry)}>Open</button>
      <div class:left={rowSubmenuOpensLeft} class="context-submenu" role="none">
        <button type="button" class="submenu-trigger" role="menuitem" aria-haspopup="menu" disabled={selectedEntry?.entryType !== 'File'}>Open with ▸</button>
        <div class="context-menu context-submenu-panel" role="menu">
          <button type="button" role="menuitem" disabled={selectedEntry?.entryType !== 'File'} on:click={() => void openSelectedWithPicker()}>Choose app...</button>
        </div>
      </div>
      <button type="button" role="menuitem" disabled={!hasSelection} on:click={() => void copySelected(false)}>Copy</button>
      <button type="button" role="menuitem" disabled={!hasSelection} on:click={() => void copySelected(true)}>Cut</button>
      <button type="button" role="menuitem" disabled={selectedEntry?.entryType !== 'Folder'} on:click={() => void pinSelectedFolderToTopBar()}>Pin to Top Bar</button>
      <button type="button" role="menuitem" disabled={!selectedEntry} on:click={beginRenameSelected}>Rename</button>
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
      bind:this={backgroundMenuElement}
      on:click|stopPropagation
      on:keydown={(event) => event.key === 'Escape' && closeMenus()}
    >
      <button type="button" role="menuitem" disabled={!currentPath} on:click={() => void pasteIntoCurrentFolder()}>Paste</button>
      <button type="button" role="menuitem" disabled={!currentPath} on:click={beginCreateFolder}>New Folder</button>
    </div>
  {/if}

  {#if deleteConfirmation}
    <div class="delete-confirm-backdrop" role="presentation" on:click|stopPropagation>
      <div
        class="delete-confirm-dialog"
        role="dialog"
        tabindex="-1"
        aria-modal="true"
        aria-labelledby="stack-delete-confirm-title"
        aria-describedby="stack-delete-confirm-message"
      >
        <h2 id="stack-delete-confirm-title">{deleteConfirmation.title}</h2>
        <p id="stack-delete-confirm-message">{deleteConfirmation.message}</p>
        <div class="delete-confirm-actions">
          <button type="button" bind:this={deleteCancelButton} on:click={cancelDeleteConfirmation}>Cancel</button>
          <button type="button" class="danger" on:click={() => void confirmDeleteSelection()}>Delete</button>
        </div>
      </div>
    </div>
  {/if}

  <button
    type="button"
    class="stack-resize-grip"
    aria-label="Resize Stack Browser"
    title="Resize Stack Browser"
    bind:this={resizeGrip}
    on:pointerdown={beginResize}
    on:click|stopPropagation
  ></button>
</section>
