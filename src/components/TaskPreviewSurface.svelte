<script lang="ts">
  import './TaskPreviewSurface.css';
  import { onMount } from 'svelte';
  import { emit, listen } from '@tauri-apps/api/event';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import {
    closePreviewedTaskWindow,
    isNativeLiveTaskPreviewPayload,
    type TaskPreviewPayload
  } from '../lib/taskbarPreview';
  import {
    TASK_PREVIEW_HIDE_REQUEST_EVENT,
    TASKBAR_REFRESH_WINDOWS_EVENT,
    TASK_PREVIEW_HOVER_ENTER_EVENT,
    type TaskPreviewHoverEnter
  } from '../lib/taskbarUi';
  import { maximizeTaskWindow } from '../lib/taskbarWindows';

  let preview: TaskPreviewPayload | null = null;
  $: isNativeLivePreview = preview ? isNativeLiveTaskPreviewPayload(preview) : false;
  $: previewSurfaceClass = `surface preview-surface${isNativeLivePreview ? ' preview-surface-native' : ''}`;
  $: previewPrimaryTitle = preview ? (preview.title || preview.processName) : '';
  $: previewSecondaryText = preview && preview.processName !== previewPrimaryTitle ? preview.processName : '';

  async function requestPreviewHide(mode: 'schedule' | 'immediate', preserveGallery = false) {
    if (mode === 'immediate') {
      preview = null;
    }

    await emit(TASK_PREVIEW_HIDE_REQUEST_EVENT, { mode, preserveGallery });
  }

  async function handlePreviewActivate() {
    if (!preview) {
      return;
    }

    try {
      await maximizeTaskWindow(preview.hwnd);
      await emit(TASKBAR_REFRESH_WINDOWS_EVENT);
      await requestPreviewHide('immediate');
    } catch (error) {
      console.error(`Failed to maximize task window ${preview.hwnd}`, error);
    }
  }

  async function handlePreviewKeydown(event: KeyboardEvent) {
    if (event.key !== 'Enter' && event.key !== ' ') {
      return;
    }

    event.preventDefault();
    await handlePreviewActivate();
  }

  function handlePreviewPointerEnter() {
    void emit<TaskPreviewHoverEnter>(TASK_PREVIEW_HOVER_ENTER_EVENT, { source: 'preview' });
  }

  async function handlePreviewPointerLeave(event: PointerEvent) {
    const root = event.currentTarget as HTMLElement | null;
    const relatedTarget = event.relatedTarget as Node | null;
    if (root && relatedTarget && root.contains(relatedTarget)) {
      return;
    }
    await requestPreviewHide('schedule');
  }

  async function handlePreviewClose(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    if (!preview) {
      return;
    }

    try {
      const preserveGallery = Boolean(preview.galleryNonce);
      await closePreviewedTaskWindow(preview.hwnd, preview.galleryNonce);
      await emit(TASKBAR_REFRESH_WINDOWS_EVENT);
      await requestPreviewHide('immediate', preserveGallery);
    } catch (error) {
      console.error(`Failed to close task window ${preview.hwnd}`, error);
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

    registerAsyncUnlistener(listen<TaskPreviewPayload>('task-preview:update', (event) => {
      preview = event.payload;
    }));

    registerAsyncUnlistener(listen('task-preview:hide', () => {
      preview = null;
    }));

    return () => {
      disposed = true;
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<div
  class="preview-interaction-root"
  role="group"
  aria-label="Task preview"
  on:pointerenter={handlePreviewPointerEnter}
  on:pointerleave={(event) => void handlePreviewPointerLeave(event)}
>
  <MeltActionButton
    class={previewSurfaceClass}
    ariaDisabled={!preview}
    ariaLabel={preview ? `Activate ${preview.title || preview.processName}` : 'Task preview unavailable'}
    onClick={() => void handlePreviewActivate()}
    onKeyDown={(event) => void handlePreviewKeydown(event)}
  >
  {#if preview}
    <div class="preview-header" aria-hidden="true">
      <div class="preview-copy">
        <div class="preview-title">{previewPrimaryTitle}</div>
        {#if previewSecondaryText}
          <div class="preview-process">{previewSecondaryText}</div>
        {/if}
      </div>
    </div>

    {#if isNativeLivePreview}
      <div class="preview-frame preview-frame-native" aria-hidden="true"></div>
    {:else if preview.imageDataUrl}
      <div class="preview-frame">
        <img
          class="preview-image"
          src={preview.imageDataUrl}
          alt={`Preview of ${preview.title || preview.processName}`}
          draggable="false"
        />
      </div>
    {:else}
      <div class="preview-empty">
        <span>{preview.error ?? 'Preview unavailable'}</span>
      </div>
    {/if}
  {/if}
  </MeltActionButton>

  {#if preview}
    <MeltActionButton
      class="preview-close-button"
      ariaLabel="Close previewed window"
      onClick={(event) => void handlePreviewClose(event)}
    >×</MeltActionButton>
  {/if}
</div>
