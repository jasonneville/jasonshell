<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { emitTo } from '@tauri-apps/api/event';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, tick } from 'svelte';
  import { hideQuickLaunchPanel, showQuickLaunchPanelContextMenu } from '../lib/quickLaunchPanel';
  import { topBarWebviewWindowEventTarget } from '../lib/topBarPins';
  import { type PinnedTaskbarLauncher } from '../lib/taskbarLaunchers';

  const QUICK_LAUNCH_OPEN_EVENT = 'quick-launch-panel:open';
  const QUICK_LAUNCH_CLOSED_EVENT = 'quick-launch-panel:closed';
  const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT = 'search:toggle-centered';
  const TOP_BAR_TARGET = topBarWebviewWindowEventTarget();

  let launchers: PinnedTaskbarLauncher[] = [];
  let sortedLaunchers: PinnedTaskbarLauncher[] = [];
  let launcherButtons: HTMLButtonElement[] = [];
  let focusedIndex = 0;
  let disposed = false;
  let quickLaunchNonce: string | null = null;
  let quickLaunchSelectionInFlight = false;
  let suppressNextRowClick = false;
  let shellSurfaceHotkeyHandled = false;

  $: sortedLaunchers = [...launchers].sort((a, b) => a.name.localeCompare(b.name));

  function getLauncherDetail(launcher: PinnedTaskbarLauncher) {
    const source = launcher.targetPath ?? launcher.shortcutPath;
    return source.split(/[\\/]/).pop() ?? source;
  }

  function clampFocusedIndex(index: number, launcherCount: number) {
    if (!launcherCount) return -1;
    return Math.min(Math.max(index, 0), launcherCount - 1);
  }

  async function focusLauncher(index = focusedIndex) {
    const nextIndex = clampFocusedIndex(index, sortedLaunchers.length);
    if (nextIndex < 0) return;
    focusedIndex = nextIndex;
    await tick();
    launcherButtons[nextIndex]?.focus();
  }

  function applyLaunchers(nextLaunchers: PinnedTaskbarLauncher[]) {
    if (disposed) return;
    launchers = nextLaunchers;
    focusedIndex = clampFocusedIndex(0, nextLaunchers.length);
  }

  async function chooseLauncher(launcher: PinnedTaskbarLauncher) {
    if (!quickLaunchNonce) {
      return;
    }
    quickLaunchSelectionInFlight = true;
    try {
      await invoke('select_quick_launch_panel', { args: { nonce: quickLaunchNonce, shortcutPath: launcher.shortcutPath } });
    } catch (error) {
      console.error('Failed to select quick launch row', error);
      quickLaunchSelectionInFlight = false;
    }
  }

  function handlePanelPointerDown(_event: PointerEvent) {
  }

  function isCtrlSpaceHotkey(event: KeyboardEvent) {
    return event.code === 'Space' && event.ctrlKey && !event.altKey && !event.metaKey;
  }

  async function openQuickLaunchNativeMenu(launcher: PinnedTaskbarLauncher, event: MouseEvent) {
    if (!quickLaunchNonce) return;
    try {
      await showQuickLaunchPanelContextMenu({ nonce: quickLaunchNonce, shortcutPath: launcher.shortcutPath, x: event.clientX, y: event.clientY });
    } catch (error) {
      console.error('Failed to show quick launch native menu', error);
    }
  }

  function handleSearchHotkeyKeydown(event: KeyboardEvent) {
    if (!isCtrlSpaceHotkey(event)) return;
    event.preventDefault();
    event.stopPropagation();
    if (!shellSurfaceHotkeyHandled && !event.repeat) {
      shellSurfaceHotkeyHandled = true;
      void emitTo(TOP_BAR_TARGET, SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      void hideQuickLaunchPanel();
      return;
    }
    if (!sortedLaunchers.length) return;
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      void focusLauncher(focusedIndex < 0 ? 0 : (focusedIndex + 1) % sortedLaunchers.length);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      void focusLauncher(focusedIndex < 0 ? sortedLaunchers.length - 1 : (focusedIndex - 1 + sortedLaunchers.length) % sortedLaunchers.length);
    } else if (event.key === 'Home') {
      event.preventDefault();
      void focusLauncher(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      void focusLauncher(sortedLaunchers.length - 1);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      if (focusedIndex >= 0) {
        void chooseLauncher(sortedLaunchers[focusedIndex]);
      }
    }
  }

  onMount(() => {
    const keyupHandler = (event: KeyboardEvent) => {
      if (event.code !== 'Space' || !shellSurfaceHotkeyHandled) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      shellSurfaceHotkeyHandled = false;
    };
    const blurHandler = () => {
      if (!quickLaunchSelectionInFlight) {
        void invoke('hide_quick_launch_panel_on_focus_loss');
      }
    };
    window.addEventListener('blur', blurHandler);
    window.addEventListener('keydown', handleSearchHotkeyKeydown, true);
    window.addEventListener('keyup', keyupHandler, true);
    const unlistenOpen = listen<{ nonce: string; rows: PinnedTaskbarLauncher[] }>(QUICK_LAUNCH_OPEN_EVENT, async (event) => {
      quickLaunchNonce = event.payload.nonce;
      applyLaunchers([...event.payload.rows].sort((a, b) => a.name.localeCompare(b.name)));
      await tick();
      launcherButtons[focusedIndex]?.focus();
    });
    const unlisten = listen<{ nonce: string | null }>(QUICK_LAUNCH_CLOSED_EVENT, (event) => {
      if (event.payload.nonce !== null && event.payload.nonce !== quickLaunchNonce) {
        return;
      }
      focusedIndex = 0;
      launchers = [];
      quickLaunchNonce = null;
      quickLaunchSelectionInFlight = false;
    });
    return () => {
      disposed = true;
      window.removeEventListener('blur', blurHandler);
      window.removeEventListener('keydown', handleSearchHotkeyKeydown, true);
      window.removeEventListener('keyup', keyupHandler, true);
      void unlistenOpen.then((fn) => fn()).catch(() => undefined);
      void unlisten.then((fn) => fn()).catch(() => undefined);
    };
  });
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="quick-launch-panel surface" aria-labelledby="quick-launch-title" aria-describedby="quick-launch-desc" role="dialog" aria-modal="true" tabindex="-1" on:pointerdown={handlePanelPointerDown}>
  <header class="quick-launch-panel-header">
    <div class="quick-launch-panel-copy">
      <span id="quick-launch-title">Quick Launch</span>
    </div>
    <kbd>↑↓ Enter Esc</kbd>
  </header>
  <div class="rows" role="listbox" aria-label="Pinned Explorer launchers">
    {#if sortedLaunchers.length}
      {#each sortedLaunchers as launcher, index (launcher.shortcutPath)}
        <div class="launcher-row" role="none">
          <!-- svelte-ignore a11y_role_supports_aria_props -->
          <button
            bind:this={launcherButtons[index]}
            class:focused={index === focusedIndex}
            tabindex={index === focusedIndex ? 0 : -1}
            title={launcher.targetPath ?? launcher.shortcutPath}
            type="button"
            role="option"
            aria-selected={index === focusedIndex}
            aria-haspopup="menu"
            on:click={() => {
              if (suppressNextRowClick) {
                suppressNextRowClick = false;
                return;
              }
              void chooseLauncher(launcher);
            }}
            on:contextmenu|preventDefault={(event) => {
              event.stopPropagation();
              void openQuickLaunchNativeMenu(launcher, event);
            }}
            on:focus={() => (focusedIndex = index)}
          >
            <span class="launcher-icon" aria-hidden="true">
              <img src={launcher.iconDataUrl} alt="" />
            </span>
            <span class="launcher-copy">
              <strong>{launcher.name}</strong>
              <small>{getLauncherDetail(launcher)}</small>
            </span>
          </button>
        </div>
      {/each}
    {:else}
      <p class="quick-launch-empty">No pinned Explorer launchers.</p>
    {/if}
  </div>
</div>

<style>
  .quick-launch-panel {
    background: var(--js-bg-surface);
    border: 1px solid var(--js-color-border);
    box-sizing: border-box;
    color: var(--js-color-text);
    display: flex;
    flex-direction: column;
    font-family: var(--js-font-sans);
    gap: var(--js-space-3);
    height: 100%;
    min-height: 0;
    overflow: hidden;
    padding: var(--js-space-3);
    width: 100%;
  }

  .quick-launch-panel-header {
    align-items: flex-end;
    border-bottom: 1px solid var(--js-color-border-soft);
    display: flex;
    flex: 0 0 auto;
    gap: var(--js-space-3);
    justify-content: space-between;
    padding-bottom: var(--js-space-2);
  }

  .quick-launch-panel-copy {
    display: grid;
    gap: 0.08rem;
    min-width: 0;
  }

  .quick-launch-panel-copy span {
    color: var(--js-color-text-muted);
    font-size: 0.62rem;
    font-weight: 750;
    letter-spacing: 0.02em;
    overflow: hidden;
    text-overflow: ellipsis;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .quick-launch-panel-header kbd {
    background: var(--js-color-surface-overlay);
    border: 1px solid var(--js-color-border);
    border-radius: var(--js-radius-xs);
    color: var(--js-color-text-muted);
    font-size: 0.56rem;
    font-weight: 800;
    line-height: 1;
    padding: 0.1rem 0.34rem;
    white-space: nowrap;
  }

  .rows {
    display: grid;
    flex: 1 1 auto;
    gap: var(--js-space-2);
    min-height: 0;
    overflow: auto;
    padding-right: 0.1rem;
    scrollbar-color: var(--js-scrollbar-thumb) var(--js-scrollbar-track);
  }

  .launcher-row {
    position: relative;
  }

  .rows button {
    font: inherit;
    align-items: center;
    background: transparent;
    border: 1px solid transparent;

    border-radius: var(--js-radius-md);
    color: inherit;
    cursor: pointer;
    display: grid;
    gap: var(--js-space-3);
    grid-template-columns: 1.6rem minmax(0, 1fr);
    min-width: 0;
    min-height: 2.35rem;
    padding: 0.3rem 0.42rem;
    text-align: left;
    width: 100%;
  }

  .rows button:hover,
  .rows button.focused {
    background: var(--js-color-selected);
  }

  .rows button:focus-visible {
    background: var(--js-color-selected);
    border-color: transparent;
    box-shadow: none;
    outline: none;
    outline-offset: 0;
  }

  .launcher-icon {
    align-items: center;
    background: transparent;
    border: none;
    border-radius: var(--js-radius-sm);
    display: grid;
    height: 1.6rem;
    justify-items: center;
    overflow: hidden;
    width: 1.6rem;
  }

  .launcher-icon img {
    display: block;
    height: 1rem;
    width: 1rem;
  }

  .launcher-copy {
    display: grid;
    gap: 0.04rem;
    min-width: 0;
  }

  .launcher-copy strong,
  .launcher-copy small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .launcher-copy strong {
    font-size: 0.74rem;
    font-weight: 760;
    line-height: 1.05;
  }

  .launcher-copy small {
    color: var(--js-color-text-muted);
    font-size: 0.58rem;
    line-height: 1.05;
  }

  .quick-launch-empty {
    color: var(--js-color-text-muted);
    display: grid;
    flex: 1 1 auto;
    font-size: 0.62rem;
    margin: 0;
    min-height: 0;
    place-items: center;
    text-align: center;
  }

  @media (max-width: 420px) {
    .quick-launch-panel {
      gap: var(--js-space-2);
      padding: var(--js-space-2);
    }

    .quick-launch-panel-header {
      align-items: center;
    }

    .quick-launch-panel-header kbd,
    .launcher-copy small {
      display: none;
    }

    .rows button {
      grid-template-columns: 1.45rem minmax(0, 1fr);
      min-height: 2.2rem;
    }

    .launcher-icon {
      height: 1.45rem;
      width: 1.45rem;
    }

    .launcher-icon img {
      height: 0.9rem;
      width: 0.9rem;
    }
  }
</style>
