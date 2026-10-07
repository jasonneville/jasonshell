<script lang="ts">
  import { emit, listen } from '@tauri-apps/api/event';
  import { onMount, tick } from 'svelte';
  import {
    activateTaskGalleryWindow,
    closeTaskGalleryWindow,
    hideTaskGallery as hideTaskGalleryNative,
    hideTaskGalleryOnFocusLoss,
    hideTaskGalleryWindowPreview,
    showTaskGalleryWindowPreview
  } from '../lib/taskGallery';
  import ContextMenu from './ContextMenu.svelte';
  import ContextMenuItem from './ContextMenuItem.svelte';
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import { nextTaskGalleryFocusIndex, reconcileTaskGalleryFocus } from '../lib/taskGallery';
  import type { TaskbarWindow } from '../lib/taskbarWindows';
  import { allocateTaskPreviewRequestId } from '../lib/taskbarPreview';
  import {
    TASK_PREVIEW_HIDE_DELAY_MS,
    TASKBAR_REFRESH_WINDOWS_EVENT,
    TASK_PREVIEW_HIDE_REQUEST_EVENT,
    TASK_PREVIEW_HOVER_ENTER_EVENT,
    type TaskPreviewHoverEnter,
    type TaskPreviewHideRequest
  } from '../lib/taskbarUi';

  type TaskGalleryPayload = { nonce: string; groupKey: string; label: string; focusGallery: boolean; windows: TaskbarWindow[] };

  let payload: TaskGalleryPayload | null = null;
  let focusedHwnd: string | null = null;
  let focusedIndex = -1;
  let rowButtons: HTMLButtonElement[] = [];
  let panelElement: HTMLDivElement | null = null;
  let currentPreviewHwnd: string | null = null;
  let currentPreviewRequestId = 0;
  let activeNonce: string | null = null;
  let disposed = false;
  let galleryHoverCloseTimer: number | null = null;
  let contextTask: { item: TaskbarWindow; x: number; y: number } | null = null;
  let sessionEpoch = 0;
  let snapshotRevision = 0;
  let galleryClosing = false;
  let pendingCloseHwnds = new Set<string>();
  let closeDiagnostic = '';

  function invalidateCloseRequests() {
    sessionEpoch += 1;
    pendingCloseHwnds = new Set();
    closeDiagnostic = '';
  }

  async function requestWindowClose(item: TaskbarWindow) {
    if (!payload || disposed || galleryClosing || pendingCloseHwnds.has(item.hwnd)) return;
    const nonce = payload.nonce;
    const hwnd = item.hwnd;
    const epoch = sessionEpoch;
    const isCurrent = () => !disposed && !galleryClosing && sessionEpoch === epoch && payload?.nonce === nonce;
    pendingCloseHwnds = new Set([...pendingCloseHwnds, hwnd]);
    closeDiagnostic = '';
    try {
      await closeTaskGalleryWindow(hwnd, nonce);
      // Accepted WM_CLOSE is not proof of destruction: only snapshots remove rows.
      if (isCurrent()) await emit(TASKBAR_REFRESH_WINDOWS_EVENT);
    } catch (error) {
      if (isCurrent()) {
        closeDiagnostic = `Could not request close for ${item.title}. Try Close again.`;
        console.error(`Failed to close gallery window ${hwnd}`, error);
      }
    } finally {
      if (isCurrent()) pendingCloseHwnds = new Set([...pendingCloseHwnds].filter((target) => target !== hwnd));
    }
  }

  function cancelGalleryHoverClose() {
    if (galleryHoverCloseTimer === null) return;
    window.clearTimeout(galleryHoverCloseTimer);
    galleryHoverCloseTimer = null;
  }

  function scheduleGalleryHoverClose() {
    cancelGalleryHoverClose();
    galleryHoverCloseTimer = window.setTimeout(() => {
      galleryHoverCloseTimer = null;
      void closeTaskGallery();
    }, TASK_PREVIEW_HIDE_DELAY_MS);
  }

  function taskGalleryTabLabel(item: TaskbarWindow) {
    const parts = [item.title, item.processName];
    if (item.isActive) parts.push('active');
    if (item.isMinimized) parts.push('minimized');
    return parts.join(', ');
  }

  function rowClickMinimizeIfActive(item: TaskbarWindow) {
    return Boolean(item.isActive && !item.isMinimized);
  }

  $: galleryItems = payload?.windows ?? [];
  $: focusedIndex = galleryItems.findIndex((item) => item.hwnd === focusedHwnd);

  async function focusRow(index: number) {
    const nextIndex = Math.min(Math.max(index, 0), galleryItems.length - 1);
    if (nextIndex < 0) return;
    focusedIndex = nextIndex;
    focusedHwnd = galleryItems[nextIndex].hwnd;
    const hwnd = focusedHwnd;
    const epoch = sessionEpoch;
    await tick();
    if (!disposed && sessionEpoch === epoch && focusedHwnd === hwnd) rowButtons[focusedIndex]?.focus();
  }

  function focusItem(item: TaskbarWindow, control: HTMLButtonElement) {
    focusedHwnd = item.hwnd;
    void queuePreview(item, control.parentElement);
  }

  async function queuePreview(item: TaskbarWindow, anchor?: HTMLElement | null) {
    if (!payload || currentPreviewHwnd === item.hwnd || disposed) return;
    const nonce = payload.nonce;
    try {
      const requestId = await allocateTaskPreviewRequestId();
      if (disposed || payload?.nonce !== nonce) return;
      currentPreviewHwnd = item.hwnd;
      currentPreviewRequestId = requestId;
      const rect = anchor?.getBoundingClientRect();
      await showTaskGalleryWindowPreview({
        nonce,
        requestId,
        hwnd: item.hwnd,
        title: item.title,
        processName: item.processName,
        iconDataUrl: item.iconDataUrl,
        isMinimized: Boolean(item.isMinimized),
        anchorLeft: rect?.left ?? 0,
        anchorWidth: rect?.width ?? 0
      });
    } catch (error) {
      if (!disposed && payload?.nonce === nonce) {
        currentPreviewHwnd = null;
        currentPreviewRequestId = 0;
        console.error(`Failed to show gallery preview for ${item.hwnd}`, error);
      }
    }
  }

  async function closePreview() {
    if (!payload || !currentPreviewHwnd || disposed) return;
    const hwnd = currentPreviewHwnd;
    const nonce = payload.nonce;
    try {
      const requestId = await allocateTaskPreviewRequestId();
      if (disposed || payload?.nonce !== nonce || currentPreviewHwnd !== hwnd) return;
      currentPreviewHwnd = null;
      currentPreviewRequestId = 0;
      await hideTaskGalleryWindowPreview({ nonce, requestId, hwnd });
    } catch (error) {
      if (!disposed) console.error('Failed to hide gallery preview', error);
    }
  }

  async function closeTaskGallery() {
    const nonce = activeNonce;
    galleryClosing = true;
    invalidateCloseRequests();
    try {
      await closePreview();
    } finally {
      await hideTaskGalleryNative(nonce).catch((error) => {
        if (!disposed) console.error('Failed to hide task gallery', error);
      });
    }
  }

  function handleGalleryPointerEnter() {
    cancelGalleryHoverClose();
    if (!payload) return;
    void emit<TaskPreviewHoverEnter>(TASK_PREVIEW_HOVER_ENTER_EVENT, { source: 'gallery', nonce: payload.nonce });
  }

  async function activateFocused(minimizeIfActive = false) {
    if (!payload || focusedIndex < 0) return;
    const item = galleryItems[focusedIndex];
    if (!item) return;
    await closePreview();
    await activateTaskGalleryWindow(item.hwnd, payload.nonce, minimizeIfActive);
  }

  async function handleTaskGalleryItemClick(item: TaskbarWindow) {
    if (!payload) return;
    focusedHwnd = item.hwnd;
    await closePreview();
    await activateTaskGalleryWindow(item.hwnd, payload.nonce, rowClickMinimizeIfActive(item));
  }

  function handleTaskGalleryItemContextMenu(event: MouseEvent, item: TaskbarWindow) {
    event.preventDefault();
    contextTask = { item, x: event.clientX, y: event.clientY };
  }

  async function activateContextTask(minimizeIfActive: boolean) {
    const menu = contextTask;
    if (!payload || !menu) return;
    contextTask = null;
    await closePreview();
    await activateTaskGalleryWindow(menu.item.hwnd, payload.nonce, minimizeIfActive);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      void closeTaskGallery();
      return;
    }
    const target = event.target;
    if (target instanceof Element && target.closest('[role="menu"]')) return;
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      const item = galleryItems[focusedIndex];
      if (!payload || !item) return;
      const button = rowButtons[focusedIndex];
      const rect = button?.getBoundingClientRect();
      if (!rect) return;
      contextTask = { item, x: Math.round(rect.left + rect.width / 2), y: Math.round(rect.top + rect.height / 2) };
      return;
    }
    const nextIndex = nextTaskGalleryFocusIndex(focusedIndex, galleryItems.length, event.key);
    if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) {
      event.preventDefault();
      void focusRow(nextIndex);
      return;
    }
    if (target instanceof Element && target.closest('.task-gallery-close')) return;
    if ((event.key === 'Enter' || event.key === ' ') && focusedIndex >= 0) {
      event.preventDefault();
      void activateFocused(rowClickMinimizeIfActive(galleryItems[focusedIndex]));
    }
  }

  onMount(() => {
    disposed = false;
    const blurHandler = () => { if (!disposed) void hideTaskGalleryOnFocusLoss(); };
    window.addEventListener('blur', blurHandler);
    const unlistenOpen = listen<TaskGalleryPayload>('task-gallery:open', async (event: { payload: TaskGalleryPayload }) => {
      const sameNonce = activeNonce === event.payload.nonce;
      if (disposed) return;
      const revision = ++snapshotRevision;
      const focusedControl = document.activeElement;
      const focusedTile = focusedControl instanceof Element ? focusedControl.closest('[data-gallery-hwnd]') : null;
      const removedFocusedControl = sameNonce && Boolean(focusedTile && panelElement?.contains(focusedTile)
        && !event.payload.windows.some((item) => item.hwnd === focusedTile.getAttribute('data-gallery-hwnd')));
      const previewHide = sameNonce && currentPreviewHwnd && !event.payload.windows.some((item) => item.hwnd === currentPreviewHwnd)
        ? closePreview() : Promise.resolve();
      if (!sameNonce) {
        invalidateCloseRequests();
        galleryClosing = false;
        currentPreviewHwnd = null;
        currentPreviewRequestId = 0;
        contextTask = null;
        cancelGalleryHoverClose();
      } else if (contextTask && !event.payload.windows.some((item) => item.hwnd === contextTask?.item.hwnd)) {
        contextTask = null;
      }
      payload = event.payload;
      activeNonce = event.payload.nonce;
      const next = reconcileTaskGalleryFocus(sameNonce ? focusedHwnd : null, event.payload.windows);
      focusedHwnd = next.focusedHwnd;
      focusedIndex = next.focusedIndex;
      await tick();
      if (disposed || activeNonce !== event.payload.nonce || snapshotRevision !== revision) return;
      // Keyed rows retain either surviving control. Only repair genuinely removed
      // control focus, never a hover-open gallery or a different foreground window.
      if (removedFocusedControl && document.hasFocus() && document.activeElement === document.body) {
        if (focusedIndex >= 0) rowButtons[focusedIndex]?.focus();
        else panelElement?.focus();
      }
      if (!sameNonce) {
        if (event.payload.focusGallery) panelElement?.focus();
      }
      await previewHide;
    });
    const unlistenClosed = listen<{ nonce: string | null }>('task-gallery:closed', (event: { payload: { nonce: string | null } }) => {
      if (activeNonce && event.payload.nonce && event.payload.nonce !== activeNonce) return;
      invalidateCloseRequests();
      snapshotRevision += 1;
      contextTask = null;
      cancelGalleryHoverClose();
      payload = null; activeNonce = null; focusedHwnd = null; focusedIndex = -1; rowButtons = []; currentPreviewHwnd = null; currentPreviewRequestId = 0;
    });
    const unlistenPreviewEnter = listen<TaskPreviewHoverEnter>(TASK_PREVIEW_HOVER_ENTER_EVENT, (event) => {
      if (event.payload.source === 'preview') {
        cancelGalleryHoverClose();
      }
    });
    const unlistenPreviewHide = listen<TaskPreviewHideRequest>(TASK_PREVIEW_HIDE_REQUEST_EVENT, (event) => {
      if (event.payload.mode === 'immediate') {
        if (!event.payload.preserveGallery) void closeTaskGallery();
      } else {
        scheduleGalleryHoverClose();
      }
    });
    return () => { disposed = true; cancelGalleryHoverClose(); window.removeEventListener('blur', blurHandler); void hideTaskGalleryNative(activeNonce).catch(() => undefined); void unlistenOpen.then((fn: () => void) => fn()).catch(() => undefined); void unlistenClosed.then((fn: () => void) => fn()).catch(() => undefined); void unlistenPreviewEnter.then((fn: () => void) => fn()).catch(() => undefined); void unlistenPreviewHide.then((fn: () => void) => fn()).catch(() => undefined); };
  });
