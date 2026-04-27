<script lang="ts">
  import './ProcessManagerSurface.css';
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    hideProcessManager,
    killProcess,
    listProcesses,
    PROCESS_MANAGER_CLOSED_EVENT,
    PROCESS_MANAGER_OPEN_EVENT,
    type ProcessInfo
  } from '../lib/processManager';
  import {
    formatProcessCpu,
    formatProcessMemory,
    formatProcessStartTime,
    nextProcessSortState,
    sortProcesses,
    type ProcessSortColumn,
    type ProcessSortState
  } from '../lib/processManagerState';

  const REFRESH_INTERVAL_MS = 1_000;

  let processes: ProcessInfo[] = [];
  let sortState: ProcessSortState = { column: 'cpuPercent', direction: 'desc' };
  let statusMessage = 'Waiting for process snapshot…';
  let isOpen = false;
  let isLoading = false;
  let refreshTimer: number | null = null;
  let killingPid: number | null = null;
  let inFlightRequest = 0;

  $: sortedProcesses = sortProcesses(processes, sortState);
  $: totalMemoryBytes = processes.reduce((total, process) => total + (process.memoryBytes ?? 0), 0);

  function sortBy(column: ProcessSortColumn) {
    sortState = nextProcessSortState(sortState, column);
  }

  function sortIndicator(column: ProcessSortColumn) {
    if (sortState.column !== column) {
      return '';
    }
    return sortState.direction === 'asc' ? ' ▲' : ' ▼';
  }

  async function refreshProcesses() {
    if (isLoading) {
      return;
    }

    const requestId = ++inFlightRequest;
    isLoading = true;
    try {
      const nextProcesses = await listProcesses();
      if (requestId !== inFlightRequest) {
        return;
      }
      processes = nextProcesses;
      const nextTotalMemoryBytes = nextProcesses.reduce(
        (total, process) => total + (process.memoryBytes ?? 0),
        0
      );
      statusMessage = `${nextProcesses.length} processes • ${formatProcessMemory(nextTotalMemoryBytes)} working set`;
    } catch (error) {
      console.error('Failed to refresh processes', error);
      if (requestId === inFlightRequest) {
        statusMessage = 'Process list unavailable';
      }
    } finally {
      if (requestId === inFlightRequest) {
        isLoading = false;
      }
    }
  }

  function stopRefreshTimer() {
    if (refreshTimer !== null) {
      window.clearInterval(refreshTimer);
      refreshTimer = null;
    }
  }

  function startRefreshTimer() {
    stopRefreshTimer();
    refreshTimer = window.setInterval(() => {
      void refreshProcesses();
    }, REFRESH_INTERVAL_MS);
  }

  function openSurface() {
    isOpen = true;
    startRefreshTimer();
    void refreshProcesses();
  }

  function closeSurface() {
    isOpen = false;
    stopRefreshTimer();
  }

  async function requestClose() {
    closeSurface();
    await hideProcessManager().catch((error) => {
      console.error('Failed to hide process manager', error);
    });
  }

  async function killRow(process: ProcessInfo) {
    if (!process.isKillable || killingPid !== null) {
      return;
    }
    killingPid = process.pid;
    statusMessage = `Killing ${process.name} (${process.pid})…`;
    try {
      await killProcess(process.pid);
      statusMessage = `Killed ${process.name} (${process.pid})`;
      await refreshProcesses();
    } catch (error) {
      console.error(`Failed to kill process ${process.pid}`, error);
      statusMessage = `Could not kill ${process.name} (${process.pid})`;
      await refreshProcesses();
    } finally {
      killingPid = null;
    }
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];

    void listen(PROCESS_MANAGER_OPEN_EVENT, openSurface).then((unlisten) => {
      unlisteners.push(unlisten);
    });
    void listen(PROCESS_MANAGER_CLOSED_EVENT, closeSurface).then((unlisten) => {
      unlisteners.push(unlisten);
    });

    return () => {
      closeSurface();
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<svelte:window on:keydown={(event) => event.key === 'Escape' && void requestClose()} />

<main class="process-manager-surface" aria-label="Process manager">
  <header class="process-manager-header">
    <div>
      <strong>Processes</strong>
      <span>{statusMessage}</span>
    </div>
    <div class="process-manager-actions">
      <button type="button" on:click={() => void refreshProcesses()} disabled={isLoading}>
        {isLoading ? 'Refreshing…' : 'Refresh'}
      </button>
      <button type="button" on:click={() => void requestClose()}>Close</button>
    </div>
  </header>

  <section class="process-table" aria-live={isOpen ? 'polite' : 'off'}>
    <div class="process-row process-row-head" role="row">
      <button type="button" on:click={() => sortBy('name')}>Name{sortIndicator('name')}</button>
      <button type="button" on:click={() => sortBy('pid')}>PID{sortIndicator('pid')}</button>
      <button type="button" on:click={() => sortBy('cpuPercent')}>CPU{sortIndicator('cpuPercent')}</button>
      <button type="button" on:click={() => sortBy('memoryBytes')}>Memory{sortIndicator('memoryBytes')}</button>
      <button type="button" on:click={() => sortBy('startTimeMs')}>Start Time{sortIndicator('startTimeMs')}</button>
      <button type="button" on:click={() => sortBy('threadCount')}>Threads{sortIndicator('threadCount')}</button>
      <span>Status</span>
      <span>Action</span>
    </div>

    <div class="process-table-body">
      {#if sortedProcesses.length}
        {#each sortedProcesses as process (process.pid)}
          <div class="process-row" role="row" title={process.executablePath ?? process.name}>
            <span class="process-name">{process.name}</span>
            <span class="process-number">{process.pid}</span>
            <span class="process-number">{formatProcessCpu(process.cpuPercent)}</span>
            <span class="process-number">{formatProcessMemory(process.memoryBytes)}</span>
            <span class="process-number">{formatProcessStartTime(process.startTimeMs)}</span>
            <span class="process-number">{process.threadCount ?? '—'}</span>
            <span class="process-status">{process.status}</span>
            <button
              class="kill-button"
              type="button"
              disabled={!process.isKillable || killingPid !== null}
              on:click={() => void killRow(process)}
            >
              {killingPid === process.pid ? 'Killing…' : 'Kill'}
            </button>
          </div>
        {/each}
      {:else}
        <div class="process-empty">{isLoading ? 'Loading processes…' : statusMessage}</div>
      {/if}
    </div>
  </section>
</main>
