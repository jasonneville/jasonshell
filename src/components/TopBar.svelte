<script lang="ts">
  import './TopBar.css';
  import { onMount, tick } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { reportShellSurfaceRuntimeMetrics } from '../lib/runtimeMetrics';
  import {
    launchPinnedTaskbarLauncher,
    listPinnedTaskbarLaunchers,
    type PinnedTaskbarLauncher
  } from '../lib/taskbarLaunchers';
  import {
    activateTaskWindow,
    listOpenTaskWindows,
    type TaskbarWindow
  } from '../lib/taskbarWindows';
  import {
    hideSearchPanel,
    openShellPath,
    publishSearchPanel,
    SEARCH_PANEL_ACTIVATE_EVENT,
    SEARCH_PANEL_CLOSED_EVENT,
    SEARCH_PANEL_INTERACTION_EVENT,
    SEARCH_PANEL_PIN_FOLDER_EVENT,
    SEARCH_PANEL_SELECT_EVENT,
    showSearchPanel,
    type SearchPanelResult
  } from '../lib/searchPanel';
  import {
    listStackPins,
    pinStackFolder,
    reorderStackPins,
    STACK_PINS_UPDATED_EVENT,
    unpinStackFolder,
    showStackPopup,
    type StackPin
  } from '../lib/stackPopup';
  import { folderPathsFromTransfer, hasFolderDragPayload, normalizeDroppedPath } from '../lib/folderDrag';
  import { buildSearchCatalog } from '../lib/searchCatalog';
  import { rankSearchResults, recordSearchUsage } from '../lib/searchRanking';
  import { isSystemPathResult, searchSystem, SEARCH_INDEX_REFRESHED_EVENT } from '../lib/systemSearch';
  import {
    showTopBarPinContextMenu,
    TOP_BAR_PIN_MENU_ACTION_EVENT,
    type TopBarPinMenuActionPayload
  } from '../lib/taskbarMenus';
  import { stackPinRevealPath, topBarWebviewWindowEventTarget } from '../lib/topBarPins';
  import {
    shouldApplySystemSearchResponse,
    shouldRefreshSystemSearchAfterIndexUpdate,
    shouldRetryIndexedSearch
  } from '../lib/systemSearchState';
  import { topBarIdentityState } from '../features/top-bar/topBarUxState';

  const timeFormatter = new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit'
  });

  const dateFormatter = new Intl.DateTimeFormat(undefined, {
    day: 'numeric',
    month: 'short',
    weekday: 'short'
  });

  let now = new Date();
  let launchers: PinnedTaskbarLauncher[] = [];
  let openWindows: TaskbarWindow[] = [];
  let stackPins: StackPin[] = [];
  let systemResults: SearchPanelResult[] = [];
  let searchQuery = '';
  let searchOpen = false;
  let selectedIndex = 0;
  let searchStatus = 'Loading search catalog...';
  let searchControl: HTMLDivElement;
  let searchInput: HTMLInputElement;
  // Pin rail UI
  let pinRailHover = false;
  let pinRailEl: HTMLDivElement | null = null;
  let showRailScrollLeft = false;
  let showRailScrollRight = false;
  let focusedPinIndex: number | null = null;
  let systemSearchTimer: number | null = null;
  let systemSearchRefreshTimer: number | null = null;
  let systemSearchSequence = 0;
  let pinDropStatus = '';
  let pinDropStatusKind: 'info' | 'error' = 'info';
  let pinDropStatusTimer: number | null = null;
  let searchBlurCloseTimer: number | null = null;
  let searchPanelInteractionUntil = 0;
  let draggingPinPath: string | null = null;
  let pendingVisiblePinPath: string | null = null;
  let stackPinsLoaded = false;

  const PIN_DRAG_TYPE = 'application/x-jasonshell-stack-pin';
  const SEARCH_BLUR_CLOSE_DELAY_MS = 180;
  const SEARCH_PANEL_INTERACTION_GRACE_MS = 350;

  $: allResults = buildSearchCatalog(launchers, openWindows, systemResults);
  $: searchResults = rankSearchResults(allResults, searchQuery);
  $: selectedIndex = Math.min(selectedIndex, Math.max(searchResults.length - 1, 0));
  $: searchPanelPayload = {
    query: searchQuery,
    results: searchResults,
    selectedIndex,
    statusMessage: searchStatus
  };
  $: identityState = topBarIdentityState(stackPins.length, launchers.length, searchStatus);
  $: if (searchOpen) {
    void publishSearchResults(searchPanelPayload);
  }

  async function loadSearchCatalog() {
    try {
      [launchers, openWindows] = await Promise.all([
        listPinnedTaskbarLaunchers(),
        listOpenTaskWindows()
      ]);
      searchStatus = launchers.length || openWindows.length
        ? 'Type to search apps, windows, files, folders, and commands'
        : 'Type to search installed apps, files, folders, and commands';
    } catch (error) {
      console.error('Failed to load search catalog', error);
      launchers = [];
      openWindows = [];
      searchStatus = 'Search catalog unavailable';
    }
  }

  async function loadStackPins() {
    try {
      await applyStackPins(await listStackPins(), false);
      stackPinsLoaded = true;
    } catch (error) {
      console.error('Failed to load stack pins', error);
      stackPins = [];
      stackPinsLoaded = true;
    }
  }

  function showPinDropStatus(message: string, kind: 'info' | 'error' = 'info') {
    pinDropStatus = message;
    pinDropStatusKind = kind;
    if (pinDropStatusTimer !== null) {
      window.clearTimeout(pinDropStatusTimer);
    }
    pinDropStatusTimer = window.setTimeout(() => {
      pinDropStatus = '';
      pinDropStatusTimer = null;
    }, 2_800);
  }

  async function pinSearchFolder(folderPath: string) {
    pendingVisiblePinPath = folderPath;
    await applyStackPins(await pinStackFolder(folderPath));
  }

  async function pinAndOpenDroppedFolders(paths: string[], target: EventTarget | null) {
    const normalizedPaths = paths
      .map((path) => normalizeDroppedPath(path))
      .filter((path): path is string => Boolean(path));
    const pinnedPaths: string[] = [];
    for (const path of paths) {
      const normalizedPath = normalizeDroppedPath(path);
      if (!normalizedPath) {
        continue;
      }
      await pinStackFolder(normalizedPath).then(() => {
        pinnedPaths.push(normalizedPath);
      }).catch((error) => {
        console.error(`Failed to pin dropped folder ${path}`, error);
      });
    }
    if (!pinnedPaths.length) {
      showPinDropStatus(
        normalizedPaths.length
          ? 'Drop a folder path, not a file or unavailable location'
          : 'Drop a folder to pin it',
        'error'
      );
      return;
    }
    showPinDropStatus(pinnedPaths.length === 1 ? 'Pinned folder' : `Pinned ${pinnedPaths.length} folders`);
    pendingVisiblePinPath = pinnedPaths[pinnedPaths.length - 1] ?? null;
    await openStackPath(pinnedPaths[0], target);
  }

  async function openPanel() {
    cancelSearchBlurClose();
    searchOpen = true;
    const rect = searchControl.getBoundingClientRect();
    await showSearchPanel({
      anchorLeft: rect.left,
      anchorWidth: rect.width
    });
    await publishSearchResults();
  }

  async function closePanel() {
    cancelSearchBlurClose();
    searchOpen = false;
    await hideSearchPanel().catch((error) => {
      console.error('Failed to hide search panel', error);
    });
  }

  function cancelSearchBlurClose() {
    if (searchBlurCloseTimer !== null) {
      window.clearTimeout(searchBlurCloseTimer);
      searchBlurCloseTimer = null;
    }
  }

  function markSearchPanelInteraction() {
    searchPanelInteractionUntil = Date.now() + SEARCH_PANEL_INTERACTION_GRACE_MS;
  }

  function scheduleSearchBlurClose() {
    cancelSearchBlurClose();
    searchBlurCloseTimer = window.setTimeout(() => {
      searchBlurCloseTimer = null;
      if (Date.now() < searchPanelInteractionUntil) {
        return;
      }
      void closePanel();
    }, SEARCH_BLUR_CLOSE_DELAY_MS);
  }

  function handleTopBarPointerDown(event: MouseEvent) {
    if (!searchOpen || !searchControl) {
      return;
    }
    const target = event.target instanceof Node ? event.target : null;
    if (target && searchControl.contains(target)) {
      return;
    }
    void closePanel();
  }

  async function openStackFromPin(pin: StackPin, target: EventTarget | null) {
    await openStackPath(pin.path, target);
  }

  function handlePinClick(event: MouseEvent, pin: StackPin, index: number) {
    event.preventDefault();
    event.stopPropagation();
    focusedPinIndex = index;
    void openStackFromPin(pin, event.currentTarget);
  }

  async function openStackPath(path: string, target: EventTarget | null) {
    const button = target as HTMLElement | null;
    const rect = button?.getBoundingClientRect() ?? searchControl?.getBoundingClientRect();
    if (!rect) {
      return;
    }

    await closePanel();
    await showStackPopup({
      anchorLeft: rect.left,
      anchorWidth: rect.width,
      folderPath: path
    }).catch((error) => {
      console.error('Failed to show stack popup', error);
    });
  }

  function handlePinRailDragOver(event: DragEvent) {
    if (!draggingPinPath && !hasFolderDragPayload(event.dataTransfer)) {
      return;
    }
    event.preventDefault();
    pinRailHover = !draggingPinPath;
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = draggingPinPath ? 'move' : 'copy';
    }
  }

  function handlePinRailDrop(event: DragEvent) {
    if (draggingPinPath) {
      draggingPinPath = null;
      pinRailHover = false;
      return;
    }
    if (!hasFolderDragPayload(event.dataTransfer)) {
      pinRailHover = false;
      return;
    }

    event.preventDefault();
    const dt = event.dataTransfer;
    pinRailHover = false;
    void pinAndOpenDroppedFolders(folderPathsFromTransfer(dt), event.currentTarget);
  }

  function handlePinRailDragLeave() {
    pinRailHover = false;
  }

  function handlePinDragStart(event: DragEvent, pin: StackPin) {
    draggingPinPath = pin.path;
    event.dataTransfer?.setData(PIN_DRAG_TYPE, pin.path);
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
    }
  }

  function handlePinDragEnd() {
    draggingPinPath = null;
    pinRailHover = false;
  }

  function handlePinDragOver(event: DragEvent, targetPin: StackPin) {
    if (!draggingPinPath || draggingPinPath === targetPin.path) {
      return;
    }
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'move';
    }
  }

  async function handlePinDrop(event: DragEvent, targetPin: StackPin) {
    if (!draggingPinPath || draggingPinPath === targetPin.path) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const sourcePath = event.dataTransfer?.getData(PIN_DRAG_TYPE) || draggingPinPath;
    draggingPinPath = null;
    const nextPins = movePinBefore(stackPins, sourcePath, targetPin.path);
    if (nextPins === stackPins) {
      return;
    }
    stackPins = nextPins;
    try {
      await applyStackPins(await reorderStackPins(nextPins.map((pin) => pin.path)));
      showPinDropStatus('Reordered pinned folders');
    } catch (error) {
      console.error('Failed to reorder stack pins', error);
      showPinDropStatus('Could not save pin order', 'error');
      await loadStackPins();
    }
  }

  function movePinBefore(pins: StackPin[], sourcePath: string, targetPath: string) {
    const sourceIndex = pins.findIndex((pin) => pin.path === sourcePath);
    const targetIndex = pins.findIndex((pin) => pin.path === targetPath);
    if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) {
      return pins;
    }
    const nextPins = [...pins];
    const [pin] = nextPins.splice(sourceIndex, 1);
    const insertIndex = nextPins.findIndex((item) => item.path === targetPath);
    nextPins.splice(Math.max(0, insertIndex), 0, pin);
    return nextPins;
  }

  function updateRailScrollButtons() {
    if (!pinRailEl) {
      showRailScrollLeft = false;
      showRailScrollRight = false;
      return;
    }
    const el = pinRailEl;
    showRailScrollLeft = el.scrollLeft > 2;
    showRailScrollRight = el.scrollWidth - el.clientWidth - el.scrollLeft > 2;
  }

  function scrollRailBy(delta: number) {
    if (!pinRailEl) {
      return;
    }
    pinRailEl.scrollBy({ left: delta, behavior: 'smooth' });
    // schedule update
    setTimeout(updateRailScrollButtons, 160);
  }

  function scrollRailLeft() {
    if (!pinRailEl) return;
    scrollRailBy(-Math.max(120, Math.floor(pinRailEl.clientWidth * 0.6)));
  }

  function scrollRailRight() {
    if (!pinRailEl) return;
    scrollRailBy(Math.max(120, Math.floor(pinRailEl.clientWidth * 0.6)));
  }

  function handleRailWheel(event: WheelEvent) {
    if (!pinRailEl) return;
    if (Math.abs(event.deltaX) > Math.abs(event.deltaY)) {
      // let horizontal scroll happen
      return;
    }
    // translate vertical wheel to horizontal scroll
    event.preventDefault();
    pinRailEl.scrollLeft += event.deltaY;
    updateRailScrollButtons();
  }

  function handlePinRailKeydown(event: KeyboardEvent) {
    if (!stackPins || !stackPins.length) return;
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      const next = focusedPinIndex === null ? 0 : Math.min(stackPins.length - 1, focusedPinIndex + 1);
      focusedPinIndex = next;
      const btn = pinRailEl?.querySelectorAll('button')[next] as HTMLElement | undefined;
      btn?.focus();
      updateRailScrollButtons();
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      const prev = focusedPinIndex === null ? stackPins.length - 1 : Math.max(0, focusedPinIndex - 1);
      focusedPinIndex = prev;
      const btn = pinRailEl?.querySelectorAll('button')[prev] as HTMLElement | undefined;
      btn?.focus();
      updateRailScrollButtons();
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      const idx = focusedPinIndex ?? 0;
      const pin = stackPins[idx];
      if (pin) {
        void openStackFromPin(pin, pinRailEl?.querySelectorAll('button')[idx] ?? null);
      }
    }
  }

  function queryPinButton(path: string) {
    if (!pinRailEl) {
      return null;
    }

    return Array.from(pinRailEl.querySelectorAll<HTMLElement>('button[data-path]'))
      .find((button) => button.dataset.path === path) ?? null;
  }

  async function revealPin(path: string) {
    await tick();
    window.requestAnimationFrame(() => {
      queryPinButton(path)?.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' });
      updateRailScrollButtons();
    });
  }

  async function applyStackPins(nextPins: StackPin[], allowDetectedAdd = stackPinsLoaded) {
    const addedPinPath = stackPinRevealPath(stackPins, nextPins, pendingVisiblePinPath, allowDetectedAdd);
    stackPins = nextPins;
    pendingVisiblePinPath = null;
    if (addedPinPath) {
      await revealPin(addedPinPath);
      return;
    }
    await tick();
    updateRailScrollButtons();
  }

  function handlePinContextMenu(event: MouseEvent, pin: StackPin) {
    event.preventDefault();
    event.stopPropagation();
    void showTopBarPinContextMenu({
      path: pin.path,
      x: event.clientX,
      y: event.clientY
    }).catch((error) => {
      console.error('Failed to show top-bar pin context menu', error);
    });
  }

  async function unpinFromMenu(path: string) {
    await unpinStackFolder(path).catch((error) => {
      console.error('Failed to unpin folder', error);
    });
  }

  async function publishSearchResults(payload = searchPanelPayload) {
    await publishSearchPanel(payload).catch((error) => {
      console.error('Failed to update search panel', error);
    });
  }

  function scheduleSystemSearch(query: string) {
    if (systemSearchTimer !== null) {
      window.clearTimeout(systemSearchTimer);
      systemSearchTimer = null;
    }
    if (systemSearchRefreshTimer !== null) {
      window.clearTimeout(systemSearchRefreshTimer);
      systemSearchRefreshTimer = null;
    }

    const trimmedQuery = query.trim();
    systemSearchSequence += 1;
    if (trimmedQuery.length < 2) {
      systemResults = [];
      return;
    }

    const sequence = systemSearchSequence;
    systemSearchTimer = window.setTimeout(() => {
      systemSearchTimer = null;
      void loadSystemSearchResults(trimmedQuery, sequence, 0);
    }, 140);
  }

  async function loadSystemSearchResults(query: string, sequence: number, refreshAttempt: number) {
    searchStatus = refreshAttempt
      ? 'Updating indexed search results...'
      : 'Searching indexed apps and files...';
    try {
      const results = await searchSystem(query);
      if (!shouldApplySystemSearchResponse(query, sequence, searchQuery, systemSearchSequence)) {
        return;
      }

      systemResults = results;
      searchStatus = results.length
        ? 'Showing apps, windows, files, folders, and commands'
        : 'No installed apps or files matched';
      if (shouldRetryIndexedSearch(results.length, refreshAttempt)) {
        systemSearchRefreshTimer = window.setTimeout(() => {
          systemSearchRefreshTimer = null;
          void loadSystemSearchResults(query, sequence, refreshAttempt + 1);
        }, 650);
      }
    } catch (error) {
      console.error('Failed to search installed apps and files', error);
      if (sequence === systemSearchSequence) {
        systemResults = [];
        searchStatus = 'Installed app and file search unavailable';
      }
    }
  }

  async function activateResult(result: SearchPanelResult | undefined) {
    if (!result) {
      return;
    }

    recordSearchUsage(result.id);
    if (isSystemPathResult(result)) {
      await openShellPath(result.path as string);
    } else if (result.kind === 'app') {
      await launchPinnedTaskbarLauncher(result.id.replace('app:', ''));
    } else if (result.kind === 'window') {
      const taskWindow = openWindows.find((item) => result.id === `window:${item.hwnd}`);
      if (taskWindow) {
        await activateTaskWindow(taskWindow.hwnd, taskWindow.isActive);
      }
    } else if (result.kind === 'folder' || result.kind === 'file') {
      await openShellPath(result.path ?? result.id.replace('folder:', ''));
    } else if (result.id === 'command:refresh-search') {
      await loadSearchCatalog();
      scheduleSystemSearch(searchQuery);
    } else if (result.id === 'command:hide-search') {
      await closePanel();
      return;
    }

    searchQuery = '';
    selectedIndex = 0;
    await closePanel();
    await loadSearchCatalog();
  }

  function handleSearchInput(event: Event) {
    searchQuery = (event.currentTarget as HTMLInputElement).value;
    selectedIndex = 0;
    scheduleSystemSearch(searchQuery);
    void openPanel();
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, Math.max(searchResults.length - 1, 0));
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      void activateResult(searchResults[selectedIndex]);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      void closePanel();
    }
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      searchInput.focus();
      scheduleSystemSearch(searchQuery);
      void openPanel();
    }
  }

  function selectSearchResult(resultId: string) {
    const index = searchResults.findIndex((result) => result.id === resultId);
    if (index >= 0) {
      selectedIndex = index;
    }
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    const timer = window.setInterval(() => {
      now = new Date();
    }, 1_000);
    const searchRefreshTimer = window.setInterval(() => {
      void listOpenTaskWindows().then((windows) => {
        openWindows = windows;
      });
    }, 1_000);
    const runtimeMetricsTimer = window.setTimeout(() => {
      void reportShellSurfaceRuntimeMetrics('top-bar').catch((error) => {
        console.error('Top bar runtime metrics failed', error);
      });
    }, 250);
    void loadSearchCatalog();
    void loadStackPins();
    void listen<string>(SEARCH_PANEL_ACTIVATE_EVENT, (event) => {
      markSearchPanelInteraction();
      void activateResult(searchResults.find((result) => result.id === event.payload));
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void listen<string>(SEARCH_PANEL_SELECT_EVENT, (event) => {
      markSearchPanelInteraction();
      selectSearchResult(event.payload);
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void listen<string>(SEARCH_PANEL_PIN_FOLDER_EVENT, (event) => {
      markSearchPanelInteraction();
      void pinSearchFolder(event.payload).catch((error) => {
        console.error('Failed to pin stack folder', error);
      });
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void listen(SEARCH_PANEL_INTERACTION_EVENT, () => {
      markSearchPanelInteraction();
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void listen(SEARCH_PANEL_CLOSED_EVENT, () => {
      cancelSearchBlurClose();
      searchOpen = false;
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void listen<StackPin[]>(STACK_PINS_UPDATED_EVENT, (event) => {
      void applyStackPins(event.payload);
    }, {
      target: topBarWebviewWindowEventTarget()
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void listen<TopBarPinMenuActionPayload>(TOP_BAR_PIN_MENU_ACTION_EVENT, (event) => {
      if (event.payload.action === 'open') {
        void openStackPath(event.payload.path, queryPinButton(event.payload.path));
      } else if (event.payload.action === 'unpin') {
        void unpinFromMenu(event.payload.path);
      }
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === 'drop') {
        void pinAndOpenDroppedFolders(event.payload.paths, pinRailEl);
      }
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void listen(SEARCH_INDEX_REFRESHED_EVENT, () => {
      if (shouldRefreshSystemSearchAfterIndexUpdate(searchOpen, searchQuery)) {
        if (systemSearchTimer !== null) {
          window.clearTimeout(systemSearchTimer);
          systemSearchTimer = null;
        }
        if (systemSearchRefreshTimer !== null) {
          window.clearTimeout(systemSearchRefreshTimer);
          systemSearchRefreshTimer = null;
        }

        systemSearchSequence += 1;
        void loadSystemSearchResults(searchQuery.trim(), systemSearchSequence, 1);
      }
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });

    return () => {
      window.clearInterval(timer);
      window.clearInterval(searchRefreshTimer);
      window.clearTimeout(runtimeMetricsTimer);
      if (systemSearchTimer !== null) {
        window.clearTimeout(systemSearchTimer);
      }
      if (systemSearchRefreshTimer !== null) {
        window.clearTimeout(systemSearchRefreshTimer);
      }
      if (pinDropStatusTimer !== null) {
        window.clearTimeout(pinDropStatusTimer);
      }
      cancelSearchBlurClose();
      void hideSearchPanel().catch(() => undefined);
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<svelte:window on:keydown={handleGlobalKeydown} on:pointerdown={handleTopBarPointerDown} />

<div class="surface top-bar">
  <button
    class="shell-home-button"
    type="button"
    aria-label="JasonShell Home: open command search"
    on:click={() => {
      searchInput?.focus();
      void openPanel();
    }}
  >
    jasonshell
  </button>
  <div class="rail-wrap">
    {#if showRailScrollLeft}
      <button type="button" class="rail-scroll left" aria-label="Scroll pinned folders left" on:click={scrollRailLeft}>&lsaquo;</button>
    {/if}
    <div
      class="stack-pins"
      class:stack-pins-hover={pinRailHover}
      role="toolbar"
      tabindex="0"
      bind:this={pinRailEl}
      aria-label="Pinned folders"
      aria-orientation="horizontal"
      on:dragenter={handlePinRailDragOver}
      on:dragover={handlePinRailDragOver}
      on:dragleave={handlePinRailDragLeave}
      on:drop={handlePinRailDrop}
      on:wheel={handleRailWheel}
      on:keydown={handlePinRailKeydown}
    >
      {#each stackPins as pin, index (pin.id)}
        <button
          type="button"
          title={pin.path}
          data-path={pin.path}
          draggable="true"
          aria-label={`Open pinned folder ${pin.name}`}
          aria-haspopup="dialog"
          class:dragging={draggingPinPath === pin.path}
          on:click={(event) => handlePinClick(event, pin, index)}
          on:contextmenu={(event) => handlePinContextMenu(event, pin)}
          on:dragstart={(event) => handlePinDragStart(event, pin)}
          on:dragend={handlePinDragEnd}
          on:dragover={(event) => handlePinDragOver(event, pin)}
          on:drop={(event) => void handlePinDrop(event, pin)}
        >
          {pin.name}
        </button>
      {/each}
      {#if pinRailHover}
        <div class="pin-drop-overlay" aria-hidden="true">Drop to pin folder</div>
      {/if}
      {#if pinDropStatus}
        <div class:error={pinDropStatusKind === 'error'} class="pin-drop-status" role="status" aria-live="polite">{pinDropStatus}</div>
      {/if}
    </div>
    {#if showRailScrollRight}
      <button type="button" class="rail-scroll right" aria-label="Scroll pinned folders right" on:click={scrollRailRight}>&rsaquo;</button>
    {/if}
  </div>
  <div
    class="time-pill"
    aria-label={`Current time ${timeFormatter.format(now)} on ${dateFormatter.format(now)}`}
  >
    <strong>{timeFormatter.format(now)}</strong>
    <span>{dateFormatter.format(now)}</span>
  </div>
  <div class="search-control" bind:this={searchControl}>
    <input
      bind:this={searchInput}
      aria-label="Search apps, windows, files, folders, and commands"
      aria-expanded={searchOpen}
      aria-haspopup="listbox"
      autocomplete="off"
      placeholder="Search"
      value={searchQuery}
      on:focus={() => void openPanel()}
      on:blur={scheduleSearchBlurClose}
      on:input={handleSearchInput}
      on:keydown={handleSearchKeydown}
    />
    <span aria-hidden="true">Ctrl K</span>
  </div>
</div>
