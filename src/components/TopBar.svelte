<script lang="ts">
  import './TopBar.css';
  import { onMount, tick } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { reportShellSurfaceRuntimeMetrics } from '../lib/runtimeMetrics';
  import {
    launchPinnedTaskbarLauncher,
    listPinnedTaskbarLaunchers,
    type PinnedTaskbarLauncher
  } from '../lib/taskbarLaunchers';
  import {
    activateTaskWindow,
    listOpenTaskWindows,
    type TaskbarWindow
  } from '../lib/taskbarWindows';
  import {
    hideSearchPanel,
    launchAppPath,
    openShellPath,
    publishSearchPanel,
    runControlPanel,
    SEARCH_PANEL_ACTIVATE_EVENT,
    SEARCH_PANEL_CLOSED_EVENT,
    SEARCH_PANEL_EXPAND_GROUP_EVENT,
    SEARCH_PANEL_INTERACTION_EVENT,
    SEARCH_PANEL_KEY_EVENT,
    SEARCH_PANEL_PIN_FOLDER_EVENT,
    SEARCH_PANEL_QUERY_EVENT,
    SEARCH_PANEL_SELECT_EVENT,
    readCenteredSearchPanelSize,
    showCenteredSearchPanel,
    showSearchPanel,
    isSearchPanelQueryPayload,
    type SearchPanelPayload,
    type SearchPanelQueryPayload,
    type SearchPanelResult
  } from '../lib/searchPanel';
  import {
    beginStackPopupFocusLossHold,
    endStackPopupFocusLossHold,
    hideStackPopup,
    listStackPins,
    openStackFolderInVscode,
    pinStackFolder,
    reorderStackPins,
    STACK_PINS_UPDATED_EVENT,
    unpinStackFolder,
    showStackPopup,
    type StackPin
  } from '../lib/stackPopup';
  import { folderPathsFromTransfer, hasFolderDragPayload, normalizeDroppedPath } from '../lib/folderDrag';
  import {
    searchEngine,
    SEARCH_ENGINE_PROGRESS_EVENT,
    SEARCH_INDEX_REFRESHED_EVENT,
    isAppSearchIndexRefreshedPayload,
    searchEngineProgressToPanelPayload,
    searchEngineResponseToPanelPayload,
    type SearchEngineResponse,
    type SearchIndexRefreshedPayload,
    type SearchProviderTiming,
    type SearchProgressPayload
  } from '../lib/searchEngine';
  import {
    addShellPreferencesChangeListener,
    formatShellDate,
    formatShellTime,
    getInitialShellPreferences,
    type ShellPreferences
  } from '../lib/shellPreferences';
  import {
    AUDIO_PANEL_CLOSED_EVENT,
    hideAudioPanel,
    showAudioPanel
  } from '../lib/audio';
  import {
    CALENDAR_PANEL_CLOSED_EVENT,
    hideCalendarPanel,
    showCalendarPanel
  } from '../lib/calendarPanel';
  import {
    TRAY_PANEL_CLOSED_EVENT,
    hideTrayPanel,
    showTrayPanel
  } from '../lib/trayPanel';
  import {
    TERMINAL_PANEL_CLOSED_EVENT,
    hideTerminalPanel,
    showTerminalPanel
  } from '../lib/terminalPanel';
  import {
    COMMAND_PANEL_CLOSED_EVENT,
    hideCommandPanel,
    showCommandPanel
  } from '../lib/commandPanel';
  import { showSettingsPanel } from '../lib/settingsPanel';
  import {
    addShellSettingsChangeListener,
    loadShellSettings,
    saveShellBarHeight,
    type ShellSettings
  } from '../lib/settings';
  import {
    clampShellBarHeight,
    createShellBarResizeScheduler,
    resizeShellBar,
    shellBarHeightForSettingsUpdate,
    shellBarHeightFromDrag
  } from '../lib/shellBarResize';
  import type { SearchMode } from '../lib/searchSettings';
  import { showControlPlane } from '../lib/controlPlane';
  import {
    showTopBarPinContextMenu,
    TOP_BAR_PIN_MENU_ACTION_EVENT,
    type TopBarPinMenuActionPayload
  } from '../lib/taskbarMenus';
  import {
    hasTaskbarGroupDragStarted,
    taskbarGroupDragDelta,
    taskbarGroupOrderFromDisplacement,
    taskbarGroupReorderOffset
  } from '../lib/taskbarGroups';
  import { reorderPinnedFolders, stackPinRevealPath, topBarWebviewWindowEventTarget } from '../lib/topBarPins';
  import {
    searchPanelAnchorState,
    searchPanelPayloadSignature,
    shouldPublishSearchPanelPayload,
    shouldShowSearchPanelForAnchor,
    type SearchPanelAnchorState
  } from '../lib/systemSearchState';
  import { terminalActivityGlyph, terminalCompletionGlyph, topBarIdentityState } from '../features/top-bar/topBarUxState';
  import {
    buildVisibleSearchRows,
    createLatestSearchQueryController,
    clearLatestSearchQuery,
    DEFAULT_VISIBLE_GROUP_LIMIT,
    nextProgressiveSearchResultSet,
    nextVisibleRowIndex,
    resolveVisibleSearchRowResultIndex,
    searchModeFromSettings,
    searchPanelKeyboardAction,
    selectedVisibleRowIndex,
    shouldApplySearchEngineResponse,
    shouldRetrySearchAfterProviderCacheWarm,
    shouldRetrySearchFreshness,
    type SearchEngineQueryRequestState,
    type SearchExpandableGroupId,
    type SearchVisibleRowIdentity
  } from '../features/search/searchUxState';
  import MeltActionButton from './melt/MeltActionButton.svelte';

  let now = new Date();
  let shellPreferences: ShellPreferences = getInitialShellPreferences();
  let shellSettings: ShellSettings | null = null;
  let topBarHeightLogical = 23.4;
  let topBarHeightLocked = true;
  let topBarResizePointerId: number | null = null;
  let topBarResizeStartY = 0;
  let topBarResizeStartHeight = 23.4;
  let topBarPendingPersistedHeight: number | null = null;
  const topBarResizeScheduler = createShellBarResizeScheduler('top', (error) => {
    console.error('Failed to resize top bar', error);
  });
  let launchers: PinnedTaskbarLauncher[] = [];
  let openWindows: TaskbarWindow[] = [];
  let lastTaskbarSnapshotSequence = 0;
  let stackPins: StackPin[] = [];
  let searchResults: SearchPanelResult[] = [];
  let searchResultsQuery = '';
  let searchQuery = '';
  let searchInputDraft = '';
  let searchOpen = false;
  let selectedIndex = 0;
  let searchStatus = 'Loading search catalog...';
  let searchControl: HTMLDivElement;
  let searchInput: HTMLInputElement;
  // Pin rail UI
  let pinRailHover = false;
  let pinRailEl: HTMLDivElement | null = null;
  let terminalControl: HTMLDivElement | null = null;
  let commandControl: HTMLDivElement | null = null;
  let trayControl: HTMLDivElement | null = null;
  let soundControl: HTMLDivElement | null = null;
  let timeControl: HTMLDivElement | null = null;
  let showRailScrollLeft = false;
  let showRailScrollRight = false;
  let focusedPinIndex: number | null = null;
  let searchEngineTimer: number | null = null;
  let searchFreshnessRetryTimer: number | null = null;
  let railScrollUpdateTimeout: number | null = null;
  let railScrollButtonsDisposed = false;
  let searchFreshnessRetriedQuery = '';
  let searchProviderCacheRetryTimer: number | null = null;
  let searchProviderCacheRetryQuery = '';
  let searchProviderCacheRetryAttempts = 0;
  let lastAppIndexRefreshGeneration = 0;
  let lastSearchPanelInputSequence = 0;
  const searchQueryController = createLatestSearchQueryController();
  let searchPanelPayloadSequence = 0;
  let searchMode: SearchMode = 'centeredHotkey';
  let searchPresentation: 'anchored' | 'centered' = 'centered';
  let expandedVisibleGroups = new Set<SearchExpandableGroupId>();
  let searchPanelAnchor: SearchPanelAnchorState | null = null;
  let lastSearchPanelPayloadSignature: string | null = null;
  let pinDropStatus = '';
  let pinDropStatusKind: 'info' | 'error' = 'info';
  let pinDropStatusTimer: number | null = null;
  let searchBlurCloseTimer: number | null = null;
  let searchPanelInteractionUntil = 0;
  let draggingPinPath: string | null = null;
  let pinDragPointerId: number | null = null;
  let pinDragStartX = 0;
  let pinDragCurrentX = 0;
  let pinDragStarted = false;
  let pinDragOriginalOrder: string[] = [];
  let pinDragElement: HTMLElement | null = null;
  let pinDragRects: ReturnType<typeof pinReorderRects> = [];
  let stackPinFocusHoldActive = false;
  let stackPinFocusHoldPromise: Promise<void> | null = null;
  let suppressNextPinClickPath: string | null = null;
  let pendingVisiblePinPath: string | null = null;
  let stackPinsLoaded = false;
  let audioOpen = false;
  let trayOpen = false;
  let terminalOpen = false;
  let commandOpen = false;
  let calendarOpen = false;
  let terminalActivityNowMs = Date.now();
  let lastTerminalActivityMs: number | null = null;
  let terminalCompletionPending = false;
  const activeTerminalActivitySessions = new Set<string>();

  const PIN_REORDER_DRAG_THRESHOLD_PX = 4;
  const SEARCH_BLUR_CLOSE_DELAY_MS = 180;
  const SEARCH_PANEL_INTERACTION_GRACE_MS = 350;
  const SEARCH_PROVIDER_CACHE_RETRY_DELAY_MS = 220;
  const SEARCH_PROVIDER_CACHE_RETRY_LIMIT = 8;
  const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT = 'search:toggle-centered';
  const TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT = 'terminal:toggle-panel';
  const TOP_BAR_TERMINAL_ACTIVITY_EVENT = 'terminal-panel:activity';
  const TASKBAR_WINDOWS_SNAPSHOT_EVENT = 'taskbar:windows-snapshot';
  const TERMINAL_PANEL_ID = 'terminal-panel';
  const COMMAND_PANEL_ID = 'command-panel';
  const TRAY_PANEL_ID = 'tray-panel';
  const SOUND_PANEL_ID = 'audio-panel';
  const CALENDAR_PANEL_ID = 'calendar-panel';
  type TopBarTerminalActivityPayload = {
    sessionId?: string;
    active?: boolean;
    completed?: boolean;
  };
  type OpenSearchPanelOptions = {
    publishCurrentPayload?: boolean;
  };

  $: selectedIndex = Math.min(selectedIndex, Math.max(searchResults.length - 1, 0));
  $: visibleRows = buildVisibleSearchRows(searchResults, {
    expandedGroups: expandedVisibleGroups,
    perGroupLimit: DEFAULT_VISIBLE_GROUP_LIMIT
  });
  $: selectedVisibleIndex = selectedVisibleRowIndex(visibleRows, selectedIndex);
  $: selectedVisibleResult = selectedVisibleIndex >= 0 ? visibleRows[selectedVisibleIndex]?.result : undefined;
  $: identityState = topBarIdentityState(stackPins.length, launchers.length, searchStatus);
  $: terminalGlyph = terminalCompletionPending
    ? terminalCompletionGlyph()
    : terminalActivityGlyph(terminalActivityNowMs, lastTerminalActivityMs);
  $: shellTime = formatShellTime(now, shellPreferences);
  $: shellDate = formatShellDate(now, shellPreferences.dateFormat);
  $: pinDragDeltaX = pinDragStarted ? taskbarGroupDragDelta(pinDragStartX, pinDragCurrentX) : 0;
  $: pinPreviewOrder = pinDragStarted && draggingPinPath
    ? taskbarGroupOrderFromDisplacement(
        draggingPinPath,
        pinDragOriginalOrder,
        pinDragRects,
        pinDragDeltaX
      )
    : stackPins.map((pin) => pin.path);

  async function loadSearchCatalog() {
    try {
      [launchers, openWindows] = await Promise.all([
        listPinnedTaskbarLaunchers(),
        listOpenTaskWindows()
      ]);
      searchStatus = 'Type to search apps, settings, and Everything';
    } catch (error) {
      console.error('Failed to load search catalog', error);
      launchers = [];
      openWindows = [];
      searchStatus = 'Everything search ready';
    }
  }

  async function loadSearchMode() {
    try {
      const settings = await loadShellSettings();
      await applyTopBarSettings(settings);
      searchMode = searchModeFromSettings(settings.ui.searchMode);
    } catch (error) {
      console.error('Failed to load search mode', error);
      searchMode = 'centeredHotkey';
    }
  }

  async function applyTopBarSettings(settings: ShellSettings) {
    shellSettings = settings;
    searchMode = searchModeFromSettings(settings.ui.searchMode);
    topBarHeightLocked = settings.ui.lockTopBarHeight;
    const persistedHeight = clampShellBarHeight('top', settings.ui.topBarHeightLogical);
    const pendingHeightSettled = topBarPendingPersistedHeight === persistedHeight;
    topBarHeightLogical = shellBarHeightForSettingsUpdate(
      'top',
      persistedHeight,
      topBarHeightLogical,
      topBarResizePointerId !== null || (topBarPendingPersistedHeight !== null && !pendingHeightSettled)
    );
    if (pendingHeightSettled) {
      topBarPendingPersistedHeight = null;
    }
    await resizeShellBar({ edge: 'top', heightLogical: topBarHeightLogical });
  }

  function startTopBarHeightResize(event: PointerEvent) {
    if (topBarHeightLocked || event.button !== 0) {
      return;
    }
    event.preventDefault();
    topBarResizePointerId = event.pointerId;
    topBarResizeStartY = event.clientY;
    topBarResizeStartHeight = topBarHeightLogical;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function moveTopBarHeightResize(event: PointerEvent) {
    if (topBarResizePointerId !== event.pointerId || topBarHeightLocked) {
      return;
    }
    const nextHeight = shellBarHeightFromDrag(
      'top',
      topBarResizeStartHeight,
      topBarResizeStartY,
      event.clientY
    );
    if (nextHeight === topBarHeightLogical) {
      return;
    }
    topBarHeightLogical = nextHeight;
    topBarResizeScheduler.schedule(nextHeight);
  }

  function finishTopBarHeightResize(event: PointerEvent) {
    if (topBarResizePointerId !== event.pointerId) {
      return;
    }
    topBarResizePointerId = null;
    const nextHeight = clampShellBarHeight('top', topBarHeightLogical);
    void topBarResizeScheduler.flush(nextHeight);
    if (!shellSettings) {
      return;
    }
    topBarPendingPersistedHeight = nextHeight;
    shellSettings = {
      ...shellSettings,
      ui: {
        ...shellSettings.ui,
        topBarHeightLogical: nextHeight
      }
    };
    void saveShellBarHeight('top', nextHeight)
      .then((savedSettings) => {
        shellSettings = savedSettings;
        if (clampShellBarHeight('top', savedSettings.ui.topBarHeightLogical) === topBarPendingPersistedHeight) {
          topBarPendingPersistedHeight = null;
        }
      })
      .catch((error) => {
        topBarPendingPersistedHeight = null;
        console.error('Failed to save top bar height', error);
      });
  }

  async function loadStackPins() {
    try {
      await applyStackPins(await listStackPins(), false);
      stackPinsLoaded = true;
    } catch (error) {
      console.error('Failed to load stack pins', error);
      stackPins = [];
      stackPinsLoaded = true;
    }
  }

  function showPinDropStatus(message: string, kind: 'info' | 'error' = 'info') {
    pinDropStatus = message;
    pinDropStatusKind = kind;
    if (pinDropStatusTimer !== null) {
      window.clearTimeout(pinDropStatusTimer);
    }
    pinDropStatusTimer = window.setTimeout(() => {
      pinDropStatus = '';
      pinDropStatusTimer = null;
    }, 2_800);
  }

  async function pinSearchFolder(folderPath: string) {
    pendingVisiblePinPath = folderPath;
    await applyStackPins(await pinStackFolder(folderPath));
  }

  async function pinAndOpenDroppedFolders(paths: string[], target: EventTarget | null) {
    const normalizedPaths = paths
      .map((path) => normalizeDroppedPath(path))
      .filter((path): path is string => Boolean(path));
    const pinnedPaths: string[] = [];
    for (const path of paths) {
      const normalizedPath = normalizeDroppedPath(path);
      if (!normalizedPath) {
        continue;
      }
      await pinStackFolder(normalizedPath).then(() => {
        pinnedPaths.push(normalizedPath);
      }).catch((error) => {
        console.error(`Failed to pin dropped folder ${path}`, error);
      });
    }
    if (!pinnedPaths.length) {
      showPinDropStatus(
        normalizedPaths.length
          ? 'Drop a folder path, not a file or unavailable location'
          : 'Drop a folder to pin it',
        'error'
      );
      return;
    }
    showPinDropStatus(pinnedPaths.length === 1 ? 'Pinned folder' : `Pinned ${pinnedPaths.length} folders`);
    pendingVisiblePinPath = pinnedPaths[pinnedPaths.length - 1] ?? null;
    await openStackPath(pinnedPaths[0], target);
  }

  async function openPanel(options: OpenSearchPanelOptions = {}) {
    cancelSearchBlurClose();
    searchPresentation = 'anchored';
    const rect = searchControl.getBoundingClientRect();
    const nextAnchor = searchPanelAnchorState(rect);
    const needsNativeShow = shouldShowSearchPanelForAnchor(searchOpen, searchPanelAnchor, nextAnchor);
    searchOpen = true;
    searchPanelAnchor = nextAnchor;
    if (audioOpen) {
      await closeAudioPanel();
    }
    if (trayOpen) {
      await closeTrayPanel();
    }
    if (commandOpen) {
      await closeCommandPanel();
    }
    if (terminalOpen) {
      await closeTerminalPanel();
    }
    if (options.publishCurrentPayload ?? true) {
      queueSearchPanelPublish();
    }
    if (needsNativeShow) {
      await showSearchPanel({
        anchorLeft: rect.left,
        anchorWidth: rect.width
      });
    }
  }

  async function openCenteredPanel(options: OpenSearchPanelOptions = {}) {
    cancelSearchBlurClose();
    const needsNativeShow = !searchOpen || searchPresentation !== 'centered';
    searchPresentation = 'centered';
    searchOpen = true;
    searchPanelAnchor = null;
    if (audioOpen) {
      await closeAudioPanel();
    }
    if (trayOpen) {
      await closeTrayPanel();
    }
    if (commandOpen) {
      await closeCommandPanel();
    }
    if (terminalOpen) {
      await closeTerminalPanel();
    }
    if (options.publishCurrentPayload ?? true) {
      queueSearchPanelPublish();
    }
    if (needsNativeShow) {
      await showCenteredSearchPanel(readCenteredSearchPanelSize()).catch((error) => {
        console.error('Failed to show centered search panel', error);
      });
    }
  }

  function openConfiguredPanel(options: OpenSearchPanelOptions = {}) {
    void (searchMode === 'topRight' ? openPanel(options) : openCenteredPanel(options));
  }

  function toggleCenteredSearchFromHotkey() {
    if (searchOpen) {
      void closePanel();
      return;
    }
    void openCenteredPanel({ publishCurrentPayload: true });
    void tick().then(() => searchInput?.focus({ preventScroll: true }));
  }

  function isCtrlSpaceHotkey(event: KeyboardEvent) {
    return event.code === 'Space' && event.ctrlKey && !event.altKey && !event.metaKey;
  }

  function isSpaceKey(event: KeyboardEvent) {
    return event.code === 'Space';
  }

  function handleSearchFocus() {
    openConfiguredPanel();
  }

  async function closePanel() {
    resetActiveSearchState();
    searchOpen = false;
    searchPanelAnchor = null;
    lastSearchPanelPayloadSignature = null;
    searchInput?.blur();
    await publishSearchPanel({
      query: '',
      results: [],
      selectedIndex: 0,
      statusMessage: 'Search is ready',
      sequence: ++searchPanelPayloadSequence
    }).catch(() => undefined);
    await hideSearchPanel().catch((error) => {
      console.error('Failed to hide search panel', error);
    });
  }

  function cancelSearchBlurClose() {
    if (searchBlurCloseTimer !== null) {
      window.clearTimeout(searchBlurCloseTimer);
      searchBlurCloseTimer = null;
    }
  }

  function cancelSearchEngineTimer() {
    if (searchEngineTimer !== null) {
      window.clearTimeout(searchEngineTimer);
      searchEngineTimer = null;
    }
  }

  function cancelSearchFreshnessRetry() {
    if (searchFreshnessRetryTimer !== null) {
      window.clearTimeout(searchFreshnessRetryTimer);
      searchFreshnessRetryTimer = null;
    }
  }

  function cancelSearchProviderCacheRetry() {
    if (searchProviderCacheRetryTimer !== null) {
      window.clearTimeout(searchProviderCacheRetryTimer);
      searchProviderCacheRetryTimer = null;
    }
  }

  function resetSearchProviderCacheRetry() {
    cancelSearchProviderCacheRetry();
    searchProviderCacheRetryQuery = '';
    searchProviderCacheRetryAttempts = 0;
  }

  function cleanupSearchWorkAfterClose() {
    cancelSearchBlurClose();
    cancelSearchEngineTimer();
    cancelSearchFreshnessRetry();
    resetSearchProviderCacheRetry();
    invalidateSearchEngineResponses();
  }

  function resetActiveSearchState() {
    cleanupSearchWorkAfterClose();
    // cleanupSearchWorkAfterClose() calls invalidateSearchEngineResponses().
    searchQuery = '';
    searchInputDraft = '';
    searchResults = [];
    searchResultsQuery = '';
    selectedIndex = 0;
    searchStatus = 'Search is ready';
    expandedVisibleGroups = new Set<SearchExpandableGroupId>();
  }

  function invalidateSearchEngineResponses() {
    clearLatestSearchQuery(searchQueryController);
  }

  function markSearchPanelInteraction() {
    searchPanelInteractionUntil = Date.now() + SEARCH_PANEL_INTERACTION_GRACE_MS;
  }

  function scheduleSearchBlurClose() {
    cancelSearchBlurClose();
    searchBlurCloseTimer = window.setTimeout(() => {
      searchBlurCloseTimer = null;
      if (Date.now() < searchPanelInteractionUntil) {
        return;
      }
      void closePanel();
    }, SEARCH_BLUR_CLOSE_DELAY_MS);
  }

  function handleTopBarPointerDown(event: MouseEvent) {
    const target = event.target instanceof Node ? event.target : null;
    if (!isPinnedFolderPointerTarget(target)) {
      void hideStackPopup().catch((error) => {
        console.error('Failed to hide stack popup after top bar pointer press', error);
      });
    }
    if (commandOpen && (!target || !commandControl?.contains(target))) {
      void closeCommandPanel();
    }
    if (terminalOpen && (!target || !terminalControl?.contains(target))) {
      void closeTerminalPanel();
    }
    if (audioOpen && (!target || !soundControl?.contains(target))) {
      void closeAudioPanel();
    }
    if (trayOpen && (!target || !trayControl?.contains(target))) {
      void closeTrayPanel();
    }
    if (!searchOpen || !searchControl) {
      return;
    }
    if (target && searchControl.contains(target)) {
      return;
    }
    void closePanel();
  }

  function isPinnedFolderPointerTarget(target: Node | null) {
    const element = target instanceof Element ? target : target?.parentElement ?? null;
    const pinnedFolderButton = element?.closest('button[data-path]');
    return !!pinnedFolderButton && !!pinRailEl?.contains(pinnedFolderButton);
  }

  function isAltBackquoteHotkey(event: KeyboardEvent) {
    return event.altKey && !event.ctrlKey && !event.metaKey && (event.key === '`' || event.code === 'Backquote');
  }

  function handleTopBarKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && pinDragPointerId !== null) {
      event.preventDefault();
      cancelPinPointerDrag();
    }
  }

  async function openSettingsPanel(target: EventTarget | null) {
    const button = target instanceof HTMLElement ? target : null;
    const rect = button?.getBoundingClientRect();
    if (!rect) {
      return;
    }
    await closePanel();
    await showSettingsPanel({
      anchorLeft: rect.left,
      anchorWidth: rect.width
    }).catch((error) => {
      console.error('Failed to show settings panel', error);
    });
  }

  async function closeAudioPanel() {
    audioOpen = false;
    await hideAudioPanel().catch((error) => {
      console.error('Failed to hide audio panel', error);
    });
  }

  async function closeCommandPanel() {
    commandOpen = false;
    await hideCommandPanel().catch((error) => {
      console.error('Failed to hide command panel', error);
    });
  }

  async function closeTerminalPanel() {
    terminalOpen = false;
    await hideTerminalPanel().catch((error) => {
      console.error('Failed to hide terminal panel', error);
    });
  }

  async function closeTrayPanel() {
    trayOpen = false;
    await hideTrayPanel().catch((error) => {
      console.error('Failed to hide tray panel', error);
    });
  }

  async function closeCalendarPanel() {
    calendarOpen = false;
    await hideCalendarPanel().catch((error) => {
      console.error('Failed to hide calendar panel', error);
    });
  }

  async function toggleSoundPanel(target: EventTarget | null) {
    if (audioOpen) {
      await closeAudioPanel();
      return;
    }
    const button = target instanceof HTMLElement ? target : null;
    const rect = button?.getBoundingClientRect();
    if (!rect) {
      return;
    }
    if (trayOpen) {
      await closeTrayPanel();
    }
    if (commandOpen) {
      await closeCommandPanel();
    }
    if (calendarOpen) {
      await closeCalendarPanel();
    }
    await closePanel();
    audioOpen = true;
    await showAudioPanel({
      anchorLeft: rect.left,
      anchorWidth: rect.width
    }).catch((error) => {
      audioOpen = false;
      console.error('Failed to show audio panel', error);
    });
  }

  async function toggleTrayPanel(target: EventTarget | null) {
    if (trayOpen) {
      await closeTrayPanel();
      return;
    }
    const button = target instanceof HTMLElement ? target : null;
    const rect = button?.getBoundingClientRect();
    if (!rect) {
      return;
    }
    await closePanel();
    await closeAudioPanel();
    await closeCommandPanel();
    await closeTerminalPanel();
    await closeCalendarPanel();
    trayOpen = true;
    await showTrayPanel({
      anchorLeft: rect.left,
      anchorWidth: rect.width
    }).catch((error) => {
      trayOpen = false;
      console.error('Failed to show tray panel', error);
    });
  }

  async function toggleCommandPanel(target: EventTarget | null) {
    if (commandOpen) {
      await closeCommandPanel();
      return;
    }
    const button = target instanceof HTMLElement ? target : null;
    const rect = button?.getBoundingClientRect();
    if (!rect) {
      return;
    }
    await closePanel();
    await closeAudioPanel();
    await closeTrayPanel();
    await closeTerminalPanel();
    await closeCalendarPanel();
    commandOpen = true;
    await showCommandPanel({
      anchorLeft: rect.left,
      anchorWidth: rect.width
    }).catch((error) => {
      commandOpen = false;
      console.error('Failed to show command panel', error);
    });
  }

  async function toggleTerminalPanel(target: EventTarget | null) {
    clearTerminalCompletionNotification();
    if (terminalOpen) {
      await closeTerminalPanel();
      return;
    }
    const button = target instanceof HTMLElement ? target : null;
    const rect = button?.getBoundingClientRect();
    if (!rect) {
      return;
    }
    await closePanel();
    await closeAudioPanel();
    await closeTrayPanel();
    await closeCommandPanel();
    await closeCalendarPanel();
    terminalOpen = true;
    await showTerminalPanel({
      anchorLeft: rect.left,
      anchorWidth: rect.width
    }).catch((error) => {
      terminalOpen = false;
      console.error('Failed to show terminal panel', error);
    });
  }

  function clearTerminalCompletionNotification() {
    terminalCompletionPending = false;
  }

  function playTerminalCompletionSound() {
    try {
      const AudioContextCtor = window.AudioContext ?? (window as Window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
      if (!AudioContextCtor) return;
      const context = new AudioContextCtor();
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      oscillator.type = 'sine';
      oscillator.frequency.value = 880;
      gain.gain.setValueAtTime(0.0001, context.currentTime);
      gain.gain.exponentialRampToValueAtTime(0.08, context.currentTime + 0.01);
      gain.gain.exponentialRampToValueAtTime(0.0001, context.currentTime + 0.18);
      oscillator.connect(gain);
      gain.connect(context.destination);
      oscillator.start();
      oscillator.stop(context.currentTime + 0.2);
      window.setTimeout(() => void context.close().catch(() => undefined), 260);
    } catch (error) {
      console.warn('Failed to play terminal completion sound', error);
    }
  }

  async function toggleCalendarPanel(target: EventTarget | null) {
    if (calendarOpen) {
      await closeCalendarPanel();
      return;
    }
    const button = target instanceof HTMLElement ? target : null;
    const rect = button?.getBoundingClientRect();
    if (!rect) {
      return;
    }
    await closePanel();
    await closeAudioPanel();
    await closeTrayPanel();
    await closeCommandPanel();
    await closeTerminalPanel();
    calendarOpen = true;
    await showCalendarPanel({
      anchorLeft: rect.left,
      anchorWidth: rect.width
    }).catch((error) => {
      calendarOpen = false;
      console.error('Failed to show calendar panel', error);
    });
  }

  async function openStackFromPin(pin: StackPin, target: EventTarget | null) {
    await openStackPath(pin.path, target);
  }

  function handlePinClick(event: MouseEvent, pin: StackPin, index: number) {
    event.preventDefault();
    event.stopPropagation();
    if (suppressNextPinClickPath === pin.path) {
      suppressNextPinClickPath = null;
      event.preventDefault();
      return;
    }
    focusedPinIndex = index;
    void openStackPath(pin.path, event.currentTarget);
  }

  async function openStackPath(path: string, target: EventTarget | null) {
    const button = target as HTMLElement | null;
    const rect = button?.getBoundingClientRect() ?? searchControl?.getBoundingClientRect();
    if (!rect) {
      return;
    }

    await closePanel();
    await showStackPopup({
      anchorLeft: rect.left,
      anchorWidth: rect.width,
      folderPath: path
    }).catch((error) => {
      console.error('Failed to show stack popup', error);
    });
  }

  function handlePinRailDragOver(event: DragEvent) {
    if (!draggingPinPath && !hasFolderDragPayload(event.dataTransfer)) {
      return;
    }
    event.preventDefault();
    pinRailHover = !draggingPinPath;
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = draggingPinPath ? 'move' : 'copy';
    }
  }

  function handlePinRailDrop(event: DragEvent) {
    if (draggingPinPath) {
      draggingPinPath = null;
      pinRailHover = false;
      return;
    }
    if (!hasFolderDragPayload(event.dataTransfer)) {
      pinRailHover = false;
      return;
    }

    event.preventDefault();
    const dt = event.dataTransfer;
    pinRailHover = false;
    void pinAndOpenDroppedFolders(folderPathsFromTransfer(dt), event.currentTarget);
  }

  function handlePinRailDragLeave() {
    pinRailHover = false;
  }

  function pinReorderRects() {
    return Array.from(pinRailEl?.querySelectorAll<HTMLElement>('button[data-path]') ?? [])
      .map((element) => {
        const rect = element.getBoundingClientRect();
        return {
          key: element.dataset.path ?? '',
          left: rect.left,
          width: rect.width
        };
      })
      .filter((rect) => rect.key);
  }

  function pinStyle(pin: StackPin) {
    const previewOrderIndex = pinPreviewOrder.indexOf(pin.path);
    if (draggingPinPath !== pin.path || !pinDragStarted) {
      return previewOrderIndex >= 0 ? `order: ${previewOrderIndex};` : '';
    }
    const liveReorderOffset = taskbarGroupReorderOffset(
      pin.path,
      pinPreviewOrder,
      pinDragRects
    );
    const visualDelta = pinDragDeltaX + liveReorderOffset;
    return `order: ${previewOrderIndex}; transform: translate3d(${visualDelta}px, -1px, 0); z-index: 2;`;
  }

  function releasePinPointerCapture() {
    if (!pinDragElement || pinDragPointerId === null) {
      return;
    }
    try {
      if (pinDragElement.hasPointerCapture(pinDragPointerId)) {
        pinDragElement.releasePointerCapture(pinDragPointerId);
      }
    } catch {
      // Pointer capture can already be gone if the OS/browser canceled the pointer stream.
    }
  }

  function resetPinPointerDrag() {
    draggingPinPath = null;
    pinDragPointerId = null;
    pinDragStartX = 0;
    pinDragCurrentX = 0;
    pinDragStarted = false;
    pinDragOriginalOrder = [];
    pinDragElement = null;
    pinDragRects = [];
    pinRailHover = false;
  }

  function cancelPinPointerDrag() {
    releaseStackPinFocusHold();
    releasePinPointerCapture();
    resetPinPointerDrag();
  }

  function startPinPointerDrag(pin: StackPin, event: PointerEvent) {
    if (event.button !== 0) {
      return;
    }
    event.preventDefault();
    beginStackPinFocusHold();
    draggingPinPath = pin.path;
    pinDragPointerId = event.pointerId;
    pinDragStartX = event.clientX;
    pinDragCurrentX = event.clientX;
    pinDragStarted = false;
    pinDragOriginalOrder = stackPins.map((item) => item.path);
    pinDragElement = event.currentTarget as HTMLElement;
    pinDragRects = pinReorderRects();
    try {
      pinDragElement.setPointerCapture(event.pointerId);
    } catch {
      // Reorder still works while pointer remains over the pin rail.
    }
  }

  function movePinPointerDrag(event: PointerEvent) {
    if (pinDragPointerId !== event.pointerId || !draggingPinPath) {
      return;
    }
    pinDragCurrentX = event.clientX;
    const started = hasTaskbarGroupDragStarted(pinDragStartX, event.clientX, PIN_REORDER_DRAG_THRESHOLD_PX);
    if (!pinDragStarted && started) {
      pinDragStarted = true;
    }
    if (pinDragStarted) {
      event.preventDefault();
    }
  }

  async function persistPinReorder(sourcePath: string, targetIndex: number) {
    const nextPins = reorderPinnedFolders(stackPins, sourcePath, targetIndex);
    if (nextPins === stackPins) {
      return;
    }
    stackPins = nextPins;
    try {
      await applyStackPins(await reorderStackPins(nextPins.map((pin) => pin.path)));
    } catch (error) {
      console.error('Failed to reorder stack pins', error);
      await loadStackPins();
    }
  }

  function finishPinPointerDrag(event: PointerEvent) {
    if (pinDragPointerId !== event.pointerId) {
      return;
    }
    const sourcePath = draggingPinPath;
    const didDrag = pinDragStarted;
    let targetIndex = -1;
    if (didDrag && sourcePath) {
      const nextOrder = taskbarGroupOrderFromDisplacement(
        sourcePath,
        pinDragOriginalOrder,
        pinDragRects,
        taskbarGroupDragDelta(pinDragStartX, event.clientX)
      );
      targetIndex = nextOrder.indexOf(sourcePath);
    }
    releasePinPointerCapture();
    resetPinPointerDrag();
    if (didDrag && sourcePath) {
      suppressNextPinClickPath = sourcePath;
      releaseStackPinFocusHold();
      if (targetIndex >= 0) {
        void persistPinReorder(sourcePath, targetIndex);
      }
      return;
    }
    if (sourcePath) {
      suppressNextPinClickPath = sourcePath;
      const sourceIndex = stackPins.findIndex((pin) => pin.path === sourcePath);
      focusedPinIndex = sourceIndex >= 0 ? sourceIndex : null;
      void openStackPath(sourcePath, event.currentTarget).finally(() => {
        releaseStackPinFocusHold();
      });
      return;
    }
    releaseStackPinFocusHold();
  }

  function beginStackPinFocusHold() {
    if (stackPinFocusHoldActive) {
      return;
    }
    stackPinFocusHoldActive = true;
    stackPinFocusHoldPromise = beginStackPopupFocusLossHold().catch((error) => {
      stackPinFocusHoldActive = false;
      stackPinFocusHoldPromise = null;
      console.error('Failed to hold stack popup focus while pressing pinned folder', error);
    });
  }

  function releaseStackPinFocusHold() {
    if (!stackPinFocusHoldActive) {
      return;
    }
    stackPinFocusHoldActive = false;
    const pendingHold = stackPinFocusHoldPromise;
    stackPinFocusHoldPromise = null;
    void (pendingHold ?? Promise.resolve())
      .then(() => endStackPopupFocusLossHold())
      .catch((error) => {
        console.error('Failed to release stack popup focus hold after pinned folder press', error);
      });
  }

  function updateRailScrollButtons() {
    if (railScrollButtonsDisposed) {
      return;
    }
    if (!pinRailEl) {
      showRailScrollLeft = false;
      showRailScrollRight = false;
      return;
    }
    const el = pinRailEl;
    showRailScrollLeft = el.scrollLeft > 2;
    showRailScrollRight = el.scrollWidth - el.clientWidth - el.scrollLeft > 2;
  }

  function scrollRailBy(delta: number) {
    if (!pinRailEl) {
      return;
    }
    pinRailEl.scrollBy({ left: delta, behavior: 'smooth' });
    scheduleRailScrollButtonUpdate();
  }

  function scheduleRailScrollButtonUpdate() {
    if (railScrollUpdateTimeout !== null) {
      window.clearTimeout(railScrollUpdateTimeout);
    }
    railScrollUpdateTimeout = window.setTimeout(() => {
      railScrollUpdateTimeout = null;
      updateRailScrollButtons();
    }, 160);
  }

  function cancelRailScrollButtonUpdate() {
    if (railScrollUpdateTimeout !== null) {
      window.clearTimeout(railScrollUpdateTimeout);
      railScrollUpdateTimeout = null;
    }
  }

  function scrollRailLeft() {
    if (!pinRailEl) return;
    scrollRailBy(-Math.max(120, Math.floor(pinRailEl.clientWidth * 0.6)));
  }

  function scrollRailRight() {
    if (!pinRailEl) return;
    scrollRailBy(Math.max(120, Math.floor(pinRailEl.clientWidth * 0.6)));
  }

  function handleRailWheel(event: WheelEvent) {
    if (!pinRailEl) return;
    if (Math.abs(event.deltaX) > Math.abs(event.deltaY)) {
      // let horizontal scroll happen
      return;
    }
    // translate vertical wheel to horizontal scroll
    event.preventDefault();
    pinRailEl.scrollLeft += event.deltaY;
    updateRailScrollButtons();
  }

  function handlePinRailKeydown(event: KeyboardEvent) {
    if (!stackPins || !stackPins.length) return;
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      const next = focusedPinIndex === null ? 0 : Math.min(stackPins.length - 1, focusedPinIndex + 1);
      focusedPinIndex = next;
      const btn = pinRailEl?.querySelectorAll('button')[next] as HTMLElement | undefined;
      btn?.focus();
      updateRailScrollButtons();
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      const prev = focusedPinIndex === null ? stackPins.length - 1 : Math.max(0, focusedPinIndex - 1);
      focusedPinIndex = prev;
      const btn = pinRailEl?.querySelectorAll('button')[prev] as HTMLElement | undefined;
      btn?.focus();
      updateRailScrollButtons();
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      const idx = focusedPinIndex ?? 0;
      const pin = stackPins[idx];
      if (pin) {
        void openStackFromPin(pin, pinRailEl?.querySelectorAll('button')[idx] ?? null);
      }
    }
  }

  function queryPinButton(path: string) {
    if (!pinRailEl) {
      return null;
    }

    return Array.from(pinRailEl.querySelectorAll<HTMLElement>('button[data-path]'))
      .find((button) => button.dataset.path === path) ?? null;
  }

  async function revealPin(path: string) {
    await tick();
    window.requestAnimationFrame(() => {
      queryPinButton(path)?.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' });
      updateRailScrollButtons();
    });
  }

  async function applyStackPins(nextPins: StackPin[], allowDetectedAdd = stackPinsLoaded) {
    const addedPinPath = stackPinRevealPath(stackPins, nextPins, pendingVisiblePinPath, allowDetectedAdd);
    stackPins = nextPins;
    pendingVisiblePinPath = null;
    if (addedPinPath) {
      await revealPin(addedPinPath);
      return;
    }
    await tick();
    updateRailScrollButtons();
  }

  function handlePinContextMenu(event: MouseEvent, pin: StackPin) {
    event.preventDefault();
    event.stopPropagation();
    void showTopBarPinContextMenu({
      path: pin.path,
      x: event.clientX,
      y: event.clientY
    }).catch((error) => {
      console.error('Failed to show top-bar pin context menu', error);
    });
  }

  async function unpinFromMenu(path: string) {
    await unpinStackFolder(path).catch((error) => {
      console.error('Failed to unpin folder', error);
    });
  }

  function currentSearchPanelPayload(): SearchPanelPayload {
    return {
      query: searchQuery,
      results: searchResults,
      selectedIndex,
      statusMessage: searchStatus,
      presentation: searchPresentation
    };
  }

  function queueSearchPanelPublish(payload: SearchPanelPayload = currentSearchPanelPayload()) {
    const signature = searchPanelPayloadSignature(payload);
    if (!shouldPublishSearchPanelPayload(lastSearchPanelPayloadSignature, payload)) {
      return;
    }
    lastSearchPanelPayloadSignature = signature;
    const sequencedPayload =
      payload.sequence === undefined
        ? {
            ...payload,
            sequence: ++searchPanelPayloadSequence
          }
        : {
            ...payload,
            sequence: payload.sequence
          };
    searchPanelPayloadSequence = Math.max(searchPanelPayloadSequence, sequencedPayload.sequence ?? 0);
    void publishSearchPanel(sequencedPayload).catch((error) => {
      console.error('Failed to update search panel', error);
      lastSearchPanelPayloadSignature = null;
    });
  }

  function publishPendingSearchPayload(sequence: number, results: SearchPanelResult[] = searchResults) {
    queueSearchPanelPublish({
      query: searchQuery.trim(),
      results,
      selectedIndex,
      statusMessage: searchStatus,
      presentation: searchPresentation,
      phase: 'typing',
      sequence
    });
  }

  function scheduleSearchEngine(query: string, existingRequest?: { query: string; sequence: number } | null) {
    cancelSearchEngineTimer();

    if (!query.trim()) {
      searchResults = [];
      searchResultsQuery = '';
      searchStatus = 'Search is ready';
      queueSearchPanelPublish();
      return null;
    }

    const request = existingRequest ?? searchQueryController.next(query);
    searchStatus = 'Searching...';
    searchEngineTimer = window.setTimeout(() => {
      searchEngineTimer = null;
      void loadSearchEngineResults(request);
    }, 0);
    return request;
  }

  function scheduleSearchFreshnessRetry(request: { query: string; sequence: number }) {
    cancelSearchFreshnessRetry();
    searchFreshnessRetryTimer = window.setTimeout(() => {
      searchFreshnessRetryTimer = null;
      if (
        searchFreshnessRetriedQuery === request.query ||
        !shouldRetrySearchFreshness(
          searchQuery,
          searchResultsQuery,
          searchStatus,
          request.sequence,
          searchQueryController.currentSequence()
        )
      ) {
        return;
      }
      searchFreshnessRetriedQuery = request.query;
      scheduleSearchEngine(searchQuery);
    }, 180);
  }

  function scheduleSearchProviderCacheRetry(
    request: { query: string; sequence: number },
    providerTimings: SearchProviderTiming[] | undefined
  ) {
    const shouldRetry = shouldRetrySearchAfterProviderCacheWarm(
      request,
      searchQuery,
      searchQueryController.currentSequence(),
      providerTimings ?? []
    );
    if (!shouldRetry) {
      if (request.query.trim() === searchProviderCacheRetryQuery) {
        resetSearchProviderCacheRetry();
      }
      return;
    }

    const normalizedQuery = request.query.trim();
    if (searchProviderCacheRetryQuery !== normalizedQuery) {
      searchProviderCacheRetryQuery = normalizedQuery;
      searchProviderCacheRetryAttempts = 0;
    }
    if (
      searchProviderCacheRetryAttempts >= SEARCH_PROVIDER_CACHE_RETRY_LIMIT ||
      searchProviderCacheRetryTimer !== null
    ) {
      return;
    }

    searchProviderCacheRetryAttempts += 1;
    searchProviderCacheRetryTimer = window.setTimeout(() => {
      searchProviderCacheRetryTimer = null;
      if (
        !shouldApplySearchEngineResponse(
          request,
          searchQuery,
          searchQueryController.currentSequence()
        )
      ) {
        return;
      }
      scheduleSearchEngine(searchQuery);
    }, SEARCH_PROVIDER_CACHE_RETRY_DELAY_MS);
  }

  function applySearchEngineProgress(payload: SearchProgressPayload) {
    if (!searchQueryController.shouldApply({ query: payload.query, sequence: payload.sequence }, searchQuery)) {
      return;
    }
    const nextPayload = searchEngineProgressToPanelPayload(payload, selectedIndex, searchPresentation);
    const selectedIndexBeforeUpdate = selectedIndex;
    const currentStableKey = searchResults[selectedIndex]?.recordKey ?? searchResults[selectedIndex]?.id ?? null;
    const nextResultSet = nextProgressiveSearchResultSet(
      { query: searchResultsQuery, results: searchResults },
      { query: payload.query, phase: payload.phase, results: nextPayload.results }
    );
    const replacedResultSet = nextResultSet.query !== searchResultsQuery;
    searchResults = nextResultSet.results;
    searchResultsQuery = nextResultSet.query;
    if (searchResults.length && (replacedResultSet || selectedIndexBeforeUpdate <= 0 || !currentStableKey)) {
      selectedIndex = Math.min(nextPayload.selectedIndex, searchResults.length - 1);
    } else {
      selectedIndex = currentStableKey
        ? Math.max(0, searchResults.findIndex((result) => (result.recordKey ?? result.id) === currentStableKey))
        : Math.min(selectedIndex, Math.max(searchResults.length - 1, 0));
    }
    searchStatus = nextPayload.statusMessage;
    queueSearchPanelPublish({
      ...nextPayload,
      results: searchResults,
      selectedIndex
    });
    scheduleSearchProviderCacheRetry(
      { query: payload.query, sequence: payload.sequence },
      payload.providerTimings
    );
  }

  async function loadSearchEngineResults(request: { query: string; sequence: number }) {
    try {
      const response: SearchEngineResponse = await searchEngine({
        query: request.query,
        sequence: request.sequence,
        limit: 50,
        presentation: searchPresentation,
        context: {
          openWindows: openWindows.map((window) => ({
            id: String(window.hwnd),
            title: window.title,
            appName: window.processName,
            iconDataUrl: window.iconDataUrl
          }))
        }
      });
      if (!shouldApplySearchEngineResponse(
        { query: response.query, sequence: response.sequence },
        searchQuery,
        searchQueryController.currentSequence()
      )) {
        return;
      }

      const payload = searchEngineResponseToPanelPayload(response, selectedIndex, searchPresentation);
      searchResults = payload.results;
      searchResultsQuery = response.query;
      selectedIndex = payload.selectedIndex;
      searchStatus = payload.statusMessage;
      queueSearchPanelPublish({
        ...payload,
        phase: 'complete',
        sequence: response.sequence
      });
      scheduleSearchProviderCacheRetry(
        { query: response.query, sequence: response.sequence },
        response.providerTimings
      );
    } catch (error) {
      if (!searchQueryController.shouldApply(request, searchQuery)) {
        return;
      }
      console.error('Failed to search apps, settings, and files', error);
      searchStatus = 'Search unavailable';
      queueSearchPanelPublish({
        ...currentSearchPanelPayload(),
        phase: 'error',
        sequence: request.sequence
      });
    }
  }

  async function activateResult(result: SearchPanelResult | undefined) {
    if (!result) {
      return;
    }

    if (result.actionId === 'runControlPanel') {
      await runControlPanel(result.actionArgs);
    } else if (result.path && (result.providerId === 'everything' || result.providerId === 'local')) {
      await openShellPath(result.path as string);
    } else if (result.kind === 'app') {
      if (result.path) {
        await launchAppPath(result.path);
      } else {
        await launchPinnedTaskbarLauncher(result.id.replace('app:', ''));
      }
    } else if (result.kind === 'window') {
      const taskWindow = openWindows.find((item) => result.id === `window:${item.hwnd}`);
      if (taskWindow) {
        await activateTaskWindow(taskWindow.hwnd);
      }
    } else if (result.kind === 'folder' || result.kind === 'file') {
      await openShellPath(result.path ?? result.id.replace('folder:', ''));
    } else if (result.kind === 'setting' && result.path) {
      await openShellPath(result.path);
    } else if (result.id === 'command:refresh-search') {
      await loadSearchCatalog();
      scheduleSearchEngine(searchQuery);
    } else if (result.id === 'command:open-control-plane') {
      await showControlPlane();
    } else if (result.id === 'command:hide-search') {
      await closePanel();
      return;
    }

    await closePanel();
    await loadSearchCatalog();
  }

  function updateSearchInputDraft(nextQuery: string) {
    searchInputDraft = nextQuery;
  }

  function cancelSupersededSearchWorkForDraft(nextQuery: string) {
    if (nextQuery === searchQuery) {
      return;
    }
    cancelSearchEngineTimer();
    cancelSearchFreshnessRetry();
    cancelSearchProviderCacheRetry();
    invalidateSearchEngineResponses();
  }

  function publishImmediateSearchInputState(nextQuery: string): SearchEngineQueryRequestState {
    const previousNormalizedQuery = searchQuery.trim();
    if (nextQuery !== searchQuery) {
      cancelSupersededSearchWorkForDraft(nextQuery);
      expandedVisibleGroups = new Set<SearchExpandableGroupId>();
    }
    const request = searchQueryController.next(nextQuery);
    const normalizedChanged = request.query !== previousNormalizedQuery;
    searchQuery = nextQuery;
    updateSearchInputDraft(nextQuery);
    if (normalizedChanged) {
      searchFreshnessRetriedQuery = '';
      resetSearchProviderCacheRetry();
    }
    selectedIndex = 0;
    searchStatus = request.query ? 'Searching...' : 'Search is ready';
    if (!request.query || normalizedChanged) {
      searchResults = [];
      searchResultsQuery = '';
    }
    openConfiguredPanel({ publishCurrentPayload: false });
    queueSearchPanelPublish({
      query: request.query,
      results: request.query && !normalizedChanged ? searchResults : [],
      selectedIndex,
      statusMessage: searchStatus,
      presentation: searchPresentation,
      phase: request.query ? 'typing' : 'complete',
      sequence: request.sequence
    });
    return request;
  }

  function startImmediateSearchQueryExecution(request: SearchEngineQueryRequestState) {
    if (!shouldApplySearchEngineResponse(request, searchQuery, searchQueryController.currentSequence())) {
      return;
    }
    if (!request.query) {
      cancelSearchFreshnessRetry();
      resetSearchProviderCacheRetry();
      return;
    }
    publishPendingSearchPayload(request.sequence, searchResults);
    scheduleSearchFreshnessRetry(request);
    void loadSearchEngineResults(request);
  }

  function applySearchQuery(nextQuery: string) {
    const request = publishImmediateSearchInputState(nextQuery);
    startImmediateSearchQueryExecution(request);
  }

  function hasSearchClearValue() {
    return Boolean(searchInputDraft || searchQuery);
  }

  async function clearSearch() {
    const request = publishImmediateSearchInputState('');
    startImmediateSearchQueryExecution(request);
    await tick();
    searchInput?.focus({ preventScroll: true });
  }

  function handleSearchInput(event: Event) {
    const nextQuery = (event.currentTarget as HTMLInputElement).value;
    const request = publishImmediateSearchInputState(nextQuery);
    startImmediateSearchQueryExecution(request);
  }

  function applySearchKeyboardAction(key: string): boolean {
    const action = searchPanelKeyboardAction(key);
    if (action === 'selectNext') {
      const nextIndex = nextVisibleRowIndex(visibleRows, selectedVisibleIndex, 1);
      if (nextIndex >= 0) {
        selectedIndex = visibleRows[nextIndex]?.resultIndex ?? selectedIndex;
      }
      queueSearchPanelPublish();
    } else if (action === 'selectPrevious') {
      const nextIndex = nextVisibleRowIndex(visibleRows, selectedVisibleIndex, -1);
      if (nextIndex >= 0) {
        selectedIndex = visibleRows[nextIndex]?.resultIndex ?? selectedIndex;
      }
      queueSearchPanelPublish();
    } else if (action === 'activate') {
      void activateResult(selectedVisibleResult);
    } else if (action === 'close') {
      void closePanel();
    } else {
      return false;
    }
    return true;
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (applySearchKeyboardAction(event.key)) {
      event.preventDefault();
    }
  }

  function handleSearchPointerDown() {
    if (!searchOpen) {
      openConfiguredPanel();
    }
  }

  function selectSearchResult(resultId: string) {
    const index = resolveVisibleSearchRowResultIndex(searchResults, resultId);
    if (index >= 0) {
      if (selectedIndex !== index) {
        selectedIndex = index;
        queueSearchPanelPublish();
      }
    }
  }

  function selectSearchVisibleRow(identity: SearchVisibleRowIdentity | string) {
    const index = resolveVisibleSearchRowResultIndex(searchResults, identity);
    if (index >= 0) {
      if (selectedIndex !== index) {
        selectedIndex = index;
        queueSearchPanelPublish();
      }
    }
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    railScrollButtonsDisposed = false;
    const registerAsyncUnlistener = (registration: Promise<() => void>) => {
      void registration.then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlisteners.push(unlisten);
      });
    };
    const timer = window.setInterval(() => {
      now = new Date();
    }, 1_000);
    const terminalActivityTimer = window.setInterval(() => {
      terminalActivityNowMs = Date.now();
    }, 450);
    const runtimeMetricsTimer = window.setTimeout(() => {
      void reportShellSurfaceRuntimeMetrics('top-bar').catch((error) => {
        console.error('Top bar runtime metrics failed', error);
      });
    }, 250);
    void loadSearchCatalog();
    void loadSearchMode();
    void loadStackPins();
    registerAsyncUnlistener(listen<SearchVisibleRowIdentity | string>(SEARCH_PANEL_ACTIVATE_EVENT, (event) => {
      markSearchPanelInteraction();
      const index = resolveVisibleSearchRowResultIndex(searchResults, event.payload);
      void activateResult(index >= 0 ? searchResults[index] : undefined);
    }));
    registerAsyncUnlistener(listen<SearchVisibleRowIdentity | string>(SEARCH_PANEL_SELECT_EVENT, (event) => {
      markSearchPanelInteraction();
      selectSearchVisibleRow(event.payload);
    }));
    registerAsyncUnlistener(listen<SearchExpandableGroupId>(SEARCH_PANEL_EXPAND_GROUP_EVENT, (event) => {
      markSearchPanelInteraction();
      expandedVisibleGroups = new Set([...expandedVisibleGroups, event.payload]);
      queueSearchPanelPublish();
    }));
    registerAsyncUnlistener(listen(SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT, () => {
      toggleCenteredSearchFromHotkey();
    }));
    registerAsyncUnlistener(listen<{ sequence: number; windows: TaskbarWindow[] }>(TASKBAR_WINDOWS_SNAPSHOT_EVENT, (event) => {
      if (event.payload.sequence <= lastTaskbarSnapshotSequence) return;
      lastTaskbarSnapshotSequence = event.payload.sequence;
      openWindows = event.payload.windows;
    }));
    registerAsyncUnlistener(listen(TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT, () => {
      void toggleTerminalPanel(terminalControl);
    }));
    registerAsyncUnlistener(listen<TopBarTerminalActivityPayload>(TOP_BAR_TERMINAL_ACTIVITY_EVENT, (event) => {
      const sessionId = event.payload?.sessionId;
      if (sessionId && event.payload?.active === false) {
        activeTerminalActivitySessions.delete(sessionId);
        if (!activeTerminalActivitySessions.size) {
          lastTerminalActivityMs = null;
        }
        if (event.payload?.completed) {
          terminalCompletionPending = true;
          playTerminalCompletionSound();
        }
        return;
      }
      if (sessionId) {
        activeTerminalActivitySessions.add(sessionId);
      }
      terminalCompletionPending = false;
      lastTerminalActivityMs = Date.now();
      terminalActivityNowMs = lastTerminalActivityMs;
    }));
    let shellSurfaceHotkeyHandled = false;
    let terminalSurfaceHotkeyHandled = false;
    const keydownHandler = (event: KeyboardEvent) => {
      if (isAltBackquoteHotkey(event)) {
        event.preventDefault();
        event.stopPropagation();
        if (!terminalSurfaceHotkeyHandled && !event.repeat) {
          terminalSurfaceHotkeyHandled = true;
          void toggleTerminalPanel(terminalControl);
        }
        return;
      }
      if (isCtrlSpaceHotkey(event)) {
        event.preventDefault();
        event.stopPropagation();
        if (!shellSurfaceHotkeyHandled && !event.repeat) {
          shellSurfaceHotkeyHandled = true;
          toggleCenteredSearchFromHotkey();
        }
      }
    };
    const keyupHandler = (event: KeyboardEvent) => {
      if ((event.key === '`' || event.code === 'Backquote') && terminalSurfaceHotkeyHandled) {
        event.preventDefault();
        event.stopPropagation();
        terminalSurfaceHotkeyHandled = false;
        return;
      }
      if (!isSpaceKey(event) || (!event.ctrlKey && !shellSurfaceHotkeyHandled)) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      shellSurfaceHotkeyHandled = false;
    };
    window.addEventListener('keydown', keydownHandler, true);
    window.addEventListener('keyup', keyupHandler, true);
    registerAsyncUnlistener(listen<SearchPanelQueryPayload>(SEARCH_PANEL_QUERY_EVENT, (event) => {
      markSearchPanelInteraction();
      if (!isSearchPanelQueryPayload(event.payload)) {
        return;
      }
      if (event.payload.inputSequence <= lastSearchPanelInputSequence) {
        return;
      }
      lastSearchPanelInputSequence = event.payload.inputSequence;
      const nextQuery = event.payload.query;
      const request = publishImmediateSearchInputState(nextQuery);
      startImmediateSearchQueryExecution(request);
    }));
    registerAsyncUnlistener(listen<string>(SEARCH_PANEL_KEY_EVENT, (event) => {
      markSearchPanelInteraction();
      applySearchKeyboardAction(event.payload);
    }));
    registerAsyncUnlistener(listen<string>(SEARCH_PANEL_PIN_FOLDER_EVENT, (event) => {
      markSearchPanelInteraction();
      void pinSearchFolder(event.payload).catch((error) => {
        console.error('Failed to pin stack folder', error);
      });
    }));
    registerAsyncUnlistener(listen(SEARCH_PANEL_INTERACTION_EVENT, () => {
      markSearchPanelInteraction();
    }));
    registerAsyncUnlistener(listen(SEARCH_PANEL_CLOSED_EVENT, () => {
      resetActiveSearchState();
      searchOpen = false;
      searchPanelAnchor = null;
      lastSearchPanelPayloadSignature = null;
    }));
    registerAsyncUnlistener(listen<SearchProgressPayload>(SEARCH_ENGINE_PROGRESS_EVENT, (event) => {
      applySearchEngineProgress(event.payload);
    }));
    registerAsyncUnlistener(listen<SearchIndexRefreshedPayload>(SEARCH_INDEX_REFRESHED_EVENT, (event) => {
      if (!isAppSearchIndexRefreshedPayload(event.payload)) {
        return;
      }
      if ((event.payload.generatedAtEpochSecs ?? 0) <= lastAppIndexRefreshGeneration) {
        return;
      }
      lastAppIndexRefreshGeneration = event.payload.generatedAtEpochSecs ?? lastAppIndexRefreshGeneration;
      const refreshQuery = searchInputDraft !== searchQuery ? searchInputDraft : searchQuery;
      if (!searchOpen || !refreshQuery.trim()) {
        return;
      }
      resetSearchProviderCacheRetry();
      if (refreshQuery !== searchQuery) {
        const request = publishImmediateSearchInputState(refreshQuery);
        startImmediateSearchQueryExecution(request);
        return;
      }
      scheduleSearchEngine(refreshQuery);
    }));
    registerAsyncUnlistener(listen<StackPin[]>(STACK_PINS_UPDATED_EVENT, (event) => {
      void applyStackPins(event.payload);
    }, {
      target: topBarWebviewWindowEventTarget()
    }));
    registerAsyncUnlistener(listen<TopBarPinMenuActionPayload>(TOP_BAR_PIN_MENU_ACTION_EVENT, (event) => {
      if (event.payload.action === 'open') {
        void openStackPath(event.payload.path, queryPinButton(event.payload.path));
      } else if (event.payload.action === 'openInVscode') {
        void openStackFolderInVscode(event.payload.path).catch((error) => {
          console.error('Failed to open pinned folder in VS Code', error);
        });
      } else if (event.payload.action === 'unpin') {
        void unpinFromMenu(event.payload.path);
      }
    }));
    registerAsyncUnlistener(listen(AUDIO_PANEL_CLOSED_EVENT, () => {
      audioOpen = false;
    }));
    registerAsyncUnlistener(listen(CALENDAR_PANEL_CLOSED_EVENT, () => {
      calendarOpen = false;
    }));
    registerAsyncUnlistener(listen(TRAY_PANEL_CLOSED_EVENT, () => {
      trayOpen = false;
    }));
    registerAsyncUnlistener(listen(TERMINAL_PANEL_CLOSED_EVENT, () => {
      terminalOpen = false;
    }));
    registerAsyncUnlistener(listen(COMMAND_PANEL_CLOSED_EVENT, () => {
      commandOpen = false;
    }));
    registerAsyncUnlistener(getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === 'drop') {
        void pinAndOpenDroppedFolders(event.payload.paths, pinRailEl);
      }
    }));
    unlisteners.push(addShellPreferencesChangeListener((preferences) => {
      shellPreferences = preferences;
    }));
    unlisteners.push(addShellSettingsChangeListener((settings) => {
      void applyTopBarSettings(settings).catch((error) => {
        console.error('Failed to apply top bar settings update', error);
      });
    }));

    return () => {
      disposed = true;
      railScrollButtonsDisposed = true;
      window.clearInterval(timer);
      window.clearInterval(terminalActivityTimer);
      window.clearTimeout(runtimeMetricsTimer);
      cancelSearchEngineTimer();
      cancelSearchFreshnessRetry();
      cancelSearchProviderCacheRetry();
      cancelRailScrollButtonUpdate();
      window.removeEventListener('keydown', keydownHandler, true);
      window.removeEventListener('keyup', keyupHandler, true);
      if (pinDropStatusTimer !== null) {
        window.clearTimeout(pinDropStatusTimer);
      }
      releaseStackPinFocusHold();
      cancelSearchBlurClose();
      void hideSearchPanel().catch(() => undefined);
      void hideAudioPanel().catch(() => undefined);
      void hideTrayPanel().catch(() => undefined);
      void hideTerminalPanel().catch(() => undefined);
      void hideCommandPanel().catch(() => undefined);
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<svelte:window on:pointerdown={handleTopBarPointerDown} on:keydown={handleTopBarKeydown} />

<div class="surface top-bar" style={`--top-bar-height-logical: ${topBarHeightLogical}px;`}>
  <MeltActionButton
    class="shell-home-button"
    ariaHaspopup="dialog"
    ariaLabel="Open JasonShell settings"
    tooltip="Open JasonShell settings"
    onClick={(event) => void openSettingsPanel(event.currentTarget)}
  >
    jasonshell
  </MeltActionButton>
  <div class="rail-wrap">
    {#if showRailScrollLeft}
      <MeltActionButton class="rail-scroll left" ariaLabel="Scroll pinned folders left" tooltip="Scroll pinned folders left" onClick={() => scrollRailLeft()}>&lsaquo;</MeltActionButton>
    {/if}
    <div
      class="stack-pins"
      class:stack-pins-hover={pinRailHover}
      role="toolbar"
      tabindex="0"
      bind:this={pinRailEl}
      aria-label="Pinned folders"
      aria-orientation="horizontal"
      on:dragenter={handlePinRailDragOver}
      on:dragover={handlePinRailDragOver}
      on:dragleave={handlePinRailDragLeave}
      on:drop={handlePinRailDrop}
      on:wheel={handleRailWheel}
      on:keydown={handlePinRailKeydown}
    >
      {#each stackPins as pin, index (pin.id)}
        <MeltActionButton
          title={pin.path}
          tooltip={pin.path}
          dataPath={pin.path}
          ariaLabel={`Open pinned folder ${pin.name}`}
          ariaHaspopup="dialog"
          class={draggingPinPath === pin.path ? 'dragging' : ''}
          style={pinStyle(pin)}
          onPointerDown={(event) => startPinPointerDrag(pin, event)}
          onPointerMove={movePinPointerDrag}
          onPointerUp={finishPinPointerDrag}
          onPointerCancel={cancelPinPointerDrag}
          onLostPointerCapture={(event) => {
            if (pinDragPointerId === event.pointerId) {
              cancelPinPointerDrag();
            }
          }}
          onClick={(event) => handlePinClick(event, pin, index)}
          onContextMenu={(event) => handlePinContextMenu(event, pin)}
        >
          {pin.name}
        </MeltActionButton>
      {/each}
      {#if pinRailHover}
        <div class="pin-drop-overlay" aria-hidden="true">Drop to pin folder</div>
      {/if}
      {#if pinDropStatus}
        <div class:error={pinDropStatusKind === 'error'} class="pin-drop-status" role="status" aria-live="polite">{pinDropStatus}</div>
      {/if}
    </div>
    {#if showRailScrollRight}
      <MeltActionButton class="rail-scroll right" ariaLabel="Scroll pinned folders right" tooltip="Scroll pinned folders right" onClick={() => scrollRailRight()}>&rsaquo;</MeltActionButton>
    {/if}
  </div>
  <div class="terminal-control" bind:this={terminalControl}>
    <MeltActionButton
      class={`terminal-button${terminalCompletionPending ? ' terminal-complete' : ''}`}
      ariaLabel="Open persistent terminal"
      ariaHaspopup="dialog"
      ariaExpanded={terminalOpen}
      ariaControls={TERMINAL_PANEL_ID}
      tooltip="Terminal"
      onClick={(event) => void toggleTerminalPanel(event.currentTarget)}
    >
      <span class="terminal-glyph" aria-hidden="true">{terminalGlyph}</span>
    </MeltActionButton>
  </div>
  <div class="command-control" bind:this={commandControl}>
    <MeltActionButton
      class="command-button"
      ariaLabel="Open quick commands"
      ariaHaspopup="dialog"
      ariaExpanded={commandOpen}
      ariaControls={COMMAND_PANEL_ID}
      tooltip="Quick commands"
      onClick={(event) => void toggleCommandPanel(event.currentTarget)}
    >
      <span class="command-glyph" aria-hidden="true">⌘</span>
    </MeltActionButton>
  </div>
  <div class="tray-control" bind:this={trayControl}>
    <MeltActionButton
      class="tray-button"
      ariaLabel="Open notification area icons"
      ariaHaspopup="dialog"
      ariaExpanded={trayOpen}
      ariaControls={TRAY_PANEL_ID}
      tooltip="Notification area icons"
      onClick={(event) => void toggleTrayPanel(event.currentTarget)}
    >
      <span class="tray-arrow" aria-hidden="true">▾</span>
    </MeltActionButton>
  </div>
  <div class="sound-control" bind:this={soundControl}>
    <MeltActionButton
      class="sound-button"
      ariaLabel="Open sound controls"
      ariaHaspopup="dialog"
      ariaExpanded={audioOpen}
      ariaControls={SOUND_PANEL_ID}
      tooltip="Sound controls"
      onClick={(event) => void toggleSoundPanel(event.currentTarget)}
    >
      <span class="sound-icon" aria-hidden="true"></span>
    </MeltActionButton>
  </div>
  <div class="time-control" bind:this={timeControl}>
    <MeltActionButton
      class="time-pill"
      ariaLabel={`Open calendar. Current time ${shellTime} on ${shellDate}`}
      ariaHaspopup="dialog"
      ariaExpanded={calendarOpen}
      ariaControls={CALENDAR_PANEL_ID}
      tooltip="Calendar"
      onClick={(event) => void toggleCalendarPanel(event.currentTarget)}
    >
      <strong>{shellTime} {shellDate}</strong>
    </MeltActionButton>
  </div>
  <div class="search-control" bind:this={searchControl}>
    <input
      bind:this={searchInput}
      aria-label="Search apps, windows, files, folders, and commands"
      aria-expanded={searchOpen}
      aria-haspopup="listbox"
      autocomplete="off"
      placeholder="Search"
      value={searchInputDraft}
      on:focus={handleSearchFocus}
      on:pointerdown={handleSearchPointerDown}
      on:blur={scheduleSearchBlurClose}
      on:input={handleSearchInput}
      on:keydown={handleSearchKeydown}
    />
    {#if hasSearchClearValue()}
      <MeltActionButton
        class="search-clear-button"
        ariaLabel="Clear search"
        tooltip="Clear search"
        onClick={() => void clearSearch()}
      >
        ×
      </MeltActionButton>
    {/if}
  </div>
  {#if !topBarHeightLocked}
    <MeltActionButton
      class="bar-resize-handle top-bar-resize-handle"
      ariaLabel="Resize top bar"
      onPointerDown={startTopBarHeightResize}
      onPointerMove={moveTopBarHeightResize}
      onPointerUp={finishTopBarHeightResize}
      onPointerCancel={finishTopBarHeightResize}
      onLostPointerCapture={finishTopBarHeightResize}
    ></MeltActionButton>
  {/if}
</div>
