<script lang="ts">
  import './TopBar.css';
  import { onMount } from 'svelte';
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
    SEARCH_PANEL_PIN_FOLDER_EVENT,
    SEARCH_PANEL_SELECT_EVENT,
    showSearchPanel,
    type SearchPanelResult
  } from '../lib/searchPanel';
  import {
    listStackPins,
    pinStackFolder,
    unpinStackFolder,
    showStackPopup,
    type StackPin
  } from '../lib/stackPopup';
  import { folderPathFromTransfer, hasFolderDragPayload } from '../lib/folderDrag';
  import { buildSearchCatalog } from '../lib/searchCatalog';
  import { rankSearchResults, recordSearchUsage } from '../lib/searchRanking';
  import { isSystemPathResult, searchSystem, SEARCH_INDEX_REFRESHED_EVENT } from '../lib/systemSearch';
  import {
    shouldApplySystemSearchResponse,
    shouldRefreshSystemSearchAfterIndexUpdate,
    shouldRetryIndexedSearch
  } from '../lib/systemSearchState';

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

  $: allResults = buildSearchCatalog(launchers, openWindows, systemResults);
  $: searchResults = rankSearchResults(allResults, searchQuery);
  $: selectedIndex = Math.min(selectedIndex, Math.max(searchResults.length - 1, 0));
  $: searchPanelPayload = {
    query: searchQuery,
    results: searchResults,
    selectedIndex,
    statusMessage: searchStatus
  };
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
      stackPins = await listStackPins();
    } catch (error) {
      console.error('Failed to load stack pins', error);
      stackPins = [];
    }
  }

  async function pinSearchFolder(folderPath: string) {
    try {
      await pinStackFolder(folderPath);
      await loadStackPins();
    } catch (error) {
      console.error('Failed to pin stack folder', error);
    }
  }

  async function pinDroppedFolders(paths: string[]) {
    for (const path of paths) {
      await pinStackFolder(path).catch((error) => {
        console.error(`Failed to pin dropped folder ${path}`, error);
      });
    }
    await loadStackPins();
  }

  async function openPanel() {
    searchOpen = true;
    const rect = searchControl.getBoundingClientRect();
    await showSearchPanel({
      anchorLeft: rect.left,
      anchorWidth: rect.width
    });
    await publishSearchResults();
  }

  async function closePanel() {
    searchOpen = false;
    await hideSearchPanel().catch((error) => {
      console.error('Failed to hide search panel', error);
    });
  }

  async function openStackFromPin(pin: StackPin, target: EventTarget | null) {
    await openStackPath(pin.path, target);
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

  function draggedFolderPath(event: DragEvent): string | null {
    return folderPathFromTransfer(event.dataTransfer);
  }

  async function pinAndDisplayFolder(path: string, target: EventTarget | null) {
    await pinSearchFolder(path);
    await openStackPath(path, target);
  }

  function handlePinRailDragOver(event: DragEvent) {
    if (!hasFolderDragPayload(event.dataTransfer)) {
      return;
    }
    event.preventDefault();
    pinRailHover = true;
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'copy';
    }
  }

  function handlePinRailDrop(event: DragEvent) {
    if (!hasFolderDragPayload(event.dataTransfer)) {
      pinRailHover = false;
      return;
    }

    event.preventDefault();
    const dt = event.dataTransfer;
    // Try to extract file paths from DataTransfer.files (Tauri/Electron may expose .path)
    const filePaths: string[] = [];
    if (dt?.files && dt.files.length) {
      for (const f of Array.from(dt.files as any)) {
        try {
          if (f && typeof (f as any).path === 'string' && (f as any).path) {
            filePaths.push((f as any).path);
          }
        } catch {
          // ignore
        }
      }
    }

    pinRailHover = false;
    if (filePaths.length) {
      void pinDroppedFolders(filePaths);
      return;
    }

    const path = draggedFolderPath(event);
    if (!path) {
      return;
    }
    void pinAndDisplayFolder(path, event.currentTarget);
  }

  function handlePinRailDragLeave() {
    pinRailHover = false;
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
    } else if (event.key === 'Enter') {
      const idx = focusedPinIndex ?? 0;
      const pin = stackPins[idx];
      if (pin) {
        void openStackFromPin(pin, pinRailEl?.querySelectorAll('button')[idx]);
      }
    }
  }

  function handlePinContextMenu(event: MouseEvent, path: string) {
    event.preventDefault();
    const name = path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
    if (window.confirm(`Unpin '${name}'?`)) {
      void unpinStackFolder(path).then(() => loadStackPins()).catch((error) => {
        console.error('Failed to unpin folder', error);
      });
    }
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
      void activateResult(searchResults.find((result) => result.id === event.payload));
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void listen<string>(SEARCH_PANEL_SELECT_EVENT, (event) => {
      selectSearchResult(event.payload);
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void listen<string>(SEARCH_PANEL_PIN_FOLDER_EVENT, (event) => {
      void pinSearchFolder(event.payload);
    }).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === 'drop') {
        void pinDroppedFolders(event.payload.paths);
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
      void hideSearchPanel().catch(() => undefined);
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

<div class="surface top-bar">
  <div class="rail-wrap">
    {#if showRailScrollLeft}
      <button type="button" class="rail-scroll left" aria-hidden="true" on:click={scrollRailLeft}>&lsaquo;</button>
    {/if}
    <div
      class="stack-pins"
      class:stack-pins-hover={pinRailHover}
      role="toolbar"
      tabindex="0"
      bind:this={pinRailEl}
      aria-label="Pinned folders"
      on:dragenter={handlePinRailDragOver}
      on:dragover={handlePinRailDragOver}
      on:dragleave={handlePinRailDragLeave}
      on:drop={handlePinRailDrop}
      on:wheel={handleRailWheel}
      on:keydown={handlePinRailKeydown}
    >
      {#each stackPins as pin (pin.id)}
        <button
          type="button"
          title={pin.path}
          data-path={pin.path}
          on:click={(event) => void openStackFromPin(pin, event.currentTarget)}
          on:contextmenu={(event) => handlePinContextMenu(event, pin.path)}
        >
          {pin.name}
        </button>
      {/each}
      {#if pinRailHover}
        <div class="pin-drop-overlay" aria-hidden="true">Drop to pin folder</div>
      {/if}
    </div>
    {#if showRailScrollRight}
      <button type="button" class="rail-scroll right" aria-hidden="true" on:click={scrollRailRight}>&rsaquo;</button>
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
      autocomplete="off"
      placeholder="Search"
      value={searchQuery}
      on:focus={() => void openPanel()}
      on:input={handleSearchInput}
      on:keydown={handleSearchKeydown}
    />
    <span aria-hidden="true">Ctrl K</span>
  </div>
</div>
