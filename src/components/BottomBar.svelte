<script lang="ts">
  import './BottomBar.css';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, tick } from 'svelte';
  import { emit, listen } from '@tauri-apps/api/event';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import { reportShellSurfaceRuntimeMetrics } from '../lib/runtimeMetrics';
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
  import { showProcessManager } from '../lib/processManager';
  import { hideQuickLaunchPanel } from '../lib/quickLaunchPanel';
  import {
    launchPinnedTaskbarLauncher,
    listPinnedTaskbarLaunchers,
    type PinnedTaskbarLauncher
  } from '../lib/taskbarLaunchers';
  import {
    hasTaskbarLauncherDragStarted,
    orderTaskbarLaunchers,
    preserveExplorerTaskbarPins,
    reconcileTaskbarLauncherOrder,
    resolveTaskbarLauncherPointerRelease,
    taskbarLauncherDragDelta,
    taskbarLauncherKey,
    taskbarLauncherOrderFromDisplacement
  } from '../lib/taskbarPins';
  import {
    showLauncherContextMenu,
    showTaskWindowContextMenu
  } from '../lib/taskbarMenus';
  import {
    allocateTaskPreviewRequestId,
    hideTaskWindowPreview,
    showTaskWindowPreview
  } from '../lib/taskbarPreview';
  import {
    hideTaskGallery as hideTaskGalleryNative,
    showTaskGallery as showTaskGalleryNative
  } from '../lib/taskGallery';
  import { normalizeTaskGalleryProcessId } from '../lib/taskGallery';
  import {
    hasTaskbarGroupDragStarted,
    buildTaskWindowGroups,
    taskGroupDisplayMode,
    taskGroupGalleryItems,
    taskbarGroupDragDelta,
    taskbarGroupDropTargetFromDisplacement,
    taskbarGroupOrderFromDisplacement,
    taskbarGroupReorderOffset,
    taskbarStripPressureState,
    type TaskWindowGroup
  } from '../lib/taskbarGroups';
  import {
    TASKBAR_REFRESH_LAUNCHERS_EVENT,
    TASK_PREVIEW_DELAY_MS,
    TASK_PREVIEW_HIDE_DELAY_MS,
    TASK_PREVIEW_HOVER_ENTER_EVENT,
    type TaskPreviewHoverEnter,
    TASK_PREVIEW_HIDE_REQUEST_EVENT,
    type TaskPreviewHideRequest,
    taskWindowActionLabel,
    taskWindowLabel
  } from '../lib/taskbarUi';
  import {
    pendingTaskbarTilePointer,
    resolveTaskbarTilePointerRelease,
    shouldSuppressTaskbarTileClick
  } from '../lib/taskbarTilePointer';
  import {
    activateTaskWindow,
    listOpenTaskWindows,
    normalizeTaskbarWindow,
    requestTaskbarWindowsRefresh,
    type TaskbarWindow
  } from '../lib/taskbarWindows';
  import {
    nextTaskbarFocusIndex,
    taskbarOverflowState,
    taskGroupStateLabel
  } from '../features/bottom-bar/taskbarUxState';
  const TASKBAR_LAUNCHER_ORDER_STORAGE_KEY = 'jasonshell:bottom-bar:launcher-order:v1';
  const TASKBAR_WINDOWS_SNAPSHOT_EVENT = 'taskbar:windows-snapshot';
  let launcherMessage = 'Loading Explorer taskbar pins…';
  let shellSettings: ShellSettings | null = null;
  let bottomBarHeightLogical = 32.4;
  let bottomBarHeightLocked = true;
  let bottomBarResizePointerId: number | null = null;
  let bottomBarResizeStartY = 0;
  let bottomBarResizeStartHeight = 32.4;
  let bottomBarPendingPersistedHeight: number | null = null;
  const bottomBarResizeScheduler = createShellBarResizeScheduler('bottom', (error) => {
    console.error('Failed to resize bottom bar', error);
  });
  let launchers: PinnedTaskbarLauncher[] = [];
  $: quickLaunchers = [...launchers].sort((left, right) => left.name.localeCompare(right.name));
  let launcherOrder: string[] = readPersistedLauncherOrder();
  let draggingLauncherPath: string | null = null;
  let launcherDragPointerId: number | null = null;
  let launcherDragStartX = 0;
  let launcherDragCurrentX = 0;
  let launcherDragStarted = false;
  let launcherDragOriginalOrder: string[] = [];
  let launcherDragElement: HTMLElement | null = null;
  let launcherDragRects: ReturnType<typeof launcherRects> = [];
  let suppressClickLauncherKey: string | null = null;
  let taskbarMessage = 'Loading open windows…';
  let openWindows: TaskbarWindow[] = [];
  let lastTaskbarSnapshotSequence = 0;
  let launchingShortcutPath: string | null = null;
  let activatingHwnd: string | null = null;
  let previewShowTimer: number | null = null;
  let previewHideTimer: number | null = null;
  let taskGroupOrder: string[] = [];
  let draggingGroupKey: string | null = null;
  let dropTargetGroupKey: string | null = null;
  let taskGroupDragPointerId: number | null = null;
  let taskGroupDragStartX = 0;
  let taskGroupDragCurrentX = 0;
  let taskGroupDragStarted = false;
  let taskGroupDragOriginalOrder: string[] = [];
  let taskGroupPreviewOrder: string[] = [];
  let taskGroupDragElement: HTMLElement | null = null;
  let taskGroupDragRects: ReturnType<typeof taskGroupRects> = [];
  let pendingTaskWindowHwnd: string | null = null;
  let suppressClickTaskWindowHwnd: string | null = null;
  let suppressClickTaskGalleryGroupKey: string | null = null;
  let taskGalleryOpenTimer: number | null = null;
  let taskGalleryCloseTimer: number | null = null;
  let taskbarStripPressure = false;
  let taskStripWidth = 0;
  let taskGalleryOpenGroupKey: string | null = null;
  let taskGalleryOpenNonce: string | null = null;
  let taskGalleryOpenAnchor: { left: number; width: number } | null = null;
  let taskStripEl: HTMLDivElement | null = null;
  let quickLaunchPanelOpen = false;
  let quickLaunchSessionNonce: string | null = null;
  let quickLaunchOpenInFlight = false;
  let suppressQuickLaunchClick = false;
  let taskbarOverflow = taskbarOverflowState(0, 0, 0);
  const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT = 'search:toggle-centered';
  const TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT = 'terminal:toggle-panel';
  const QUICK_LAUNCH_CLOSED_EVENT = 'quick-launch-panel:closed';
  const QUICK_LAUNCH_OPEN_EVENT = 'quick-launch-panel:open';
  type QuickLaunchRow = PinnedTaskbarLauncher;
  type QuickLaunchOpenPayload = { nonce: string; rows: QuickLaunchRow[] };
  type QuickLaunchClosedPayload = { nonce: string };

  function readPersistedLauncherOrder() {
    try {
      const raw = window.localStorage.getItem(TASKBAR_LAUNCHER_ORDER_STORAGE_KEY);
      const parsed = raw ? JSON.parse(raw) : [];
      return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === 'string') : [];
    } catch {
      return [];
    }
  }

  async function loadBottomBarResizeSettings() {
    try {
      const settings = await loadShellSettings();
      await applyBottomBarSettings(settings);
    } catch (error) {
      console.error('Failed to load bottom bar resize settings', error);
    }
  }

  async function applyBottomBarSettings(settings: ShellSettings) {
    shellSettings = settings;
    bottomBarHeightLocked = settings.ui.lockBottomBarHeight;
    const persistedHeight = clampShellBarHeight('bottom', settings.ui.bottomBarHeightLogical);
    const pendingHeightSettled = bottomBarPendingPersistedHeight === persistedHeight;
    bottomBarHeightLogical = shellBarHeightForSettingsUpdate(
      'bottom',
      persistedHeight,
      bottomBarHeightLogical,
      bottomBarResizePointerId !== null || (bottomBarPendingPersistedHeight !== null && !pendingHeightSettled)
    );
    if (pendingHeightSettled) {
      bottomBarPendingPersistedHeight = null;
    }
    await resizeShellBar({ edge: 'bottom', heightLogical: bottomBarHeightLogical });
  }

  function startBottomBarHeightResize(event: PointerEvent) {
    if (bottomBarHeightLocked || event.button !== 0) {
      return;
    }
    event.preventDefault();
    bottomBarResizePointerId = event.pointerId;
    bottomBarResizeStartY = event.clientY;
    bottomBarResizeStartHeight = bottomBarHeightLogical;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function moveBottomBarHeightResize(event: PointerEvent) {
    if (bottomBarResizePointerId !== event.pointerId || bottomBarHeightLocked) {
      return;
    }
    const nextHeight = shellBarHeightFromDrag(
      'bottom',
      bottomBarResizeStartHeight,
      bottomBarResizeStartY,
      event.clientY
    );
    if (nextHeight === bottomBarHeightLogical) {
      return;
    }
    bottomBarHeightLogical = nextHeight;
    bottomBarResizeScheduler.schedule(nextHeight);
  }

  function finishBottomBarHeightResize(event: PointerEvent) {
    if (bottomBarResizePointerId !== event.pointerId) {
      return;
    }
    bottomBarResizePointerId = null;
    const nextHeight = clampShellBarHeight('bottom', bottomBarHeightLogical);
    void bottomBarResizeScheduler.flush(nextHeight);
    if (!shellSettings) {
      return;
    }
    bottomBarPendingPersistedHeight = nextHeight;
    shellSettings = {
      ...shellSettings,
      ui: {
        ...shellSettings.ui,
        bottomBarHeightLogical: nextHeight
      }
    };
    void saveShellBarHeight('bottom', nextHeight)
      .then((savedSettings) => {
        shellSettings = savedSettings;
        if (clampShellBarHeight('bottom', savedSettings.ui.bottomBarHeightLogical) === bottomBarPendingPersistedHeight) {
          bottomBarPendingPersistedHeight = null;
        }
      })
      .catch((error) => {
        bottomBarPendingPersistedHeight = null;
        console.error('Failed to save bottom bar height', error);
      });
  }

  function writePersistedLauncherOrder(order: string[]) {
    try {
      window.localStorage.setItem(TASKBAR_LAUNCHER_ORDER_STORAGE_KEY, JSON.stringify(order));
    } catch {
      // Launcher order is a convenience preference; runtime ordering still works if storage is blocked.
    }
  }

  $: taskGroupDragDeltaX = taskGroupDragStarted
    ? taskbarGroupDragDelta(taskGroupDragStartX, taskGroupDragCurrentX)
    : 0;
  $: launcherDragDeltaX = launcherDragStarted
    ? taskbarLauncherDragDelta(launcherDragStartX, launcherDragCurrentX)
    : 0;
  $: launcherPreviewOrder = launcherDragStarted && draggingLauncherPath
    ? taskbarLauncherOrderFromDisplacement(
        draggingLauncherPath,
        launcherDragOriginalOrder,
        launcherDragRects,
        launcherDragDeltaX
      )
    : launcherOrder;
  $: taskWindowGroups = buildTaskWindowGroups(openWindows, taskGroupOrder);
  $: taskbarStripPressure = taskbarStripPressureState({
    previousPressure: taskbarStripPressure,
    availableWidth: taskStripWidth,
    requiredDirectWidth: taskWindowGroups.reduce((total, group) => {
      const base = group.windows.length === 1 ? 160 : 160 * group.windows.length;
      return total + (group.windows.length >= 2 ? 96 : base);
    }, 0),
    enterThreshold: 24,
    exitThreshold: 48
  });
  $: taskGroupPreviewOrder = taskGroupDragStarted && draggingGroupKey
    ? taskbarGroupOrderFromDisplacement(
        draggingGroupKey,
        taskGroupDragOriginalOrder,
        taskGroupDragRects,
        taskGroupDragDeltaX
      )
    : taskGroupOrder;

  function clearPreviewShowTimer() {
    if (previewShowTimer !== null) {
      window.clearTimeout(previewShowTimer);
      previewShowTimer = null;
    }
  }
  function clearPreviewHideTimer() {
    if (previewHideTimer !== null) {
      window.clearTimeout(previewHideTimer);
      previewHideTimer = null;
    }
  }
  async function hidePreview() {
    clearPreviewShowTimer();
    clearPreviewHideTimer();
    try {
      const requestId = await allocateTaskPreviewRequestId();
      await hideTaskWindowPreview(requestId);
    } catch (error) {
      console.error('Failed to hide task preview', error);
    }
  }
  function schedulePreviewHide() {
    clearPreviewShowTimer();
    clearPreviewHideTimer();
    previewHideTimer = window.setTimeout(() => {
      void hidePreview();
      previewHideTimer = null;
    }, TASK_PREVIEW_HIDE_DELAY_MS);
  }
  function handlePreviewHideRequest(event: { payload: TaskPreviewHideRequest }) {
    if (event.payload.mode === 'schedule') {
      schedulePreviewHide();
      return;
    }
    void hidePreview();
  }
  function queuePreview(taskWindow: TaskbarWindow, event: MouseEvent) {
    const button = event.currentTarget as HTMLButtonElement | null;
    if (!button) {
      return;
    }
    clearPreviewShowTimer();
    clearPreviewHideTimer();
    const rect = button.getBoundingClientRect();
    previewShowTimer = window.setTimeout(async () => {
      try {
        const requestId = await allocateTaskPreviewRequestId();
        await showTaskWindowPreview({
          requestId,
          hwnd: taskWindow.hwnd,
          title: taskWindow.title,
          processName: taskWindow.processName,
          iconDataUrl: taskWindow.iconDataUrl,
          isMinimized: taskWindow.isMinimized,
          anchorLeft: rect.left,
          anchorWidth: rect.width
        });
      } catch (error) {
        console.error(`Failed to show preview for ${taskWindow.hwnd}`, error);
      }
      previewShowTimer = null;
    }, TASK_PREVIEW_DELAY_MS);
  }
  async function refreshTaskbarWindows() {
    try {
      const nextWindows = await listOpenTaskWindows();
      const nextGroups = buildTaskWindowGroups(nextWindows, taskGroupOrder);
      openWindows = nextWindows;
      taskGroupOrder = nextGroups.map((group) => group.key);
      taskbarMessage = openWindows.length ? 'Open task windows' : 'No open task windows';
      void tick().then(updateTaskbarOverflow);
    } catch (error) {
      console.error('Failed to load open task windows', error);
      openWindows = [];
      taskGroupOrder = [];
      taskbarMessage = 'Open windows unavailable';
      updateTaskbarOverflow();
    }
  }
  function applyOptimisticTaskWindowActivation(taskWindow: TaskbarWindow) {
    openWindows = openWindows.map((window) => ({
      ...window,
      isActive: window.hwnd === taskWindow.hwnd ? !taskWindow.isActive : false,
      isMinimized: window.hwnd === taskWindow.hwnd ? taskWindow.isActive : window.isMinimized
    }));
  }
  async function loadPinnedLaunchers() {
    launcherMessage = 'Loading Explorer taskbar pins…';
    try {
      const explorerLaunchers = await listPinnedTaskbarLaunchers();
      const nextLaunchers = preserveExplorerTaskbarPins(explorerLaunchers);
      launcherOrder = reconcileTaskbarLauncherOrder(launcherOrder, nextLaunchers);
      writePersistedLauncherOrder(launcherOrder);
      launchers = orderTaskbarLaunchers(nextLaunchers, launcherOrder);
      launcherMessage = launchers.length
        ? 'Pinned Explorer shortcuts'
        : 'No supported Explorer taskbar pins';
    } catch (error) {
      console.error('Failed to load pinned taskbar launchers', error);
      launchers = [];
      launcherOrder = [];
      launcherMessage = 'Pinned taskbar shortcuts unavailable';
    }
  }
  async function refreshLauncherSections() {
    await loadPinnedLaunchers();
  }
  async function launchApp(launcher: PinnedTaskbarLauncher) {
    if (launchingShortcutPath) {
      return;
    }
    launchingShortcutPath = launcher.shortcutPath;
    try {
      await launchPinnedTaskbarLauncher(launcher.shortcutPath);
      launcherMessage = `Launched ${launcher.name}`;
      await refreshTaskbarWindows();
    } catch (error) {
      console.error(`Failed to launch pinned app ${launcher.name}`, error);
      launcherMessage = `Launch unavailable: ${launcher.name}`;
      await refreshTaskbarWindows();
    } finally {
      launchingShortcutPath = null;
    }
  }
  function launcherRects() {
    return Array.from(document.querySelectorAll<HTMLElement>('.launcher-button[data-path]'))
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
  function launcherStyle(launcher: PinnedTaskbarLauncher) {
    const key = taskbarLauncherKey(launcher);
    const previewOrderIndex = launcherPreviewOrder.indexOf(key);
    if (draggingLauncherPath !== key || !launcherDragStarted) {
      return previewOrderIndex >= 0 ? `order: ${previewOrderIndex};` : '';
    }
    const liveReorderOffset = taskbarGroupReorderOffset(
      key,
      launcherPreviewOrder,
      launcherDragRects
    );
    const visualDelta = launcherDragDeltaX + liveReorderOffset;
    return `order: ${previewOrderIndex}; transform: translate3d(${visualDelta}px, -1px, 0); z-index: 2;`;
  }
  function releaseLauncherPointerCapture() {
    if (!launcherDragElement || launcherDragPointerId === null) {
      return;
    }
    try {
      if (launcherDragElement.hasPointerCapture(launcherDragPointerId)) {
        launcherDragElement.releasePointerCapture(launcherDragPointerId);
      }
    } catch {
      // Capture can already be gone after OS/browser pointer cancellation.
    }
  }
  function resetLauncherPointerDrag() {
    draggingLauncherPath = null;
    launcherDragPointerId = null;
    launcherDragStartX = 0;
    launcherDragCurrentX = 0;
    launcherDragStarted = false;
    launcherDragOriginalOrder = [];
    launcherDragElement = null;
    launcherDragRects = [];
  }
  function cancelLauncherPointerDrag() {
    releaseLauncherPointerCapture();
    resetLauncherPointerDrag();
  }
  function startLauncherPointerDrag(launcher: PinnedTaskbarLauncher, event: PointerEvent) {
    if (event.button !== 0 || launchingShortcutPath) {
      return;
    }
    draggingLauncherPath = taskbarLauncherKey(launcher);
    launcherDragPointerId = event.pointerId;
    launcherDragStartX = event.clientX;
    launcherDragCurrentX = event.clientX;
    launcherDragStarted = false;
    launcherDragOriginalOrder = launchers.map(taskbarLauncherKey);
    launcherDragElement = event.currentTarget as HTMLElement;
    launcherDragRects = launcherRects();
    try {
      launcherDragElement.setPointerCapture(event.pointerId);
    } catch {
      // Reorder still works while pointer remains over the launcher strip.
    }
  }
  function moveLauncherPointerDrag(event: PointerEvent) {
    if (launcherDragPointerId !== event.pointerId || !draggingLauncherPath) {
      return;
    }
    launcherDragCurrentX = event.clientX;
    const started = hasTaskbarLauncherDragStarted(launcherDragStartX, event.clientX);
    if (!launcherDragStarted && started) {
      launcherDragStarted = true;
      clearPreviewShowTimer();
      void hidePreview();
    }
    if (launcherDragStarted) {
      event.preventDefault();
    }
  }
  function finishLauncherPointerDrag(event: PointerEvent) {
    if (launcherDragPointerId !== event.pointerId) {
      return;
    }
    const sourcePath = draggingLauncherPath;
    const didDrag = launcherDragStarted;
    const releaseResult = resolveTaskbarLauncherPointerRelease(sourcePath, didDrag);
    if (didDrag && sourcePath) {
      const nextOrder = taskbarLauncherOrderFromDisplacement(
        sourcePath,
        launcherDragOriginalOrder,
        launcherDragRects,
        taskbarLauncherDragDelta(launcherDragStartX, event.clientX)
      );
      if (nextOrder !== launcherDragOriginalOrder) {
        launcherOrder = nextOrder;
        writePersistedLauncherOrder(launcherOrder);
        launchers = orderTaskbarLaunchers(launchers, launcherOrder);
        launcherMessage = 'Reordered pinned taskbar shortcuts';
      }
    }
    releaseLauncherPointerCapture();
    resetLauncherPointerDrag();
    suppressClickLauncherKey = releaseResult.suppressClickKey;
  }
  function handleLauncherClick(launcher: PinnedTaskbarLauncher, event: MouseEvent) {
    if (suppressClickLauncherKey === taskbarLauncherKey(launcher)) {
      event.preventDefault();
      event.stopPropagation();
      suppressClickLauncherKey = null;
      return;
    }
    void launchApp(launcher);
  }
  async function toggleWindow(taskWindow: TaskbarWindow) {
    if (activatingHwnd) {
      return;
    }
    activatingHwnd = taskWindow.hwnd;
    await hidePreview();

    try {
      await activateTaskWindow(taskWindow.hwnd, taskWindow.isActive);
      applyOptimisticTaskWindowActivation(taskWindow);
      void requestTaskbarWindowsRefresh();
      taskbarMessage = 'Window toggled';
    } catch (error) {
      console.error(`Failed to focus task window ${taskWindow.hwnd}`, error);
      taskbarMessage = `Window focus unavailable for ${taskWindowLabel(taskWindow)}`;
      void refreshTaskbarWindows();
    } finally {
      activatingHwnd = null;
    }
  }
  async function openTaskMenu(taskWindow: TaskbarWindow, event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    await hidePreview();
    try {
      await showTaskWindowContextMenu({
        hwnd: taskWindow.hwnd,
        processId: normalizeTaskGalleryProcessId(taskWindow.processId),
        isMinimized: taskWindow.isMinimized,
        x: event.clientX,
        y: event.clientY
      });
    } catch (error) {
      console.error(`Failed to open task menu for ${taskWindow.hwnd}`, error);
    }
  }
  async function openLauncherMenu(launcher: PinnedTaskbarLauncher, event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    await hidePreview();
    try {
      await showLauncherContextMenu({
        shortcutPath: launcher.shortcutPath,
        x: event.clientX,
        y: event.clientY
      });
    } catch (error) {
      console.error(`Failed to open launcher menu for ${launcher.name}`, error);
    }
  }
  function taskGroupLabel(group: TaskWindowGroup) {
    return taskGroupStateLabel(group);
  }
  function taskWindowHasVisibleAttention(taskWindow: TaskbarWindow) {
    return taskWindow.attentionState === 'requested' && !taskWindow.isActive;
  }
  function taskGroupHasToast(group: TaskWindowGroup) {
    return !group.isActive && group.toastCount > 0;
  }
  function taskGroupDisplay(group: TaskWindowGroup) {
    return taskGroupDisplayMode(group, { policy: 'auto', pressure: taskbarStripPressure });
  }
  function taskGroupDisplayClass(group: TaskWindowGroup) {
    return taskGroupDisplay(group) === 'capsule' ? 'task-group-capsule' : 'task-group-direct';
  }
  function resolveTaskGalleryOpenGroup(groupKey: string) {
    const nextGroup = taskWindowGroups.find((group) => group.key === groupKey);
    return nextGroup && taskGroupDisplay(nextGroup) === 'capsule' ? nextGroup : null;
  }
  $: if (taskGalleryOpenGroupKey) {
    const openGalleryGroup = taskWindowGroups.find((group) => group.key === taskGalleryOpenGroupKey);
    if (!openGalleryGroup || taskGroupDisplay(openGalleryGroup) !== 'capsule') void closeTaskGallery();
  }
  function taskGroupStyle(group: TaskWindowGroup) {
    const previewOrderIndex = taskGroupPreviewOrder.indexOf(group.key);
    const windowCountStyle = `--task-window-count: ${Math.max(group.windows.length, 1)}; --task-direct-count: ${Math.max(group.windows.length, 1)};`;
    if (draggingGroupKey !== group.key || !taskGroupDragStarted) {
      return `${windowCountStyle}${previewOrderIndex >= 0 ? ` order: ${previewOrderIndex};` : ''}`;
    }

    const liveReorderOffset = taskbarGroupReorderOffset(
      group.key,
      taskGroupPreviewOrder,
      taskGroupDragRects
    );
    const visualDelta = taskGroupDragDeltaX + liveReorderOffset;
    return draggingGroupKey === group.key && taskGroupDragStarted
      ? `${windowCountStyle} order: ${previewOrderIndex}; transform: translate3d(${visualDelta}px, -1px, 0); z-index: 2;`
      : `${windowCountStyle}${previewOrderIndex >= 0 ? ` order: ${previewOrderIndex};` : ''}`;
  }
  async function closeTaskGallery() {
    cancelTaskGalleryOpen();
    cancelTaskGalleryClose();
    const nonce = taskGalleryOpenNonce;
    taskGalleryOpenGroupKey = null;
    taskGalleryOpenNonce = null;
    taskGalleryOpenAnchor = null;
    if (!nonce) return;
    await hideTaskGalleryNative(nonce).catch((error) => {
      console.error('Failed to hide task gallery', error);
    });
  }
  async function openTaskGallery(group: TaskWindowGroup, anchor: HTMLElement | null, focusGallery: boolean) {
    cancelTaskGalleryOpen();
    if (draggingGroupKey === group.key && taskGroupDragStarted) {
      return;
    }
    const rect = anchor?.getBoundingClientRect();
    const nonce = crypto.randomUUID();
    const windows = taskGroupGalleryItems(group);
    if (taskGalleryOpenNonce) {
      await closeTaskGallery();
    }
    taskGalleryOpenGroupKey = group.key;
    taskGalleryOpenNonce = nonce;
    taskGalleryOpenAnchor = rect ? { left: rect.left, width: rect.width } : null;
    try {
      await showTaskGalleryNative({
        nonce,
        groupKey: group.key,
        label: group.label,
        anchorLeft: rect?.left ?? 0,
        anchorWidth: rect?.width ?? 0,
        focusGallery,
        refreshExisting: false,
        windows
      });
    } catch (error) {
      if (taskGalleryOpenNonce === nonce) {
        taskGalleryOpenGroupKey = null;
        taskGalleryOpenNonce = null;
        taskGalleryOpenAnchor = null;
      }
      console.error(`Failed to show task gallery for ${group.key}`, error);
    }
  }
  function cancelTaskGalleryOpen() {
    if (taskGalleryOpenTimer === null) return;
    window.clearTimeout(taskGalleryOpenTimer);
    taskGalleryOpenTimer = null;
  }
  function cancelTaskGalleryClose() {
    if (taskGalleryCloseTimer === null) return;
    window.clearTimeout(taskGalleryCloseTimer);
    taskGalleryCloseTimer = null;
  }
  function scheduleTaskGalleryClose(groupKey: string) {
    cancelTaskGalleryOpen();
    cancelTaskGalleryClose();
    if (taskGalleryOpenGroupKey !== groupKey) return;
    taskGalleryCloseTimer = window.setTimeout(() => {
      taskGalleryCloseTimer = null;
      if (taskGalleryOpenGroupKey === groupKey) void closeTaskGallery();
    }, TASK_PREVIEW_HIDE_DELAY_MS);
  }
  function scheduleTaskGalleryOpen(group: TaskWindowGroup, event: MouseEvent) {
    cancelTaskGalleryClose();
    if (taskGalleryOpenGroupKey === group.key || taskGroupDragPointerId !== null) return;
    cancelTaskGalleryOpen();
    const anchor = event.currentTarget as HTMLElement | null;
    taskGalleryOpenTimer = window.setTimeout(() => {
      taskGalleryOpenTimer = null;
      const nextGroup = resolveTaskGalleryOpenGroup(group.key);
      if (!nextGroup) {
        return;
      }
      void openTaskGallery(nextGroup, anchor, false);
    }, 300);
  }
  function taskGroupRects() {
    return Array.from(document.querySelectorAll<HTMLElement>('[data-task-group-key]'))
      .map((element) => {
        const rect = element.getBoundingClientRect();
        return {
          key: element.dataset.taskGroupKey ?? '',
          left: rect.left,
          width: rect.width
        };
      })
      .filter((rect) => rect.key);
  }
  function applyTaskGroupPointerPlacement(clientX: number) {
    if (!draggingGroupKey) {
      return;
    }

    const rects = taskGroupDragRects.length ? taskGroupDragRects : taskGroupRects();
    const target = taskbarGroupDropTargetFromDisplacement(
      draggingGroupKey,
      taskGroupDragOriginalOrder,
      rects,
      taskbarGroupDragDelta(taskGroupDragStartX, clientX)
    );
    if (!target) {
      return;
    }

    dropTargetGroupKey = target.targetKey;
  }
  function releaseTaskGroupPointerCapture() {
    if (!taskGroupDragElement || taskGroupDragPointerId === null) {
      return;
    }

    try {
      if (taskGroupDragElement.hasPointerCapture(taskGroupDragPointerId)) {
        taskGroupDragElement.releasePointerCapture(taskGroupDragPointerId);
      }
    } catch {
      // Pointer capture can already be gone if the OS/browser canceled the pointer stream.
    }
  }
  function captureTaskGroupPointer(pointerId: number) {
    try {
      taskGroupDragElement?.setPointerCapture(pointerId);
    } catch {
      // Drag still works while the pointer remains over the taskbar; capture is best effort.
    }
  }
  function resetTaskGroupPointerDrag() {
    draggingGroupKey = null;
    dropTargetGroupKey = null;
    taskGroupDragPointerId = null;
    taskGroupDragStartX = 0;
    taskGroupDragCurrentX = 0;
    taskGroupDragStarted = false;
    taskGroupDragOriginalOrder = [];
    taskGroupDragElement = null;
    taskGroupDragRects = [];
    pendingTaskWindowHwnd = null;
  }
  function cancelTaskGroupPointerDrag() {
    releaseTaskGroupPointerCapture();
    if (taskGroupDragStarted && taskGroupDragOriginalOrder.length) {
      taskGroupOrder = taskGroupDragOriginalOrder;
    }
    resetTaskGroupPointerDrag();
  }
  function startTaskGroupPointerDrag(group: TaskWindowGroup, event: PointerEvent) {
    if (event.button !== 0 || activatingHwnd) {
      return;
    }
    cancelTaskGalleryOpen();
    const target = event.currentTarget as HTMLElement;
    draggingGroupKey = group.key;
    dropTargetGroupKey = group.key;
    taskGroupDragPointerId = event.pointerId;
    taskGroupDragStartX = event.clientX;
    taskGroupDragCurrentX = event.clientX;
    taskGroupDragStarted = false;
    taskGroupDragOriginalOrder = taskWindowGroups.map((item) => item.key);
    taskGroupDragElement = target;
    taskGroupDragRects = taskGroupRects();
    // Capture immediately so crossing a neighbor before the drag threshold does not drop rightward moves.
    captureTaskGroupPointer(event.pointerId);
  }
  function handleTaskWindowPointerDown(taskWindow: TaskbarWindow, event: PointerEvent) {
    pendingTaskWindowHwnd = pendingTaskbarTilePointer(event.button, taskWindow.hwnd);
  }
  function moveTaskGroupPointerDrag(event: PointerEvent) {
    if (taskGroupDragPointerId !== event.pointerId || !draggingGroupKey) {
      return;
    }

    taskGroupDragCurrentX = event.clientX;
    const started = hasTaskbarGroupDragStarted(taskGroupDragStartX, event.clientX);
    if (!taskGroupDragStarted && started) {
      taskGroupDragStarted = true;
      clearPreviewShowTimer();
      void hidePreview();
    }
    if (!taskGroupDragStarted) {
      return;
    }

    event.preventDefault();
    applyTaskGroupPointerPlacement(event.clientX);
  }
  function finishTaskGroupPointerDrag(event: PointerEvent) {
    if (taskGroupDragPointerId !== event.pointerId) {
      return;
    }
    const releasedGroupKey = draggingGroupKey;
    const didDrag = taskGroupDragStarted;
    const releaseResult = resolveTaskbarTilePointerRelease(
      pendingTaskWindowHwnd,
      taskGroupDragStarted
    );
    if (taskGroupDragStarted && draggingGroupKey) {
      applyTaskGroupPointerPlacement(event.clientX);
      taskGroupOrder = taskbarGroupOrderFromDisplacement(
        draggingGroupKey,
        taskGroupDragOriginalOrder,
        taskGroupDragRects,
        taskbarGroupDragDelta(taskGroupDragStartX, event.clientX)
      );
    }
    releaseTaskGroupPointerCapture();
    resetTaskGroupPointerDrag();
    suppressClickTaskWindowHwnd = releaseResult.suppressClickHwnd;
    suppressClickTaskGalleryGroupKey = didDrag ? releasedGroupKey : null;
    if (releaseResult.activateHwnd) {
      const taskWindow = openWindows.find((item) => item.hwnd === releaseResult.activateHwnd);
      if (taskWindow) {
        void toggleWindow(taskWindow);
      }
    }
  }
  function handleTaskGroupLostPointerCapture(event: PointerEvent) {
    if (taskGroupDragPointerId === event.pointerId) {
      cancelTaskGroupPointerDrag();
    }
  }
  function handleTaskWindowClick(taskWindow: TaskbarWindow, event: MouseEvent) {
    if (shouldSuppressTaskbarTileClick(suppressClickTaskWindowHwnd, taskWindow.hwnd)) {
      event.preventDefault();
      event.stopPropagation();
      suppressClickTaskWindowHwnd = null;
      return;
    }
    void toggleWindow(taskWindow);
  }
  function handleTaskGalleryClick(group: TaskWindowGroup, event: MouseEvent) {
    event.stopPropagation();
    cancelTaskGalleryOpen();
    if (suppressClickTaskGalleryGroupKey === group.key) {
      event.preventDefault();
      suppressClickTaskGalleryGroupKey = null;
      return;
    }
    if (taskGalleryOpenGroupKey === group.key) {
      void closeTaskGallery();
      return;
    }
    void openTaskGallery(group, event.currentTarget as HTMLElement | null, true);
  }
  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') {
      return;
    }
    if (quickLaunchPanelOpen) {
      event.preventDefault();
      void hideQuickLaunchPanel();
      return;
    }
    if (taskGalleryOpenNonce) {
      event.preventDefault();
      void closeTaskGallery();
      return;
    }
    if (launcherDragPointerId !== null) {
      event.preventDefault();
      cancelLauncherPointerDrag();
      return;
    }
    if (taskGroupDragPointerId !== null) {
      event.preventDefault();
      cancelTaskGroupPointerDrag();
    }
  }
  function updateTaskbarOverflow() {
    if (!taskStripEl) {
      taskbarOverflow = taskbarOverflowState(0, 0, taskWindowGroups.length);
      return;
    }
    taskStripWidth = taskStripEl.clientWidth;
    taskbarOverflow = taskbarOverflowState(
      taskStripEl.clientWidth,
      taskStripEl.scrollWidth,
      taskWindowGroups.length
    );
  }
  function taskStripButtons() {
    return Array.from(taskStripEl?.querySelectorAll<HTMLButtonElement>('.task-button') ?? []);
  }
  function handleTaskStripKeydown(event: KeyboardEvent) {
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) {
      return;
    }
    const buttons = taskStripButtons();
    if (!buttons.length) {
      return;
    }
    const currentIndex = Math.max(0, buttons.indexOf(document.activeElement as HTMLButtonElement));
    const nextIndex = nextTaskbarFocusIndex(currentIndex, buttons.length, event.key);
    if (nextIndex < 0) {
      return;
    }
    event.preventDefault();
    buttons[nextIndex]?.focus();
    buttons[nextIndex]?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    updateTaskbarOverflow();
  }
  function isCtrlSpaceHotkey(event: KeyboardEvent) {
    return event.code === 'Space' && event.ctrlKey && !event.altKey && !event.metaKey;
  }
  function isAltBackquoteHotkey(event: KeyboardEvent) {
    return event.altKey && !event.ctrlKey && !event.metaKey && (event.key === '`' || event.code === 'Backquote');
  }
  function isSpaceKey(event: KeyboardEvent) {
    return event.code === 'Space';
  }
  async function openProcessManager(event: MouseEvent) {
    const button = event.currentTarget as HTMLButtonElement | null;
    if (!button) {
      return;
    }
    await hidePreview();
    const rect = button.getBoundingClientRect();
    try {
      await showProcessManager({ anchorLeft: rect.left, anchorWidth: rect.width });
    } catch (error) {
      console.error('Failed to open process manager', error);
    }
  }
  async function openQuickLaunchPanel() {
    if (quickLaunchOpenInFlight) return;
    if (quickLaunchPanelOpen) {
      await hideQuickLaunchPanel();
      return;
    }
    quickLaunchOpenInFlight = true;
    const nonce = crypto.randomUUID();
    const button = document.querySelector<HTMLButtonElement>('.quick-launch-button');
    const rect = button?.getBoundingClientRect();
    const rows = [...launchers].sort((left, right) => left.name.localeCompare(right.name));
    quickLaunchSessionNonce = nonce;
    quickLaunchPanelOpen = true;
    try {
      await invoke('show_quick_launch_panel', { args: { anchorLeft: rect?.left ?? 0, anchorWidth: rect?.width ?? 0, nonce, rows } });
    } catch (error) {
      if (quickLaunchSessionNonce === nonce) {
        quickLaunchSessionNonce = null;
        quickLaunchPanelOpen = false;
      }
      throw error;
    } finally {
      quickLaunchOpenInFlight = false;
    }
  }
  function handleQuickLaunchPointerDown(event: PointerEvent) {
    if (event.button !== 0 || !quickLaunchPanelOpen) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    suppressQuickLaunchClick = true;
    void hideQuickLaunchPanel();
  }
  function handleQuickLaunchClick() {
    if (suppressQuickLaunchClick) {
      suppressQuickLaunchClick = false;
      return;
    }
    void openQuickLaunchPanel();
  }
  function isValidQuickLaunchPayload(payload: unknown): payload is QuickLaunchClosedPayload {
    return !!payload && typeof payload === 'object' && typeof (payload as { nonce?: unknown }).nonce === 'string';
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
    void loadBottomBarResizeSettings();
    void Promise.all([refreshLauncherSections(), refreshTaskbarWindows()]);

    registerAsyncUnlistener(listen<{ sequence: number; windows: TaskbarWindow[] }>('taskbar:windows-snapshot', (event: { payload: { sequence: number; windows: TaskbarWindow[] } }) => {
      if (event.payload.sequence <= lastTaskbarSnapshotSequence) return;
      lastTaskbarSnapshotSequence = event.payload.sequence;
      const nextWindows = event.payload.windows.map(normalizeTaskbarWindow);
      const nextGroups = buildTaskWindowGroups(nextWindows, taskGroupOrder);
      openWindows = nextWindows;
      taskGroupOrder = nextGroups.map((group) => group.key);
      const galleryGroup = taskGalleryOpenGroupKey
        ? nextGroups.find((group) => group.key === taskGalleryOpenGroupKey)
        : null;
      if (taskGalleryOpenGroupKey && (!galleryGroup || galleryGroup.windows.length < 2 || taskGroupDisplay(galleryGroup) !== 'capsule')) {
        void closeTaskGallery();
      } else if (galleryGroup && taskGalleryOpenNonce) {
        const galleryElement = document.querySelector<HTMLElement>(`[data-task-group-key="${CSS.escape(galleryGroup.key)}"] .task-capsule`);
        const galleryRect = galleryElement?.getBoundingClientRect();
        taskGalleryOpenAnchor = galleryRect ? { left: galleryRect.left, width: galleryRect.width } : taskGalleryOpenAnchor;
        void showTaskGalleryNative({
          nonce: taskGalleryOpenNonce,
          groupKey: galleryGroup.key,
          label: galleryGroup.label,
          anchorLeft: galleryRect?.left ?? taskGalleryOpenAnchor?.left ?? 0,
          anchorWidth: galleryRect?.width ?? taskGalleryOpenAnchor?.width ?? 0,
          focusGallery: false,
          refreshExisting: true,
          windows: taskGroupGalleryItems(galleryGroup)
        }).catch((error) => console.error(`Failed to refresh task gallery for ${galleryGroup.key}`, error));
      }
    }));
    registerAsyncUnlistener(listen('taskbar:refresh-windows', () => {
      void requestTaskbarWindowsRefresh();
    }));
    registerAsyncUnlistener(listen<QuickLaunchClosedPayload>(QUICK_LAUNCH_CLOSED_EVENT, (event: { payload: QuickLaunchClosedPayload }) => {
      if (!isValidQuickLaunchPayload(event.payload) || event.payload.nonce !== quickLaunchSessionNonce) return;
      quickLaunchSessionNonce = null;
      quickLaunchPanelOpen = false;
    }));
    registerAsyncUnlistener(listen<{ nonce: string | null }>('task-gallery:closed', (event: { payload: { nonce: string | null } }) => {
      if (event.payload.nonce && event.payload.nonce !== taskGalleryOpenNonce) return;
      taskGalleryOpenGroupKey = null;
      taskGalleryOpenNonce = null;
      taskGalleryOpenAnchor = null;
    }));

    registerAsyncUnlistener(listen(TASKBAR_REFRESH_LAUNCHERS_EVENT, () => {
      void refreshLauncherSections();
    }));

    registerAsyncUnlistener(listen<TaskPreviewHoverEnter>(TASK_PREVIEW_HOVER_ENTER_EVENT, () => {
      clearPreviewHideTimer();
      cancelTaskGalleryClose();
    }));

    registerAsyncUnlistener(listen<TaskPreviewHideRequest>(TASK_PREVIEW_HIDE_REQUEST_EVENT, (event: { payload: TaskPreviewHideRequest }) => {
      handlePreviewHideRequest(event);
    }));
    unlisteners.push(addShellSettingsChangeListener((settings) => {
      void applyBottomBarSettings(settings).catch((error) => {
        console.error('Failed to apply bottom bar settings update', error);
      });
    }));

    const resizeHandler = () => updateTaskbarOverflow();
    let shellSurfaceHotkeyHandled = false;
    let terminalSurfaceHotkeyHandled = false;
    const keydownHandler = (event: KeyboardEvent) => {
      if (isAltBackquoteHotkey(event)) {
        event.preventDefault();
        event.stopPropagation();
        if (!terminalSurfaceHotkeyHandled && !event.repeat) {
          terminalSurfaceHotkeyHandled = true;
          void emit(TERMINAL_HOTKEY_TOGGLE_TERMINAL_EVENT);
        }
        return;
      }
      if (!isCtrlSpaceHotkey(event)) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      if (!shellSurfaceHotkeyHandled && !event.repeat) {
        shellSurfaceHotkeyHandled = true;
        void emit(SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT);
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
    window.addEventListener('resize', resizeHandler);
    window.addEventListener('keydown', keydownHandler, true);
    window.addEventListener('keyup', keyupHandler, true);

    const runtimeMetricsTimer = window.setTimeout(() => {
      void reportShellSurfaceRuntimeMetrics('bottom-bar').catch((error) => {
        console.error('Bottom bar runtime metrics failed', error);
      });
    }, 250);

    return () => {
      disposed = true;
      clearPreviewShowTimer();
      clearPreviewHideTimer();
      cancelTaskGalleryOpen();
      cancelTaskGalleryClose();
      void hideTaskGalleryNative().catch(() => undefined);
      void hidePreview();
      window.clearTimeout(runtimeMetricsTimer);
      window.removeEventListener('resize', resizeHandler);
      window.removeEventListener('keydown', keydownHandler, true);
      window.removeEventListener('keyup', keyupHandler, true);
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<div class="surface bottom-bar" style={`--bottom-bar-height-logical: ${bottomBarHeightLogical}px;`}>
  {#if !bottomBarHeightLocked}
    <MeltActionButton
      class="bar-resize-handle bottom-bar-resize-handle"
      ariaLabel="Resize bottom bar"
      onPointerDown={startBottomBarHeightResize}
      onPointerMove={moveBottomBarHeightResize}
      onPointerUp={finishBottomBarHeightResize}
      onPointerCancel={finishBottomBarHeightResize}
      onLostPointerCapture={finishBottomBarHeightResize}
    ></MeltActionButton>
  {/if}
  <section class="taskbar-strip" aria-label="Taskbar">
    <div class="launcher-strip" aria-label="Pinned Explorer taskbar apps">
      <MeltActionButton class="quick-launch-button" type="button" ariaLabel="Quick Launch" ariaExpanded={quickLaunchPanelOpen} ariaHaspopup="dialog" onPointerDown={handleQuickLaunchPointerDown} onClick={handleQuickLaunchClick}>Quick Launch</MeltActionButton>
    </div>

    <div
      class:task-strip-overflow={taskbarOverflow.hasOverflow}
      class="task-strip"
      role="toolbar"
      aria-label="Open windows"
      aria-orientation="horizontal"
      aria-describedby="taskbar-overflow-status"
      tabindex="-1"
      bind:this={taskStripEl}
      on:keydown={handleTaskStripKeydown}
    >
      {#if taskWindowGroups.length}
        {#each taskWindowGroups as group (group.key)}
          <div
            class:task-group-active={group.isActive}
            class:task-group-toasted={taskGroupHasToast(group)}
            class:task-group-busy={group.isBusy}
            class:task-group-minimized={group.isMinimized}
            class:task-group-dragging={draggingGroupKey === group.key}
            class:task-group-drop-target={dropTargetGroupKey === group.key && draggingGroupKey !== group.key}
            class={`task-group ${taskGroupDisplayClass(group)}`}
            data-task-group-key={group.key}
            data-window-count={group.windows.length}
            role="group"
            aria-label={taskGroupLabel(group)}
            style={taskGroupStyle(group)}
            on:pointerdown={(event) => startTaskGroupPointerDrag(group, event)}
            on:pointermove={moveTaskGroupPointerDrag}
            on:pointerup={finishTaskGroupPointerDrag}
            on:pointercancel={cancelTaskGroupPointerDrag}
            on:lostpointercapture={handleTaskGroupLostPointerCapture}
          >
            {#if taskGroupDisplay(group) === 'direct'}
              {#each group.windows as taskWindow (taskWindow.hwnd)}
                <MeltActionButton
                  class={`task-button${taskWindow.isActive ? ' task-button-active' : ''}${taskWindow.isMinimized ? ' task-button-minimized' : ''}${taskWindowHasVisibleAttention(taskWindow) ? ' task-window-attention' : ''}`}
                  type="button"
                  disabled={activatingHwnd === taskWindow.hwnd}
                  onPointerDown={(event) => handleTaskWindowPointerDown(taskWindow, event)}
                  onClick={(event) => handleTaskWindowClick(taskWindow, event)}
                  onMouseEnter={(event) => queuePreview(taskWindow, event)}
                  onMouseLeave={schedulePreviewHide}
                  onContextMenu={(event) => void openTaskMenu(taskWindow, event)}
                >
                  <img class="task-icon" src={taskWindow.iconDataUrl} alt="" draggable="false" />
                  <span class="task-label">{taskWindowLabel(taskWindow)}</span>
                </MeltActionButton>
              {/each}
            {:else}
              <MeltActionButton
                class={`task-button task-capsule${group.isActive ? ' task-button-active' : ''}${group.isMinimized ? ' task-button-minimized' : ''}`}
                type="button"
                ariaExpanded={taskGalleryOpenGroupKey === group.key}
                ariaHaspopup="dialog"
                onClick={(event) => handleTaskGalleryClick(group, event)}
                onMouseEnter={(event) => scheduleTaskGalleryOpen(group, event)}
                onMouseLeave={() => scheduleTaskGalleryClose(group.key)}
              >
                <img class="task-icon" src={taskGroupGalleryItems(group)[0]?.iconDataUrl ?? ''} alt="" draggable="false" />
                <span class="task-label">{group.label}</span>
                <span class="task-count" aria-label={`${group.windows.length} windows`}>{group.windows.length}</span>
              </MeltActionButton>
            {/if}
          </div>
        {/each}
      {:else}
        <div class="strip-fallback">{taskbarMessage}</div>
      {/if}
      <div
        id="taskbar-overflow-status"
        class:visible={taskbarOverflow.hasOverflow}
        class="taskbar-overflow-status"
        role="status"
        aria-live="polite"
      >
        {taskbarOverflow.summary}
      </div>
    </div>
  </section>
  <MeltActionButton
    class="process-manager-button"
    type="button"
    title="Processes"
    ariaLabel="Open process manager"
    onClick={(event) => void openProcessManager(event)}
  >
    ▦
  </MeltActionButton>
</div>