</script>

<svelte:window on:keydown={handleKeydown} />

{#if payload}
<div bind:this={panelElement} class="task-gallery-panel surface" role="dialog" aria-modal="false" aria-label="Task window gallery" tabindex="0" on:pointerenter={handleGalleryPointerEnter} on:pointerleave={scheduleGalleryHoverClose}>
  <div class="task-gallery-strip" role="group" aria-label={payload?.label ?? 'Task windows'}>
    {#each galleryItems as item, index (item.hwnd)}
      <div
        class="task-gallery-tile"
        role="group"
        aria-label={taskGalleryTabLabel(item)}
        data-gallery-hwnd={item.hwnd}
        class:focused={index === focusedIndex}
        class:active={item.isActive}
        class:minimized={item.isMinimized}
      >
      <button
        bind:this={rowButtons[index]}
        type="button"
        class="task-gallery-activate"
        aria-label={`Activate ${taskGalleryTabLabel(item)}`}
        title={item.title}
        tabindex={index === focusedIndex ? 0 : -1}
        on:focus={(event) => focusItem(item, event.currentTarget)}
        on:mouseenter={(event) => void queuePreview(item, event.currentTarget.parentElement)}
        on:click={() => void handleTaskGalleryItemClick(item)}
        on:contextmenu={(event) => handleTaskGalleryItemContextMenu(event, item)}
      >
        <img src={item.iconDataUrl} alt="" draggable="false" />
        <span class="task-gallery-tab-title">{item.title}</span>
      </button>
      <button
        type="button"
        class="task-gallery-close"
        aria-label={`Close ${item.title}`}
        aria-disabled={pendingCloseHwnds.has(item.hwnd)}
        aria-busy={pendingCloseHwnds.has(item.hwnd)}
        title={`Close ${item.title}`}
        tabindex={index === focusedIndex ? 0 : -1}
        on:focus={(event) => focusItem(item, event.currentTarget)}
        on:mouseenter={(event) => void queuePreview(item, event.currentTarget.parentElement)}
        on:click={() => { focusedHwnd = item.hwnd; void requestWindowClose(item); }}
        on:contextmenu={(event) => handleTaskGalleryItemContextMenu(event, item)}
      ><MaterialSymbolIcon name="close" /></button>
      </div>
    {/each}
  </div>
  <span class="task-gallery-status" role="status">{closeDiagnostic}</span>
  {#if contextTask}
    <ContextMenu
      ariaLabel={`${contextTask.item.title} actions`}
      style={`left: ${contextTask.x}px; top: ${contextTask.y}px;`}
      on:click={() => (contextTask = null)}
    >
      <ContextMenuItem icon="preview" onClick={() => void activateContextTask(false)}>Restore / Switch</ContextMenuItem>
      <ContextMenuItem icon={null} onClick={() => void activateContextTask(true)}>Minimize</ContextMenuItem>
    </ContextMenu>
  {/if}
</div>
{/if}

<style>
  .task-gallery-panel {
    background: var(--js-bg-bar);
    background-image: linear-gradient(to bottom, var(--js-color-surface-overlay), transparent);
    border: 0;
    box-shadow: inset 0 0 0 1px var(--js-color-border-soft);
    box-sizing: border-box;
    color: var(--js-color-text);
    display: flex;
    flex-direction: column;
    gap: 0;
    height: 100%;
    min-height: 24px;
    overflow: hidden;
    padding: 2px 4px;
  }
  .task-gallery-strip {
    align-items: stretch;
    display: flex;
    flex: 1 1 auto;
    gap: 3px;
    min-height: 0;
    overflow-x: auto;
    overflow-y: hidden;
    /* Keep scrolling without consuming the minimum-height control space. */
    scrollbar-width: none;
  }
  .task-gallery-strip::-webkit-scrollbar {
    display: none;
    height: 0;
  }
  .task-gallery-tile {
    align-items:center;
    background: var(--js-color-control);
    background-color: color-mix(in srgb, var(--js-bg-bar) 93%, var(--js-color-text) 7%);
    background-image: linear-gradient(to bottom, var(--js-color-surface-overlay), transparent);
    border: 1px solid var(--js-color-border);
    border-radius: 2px;
    box-shadow: var(--js-inset-highlight);
    color: inherit;
    display:flex;
    flex: 1 1 10rem;
    font-size: 0.62rem;
    font-weight: 600;
    min-width: 48px;
    min-height: 0;
    overflow:hidden;
    padding: 0;
    text-align:left;
    transition: border-color 140ms ease, background 140ms ease, box-shadow 140ms ease, opacity 140ms ease;
  }
  .task-gallery-tile:hover,
  .task-gallery-tile:focus-within,
  .task-gallery-tile.focused {
    background: var(--js-color-control-hover);
  
  }
  .task-gallery-tile.active {
    background: var(--js-bg-active);
    background-color: color-mix(in srgb, var(--js-bg-bar) 76%, var(--js-color-accent) 24%);
    background-image: linear-gradient(to bottom, var(--js-color-surface-overlay), transparent);
    border-color: var(--js-color-accent-border);
    box-shadow: var(--js-inset-highlight), inset 0 0 0 1px var(--js-color-accent-soft);
  }
  .task-gallery-tile.minimized {
    color: var(--js-color-text-muted);
  }
  .task-gallery-tile.minimized .task-gallery-activate {
    opacity: .84;
  }
  .task-gallery-activate {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: 0;
    color: inherit;
    display: flex;
    flex: 1 1 auto;
    font: inherit;
    gap: 0.28rem;
    height: 100%;
    min-width: 0;
    overflow: hidden;
    padding: 0 0.38rem;
    text-align: left;
  }
  .task-gallery-close {
    align-items: center;
    background: transparent;
    border: 0;
    border-radius: 0;
    color: var(--js-color-text);
    display: flex;
    flex: 0 0 24px;
    height: min(24px, 100%);
    justify-content: center;
    opacity: 0;
    padding: 0;
    pointer-events: none;
    width: 24px;
  }
  .task-gallery-tile:hover .task-gallery-close,
  .task-gallery-tile:focus-within .task-gallery-close {
    opacity: 1;
    pointer-events: auto;
  }
  .task-gallery-close:hover {
    background: var(--js-color-control-hover);
  }
  .task-gallery-close[aria-disabled="true"] {
    color: var(--js-color-text-muted);
    cursor: progress;
  }
  .task-gallery-activate:focus-visible,
  .task-gallery-close:focus-visible {
    outline: 2px solid var(--js-color-accent);
    outline-offset: -2px;
  }
  .task-gallery-status {
    clip-path: inset(50%);
    height: 1px;
    overflow: hidden;
    position: absolute;
    white-space: nowrap;
    width: 1px;
  }
  .task-gallery-activate img {
    display:block;
    flex: 0 0 auto;
    height: 0.74rem;
    pointer-events:none;
    width: 0.74rem;
  }
  .task-gallery-tab-title {
    display:block;
    flex:1 1 auto;
    min-width:0;
    overflow:hidden;
    text-overflow:ellipsis;
    white-space:nowrap;
    font-size: inherit;
    font-weight: inherit;
    line-height: normal;
    letter-spacing: 0;
  }
  .task-gallery-tile.active {
    position: relative;
  }
  .task-gallery-tile:focus-within,
  .task-gallery-tile.focused {
    position: relative;
  }
  @media (prefers-reduced-motion: reduce) {
    .task-gallery-tile,
    .task-gallery-activate,
    .task-gallery-close { transition: none; }
  }
</style>
