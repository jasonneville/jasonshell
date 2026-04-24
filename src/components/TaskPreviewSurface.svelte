<script lang="ts">
  import { onMount } from 'svelte';
  import { emit, listen } from '@tauri-apps/api/event';
  import {
    hideTaskWindowPreview,
    type TaskPreviewPayload
  } from '../lib/taskbarPreview';
  import {
    TASKBAR_REFRESH_WINDOWS_EVENT,
    TASK_PREVIEW_HOVER_ENTER_EVENT
  } from '../lib/taskbarUi';
  import { maximizeTaskWindow } from '../lib/taskbarWindows';

  let preview: TaskPreviewPayload | null = null;

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

<div
  class="surface preview-surface"
  aria-disabled={!preview}
  role="button"
  tabindex="0"
  on:click={() => void handlePreviewActivate()}
  on:keydown={(event) => void handlePreviewKeydown(event)}
  on:mouseenter={() => void emit(TASK_PREVIEW_HOVER_ENTER_EVENT)}
  on:mouseleave={() => void hidePreviewSurface()}
>
  {#if preview}
    <div class="preview-header">
      <img class="preview-icon" src={preview.iconDataUrl} alt="" draggable="false" />
      <div class="preview-copy">
        <strong>{preview.title || preview.processName}</strong>
        <span>{preview.isMinimized ? 'Minimized window' : preview.processName}</span>
      </div>
    </div>

    {#if preview.imageDataUrl}
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
</div>

<style>
  .surface {
    background:
      linear-gradient(180deg, rgba(17, 22, 31, 0.98) 0%, rgba(10, 13, 20, 0.99) 100%);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 0.26rem;
    box-shadow:
      0 18px 36px rgba(0, 0, 0, 0.42),
      inset 0 1px 0 rgba(255, 255, 255, 0.06);
    color: #eef3ff;
    display: grid;
    gap: 0.5rem;
    height: 100%;
    outline: none;
    padding: 0.55rem;
    width: 100%;
  }

  .preview-surface {
    cursor: pointer;
  }

  .preview-surface:focus-visible {
    border-color: rgba(152, 186, 255, 0.72);
    box-shadow:
      0 18px 36px rgba(0, 0, 0, 0.42),
      inset 0 0 0 1px rgba(152, 186, 255, 0.4);
  }

  .preview-header {
    align-items: center;
    display: flex;
    gap: 0.45rem;
    min-width: 0;
  }

  .preview-icon {
    display: block;
    flex: 0 0 auto;
    height: 1rem;
    width: 1rem;
  }

  .preview-copy {
    display: grid;
    gap: 0.08rem;
    min-width: 0;
  }

  .preview-copy strong {
    font-size: 0.72rem;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preview-copy span {
    color: rgba(215, 223, 245, 0.72);
    font-size: 0.58rem;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preview-frame,
  .preview-empty {
    align-items: center;
    background: rgba(7, 9, 14, 0.72);
    border: 1px solid rgba(255, 255, 255, 0.06);
    display: flex;
    flex: 1;
    justify-content: center;
    min-height: 0;
    overflow: hidden;
  }

  .preview-image {
    display: block;
    height: 100%;
    object-fit: contain;
    width: 100%;
  }

  .preview-empty {
    color: rgba(214, 223, 255, 0.74);
    font-size: 0.64rem;
    font-weight: 600;
    padding: 0.75rem;
    text-align: center;
  }
</style>
