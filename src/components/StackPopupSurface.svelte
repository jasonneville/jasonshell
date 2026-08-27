<script lang="ts">
  import './StackPopupSurface.css';
  import { emit, emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { DragDropEvent } from '@tauri-apps/api/webview';
  import { onMount, tick } from 'svelte';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import StackGitPanel from './StackGitPanel.svelte';
  import StackTerminalPane from './StackTerminalPane.svelte';
  import {
    beginStackPopupFocusLossHold,
    copyStackItems,
    deleteStackItem,
    endStackPopupFocusLossHold,
    emitStackFolderListingDiagnostics,
    extractStackArchive,
    getStackGitStatus,
    getStackPopupRequest,
    hideStackPopup,
    listStackOpenWithCandidates,
    listStackFolder,
    newStackFolder,
    newStackTextFile,
    openStackItem,
    openStackItemWithApp,
    openStackItemWithPicker,
    openStackFolderInVscode,
    openStackGitRemoteUrl,
    openStackTerminalHere,
    pinStackFolder,
    pasteStackItems,
    prepareStackFileDrag,
    renameStackItem,
    resolveStackItemIcons,
    resizeStackPopup,
    revealStackItem,
    showStackItemProperties,
    suggestStackPaths,
    STACK_POPUP_OPEN_EVENT,
    STACK_TERMINAL_PROFILE_OPTIONS,
    type StackEntry,
    type StackArchiveDestinationMode,
    type StackArchiveExtractor,
    type StackGitStatus,
    type StackGitFileStatusKind,
    type StackItemIconResolution,
    type StackFolderListing,
    type StackOpenWithCandidate,
    type StackPathSuggestion,
    type StackTerminalProfile
  } from '../lib/stackPopup';
  import { loadShellSettings } from '../lib/settings';
  import { normalizeStackTerminalProfile } from '../lib/stackPopup';
  import { folderPathToUri, folderPathsFromTransfer, normalizeDroppedPath, setFolderDragPayload } from '../lib/folderDrag';
  import {
    applyStackEntryIconUpdates,
    applyStackFolderListing,
    canNavigateStackBack,
    canNavigateStackForward,
    commitValidatedStackFolderListing,
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
    selectStackEntryPaths,
    stackPopupHasRetainedRows,
    stackIconHydrationStatus,
    stackBreadcrumbSegments,
    stackPopupOpenPath,
    stackPopupRequestKey,
    stackGitStatusPathMatchesEntry,
    stackOpenWithSuggestions,
    stackSortHeaderState,
    updateStackSort,
    type StackEntryIconUpdate,
    type StackOpenWithSuggestion,
    type StackPopupOpenPayload,
    type StackSortColumn
  } from '../lib/stackPopupState';
  import { stackFileIconForEntry } from '../lib/stackFileIcons';
  import { positionScrollableContextMenuInViewport } from '../lib/contextMenuPosition';
  import {
    STACK_BROWSER_FRONTEND_EVENTS,
    STACK_BROWSER_BACKGROUND_CONTEXT_MENU_IGNORE_SELECTORS,
    classifyStackMarqueeStartTarget,
    stackBrowserBreadcrumbOverflow,
    stackBrowserCreatedTextFileRenamePlan,
    stackBrowserDeletePrompt,
    getStackPathAutocompleteQuery,
    getStackPathInlineCompletion,
    getNextStackPathCompletionCycleIndex,
    isStackBrowsableArchiveEntry,
    stackBrowserMarqueeRect,
    stackBrowserMarqueeSelectedVirtualPaths,
    stackBrowserSearchEntries,
    stackBrowserScrollTopForIndex,
    stackBrowserVirtualWindow,
    type StackBrowserMarqueePoint,
    type StackBrowserMarqueeRect
  } from '../features/stack-browser/viewModel';
  import { topBarWebviewWindowEventTarget } from '../lib/topBarPins';

  const STACK_PATHS_DRAG_TYPE = 'application/x-jasonshell-stack-paths';
  const STACK_POPUP_MIN_WIDTH = 560;
  const STACK_POPUP_MIN_HEIGHT = 280;
  const STACK_ICON_RESOLVE_BATCH_SIZE = 24;
  const STACK_ICON_RESOLVE_MAX_CONCURRENCY = 2;
  const STACK_CONTEXT_MENU_VIEWPORT_PADDING = 8;
  const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT = 'search:toggle-centered';
  const TOP_BAR_TARGET = topBarWebviewWindowEventTarget();
  type StackContextMenuPlacement = {
    x: number;
    y: number;
    maxHeight?: number;
    width?: number;
    submenuMaxHeight?: number;
  };

  type StackBrowserViewMode = 'files' | 'terminal';
  let stackState = defaultStackPopupViewState;
  let loadingPath: string | null = null;
  let errorMessage = '';
  let rowMenu: (StackContextMenuPlacement & { path: string }) | null = null;
  let backgroundMenu: StackContextMenuPlacement | null = null;
  let rowMenuElement: HTMLDivElement | null = null;
  let backgroundMenuElement: HTMLDivElement | null = null;
  let rowSubmenuElement: HTMLDivElement | null = null;
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
  let marqueeSelection:
    | {
        pointerId: number;
        start: StackBrowserMarqueePoint;
        current: StackBrowserMarqueePoint;
        additive: boolean;
        baseSelection: string[];
        folderPath: string;
      }
    | null = null;
  let marqueeAutoscrollFrame: number | null = null;
  let pathDraft = '';
  let pathDraftBase = '';
  let pathInput: HTMLInputElement | null = null;
  let pathInputFocused = false;
  let pathBlurResetTimer: number | null = null;
  let pathSuggestions: StackPathSuggestion[] = [];
  let pathCompletionCycleIndex = -1;
  let pathSuggestionRequestSeq = 0;
  let searchQuery = '';
  let openWithCandidates: StackOpenWithCandidate[] = [];
  let openWithCandidatePath: string | null = null;
  let iconCache = new Map<string, string | null>();
  let iconHydrationJobToken = 0;
  let iconHydrationVisiblePriority: string[] = [];
  let iconHydrationPending: string[] = [];
  let iconHydrationInFlight = 0;
  let iconHydrationInFlightPathCount = 0;
  let iconHydrationResolvedCount = 0;
  let iconHydrationTargetCount = 0;
  let iconHydrationStatusMessage = '';
  let iconHydrationCacheHits = 0;
  let iconHydrationCacheMisses = 0;
  let iconHydrationFallbackCount = 0;
  let iconHydrationStartedAt = 0;
  let iconQueueCompleteDurationMs = 0;
  let iconQueueDiagnosticsEmitted = false;
  let iconDiagnosticsPath: string | null = null;
  let iconDiagnosticsFirstPaintDurationMs = 0;
  let iconDiagnosticsMetadataCompleteDurationMs = 0;
  let stackSortLockedByUser = false;
  let gitStatus: StackGitStatus | null = null;
  let gitStatusPath = '';
  let gitStatusPending: StackGitStatus | null | undefined = undefined;
  let gitStatusPendingPath = '';
  let gitStatusRequestSequence = 0;
  let gitStatusPopupOpen = false;
  let gitStatusPopupFilter: StackGitFileStatusKind | 'all' = 'all';
  let stackBrowserViewMode: StackBrowserViewMode = 'files';
  let stackTerminalProfile: StackTerminalProfile = 'windowsTerminal';
  let stackTerminalPane: StackTerminalPane | null = null;
  let stackPopupSurface: HTMLElement | null = null;
  let shellSurfaceHotkeyHandled = false;
  $: stackTerminalProfileLabel =
    STACK_TERMINAL_PROFILE_OPTIONS.find((option) => option.value === stackTerminalProfile)?.label ?? 'PowerShell';

  $: currentPath = stackState.currentPath;
  $: entries = stackState.entries;
  $: visibleEntries = stackBrowserSearchEntries(entries, searchQuery);
  $: selectedEntry = selectedStackEntry(stackState);
  $: selectedPaths = selectedStackPaths(stackState);
  $: hasSelection = selectedPaths.length > 0;
  $: canGoBack = canNavigateStackBack(stackState);
  $: canGoForward = canNavigateStackForward(stackState);
  $: breadcrumbs = stackBreadcrumbSegments(currentPath);
  $: breadcrumbOverflow = stackBrowserBreadcrumbOverflow(breadcrumbs, 5);
  $: hasRetainedRows = stackPopupHasRetainedRows(stackState);
  $: virtualEntries = stackBrowserVirtualWindow(visibleEntries, detailsBodyScrollTop, detailsBodyHeight);
  $: marqueeRect = marqueeSelection ? stackBrowserMarqueeRect(marqueeSelection.start, marqueeSelection.current) : null;
  $: openWithSuggestions = openWithCandidates.length ? openWithCandidates : stackOpenWithSuggestions(selectedEntry);
  $: if (currentPath !== pathDraftBase && (!pathInputFocused || pathDraft === pathDraftBase)) {
    pathDraft = currentPath;
    pathDraftBase = currentPath;
  }
  $: pathInlineCompletion = getStackPathInlineCompletion(
    pathDraft,
    pathSuggestions[pathCompletionCycleIndex >= 0 ? pathCompletionCycleIndex : 0]
  );

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    const keyupHandler = (event: KeyboardEvent) => {
      if (event.code !== 'Space' || !shellSurfaceHotkeyHandled) return;
      event.preventDefault();
      event.stopPropagation();
      shellSurfaceHotkeyHandled = false;
    };
    const latestRequestTimer = window.setInterval(() => {
      void reconcileLatestStackPopupRequest();
    }, 250);

    void initializeOpenRequestDelivery(unlisteners, () => disposed);
    void loadStackTerminalProfile();
    window.addEventListener('keydown', handleSearchHotkeyKeydown, true);
    window.addEventListener('keyup', keyupHandler, true);
    void getCurrentWindow().onDragDropEvent((event: { payload: DragDropEvent }) => {
      if (event.payload.type === 'drop' && currentPath && Date.now() - lastHtmlDropAt > 500) {
        void pasteDroppedPaths(event.payload.paths, currentPath, false);
      }
    }).then((unlisten: () => void) => {
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
      cancelPathBlurReset();
      if (resizeFrame !== null) {
        window.cancelAnimationFrame(resizeFrame);
      }
      window.removeEventListener('keydown', handleSearchHotkeyKeydown, true);
      window.removeEventListener('keyup', keyupHandler, true);
      stopMarqueeAutoscroll();
      const terminalPaneForCleanup = stackTerminalPane as StackTerminalPane | null;
      void terminalPaneForCleanup?.stopTerminal();
      window.clearInterval(latestRequestTimer);
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });

  async function initializeOpenRequestDelivery(unlisteners: Array<() => void>, isDisposed: () => boolean) {
    try {
      const unlisten = await getCurrentWindow().listen<StackPopupOpenPayload>(STACK_POPUP_OPEN_EVENT, (event: { payload: StackPopupOpenPayload }) => {
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
      await stopCurrentStackTerminal();
      stackBrowserViewMode = 'files';
      await openFolder(path);
      lastHandledOpenRequestKey = requestKey;
    } finally {
      if (pendingOpenRequestKey === requestKey) {
        pendingOpenRequestKey = null;
      }
    }
  }

  async function openFolder(folderPath: string, _options: { warmTerminal?: boolean } = {}) {
    closeMenus();
    prepareGitStateForFolderCommit(folderPath);
    stackState = openStackFolder(stackState, folderPath);
    await loadFolder(stackState.currentPath);
  }

  async function submitPathDraft() {
    clearPathSuggestions();
    const folderPath = pathDraft.trim();
    if (!folderPath) {
      resetPathDraft();
      return;
    }

    closeMenus();
    const loadSequence = ++folderLoadSequence;
    stackSortLockedByUser = false;
    startNewIconHydrationSession(folderPath);
    void refreshStackGitStatus(folderPath, loadSequence);
    const listingStartedAt = performance.now();
    let firstPaintDurationMs = 0;
    loadingPath = folderPath;
    errorMessage = '';
    try {
      const listing = await listStackFolder(folderPath, async (page) => {
        if (loadSequence !== folderLoadSequence) {
          return;
        }
        if (page.offset === 0 && !stackSortLockedByUser && page.sortColumn && page.sortDirection) {
          stackState = { ...stackState, sortColumn: page.sortColumn, sortDirection: page.sortDirection };
        }
        if (!firstPaintDurationMs && page.entries.length) {
          firstPaintDurationMs = Math.max(0, performance.now() - listingStartedAt);
        }
        maybeFocusDetailsGridAfterPageAppend();
      });
      if (loadSequence !== folderLoadSequence) {
        return;
      }
      iconDiagnosticsPath = folderPath;
      iconDiagnosticsFirstPaintDurationMs = firstPaintDurationMs || Math.max(0, performance.now() - listingStartedAt);
      iconDiagnosticsMetadataCompleteDurationMs = Math.max(0, performance.now() - listingStartedAt);
      prepareGitStateForFolderCommit(folderPath);
      stackState = commitValidatedStackFolderListing(stackState, folderPath, listing);
      promotePendingGitStatus(folderPath);
      scheduleVisibleIconHydration(folderPath, loadSequence);
      pathDraft = stackState.currentPath;
      pathDraftBase = stackState.currentPath;
      updateDetailsViewport();
      focusDetailsGrid();
    } catch (error) {
      console.error('Failed to open typed stack folder path', error);
      if (loadSequence === folderLoadSequence && loadingPath === folderPath) {
        errorMessage = operationErrorMessage(error, `Folder unavailable: ${folderPath}`);
      }
    } finally {
      if (loadSequence === folderLoadSequence && loadingPath === folderPath) {
        loadingPath = null;
      }
    }
  }

  function resetPathDraft() {
    pathDraft = currentPath;
    pathDraftBase = currentPath;
    clearPathSuggestions();
  }

  function handlePathKeydown(event: KeyboardEvent) {
    event.stopPropagation();
    if (pathInlineCompletion && event.key === 'ArrowRight') {
      event.preventDefault();
      acceptInlinePathCompletion();
      return;
    }
    if (event.key === 'Tab' && !event.shiftKey) {
      const caret = event.currentTarget instanceof HTMLInputElement ? (event.currentTarget.selectionStart ?? pathDraft.length) : pathDraft.length;
      if (pathSuggestions.length || getStackPathAutocompleteQuery(pathDraft, caret)) {
        event.preventDefault();
        void cyclePathCompletion(caret);
        return;
      }
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      resetPathDraft();
    }
  }

  async function refreshPathSuggestions(input: HTMLInputElement) {
    pathDraft = input.value;
    await refreshPathSuggestionsForValue(input.value, input.selectionStart ?? input.value.length);
  }

  async function refreshPathSuggestionsForValue(value: string, caret: number) {
    pathDraft = value;
    const query = getStackPathAutocompleteQuery(value, caret);
    const requestSeq = ++pathSuggestionRequestSeq;
    if (!query) {
      clearPathSuggestions();
      return false;
    }

    clearPathSuggestions(false);

    try {
      const suggestions = await suggestStackPaths({ ...query, limit: 20 });
      if (requestSeq !== pathSuggestionRequestSeq || value !== pathDraft) {
        return false;
      }
      pathCompletionCycleIndex = -1;
      pathSuggestions = suggestions;
      return suggestions.length > 0;
    } catch {
      if (requestSeq === pathSuggestionRequestSeq) {
        clearPathSuggestions();
      }
      return false;
    }
  }

  function clearPathSuggestions(invalidateRequests = true) {
    pathSuggestions = [];
    pathCompletionCycleIndex = -1;
    if (invalidateRequests) {
      pathSuggestionRequestSeq += 1;
    }
  }

  async function cyclePathCompletion(caret = pathDraft.length) {
    if (!pathSuggestions.length) {
      const refreshed = await refreshPathSuggestionsForValue(pathDraft, caret);
      if (!refreshed) {
        return;
      }
    }
    cancelPathBlurReset();
    // Keep the original suggestion set so repeated Tab walks sibling matches after the draft becomes a full candidate.
    pathCompletionCycleIndex = getNextStackPathCompletionCycleIndex(pathDraft, pathSuggestions, pathCompletionCycleIndex);
    const suggestion = pathSuggestions[pathCompletionCycleIndex];
    const committedPath = suggestion.path;
    pathDraft = committedPath;
    void focusPathInput(committedPath.length);
  }

  function acceptInlinePathCompletion() {
    if (!pathInlineCompletion) {
      return;
    }
    cancelPathBlurReset();
    const committedPath = pathInlineCompletion.commitPath;
    pathDraft = pathInlineCompletion.commitPath;
    clearPathSuggestions();
    void focusPathInput(committedPath.length);
    void refreshPathSuggestionsForValue(committedPath, committedPath.length);
  }

  function schedulePathBlurReset() {
    cancelPathBlurReset();
    pathBlurResetTimer = window.setTimeout(() => {
      pathBlurResetTimer = null;
      if (!pathInputFocused) {
        resetPathDraft();
      }
    }, 100);
  }

  function cancelPathBlurReset() {
    if (pathBlurResetTimer !== null) {
      window.clearTimeout(pathBlurResetTimer);
      pathBlurResetTimer = null;
    }
  }

  async function focusPathInput(caret = pathDraft.length) {
    await tick();
    pathInput?.focus();
    pathInput?.setSelectionRange(caret, caret);
  }

  async function loadFolder(folderPath: string) {
    if (!folderPath) {
      return;
    }

    const loadSequence = ++folderLoadSequence;
    stackSortLockedByUser = false;
    startNewIconHydrationSession(folderPath);
    const listingStartedAt = performance.now();
    let firstPaintDurationMs = 0;
    let mergedListing: StackFolderListing | null = null;
    loadingPath = folderPath;
    errorMessage = '';
    void refreshStackGitStatus(folderPath, loadSequence);
    try {
      const listing = await listStackFolder(folderPath, async (page) => {
        if (loadSequence !== folderLoadSequence) {
          return;
        }

        mergedListing = mergeStackFolderListings(mergedListing, page);
        if (page.offset === 0 && !stackSortLockedByUser && page.sortColumn && page.sortDirection) {
          stackState = { ...stackState, sortColumn: page.sortColumn, sortDirection: page.sortDirection };
        }
        if (!firstPaintDurationMs && mergedListing.entries.length > 0) {
          firstPaintDurationMs = Math.max(0, performance.now() - listingStartedAt);
        }
        prepareGitStateForFolderCommit(folderPath);
        stackState = applyStackFolderListing(stackState, folderPath, mergedListing);
        promotePendingGitStatus(folderPath);
        scheduleVisibleIconHydration(folderPath, loadSequence);
        updateDetailsViewport();
        maybeFocusDetailsGridAfterPageAppend();
      });
      if (loadSequence !== folderLoadSequence) {
        return;
      }
      iconDiagnosticsPath = folderPath;
      iconDiagnosticsFirstPaintDurationMs = firstPaintDurationMs || Math.max(0, performance.now() - listingStartedAt);
      iconDiagnosticsMetadataCompleteDurationMs = Math.max(0, performance.now() - listingStartedAt);
      prepareGitStateForFolderCommit(folderPath);
      stackState = applyStackFolderListing(stackState, folderPath, mergedListing ?? listing);
      promotePendingGitStatus(folderPath);
      scheduleVisibleIconHydration(folderPath, loadSequence);
      updateDetailsViewport();
      maybeFocusDetailsGridAfterPageAppend();
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

  async function refreshStackGitStatus(folderPath: string, loadSequence: number) {
    const requestSequence = ++gitStatusRequestSequence;
    gitStatusPending = undefined;
    gitStatusPendingPath = folderPath;
    try {
      const status = await getStackGitStatus(folderPath);
      if (
        requestSequence !== gitStatusRequestSequence
        || loadSequence !== folderLoadSequence
        || folderPath !== gitStatusPendingPath
      ) {
        return;
      }
      if (folderPath === stackState.currentPath) {
        gitStatus = status;
        gitStatusPath = folderPath;
        gitStatusPending = undefined;
        gitStatusPendingPath = '';
      } else {
        gitStatusPending = status;
      }
    } catch (error) {
      if (requestSequence === gitStatusRequestSequence && loadSequence === folderLoadSequence) {
        console.debug('Stack git status unavailable', error);
        gitStatusPending = undefined;
        gitStatusPendingPath = '';
      }
    }
  }

  function promotePendingGitStatus(folderPath: string) {
    if (folderPath !== gitStatusPendingPath || gitStatusPending === undefined) {
      return;
    }
    gitStatus = gitStatusPending;
    gitStatusPath = folderPath;
    gitStatusPending = undefined;
    gitStatusPendingPath = '';
  }

  function prepareGitStateForFolderCommit(folderPath: string) {
    if (folderPath === currentPath) {
      return;
    }
    gitStatusPopupOpen = false;
  }

  function startNewIconHydrationSession(folderPath: string) {
    iconHydrationJobToken += 1;
    iconHydrationVisiblePriority = [];
    iconHydrationPending = [];
    iconHydrationInFlight = 0;
    iconHydrationInFlightPathCount = 0;
    iconHydrationResolvedCount = 0;
    iconHydrationTargetCount = 0;
    iconHydrationStatusMessage = '';
    iconHydrationCacheHits = 0;
    iconHydrationCacheMisses = 0;
    iconHydrationFallbackCount = 0;
    iconHydrationStartedAt = performance.now();
    iconQueueCompleteDurationMs = 0;
    iconQueueDiagnosticsEmitted = false;
    iconDiagnosticsPath = folderPath;
    iconDiagnosticsFirstPaintDurationMs = 0;
    iconDiagnosticsMetadataCompleteDurationMs = 0;
  }

  function scheduleVisibleIconHydration(folderPath: string, loadSequence: number) {
    if (loadSequence !== folderLoadSequence || folderPath !== stackState.currentPath) {
      return;
    }

    const cachedUpdates: StackEntryIconUpdate[] = [];
    const pending = new Set([...iconHydrationVisiblePriority, ...iconHydrationPending]);
    let targetCount = 0;
    for (const entry of stackState.entries) {
      if (entry.iconDataUrl) {
        continue;
      }
      const cacheKey = normalizeIconCacheKey(entry.path);
      if (!cacheKey) {
        continue;
      }
      targetCount += 1;
      if (iconCache.has(cacheKey)) {
        const cachedIcon = iconCache.get(cacheKey) ?? null;
        if (cachedIcon) {
          cachedUpdates.push({ path: entry.path, iconDataUrl: cachedIcon });
        }
        continue;
      }
      pending.add(entry.path);
    }

    if (cachedUpdates.length) {
      stackState = applyStackEntryIconUpdates(stackState, folderPath, cachedUpdates);
    }

    iconHydrationPending = mergeIconHydrationPending([], [...pending]);
    queueVisibleIconHydrationPriority(folderPath, loadSequence);
    iconHydrationTargetCount = targetCount;
    iconHydrationResolvedCount = resolvedIconHydrationCount();
    updateIconHydrationStatusMessage();
    void drainIconHydrationQueue(folderPath, loadSequence, iconHydrationJobToken);
  }

  async function drainIconHydrationQueue(folderPath: string, loadSequence: number, jobToken: number) {
    while (
      loadSequence === folderLoadSequence
      && folderPath === stackState.currentPath
      && jobToken === iconHydrationJobToken
      && iconHydrationInFlight < STACK_ICON_RESOLVE_MAX_CONCURRENCY
      && (iconHydrationVisiblePriority.length > 0 || iconHydrationPending.length > 0)
    ) {
      const batchPaths = nextIconHydrationBatch();
      iconHydrationInFlight += 1;
      iconHydrationInFlightPathCount += batchPaths.length;
      void resolveIconBatch(folderPath, loadSequence, jobToken, batchPaths);
    }
  }

  function nextIconHydrationBatch() {
    const priorityBatch = iconHydrationVisiblePriority.slice(0, STACK_ICON_RESOLVE_BATCH_SIZE);
    iconHydrationVisiblePriority = iconHydrationVisiblePriority.slice(priorityBatch.length);
    const remaining = STACK_ICON_RESOLVE_BATCH_SIZE - priorityBatch.length;
    if (remaining <= 0) {
      return priorityBatch;
    }
    const backlogBatch = iconHydrationPending
      .filter((path) => !priorityBatch.includes(path))
      .slice(0, remaining);
    iconHydrationPending = iconHydrationPending.filter(
      (path) => !priorityBatch.includes(path) && !backlogBatch.includes(path)
    );
    return [...priorityBatch, ...backlogBatch];
  }

  function queueVisibleIconHydrationPriority(folderPath: string, loadSequence: number) {
    if (loadSequence !== folderLoadSequence || folderPath !== stackState.currentPath) {
      return;
    }
    const visiblePaths = stackBrowserVirtualWindow(visibleEntries, detailsBodyScrollTop, detailsBodyHeight)
      .rows
      .map((row) => row.item)
      .filter((entry) => !entry.iconDataUrl && !iconCache.has(normalizeIconCacheKey(entry.path)))
      .map((entry) => entry.path);
    iconHydrationVisiblePriority = mergeIconHydrationPending(iconHydrationVisiblePriority, visiblePaths);
    iconHydrationPending = iconHydrationPending.filter((path) => !iconHydrationVisiblePriority.includes(path));
    void drainIconHydrationQueue(folderPath, loadSequence, iconHydrationJobToken);
  }

  function mergeIconHydrationPending(existing: string[], incoming: string[]) {
    const seen = new Set<string>();
    const merged: string[] = [];
    for (const path of [...existing, ...incoming]) {
      const key = normalizeIconCacheKey(path);
      if (!key || iconCache.has(key) || seen.has(key)) {
        continue;
      }
      seen.add(key);
      merged.push(path);
    }
    return merged;
  }

  async function resolveIconBatch(
    folderPath: string,
    loadSequence: number,
    jobToken: number,
    paths: string[]
  ) {
    try {
      const batch = await resolveStackItemIcons(paths);
      iconHydrationCacheHits += batch.cacheHits;
      iconHydrationCacheMisses += batch.cacheMisses;
      iconHydrationFallbackCount += batch.items.filter((item) => !item.iconDataUrl).length;
      const updates = stackIconUpdatesFromBatch(batch.items);
      for (const item of batch.items) {
        const cacheKey = normalizeIconCacheKey(item.path);
        if (!cacheKey) {
          continue;
        }
        iconCache.set(cacheKey, item.iconDataUrl ?? null);
      }
      if (
        updates.length
        && loadSequence === folderLoadSequence
        && folderPath === stackState.currentPath
        && jobToken === iconHydrationJobToken
      ) {
        stackState = applyStackEntryIconUpdates(stackState, folderPath, updates);
      }
    } catch (error) {
      console.error('Failed to resolve stack row icons', error);
    } finally {
      if (jobToken !== iconHydrationJobToken) {
        return;
      }
      iconHydrationInFlight = Math.max(0, iconHydrationInFlight - 1);
      iconHydrationInFlightPathCount = Math.max(0, iconHydrationInFlightPathCount - paths.length);
      iconHydrationResolvedCount = resolvedIconHydrationCount();
      updateIconHydrationStatusMessage();
      maybeEmitIconQueueCompletionDiagnostics(folderPath, loadSequence, jobToken);
      await drainIconHydrationQueue(folderPath, loadSequence, jobToken);
    }
  }

  function stackIconUpdatesFromBatch(items: StackItemIconResolution[]): StackEntryIconUpdate[] {
    return items
      .filter((item) => Boolean(item.path && item.iconDataUrl))
      .map((item) => ({
        path: item.path,
        iconDataUrl: item.iconDataUrl
      }));
  }

  function unresolvedIconHydrationCount() {
    return iconHydrationPending.length + iconHydrationVisiblePriority.length + iconHydrationInFlightPathCount;
  }

  function resolvedIconHydrationCount() {
    return Math.max(0, iconHydrationTargetCount - unresolvedIconHydrationCount());
  }

  function normalizeIconCacheKey(path: string) {
    const trimmed = path.trim();
    if (!trimmed) {
      return '';
    }
    return trimmed.replace(/\//g, '\\').toLocaleLowerCase();
  }

  function updateIconHydrationStatusMessage() {
    iconHydrationStatusMessage = stackIconHydrationStatus(
      iconHydrationResolvedCount,
      iconHydrationTargetCount
    );
  }

  function maybeEmitIconQueueCompletionDiagnostics(
    folderPath: string,
    loadSequence: number,
    jobToken: number
  ) {
    if (
      iconQueueDiagnosticsEmitted
      || jobToken !== iconHydrationJobToken
      || loadSequence !== folderLoadSequence
      || folderPath !== stackState.currentPath
      || iconHydrationPending.length > 0
      || iconHydrationVisiblePriority.length > 0
      || iconHydrationInFlight > 0
      || !iconHydrationTargetCount
      || iconDiagnosticsPath !== folderPath
    ) {
      return;
    }

    iconQueueCompleteDurationMs = Math.max(0, performance.now() - iconHydrationStartedAt);
    iconQueueDiagnosticsEmitted = true;
    emitStackFolderListingDiagnostics({
      phase: 'icon-queue-complete',
      path: folderPath,
      pageOffset: stackState.entries.length,
      requestedLimit: 0,
      pageDurationMs: 0,
      folderOpenDurationMs: iconDiagnosticsMetadataCompleteDurationMs,
      firstPaintDurationMs: iconDiagnosticsFirstPaintDurationMs,
      metadataListingCompleteDurationMs: iconDiagnosticsMetadataCompleteDurationMs,
      iconQueueCompleteDurationMs,
      pageItemCount: stackState.entries.length,
      iconResolutionCount: iconHydrationResolvedCount,
      iconResolutionDurationMs: iconQueueCompleteDurationMs,
      iconCacheHits: iconHydrationCacheHits,
      iconCacheMisses: iconHydrationCacheMisses,
      iconFallbackCount: iconHydrationFallbackCount,
      payloadItemCount: stackState.entries.length,
      totalItems: stackState.entries.length,
      hasMore: false
    });
  }

  async function navigateHistory(direction: -1 | 1) {
    const nextState = navigateStackHistory(stackState, direction);
    prepareGitStateForFolderCommit(nextState.currentPath);
    stackState = nextState;
    await loadFolder(stackState.currentPath);
  }

  async function activateEntry(entry: StackEntry) {
    closeMenus();
    if (entry.entryType === 'Folder' || isStackBrowsableArchiveEntry(entry)) {
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

  async function loadOpenWithCandidates(entry: StackEntry) {
    openWithCandidatePath = entry.path;
    openWithCandidates = [];
    if (entry.entryType !== 'File') {
      return;
    }

    try {
      const candidates = await listStackOpenWithCandidates(entry.path);
      if (openWithCandidatePath === entry.path) {
        openWithCandidates = candidates;
      }
    } catch (error) {
      console.error('Failed to load Open With candidates', error);
    }
  }

  async function openSelectedWithSuggestedApp(app: StackOpenWithSuggestion | StackOpenWithCandidate) {
    closeMenus();
    if (!selectedEntry || selectedEntry.entryType !== 'File') {
      return;
    }
    try {
      await openStackItemWithApp(selectedEntry.path, app.id);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to open stack item with app', error);
      errorMessage = operationErrorMessage(error, `Open with ${app.label} unavailable`);
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

  async function beginCreateTextFile() {
    closeMenus();
    if (!currentPath) {
      return;
    }
    try {
      const created = await newStackTextFile(currentPath);
      const listing = await listStackFolder(currentPath);
      stackState = applyStackFolderListing(stackState, currentPath, listing);
      const renamePlan = stackBrowserCreatedTextFileRenamePlan(created);
      stackState = selectStackEntry(stackState, renamePlan.selectedPath);
      createFolderDraft = null;
      renameDraft = renamePlan.renameDraft;
      errorMessage = '';
      updateDetailsViewport();
      if (renamePlan.focusTarget === 'inline-editor') {
        focusEditorInput();
      }
    } catch (error) {
      console.error('Failed to create text file', error);
      errorMessage = operationErrorMessage(error, 'New Text File unavailable');
    }
  }

  async function openTerminalHere() {
    closeMenus();
    if (!currentPath) {
      return;
    }
    try {
      await openStackTerminalHere(currentPath);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to open terminal here', error);
      errorMessage = operationErrorMessage(error, 'Open Terminal Here unavailable');
    }
  }

  async function loadStackTerminalProfile() {
    try {
      const settings = await loadShellSettings();
      stackTerminalProfile = normalizeStackTerminalProfile(settings.stackBrowser?.terminalProfile);
    } catch (error) {
      console.debug('Stack terminal profile unavailable', error);
      stackTerminalProfile = 'windowsTerminal';
    }
  }

  async function switchStackBrowserView(mode: StackBrowserViewMode) {
    closeMenus();
    if (mode === 'files') {
      await stackTerminalPane?.syncFolderToTerminalCwd();
      stackBrowserViewMode = 'files';
      return;
    }
    stackBrowserViewMode = 'terminal';
    await tick();
    await stackTerminalPane?.startTerminal(true);
  }

  async function ensureStackTerminal() {
    if (!currentPath) {
      return;
    }
    await tick();
    await stackTerminalPane?.startTerminal(true);
  }

  async function warmStackTerminalForCurrentFolder() {
    if (!currentPath) {
      return;
    }
    await tick();
    await stackTerminalPane?.startTerminal(false);
  }

  async function restartStackTerminal(focusAfterStart = false) {
    await loadStackTerminalProfile();
    await tick();
    await stackTerminalPane?.startTerminal(focusAfterStart || stackBrowserViewMode === 'terminal');
  }

  function focusStackTerminalInput() {
    stackTerminalPane?.focusTerminal();
  }

  async function stopCurrentStackTerminal() {
    await stackTerminalPane?.stopTerminal();
  }

  async function handleStackTerminalCwdChange(cwd: string) {
    if (stackBrowserViewMode === 'terminal' && cwd && cwd !== currentPath) {
      await openFolder(cwd, { warmTerminal: false });
    }
  }

  async function closeStackPopupFromSurface() {
    await stopCurrentStackTerminal();
    stackBrowserViewMode = 'files';
    await hideStackPopup();
  }

  async function openSelectedFolderInVscode() {
    closeMenus();
    if (!selectedEntry || selectedEntry.entryType !== 'Folder') {
      return;
    }
    try {
      await openStackFolderInVscode(selectedEntry.path);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to open folder in VS Code', error);
      errorMessage = operationErrorMessage(error, 'Open in VS Code unavailable');
    }
  }

  async function openCurrentFolderInVscode() {
    closeMenus();
    if (!currentPath) {
      return;
    }
    try {
      await openStackFolderInVscode(currentPath);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to open current folder in VS Code', error);
      errorMessage = operationErrorMessage(error, 'Open in VS Code unavailable');
    }
  }

  async function copyTextToClipboard(text: string, fallback: string) {
    closeMenus();
    if (!text) {
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to copy stack browser text', error);
      errorMessage = operationErrorMessage(error, fallback);
    }
  }

  function selectedDirectoryPath() {
    if (!selectedEntry) {
      return currentPath;
    }
    if (selectedEntry.entryType === 'Folder') {
      return selectedEntry.path;
    }
    return parentStackPath(selectedEntry.path) || currentPath;
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

  async function showSelectedProperties() {
    closeMenus();
    if (!selectedEntry) {
      return;
    }
    try {
      await showStackItemProperties(selectedEntry.path);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to show stack item properties', error);
      errorMessage = operationErrorMessage(error, 'Properties unavailable');
    }
  }

  async function showCurrentFolderProperties() {
    closeMenus();
    if (!currentPath) {
      return;
    }
    try {
      await showStackItemProperties(currentPath);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to show current folder properties', error);
      errorMessage = operationErrorMessage(error, 'Properties unavailable');
    }
  }

  function selectedArchiveEntry() {
    if (!selectedEntry || selectedEntry.entryType !== 'File' || !/\.(zip|rar)$/i.test(selectedEntry.name)) {
      return null;
    }
    return selectedEntry;
  }

  function selectedZipArchiveEntry() {
    const archive = selectedArchiveEntry();
    return archive && /\.zip$/i.test(archive.name) ? archive : null;
  }

  function selectedSevenZipArchiveEntry() {
    return selectedArchiveEntry();
  }

  async function extractSelectedArchive(destinationMode: StackArchiveDestinationMode, extractor: StackArchiveExtractor = 'builtin') {
    closeMenus();
    const archive = selectedArchiveEntry();
    if (!archive) {
      return;
    }

    try {
      await extractStackArchive(archive.path, destinationMode, extractor);
      const listing = await listStackFolder(currentPath);
      stackState = applyStackFolderListing(stackState, currentPath, listing);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to extract stack archive', error);
      errorMessage = operationErrorMessage(error, 'Extract archive unavailable');
    }
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

  function openGitStatusPopup(filter: StackGitFileStatusKind | 'all' = 'all') {
    closeMenus();
    gitStatusPopupFilter = filter;
    gitStatusPopupOpen = true;
  }

  function closeGitStatusPopup() {
    gitStatusPopupOpen = false;
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
      await tick();
      if (rowMenu && rowMenuElement) {
        rowMenu = positionedSubmenu(rowMenu, rowMenuElement);
      }
    }
    if (backgroundMenu && backgroundMenuElement) {
      backgroundMenu = positionedMenu(backgroundMenu, backgroundMenuElement);
    }
  }

  function positionedMenu<T extends StackContextMenuPlacement>(menu: T, element: HTMLElement): T {
    const rect = element.getBoundingClientRect();
    const placement = positionScrollableContextMenuInViewport(
      menu,
      { width: rect.width, height: rect.height },
      { width: window.innerWidth, height: window.innerHeight },
      STACK_CONTEXT_MENU_VIEWPORT_PADDING
    );
    return {
      ...menu,
      ...placement,
      width: rect.width,
      submenuMaxHeight: placement.maxHeight
    };
  }

  function positionedSubmenu<T extends StackContextMenuPlacement>(menu: T, element: HTMLElement): T {
    const rect = element.getBoundingClientRect();
    const triggerTop = rowSubmenuElement?.getBoundingClientRect().top ?? rect.top;
    const maxHeight = availableContextMenuHeightFromTop(triggerTop);
    return {
      ...menu,
      width: rect.width,
      submenuMaxHeight: maxHeight
    };
  }

  function availableContextMenuHeight() {
    return Math.max(0, window.innerHeight - STACK_CONTEXT_MENU_VIEWPORT_PADDING * 2);
  }

  function availableContextMenuHeightFromTop(top: number) {
    return Math.max(
      0,
      window.innerHeight - Math.max(top, STACK_CONTEXT_MENU_VIEWPORT_PADDING) - STACK_CONTEXT_MENU_VIEWPORT_PADDING
    );
  }

  function contextMenuMaxHeightCss(menu: StackContextMenuPlacement) {
    return `${Math.max(0, Math.round(menu.maxHeight ?? availableContextMenuHeight()))}px`;
  }

  function contextMenuWidthCss(menu: StackContextMenuPlacement) {
    return `${Math.max(0, Math.round(menu.width ?? 0))}px`;
  }

  function contextSubmenuMaxHeightCss(menu: StackContextMenuPlacement) {
    return `${Math.max(0, Math.round(menu.submenuMaxHeight ?? menu.maxHeight ?? availableContextMenuHeight()))}px`;
  }

  function focusDetailsGrid() {
    window.requestAnimationFrame(() => detailsGrid?.focus());
  }

  function maybeFocusDetailsGridAfterPageAppend() {
    if (!detailsGrid) {
      return;
    }
    const active = document.activeElement;
    if (active && (active === detailsGrid || detailsGrid.contains(active))) {
      return;
    }
    focusDetailsGrid();
  }

  function updateDetailsViewport() {
    window.requestAnimationFrame(() => {
      if (!detailsBody) {
        detailsBodyScrollTop = 0;
        detailsBodyHeight = 0;
        emitVisibleRowsWindowChanged();
        return;
      }
      detailsBodyScrollTop = detailsBody.scrollTop;
      detailsBodyHeight = detailsBody.getBoundingClientRect().height;
      emitVisibleRowsWindowChanged();
    });
  }

  function handleDetailsBodyScroll() {
    detailsBodyScrollTop = detailsBody?.scrollTop ?? 0;
    detailsBodyHeight = detailsBody?.getBoundingClientRect().height ?? 0;
    emitVisibleRowsWindowChanged();
  }

  async function handleStackSearchInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    searchQuery = input.value;
    detailsBodyScrollTop = 0;
    if (detailsBody) {
      detailsBody.scrollTop = 0;
    }
    await tick();
    emitVisibleRowsWindowChanged();
  }

  function emitVisibleRowsWindowChanged() {
    const windowSlice = stackBrowserVirtualWindow(visibleEntries, detailsBodyScrollTop, detailsBodyHeight);
    void emit(STACK_BROWSER_FRONTEND_EVENTS.folderRowsWindowChanged, {
      path: currentPath,
      startIndex: windowSlice.startIndex,
      endIndex: windowSlice.endIndex,
      totalRows: visibleEntries.length
    }).catch(() => undefined);
    queueVisibleIconHydrationPriority(currentPath, folderLoadSequence);
  }

  function sortBy(column: StackSortColumn) {
    stackSortLockedByUser = true;
    stackState = updateStackSort(stackState, column);
    focusDetailsGrid();
  }

  function sortHeader(column: StackSortColumn) {
    return stackSortHeaderState(stackState, column);
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

  function stackGitStatusSymbol(status: StackGitFileStatusKind | null | undefined) {
    if (status === 'added') return '+';
    if (status === 'deleted') return '-';
    if (status === 'modified') return 'M';
    if (status === 'untracked') return '?';
    if (status === 'conflict') return '!';
    return null;
  }

  function stackGitStatusLabel(status: StackGitFileStatusKind | null | undefined) {
    if (status === 'added') return 'Added';
    if (status === 'deleted') return 'Deleted';
    if (status === 'modified') return 'Modified';
    if (status === 'untracked') return 'Untracked';
    if (status === 'conflict') return 'Conflict';
    return '';
  }

  async function openGitRemoteRepository(url: string | null | undefined) {
    if (!url) return;
    try {
      await openStackGitRemoteUrl(url);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to open git remote repository', error);
      errorMessage = operationErrorMessage(error, 'Git remote unavailable');
    }
  }

  function filteredGitStatusEntries() {
    if (!gitStatus) return [];
    if (gitStatusPopupFilter === 'all') {
      return gitStatus.entries;
    }
    return gitStatus.entries.filter((entry) => entry.status === gitStatusPopupFilter);
  }

  function stackGitSummaryParts(status: StackGitStatus | null) {
    if (!status) return [];
    const parts: Array<{ status: StackGitFileStatusKind; label: string; title: string }> = [];
    if (status.added) parts.push({ status: 'added', label: `+${status.added}`, title: `${status.added} added` });
    if (status.modified) parts.push({ status: 'modified', label: `M${status.modified}`, title: `${status.modified} modified` });
    if (status.deleted) parts.push({ status: 'deleted', label: `-${status.deleted}`, title: `${status.deleted} deleted` });
    if (status.untracked) parts.push({ status: 'untracked', label: `?${status.untracked}`, title: `${status.untracked} untracked` });
    if (status.conflicts) parts.push({ status: 'conflict', label: `!${status.conflicts}`, title: `${status.conflicts} conflict${status.conflicts === 1 ? '' : 's'}` });
    return parts;
  }

  function stackGitStatusForEntry(
    entry: StackEntry,
    status: StackGitStatus | null,
    statusPath: string,
    folderPath: string
  ) {
    if (!status || statusPath !== folderPath) {
      return null;
    }
    let nestedStatus: StackGitFileStatusKind | null = null;
    for (const item of status.entries) {
      if (stackGitStatusPathMatchesEntry(entry.path, item.path, false)) {
        return item.status;
      }
      if (entry.entryType === 'Folder' && stackGitStatusPathMatchesEntry(entry.path, item.path, true)) {
        nestedStatus = stackGitStatusPriority(nestedStatus, item.status);
      }
    }
    return nestedStatus;
  }

  function stackGitStatusPriority(current: StackGitFileStatusKind | null, next: StackGitFileStatusKind) {
    const rank: Record<StackGitFileStatusKind, number> = {
      modified: 1,
      untracked: 2,
      added: 3,
      deleted: 4,
      conflict: 5
    };
    return !current || rank[next] > rank[current] ? next : current;
  }

  function beginMarqueeSelection(event: PointerEvent) {
    if (
      event.button !== 0
      || hasRetainedRows
      || !detailsGrid
      || !detailsBody
      || isStackMarqueeScrollbarTarget(event)
      || !isStackMarqueeStartTarget(event.target)
    ) {
      return;
    }

    closeMenus();
    event.preventDefault();
    event.stopPropagation();
    const start = { x: event.clientX, y: event.clientY };
    marqueeSelection = {
      pointerId: event.pointerId,
      start,
      current: start,
      additive: event.ctrlKey || event.metaKey,
      baseSelection: [...selectedPaths],
      folderPath: currentPath
    };
    try {
      detailsGrid.setPointerCapture(event.pointerId);
    } catch {
      marqueeSelection = null;
      return;
    }
    updateMarqueeSelection();
  }


  function isStackMarqueeScrollbarTarget(event: PointerEvent) {
    if (!detailsBody || event.target !== detailsBody) {
      return false;
    }
    if (detailsBody.scrollHeight <= detailsBody.clientHeight) {
      return false;
    }
    return event.offsetX >= detailsBody.clientWidth;
  }

  function isStackMarqueeStartTarget(target: EventTarget | null) {
    if (!(target instanceof Element)) {
      return false;
    }
    return classifyStackMarqueeStartTarget({
      self: target === detailsBody || target === stackPopupSurface,
      closest: (selector) => Boolean(target.closest(selector))
    }) !== 'blocked';
  }

  function handleMarqueePointerMove(event: PointerEvent) {
    if (!marqueeSelection || event.pointerId !== marqueeSelection.pointerId) {
      return;
    }

    event.preventDefault();
    marqueeSelection = {
      ...marqueeSelection,
      current: { x: event.clientX, y: event.clientY }
    };
    updateMarqueeSelection();
    scheduleMarqueeAutoscroll(event.clientY);
  }

  function endMarqueeSelection(event: PointerEvent) {
    if (!marqueeSelection || event.pointerId !== marqueeSelection.pointerId) {
      return;
    }

    event.preventDefault();
    try {
      if (detailsGrid?.hasPointerCapture(event.pointerId)) {
        detailsGrid.releasePointerCapture(event.pointerId);
      } else if (detailsBody?.hasPointerCapture(event.pointerId)) {
        detailsBody.releasePointerCapture(event.pointerId);
      }
    } finally {
      stopMarqueeAutoscroll();
      marqueeSelection = null;
    }
  }

  function updateMarqueeSelection() {
    if (!marqueeSelection || !detailsBody || marqueeSelection.folderPath !== currentPath) {
      marqueeSelection = null;
      stopMarqueeAutoscroll();
      return;
    }

    const rect = stackBrowserMarqueeRect(marqueeSelection.start, marqueeSelection.current);
    const selected = stackBrowserMarqueeSelectedVirtualPaths(
      entries.map((entry) => entry.path),
      rect,
      {
        rowHeight: virtualEntries.rowHeight,
        rowLeft: detailsBody.getBoundingClientRect().left,
        rowRight: detailsBody.getBoundingClientRect().right,
        viewportTop: detailsBody.getBoundingClientRect().top,
        scrollTop: detailsBody.scrollTop,
        existingSelection: marqueeSelection.additive ? marqueeSelection.baseSelection : undefined,
        additive: marqueeSelection.additive
      }
    );
    stackState = selectStackEntryPaths(stackState, selected);
  }

  function scheduleMarqueeAutoscroll(pointerY: number) {
    if (!detailsBody) {
      return;
    }

    const bounds = detailsBody.getBoundingClientRect();
    const edgeSize = 32;
    const maxStep = 18;
    let step = 0;
    if (pointerY < bounds.top + edgeSize) {
      step = -Math.ceil(maxStep * (1 - Math.max(0, pointerY - bounds.top) / edgeSize));
    } else if (pointerY > bounds.bottom - edgeSize) {
      step = Math.ceil(maxStep * (1 - Math.max(0, bounds.bottom - pointerY) / edgeSize));
    }

    if (!step) {
      stopMarqueeAutoscroll();
      return;
    }

    if (marqueeAutoscrollFrame !== null) {
      return;
    }

    marqueeAutoscrollFrame = window.requestAnimationFrame(() => {
      marqueeAutoscrollFrame = null;
      if (!marqueeSelection || !detailsBody) {
        return;
      }
      detailsBody.scrollTop += step;
      handleDetailsBodyScroll();
      updateMarqueeSelection();
      if (marqueeSelection) {
        scheduleMarqueeAutoscroll(marqueeSelection.current.y);
      }
    });
  }

  function stopMarqueeAutoscroll() {
    if (marqueeAutoscrollFrame !== null) {
      window.cancelAnimationFrame(marqueeAutoscrollFrame);
      marqueeAutoscrollFrame = null;
    }
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
    void loadOpenWithCandidates(entry);
    rowMenu = { x: event.clientX, y: event.clientY, path: entry.path };
    backgroundMenu = null;
    void positionOpenMenus();
  }

  function shouldIgnoreBackgroundContextMenu(event: MouseEvent) {
    const target = event.target instanceof HTMLElement ? event.target : null;
    return !!target?.closest(STACK_BROWSER_BACKGROUND_CONTEXT_MENU_IGNORE_SELECTORS.join(','));
  }

  function handleBackgroundContextMenu(event: MouseEvent) {
    if (gitStatusPopupOpen || shouldIgnoreBackgroundContextMenu(event)) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
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
    void prepareStackFileDrag(paths).catch((error) => {
      console.error('Failed to prepare native Stack Browser file drag', error);
    });
    event.dataTransfer?.setData(STACK_PATHS_DRAG_TYPE, JSON.stringify(paths));
    event.dataTransfer?.setData('text/plain', paths.join('\n'));
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'copy';
      event.dataTransfer.setData('text/uri-list', paths.map((path) => folderPathToUri(path)).join('\r\n'));
      event.dataTransfer.setData('DownloadURL', paths.map((path) => `application/octet-stream:${path.split(/[\\/]/).filter(Boolean).at(-1) ?? 'file'}:${folderPathToUri(path)}`).join('\n'));
      const folderPaths = paths.filter((path) => entries.some((item) => item.path === path && item.entryType === 'Folder'));
      if (folderPaths.length) {
        setFolderDragPayload(event.dataTransfer, folderPaths, 'copy');
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

  function isCtrlSpaceHotkey(event: KeyboardEvent) {
    return event.code === 'Space' && event.ctrlKey && !event.altKey && !event.metaKey;
  }

  function handleSearchHotkeyKeydown(event: KeyboardEvent) {
    if (!isCtrlSpaceHotkey(event)) return;
    event.preventDefault();
    event.stopPropagation();
    if (!shellSurfaceHotkeyHandled && !event.repeat) {
      shellSurfaceHotkeyHandled = true;
      void emitTo(TOP_BAR_TARGET, SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT);
    }
  }

  function isEditableKeyTarget(target: EventTarget | null) {
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) {
      return true;
    }
    return target instanceof HTMLElement && (target.isContentEditable || Boolean(target.closest('.inline-editor, [contenteditable="true"]')));
  }

  function handleKeydown(event: KeyboardEvent) {
    if (gitStatusPopupOpen || isEditableKeyTarget(event.target)) {
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
        void closeStackPopupFromSurface();
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
  on:pointermove={handleMarqueePointerMove}
  on:pointerup={endResize}
  on:pointerup={endMarqueeSelection}
  on:pointercancel={endResize}
  on:pointercancel={endMarqueeSelection}
  on:resize={updateDetailsViewport}
/>

<section
  bind:this={stackPopupSurface}
  class:resizing={!!resizeDrag}
  class:terminal-mode={stackBrowserViewMode === 'terminal'}
  class="stack-popup"
  aria-label="Stack browser"
  aria-busy={loadingPath ? 'true' : 'false'}
  on:contextmenu={handleBackgroundContextMenu}
  on:pointerdown={beginMarqueeSelection}
>
  <MeltActionButton
    class="stack-browser-close-button"
    ariaLabel="Close stack browser"
    onClick={() => void closeStackPopupFromSurface()}
  >×</MeltActionButton>

  <header class="stack-toolbar">
    <div class="stack-path" title={currentPath}>
      <form
        class="stack-path-editor"
        aria-label="Current folder path"
        on:submit|preventDefault={() => void submitPathDraft()}
      >
        <div class="path-input-shell">
          <input
            bind:this={pathInput}
            aria-label="Current folder path"
            value={pathDraft}
            placeholder="Stack Browser"
            spellcheck="false"
            autocomplete="off"
            on:focus={(event) => {
              cancelPathBlurReset();
              pathInputFocused = true;
              void refreshPathSuggestions(event.currentTarget);
            }}
            on:blur={() => {
              pathInputFocused = false;
              schedulePathBlurReset();
            }}
            on:input={(event) => void refreshPathSuggestions(event.currentTarget)}
            on:keydown={handlePathKeydown}
          />
          {#if pathInlineCompletion}
            <span class="path-inline-ghost">
              <span class="path-inline-typed" aria-hidden="true">{pathDraft}</span><button
                type="button"
                class="path-inline-completion"
                aria-label={`Accept path autocomplete ${pathInlineCompletion.displayText}`}
                tabindex="-1"
                on:mousedown|preventDefault={acceptInlinePathCompletion}
              >{pathInlineCompletion.displayText}</button>
            </span>
          {/if}
        </div>
        {#if currentPath}
          <nav class="path-segments" aria-label="Path segments">
            {#each breadcrumbOverflow.visibleSegments as crumb, i (crumb.path)}
              <MeltActionButton class="path-segment" ariaCurrent={crumb.path === currentPath ? 'page' : undefined} title={crumb.path} onClick={() => void openFolder(crumb.path)}>{crumb.name}</MeltActionButton>
              {#if i === 0 && breadcrumbOverflow.hiddenCount}
                <span class="crumb-sep">/</span>
                <span class="crumb-overflow" title={breadcrumbOverflow.hiddenTitle} aria-label={`${breadcrumbOverflow.hiddenCount} collapsed path segments`}>...</span>
              {/if}
              {#if i < breadcrumbOverflow.visibleSegments.length - 1}
                <span class="crumb-sep">/</span>
              {/if}
            {/each}
          </nav>
        {/if}
        {#if gitStatus && gitStatusPath === currentPath}
          <div class="stack-git-summary" aria-label={`Git ${gitStatus.branch} ${stackGitSummaryParts(gitStatus).map((part) => part.title).join(', ') || 'clean'}`}>
            <button type="button" class="stack-git-branch" on:click={() => openGitStatusPopup('all')}>{gitStatus.branch}</button>
            {#if stackGitSummaryParts(gitStatus).length}
              {#each stackGitSummaryParts(gitStatus) as part}
                <button type="button" class={`stack-git-count git-status-${part.status}`} title={part.title} on:click={() => openGitStatusPopup(part.status)}>{part.label}</button>
              {/each}
            {:else}
              <button type="button" class="stack-git-clean" on:click={() => openGitStatusPopup('all')}>clean</button>
            {/if}
            {#if gitStatus.remoteRepositoryUrl}
              <button
                type="button"
                class="stack-git-remote-link"
                title={`Open remote repository ${gitStatus.remoteRepositoryUrl}`}
                aria-label="Open remote repository in browser"
                on:click={() => void openGitRemoteRepository(gitStatus?.remoteRepositoryUrl)}
              >↗</button>
            {/if}
          </div>
        {/if}
      </form>
    </div>
    <div class="stack-actions">
      <MeltActionButton disabled={!canGoBack} onClick={() => void navigateHistory(-1)}>Back</MeltActionButton>
      <MeltActionButton disabled={!canGoForward} onClick={() => void navigateHistory(1)}>Forward</MeltActionButton>
      <MeltActionButton onClick={() => void loadFolder(currentPath)}>Refresh</MeltActionButton>
      <MeltActionButton disabled={!hasSelection} onClick={() => void copySelected(false)}>Copy</MeltActionButton>
      <MeltActionButton disabled={!hasSelection} onClick={() => void copySelected(true)}>Cut</MeltActionButton>
      <MeltActionButton disabled={!currentPath} onClick={() => void pasteIntoCurrentFolder()}>Paste</MeltActionButton>
      <MeltActionButton disabled={!selectedEntry} onClick={beginRenameSelected}>Rename</MeltActionButton>
      <MeltActionButton disabled={!hasSelection} onClick={() => void deleteSelected()}>Delete</MeltActionButton>
      <MeltActionButton disabled={!currentPath} onClick={beginCreateFolder}>New Folder</MeltActionButton>
      <MeltActionButton disabled={!selectedEntry} onClick={() => void revealSelected()}>Reveal</MeltActionButton>
      <label class="stack-search" aria-label="Search current folder">
        <span>Search</span>
        <input
          aria-label="Search current folder"
          value={searchQuery}
          placeholder="Search folder"
          spellcheck="false"
          autocomplete="off"
          on:input={handleStackSearchInput}
          on:keydown={(event) => event.stopPropagation()}
        />
      </label>
    </div>
  </header>

  <div class="stack-status surface-state" class:error={!!errorMessage} class:info={!errorMessage} role="status" aria-live="polite">
    <span>{errorMessage || stackState.statusMessage}</span>
    {#if loadingPath}
      <span>Loading...</span>
    {:else if iconHydrationStatusMessage}
      <span>{iconHydrationStatusMessage}</span>
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
      <MeltActionButton type="submit">OK</MeltActionButton>
      <MeltActionButton onClick={cancelInlineEditor}>Cancel</MeltActionButton>
    </form>
  {/if}
  {#if gitStatusPopupOpen}
    <StackGitPanel
      folderPath={currentPath}
      initialStatus={gitStatus}
      initialChangeFilter={gitStatusPopupFilter}
      onClose={closeGitStatusPopup}
      onRefresh={() => void refreshStackGitStatus(currentPath, folderLoadSequence)}
    />
  {:else}
    <div
      class="details-table"
      class:marquee-selecting={!!marqueeSelection}
      role="grid"
      aria-label="Folder details"
      aria-busy={loadingPath ? 'true' : 'false'}
      aria-rowcount={visibleEntries.length + 1}
      aria-colcount="4"
      tabindex="0"
      bind:this={detailsGrid}
      on:contextmenu={handleBackgroundContextMenu}
      on:dragover={(event) => handleDropOver(event)}
      on:drop={(event) => void handleDrop(event, currentPath)}
    >
    <div class="details-header" role="row" aria-rowindex="1">
      <MeltActionButton class={sortHeader('name').className} role="columnheader" ariaColindex={1} ariaSort={sortHeader('name').ariaSort} onClick={() => sortBy('name')}><span>Name</span><span class="sort-indicator" aria-hidden="true">{sortHeader('name').indicator}</span></MeltActionButton>
      <MeltActionButton class={sortHeader('type').className} role="columnheader" ariaColindex={2} ariaSort={sortHeader('type').ariaSort} onClick={() => sortBy('type')}><span>Type</span><span class="sort-indicator" aria-hidden="true">{sortHeader('type').indicator}</span></MeltActionButton>
      <MeltActionButton class={sortHeader('size').className} role="columnheader" ariaColindex={3} ariaSort={sortHeader('size').ariaSort} onClick={() => sortBy('size')}><span>Size</span><span class="sort-indicator" aria-hidden="true">{sortHeader('size').indicator}</span></MeltActionButton>
      <MeltActionButton class={sortHeader('modified').className} role="columnheader" ariaColindex={4} ariaSort={sortHeader('modified').ariaSort} onClick={() => sortBy('modified')}><span>Modified</span><span class="sort-indicator" aria-hidden="true">{sortHeader('modified').indicator}</span></MeltActionButton>
    </div>

    {#if visibleEntries.length}
      <div
        class="details-body"
        class:marquee-selecting={!!marqueeSelection}
        role="rowgroup"
        bind:this={detailsBody}
        data-stack-marquee-start="body"
        on:scroll={handleDetailsBodyScroll}
      >
        {#if virtualEntries.beforeHeight}
          <div class="virtual-spacer" data-stack-marquee-start="spacer" style={`height:${virtualEntries.beforeHeight}px`} aria-hidden="true"></div>
        {/if}
        {#each virtualEntries.rows as virtualRow (virtualRow.item.id)}
          {@const entry = virtualRow.item}
          {@const fileIcon = stackFileIconForEntry(entry)}
          {@const gitEntryStatus = stackGitStatusForEntry(entry, gitStatus, gitStatusPath, currentPath)}
          <button
            class:selected={stackState.selectedPaths.includes(entry.path)}
            class:subdued={entry.isHidden || entry.isSystem}
            class:readonly={entry.isReadonly}
            class:linked={entry.isSymlink || entry.isReparsePoint}
            class:retained={hasRetainedRows}
            class:git-added={gitEntryStatus === 'added'}
            class:git-modified={gitEntryStatus === 'modified'}
            class:git-deleted={gitEntryStatus === 'deleted'}
            class:git-untracked={gitEntryStatus === 'untracked'}
            class:git-conflicted={gitEntryStatus === 'conflict'}
            data-git-status={gitEntryStatus ?? undefined}
            type="button"
            role="row"
            aria-rowindex={virtualRow.index + 2}
            aria-selected={stackState.selectedPaths.includes(entry.path)}
            aria-disabled={hasRetainedRows}
            disabled={hasRetainedRows}
            draggable={!hasRetainedRows}
            data-stack-entry-path={entry.path}
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
              {#if gitEntryStatus}
                <span class={`git-file-badge git-status-badge git-status-${gitEntryStatus}`} aria-label={stackGitStatusLabel(gitEntryStatus)} title={stackGitStatusLabel(gitEntryStatus)}>{stackGitStatusSymbol(gitEntryStatus)}</span>
              {/if}
            </span>
            <span role="gridcell" aria-colindex="2">{entry.typeLabel}</span>
            <span role="gridcell" aria-colindex="3">{formatStackSize(entry.size)}</span>
            <span role="gridcell" aria-colindex="4">{formatModified(entry.modifiedMs)}</span>
          </button>
        {/each}
        {#if virtualEntries.afterHeight}
          <div class="virtual-spacer" data-stack-marquee-start="spacer" style={`height:${virtualEntries.afterHeight}px`} aria-hidden="true"></div>
        {/if}
        {#if marqueeRect}
          <div
            class="stack-marquee-rect"
            style={`left:${marqueeRect.left}px;top:${marqueeRect.top}px;width:${marqueeRect.width}px;height:${marqueeRect.height}px`}
            aria-hidden="true"
          ></div>
        {/if}
      </div>
    {:else}
      <div class="empty-stack surface-state" class:loading={!!loadingPath} class:info={!loadingPath} role="status">{loadingPath ? 'Loading folder...' : stackState.statusMessage}</div>
    {/if}
    </div>
  {/if}
  {#if rowMenu}
    <div
      class="context-menu"
      style={`left:${rowMenu.x}px;top:${rowMenu.y}px;--stack-context-menu-max-height:${contextMenuMaxHeightCss(rowMenu)};--stack-context-menu-left:${rowMenu.x}px;--stack-context-menu-top:${rowMenu.y}px;--stack-context-menu-width:${contextMenuWidthCss(rowMenu)};--stack-context-submenu-max-height:${contextSubmenuMaxHeightCss(rowMenu)}`}
      role="menu"
      tabindex="-1"
      bind:this={rowMenuElement}
      on:click|stopPropagation
      on:contextmenu|stopPropagation
      on:keydown={(event) => event.key === 'Escape' && closeMenus()}
      on:scroll={() => void positionOpenMenus()}
    >
      <MeltActionButton role="menuitem" disabled={!selectedEntry} onClick={() => selectedEntry && void activateEntry(selectedEntry)}>Open</MeltActionButton>
      <div bind:this={rowSubmenuElement} class:left={rowSubmenuOpensLeft} class="context-submenu" role="none">
        <MeltActionButton class="submenu-trigger" role="menuitem" ariaHaspopup="menu" disabled={selectedEntry?.entryType !== 'File'}>Open with ▸</MeltActionButton>
        <div class="context-menu context-submenu-panel" role="menu">
          {#each openWithSuggestions as app (app.id)}
            <MeltActionButton role="menuitem" disabled={selectedEntry?.entryType !== 'File'} onClick={() => void openSelectedWithSuggestedApp(app)}>{app.label}</MeltActionButton>
          {/each}
          <MeltActionButton role="menuitem" disabled={selectedEntry?.entryType !== 'File'} onClick={() => void openSelectedWithPicker()}>Choose app...</MeltActionButton>
        </div>
      </div>
      <MeltActionButton role="menuitem" disabled={!hasSelection} onClick={() => void copySelected(false)}>Copy</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!hasSelection} onClick={() => void copySelected(true)}>Cut</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={selectedEntry?.entryType !== 'Folder'} onClick={() => void pinSelectedFolderToTopBar()}>Pin to Top Bar</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={selectedEntry?.entryType !== 'Folder'} onClick={() => void openSelectedFolderInVscode()}>Open in VS Code</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedZipArchiveEntry()} onClick={() => void extractSelectedArchive('here')}>Extract here</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedZipArchiveEntry()} onClick={() => void extractSelectedArchive('folder')}>Extract to folder</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedSevenZipArchiveEntry()} onClick={() => void extractSelectedArchive('here', 'sevenZip')}>Extract here with 7-Zip</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedSevenZipArchiveEntry()} onClick={() => void extractSelectedArchive('folder', 'sevenZip')}>Extract to folder with 7-Zip</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedEntry} onClick={() => void copyTextToClipboard(selectedEntry?.path ?? '', 'Copy path unavailable')}>Copy Path</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedEntry} onClick={() => void copyTextToClipboard(selectedEntry?.name ?? '', 'Copy name unavailable')}>Copy Name</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedEntry} onClick={() => void copyTextToClipboard(selectedDirectoryPath(), 'Copy containing folder unavailable')}>Copy Containing Folder</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedEntry} onClick={beginRenameSelected}>Rename</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!hasSelection} onClick={() => void deleteSelected()}>Delete</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedEntry} onClick={() => void revealSelected()}>Reveal</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedEntry} onClick={() => void showSelectedProperties()}>Properties</MeltActionButton>
    </div>
  {/if}

  {#if backgroundMenu}
    <div
      class="context-menu"
      style={`left:${backgroundMenu.x}px;top:${backgroundMenu.y}px;--stack-context-menu-max-height:${contextMenuMaxHeightCss(backgroundMenu)}`}
      role="menu"
      tabindex="-1"
      bind:this={backgroundMenuElement}
      on:click|stopPropagation
      on:contextmenu|stopPropagation
      on:keydown={(event) => event.key === 'Escape' && closeMenus()}
    >
      <MeltActionButton role="menuitem" disabled={!hasSelection} onClick={() => void copySelected(false)}>Copy</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!hasSelection} onClick={() => void copySelected(true)}>Cut</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedEntry} onClick={beginRenameSelected}>Rename</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!hasSelection} onClick={() => void deleteSelected()}>Delete</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!selectedEntry} onClick={() => void revealSelected()}>Reveal</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!currentPath} onClick={() => void pasteIntoCurrentFolder()}>Paste</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!currentPath} onClick={beginCreateFolder}>New Folder</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!currentPath} onClick={() => void beginCreateTextFile()}>New Text File</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!currentPath} onClick={() => void copyTextToClipboard(currentPath, 'Copy folder path unavailable')}>Copy Folder Path</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!currentPath} onClick={() => void openCurrentFolderInVscode()}>Open in VS Code</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!currentPath} onClick={() => void openTerminalHere()}>Open Terminal Here</MeltActionButton>
      <MeltActionButton role="menuitem" disabled={!currentPath} onClick={() => void showCurrentFolderProperties()}>Properties</MeltActionButton>
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
          <MeltActionButton class="danger" onClick={() => void confirmDeleteSelection()}>Delete</MeltActionButton>
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
