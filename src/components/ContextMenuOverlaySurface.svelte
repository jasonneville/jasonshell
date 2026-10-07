<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import ContextMenu from './ContextMenu.svelte';
  import ContextMenuItem from './ContextMenuItem.svelte';
  import ContextMenuSeparator from './ContextMenuSeparator.svelte';
  import {
    CONTEXT_MENU_OVERLAY_OPEN_EVENT,
    type ContextMenuOverlayRequest,
    dispatchContextMenuOverlaySelection,
    hideContextMenuOverlay
  } from '../lib/contextMenuOverlay';
  import { positionScrollableContextMenuInViewport } from '../lib/contextMenuPosition';

  let request: ContextMenuOverlayRequest | null = null;
  let menuElement: HTMLDivElement | null = null;
  const MENU_INSET = 8;
  let placement: { x: number; y: number; maxHeight?: number } = { x: MENU_INSET, y: MENU_INSET };
  let positioned = false;

  function enabledMenuItems(): HTMLButtonElement[] {
    return Array.from(menuElement?.querySelectorAll<HTMLButtonElement>('button:not(:disabled).context-menu-item') ?? []);
  }

  function focusMenuItem(index: number) {
    const items = enabledMenuItems();
    if (items.length === 0) return;
    items[(index + items.length) % items.length]?.focus();
  }

  async function positionMenu() {
    const currentRequest = request;
    await tick();
    if (!currentRequest || request !== currentRequest || !menuElement) return;
    const currentMenuElement = menuElement;
    // scrollHeight reflects the full content even when the previous menu was scroll-clamped.
    // offsetWidth is not affected by the shared menu's opening transform.
    const menuHeight = menuElement.scrollHeight + menuElement.offsetHeight - menuElement.clientHeight;
    const menuWidth = menuElement.offsetWidth;
    const source = request.source;
    const anchor = source === 'bottom-bar'
      ? { x: MENU_INSET, y: Math.max(MENU_INSET, window.innerHeight - menuHeight - MENU_INSET) }
      : { x: MENU_INSET, y: MENU_INSET };
    placement = positionScrollableContextMenuInViewport(
      anchor,
      { width: menuWidth, height: menuHeight },
      { width: window.innerWidth, height: window.innerHeight }
    );
    positioned = true;
    await tick();
    if (request !== currentRequest || !menuElement || menuElement !== currentMenuElement) return;
    focusMenuItem(0);
  }

  function dismiss() {
    const source = request?.source;
    request = null;
    // The native hide command restores origin focus only for dismissal, not selected actions.
    void hideContextMenuOverlay(source, true).catch((error) => console.error('Failed to hide context menu overlay', error));
  }

  function select(action: string) {
    if (!request) return;
    const selection = { source: request.source, kind: request.kind, token: request.token, action };
    request = null;
    void dispatchContextMenuOverlaySelection(selection).catch((error) => {
      console.error('Failed to dispatch context menu action', error);
    });
    void hideContextMenuOverlay().catch((error) => console.error('Failed to hide context menu overlay', error));
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      dismiss();
      return;
    }
    const items = enabledMenuItems();
    if (items.length === 0) return;
    const activeIndex = items.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      focusMenuItem(activeIndex + 1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      focusMenuItem(activeIndex - 1);
    } else if (event.key === 'Home') {
      event.preventDefault();
      focusMenuItem(0);
    } else if (event.key === 'End') {
      event.preventDefault();
      focusMenuItem(items.length - 1);
    }
  }

  onMount(() => {
    let disposed = false;
    const registration = listen<ContextMenuOverlayRequest>(CONTEXT_MENU_OVERLAY_OPEN_EVENT, (event) => {
      positioned = false;
      placement = { x: MENU_INSET, y: MENU_INSET };
      request = event.payload;
      void positionMenu();
    });
    window.addEventListener('resize', positionMenu);
    return () => {
      disposed = true;
      window.removeEventListener('resize', positionMenu);
      void registration.then((unlisten) => unlisten());
      void disposed;
    };
  });
</script>

<svelte:window on:keydown={handleKeydown} on:blur={dismiss} />

{#if request}
  <div class="context-menu-overlay-backdrop" class:positioned on:pointerdown={dismiss} role="presentation">
    <ContextMenu
      className="context-menu-overlay"
      ariaLabel="Context menu"
      style={`left:${placement.x}px;top:${placement.y}px;${placement.maxHeight === undefined ? '' : `max-height:${placement.maxHeight}px`}`}
      bind:element={menuElement}
      on:click={(event) => event.stopPropagation()}
      on:pointerdown={(event) => event.stopPropagation()}
    >
    {#if request.kind === 'pin'}
      <ContextMenuItem icon="preview" onClick={() => select('open')}>Open</ContextMenuItem>
      <ContextMenuItem icon="code_blocks" onClick={() => select('openInVscode')}>Open in VS Code</ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon="delete" destructive onClick={() => select('unpin')}>Unpin</ContextMenuItem>
    {:else if request.kind === 'task-group'}
      <ContextMenuItem icon="monitor_heart" onClick={() => select('process')} disabled={!request.processId}>Open in Process Manager</ContextMenuItem>
      <ContextMenuItem icon="add_location" onClick={() => select('pin')}>Pin to taskbar</ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon="close" destructive onClick={() => select('closeAll')}>Close all windows</ContextMenuItem>
    {:else if request.kind === 'task-window'}
      <ContextMenuItem icon="preview" onClick={() => select('focus')}>{request.isMinimized ? 'Restore' : 'Switch to'}</ContextMenuItem>
      <ContextMenuItem icon="preview" onClick={() => select('minimize')} disabled={request.isMinimized}>Minimize</ContextMenuItem>
      <ContextMenuItem icon="monitor_heart" onClick={() => select('process')} disabled={!request.processId}>Open in Process Manager</ContextMenuItem>
      <ContextMenuItem icon="add_location" onClick={() => select('pin')}>Pin to taskbar</ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon="close" destructive onClick={() => select('close')}>Close window</ContextMenuItem>
    {:else}
      <ContextMenuItem icon="preview" onClick={() => select('launch')}>Launch</ContextMenuItem>
      <ContextMenuItem icon="settings" onClick={() => select('runas')}>Run as administrator</ContextMenuItem>
      <ContextMenuItem icon="settings" onClick={() => select('properties')}>Properties</ContextMenuItem>
      <ContextMenuItem icon="folder" onClick={() => select('reveal')}>Open shortcut location</ContextMenuItem>
      <ContextMenuItem icon="folder" onClick={() => select('revealTarget')}>Open target location</ContextMenuItem>
      <ContextMenuItem icon="file_copy" onClick={() => select('copyPath')}>Copy shortcut path</ContextMenuItem>
      <ContextMenuSeparator />
      <ContextMenuItem icon="delete" destructive onClick={() => select('unpin')}>Unpin from taskbar</ContextMenuItem>
    {/if}
    </ContextMenu>
  </div>
{/if}

<style>
  .context-menu-overlay-backdrop {
    height: 100%;
    inset: 0;
    position: fixed;
    width: 100%;
  }
  .context-menu-overlay-backdrop :global(.context-menu-overlay) { visibility: hidden; }
  .context-menu-overlay-backdrop.positioned :global(.context-menu-overlay) { visibility: visible; }
</style>
