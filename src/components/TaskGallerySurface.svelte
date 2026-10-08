<script lang="ts">
  import { emit, listen } from '@tauri-apps/api/event';
  import { onMount, tick } from 'svelte';
  import {
    activateTaskGalleryWindow,
    closeTaskGalleryWindow,
    hideTaskGallery as hideTaskGalleryNative,
    hideTaskGalleryOnFocusLoss,
    nextTaskGalleryFocusIndex,
    reconcileTaskGalleryFocus,
    syncTaskGalleryThumbnails,
    type TaskGalleryPayload,
    type TaskGalleryThumbnail,
    type TaskGalleryThumbnailSync
  } from '../lib/taskGallery';
  import { taskGalleryLayout, TASK_GALLERY_GEOMETRY } from '../lib/taskGalleryLayout';
  import ContextMenu from './ContextMenu.svelte';
  import ContextMenuItem from './ContextMenuItem.svelte';
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import OverflowTicker from './OverflowTicker.svelte';
  import type { TaskbarWindow } from '../lib/taskbarWindows';
  import {
    TASK_PREVIEW_HIDE_DELAY_MS,
    TASKBAR_REFRESH_WINDOWS_EVENT,
    TASK_PREVIEW_HIDE_REQUEST_EVENT,
    TASK_PREVIEW_HOVER_ENTER_EVENT,
    type TaskPreviewHoverEnter,
    type TaskPreviewHideRequest
  } from '../lib/taskbarUi';
  import { positionScrollableContextMenuInViewport } from '../lib/contextMenuPosition';
  import './TaskGallerySurface.css';

  let payload: TaskGalleryPayload | null = null;
  let focusedHwnd: string | null = null;
  let focusedIndex = -1;
  let rowButtons: HTMLButtonElement[] = [];
  let panelElement: HTMLDivElement | null = null;
  let viewportElement: HTMLDivElement | null = null;
  let activeNonce: string | null = null;
  let disposed = false;
  let galleryHoverCloseTimer: number | null = null;
  let contextTask: { item: TaskbarWindow; x: number; y: number } | null = null;
  type ContextIntent = { request: number; nonce: string; revision: number; epoch: number; publication: number;
    item: TaskbarWindow; x: number; y: number };
  let pendingContextTask: ContextIntent | null = null;
  let contextRequestId = 0;
  let contextDiagnostic = '';
  let contextMenuElement: HTMLDivElement | null = null;
  let contextMenuPositioned = false;
  let contextMenuPlacement = { x: 0, y: 0, maxWidth: 128, maxHeight: 86 };
  let sessionEpoch = 0;
  let publicationEpoch = 0;
  let galleryClosing = false;
  let pendingCloseHwnds = new Set<string>();
  let closeDiagnostic = '';
  let thumbnailDiagnostic = '';
  let thumbnails = new Map<string, TaskGalleryThumbnail>();
  type CapturedImage = { imageDataUrl: string; nonce: string; epoch: number; requestId: number };
  let capturedImages = new Map<string, CapturedImage>();
  let visibleLiveHwnds = new Set<string>();
  // IDs never reset on snapshot/session replacement: older replies cannot collide.
  let requestId = 0;
  let geometryEpoch = 0;
  let syncFrame: number | null = null;
  let resizeObserver: ResizeObserver | null = null;

  $: galleryItems = payload?.windows ?? [];
  $: layout = payload?.layout ?? taskGalleryLayout(galleryItems.length);
  $: focusedIndex = galleryItems.findIndex((item) => item.hwnd === focusedHwnd);
  $: layoutStyle = `--gallery-columns:${layout.columns}; --gallery-card-width:${layout.cardWidth}px; --gallery-card-height:${layout.cardHeight}px; --gallery-thumbnail-width:${layout.thumbnailWidth}px; --gallery-thumbnail-height:${layout.thumbnailHeight}px; --gallery-footer-height:${layout.footerHeight}px; --gallery-viewport-width:${layout.viewportWidth}px; --gallery-viewport-height:${layout.viewportHeight}px;`;

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
      // Accepted WM_CLOSE is not proof of destruction: only snapshots remove cards.
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
    if (contextTask || pendingContextTask) return;
    galleryHoverCloseTimer = window.setTimeout(() => {
      galleryHoverCloseTimer = null;
      void closeTaskGallery();
    }, TASK_PREVIEW_HIDE_DELAY_MS);
  }

  function cancelThumbnailSync() {
    geometryEpoch += 1;
    if (syncFrame !== null) window.cancelAnimationFrame(syncFrame);
    syncFrame = null;
  }

  function thumbnailRequest(empty = false): TaskGalleryThumbnailSync | null {
    if (!payload || !viewportElement || requestId >= Number.MAX_SAFE_INTEGER) return null;
    const rect = viewportElement.getBoundingClientRect();
    const viewport = {
      left: rect.left + viewportElement.clientLeft,
      top: rect.top + viewportElement.clientTop,
      width: viewportElement.clientWidth,
      height: viewportElement.clientHeight
    };
    const frames: TaskGalleryThumbnailSync['frames'] = [];
    if (!empty && !pendingContextTask && !contextTask && document.visibilityState !== 'hidden') {
      for (const frame of viewportElement.querySelectorAll<HTMLElement>('[data-thumbnail-hwnd]')) {
        const bounds = frame.getBoundingClientRect();
        // Do not move/shrink a native thumbnail into a partial scroll fragment.
        if (bounds.width <= 0 || bounds.height <= 0 || bounds.left < viewport.left || bounds.top < viewport.top
          || bounds.right > viewport.left + viewport.width || bounds.bottom > viewport.top + viewport.height) continue;
        const hwnd = frame.dataset.thumbnailHwnd;
        if (!hwnd) continue;
        frames.push({ hwnd, bounds: { left: bounds.left, top: bounds.top, width: bounds.width, height: bounds.height } });
        if (frames.length === TASK_GALLERY_GEOMETRY.maxFrames) break;
      }
    }
    return { nonce: payload.nonce, revision: payload.revision, requestId: ++requestId, viewport, frames };
  }

  async function synchronizeThumbnails(empty = false) {
    if (disposed || galleryClosing || (!empty && (pendingContextTask || contextTask))) return;
    const request = thumbnailRequest(empty);
    if (!request) return;
    const epoch = sessionEpoch;
    const geometry = geometryEpoch;
    const isCurrent = () => !disposed && !galleryClosing && epoch === sessionEpoch && geometry === geometryEpoch
      && payload?.nonce === request.nonce && payload?.revision === request.revision && requestId === request.requestId;
    try {
      const result = await syncTaskGalleryThumbnails(request);
      if (!isCurrent() || !result || result.nonce !== request.nonce || result.revision !== request.revision
        || result.requestId !== request.requestId) return;
      const allowed = new Set(request.frames.map((frame) => frame.hwnd));
      const next = new Map(thumbnails);
      const captures = new Map(capturedImages);
      const live = new Set<string>();
      for (const thumbnail of result.thumbnails) {
        if (!allowed.has(thumbnail.hwnd)) continue;
        next.set(thumbnail.hwnd, thumbnail);
        if ((thumbnail.source === 'captured' || thumbnail.source === 'live') && thumbnail.imageDataUrl) {
          captures.set(thumbnail.hwnd, { imageDataUrl: thumbnail.imageDataUrl,
            nonce: request.nonce, epoch, requestId: request.requestId });
        }
        if (thumbnail.source === 'live') live.add(thumbnail.hwnd);
      }
      thumbnails = next;
      capturedImages = captures;
      visibleLiveHwnds = live;
      thumbnailDiagnostic = '';
    } catch (error) {
      if (isCurrent()) {
        thumbnailDiagnostic = 'Window previews unavailable. Window actions still work.';
        console.error('Failed to synchronize gallery thumbnails', error);
      }
    }
  }

  function handleCapturedImageError(event: Event, hwnd: string, capture: CapturedImage) {
    // Each accepted capture owns a keyed image node, even for identical URLs.
    // A detached node's delayed error must not invalidate its replacement.
    if (disposed || galleryClosing || payload?.nonce !== capture.nonce || sessionEpoch !== capture.epoch
      || capturedImages.get(hwnd) !== capture || !(event.currentTarget instanceof HTMLImageElement)
      || !viewportElement?.contains(event.currentTarget)) return;
    const captures = new Map(capturedImages);
    captures.delete(hwnd);
    capturedImages = captures;
    if (!visibleLiveHwnds.has(hwnd)) {
      const next = new Map(thumbnails);
      next.set(hwnd, { hwnd, source: 'unavailable', imageDataUrl: null,
        error: 'Captured image could not be displayed.' });
      thumbnails = next;
    }
  }

  function scheduleThumbnailSync() {
    if (disposed || galleryClosing || !payload || pendingContextTask || contextTask) return;
    geometryEpoch += 1;
    if (syncFrame !== null) return;
    syncFrame = window.requestAnimationFrame(() => {
      syncFrame = null;
      void synchronizeThumbnails();
    });
  }

  function handleViewportChange() {
    if (disposed || galleryClosing || !payload || pendingContextTask) return;
    if (contextTask) {
      void positionContextTask();
      return;
    }
    // First remove old native destinations; measure the final scroll position once per frame.
    if (syncFrame === null) void synchronizeThumbnails(true);
    scheduleThumbnailSync();
  }

  function clearNativeFrames() {
    cancelThumbnailSync();
    visibleLiveHwnds = new Set();
    const request = thumbnailRequest(true);
    if (request) void syncTaskGalleryThumbnails(request).catch(() => undefined);
  }

  async function closeTaskGallery() {
    const nonce = activeNonce;
    if (!nonce || galleryClosing) return;
    invalidateContextTask();
    clearNativeFrames();
    galleryClosing = true;
    invalidateCloseRequests();
    await hideTaskGalleryNative(nonce).catch((error) => {
      if (!disposed) console.error('Failed to hide task gallery', error);
    });
  }

  function taskGalleryTabLabel(item: TaskbarWindow) {
    const parts = [item.title, item.processName];
    if (item.isActive) parts.push('active');
    if (item.isMinimized) parts.push('minimized');
    return parts.join(', ');
  }

  function sameRetainedMember(previous: TaskbarWindow | undefined, incoming: TaskbarWindow) {
    // HWND/PID continuity is the renderer's available boundary, not proof
    // against PID reuse. Unknown/invalid PIDs never inherit images or intent.
    return previous?.hwnd === incoming.hwnd && Number.isSafeInteger(previous.processId)
      && (previous.processId ?? 0) > 0 && previous.processId === incoming.processId;
  }

  function rowClickMinimizeIfActive(item: TaskbarWindow) {
    return Boolean(item.isActive && !item.isMinimized);
  }

  async function focusRow(index: number) {
    const nextIndex = Math.min(Math.max(index, 0), galleryItems.length - 1);
    if (nextIndex < 0) return;
    focusedHwnd = galleryItems[nextIndex].hwnd;
    const hwnd = focusedHwnd;
    const epoch = sessionEpoch;
    await tick();
    if (disposed || galleryClosing || sessionEpoch !== epoch || focusedHwnd !== hwnd) return;
    const button = rowButtons[focusedIndex];
    button?.closest('.task-gallery-tile')?.scrollIntoView?.({ block: 'nearest', inline: 'nearest', behavior: 'instant' });
    button?.focus({ preventScroll: true });
    handleViewportChange();
  }

  function focusItem(item: TaskbarWindow) {
    focusedHwnd = item.hwnd;
  }

  function handleGalleryPointerEnter() {
    cancelGalleryHoverClose();
    if (!payload) return;
    void emit<TaskPreviewHoverEnter>(TASK_PREVIEW_HOVER_ENTER_EVENT, { source: 'gallery', nonce: payload.nonce });
  }

  async function activateFocused(minimizeIfActive = false) {
    if (!payload || galleryClosing || focusedIndex < 0) return;
    const item = galleryItems[focusedIndex];
    if (item) await activateTaskGalleryWindow(item.hwnd, payload.nonce, minimizeIfActive);
  }

  async function handleTaskGalleryItemClick(item: TaskbarWindow) {
    if (!payload || galleryClosing) return;
    focusedHwnd = item.hwnd;
    await activateTaskGalleryWindow(item.hwnd, payload.nonce, rowClickMinimizeIfActive(item));
  }

  function invalidateContextTask() {
    contextRequestId += 1;
    pendingContextTask = null;
    contextTask = null;
    contextDiagnostic = '';
    contextMenuPositioned = false;
  }

  function galleryClientBounds() {
    const rect = panelElement?.getBoundingClientRect();
    const measured = rect && rect.width > 0 && rect.height > 0;
    // Intersect the actual gallery host with the webview client, not fixture chrome.
    const left = Math.max(0, measured ? rect.left : 0);
    const top = Math.max(0, measured ? rect.top : 0);
    const right = Math.min(window.innerWidth, measured ? rect.right : layout.panelWidth ?? window.innerWidth);
    const bottom = Math.min(window.innerHeight, measured ? rect.bottom : layout.panelHeight ?? window.innerHeight);
    return { left, top, width: Math.max(0, right - left), height: Math.max(0, bottom - top) };
  }

  async function positionContextTask(focusFirst = false) {
    const menu = contextTask;
    const version = contextRequestId;
    const isCurrent = () => !disposed && !galleryClosing && contextTask === menu && contextRequestId === version;
    if (!menu || !isCurrent()) return;
    const bounds = galleryClientBounds();
    contextMenuPlacement = { ...contextMenuPlacement,
      maxWidth: Math.max(1, bounds.width - 16), maxHeight: Math.max(1, bounds.height - 16) };
    await tick();
    if (!isCurrent() || !contextMenuElement) return;
    const element = contextMenuElement;
    // Offset/scroll sizes measure the full menu without opening-transform distortion.
    const height = element.scrollHeight + element.offsetHeight - element.clientHeight;
    const placement = positionScrollableContextMenuInViewport(
      { x: menu.x - bounds.left, y: menu.y - bounds.top },
      { width: element.offsetWidth, height }, { width: bounds.width, height: bounds.height }
    );
    contextMenuPlacement = { x: placement.x + bounds.left, y: placement.y + bounds.top,
      maxWidth: Math.max(1, bounds.width - 16), maxHeight: Math.max(1, placement.maxHeight) };
    contextMenuPositioned = true;
    if (!focusFirst) return;
    await tick();
    if (isCurrent() && contextMenuElement === element) {
      element.querySelector<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')?.focus({ preventScroll: true });
    }
  }

  async function openContextTask(item: TaskbarWindow, x: number, y: number) {
    if (!payload || disposed || galleryClosing) return;
    cancelGalleryHoverClose();
    invalidateContextTask();
    const intent = { request: contextRequestId, nonce: payload.nonce, revision: payload.revision,
      epoch: sessionEpoch, publication: publicationEpoch, item, x, y };
    // Suppress re-registration before allocating the clear. IPC receipt alone
    // cannot authorize an overlay above the still-live native destinations.
    pendingContextTask = intent;
    await clearFramesForContext(intent);
  }

  async function clearFramesForContext(intent: ContextIntent) {
    cancelThumbnailSync();
    visibleLiveHwnds = new Set();
    const request = thumbnailRequest(true);
    const isCurrent = () => !disposed && !galleryClosing && pendingContextTask === intent
      && contextRequestId === intent.request && sessionEpoch === intent.epoch && publicationEpoch === intent.publication
      && payload?.nonce === intent.nonce && payload?.revision === intent.revision;
    try {
      if (!request) throw new Error('Thumbnail cleanup request unavailable');
      const result = await syncTaskGalleryThumbnails(request);
      if (!isCurrent()) return;
      if (!result || result.nonce !== request.nonce || result.revision !== request.revision
        || result.requestId !== request.requestId || requestId !== request.requestId
        || !Array.isArray(result.thumbnails) || result.thumbnails.length !== 0) {
        throw new Error('Thumbnail cleanup acknowledgement did not match');
      }
      pendingContextTask = null;
      contextTask = { item: intent.item, x: intent.x, y: intent.y };
      void positionContextTask(true);
    } catch (error) {
      if (!isCurrent()) return;
      pendingContextTask = null;
      contextDiagnostic = 'Could not clear window previews. Try again to open window actions.';
      console.error('Failed to clear gallery thumbnails before opening window actions', error);
      scheduleThumbnailSync();
    }
  }

  function dismissContextTask() {
    const hwnd = contextTask?.item.hwnd ?? pendingContextTask?.item.hwnd;
    invalidateContextTask();
    scheduleThumbnailSync();
    if (hwnd) {
      const index = galleryItems.findIndex((item) => item.hwnd === hwnd);
      if (index >= 0) void focusRow(index);
    }
  }

  function handleTaskGalleryItemContextMenu(event: MouseEvent, item: TaskbarWindow) {
    event.preventDefault();
    void openContextTask(item, event.clientX, event.clientY);
  }

  async function activateContextTask(minimizeIfActive: boolean) {
    const menu = contextTask;
    if (!payload || !menu) return;
    dismissContextTask();
    await activateTaskGalleryWindow(menu.item.hwnd, payload.nonce, minimizeIfActive);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (!payload || galleryClosing) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      if (contextTask || pendingContextTask) dismissContextTask();
      else void closeTaskGallery();
      return;
    }
    const target = event.target;
    if (target instanceof Element && target.closest('[role="menu"]')) return;
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      const item = galleryItems[focusedIndex];
      const rect = rowButtons[focusedIndex]?.getBoundingClientRect();
      if (item && rect) void openContextTask(item, Math.round(rect.left + rect.width / 2), Math.round(rect.top + rect.height / 2));
      return;
    }
    if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) {
      event.preventDefault();
      void focusRow(nextTaskGalleryFocusIndex(focusedIndex, galleryItems.length, event.key, layout.columns));
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
    const blurHandler = () => {
      if (disposed) return;
      invalidateContextTask();
      clearNativeFrames();
      void hideTaskGalleryOnFocusLoss();
    };
    const visibilityHandler = () => {
      if (document.visibilityState === 'hidden') {
        invalidateContextTask();
        clearNativeFrames();
      } else scheduleThumbnailSync();
    };
    window.addEventListener('blur', blurHandler);
    window.addEventListener('resize', handleViewportChange);
    document.addEventListener('visibilitychange', visibilityHandler);
    if (typeof ResizeObserver !== 'undefined') resizeObserver = new ResizeObserver(handleViewportChange);
    const unlistenOpen = listen<TaskGalleryPayload>('task-gallery:open', async (event) => {
      const incoming = event.payload;
      const sameNonce = activeNonce === incoming.nonce;
      if (disposed || (sameNonce && galleryClosing)) return;
      // New sessions may start with smaller revisions; only compare within a nonce.
      const incomingRevision = Number.isSafeInteger(incoming.revision) && incoming.revision > 0
        ? incoming.revision : (sameNonce ? (payload?.revision ?? 0) + 1 : 1);
      if (sameNonce && incomingRevision <= (payload?.revision ?? 0)) return;
      const incomingLayout = incoming.layout ?? taskGalleryLayout(incoming.windows.length);
      const previousMembers = new Map(payload?.windows.map((item) => [item.hwnd, item]) ?? []);
      const retainedMembers = new Set(sameNonce ? incoming.windows
        .filter((item) => sameRetainedMember(previousMembers.get(item.hwnd), item)).map((item) => item.hwnd) : []);
      const geometryChanged = !sameNonce || JSON.stringify(payload?.layout) !== JSON.stringify(incomingLayout)
        || payload?.windows.map((item) => item.hwnd).join(',') !== incoming.windows.map((item) => item.hwnd).join(',')
        || retainedMembers.size !== incoming.windows.length;
      const contextItem = contextTask?.item ?? pendingContextTask?.item;
      const survivingContext = sameNonce && incoming.windows.find((item) => sameRetainedMember(contextItem, item));
      if (!survivingContext) invalidateContextTask();
      // Invalidate old replies without dropping unchanged native destinations.
      cancelThumbnailSync();
      if (!sameNonce) clearNativeFrames();
      const publication = ++publicationEpoch;
      const focusedControl = document.activeElement;
      const focusedTile = focusedControl instanceof Element ? focusedControl.closest('[data-gallery-hwnd]') : null;
      const removedFocusedControl = sameNonce && Boolean(focusedTile && panelElement?.contains(focusedTile)
        && !incoming.windows.some((item) => item.hwnd === focusedTile.getAttribute('data-gallery-hwnd')));
      if (!sameNonce) {
        invalidateCloseRequests();
        galleryClosing = false;
        contextTask = null;
        thumbnails = new Map();
        capturedImages = new Map();
        thumbnailDiagnostic = '';
        cancelGalleryHoverClose();
      } else if (contextTask && survivingContext) {
        contextTask = { ...contextTask, item: survivingContext };
      }
      thumbnails = new Map([...thumbnails].filter(([hwnd]) => retainedMembers.has(hwnd)));
      capturedImages = new Map([...capturedImages].filter(([hwnd]) => retainedMembers.has(hwnd)));
      visibleLiveHwnds = new Set([...visibleLiveHwnds].filter((hwnd) => retainedMembers.has(hwnd)));
      payload = { ...incoming, revision: incomingRevision,
        layout: incomingLayout };
      activeNonce = incoming.nonce;
      if (pendingContextTask && survivingContext) {
        const intent = { ...pendingContextTask, item: survivingContext,
          revision: incomingRevision, publication };
        pendingContextTask = intent;
        void clearFramesForContext(intent);
      } else if (sameNonce && geometryChanged && !contextTask) clearNativeFrames();
      const next = reconcileTaskGalleryFocus(sameNonce ? focusedHwnd : null, incoming.windows);
      focusedHwnd = next.focusedHwnd;
      await tick();
      if (disposed || activeNonce !== incoming.nonce || publicationEpoch !== publication) return;
      // Re-observing an unchanged viewport emits a synthetic initial resize and
      // would clear the native frames on every taskbar snapshot refresh.
      if (!sameNonce) {
        resizeObserver?.disconnect();
        if (viewportElement) resizeObserver?.observe(viewportElement);
      }
      // Keyed cards preserve surviving activation/Close focus, including mode changes.
      if (removedFocusedControl && document.hasFocus() && document.activeElement === document.body) {
        if (focusedIndex >= 0) void focusRow(focusedIndex);
        else panelElement?.focus();
      }
      if (!sameNonce && incoming.focusGallery) panelElement?.focus();
      if (contextTask) void positionContextTask();
      else scheduleThumbnailSync();
    });
    const unlistenClosed = listen<{ nonce: string | null }>('task-gallery:closed', (event) => {
      if (activeNonce && event.payload.nonce && event.payload.nonce !== activeNonce) return;
      invalidateContextTask();
      clearNativeFrames();
      invalidateCloseRequests();
      publicationEpoch += 1;
      resizeObserver?.disconnect();
      contextTask = null;
      cancelGalleryHoverClose();
      payload = null;
      activeNonce = null;
      focusedHwnd = null;
      rowButtons = [];
      thumbnails = new Map();
      capturedImages = new Map();
      thumbnailDiagnostic = '';
    });
    const unlistenPreviewHide = listen<TaskPreviewHideRequest>(TASK_PREVIEW_HIDE_REQUEST_EVENT, (event) => {
      // The local action menu owns hover lifetime; blur/closed still invalidate it.
      if (contextTask || pendingContextTask) {
        cancelGalleryHoverClose();
        return;
      }
      if (event.payload.mode === 'immediate') {
        if (!event.payload.preserveGallery) void closeTaskGallery();
      } else scheduleGalleryHoverClose();
    });
    return () => {
      invalidateContextTask();
      clearNativeFrames();
      disposed = true;
      invalidateCloseRequests();
      cancelGalleryHoverClose();
      resizeObserver?.disconnect();
      window.removeEventListener('blur', blurHandler);
      window.removeEventListener('resize', handleViewportChange);
      document.removeEventListener('visibilitychange', visibilityHandler);
      void hideTaskGalleryNative(activeNonce).catch(() => undefined);
      for (const unlisten of [unlistenOpen, unlistenClosed, unlistenPreviewHide]) {
        void unlisten.then((fn) => fn()).catch(() => undefined);
      }
    };
  });
