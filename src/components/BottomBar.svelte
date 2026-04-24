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
    TASKBAR_REFRESH_LAUNCHERS_EVENT,
    TASKBAR_REFRESH_WINDOWS_EVENT,
    TASK_PREVIEW_DELAY_MS,
    TASK_PREVIEW_HIDE_DELAY_MS,
    TASK_PREVIEW_HOVER_ENTER_EVENT,
    taskWindowActionLabel,
    taskWindowLabel
  } from '../lib/taskbarUi';
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
      openWindows = await listOpenTaskWindows();
      taskbarMessage = openWindows.length ? 'Open task windows' : 'No open task windows';
    } catch (error) {
      console.error('Failed to load open task windows', error);
      openWindows = [];
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
      {#if openWindows.length}
        {#each openWindows as taskWindow (taskWindow.hwnd)}
          <button
            class:task-button-active={taskWindow.isActive}
            class:task-button-minimized={taskWindow.isMinimized}
            class="task-button"
            type="button"
            title={taskWindowLabel(taskWindow)}
            aria-label={taskWindowActionLabel(taskWindow)}
            disabled={activatingHwnd === taskWindow.hwnd}
            on:click={() => void toggleWindow(taskWindow)}
            on:mouseenter={(event) => queuePreview(taskWindow, event)}
            on:mouseleave={schedulePreviewHide}
            on:contextmenu={(event) => void openTaskMenu(taskWindow, event)}
          >
            <img class="task-icon" src={taskWindow.iconDataUrl} alt="" draggable="false" />
            <span class="task-label">{taskWindowLabel(taskWindow)}</span>
          </button>
        {/each}
      {:else}
        <div class="strip-fallback">{taskbarMessage}</div>
      {/if}
    </div>
  </section>
</div>
