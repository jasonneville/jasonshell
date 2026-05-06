<script lang="ts">
  import './BottomBar.css';
  import { onMount, tick } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import { reportShellSurfaceRuntimeMetrics } from '../lib/runtimeMetrics';
  import { showProcessManager } from '../lib/processManager';
  import {
    launchPinnedTaskbarLauncher,
    listPinnedTaskbarLaunchers,
    type PinnedTaskbarLauncher
  } from '../lib/taskbarLaunchers';
  import {
    buildQuickIconLaunchFailureState,
    filterExplorerLaunchersForQuickIcons,
    launchQuickIcon,
    listQuickIcons,
    quickIconLaunchErrorFromUnknown,
    unpinQuickIcon,
    type QuickIcon,
    type QuickIconLaunchError
  } from '../lib/quickIcons';
  import {
    showLauncherContextMenu,
    showQuickIconContextMenu,
    showTaskWindowContextMenu
  } from '../lib/taskbarMenus';
  import {
    hideTaskWindowPreview,
    showTaskWindowPreview
  } from '../lib/taskbarPreview';
  import {
    hasTaskbarGroupDragStarted,
    buildTaskWindowGroups,
    taskbarGroupDragDelta,
    taskbarGroupDropTargetFromDisplacement,
    taskbarGroupOrderFromDisplacement,
    taskbarGroupReorderOffset,
    type TaskWindowGroup
  } from '../lib/taskbarGroups';
  import {
    TASKBAR_REFRESH_LAUNCHERS_EVENT,
    TASKBAR_REFRESH_WINDOWS_EVENT,
    TASK_PREVIEW_DELAY_MS,
    TASK_PREVIEW_HIDE_DELAY_MS,
    TASK_PREVIEW_HOVER_ENTER_EVENT,
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
    type TaskbarWindow
  } from '../lib/taskbarWindows';
  import {
    nextTaskbarFocusIndex,
    taskbarOverflowState,
    taskGroupStateLabel
  } from '../features/bottom-bar/taskbarUxState';
  let quickIconMessage = 'Loading quick icons…';
  let launcherMessage = 'Loading Explorer taskbar pins…';
  let quickIcons: QuickIcon[] = [];
  let quickIconLaunchErrors: Record<string, QuickIconLaunchError> = {};
  let launchingQuickIconId: string | null = null;
  let launchers: PinnedTaskbarLauncher[] = [];
  let taskbarMessage = 'Loading open windows…';
  let openWindows: TaskbarWindow[] = [];
  let launchingShortcutPath: string | null = null;
  let activatingHwnd: string | null = null;
  let previewRequestId = 0;
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
  let taskStripEl: HTMLDivElement | null = null;
  let taskbarOverflow = taskbarOverflowState(0, 0, 0);

  $: taskGroupDragDeltaX = taskGroupDragStarted
    ? taskbarGroupDragDelta(taskGroupDragStartX, taskGroupDragCurrentX)
    : 0;
  $: taskWindowGroups = buildTaskWindowGroups(openWindows, taskGroupOrder);
  $: taskGroupPreviewOrder = taskGroupDragStarted && draggingGroupKey
    ? taskbarGroupOrderFromDisplacement(
        draggingGroupKey,
        taskGroupDragOriginalOrder,
        taskGroupDragRects,
        taskGroupDragDeltaX
      )
    : taskGroupOrder;

  function nextPreviewRequestId() {
    previewRequestId += 1;
    return previewRequestId;
  }
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
    const requestId = nextPreviewRequestId();
    await hideTaskWindowPreview(requestId).catch((error) => {
      console.error('Failed to hide task preview', error);
    });
  }
  function schedulePreviewHide() {
    clearPreviewShowTimer();
    clearPreviewHideTimer();
    const requestId = nextPreviewRequestId();
    previewHideTimer = window.setTimeout(() => {
      void hideTaskWindowPreview(requestId).catch((error) => {
        console.error('Failed to hide task preview', error);
      });
      previewHideTimer = null;
    }, TASK_PREVIEW_HIDE_DELAY_MS);
  }
  function queuePreview(taskWindow: TaskbarWindow, event: MouseEvent) {
    const button = event.currentTarget as HTMLButtonElement | null;
    if (!button) {
      return;
    }
    clearPreviewShowTimer();
    clearPreviewHideTimer();
    const requestId = nextPreviewRequestId();
    const rect = button.getBoundingClientRect();
    previewShowTimer = window.setTimeout(() => {
      void showTaskWindowPreview({
        requestId,
        hwnd: taskWindow.hwnd,
        title: taskWindow.title,
        processName: taskWindow.processName,
        iconDataUrl: taskWindow.iconDataUrl,
        isMinimized: taskWindow.isMinimized,
        anchorLeft: rect.left,
        anchorWidth: rect.width
      }).catch((error) => {
        console.error(`Failed to show preview for ${taskWindow.hwnd}`, error);
      });
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
  async function loadPinnedLaunchers() {
    launcherMessage = 'Loading Explorer taskbar pins…';
    try {
      const explorerLaunchers = await listPinnedTaskbarLaunchers();
      launchers = filterExplorerLaunchersForQuickIcons(quickIcons, explorerLaunchers);
      launcherMessage = launchers.length
        ? 'Pinned Explorer shortcuts'
        : 'No supported Explorer taskbar pins';
    } catch (error) {
      console.error('Failed to load pinned taskbar launchers', error);
      launchers = [];
      launcherMessage = 'Pinned taskbar shortcuts unavailable';
    }
  }
  async function refreshLauncherSections() {
    await loadQuickIconLaunchers();
    await loadPinnedLaunchers();
  }
  async function loadQuickIconLaunchers() {
    quickIconMessage = 'Loading quick icons…';
    try {
      quickIcons = await listQuickIcons();
      quickIconLaunchErrors = {};
      quickIconMessage = quickIcons.length
        ? 'Pinned quick icons'
        : 'No pinned quick icons';
    } catch (error) {
      console.error('Failed to load quick icons', error);
      quickIcons = [];
      quickIconMessage = 'Quick icons unavailable';
    }
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
      launchers = launchers.filter((item) => item.shortcutPath !== launcher.shortcutPath);
      launcherMessage = `Skipped unavailable launcher: ${launcher.name}`;
      await refreshTaskbarWindows();
    } finally {
      launchingShortcutPath = null;
    }
  }
  async function launchQuickIconFromBottomBar(quickIcon: QuickIcon) {
    if (launchingQuickIconId) {
      return;
    }
    launchingQuickIconId = quickIcon.id;
    const { [quickIcon.id]: _clearedLaunchError, ...remainingLaunchErrors } = quickIconLaunchErrors;
    void _clearedLaunchError;
    quickIconLaunchErrors = remainingLaunchErrors;
    try {
      await launchQuickIcon({ id: quickIcon.id });
      quickIconMessage = `Launched ${quickIcon.name}`;
      await refreshTaskbarWindows();
    } catch (error) {
      console.error(`Failed to launch quick icon ${quickIcon.name}`, error);
      const failure = buildQuickIconLaunchFailureState(
        quickIcons,
        quickIcon.id,
        quickIconLaunchErrorFromUnknown(error, quickIcon.id)
      );
      quickIcons = failure.quickIcons;
      quickIconLaunchErrors = { ...quickIconLaunchErrors, ...failure.errorById };
      quickIconMessage = `Launch unavailable: ${quickIcon.name}`;
      await refreshTaskbarWindows();
    } finally {
      launchingQuickIconId = null;
    }
  }
  async function openQuickIconMenu(quickIcon: QuickIcon, event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    await hidePreview();
    try {
      await showQuickIconContextMenu({
        quickIconId: quickIcon.id,
        x: event.clientX,
        y: event.clientY
      });
    } catch (error) {
      console.error(`Failed to open quick icon menu for ${quickIcon.name}`, error);
    }
  }
  async function unpinQuickIconFromBottomBar(quickIcon: QuickIcon) {
    try {
      quickIcons = await unpinQuickIcon({ id: quickIcon.id });
      quickIconMessage = `Unpinned ${quickIcon.name}`;
    } catch (error) {
      console.error(`Failed to unpin quick icon ${quickIcon.name}`, error);
      quickIconMessage = `Unpin unavailable: ${quickIcon.name}`;
    }
  }
  async function toggleWindow(taskWindow: TaskbarWindow) {
    if (activatingHwnd) {
      return;
    }
    activatingHwnd = taskWindow.hwnd;
    await hidePreview();

    try {
      await activateTaskWindow(taskWindow.hwnd, taskWindow.isActive);
      taskbarMessage = taskWindow.isActive && !taskWindow.isMinimized
        ? `Minimized ${taskWindowLabel(taskWindow)}`
        : `Focused ${taskWindowLabel(taskWindow)}`;
      await refreshTaskbarWindows();
    } catch (error) {
      console.error(`Failed to toggle task window ${taskWindow.hwnd}`, error);
      taskbarMessage = `Window toggle unavailable for ${taskWindowLabel(taskWindow)}`;
      await refreshTaskbarWindows();
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
  function taskGroupStyle(group: TaskWindowGroup) {
    const previewOrderIndex = taskGroupPreviewOrder.indexOf(group.key);
    if (draggingGroupKey !== group.key || !taskGroupDragStarted) {
      return previewOrderIndex >= 0 ? `order: ${previewOrderIndex};` : '';
    }

    const liveReorderOffset = taskbarGroupReorderOffset(
      group.key,
      taskGroupPreviewOrder,
      taskGroupDragRects
    );
    const visualDelta = taskGroupDragDeltaX + liveReorderOffset;
    return draggingGroupKey === group.key && taskGroupDragStarted
      ? `order: ${previewOrderIndex}; transform: translate3d(${visualDelta}px, -1px, 0); z-index: 2;`
      : (previewOrderIndex >= 0 ? `order: ${previewOrderIndex};` : '');
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
  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && taskGroupDragPointerId !== null) {
      event.preventDefault();
      cancelTaskGroupPointerDrag();
    }
  }
  function updateTaskbarOverflow() {
    if (!taskStripEl) {
      taskbarOverflow = taskbarOverflowState(0, 0, taskWindowGroups.length);
      return;
    }
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
    void Promise.all([refreshLauncherSections(), refreshTaskbarWindows()]);

    registerAsyncUnlistener(listen(TASKBAR_REFRESH_WINDOWS_EVENT, () => {
      void refreshTaskbarWindows();
    }));

    registerAsyncUnlistener(listen(TASKBAR_REFRESH_LAUNCHERS_EVENT, () => {
      void refreshLauncherSections();
    }));

    registerAsyncUnlistener(listen(TASK_PREVIEW_HOVER_ENTER_EVENT, () => {
      clearPreviewHideTimer();
    }));

    const taskbarPollTimer = window.setInterval(() => {
      void refreshTaskbarWindows();
    }, 1_000);
    const resizeHandler = () => updateTaskbarOverflow();
    window.addEventListener('resize', resizeHandler);

    const runtimeMetricsTimer = window.setTimeout(() => {
      void reportShellSurfaceRuntimeMetrics('bottom-bar').catch((error) => {
        console.error('Bottom bar runtime metrics failed', error);
      });
    }, 250);

    return () => {
      disposed = true;
      clearPreviewShowTimer();
      clearPreviewHideTimer();
      const requestId = nextPreviewRequestId();
      void hideTaskWindowPreview(requestId).catch(() => undefined);
      window.clearInterval(taskbarPollTimer);
      window.clearTimeout(runtimeMetricsTimer);
      window.removeEventListener('resize', resizeHandler);
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<div class="surface bottom-bar">
  <section class="taskbar-strip" aria-label="Taskbar">
    <div class="launcher-strip quick-icon-strip" aria-label="Pinned quick icons">
      {#if quickIcons.length}
        {#each quickIcons as icon (icon.id)}
          <span class="quick-icon-slot">
            <MeltActionButton
              class="launcher-button quick-icon-button"
              type="button"
              title={icon.name}
              ariaLabel={`Launch ${icon.name}`}
              disabled={launchingQuickIconId === icon.id}
              onClick={() => void launchQuickIconFromBottomBar(icon)}
              onContextMenu={(event) => void openQuickIconMenu(icon, event)}
            >
              <img class="launcher-icon" src={icon.iconDataUrl} alt="" draggable="false" />
            </MeltActionButton>
            {#if quickIconLaunchErrors[icon.id]}
              <span
                class="quick-icon-error"
                role="status"
                aria-live="polite"
                aria-label={quickIconLaunchErrors[icon.id].message}
              >
                <span aria-hidden="true">!</span>
                <span class="quick-icon-error-text">{quickIconLaunchErrors[icon.id].message}</span>
              </span>
            {/if}
          </span>
        {/each}
      {:else}
        <div class="strip-fallback">{quickIconMessage}</div>
      {/if}
    </div>

    <div class="launcher-strip" aria-label="Pinned Explorer taskbar apps">
      {#if launchers.length}
        {#each launchers as launcher (launcher.id)}
          <MeltActionButton
            class="launcher-button"
            type="button"
            title={launcher.name}
            ariaLabel={`Launch ${launcher.name}`}
            disabled={launchingShortcutPath === launcher.shortcutPath}
            onClick={() => void launchApp(launcher)}
            onContextMenu={(event) => void openLauncherMenu(launcher, event)}
          >
            <img class="launcher-icon" src={launcher.iconDataUrl} alt="" draggable="false" />
          </MeltActionButton>
        {/each}
      {:else}
        <div class="strip-fallback">{launcherMessage}</div>
      {/if}
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
            class:task-group-busy={group.isBusy}
            class:task-group-minimized={group.isMinimized}
            class:task-group-dragging={draggingGroupKey === group.key}
            class:task-group-drop-target={dropTargetGroupKey === group.key && draggingGroupKey !== group.key}
            class="task-group"
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
            {#if group.windows.length > 1}
              <span class="task-count" aria-label={`${group.windows.length} windows`}>{group.windows.length}</span>
            {/if}
            {#each group.windows as taskWindow (taskWindow.hwnd)}
              <MeltActionButton
                class={`task-button${taskWindow.isActive ? ' task-button-active' : ''}${taskWindow.isMinimized ? ' task-button-minimized' : ''}`}
                type="button"
                title={taskWindowLabel(taskWindow)}
                ariaLabel={taskWindowActionLabel(taskWindow)}
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
