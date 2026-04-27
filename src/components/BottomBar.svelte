<script lang="ts">
  import './BottomBar.css';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { reportShellSurfaceRuntimeMetrics } from '../lib/runtimeMetrics';
  import {
    launchPinnedTaskbarLauncher,
    listPinnedTaskbarLaunchers,
    type PinnedTaskbarLauncher
  } from '../lib/taskbarLaunchers';
  import {
    showLauncherContextMenu,
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
  let launcherMessage = 'Loading Explorer taskbar pins…';
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
    } catch (error) {
      console.error('Failed to load open task windows', error);
      openWindows = [];
      taskGroupOrder = [];
      taskbarMessage = 'Open windows unavailable';
    }
  }
  async function loadPinnedLaunchers() {
    launcherMessage = 'Loading Explorer taskbar pins…';
    try {
      launchers = await listPinnedTaskbarLaunchers();
      launcherMessage = launchers.length
        ? 'Pinned Explorer shortcuts'
        : 'No supported Explorer taskbar pins';
    } catch (error) {
      console.error('Failed to load pinned taskbar launchers', error);
      launchers = [];
      launcherMessage = 'Pinned taskbar shortcuts unavailable';
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
    return group.windows.length > 1
      ? `${group.label} (${group.windows.length} windows)`
      : group.label;
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
  onMount(() => {
    const unlisteners: Array<() => void> = [];
    void Promise.all([loadPinnedLaunchers(), refreshTaskbarWindows()]);

    void listen(TASKBAR_REFRESH_WINDOWS_EVENT, () => {
      void refreshTaskbarWindows();
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });

    void listen(TASKBAR_REFRESH_LAUNCHERS_EVENT, () => {
      void loadPinnedLaunchers();
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });

    void listen(TASK_PREVIEW_HOVER_ENTER_EVENT, () => {
      clearPreviewHideTimer();
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });

    const taskbarPollTimer = window.setInterval(() => {
      void refreshTaskbarWindows();
    }, 1_000);

    const runtimeMetricsTimer = window.setTimeout(() => {
      void reportShellSurfaceRuntimeMetrics('bottom-bar').catch((error) => {
        console.error('Bottom bar runtime metrics failed', error);
      });
    }, 250);

    return () => {
      clearPreviewShowTimer();
      clearPreviewHideTimer();
      const requestId = nextPreviewRequestId();
      void hideTaskWindowPreview(requestId).catch(() => undefined);
      window.clearInterval(taskbarPollTimer);
      window.clearTimeout(runtimeMetricsTimer);
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<div class="surface bottom-bar">
  <section class="taskbar-strip" aria-label="Taskbar">
    <div class="launcher-strip" aria-label="Pinned Explorer taskbar apps">
      {#if launchers.length}
        {#each launchers as launcher (launcher.id)}
          <button
            class="launcher-button"
            type="button"
            title={launcher.name}
            aria-label={`Launch ${launcher.name}`}
            disabled={launchingShortcutPath === launcher.shortcutPath}
            on:click={() => void launchApp(launcher)}
            on:contextmenu={(event) => void openLauncherMenu(launcher, event)}
          >
            <img class="launcher-icon" src={launcher.iconDataUrl} alt="" draggable="false" />
          </button>
        {/each}
      {:else}
        <div class="strip-fallback">{launcherMessage}</div>
      {/if}
    </div>

    <div class="task-strip" aria-label="Open windows">
      {#if taskWindowGroups.length}
        {#each taskWindowGroups as group (group.key)}
          <div
            class:task-group-active={group.isActive}
            class:task-group-busy={group.isBusy}
            class:task-group-dragging={draggingGroupKey === group.key}
            class:task-group-drop-target={dropTargetGroupKey === group.key && draggingGroupKey !== group.key}
            class="task-group"
            data-task-group-key={group.key}
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
              <button
                class:task-button-active={taskWindow.isActive}
                class:task-button-minimized={taskWindow.isMinimized}
                class="task-button"
                type="button"
                title={taskWindowLabel(taskWindow)}
                aria-label={taskWindowActionLabel(taskWindow)}
                disabled={activatingHwnd === taskWindow.hwnd}
                on:pointerdown={(event) => handleTaskWindowPointerDown(taskWindow, event)}
                on:click={(event) => handleTaskWindowClick(taskWindow, event)}
                on:mouseenter={(event) => queuePreview(taskWindow, event)}
                on:mouseleave={schedulePreviewHide}
                on:contextmenu={(event) => void openTaskMenu(taskWindow, event)}
              >
                <img class="task-icon" src={taskWindow.iconDataUrl} alt="" draggable="false" />
                <span class="task-label">{taskWindowLabel(taskWindow)}</span>
              </button>
            {/each}
          </div>
        {/each}
      {:else}
        <div class="strip-fallback">{taskbarMessage}</div>
      {/if}
    </div>
  </section>
</div>
