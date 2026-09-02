<script lang="ts">
  import './TrayPanelSurface.css';
  import { onMount } from 'svelte';
  import {
    hideTrayPanel,
    invokeTrayPanelIcon,
    listTrayPanelIcons,
    TRAY_PANEL_OPEN_EVENT,
    type SystemTrayIconSnapshot
  } from '../lib/trayPanel';
  import { listen } from '@tauri-apps/api/event';

  let icons: SystemTrayIconSnapshot[] = [];
  let loading = false;
  let loadError = '';
  let invokeError = '';
  let activeIconId: string | null = null;
  let isInvokingTrayIcon = false;
  let disposed = false;

  async function loadTrayIcons() {
    if (disposed) return;
    loading = true;
    loadError = '';
    try {
      const nextIcons = await listTrayPanelIcons();
      if (disposed) return;
      icons = nextIcons;
    } catch (error) {
      if (disposed) return;
      loadError = 'Notification area is unavailable';
      console.error('Failed to load tray icons', error);
    } finally {
      if (disposed) return;
      loading = false;
    }
  }

  async function triggerTrayIcon(icon: SystemTrayIconSnapshot, button: 'left' | 'right') {
    if (disposed) return;
    if (isInvokingTrayIcon) {
      return;
    }
    isInvokingTrayIcon = true;
    activeIconId = icon.id;
    invokeError = '';
    try {
      await invokeTrayPanelIcon(icon.id, button);
      if (disposed) return;
      await loadTrayIcons();
    } catch (error) {
      if (disposed) return;
      invokeError = 'Tray icon action failed. Notification area remains open.';
      console.error(`Failed to invoke tray icon ${icon.id}`, error);
    } finally {
      if (disposed) return;
      activeIconId = null;
      isInvokingTrayIcon = false;
    }
  }

  function handleTrayContextMenu(event: MouseEvent, icon: SystemTrayIconSnapshot) {
    event.preventDefault();
    void triggerTrayIcon(icon, 'right');
  }

  function handleTrayPanelKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      void hideTrayPanel();
    }
  }

  function handleTrayIconKeydown(event: KeyboardEvent, icon: SystemTrayIconSnapshot) {
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      event.stopPropagation();
      void triggerTrayIcon(icon, 'right');
    }
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    disposed = false;
    function registerAsyncUnlistener(registration: Promise<() => void>) {
      void registration.then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlisteners.push(unlisten);
      }).catch((error) => {
        if (!disposed) console.error('Failed to register tray panel listener', error);
      });
    }

    registerAsyncUnlistener(listen(TRAY_PANEL_OPEN_EVENT, () => {
      if (disposed) return;
      void loadTrayIcons();
    }));

    return () => {
      disposed = true;
      while (unlisteners.length) {
        try {
          unlisteners.pop()?.();
        } catch (error) {
          console.error('Failed to dispose tray panel listener', error);
        }
      }
    };
  });
</script>

<svelte:window on:keydown={handleTrayPanelKeydown} />

<div class="tray-panel" id="tray-panel" role="dialog" tabindex="-1" aria-label="Notification area icons">
  <header class="tray-header">
    <button type="button" class="tray-close-button" aria-label="Close notification area icons" on:click={() => void hideTrayPanel()}>
      ×
    </button>
  </header>
  <div class="tray-main">
    {#if invokeError}
      <div class="tray-state tray-state-error" role="alert">{invokeError}</div>
    {/if}
    {#if loading}
      <div class="tray-state" role="status">Loading notification icons…</div>
    {:else if loadError}
      <div class="tray-state tray-state-error" role="alert">{loadError}</div>
    {:else if !icons.length}
      <div class="tray-state" role="status">No notification icons are currently available.</div>
    {:else}
      <div class="tray-content">
        <ul class="tray-grid" role="list">
          {#each icons as icon (icon.id)}
            <li>
              <button
                type="button"
                class="tray-icon-button"
                aria-label={icon.label}
                title={icon.label}
                disabled={Boolean(activeIconId)}
                on:click={() => void triggerTrayIcon(icon, 'left')}
                on:contextmenu={(event) => handleTrayContextMenu(event, icon)}
                on:keydown={(event) => handleTrayIconKeydown(event, icon)}
              >
                <img src={icon.iconDataUrl} alt="" />
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>
</div>
