<script lang="ts">
  import './StackPopupSurface.css';
  import { emit, emitTo, listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { DragDropEvent } from '@tauri-apps/api/webview';
  import { onMount, tick } from 'svelte';
  import { StackIconCache } from '../lib/stackIconCache';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import ContextMenuItem from './ContextMenuItem.svelte';
  import ContextMenuSeparator from './ContextMenuSeparator.svelte';
  // FE-34: the editor (CodeMirror) and Git panes load with import() on first use. Type-only imports are erased from the bundle.
  import type StackGitPanelComponent from './StackGitPanel.svelte';
  import type StackTextEditorComponent from './StackTextEditor.svelte';
  import StackConfirmDialog from './StackConfirmDialog.svelte';
  import {
    beginStackPopupFocusLossHold,
    copyStackItems,
    deleteStackItems,
    endStackPopupFocusLossHold,
    emitStackFolderListingDiagnostics,
    extractStackArchive,
    getStackGitContext,
    getStackGitSummary,
    getStackGitMetadata,
    startStackFolderWatch,
    stopStackFolderWatch,
    STACK_FOLDER_CHANGED_EVENT,
    STACK_FOLDER_WATCH_ERROR_EVENT,
    type StackFolderWatchChange,
    type StackFolderWatchError,
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
    pasteFailureSummary,
    startStackFileDrag,
    prepareStackLocalDrop,
    commitStackLocalDrop,
    undoStackMove,
    renameStackItem,
    resolveStackItemIcons,
    resizeStackPopup,
    revealStackItem,
    showStackItemProperties,
    suggestStackPaths,
    STACK_POPUP_OPEN_EVENT,
    STACK_POPUP_CLOSE_REQUESTED_EVENT,
    type StackEntry,
    type StackArchiveDestinationMode,
    type StackArchiveExtractor,
    type StackGitStatus,
    type StackGitFileStatusKind,
    type StackItemIconResolution,
    type StackFolderListing,
    type StackOpenWithCandidate,
    type StackPathSuggestion
  } from '../lib/stackPopup';
  import { folderPathsFromTransfer, normalizeDroppedPath } from '../lib/folderDrag';
  import { beginStackRowDrag, moveStackRowDrag, type StackRowDragIntent } from '../features/stack-browser/nativeDragIntent';
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
    stackSortHeaderState,
    updateStackSort,
    type StackEntryIconUpdate,
    type StackPopupOpenPayload,
    type StackSortColumn
  } from '../lib/stackPopupState';
  import { stackFileIconForEntry } from '../lib/stackFileIcons';
  import { buildStackGitStatusIndex, formatStackModified, stackAttributeLabels, stackGitStatusFromIndex } from '../lib/stackRowDisplay';
  import { positionScrollableContextMenuInViewport } from '../lib/contextMenuPosition';
  import { IPC_EVENTS, type StackFileOperationProgress } from '../ipc/events';

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
  import {
    beginBasicTextEditorExit,
    cancelPendingEditorExit,
    takePendingEditorExit,
    type PendingBasicTextEditorExit
  } from '../features/stack-browser/basicTextEditorExit';

  const STACK_PATHS_DRAG_TYPE = 'application/x-jasonshell-stack-paths';
  const STACK_DROP_DELIVERY_GRACE_MS = 500;
  const STACK_POPUP_MIN_WIDTH = 560;
  const STACK_POPUP_MIN_HEIGHT = 280;
  const STACK_ICON_RESOLVE_BATCH_SIZE = 24;
  const STACK_ICON_RESOLVE_MAX_CONCURRENCY = 2;
  const STACK_CONTEXT_MENU_VIEWPORT_PADDING = 8;
  const STACK_BASIC_TEXT_EXTENSIONS = new Set([
    'txt', 'md', 'json', 'js', 'ts', 'svelte', 'css', 'html', 'xml', 'yml', 'yaml', 'csv', 'log'
  ]);
  type StackContextMenuPlacement = {
    x: number;
    y: number;
    maxHeight?: number;
    width?: number;
    submenuMaxHeight?: number;
    submenuTop?: number;
  };

  type LocalDragSession = {
    paths: string[];
    folderPath: string;
    loadSequence: number;
    visibilityEpoch: number;
    grid: HTMLDivElement;
    prepared: Promise<{ requestId: string } | { error: unknown }>;
  };
  let stackState = defaultStackPopupViewState;
  let loadingPath: string | null = null;
  let errorMessage = '';
  let rowMenu: (StackContextMenuPlacement & { path: string }) | null = null;
  let backgroundMenu: StackContextMenuPlacement | null = null;
  let rowMenuElement: HTMLDivElement | null = null;
  let backgroundMenuElement: HTMLDivElement | null = null;
  let rowSubmenuElement: HTMLDivElement | null = null;
  let rowSubmenuPanelElement: HTMLDivElement | null = null;
  let openWithTrigger: HTMLButtonElement | null = null;
  let openWithOpen = false;
  let openWithCloseTimer: number | null = null;
  let openWithRevision = 0;
  let openWithPendingCandidates: StackOpenWithCandidate[] | null = null;
  let openWithStatus: 'loading' | 'ready' | 'error' = 'ready';
  let openWithLaunching = false;
  let openWithPosition = { x: 8, y: 8 };
  let deleteConfirmation: {
    title: string; message: string; paths: string[]; folderPath: string; loadSequence: number; visibilityEpoch: number;
  } | null = null;
  let rowSubmenuOpensLeft = false;
  let createFolderDraft: string | null = null;
  let renameDraft: string | null = null;
  let editorInput: HTMLInputElement | null = null;
  let typeToSelectBuffer = '';
  let typeToSelectTimer: number | null = null;
  let lastHtmlDrop: { at: number; key: string } | null = null;
  let rowDragIntent: StackRowDragIntent = null;
  let rowDragPath: string | null = null;
  let rowDragCapture: HTMLElement | null = null;
  let localDrag: LocalDragSession | null = null;
  let pendingLocalDrop: LocalDragSession | null = null;
  let outboundDragging = false;
  let localDropTargetPath: string | null = null;
  let suppressRowClick = false;
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
  let stackSearchInput: HTMLInputElement | null = null;
  let openWithCandidates: StackOpenWithCandidate[] = [];
  const iconCache = new StackIconCache();
  let disposed = false;
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
  type GitContext = NonNullable<Awaited<ReturnType<typeof getStackGitContext>>>;
  let gitRepositoryContext: GitContext | null = null;
  let gitRepositoryGeneration = 0;
  let gitRefreshRevision = 0;
  let gitContextSequence = 0;
  let gitVerifiedFolder = '';
  let gitSummaryPending: { generation: number; revision: number; promise: Promise<void> } | null = null;
  let gitSummaryRequested = false;
  let gitDisposed = false;
  let gitErrorMessage = '';
  let gitStatusPopupOpen = false;
  let gitPanelRefresh: () => void = () => {};
  let editorSaved: () => void = () => {};
  let gitStatusPopupFilter: StackGitFileStatusKind | 'all' = 'all';
  let stackPopupSurface: HTMLElement | null = null;
  let editorPath: string | null = null;
  let editorDirty = false;
  let pendingEditorExit: PendingBasicTextEditorExit | null = null;
  let editorExitFocusOrigin: HTMLElement | null = null;
  let activeOperation: StackFileOperationProgress | null = null;
  let undoReceipt: { id: string; kind: 'move' | 'delete' } | null = null;
  $: undoLabel = undoReceipt?.kind === 'delete' ? 'Undo delete' : 'Undo move';
  let fileTransferPending = false;
  let pasteBusyMessage = '';
  $: fileOperationBusy = !!activeOperation && activeOperation.phase !== 'completed' && activeOperation.phase !== 'failed';
  $: canUndoMove = !!undoReceipt && !fileTransferPending && !fileOperationBusy && folderWatchVisible &&
    !!currentPath && !editorPath && !gitStatusPopupOpen &&
    !rowMenu && !backgroundMenu && !deleteConfirmation && !pendingEditorExit &&
    createFolderDraft === null && renameDraft === null && !rowDragIntent && !pendingLocalDrop && !outboundDragging;
  let operationClock = Date.now();
  let operationClockTimer: number | null = null;
  let folderWatchVisible = true;
  let folderWatchEnabled = true;
  let popupVisibilityEpoch = 0;
  let folderWatchEpoch = 0;
  let desiredFolderWatch: { key: string; path: string; epoch: number } | null = null;
  let activeFolderWatch: { key: string; watchId: string; epoch: number } | null = null;
  let folderWatchChain: Promise<void> = Promise.resolve();
  let folderWatchErrorMessage = '';
  let folderWatchTimer: number | null = null;
  let externalRefreshDirty = false;
  let externalGitDirty = false;
  let externalRefreshPending = false;
  let folderWatchDeliveryReady = false;
  let folderWatchStartingEpoch: number | null = null;
  let pendingFolderWatchErrors: StackFolderWatchError[] = [];
  let folderReadPending: { sequence: number; promise: Promise<StackFolderListing> } | null = null;
  $: currentPath = stackState.currentPath;
  $: entries = stackState.entries;
  $: visibleEntries = stackBrowserSearchEntries(entries, searchQuery);
  $: selectedEntry = selectedStackEntry(stackState);
  $: selectedPaths = selectedStackPaths(stackState);
  $: selectedPathSet = new Set(stackState.selectedPaths);
  $: gitStatusIndex = buildStackGitStatusIndex(gitStatus, gitStatusPath, currentPath, gitRepositoryContext?.canonicalPath);
  $: hasSelection = selectedPaths.length > 0;
  $: canGoBack = canNavigateStackBack(stackState);
  $: canGoForward = canNavigateStackForward(stackState);
  $: breadcrumbs = stackBreadcrumbSegments(currentPath);
  $: breadcrumbOverflow = stackBrowserBreadcrumbOverflow(breadcrumbs, 5);
  $: hasRetainedRows = stackPopupHasRetainedRows(stackState);
  $: virtualEntries = stackBrowserVirtualWindow(visibleEntries, detailsBodyScrollTop, detailsBodyHeight);
  $: marqueeRect = marqueeSelection ? stackBrowserMarqueeRect(marqueeSelection.start, marqueeSelection.current) : null;
  $: if (loadingPath || !folderWatchVisible || editorPath || gitStatusPopupOpen ||
      rowMenu || backgroundMenu || deleteConfirmation || pendingEditorExit || createFolderDraft !== null || renameDraft !== null) {
    cancelRowDrag();
  }
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
    void initializeFolderWatchDelivery(unlisteners, () => disposed).then(() => {
      if (!disposed) void initializeOpenRequestDelivery(unlisteners, () => disposed);
    });
    void getCurrentWindow().listen(STACK_POPUP_CLOSE_REQUESTED_EVENT, () => {
      closeStackPopupFromSurface();
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    }).catch((error) => console.error('Failed to listen for Stack popup close requests', error));
    void getCurrentWindow().listen<StackFileOperationProgress>(IPC_EVENTS.stackFileOperationProgress, ({ payload }) => {
      if (!activeOperation || payload.operationId !== activeOperation.operationId) return;
      activeOperation = { ...activeOperation, ...payload };
      if (payload.phase === 'completed') {
        refreshFileOperation(payload.operationId);
      }
      if (payload.phase === 'failed') {
        errorMessage = payload.error || payload.message || `${operationLabel(payload.operation)} failed`;
        // Individual Recycle Bin operations may fail while the selected batch continues.
        if (payload.operation !== 'delete') {
          stopOperationClock();
        }
      }
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    });
    void getCurrentWindow().onDragDropEvent((event: { payload: DragDropEvent }) => {
      if (event.payload.type === 'drop') handleNativeDrop(event.payload);
    }).then((unlisten: () => void) => {
      if (disposed) {
        unlisten();
      } else {
        unlisteners.push(unlisten);
      }
    });

    return () => {
      disposed = true;
      cancelRowDrag();
      gitDisposed = true;
      gitRepositoryGeneration += 1;
      gitContextSequence += 1;
      suspendFolderWatch();
      closeMenus();
      folderLoadSequence += 1;
      iconHydrationJobToken += 1;
      iconHydrationVisiblePriority = [];
      iconHydrationPending = [];
      iconCache.clear();
      if (typeToSelectTimer !== null) {
        window.clearTimeout(typeToSelectTimer);
      }
      cancelPathBlurReset();
      if (resizeFrame !== null) {
        window.cancelAnimationFrame(resizeFrame);
      }
      stopMarqueeAutoscroll();
      stopOperationClock();
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });

  async function initializeFolderWatchDelivery(unlisteners: Array<() => void>, isDisposed: () => boolean) {
    const popup = getCurrentWindow();
    const registrations = [
      popup.listen<StackFolderWatchChange>(STACK_FOLDER_CHANGED_EVENT, ({ payload }) => {
        if (payload.watchId === activeFolderWatch?.watchId) queueExternalRefresh(true);
      }),
      popup.listen<StackFolderWatchError>(STACK_FOLDER_WATCH_ERROR_EVENT, ({ payload }) => {
        if (payload.watchId === activeFolderWatch?.watchId) failFolderWatch(payload);
        else if (folderWatchStartingEpoch !== null && folderWatchStartingEpoch === desiredFolderWatch?.epoch) {
          // A native worker can fail after registration but before its ACK
          // reaches this renderer. Admit only the ACK's exact token below.
          pendingFolderWatchErrors = [...pendingFolderWatchErrors, payload].slice(-4);
        }
      }),
      popup.listen('stack-popup:closed', () => suspendFolderWatch()),
      popup.listen('tauri://destroyed', () => suspendFolderWatch())
    ];
    // Focus is only a visibility observation, never an eligibility requirement.
    if (typeof popup.onFocusChanged === 'function') {
      registrations.push(popup.onFocusChanged((event: { payload: boolean }) => {
        void observeFolderWatchVisibility();
        // Recovery for an open event missed while hidden/unfocused; losing focus never pulls.
        if (event?.payload === true) void reconcileLatestStackPopupRequest();
      }));
    }
    const listeners = await Promise.allSettled(registrations);
    folderWatchDeliveryReady = listeners.every((registration) => registration.status === 'fulfilled');
    for (const registration of listeners) {
      if (registration.status === 'rejected') {
        if (!isDisposed()) folderWatchErrorMessage = 'Automatic folder updates unavailable. Use Refresh.';
        continue;
      }
      if (isDisposed()) registration.value();
      else unlisteners.push(registration.value);
    }
    if (isDisposed()) return;
    const visibilityChanged = () => {
      void observeFolderWatchVisibility();
      if (!document.hidden) void reconcileLatestStackPopupRequest();
    };
    document.addEventListener('visibilitychange', visibilityChanged);
    unlisteners.push(() => document.removeEventListener('visibilitychange', visibilityChanged));
  }

  async function popupIsVisible() {
    if (gitDisposed || !folderWatchVisible) return false;
    try {
      const visible = await getCurrentWindow().isVisible();
      return visible && folderWatchVisible && !gitDisposed;
    } catch {
      return false;
    }
  }

  function failFolderWatch(error: StackFolderWatchError) {
    folderWatchErrorMessage = `Automatic folder updates stopped: ${error.message}. Use Refresh to retry.`;
    suspendFolderWatch(false);
  }

  async function observeFolderWatchVisibility() {
    if (gitDisposed) return;
    try {
      if (!await getCurrentWindow().isVisible()) suspendFolderWatch();
    } catch {
      // Failure to establish visibility never admits background work.
      suspendFolderWatch();
    }
  }

  function suspendFolderWatch(hiding = true) {
    cancelRowDrag();
    if (hiding) {
      deleteConfirmation = null;
      if (folderWatchVisible) {
        gitRefreshRevision += 1;
        gitSummaryRequested = false;
      }
      folderWatchVisible = false;
      popupVisibilityEpoch += 1;
    } else {
      folderWatchEnabled = false;
    }
    folderWatchEpoch += 1;
    const epoch = folderWatchEpoch;
    desiredFolderWatch = null;
    externalRefreshDirty = false;
    externalGitDirty = false;
    if (folderWatchTimer !== null) window.clearTimeout(folderWatchTimer);
    folderWatchTimer = null;
    const retired = activeFolderWatch;
    activeFolderWatch = null;
    folderWatchChain = folderWatchChain.then(async () => {
      if (retired) await stopStackFolderWatch(retired.watchId);
    }).catch((error) => {
      if (!gitDisposed && epoch === folderWatchEpoch) folderWatchErrorMessage = `Automatic folder updates unavailable: ${operationErrorMessage(error, 'Watch cleanup failed')}. Use Refresh.`;
    });
  }

  function reconcileFolderWatch(folderPath: string, context: GitContext | null, catchup = true) {
    if (!folderWatchVisible || !folderWatchEnabled || gitDisposed || !folderWatchDeliveryReady) return Promise.resolve();
    const key = context ? `${context.repositoryRoot}\0${context.gitDir}` : `folder:${folderPath}`;
    if (desiredFolderWatch?.key === key) return folderWatchChain;
    const desired = { key, path: context?.repositoryRoot ?? folderPath, epoch: ++folderWatchEpoch };
    desiredFolderWatch = desired;
    const retired = activeFolderWatch;
    activeFolderWatch = null;
    folderWatchChain = folderWatchChain.then(async () => {
      if (retired) await stopStackFolderWatch(retired.watchId);
      if (!await popupIsVisible() || desiredFolderWatch !== desired) return;
      folderWatchStartingEpoch = desired.epoch;
      pendingFolderWatchErrors = [];
      let watch: Awaited<ReturnType<typeof startStackFolderWatch>>;
      let earlyError: StackFolderWatchError | undefined;
      try {
        watch = await startStackFolderWatch(desired.path);
        earlyError = pendingFolderWatchErrors.find((error) => error.watchId === watch.watchId);
      } finally {
        folderWatchStartingEpoch = null;
        pendingFolderWatchErrors = [];
      }
      if (!await popupIsVisible() || gitDisposed || desiredFolderWatch !== desired) {
        await stopStackFolderWatch(watch.watchId);
        return;
      }
      activeFolderWatch = { key, watchId: watch.watchId, epoch: desired.epoch };
      if (earlyError) {
        failFolderWatch(earlyError);
        return;
      }
      folderWatchErrorMessage = '';
      // Registration can follow foreground reads. Catch up current rows and
      // any existing summary to close that event-delivery gap.
      if (catchup) queueExternalRefresh(gitSummaryRequested);
    }).catch(async (error) => {
      if (!gitDisposed && desiredFolderWatch === desired) {
        await joinPendingGitSummary();
        if (gitDisposed || desiredFolderWatch !== desired) return;
        folderWatchErrorMessage = `Automatic folder updates unavailable: ${operationErrorMessage(error, 'Watch start failed')}. Use Refresh to retry.`;
      }
    });
    return folderWatchChain;
  }

  function queueExternalRefresh(refreshGit: boolean) {
    if (!folderWatchVisible || !activeFolderWatch || gitDisposed) return;
    externalRefreshDirty = true;
    externalGitDirty ||= refreshGit;
    if (externalRefreshPending) return;
    if (folderWatchTimer !== null) window.clearTimeout(folderWatchTimer);
    folderWatchTimer = window.setTimeout(() => {
      folderWatchTimer = null;
      void drainExternalRefresh();
    }, 200);
  }

  async function drainExternalRefresh() {
    if (externalRefreshPending) return;
    externalRefreshPending = true;
    try {
      while (externalRefreshDirty && activeFolderWatch && await popupIsVisible()) {
        if (!activeFolderWatch) break;
        const watch = activeFolderWatch;
        const epoch = folderWatchEpoch;
        const path = stackState.currentPath;
        const loadSequence = folderLoadSequence;
        const refreshGit = externalGitDirty;
        externalRefreshDirty = false;
        externalGitDirty = false;
        try {
          await folderReadPending?.promise;
        } catch {
          // Catchup still retries rows after a failed foreground read.
        }
        await joinPendingGitSummary();
        if (!await popupIsVisible() || gitDisposed || epoch !== folderWatchEpoch || watch !== activeFolderWatch) continue;
        if (path !== stackState.currentPath || loadSequence !== folderLoadSequence) {
          externalRefreshDirty = true;
          externalGitDirty ||= refreshGit;
          continue;
        }
        const contextReady = await revalidateExternalGitContext(path, loadSequence, epoch);
        if (!contextReady || !await popupIsVisible() || gitDisposed || path !== stackState.currentPath || loadSequence !== folderLoadSequence) {
          // Navigation can supersede the context read after this flight has
          // consumed its dirty flags. Keep that work only under the exact same
          // subscription; the next flight must read the newly current folder.
          if (!gitDisposed && folderWatchVisible && watch === activeFolderWatch && epoch === folderWatchEpoch
            && (path !== stackState.currentPath || loadSequence !== folderLoadSequence)) {
            externalRefreshDirty = true;
            externalGitDirty ||= refreshGit;
          }
          continue;
        }
        const currentEpoch = folderWatchEpoch;
        const rows = refreshExternalFolderRows(path, loadSequence, currentEpoch);
        if (refreshGit && contextReady) invalidateGitSummary();
        await Promise.all([rows, joinPendingGitSummary()]);
      }
    } finally {
      externalRefreshPending = false;
    }
  }

  async function joinPendingGitSummary() {
    while (gitSummaryPending) await gitSummaryPending.promise;
  }

  async function revalidateExternalGitContext(path: string, loadSequence: number, epoch: number) {
    const sequence = ++gitContextSequence;
    try {
      const context = await getStackGitContext(path);
      if (!await popupIsVisible() || gitDisposed || sequence !== gitContextSequence || epoch !== folderWatchEpoch || path !== stackState.currentPath || loadSequence !== folderLoadSequence) return false;
      if (!sameGitRepository(gitRepositoryContext, context) && (gitRepositoryContext || context)) retireGitRepository(context);
      gitRepositoryContext = context;
      gitVerifiedFolder = context ? path : '';
      gitStatusPath = gitVerifiedFolder;
      // This flight reads rows/status after registration itself, so a replaced
      // watch does not enqueue a redundant registration-gap catchup.
      await reconcileFolderWatch(path, context, false);
      return sequence === gitContextSequence;
    } catch (error) {
      if (!await popupIsVisible() || gitDisposed || sequence !== gitContextSequence || epoch !== folderWatchEpoch || path !== stackState.currentPath || loadSequence !== folderLoadSequence) return false;
      retireGitRepository(null);
      gitErrorMessage = `Git context unavailable: ${operationErrorMessage(error, 'Try Refresh')}. Retry with Refresh.`;
      // Git discovery is optional. Native registration independently verifies
      // the current folder and can still deliver plain filesystem changes.
      await reconcileFolderWatch(path, null, false);
      return sequence === gitContextSequence;
    }
  }

  async function refreshExternalFolderRows(path: string, loadSequence: number, epoch: number, afterMutation = false) {
    const isCurrent = () => !gitDisposed && folderWatchVisible &&
      epoch === (afterMutation ? popupVisibilityEpoch : folderWatchEpoch) &&
      path === stackState.currentPath && loadSequence === folderLoadSequence;
    try {
      const listing = await listStackFolder(path);
      if (!await popupIsVisible() || !isCurrent()) return;
      // Commit a complete listing in-place: do not navigate, refocus, reset
      // sorting/search/scroll, or remount an open editor or inline input.
      stackState = applyStackFolderListing(stackState, path, listing);
      scheduleVisibleIconHydration(path, loadSequence);
    } catch (error) {
      if (isCurrent()) {
        folderWatchErrorMessage = `${afterMutation ? 'Folder refresh failed' : 'Automatic folder refresh failed'}: ${operationErrorMessage(error, 'Folder unavailable')}. Use Refresh.`;
      }
    }
  }

  async function initializeOpenRequestDelivery(unlisteners: Array<() => void>, isDisposed: () => boolean) {
    try {
      const unlisten = await getCurrentWindow().listen<StackPopupOpenPayload>(STACK_POPUP_OPEN_EVENT, (event: { payload: StackPopupOpenPayload }) => {
        // A newer event always supersedes any pull still in flight.
        openRequestGeneration += 1;
        void handleOpenRequest(event.payload);
      });
      if (isDisposed()) {
        unlisten();
        return;
      }
      unlisteners.push(unlisten);
      await openLatestStackPopupRequest();
    } catch (error) {
      if (isDisposed()) return;
      console.error('Failed to initialize stack popup open listener', error);
      await openLatestStackPopupRequest();
    }
  }

  // Each pull takes a generation; an open event or a newer pull bumps it, so a stale pull result
  // never navigates. Request keys already applied are rejected again inside handleOpenRequest.
  let openRequestGeneration = 0;
  async function pullStackPopupRequest(failure: string) {
    if (disposed) return;
    const generation = ++openRequestGeneration;
    try {
      const payload = await getStackPopupRequest();
      if (disposed || generation !== openRequestGeneration) return;
      await handleOpenRequest(payload);
    } catch (error) {
      console.error(failure, error);
    }
  }

  async function openLatestStackPopupRequest() {
    await pullStackPopupRequest('Failed to load latest stack popup request');
  }

  async function reconcileLatestStackPopupRequest() {
    await pullStackPopupRequest('Failed to reconcile latest stack popup request');
  }

  async function handleOpenRequest(payload: StackPopupOpenPayload) {
    if (disposed) return;
    const path = stackPopupOpenPath(payload);
    const requestKey = stackPopupRequestKey(payload);
    if (!path || !requestKey || requestKey === lastHandledOpenRequestKey || requestKey === pendingOpenRequestKey) {
      return;
    }

    const popup = getCurrentWindow();
    const visibilityEpoch = folderWatchEpoch;
    if (typeof popup.isVisible === 'function' && !await popup.isVisible()) return;
    if (visibilityEpoch !== folderWatchEpoch && !folderWatchVisible) return;
    if (requestKey === lastHandledOpenRequestKey || requestKey === pendingOpenRequestKey) return;
    folderWatchVisible = true;
    folderWatchEnabled = true;
    pendingOpenRequestKey = requestKey;
    try {
      lastHandledOpenRequestKey = requestKey;
      await requestEditorExit(async () => {
        if (disposed) return;
        await performOpenFolder(path);
      });
    } finally {
      if (pendingOpenRequestKey === requestKey) {
        pendingOpenRequestKey = null;
      }
    }
  }

  async function openFolder(folderPath: string, _options: { warmTerminal?: boolean } = {}) {
    if (disposed) return;
    await requestEditorExit(() => performOpenFolder(folderPath, _options));
  }

  async function performOpenFolder(folderPath: string, _options: { warmTerminal?: boolean } = {}) {
    if (disposed) return;
    closeMenus();
    prepareGitStateForFolderCommit(folderPath);
    stackState = openStackFolder(stackState, folderPath);
    await loadFolder(stackState.currentPath);
  }

  async function submitPathDraft() {
    if (disposed) return;
    if (editorPath) {
      await requestEditorExit(() => submitPathDraft());
      return;
    }
    clearPathSuggestions();
    const folderPath = pathDraft.trim();
    if (!folderPath) {
      resetPathDraft();
      return;
    }

    closeMenus();
    deleteConfirmation = null;
    const loadSequence = ++folderLoadSequence;
    stackSortLockedByUser = false;
    startNewIconHydrationSession(folderPath);
    const listingStartedAt = performance.now();
    let firstPaintDurationMs = 0;
    loadingPath = folderPath;
    errorMessage = '';
    try {
      const read = listStackFolder(folderPath, async (page) => {
        if (disposed || loadSequence !== folderLoadSequence) {
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
      folderReadPending = { sequence: loadSequence, promise: read };
      const listing = await read;
      if (disposed || loadSequence !== folderLoadSequence) {
        return;
      }
      iconDiagnosticsPath = folderPath;
      iconDiagnosticsFirstPaintDurationMs = firstPaintDurationMs || Math.max(0, performance.now() - listingStartedAt);
      iconDiagnosticsMetadataCompleteDurationMs = Math.max(0, performance.now() - listingStartedAt);
      prepareGitStateForFolderCommit(folderPath);
      stackState = commitValidatedStackFolderListing(stackState, folderPath, listing);
      void resolveFolderGitContext(folderPath);
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
      if (folderReadPending?.sequence === loadSequence) folderReadPending = null;
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
    if (disposed || !folderPath) {
      return;
    }

    deleteConfirmation = null;
    const loadSequence = ++folderLoadSequence;
    stackSortLockedByUser = false;
    startNewIconHydrationSession(folderPath);
    const listingStartedAt = performance.now();
    let firstPaintDurationMs = 0;
    let mergedListing: StackFolderListing | null = null;
    loadingPath = folderPath;
    errorMessage = '';
    void resolveFolderGitContext(folderPath);
    try {
      const read = listStackFolder(folderPath, async (page) => {
        if (disposed || loadSequence !== folderLoadSequence) {
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
        scheduleVisibleIconHydration(folderPath, loadSequence);
        updateDetailsViewport();
        maybeFocusDetailsGridAfterPageAppend();
      });
      folderReadPending = { sequence: loadSequence, promise: read };
      const listing = await read;
      if (disposed || loadSequence !== folderLoadSequence) {
        return;
      }
      iconDiagnosticsPath = folderPath;
      iconDiagnosticsFirstPaintDurationMs = firstPaintDurationMs || Math.max(0, performance.now() - listingStartedAt);
      iconDiagnosticsMetadataCompleteDurationMs = Math.max(0, performance.now() - listingStartedAt);
      prepareGitStateForFolderCommit(folderPath);
      stackState = applyStackFolderListing(stackState, folderPath, mergedListing ?? listing);
      scheduleVisibleIconHydration(folderPath, loadSequence);
      updateDetailsViewport();
      maybeFocusDetailsGridAfterPageAppend();
    } catch (error) {
      console.error('Failed to load stack folder', error);
      if (loadSequence === folderLoadSequence && loadingPath === folderPath) {
        errorMessage = error instanceof Error ? error.message : String(error || 'Folder unavailable');
      }
    } finally {
      if (folderReadPending?.sequence === loadSequence) folderReadPending = null;
      if (loadSequence === folderLoadSequence && loadingPath === folderPath) {
        loadingPath = null;
      }
    }
  }

  function sameGitRepository(left: Pick<GitContext, 'repositoryRoot' | 'gitDir'> | null, right: Pick<GitContext, 'repositoryRoot' | 'gitDir'> | null) {
    return left !== null && right !== null
      && left.repositoryRoot === right.repositoryRoot && left.gitDir === right.gitDir;
  }

  function retireGitRepository(context: GitContext | null) {
    gitRepositoryGeneration += 1;
    gitRefreshRevision = 0;
    gitRepositoryContext = context;
    gitSummaryPending = null;
    gitSummaryRequested = false;
    gitStatus = null;
    gitVerifiedFolder = '';
    gitStatusPath = '';
    gitErrorMessage = '';
    gitStatusPopupOpen = false;
  }

  async function resolveFolderGitContext(folderPath: string) {
    const sequence = ++gitContextSequence;
    if (folderPath !== gitVerifiedFolder) {
      gitVerifiedFolder = '';
      gitStatusPath = '';
    }
    try {
      const context = await getStackGitContext(folderPath);
      if (gitDisposed || !folderWatchVisible || sequence !== gitContextSequence) return;
      if (!sameGitRepository(gitRepositoryContext, context)) retireGitRepository(context);
      if (context) {
        gitRepositoryContext = context;
        gitVerifiedFolder = folderPath;
        gitStatusPath = folderPath;
      }
      const watchReady = reconcileFolderWatch(folderPath, context);
      if (context && !gitSummaryRequested) void requestGitSummary();
      await watchReady;
    } catch (error) {
      if (gitDisposed || !folderWatchVisible || sequence !== gitContextSequence) return;
      retireGitRepository(null);
      gitErrorMessage = `Git context unavailable: ${operationErrorMessage(error, 'Try Refresh')}. Retry with Refresh.`;
      await reconcileFolderWatch(folderPath, null);
    }
  }

  function ownsGitRequest(generation: number, revision: number) {
    return !gitDisposed && generation === gitRepositoryGeneration && revision === gitRefreshRevision;
  }

  function requestGitSummary(): Promise<void> {
    const context = gitRepositoryContext;
    if (!context || !folderWatchVisible || gitDisposed) return Promise.resolve();
    const generation = gitRepositoryGeneration;
    const revision = gitRefreshRevision;
    if (gitSummaryPending?.generation === generation && gitSummaryPending.revision === revision) {
      return gitSummaryPending.promise;
    }
    gitErrorMessage = '';
    gitSummaryRequested = true;
    const promise = (async () => {
      try {
        const status = await getStackGitSummary(context.repositoryRoot);
        if (!ownsGitRequest(generation, revision)) return;
        if (status && status.repositoryRoot !== context.repositoryRoot) throw new Error('Repository changed.');
        // A refresh retains the known remote link until this revision's metadata arrives.
        gitStatus = status ? { ...status, remoteRepositoryUrl: gitStatus?.remoteRepositoryUrl ?? status.remoteRepositoryUrl } : null;
        gitStatusPath = gitVerifiedFolder;
        await tick();
        if (status && ownsGitRequest(generation, revision)) void enrichGitSummary(context, generation, revision);
      } catch (error) {
        if (ownsGitRequest(generation, revision)) {
          gitErrorMessage = `Git refresh failed: ${operationErrorMessage(error, 'Status unavailable')}. Retry with Refresh.`;
        }
      } finally {
        if (gitSummaryPending?.generation === generation && gitSummaryPending.revision === revision) gitSummaryPending = null;
      }
    })();
    gitSummaryPending = { generation, revision, promise };
    return promise;
  }

  async function enrichGitSummary(context: GitContext, generation: number, revision: number) {
    try {
      const metadata = await getStackGitMetadata(context.repositoryRoot);
      if (!ownsGitRequest(generation, revision) || !metadata || !sameGitRepository(context, metadata) || !gitStatus) return;
      gitStatus = { ...gitStatus, remoteRepositoryUrl: metadata.remoteRepositoryUrl, ahead: metadata.ahead, behind: metadata.behind };
    } catch (error) {
      if (ownsGitRequest(generation, revision)) gitErrorMessage = `Git metadata unavailable: ${operationErrorMessage(error, 'Try Refresh')}. Retry with Refresh.`;
    }
  }

  function invalidateGitSummary(generation = gitRepositoryGeneration) {
    if (generation !== gitRepositoryGeneration || !gitRepositoryContext) return;
    gitRefreshRevision += 1;
    gitSummaryRequested = false;
    if (folderWatchVisible && gitVerifiedFolder === stackState.currentPath) void requestGitSummary();
  }

  function captureGitInvalidation() {
    const generation = gitRepositoryGeneration;
    const contextSequence = gitContextSequence;
    const folderPath = stackState.currentPath;
    return () => {
      // The first context can finish while a save/mutation is pending. Its new
      // generation still owns this operation if no navigation superseded it.
      if (generation === gitRepositoryGeneration
        || (contextSequence === gitContextSequence && folderPath === gitVerifiedFolder)) invalidateGitSummary();
    };
  }

  async function refreshCurrentFolder() {
    folderWatchVisible = true;
    folderWatchEnabled = true;
    if (!activeFolderWatch) desiredFolderWatch = null;
    invalidateGitSummary();
    await loadFolder(currentPath);
  }

  function prepareGitStateForFolderCommit(folderPath: string) {
    if (folderPath === currentPath) {
      return;
    }
    gitStatusPopupOpen = false;
    gitVerifiedFolder = '';
    gitStatusPath = '';
  }

  function startNewIconHydrationSession(folderPath: string) {
    if (disposed) return;
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
    if (disposed || loadSequence !== folderLoadSequence || folderPath !== stackState.currentPath) {
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
      !disposed
      && loadSequence === folderLoadSequence
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
    if (disposed || loadSequence !== folderLoadSequence || folderPath !== stackState.currentPath) {
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
    if (disposed) return;
    try {
      const batch = await resolveStackItemIcons(paths);
      if (
        disposed
        || loadSequence !== folderLoadSequence
        || folderPath !== stackState.currentPath
        || jobToken !== iconHydrationJobToken
      ) return;
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
      if (disposed || jobToken !== iconHydrationJobToken) {
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
    if (editorPath) {
      await requestEditorExit(() => navigateHistory(direction));
      return;
    }
    const nextState = navigateStackHistory(stackState, direction);
    prepareGitStateForFolderCommit(nextState.currentPath);
    stackState = nextState;
    await loadFolder(stackState.currentPath);
  }

  async function activateEntry(entry: StackEntry) {
    closeMenus();
    if (entry.entryType === 'Folder' || isStackBrowsableArchiveEntry(entry)) {
      await openFolder(entry.path);
    } else if (entry.entryType === 'File' && isStackBasicTextFile(entry.path)) {
      captureFolderViewport();
      editorSaved = captureGitInvalidation();
      editorPath = entry.path;
      loadEditorPane();
      return;
    } else {
      await openStackItem(entry.path);
    }
  }

  function isStackBasicTextFile(path: string) {
    const filename = path.split(/[\\/]/).at(-1) ?? '';
    const extensionIndex = filename.lastIndexOf('.');
    if (extensionIndex <= 0 || extensionIndex === filename.length - 1) return false;
    return STACK_BASIC_TEXT_EXTENSIONS.has(filename.slice(extensionIndex + 1).toLocaleLowerCase());
  }

  async function openSelectedWithPicker() {
    const path = rowMenu?.path;
    if (!path || selectedEntry?.entryType !== 'File' || openWithLaunching) return;
    openWithLaunching = true;
    closeMenus();

    try {
      await openStackItemWithPicker(path);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to open stack item with picker', error);
      errorMessage = operationErrorMessage(error, 'Open with unavailable');
    } finally {
      openWithLaunching = false;
    }
  }

  async function loadOpenWithCandidates(entry: StackEntry) {
    const revision = ++openWithRevision;
    openWithCandidates = [];
    if (entry.entryType !== 'File') {
      openWithStatus = 'ready';
      return;
    }
    openWithStatus = 'loading';

    try {
      const candidates = await listStackOpenWithCandidates(entry.path);
      if (revision === openWithRevision && rowMenu?.path === entry.path) {
        // Never insert rows underneath an engaged pointer or keyboard focus.
        if (openWithOpen) openWithPendingCandidates = candidates;
        else openWithCandidates = candidates;
        openWithStatus = 'ready';
        void positionOpenMenus();
      }
    } catch (error) {
      console.error('Failed to load Open With candidates', error);
      if (revision === openWithRevision && rowMenu?.path === entry.path) openWithStatus = 'error';
    }
  }

  async function openSelectedWithSuggestedApp(app: StackOpenWithCandidate) {
    const path = rowMenu?.path;
    const appId = app.id;
    if (!path || selectedEntry?.entryType !== 'File' || openWithLaunching) return;
    openWithLaunching = true;
    closeMenus();
    try {
      await openStackItemWithApp(path, appId);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to open stack item with app', error);
      errorMessage = operationErrorMessage(error, `Open with ${app.label} unavailable`);
    } finally {
      openWithLaunching = false;
    }
  }

  async function copySelected(cut: boolean) {
    const paths = selectedPaths;
    if (!paths.length || fileTransferPending || fileOperationBusy || disposed) return;
    closeMenus();
    fileTransferPending = true;
    try {
      await copyStackItems(paths, cut);
      if (disposed) return;
      errorMessage = '';
      stackState = { ...stackState, statusMessage: `${cut ? 'Cut' : 'Copied'} ${paths.length} item${paths.length === 1 ? '' : 's'}` };
    } catch (error) {
      if (disposed) return;
      console.error('Failed to copy stack items', error);
      errorMessage = operationErrorMessage(error, cut ? 'Cut unavailable' : 'Copy unavailable');
    } finally {
      fileTransferPending = false;
    }
  }

  async function pasteIntoCurrentFolder() {
    const folder = currentPath;
    if (!folder || disposed) return;
    if (fileTransferPending || fileOperationBusy) {
      pasteBusyMessage = 'File operation busy. Try Paste again when it finishes.';
      return;
    }
    pasteBusyMessage = '';
    closeMenus();

    const sequence = folderLoadSequence;
    const epoch = popupVisibilityEpoch;
    fileTransferPending = true;
    const operationId = beginFileOperation('paste', 'planning');
    const refreshGit = captureGitInvalidation();
    try {
      const listing = await invokePaste(folder, operationId, refreshGit, recordUndoMoveId);
      if (disposed || activeOperation?.operationId !== operationId) return;
      if (folder === stackState.currentPath && sequence === folderLoadSequence && epoch === popupVisibilityEpoch) {
        stackState = applyStackFolderListing(stackState, folder, listing);
      }
      errorMessage = pasteFailureSummary(listing.pasteFailures);
      finishFileOperation(operationId, errorMessage || undefined);
    } catch (error) {
      if (disposed || activeOperation?.operationId !== operationId) return;
      console.error('Failed to paste stack items', error);
      errorMessage = operationErrorMessage(error, 'Paste unavailable');
      failFileOperation(operationId, errorMessage);
    } finally {
      fileTransferPending = false;
    }
  }

  function recordUndoMoveId(receipt: string | null | undefined, kind: 'move' | 'delete' = 'move') {
    if (!disposed && receipt !== undefined) undoReceipt = receipt === null ? null : { id: receipt, kind };
  }

  async function undoLastMove() {
    if (!canUndoMove || !undoReceipt || fileTransferPending || disposed || document.hidden) return;
    const receipt = undoReceipt;
    const folder = stackState.currentPath;
    const sequence = folderLoadSequence;
    const epoch = popupVisibilityEpoch;
    const refreshGit = captureGitInvalidation();
    fileTransferPending = true;
    undoReceipt = null;
    errorMessage = '';
    const operationId = beginFileOperation('paste', 'moving', { message: receipt.kind === 'delete' ? 'Undoing delete…' : 'Undoing move…' });
    try {
      const result = await undoStackMove(receipt.id);
      if (disposed) return;
      if (result.moved) {
        refreshGit();
        await refreshExternalFolderRows(folder, sequence, epoch, true);
      }
      if (disposed || activeOperation?.operationId !== operationId) return;
      errorMessage = pasteFailureSummary(result.failures, 'Undo');
      finishFileOperation(operationId, errorMessage || undefined);
      activeOperation = { ...activeOperation, message: errorMessage ? 'Undo incomplete' : result.moved ? (receipt.kind === 'delete' ? 'Deleted items restored' : 'Move undone') : 'Nothing to undo' };
    } catch (error) {
      if (disposed || activeOperation?.operationId !== operationId) return;
      errorMessage = operationErrorMessage(error, `Undo ${receipt.kind} unavailable`);
      failFileOperation(operationId, errorMessage);
      activeOperation = { ...activeOperation, message: 'Undo failed' };
    } finally {
      fileTransferPending = false;
    }
  }

  async function deleteSelected() {
    if (fileTransferPending || fileOperationBusy || deleteConfirmation || disposed || !folderWatchVisible || document.hidden || loadingPath || hasRetainedRows) return;
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
      folderPath: currentPath,
      loadSequence: folderLoadSequence,
      visibilityEpoch: popupVisibilityEpoch
    };
  }

  function cancelDeleteConfirmation() {
    deleteConfirmation = null;
    focusDetailsGrid();
  }

  async function confirmDeleteSelection() {
    const pendingDelete = deleteConfirmation;
    if (!pendingDelete || fileTransferPending || fileOperationBusy) return;

    const { loadSequence: sequence, visibilityEpoch: epoch } = pendingDelete;
    const isCurrent = () => !disposed && folderWatchVisible && !document.hidden &&
      currentPath === pendingDelete.folderPath && sequence === folderLoadSequence && epoch === popupVisibilityEpoch;
    if (!isCurrent()) {
      deleteConfirmation = null;
      return;
    }
    const refreshGit = captureGitInvalidation();
    fileTransferPending = true;
    deleteConfirmation = null;
    const operationId = beginFileOperation('delete', 'deleting', {
      determinate: false,
      completedItems: 0,
      totalItems: pendingDelete.paths.length
    });
    let focusHoldStarted = false;
    try {
      await beginStackPopupFocusLossHold();
      focusHoldStarted = true;
      if (!await popupIsVisible() || !isCurrent()) {
        finishFileOperation(operationId);
        return;
      }
      const result = await deleteStackItems(pendingDelete.paths, operationId);
      if (disposed) return;
      recordUndoMoveId(result.undoMoveId, 'delete');
      if (result.deleted) {
        refreshGit();
        refreshFileOperation(operationId);
        await refreshExternalFolderRows(pendingDelete.folderPath, sequence, epoch, true);
      }
      if (disposed || activeOperation?.operationId !== operationId) return;
      if (sequence === folderLoadSequence && epoch === popupVisibilityEpoch) focusDetailsGrid();
      errorMessage = pasteFailureSummary(result.failures, 'Delete');
      finishFileOperation(operationId, errorMessage || undefined);
    } catch (error) {
      if (disposed || activeOperation?.operationId !== operationId) return;
      console.error('Failed to delete stack item', error);
      errorMessage = operationErrorMessage(error, 'Delete unavailable');
      failFileOperation(operationId, errorMessage);
    } finally {
      if (focusHoldStarted) {
        try {
          await endStackPopupFocusLossHold();
        } catch (error) {
          console.error('Failed to release stack popup focus hold', error);
          errorMessage ||= operationErrorMessage(error, 'Stack Browser focus restore failed');
        }
      }
      fileTransferPending = false;
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
    const refreshGit = captureGitInvalidation();
    try {
      const created = await newStackTextFile(currentPath);
      refreshGit();
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

  function handleEditorDirtyChange(dirty: boolean) {
    editorDirty = dirty;
  }

  function requestEditorClose(dirty: boolean) {
    editorDirty = dirty;
    void requestEditorExit(() => {
      focusDetailsGrid();
    });
  }

  function dismissEditor() {
    editorPath = null;
    editorDirty = false;
  }

  async function requestEditorExit(action: () => void | Promise<void>) {
    if (textEditor?.hasPendingFileOperation?.()) {
      errorMessage = 'Wait for the editor operation to finish before closing.';
      return;
    }
    const request = beginBasicTextEditorExit(pendingEditorExit, Boolean(editorPath), editorDirty, action, dismissEditor);
    pendingEditorExit = request.state;
    if (request.completion) {
      await request.completion;
      return;
    }
    if (!request.accepted) return;
    editorExitFocusOrigin = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  }

  function cancelEditorExit() {
    pendingEditorExit = cancelPendingEditorExit(pendingEditorExit);
  }

  async function confirmEditorExit() {
    const pending = takePendingEditorExit(pendingEditorExit);
    pendingEditorExit = pending.state;
    const action = pending.action;
    if (!action) return;
    dismissEditor();
    await action();
  }

  function closeStackPopupFromSurface() {
    void requestEditorExit(async () => {
      suspendFolderWatch();
      // A Git pane still loading or failed must not mount into a hidden popup.
      if (gitStatusPopupOpen && !StackGitPanelPane) closeGitStatusPopup();
      await hideStackPopup();
    }).catch((error) => {
      console.error('Failed to hide stack popup', error);
    });
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

    const refreshGit = captureGitInvalidation();
    try {
      const createdEntry = await newStackFolder(currentPath, name);
      refreshGit();
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

    const operationId = beginFileOperation('extract', 'extracting', { currentPath: archive.path, determinate: false });
    const refreshGit = captureGitInvalidation();
    try {
      await invokeExtract(archive.path, destinationMode, extractor, operationId);
      refreshGit();
      refreshFileOperation(operationId);
      const listing = await listStackFolder(currentPath);
      stackState = applyStackFolderListing(stackState, currentPath, listing);
      errorMessage = '';
      finishFileOperation(operationId);
    } catch (error) {
      console.error('Failed to extract stack archive', error);
      errorMessage = operationErrorMessage(error, 'Extract archive unavailable');
      failFileOperation(operationId, errorMessage);
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

    const refreshGit = captureGitInvalidation();
    try {
      const renamedEntry = await renameStackItem(selectedEntry.path, nextName);
      refreshGit();
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

  function closeMenus() {
    ++openWithRevision;
    closeOpenWith();
    openWithCandidates = [];
    openWithPendingCandidates = null;
    rowMenu = null;
    backgroundMenu = null;
  }

  function openGitStatusPopup(filter: StackGitFileStatusKind | 'all' = 'all') {
    closeMenus();
    captureFolderViewport();
    gitStatusPopupFilter = filter;
    gitPanelRefresh = captureGitInvalidation();
    gitStatusPopupOpen = true;
    loadGitPane();
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
    const triggerTop = openWithTrigger?.getBoundingClientRect().top ?? rect.top;
    const panel = rowSubmenuPanelElement?.getBoundingClientRect();
    const width = panel?.width ?? 144;
    const padding = STACK_CONTEXT_MENU_VIEWPORT_PADDING;
    rowSubmenuOpensLeft = rect.right + width + 4 > window.innerWidth && rect.left > window.innerWidth - rect.right;
    const x = rowSubmenuOpensLeft ? rect.left - width - 4 : rect.right + 4;
    openWithPosition = {
      x: Math.max(padding, Math.min(x, window.innerWidth - padding - width)),
      y: Math.max(padding, Math.min(triggerTop, window.innerHeight - padding - (panel?.height ?? 0)))
    };
    const maxHeight = availableContextMenuHeight();
    return {
      ...menu,
      width: rect.width,
      submenuMaxHeight: maxHeight,
      submenuTop: Math.max(0, triggerTop - rect.top)
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

  function beginFileOperation(
    operation: StackFileOperationProgress['operation'],
    phase: StackFileOperationProgress['phase'],
    details: Partial<StackFileOperationProgress> = {}
  ) {
    const operationId = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const startedAt = Date.now();
    activeOperation = {
      operationId,
      operation,
      phase,
      determinate: false,
      ...details,
      startedAt,
      updatedAt: startedAt,
      elapsedMs: 0
    };
    startOperationClock();
    errorMessage = '';
    return operationId;
  }

  function startOperationClock() {
    operationClock = Date.now();
    if (operationClockTimer !== null) return;
    operationClockTimer = window.setInterval(() => {
      operationClock = Date.now();
    }, 1000);
  }

  function stopOperationClock() {
    if (operationClockTimer === null) return;
    window.clearInterval(operationClockTimer);
    operationClockTimer = null;
  }

  function refreshFileOperation(operationId: string) {
    if (activeOperation?.operationId !== operationId) return;
    activeOperation = { ...activeOperation, phase: 'refreshing', determinate: false };
  }

  function failFileOperation(operationId: string, message: string) {
    if (activeOperation?.operationId !== operationId) return;
    activeOperation = { ...activeOperation, phase: 'failed', error: message };
    stopOperationClock();
  }

  function finishFileOperation(operationId: string, message?: string) {
    if (activeOperation?.operationId !== operationId) return;
    activeOperation = { ...activeOperation, phase: message ? 'failed' : 'completed', error: message };
    stopOperationClock();
  }

  const invokePaste = pasteStackItems;
  const invokeExtract = extractStackArchive as unknown as (
    archivePath: string,
    destinationMode: StackArchiveDestinationMode,
    extractor: StackArchiveExtractor,
    operationId: string
  ) => ReturnType<typeof extractStackArchive>;

  function operationLabel(operation: StackFileOperationProgress['operation']) {
    return operation === 'paste' ? 'Paste' : operation === 'delete' ? 'Delete' : 'Extract';
  }

  function operationPhaseLabel(operation: StackFileOperationProgress) {
    if (operation.phase === 'completed') return `${operationLabel(operation.operation)} complete`;
    if (operation.phase === 'failed') return `${operationLabel(operation.operation)} failed`;
    const labels: Record<StackFileOperationProgress['phase'], string> = {
      planning: 'Preparing', copying: 'Copying', moving: 'Moving', deleting: 'Deleting', extracting: 'Extracting',
      refreshing: 'Refreshing', completed: 'Complete', failed: 'Failed'
    };
    return labels[operation.phase];
  }

  function operationElapsed(operation: StackFileOperationProgress) {
    const elapsed = Math.max(operation.elapsedMs, operationClock - operation.startedAt, 0);
    const seconds = Math.floor(elapsed / 1000);
    return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
  }

  function compactBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    const units = ['KB', 'MB', 'GB', 'TB'];
    let value = bytes / 1024;
    let unit = units[0];
    for (let index = 1; value >= 1024 && index < units.length; index += 1) {
      value /= 1024;
      unit = units[index];
    }
    return `${value >= 10 ? Math.round(value) : value.toFixed(1)} ${unit}`;
  }

  function operationPath(path?: string) {
    return path?.split(/[\\/]/).filter(Boolean).at(-1) || path || '';
  }

  function operationStatusText(operation: StackFileOperationProgress) {
    const parts = [operation.message || operationPhaseLabel(operation)];
    if (operation.determinate && operation.totalFiles !== undefined) {
      parts.push(`${operation.completedFiles ?? 0} / ${operation.totalFiles} files`);
    } else if (operation.operation === 'delete' && operation.totalItems !== undefined) {
      parts.push(`${operation.completedItems ?? 0} / ${operation.totalItems} items`);
    }
    if (operation.determinate && operation.totalBytes !== undefined) {
      parts.push(`${compactBytes(operation.completedBytes ?? 0)} / ${compactBytes(operation.totalBytes)}`);
    }
    const path = operationPath(operation.currentPath);
    if (path) parts.push(path);
    parts.push(operationElapsed(operation));
    return parts.join(' · ');
  }

  function operationProgressValue(operation: StackFileOperationProgress) {
    if (operation.totalBytes) return Math.min(1, (operation.completedBytes ?? 0) / operation.totalBytes);
    if (operation.totalFiles) return Math.min(1, (operation.completedFiles ?? 0) / operation.totalFiles);
    return 0;
  }

  function contextSubmenuTopCss(menu: StackContextMenuPlacement) {
    return `${Math.max(0, Math.round(menu.submenuTop ?? 0))}px`;
  }

  async function handleRowMenuKeydown(event: KeyboardEvent) {
    // The folder's window-level shortcuts must not activate files behind a menu.
    event.stopPropagation();
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      if (openWithOpen) closeOpenWith(true);
      else closeMenus();
      return;
    }
    const inSubmenu = rowSubmenuPanelElement?.contains(event.target as Node);
    if (inSubmenu && event.key === 'ArrowLeft') {
      event.preventDefault();
      event.stopPropagation();
      closeOpenWith(true);
    } else if (inSubmenu && ['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
      event.preventDefault();
      event.stopPropagation();
      const buttons = Array.from(rowSubmenuPanelElement!.querySelectorAll<HTMLButtonElement>('button:not(:disabled)'));
      const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (index + (event.key === 'ArrowUp' ? -1 : 1) + buttons.length) % buttons.length;
      buttons[next]?.focus();
    } else if (event.target === openWithTrigger && ['ArrowRight', 'Enter', ' '].includes(event.key)) {
      event.preventDefault();
      event.stopPropagation();
      await openOpenWith(event.key === 'ArrowRight');
    }
  }

  function cancelOpenWithClose() {
    if (openWithCloseTimer !== null) window.clearTimeout(openWithCloseTimer);
    openWithCloseTimer = null;
  }

  function closeOpenWith(restoreFocus = false) {
    cancelOpenWithClose();
    openWithOpen = false;
    if (openWithPendingCandidates) {
      openWithCandidates = openWithPendingCandidates;
      openWithPendingCandidates = null;
    }
    if (restoreFocus) openWithTrigger?.focus({ preventScroll: true });
  }

  function scheduleOpenWithClose() {
    // Movement across neighboring rows is transit, not an immediate dismissal.
    // Keep the original deadline so settling over another action still closes.
    if (!openWithOpen || openWithCloseTimer !== null) return;
    openWithCloseTimer = window.setTimeout(() => closeOpenWith(), 220);
  }

  async function openOpenWith(focus = false) {
    if (!rowMenu || selectedEntry?.entryType !== 'File') return;
    cancelOpenWithClose();
    openWithOpen = true;
    await positionOpenMenus();
    if (focus && openWithOpen) rowSubmenuPanelElement?.querySelector<HTMLElement>('button:not(:disabled)')?.focus();
  }

  function handleParentMenuPointer(event: PointerEvent) {
    if ((event.target as HTMLElement).closest('.context-submenu')) cancelOpenWithClose();
    else scheduleOpenWithClose();
  }

  function observeOpenWithContent(node: HTMLElement) {
    const observer = new ResizeObserver(() => { if (openWithOpen) void positionOpenMenus(); });
    observer.observe(node);
    return { destroy: () => observer.disconnect() };
  }

  function focusDetailsGrid() {
    window.requestAnimationFrame(() => { if (!rowMenu && !backgroundMenu) detailsGrid?.focus({ preventScroll: true }); });
  }

  let folderViewport: { path: string; search: string; scrollTop: number; height: number } | null = null;

  function captureFolderViewport() {
    if (editorPath || gitStatusPopupOpen || !detailsBody) return;
    detailsBodyScrollTop = detailsBody.scrollTop;
    detailsBodyHeight = detailsBody.getBoundingClientRect().height;
    folderViewport = { path: currentPath, search: searchQuery, scrollTop: detailsBodyScrollTop, height: detailsBodyHeight };
  }

  function restoreFolderViewport(node: HTMLElement) {
    const checkpoint = folderViewport;
    if (!checkpoint || checkpoint.path !== currentPath || checkpoint.search !== searchQuery) {
      folderViewport = null;
      return;
    }
    detailsBodyScrollTop = checkpoint.scrollTop;
    detailsBodyHeight = checkpoint.height;
    // Let virtual spacers mount before applying the real DOM scroll offset.
    let cancelled = false;
    void tick().then(() => {
      if (folderViewport !== checkpoint) return;
      if (cancelled || checkpoint.path !== currentPath || checkpoint.search !== searchQuery) {
        // Abandoned restoration must not keep the destination viewport blocked.
        // Only release this callback's checkpoint, never a newer capture.
        folderViewport = null;
        return;
      }
      node.scrollTop = checkpoint.scrollTop;
      detailsBodyHeight = node.getBoundingClientRect().height;
      folderViewport = null;
      emitVisibleRowsWindowChanged();
    });
    return { destroy() { cancelled = true; } };
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
      if (editorPath || gitStatusPopupOpen || folderViewport) return;
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
    if (editorPath || gitStatusPopupOpen || folderViewport) return;
    detailsBodyScrollTop = detailsBody?.scrollTop ?? 0;
    detailsBodyHeight = detailsBody?.getBoundingClientRect().height ?? 0;
    emitVisibleRowsWindowChanged();
  }

  async function handleStackSearchInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    searchQuery = input.value;
    folderViewport = null;
    detailsBodyScrollTop = 0;
    if (detailsBody) {
      detailsBody.scrollTop = 0;
    }
    await tick();
    emitVisibleRowsWindowChanged();
  }

  async function pinCurrentFolderToQuickBar() {
    closeMenus();
    if (!currentPath) {
      return;
    }

    try {
      await pinStackFolder(currentPath);
      errorMessage = '';
    } catch (error) {
      console.error('Failed to pin current stack folder', error);
      errorMessage = operationErrorMessage(error, 'Pin unavailable');
    }
  }

  function handleStackSearchKeydown(event: KeyboardEvent) {
    const input = event.currentTarget as HTMLInputElement;
    if (event.key === 'Escape') {
      event.preventDefault();
      input.blur();
    }
    event.stopPropagation();
  }

  async function clearStackSearch() {
    folderViewport = null;
    searchQuery = '';
    detailsBodyScrollTop = 0;
    if (detailsBody) {
      detailsBody.scrollTop = 0;
    }
    await tick();
    emitVisibleRowsWindowChanged();
    stackSearchInput?.focus();
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

  function selectEntryFromMouse(event: MouseEvent, entry: StackEntry) {
    if (hasRetainedRows) {
      return;
    }
    const mode = event.shiftKey ? 'range' : event.ctrlKey || event.metaKey ? 'toggle' : 'single';
    stackState = selectStackEntry(stackState, entry.path, mode);
  }

  function stackGitStatusSymbol(status: StackGitFileStatusKind | null | undefined) {
    if (status === 'added') return 'A';
    if (status === 'deleted') return 'D';
    if (status === 'modified') return 'M';
    if (status === 'untracked') return '?';
    if (status === 'conflict') return 'M';
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

  // Row badges resolve through a precomputed index (see `stackGitStatusFromIndex`) built in the
  // reactive block near `visibleEntries`; it keeps the `stackGitStatusPathMatchesEntry` matching rules.
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
    closeMenus();
    rowMenu = { x: event.clientX, y: event.clientY, path: entry.path };
    void loadOpenWithCandidates(entry);
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
    closeMenus();
    backgroundMenu = { x: event.clientX, y: event.clientY };
    void positionOpenMenus();
  }

  function selectedDragPaths(entry: StackEntry) {
    return stackState.selectedPaths.includes(entry.path) ? [...selectedStackPaths(stackState)] : [entry.path];
  }

  function beginRowDrag(event: PointerEvent, entry: StackEntry) {
    if (!event.isPrimary || event.button !== 0 || rowDragIntent) return;
    if (!canDropLocally() || pendingLocalDrop || outboundDragging) return;
    rowDragIntent = beginStackRowDrag({ pointerId: event.pointerId, button: event.button, buttons: event.buttons, x: event.clientX, y: event.clientY });
    rowDragPath = rowDragIntent ? entry.path : null;
  }

  function resetDragClickOnPress(event: PointerEvent) {
    if (!event.isPrimary || event.button !== 0) return;
    if (rowDragIntent?.pointerId === event.pointerId) clearRowDrag();
    // Only a fresh pointer press unlatches suppression; keyboard clicks are independent.
    suppressRowClick = false;
  }

  function clearRowDrag() {
    const pointerId = rowDragIntent?.pointerId;
    const capture = rowDragCapture;
    rowDragIntent = null;
    rowDragPath = null;
    rowDragCapture = null;
    localDrag = null;
    localDropTargetPath = null;
    if (pointerId !== undefined && capture?.hasPointerCapture(pointerId)) capture.releasePointerCapture(pointerId);
  }

  function cancelRowDrag() {
    clearRowDrag();
    pendingLocalDrop = null;
  }

  function cancelRowDragOnEscape(event: KeyboardEvent) {
    if (event.key !== 'Escape' || (!rowDragIntent && !pendingLocalDrop)) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    cancelRowDrag();
  }

  function suppressTrailingDragClick(event: MouseEvent) {
    if (!suppressRowClick || event.detail === 0) return;
    event.preventDefault();
    event.stopImmediatePropagation();
  }

  function cancelRowPointer(event: PointerEvent) {
    if (rowDragIntent?.pointerId !== event.pointerId) return;
    if (event.type === 'lostpointercapture' && event.target !== rowDragCapture) return;
    clearRowDrag();
  }

  function canDropLocally(session?: LocalDragSession) {
    return !disposed && !fileTransferPending && folderWatchVisible && !document.hidden && !loadingPath && !hasRetainedRows &&
      !!stackState.currentPath && stackState.entriesPath === stackState.currentPath &&
      !editorPath && !gitStatusPopupOpen && !rowMenu && !backgroundMenu &&
      !deleteConfirmation && !pendingEditorExit && createFolderDraft === null && renameDraft === null &&
      (!activeOperation || activeOperation.phase === 'completed' || activeOperation.phase === 'failed') &&
      (!session || (session.grid === detailsGrid && session.grid.isConnected &&
        session.folderPath === stackState.currentPath && session.loadSequence === folderLoadSequence &&
        session.visibilityEpoch === popupVisibilityEpoch));
  }

  function startLocalDrag(event: PointerEvent) {
    const grid = detailsGrid;
    const entry = stackState.entries.find((item) => item.path === rowDragPath);
    if (!grid || !entry || !canDropLocally()) return null;
    const paths = selectedDragPaths(entry);
    if (!stackState.selectedPaths.includes(entry.path)) stackState = selectStackEntry(stackState, entry.path);
    suppressRowClick = true;
    // Capture only after threshold, on the stable grid rather than the virtual source row.
    grid.setPointerCapture(event.pointerId);
    rowDragCapture = grid;
    return {
      paths, grid, folderPath: stackState.currentPath,
      loadSequence: folderLoadSequence, visibilityEpoch: popupVisibilityEpoch,
      prepared: Promise.resolve().then(() => prepareStackLocalDrop(paths)).then(
        (requestId) => ({ requestId }), (error: unknown) => ({ error })
      )
    };
  }

  function moveRowDrag(event: PointerEvent) {
    if (rowDragIntent?.pointerId !== event.pointerId) return;
    if (!(event.buttons & 1) || (localDrag && !rowDragCapture?.hasPointerCapture(event.pointerId))) {
      clearRowDrag();
      return;
    }
    if (!localDrag && moveStackRowDrag(rowDragIntent, {
      pointerId: event.pointerId, buttons: event.buttons, x: event.clientX, y: event.clientY
    }).startNativeDrag) localDrag = startLocalDrag(event);
    const session = localDrag;
    if (!session) return;
    if (!canDropLocally(session)) {
      clearRowDrag();
    } else if (event.clientX < 0 || event.clientY < 0 || event.clientX >= window.innerWidth || event.clientY >= window.innerHeight) {
      clearRowDrag();
      void handOffOutboundDrag(session);
    } else {
      localDropTargetPath = localDropTarget(session, event.clientX, event.clientY)?.path ?? null;
    }
  }

  async function handOffOutboundDrag(session: LocalDragSession) {
    outboundDragging = true;
    try {
      const outcome = await startStackFileDrag(session.paths);
      if (outcome.status === 'failed' || outcome.status === 'unsupported') {
        if (canDropLocally(session)) errorMessage = outcome.message || `Native file drag unavailable (${outcome.stage ?? 'unknown'})`;
      }
    } catch (error) {
      if (canDropLocally(session)) errorMessage = operationErrorMessage(error, 'Native file drag unavailable');
    } finally {
      outboundDragging = false;
    }
  }

  function endRowDrag(event: PointerEvent) {
    if (rowDragIntent?.pointerId !== event.pointerId) return;
    const session = localDrag;
    const { clientX, clientY } = event;
    const target = session && event.button === 0 && !(event.buttons & 1) && rowDragCapture?.hasPointerCapture(event.pointerId)
      ? localDropTarget(session, clientX, clientY) : null;
    clearRowDrag();
    if (session && target) {
      pendingLocalDrop = session;
      errorMessage = '';
      void finishLocalDrop(session, target, clientX, clientY);
    }
  }

  async function finishLocalDrop(session: LocalDragSession, target: StackEntry, x: number, y: number) {
    const prepared = await session.prepared;
    if (pendingLocalDrop !== session) return;
    pendingLocalDrop = null;
    const liveTarget = localDropTarget(session, x, y);
    if (!liveTarget || liveTarget.path !== target.path || liveTarget.id !== target.id) return;
    if ('error' in prepared) {
      errorMessage = operationErrorMessage(prepared.error, 'Move unavailable');
      return;
    }
    await moveLocalDrop(session, prepared.requestId, liveTarget.path);
  }

  function stackDropPathKey(path: string) {
    return path.replace(/\//g, '\\').replace(/^\\\\\?\\UNC\\/i, '\\\\').replace(/^\\\\\?\\/, '')
      .replace(/\\+$/, '').toLowerCase();
  }

  function localDropTarget(session: LocalDragSession, x: number, y: number) {
    if (!canDropLocally(session)) return null;
    const row = document.elementFromPoint(x, y)?.closest<HTMLButtonElement>('button[role="row"][data-stack-entry-path]');
    if (!row || row.disabled || row.parentElement !== detailsBody || !session.grid.contains(row)) return null;
    const destination = stackState.entries.find((entry) => entry.path === row.dataset.stackEntryPath);
    if (!destination || destination.entryType !== 'Folder' || destination.isSymlink || destination.isReparsePoint ||
        destination.path === session.folderPath || session.paths.includes(destination.path) ||
        stackState.selectedPaths.includes(destination.path)) return null;
    return destination;
  }

  function handleNativeDrop({ paths }: Extract<DragDropEvent, { type: 'drop' }>) {
    if (disposed || !paths.length) return;
    const keys = paths.map(stackDropPathKey);
    if (lastHtmlDrop?.key === JSON.stringify(keys.sort()) && performance.now() - lastHtmlDrop.at <= STACK_DROP_DELIVERY_GRACE_MS) return;
    void pasteDroppedPaths(paths, stackState.currentPath, false);
  }

  async function moveLocalDrop(session: LocalDragSession, requestId: string, destination: string) {
    if (!canDropLocally(session)) return;
    fileTransferPending = true;
    const operationId = beginFileOperation('paste', 'moving');
    const refreshGit = captureGitInvalidation();
    try {
      const result = await commitStackLocalDrop(requestId, destination);
      if (disposed) return;
      recordUndoMoveId(result.undoMoveId);
      if (result.moved) {
        refreshGit();
        await refreshExternalFolderRows(session.folderPath, session.loadSequence, session.visibilityEpoch, true);
      }
      if (disposed || activeOperation?.operationId !== operationId) return;
      errorMessage = pasteFailureSummary(result.failures);
      finishFileOperation(operationId, errorMessage || undefined);
    } catch (error) {
      if (disposed || activeOperation?.operationId !== operationId) return;
      errorMessage = operationErrorMessage(error, 'Move unavailable');
      failFileOperation(operationId, errorMessage);
    } finally {
      fileTransferPending = false;
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
    const paths = pathsFromDrop(event);
    const keys = paths.map(stackDropPathKey);
    if (!keys.length) return;
    lastHtmlDrop = { at: performance.now(), key: JSON.stringify(keys.sort()) };
    closeMenus();
    await pasteDroppedPaths(paths, destinationPath, event.shiftKey);
  }

  async function pasteDroppedPaths(paths: string[], destinationPath: string, move: boolean) {
    if (!paths.length || !destinationPath || fileTransferPending || fileOperationBusy || disposed) return;
    const folder = stackState.currentPath;
    const sequence = folderLoadSequence;
    const epoch = popupVisibilityEpoch;
    fileTransferPending = true;
    const operationId = beginFileOperation('paste', move ? 'moving' : 'copying');
    const refreshGit = captureGitInvalidation();
    try {
      await copyStackItems(paths, move);
      const listing = await invokePaste(destinationPath, operationId, refreshGit, recordUndoMoveId);
      if (disposed) return;
      if (destinationPath === folder && folder === stackState.currentPath && sequence === folderLoadSequence && epoch === popupVisibilityEpoch) {
        stackState = applyStackFolderListing(stackState, folder, listing);
      } else if (destinationPath !== folder) {
        await refreshExternalFolderRows(folder, sequence, epoch, true);
      }
      if (disposed || activeOperation?.operationId !== operationId) return;
      errorMessage = pasteFailureSummary(listing.pasteFailures);
      finishFileOperation(operationId, errorMessage || undefined);
    } catch (error) {
      if (disposed || activeOperation?.operationId !== operationId) return;
      console.error('Failed to drop stack items', error);
      errorMessage = operationErrorMessage(error, 'Drop unavailable');
      failFileOperation(operationId, errorMessage);
      await refreshExternalFolderRows(folder, sequence, epoch, true);
    } finally {
      fileTransferPending = false;
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

  function isEditableKeyTarget(target: EventTarget | null) {
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) {
      return true;
    }
    return target instanceof HTMLElement && (target.isContentEditable || Boolean(target.closest('.inline-editor, [contenteditable="true"]')));
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.key.toLowerCase() === 'z') {
      if (event.defaultPrevented || event.repeat || event.isComposing || event.keyCode === 229 ||
          event.metaKey || event.altKey || event.shiftKey || event.composedPath().some(isEditableKeyTarget) ||
          !canUndoMove || fileTransferPending || disposed || document.hidden) return;
      event.preventDefault();
      void undoLastMove();
      return;
    }
    if (editorPath || gitStatusPopupOpen || isEditableKeyTarget(event.target)) {
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
      } else if (openWithOpen) {
        // Hover does not move focus into the menu; preserve that focus on Escape.
        closeOpenWith();
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
        closeMenus();
        rowMenu = { x: 24, y: 72, path: entry.path };
        void loadOpenWithCandidates(entry);
        void positionOpenMenus();
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
    if ((event as PointerEvent).pointerType !== 'mouse' || (event.button !== 3 && event.button !== 4)) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    const direction = event.button === 3 ? -1 : 1;
    if (pendingEditorExit) { editorExitDialog?.handleMouseNavigation(direction); return; }
    if (deleteConfirmation) { deleteDialog?.handleMouseNavigation(direction); return; }
    if (editorPath) {
      // Pane still loading or failed to load: no child exists, so Back leaves via the shared exit path.
      if (textEditor) textEditor.handleMouseNavigation(direction);
      else if (direction === -1) requestEditorClose(false);
      return;
    }
    if (gitStatusPopupOpen) {
      if (gitPanel) gitPanel.handleMouseNavigation(direction);
      else if (direction === -1) closeGitStatusPopup();
      return;
    }
    if (rowMenu || backgroundMenu) { if (direction === -1) closeMenus(); return; }
    if (renameDraft !== null || createFolderDraft !== null) { if (direction === -1) cancelInlineEditor(); return; }
    if (direction === -1 ? canGoBack : canGoForward) void navigateHistory(direction);
  }

  let textEditor: StackTextEditorComponent | null | undefined;
  let gitPanel: StackGitPanelComponent | null | undefined;
  // FE-34 lazy panes. A failed import() is cached by the browser for the same specifier, so Retry may keep failing until reload.
  let StackTextEditorPane: typeof StackTextEditorComponent | null = null;
  let StackGitPanelPane: typeof StackGitPanelComponent | null = null;
  let editorLoadError = false;
  let gitLoadError = false;
  let editorLoadToken = 0;
  let gitLoadToken = 0;
  $: if (!editorPath) editorLoadError = false;
  $: if (!gitStatusPopupOpen) gitLoadError = false;

  function focusPendingPane(node: HTMLElement) {
    node.focus({ preventScroll: true });
  }

  // Escape leaves a pane that is still loading or failed, as it does once mounted (no child owns the key yet).
  function handlePendingPaneEscape(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.defaultPrevented) return;
    if (editorPath && !StackTextEditorPane) {
      event.preventDefault();
      requestEditorClose(false);
    } else if (!editorPath && gitStatusPopupOpen && !StackGitPanelPane) {
      event.preventDefault();
      closeGitStatusPopup();
    }
  }

  function loadEditorPane() {
    if (StackTextEditorPane) return;
    editorLoadError = false;
    const token = ++editorLoadToken;
    import('./StackTextEditor.svelte').then((module) => {
      if (!disposed) StackTextEditorPane = module.default;
    }).catch((error) => {
      console.error('Failed to load Stack editor', error);
      if (!disposed && token === editorLoadToken && editorPath) editorLoadError = true;
    });
  }

  function loadGitPane() {
    if (StackGitPanelPane) return;
    gitLoadError = false;
    const token = ++gitLoadToken;
    import('./StackGitPanel.svelte').then((module) => {
      if (!disposed) StackGitPanelPane = module.default;
    }).catch((error) => {
      console.error('Failed to load Stack Git panel', error);
      if (!disposed && token === gitLoadToken && gitStatusPopupOpen) gitLoadError = true;
    });
  }
  let deleteDialog: StackConfirmDialog | undefined;
  let editorExitDialog: StackConfirmDialog | undefined;

  function suppressMouseNavigationCompatibility(event: MouseEvent) {
    if (event.button !== 3 && event.button !== 4) return;
    // WebView compatibility events must not perform browser history or reach
    // outside-click handlers after pointerdown has already resolved the owner.
    event.preventDefault();
    event.stopImmediatePropagation();
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

<style>
  button.local-drop-target {
    background: var(--js-color-selected);
    outline: 2px solid var(--js-color-accent-border);
    outline-offset: -2px;
    cursor: move;
  }
</style>

<svelte:window
  on:keydown|capture={cancelRowDragOnEscape}
  on:keydown={handlePendingPaneEscape}
  on:keydown={handleKeydown}
  on:click|capture={suppressTrailingDragClick}
  on:dblclick|capture={suppressTrailingDragClick}
  on:pointerdown|capture={resetDragClickOnPress}
  on:click={closeMenus}
  on:pointerdown|capture={handleMouseNavigation}
  on:mousedown|capture={suppressMouseNavigationCompatibility}
  on:auxclick|capture={suppressMouseNavigationCompatibility}
  on:pointermove={handleResizePointerMove}
  on:pointermove={handleMarqueePointerMove}
  on:pointermove={moveRowDrag}
  on:pointerup={endResize}
  on:pointerup={endMarqueeSelection}
  on:pointerup={endRowDrag}
  on:pointercancel={endResize}
  on:pointercancel={endMarqueeSelection}
  on:pointercancel={cancelRowPointer}
  on:lostpointercapture={cancelRowPointer}
  on:resize={() => { updateDetailsViewport(); void positionOpenMenus(); }}
/>

<svelte:document on:visibilitychange={() => {
  if (document.hidden) {
    deleteConfirmation = null;
    popupVisibilityEpoch += 1;
    cancelRowDrag();
  }
}} />

<section
  bind:this={stackPopupSurface}
  class:resizing={!!resizeDrag}
  class="stack-popup"
  aria-label="Stack browser"
  aria-busy={loadingPath ? 'true' : 'false'}
  on:contextmenu={handleBackgroundContextMenu}
  on:pointerdown={beginMarqueeSelection}
>
  <MeltActionButton
    class="stack-browser-close-button"
    ariaLabel="Close stack browser"
    tooltip="Close stack browser"
    onClick={() => void closeStackPopupFromSurface()}
  ><MaterialSymbolIcon name="close" /></MeltActionButton>

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
        {#if gitStatus}
          <div class="stack-git-summary" aria-label={`Git ${gitStatus.branch} ${stackGitSummaryParts(gitStatus).map((part) => part.title).join(', ') || 'clean'}`}>
            <button type="button" class="stack-git-branch" disabled={gitVerifiedFolder !== currentPath} on:click={() => openGitStatusPopup('all')}>{gitStatus.branch}</button>
            {#if stackGitSummaryParts(gitStatus).length}
              {#each stackGitSummaryParts(gitStatus) as part (part.status)}
                <button type="button" class={`stack-git-count git-status-${part.status}`} disabled={gitVerifiedFolder !== currentPath} title={part.title} on:click={() => openGitStatusPopup(part.status)}>{part.label}</button>
              {/each}
            {:else}
              <button type="button" class="stack-git-clean" disabled={gitVerifiedFolder !== currentPath} on:click={() => openGitStatusPopup('all')}>clean</button>
            {/if}
            {#if gitStatus.remoteRepositoryUrl}
              <button
                type="button"
                class="stack-git-remote-link"
                disabled={gitVerifiedFolder !== currentPath}
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
      <MeltActionButton class="stack-action-icon-button" ariaLabel="Back" tooltip="Back" disabled={!canGoBack} onClick={() => void navigateHistory(-1)}><MaterialSymbolIcon name="arrow_back" /></MeltActionButton>
      <MeltActionButton class="stack-action-icon-button" ariaLabel="Forward" tooltip="Forward" disabled={!canGoForward} onClick={() => void navigateHistory(1)}><MaterialSymbolIcon name="arrow_forward" /></MeltActionButton>
      <MeltActionButton class="stack-action-icon-button" ariaLabel="Refresh" tooltip="Refresh" onClick={() => void refreshCurrentFolder()}><MaterialSymbolIcon name="refresh" /></MeltActionButton>
      <MeltActionButton class="stack-action-icon-button" ariaLabel={undoLabel} tooltip={`${undoLabel} (Ctrl+Z)`} disabled={!canUndoMove} onKeyDown={(event) => { if (event.key === 'Enter') event.stopPropagation(); }} onClick={() => void undoLastMove()}>
        <svg viewBox="0 -960 960 960" width="16" height="16" fill="currentColor" aria-hidden="true" focusable="false">
          <path d="M280-200v-80h284q65 0 110.5-44T720-432q0-64-45.5-108T564-584H312l104 104-56 56-200-200 200-200 56 56-104 104h252q98 0 167 67t69 165q0 98-69 165t-167 67H280Z" />
        </svg>
      </MeltActionButton>
      <MeltActionButton class="stack-action-icon-button" ariaLabel={selectedEntry?.entryType === 'Folder' ? 'Copy folder' : 'Copy selected item'} tooltip={selectedEntry?.entryType === 'Folder' ? 'Copy folder' : 'Copy selected item'} disabled={!hasSelection} onClick={() => void copySelected(false)}><MaterialSymbolIcon name={selectedEntry?.entryType === 'Folder' ? 'folder_copy' : 'file_copy'} /></MeltActionButton>
      <MeltActionButton class="stack-action-icon-button" ariaLabel="Cut selected item" tooltip="Cut selected item" disabled={!hasSelection} onClick={() => void copySelected(true)}><MaterialSymbolIcon name="content_cut" /></MeltActionButton>
      <MeltActionButton class="stack-action-icon-button" ariaLabel="Paste into current folder" tooltip="Paste into current folder" disabled={!currentPath || fileTransferPending || fileOperationBusy} onClick={() => void pasteIntoCurrentFolder()}><MaterialSymbolIcon name="content_paste" /></MeltActionButton>
      <MeltActionButton class="stack-action-icon-button" ariaLabel="Rename selected item" tooltip="Rename selected item" disabled={!selectedEntry} onClick={beginRenameSelected}><MaterialSymbolIcon name="drive_file_rename" /></MeltActionButton>
      <MeltActionButton class="stack-action-icon-button" ariaLabel="Delete selected item" tooltip="Delete selected item" disabled={!hasSelection || fileTransferPending || fileOperationBusy} onClick={() => void deleteSelected()}><MaterialSymbolIcon name="delete" /></MeltActionButton>
      <MeltActionButton class="stack-action-icon-button" ariaLabel="New folder" tooltip="New folder" disabled={!currentPath} onClick={beginCreateFolder}><MaterialSymbolIcon name="create_new_folder" /></MeltActionButton>
      <MeltActionButton class="stack-action-icon-button" ariaLabel="Reveal selected item" tooltip="Reveal selected item" disabled={!selectedEntry} onClick={() => void revealSelected()}><MaterialSymbolIcon name="preview" /></MeltActionButton>
      <div class="stack-pin-status">
        <MeltActionButton class="stack-action-icon-button" ariaLabel="Pin to quick bar" tooltip="Pin to quick bar" disabled={!currentPath} onClick={() => void pinCurrentFolderToQuickBar()}><MaterialSymbolIcon name="add_location" /></MeltActionButton>
        <span class="stack-status-separator" aria-hidden="true"></span>
        <div class="stack-status" class:error={!!errorMessage || !!gitErrorMessage || !!folderWatchErrorMessage || activeOperation?.phase === 'failed'} class:info={!errorMessage && !gitErrorMessage && !folderWatchErrorMessage} role="status" aria-live="polite">
          {#if activeOperation}
            <div class="stack-operation-status" title={operationStatusText(activeOperation)}>
              {#if activeOperation.determinate}
                <progress
                  max="1"
                  value={operationProgressValue(activeOperation)}
                  aria-label={operationStatusText(activeOperation)}
                ></progress>
              {:else if activeOperation.phase !== 'completed' && activeOperation.phase !== 'failed'}
                <span class="stack-operation-spinner" aria-hidden="true"></span>
              {/if}
              <span>{operationStatusText(activeOperation)}</span>
              {#if (activeOperation.operation === 'paste' || activeOperation.operation === 'delete') && activeOperation.error}
                <span title={activeOperation.error}>{activeOperation.error}</span>
              {/if}
            </div>
          {:else}
            <span title={errorMessage || gitErrorMessage || stackState.statusMessage}>{errorMessage || gitErrorMessage || stackState.statusMessage}</span>
          {/if}
          {#if folderWatchErrorMessage}
            <span title={folderWatchErrorMessage}>{folderWatchErrorMessage}</span>
          {/if}
          {#if pasteBusyMessage}
            <span title={pasteBusyMessage}>{pasteBusyMessage}</span>
          {/if}
          {#if !activeOperation && loadingPath}
            <span title="Loading...">Loading...</span>
          {:else if !activeOperation && iconHydrationStatusMessage}
            <span title={iconHydrationStatusMessage}>{iconHydrationStatusMessage}</span>
          {/if}
        </div>
      </div>
      <div class="stack-search">
        <div class="stack-search-input-wrapper">
          <MaterialSymbolIcon name="search" />
          <input
            bind:this={stackSearchInput}
            aria-label="Search current folder"
            value={searchQuery}
            placeholder="Search folder"
            spellcheck="false"
            autocomplete="off"
            on:input={handleStackSearchInput}
            on:keydown={handleStackSearchKeydown}
          />
          {#if searchQuery}
            <MeltActionButton class="stack-search-clear-button" ariaLabel="Clear search" tooltip="Clear search" onClick={clearStackSearch}>
              <MaterialSymbolIcon name="close" />
            </MeltActionButton>
          {/if}
        </div>
      </div>
    </div>
  </header>

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
  {#if editorPath}
    {#if StackTextEditorPane}
      <svelte:component this={StackTextEditorPane} bind:this={textEditor} path={editorPath} onDirtyChange={handleEditorDirtyChange} onDismiss={requestEditorClose} onSaved={editorSaved} />
    {:else if editorLoadError}
      <div class="empty-stack surface-state error" role="alert" tabindex="-1" use:focusPendingPane>
        <span>Couldn't load the editor. Loading failed.</span>
        <MeltActionButton onClick={loadEditorPane}>Retry</MeltActionButton>
      </div>
    {:else}
      <div class="empty-stack surface-state loading" role="status" aria-label="Loading editor…" tabindex="-1" use:focusPendingPane>Loading editor…</div>
    {/if}
  {:else if gitStatusPopupOpen}
    {#if StackGitPanelPane}
      <svelte:component
        this={StackGitPanelPane}
        bind:this={gitPanel}
        folderPath={currentPath}
        initialStatus={null}
        initialChangeFilter={gitStatusPopupFilter}
        onClose={closeGitStatusPopup}
        onRefresh={gitPanelRefresh}
      />
    {:else if gitLoadError}
      <div class="empty-stack surface-state error" role="alert">
        <span>Couldn't load the Git panel. Loading failed.</span>
        <MeltActionButton onClick={loadGitPane}>Retry</MeltActionButton>
      </div>
    {:else}
      <div class="empty-stack surface-state loading" role="status" aria-label="Loading Git panel…">Loading Git panel…</div>
    {/if}
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
        use:restoreFolderViewport
        data-stack-marquee-start="body"
        on:scroll={handleDetailsBodyScroll}
      >
        {#if virtualEntries.beforeHeight}
          <div class="virtual-spacer" data-stack-marquee-start="spacer" style={`height:${virtualEntries.beforeHeight}px`} aria-hidden="true"></div>
        {/if}
        {#each virtualEntries.rows as virtualRow (virtualRow.item.id)}
          {@const entry = virtualRow.item}
          {@const fileIcon = stackFileIconForEntry(entry)}
          {@const gitEntryStatus = stackGitStatusFromIndex(gitStatusIndex, entry)}
          {@const attributeLabels = stackAttributeLabels(entry)}
          {@const entrySelected = selectedPathSet.has(entry.path)}
          <button
            class:selected={entrySelected}
            class:local-drop-target={localDropTargetPath === entry.path}
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
            aria-selected={entrySelected}
            aria-disabled={hasRetainedRows}
            disabled={hasRetainedRows}
            data-stack-entry-path={entry.path}
            on:click={(event) => selectEntryFromMouse(event, entry)}
            on:dblclick={() => void activateEntry(entry)}
            on:contextmenu={(event) => handleRowContextMenu(event, entry)}
            on:pointerdown={(event) => beginRowDrag(event, entry)}
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
              {#if attributeLabels.length}
                <span class="item-badges" aria-label={attributeLabels.join(', ')}>
                  {#each attributeLabels as label (label)}
                    <span>{label}</span>
                  {/each}
                </span>
              {/if}
              {#if gitEntryStatus}
                <span class={`git-status-badge git-status-${gitEntryStatus}`} aria-label={stackGitStatusLabel(gitEntryStatus)} title={stackGitStatusLabel(gitEntryStatus)}>{stackGitStatusSymbol(gitEntryStatus)}</span>
              {/if}
            </span>
            <span role="gridcell" aria-colindex="2">{entry.typeLabel}</span>
            <span role="gridcell" aria-colindex="3">{formatStackSize(entry.size)}</span>
            <span role="gridcell" aria-colindex="4">{formatStackModified(entry.modifiedMs)}</span>
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
      class="context-menu-shell"
      style={`left:${rowMenu.x}px;top:${rowMenu.y}px;--stack-context-menu-max-height:${contextMenuMaxHeightCss(rowMenu)};--stack-context-menu-left:${rowMenu.x}px;--stack-context-menu-top:${rowMenu.y}px;--stack-context-menu-width:${contextMenuWidthCss(rowMenu)};--stack-context-submenu-max-height:${contextSubmenuMaxHeightCss(rowMenu)};--stack-context-submenu-top:${contextSubmenuTopCss(rowMenu)}`}
      role="none"
      on:click={(event) => event.stopPropagation()}
      on:contextmenu={(event) => event.stopPropagation()}
      on:keydown={(event) => void handleRowMenuKeydown(event)}
      on:pointermove={(event) => { if (rowMenuElement?.contains(event.target as Node)) handleParentMenuPointer(event); }}
      on:focusin={(event) => { if (rowMenuElement?.contains(event.target as Node) && event.target !== openWithTrigger) closeOpenWith(); }}
      on:focusout={(event) => { if (!(event.currentTarget as HTMLElement).contains(event.relatedTarget as Node)) scheduleOpenWithClose(); }}
    >
      <ContextMenu bind:element={rowMenuElement} className="context-menu context-menu-scroll" on:scroll={() => void positionOpenMenus()} on:pointerdown={(event) => { if (!(event.target as HTMLElement).closest('.context-submenu')) closeOpenWith(); }}>
        <ContextMenuItem icon="preview" disabled={!selectedEntry} onClick={() => selectedEntry && void activateEntry(selectedEntry)}>Open</ContextMenuItem>
        <div bind:this={rowSubmenuElement} class:left={rowSubmenuOpensLeft} class="context-submenu" role="none" on:pointerenter={() => void openOpenWith()} on:pointerleave={scheduleOpenWithClose}>
          <button bind:this={openWithTrigger} class="context-menu-item submenu-trigger" type="button" role="menuitem" aria-haspopup="menu" aria-expanded={openWithOpen} aria-controls="stack-open-with-menu" disabled={selectedEntry?.entryType !== 'File'} on:click={() => void openOpenWith()}>
            <span class="context-menu-icon"><MaterialSymbolIcon name="preview" /></span>Open with ▸
          </button>
        </div>
        <ContextMenuItem icon="file_copy" disabled={!hasSelection} onClick={() => void copySelected(false)}>Copy</ContextMenuItem>
        <ContextMenuItem icon="content_cut" disabled={!hasSelection} onClick={() => void copySelected(true)}>Cut</ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem icon="add_location" disabled={selectedEntry?.entryType !== 'Folder'} onClick={() => void pinSelectedFolderToTopBar()}>Pin to Top Bar</ContextMenuItem>
        <ContextMenuItem icon="code_blocks" disabled={selectedEntry?.entryType !== 'Folder'} onClick={() => void openSelectedFolderInVscode()}>Open in VS Code</ContextMenuItem>
        <ContextMenuItem disabled={!selectedZipArchiveEntry()} onClick={() => void extractSelectedArchive('here')}>Extract here</ContextMenuItem>
        <ContextMenuItem disabled={!selectedZipArchiveEntry()} onClick={() => void extractSelectedArchive('folder')}>Extract to folder</ContextMenuItem>
        <ContextMenuItem disabled={!selectedSevenZipArchiveEntry()} onClick={() => void extractSelectedArchive('here', 'sevenZip')}>Extract here with 7-Zip</ContextMenuItem>
        <ContextMenuItem disabled={!selectedSevenZipArchiveEntry()} onClick={() => void extractSelectedArchive('folder', 'sevenZip')}>Extract to folder with 7-Zip</ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem icon="file_copy" disabled={!selectedEntry} onClick={() => void copyTextToClipboard(selectedEntry?.path ?? '', 'Copy path unavailable')}>Copy Path</ContextMenuItem>
        <ContextMenuItem icon="file_copy" disabled={!selectedEntry} onClick={() => void copyTextToClipboard(selectedEntry?.name ?? '', 'Copy name unavailable')}>Copy Name</ContextMenuItem>
        <ContextMenuItem icon="folder_copy" disabled={!selectedEntry} onClick={() => void copyTextToClipboard(selectedDirectoryPath(), 'Copy containing folder unavailable')}>Copy Containing Folder</ContextMenuItem>
        <ContextMenuItem icon="drive_file_rename" disabled={!selectedEntry} onClick={beginRenameSelected}>Rename</ContextMenuItem>
        <ContextMenuItem icon="delete" destructive disabled={!hasSelection} onClick={() => void deleteSelected()}>Delete</ContextMenuItem>
        <ContextMenuItem icon="folder" disabled={!selectedEntry} onClick={() => void revealSelected()}>Reveal</ContextMenuItem>
        <ContextMenuItem icon="settings" disabled={!selectedEntry} onClick={() => void showSelectedProperties()}>Properties</ContextMenuItem>
      </ContextMenu>
      <div class="context-submenu-region" class:open={openWithOpen} class:left={rowSubmenuOpensLeft} style={`left:${openWithPosition.x}px;top:${openWithPosition.y}px`} role="none" on:pointerenter={cancelOpenWithClose} on:pointerleave={scheduleOpenWithClose}>
      <div id="stack-open-with-menu" role="none" use:observeOpenWithContent>
      <ContextMenu bind:element={rowSubmenuPanelElement} ariaLabel="Open with" className="context-menu context-submenu-panel">
        {#each openWithCandidates as app (app.id)}
          <ContextMenuItem disabled={selectedEntry?.entryType !== 'File'} onClick={() => void openSelectedWithSuggestedApp(app)}>{app.label}</ContextMenuItem>
        {/each}
        {#if openWithStatus === 'loading'}
          <div class="open-with-status" role="status">Loading apps...</div>
        {:else if openWithStatus === 'error'}
          <div class="open-with-status" role="status">Apps unavailable. Choose an app below.</div>
        {:else if openWithPendingCandidates}
          <div class="open-with-status" role="status">Apps ready. Reopen to view.</div>
        {:else if !openWithCandidates.length}
          <div class="open-with-status" role="status">No associated apps.</div>
        {/if}
        <ContextMenuItem disabled={selectedEntry?.entryType !== 'File'} onClick={() => void openSelectedWithPicker()}>Choose app...</ContextMenuItem>
      </ContextMenu>
      </div>
      </div>
    </div>
  {/if}

  {#if backgroundMenu}
    <ContextMenu
      className="context-menu"
      style={`left:${backgroundMenu.x}px;top:${backgroundMenu.y}px;--stack-context-menu-max-height:${contextMenuMaxHeightCss(backgroundMenu)}`}
      bind:element={backgroundMenuElement}
      on:click={(event) => event.stopPropagation()}
      on:contextmenu={(event) => event.stopPropagation()}
      on:keydown={(event) => event.key === 'Escape' && closeMenus()}
    >
      <ContextMenuItem icon="file_copy" disabled={!hasSelection} onClick={() => void copySelected(false)}>Copy</ContextMenuItem>
      <ContextMenuItem icon="content_cut" disabled={!hasSelection} onClick={() => void copySelected(true)}>Cut</ContextMenuItem>
      <ContextMenuItem icon="drive_file_rename" disabled={!selectedEntry} onClick={beginRenameSelected}>Rename</ContextMenuItem>
      <ContextMenuItem icon="delete" destructive disabled={!hasSelection} onClick={() => void deleteSelected()}>Delete</ContextMenuItem>
      <ContextMenuItem icon="folder" disabled={!selectedEntry} onClick={() => void revealSelected()}>Reveal</ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon="create_new_folder" disabled={!currentPath} onClick={beginCreateFolder}>New Folder</ContextMenuItem>
      <ContextMenuItem disabled={!currentPath} onClick={() => void beginCreateTextFile()}>New Text File</ContextMenuItem>
      <ContextMenuItem icon="folder_copy" disabled={!currentPath} onClick={() => void copyTextToClipboard(currentPath, 'Copy folder path unavailable')}>Copy Folder Path</ContextMenuItem>
      <ContextMenuItem icon="code_blocks" disabled={!currentPath} onClick={() => void openCurrentFolderInVscode()}>Open in VS Code</ContextMenuItem>
      <ContextMenuItem icon="terminal" disabled={!currentPath} onClick={() => void openTerminalHere()}>Open Terminal Here</ContextMenuItem>
      <ContextMenuItem icon="settings" disabled={!currentPath} onClick={() => void showCurrentFolderProperties()}>Properties</ContextMenuItem>
    </ContextMenu>
  {/if}

  {#if deleteConfirmation}
    <StackConfirmDialog bind:this={deleteDialog} title={deleteConfirmation.title} message={deleteConfirmation.message} confirmLabel="Delete" tone="danger" initialFocus="cancel" dismissOnBackdrop={false} returnFocus={detailsGrid} onCancel={cancelDeleteConfirmation} onConfirm={() => void confirmDeleteSelection()} />
  {/if}

  {#if pendingEditorExit}
    <StackConfirmDialog bind:this={editorExitDialog} title="Discard unsaved draft?" message="Your changes will be lost." confirmLabel="Discard" tone="danger" initialFocus="cancel" dismissOnBackdrop={false} returnFocus={editorExitFocusOrigin} onCancel={cancelEditorExit} onConfirm={() => void confirmEditorExit()} />
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
