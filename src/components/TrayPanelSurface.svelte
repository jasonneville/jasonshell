<script lang="ts">
  import './TrayPanelSurface.css';
  import { onMount } from 'svelte';
  import {
    invokeTrayPanelIcon,
    listTrayPanelIcons,
    type SystemTrayIconSnapshot
  } from '../lib/trayPanel';

  let icons: SystemTrayIconSnapshot[] = [];
  let loading = false;
  let loadError = '';
  let activeIconId: string | null = null;

  async function loadTrayIcons() {
    loading = true;
    loadError = '';
    try {
      icons = await listTrayPanelIcons();
    } catch (error) {
      loadError = 'Notification area is unavailable';
      console.error('Failed to load tray icons', error);
    } finally {
      loading = false;
    }
  }

  async function triggerTrayIcon(icon: SystemTrayIconSnapshot, button: 'left' | 'right') {
    if (activeIconId) {
      return;
    }
    activeIconId = icon.id;
    try {
      await invokeTrayPanelIcon(icon.id, button);
      await loadTrayIcons();
    } catch (error) {
      loadError = 'Notification area is unavailable';
      console.error(`Failed to invoke tray icon ${icon.id}`, error);
    } finally {
      activeIconId = null;
    }
  }

  function handleTrayContextMenu(event: MouseEvent, icon: SystemTrayIconSnapshot) {
    event.preventDefault();
    void triggerTrayIcon(icon, 'right');
  }

  onMount(() => {
    void loadTrayIcons();
  });
</script>

<div class="tray-panel" id="tray-panel" role="dialog" aria-label="Notification area icons">
  {#if loading}
    <div class="tray-state" role="status">Loading notification icons…</div>
  {:else if loadError}
    <div class="tray-state tray-state-error" role="alert">{loadError}</div>
  {:else if !icons.length}
    <div class="tray-state" role="status">No notification icons are currently available.</div>
  {:else}
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
          >
            <img src={icon.iconDataUrl} alt="" />
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
