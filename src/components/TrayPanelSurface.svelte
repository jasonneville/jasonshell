<script lang="ts">
  import './TrayPanelSurface.css';
  import { onMount } from 'svelte';
  import AnchoredPanelCloseButton from './AnchoredPanelCloseButton.svelte';
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
  let keyboardRelayIconId: string | null = null;
  let isInvokingTrayIcon = false;
  let disposed = false;
  let openGeneration = 0;
  let latestRead = 0;

  async function loadTrayIcons(keepGrid = false) {
    if (disposed) return;
    const generation = openGeneration;
    const read = ++latestRead;
    const isCurrent = () => !disposed && generation === openGeneration && read === latestRead;
    loading = !keepGrid;
    loadError = '';
    try {
      const nextIcons = await listTrayPanelIcons();
      if (!isCurrent()) return;
      icons = nextIcons;
    } catch (error) {
      if (!isCurrent()) return;
      loadError = 'Notification area is unavailable';
      console.error('Failed to load tray icons', error);
    } finally {
      if (isCurrent()) loading = false;
    }
  }

  async function triggerTrayIcon(icon: SystemTrayIconSnapshot, button: 'left' | 'right', fromFocusedKeyboard = false) {
    if (disposed) return;
    if (isInvokingTrayIcon) {
      return;
    }
    isInvokingTrayIcon = true;
    activeIconId = icon.id;
    keyboardRelayIconId = fromFocusedKeyboard ? icon.id : null;
    const generation = openGeneration;
    invokeError = '';
    try {
      await invokeTrayPanelIcon(icon.id, button);
      if (disposed || generation !== openGeneration) return;
      // Keep keyed rows mounted during keyboard refresh; never reclaim DOM or native focus.
      await loadTrayIcons(fromFocusedKeyboard);
    } catch (error) {
      if (disposed || generation !== openGeneration) return;
      invokeError = 'Tray icon action failed. Notification area remains open.';
      console.error(`Failed to invoke tray icon ${icon.id}`, error);
    } finally {
      if (disposed) return;
      activeIconId = null;
      keyboardRelayIconId = null;
      isInvokingTrayIcon = false;
    }
  }

  function handleTrayContextMenu(event: MouseEvent, icon: SystemTrayIconSnapshot) {
    event.preventDefault();
    void triggerTrayIcon(icon, 'right');
  }

  async function closeTrayPanel() {
    const generation = openGeneration;
    await hideTrayPanel();
    if (disposed || generation !== openGeneration) return;
    openGeneration += 1;
    keyboardRelayIconId = null;
  }

  function handleTrayPanelKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      void closeTrayPanel();
    }
  }

  function handleTrayIconKeydown(event: KeyboardEvent, icon: SystemTrayIconSnapshot) {
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      event.stopPropagation();
      void triggerTrayIcon(icon, 'right', event.currentTarget === document.activeElement);
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
      openGeneration += 1;
      keyboardRelayIconId = null;
      invokeError = '';
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
    <AnchoredPanelCloseButton label="Close notification area icons" onClick={() => void closeTrayPanel()} />
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
                 disabled={Boolean(activeIconId) && keyboardRelayIconId !== icon.id}
                 aria-disabled={keyboardRelayIconId === icon.id ? true : undefined}
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
