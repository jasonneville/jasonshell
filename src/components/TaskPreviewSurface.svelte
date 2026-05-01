<script lang="ts">
  import './TaskPreviewSurface.css';
  import { onMount } from 'svelte';
  import { emit, listen } from '@tauri-apps/api/event';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import {
    hideTaskWindowPreview,
    isNativeLiveTaskPreviewPayload,
    type TaskPreviewPayload
  } from '../lib/taskbarPreview';
  import {
    TASKBAR_REFRESH_WINDOWS_EVENT,
    TASK_PREVIEW_HOVER_ENTER_EVENT
  } from '../lib/taskbarUi';
  import { maximizeTaskWindow } from '../lib/taskbarWindows';

  let preview: TaskPreviewPayload | null = null;
  $: isNativeLivePreview = preview ? isNativeLiveTaskPreviewPayload(preview) : false;
  $: previewSurfaceClass = `surface preview-surface${isNativeLivePreview ? ' preview-surface-native' : ''}`;

  async function hidePreviewSurface() {
    preview = null;
    await hideTaskWindowPreview(Date.now()).catch((error) => {
      console.error('Failed to hide task preview', error);
    });
  }

  async function handlePreviewActivate() {
    if (!preview) {
      return;
    }

    try {
      await maximizeTaskWindow(preview.hwnd);
      await emit(TASKBAR_REFRESH_WINDOWS_EVENT);
      await hidePreviewSurface();
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

  onMount(() => {
    const unlisteners: Array<() => void> = [];

    void listen<TaskPreviewPayload>('task-preview:update', (event) => {
      preview = event.payload;
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });

    void listen('task-preview:hide', () => {
      preview = null;
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });

    return () => {
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<MeltActionButton
  class={previewSurfaceClass}
  ariaDisabled={!preview}
  ariaLabel={preview ? `Activate ${preview.title || preview.processName}` : 'Task preview unavailable'}
  onClick={() => void handlePreviewActivate()}
  onKeyDown={(event) => void handlePreviewKeydown(event)}
  onMouseEnter={() => void emit(TASK_PREVIEW_HOVER_ENTER_EVENT)}
  onMouseLeave={() => void hidePreviewSurface()}
>
  {#if preview}

    
    <span><div class="preview-title" aria-hidden="true">{preview.title || preview.processName}</div></span>
    
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
