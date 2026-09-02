<script lang="ts">
  import { emit, listen } from '@tauri-apps/api/event';
  import { onMount, tick } from 'svelte';
  import {
    activateTaskGalleryWindow,
    hideTaskGallery as hideTaskGalleryNative,
    hideTaskGalleryOnFocusLoss,
    hideTaskGalleryWindowPreview,
    showTaskGalleryWindowContextMenu,
    showTaskGalleryWindowPreview
  } from '../lib/taskGallery';
  import { nextTaskGalleryFocusIndex, reconcileTaskGalleryFocus } from '../lib/taskGallery';
  import type { TaskbarWindow } from '../lib/taskbarWindows';
  import { allocateTaskPreviewRequestId } from '../lib/taskbarPreview';
  import {
    TASK_PREVIEW_HIDE_DELAY_MS,
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
    await tick();
    rowButtons[nextIndex]?.focus();
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
    void emit<TaskPreviewHoverEnter>(TASK_PREVIEW_HOVER_ENTER_EVENT, { source: 'gallery' });
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
    await closePreview();
    await activateTaskGalleryWindow(item.hwnd, payload.nonce, rowClickMinimizeIfActive(item));
  }

  function handleTaskGalleryItemContextMenu(event: MouseEvent, item: TaskbarWindow) {
    event.preventDefault();
    void showTaskGalleryWindowContextMenu({
      nonce: payload?.nonce ?? '',
      hwnd: item.hwnd,
      x: event.clientX,
      y: event.clientY
    });
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      void closeTaskGallery();
      return;
    }
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      const item = galleryItems[focusedIndex];
      if (!payload || !item) return;
      const button = rowButtons[focusedIndex];
      const rect = button?.getBoundingClientRect();
      if (!rect) return;
      void showTaskGalleryWindowContextMenu({ nonce: payload.nonce, hwnd: item.hwnd, x: Math.round(rect.left + rect.width / 2), y: Math.round(rect.top + rect.height / 2) });
      return;
    }
    const nextIndex = nextTaskGalleryFocusIndex(focusedIndex, galleryItems.length, event.key);
    if (nextIndex !== focusedIndex && ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) {
      event.preventDefault();
      void focusRow(nextIndex);
      return;
    }
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
      if (sameNonce && currentPreviewHwnd && !event.payload.windows.some((item) => item.hwnd === currentPreviewHwnd)) {
        await closePreview();
      }
      if (disposed) return;
      payload = event.payload;
      activeNonce = event.payload.nonce;
      const next = reconcileTaskGalleryFocus(sameNonce ? focusedHwnd : null, event.payload.windows);
      focusedHwnd = next.focusedHwnd;
      focusedIndex = next.focusedIndex;
      await tick();
      if (disposed || activeNonce !== event.payload.nonce) return;
      if (event.payload.focusGallery) panelElement?.focus();
    });
    const unlistenClosed = listen<{ nonce: string | null }>('task-gallery:closed', (event: { payload: { nonce: string | null } }) => {
      if (activeNonce && event.payload.nonce && event.payload.nonce !== activeNonce) return;
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
  <div class="task-gallery-strip" role="listbox" aria-orientation="horizontal" aria-label={payload?.label ?? 'Task windows'}>
    {#each galleryItems as item, index (item.hwnd)}
      <button
        bind:this={rowButtons[index]}
        role="option"
        aria-selected={index === focusedIndex}
        class:focused={index === focusedIndex}
        class:active={item.isActive}
        class:minimized={item.isMinimized}
        tabindex={index === focusedIndex ? 0 : -1}
        on:focus={(event) => { focusedHwnd = item.hwnd; void queuePreview(item, event.currentTarget); }}
        on:mouseenter={(event) => void queuePreview(item, event.currentTarget)}
        on:click={() => void handleTaskGalleryItemClick(item)}
        on:contextmenu={(event) => handleTaskGalleryItemContextMenu(event, item)}
      >
        <img src={item.iconDataUrl} alt="" draggable="false" />
        <span class="task-gallery-tab-title">{item.title}</span>
      </button>
    {/each}
  </div>
</div>
{/if}

<style>
  .task-gallery-panel {
    background: var(--js-color-surface);
    border: 1px solid var(--js-color-border-soft);
    box-sizing: border-box;
    color: var(--js-color-text);
    display: flex;
    flex-direction: column;
    gap: 0;
    height: 100%;
    overflow: hidden;
    padding: 0;
  }
  .task-gallery-strip {
    align-items: stretch;
    display: flex;
    flex: 1 1 auto;
    gap: 0;
    min-height:0;
    overflow:hidden;
  }
  .task-gallery-strip > button {
    align-items:center;
    background: var(--js-color-control);
    border: 0;
    border-left: 1px solid var(--js-color-border-soft);
    border-radius: 0;
    box-shadow: var(--js-inset-highlight);
    color: inherit;
    display:flex;
    flex: 1 1 10rem;
    font-size: 0.62rem;
    font-weight: 600;
    gap: 0.28rem;
    min-width: 0;
    min-height: 0;
    overflow:hidden;
    padding: 0 0.38rem;
    text-align:left;
    transition: border-color 140ms ease, background 140ms ease, box-shadow 140ms ease, opacity 140ms ease;
  }
  .task-gallery-strip > button:hover,
  .task-gallery-strip > button:focus-visible,
  .task-gallery-strip > button.focused {
    background: var(--js-color-control-hover);
  
  }
  .task-gallery-strip > button.active {
    background: var(--js-bg-active);
    border-color: var(--js-color-accent-border);
  }
  .task-gallery-strip > button.minimized {
    color: var(--js-color-text-muted);
    opacity: .84;
  }
  .task-gallery-strip > button img {
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
  .task-gallery-strip > button.active::before {
    background: var(--js-color-accent);
    border-radius: inherit;
    content: '';
    inset: 0 auto 0 0;
    pointer-events: none;
    position: absolute;
    width: 2px;
  }
  .task-gallery-strip > button.active {
    position: relative;
  }
  .task-gallery-strip > button:focus-visible,
  .task-gallery-strip > button.focused {
    position: relative;
  }
</style>