</script>

<svelte:window on:keydown={handleKeydown} />

{#if payload}
<div bind:this={panelElement} class="task-gallery-panel surface" class:task-gallery-matrix={layout.mode === 'matrix'}
  data-gallery-mode={layout.mode} style={layoutStyle} role="dialog" aria-modal="false" aria-label="Task window gallery"
  tabindex="0" on:pointerenter={handleGalleryPointerEnter} on:pointerleave={scheduleGalleryHoverClose}>
  <div class="task-gallery-header">
    <span class="task-gallery-app-label">{payload.label}</span>
    <span class="task-gallery-count">{galleryItems.length} windows</span>
    {#if contextDiagnostic || closeDiagnostic || thumbnailDiagnostic}
      <span class="task-gallery-status" role={contextDiagnostic ? 'alert' : 'status'}
        title={contextDiagnostic || closeDiagnostic || thumbnailDiagnostic}>{contextDiagnostic || closeDiagnostic || thumbnailDiagnostic}</span>
    {/if}
  </div>
  <div bind:this={viewportElement} class="task-gallery-viewport" on:scroll={handleViewportChange}>
    <div class="task-gallery-strip" role="group" aria-label={payload.label}>
      {#each galleryItems as item, index (item.hwnd)}
        <div class="task-gallery-tile" role="group" aria-label={taskGalleryTabLabel(item)} data-gallery-hwnd={item.hwnd}
          class:focused={index === focusedIndex} class:active={item.isActive} class:minimized={item.isMinimized}>
          <button bind:this={rowButtons[index]} type="button" class="task-gallery-activate"
            aria-label={`Activate ${taskGalleryTabLabel(item)}`} title={item.title} tabindex={index === focusedIndex ? 0 : -1}
            on:focus={() => focusItem(item)} on:click={() => void handleTaskGalleryItemClick(item)}
            on:contextmenu={(event) => handleTaskGalleryItemContextMenu(event, item)}>
            <span class="task-gallery-thumbnail" data-thumbnail-hwnd={item.hwnd}>
              {#if capturedImages.get(item.hwnd)}
                {@const capture = capturedImages.get(item.hwnd)!}
                {#key capture}
                  <img class="task-gallery-captured-image" src={capture.imageDataUrl} alt="" draggable="false"
                    on:error={(event) => handleCapturedImageError(event, item.hwnd, capture)} />
                {/key}
              {:else}
                <span class="task-gallery-placeholder">{visibleLiveHwnds.has(item.hwnd) ? 'Live preview' : 'Preview unavailable'}</span>
              {/if}
            </span>
            <span class="task-gallery-caption">
              {#if item.iconDataUrl}<img class="task-gallery-app-icon" src={item.iconDataUrl} alt="" draggable="false"
                on:error={(event) => { if (event.currentTarget instanceof HTMLImageElement) event.currentTarget.style.visibility = 'hidden'; }} />{/if}
              <span class="task-gallery-copy"><span class="task-gallery-tab-title"><OverflowTicker text={item.title} /></span>
                <span class="task-gallery-window-state">{item.isActive ? 'Active' : item.isMinimized ? 'Minimized' : 'Open'}
                  {#if capturedImages.has(item.hwnd) && !visibleLiveHwnds.has(item.hwnd)} · Captured{/if}</span>
              </span>
            </span>
          </button>
          <button type="button" class="task-gallery-close" aria-label={`Close ${item.title}`}
            aria-disabled={pendingCloseHwnds.has(item.hwnd)} aria-busy={pendingCloseHwnds.has(item.hwnd)} title={`Close ${item.title}`}
            tabindex={index === focusedIndex ? 0 : -1} on:focus={() => focusItem(item)}
            on:click={() => { focusedHwnd = item.hwnd; void requestWindowClose(item); }}
            on:contextmenu={(event) => handleTaskGalleryItemContextMenu(event, item)}><MaterialSymbolIcon name="close" /></button>
        </div>
      {/each}
    </div>
  </div>
  {#if contextTask}
    <ContextMenu bind:element={contextMenuElement} className="task-gallery-context-menu" ariaLabel={`${contextTask.item.title} actions`}
      style={`left:${contextMenuPlacement.x}px; top:${contextMenuPlacement.y}px; --gallery-menu-max-width:${contextMenuPlacement.maxWidth}px; --gallery-menu-max-height:${contextMenuPlacement.maxHeight}px; visibility:${contextMenuPositioned ? 'visible' : 'hidden'};`}
      on:click={dismissContextTask}>
      <ContextMenuItem icon="preview" onClick={() => void activateContextTask(false)}>Restore / Switch</ContextMenuItem>
      <ContextMenuItem icon={null} onClick={() => void activateContextTask(true)}>Minimize</ContextMenuItem>
    </ContextMenu>
  {/if}
</div>
{/if}
