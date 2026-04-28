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
    type ProcessKillConfirmation,
    type ProcessInfo
  } from '../lib/processManager';
  import {
    formatProcessCpu,
    formatProcessMemory,
    formatProcessPorts,
    formatProcessStartTime,
    nextProcessSortState,
    processDeveloperSummary,
    sortProcesses,
    type ProcessSortColumn,
    type ProcessSortState
  } from '../lib/processManagerState';
  import {
    buildProcessKillPlan,
    buildProcessTreeRows,
    filterProcesses,
    processMetricPercent,
    safeKillButtonState
  } from '../features/process-manager/processManagerUxState';

  const REFRESH_INTERVAL_MS = 1_000;

  let processes: ProcessInfo[] = [];
  let sortState: ProcessSortState = { column: 'cpuPercent', direction: 'desc' };
  let statusMessage = 'Waiting for process snapshot…';
  let isOpen = false;
  let isLoading = false;
  let refreshTimer: number | null = null;
  let killingPid: number | null = null;
  let armedKillPid: number | null = null;
  let processFilter = '';
  let inFlightRequest = 0;

  $: sortedProcesses = sortProcesses(processes, sortState);
  $: visibleProcesses = filterProcesses(sortedProcesses, processFilter);
  $: processRows = buildProcessTreeRows(visibleProcesses);
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

  function ariaSort(column: ProcessSortColumn) {
    if (sortState.column !== column) {
      return 'none';
    }
    return sortState.direction === 'asc' ? 'ascending' : 'descending';
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
      if (armedKillPid !== null && !nextProcesses.some((process) => process.pid === armedKillPid)) {
        armedKillPid = null;
      }
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
    armedKillPid = null;
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
    const killPlan = buildProcessKillPlan(processes, process, false);
    if (armedKillPid !== process.pid) {
      armedKillPid = process.pid;
      statusMessage = killPlan.warnings.length
        ? `Review single-process kill for ${process.name}: ${killPlan.warnings[0]}`
        : `Confirm single-process kill for ${process.name} (${process.pid})`;
      return;
    }
    if (!killPlan.canExecute) {
      statusMessage = `Kill is guarded for ${process.name} (${process.pid})`;
      armedKillPid = null;
      return;
    }
    killingPid = process.pid;
    armedKillPid = null;
    statusMessage = `Killing ${process.name} (${process.pid})…`;
    try {
      await killProcess(process.pid, killConfirmationFromPlan(killPlan));
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

  function clearKillArm() {
    if (armedKillPid !== null) {
      armedKillPid = null;
      statusMessage = 'Kill confirmation canceled';
    }
  }

  function processRowStyle(rowDepth: number, process: ProcessInfo) {
    const cpuFill = processMetricPercent(process.cpuPercent, 100);
    const memoryFill = processMetricPercent(process.memoryBytes, totalMemoryBytes);
    return `--process-depth: ${rowDepth}; --cpu-fill: ${cpuFill}%; --memory-fill: ${memoryFill}%;`;
  }

  function killConfirmationFromPlan(killPlan: ReturnType<typeof buildProcessKillPlan>): ProcessKillConfirmation {
    return {
      confirmedTargetPid: killPlan.targetPid,
      mode: killPlan.mode,
      affectedPids: killPlan.affectedPids,
      descendantPids: killPlan.descendantPids,
      acknowledgedWarningCount: killPlan.warnings.length,
      requiresSecondConfirmation: killPlan.requiresSecondConfirmation,
      canExecute: killPlan.canExecute
    };
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

<svelte:window
  on:keydown={(event) => {
    if (event.key === 'Escape' && armedKillPid !== null) {
      clearKillArm();
      return;
    }
    if (event.key === 'Escape') {
      void requestClose();
    }
  }}
/>

<main class="process-manager-surface" aria-label="Process manager">
  <header class="process-manager-header">
    <div>
      <strong>Processes</strong>
      <span role="status" aria-live="polite">{statusMessage}</span>
    </div>
    <div class="process-manager-actions">
      <label class="process-filter">
        <span>Filter</span>
        <input
          bind:value={processFilter}
          aria-label="Filter processes by name, PID, parent, path, command line, port, workspace, or status"
          placeholder="name, pid, port, workspace"
          on:input={() => { armedKillPid = null; }}
        />
      </label>
      <button type="button" on:click={() => void refreshProcesses()} disabled={isLoading}>
        {isLoading ? 'Refreshing…' : 'Refresh'}
      </button>
      <button type="button" on:click={() => void requestClose()}>Close</button>
    </div>
  </header>

  <div class="process-table" role="grid" tabindex="0" aria-label="Running processes" aria-live={isOpen ? 'polite' : 'off'}>
    <div class="process-row process-row-head" role="row">
      <button type="button" role="columnheader" aria-sort={ariaSort('name')} on:click={() => sortBy('name')}>Name{sortIndicator('name')}</button>
      <button type="button" role="columnheader" aria-sort={ariaSort('pid')} on:click={() => sortBy('pid')}>PID{sortIndicator('pid')}</button>
      <button type="button" role="columnheader" aria-sort={ariaSort('cpuPercent')} on:click={() => sortBy('cpuPercent')}>CPU{sortIndicator('cpuPercent')}</button>
      <button type="button" role="columnheader" aria-sort={ariaSort('memoryBytes')} on:click={() => sortBy('memoryBytes')}>Memory{sortIndicator('memoryBytes')}</button>
      <button type="button" role="columnheader" aria-sort={ariaSort('startTimeMs')} on:click={() => sortBy('startTimeMs')}>Start Time{sortIndicator('startTimeMs')}</button>
      <button type="button" role="columnheader" aria-sort={ariaSort('threadCount')} on:click={() => sortBy('threadCount')}>Threads{sortIndicator('threadCount')}</button>
      <span role="columnheader">Status</span>
      <span role="columnheader">Action</span>
    </div>

    <div class="process-table-body" role="rowgroup">
      {#if processRows.length}
        {#each processRows as row (row.process.pid)}
          {@const process = row.process}
          {@const killState = safeKillButtonState(process, armedKillPid, killingPid)}
          {@const developerSummary = processDeveloperSummary(process)}
          {@const portsLabel = formatProcessPorts(process.listeningPorts)}
          <div
            class:process-row-armed={killState.isArmed}
            class="process-row"
            role="row"
            title={process.commandLine ?? process.executablePath ?? process.name}
            style={processRowStyle(row.depth, process)}
          >
            <span class="process-name-cell" role="gridcell">
              <span class="process-name">
                <span class="process-tree-indent" aria-hidden="true"></span>
                <span class="process-name-copy">{process.name}</span>
                {#if process.workspaceHint}
                  <span class="process-workspace" aria-label={`Workspace hint ${process.workspaceHint.label}`}>{process.workspaceHint.label}</span>
                {/if}
                {#if row.childCount}
                  <span class="process-child-count" aria-label={`${row.childCount} visible child processes`}>{row.childCount}</span>
                {/if}
                {#if (process.descendantProcessCount ?? 0) > 0}
                  <span class="process-tree-guard" aria-label={`${process.descendantProcessCount} descendant processes; tree kill is guarded`}>tree</span>
                {/if}
                {#if portsLabel !== '—'}
                  <span class="process-port-count" aria-label={`Listening ports ${portsLabel}`}>{portsLabel}</span>
                {/if}
              </span>
              {#if developerSummary}
                <span class="process-meta">{developerSummary}</span>
              {/if}
            </span>
            <span class="process-number" role="gridcell">{process.pid}</span>
            <span class="process-number process-meter" role="gridcell">
              <span aria-hidden="true"></span>
              <strong>{formatProcessCpu(process.cpuPercent)}</strong>
            </span>
            <span class="process-number process-meter memory" role="gridcell">
              <span aria-hidden="true"></span>
              <strong>{formatProcessMemory(process.memoryBytes)}</strong>
            </span>
            <span class="process-number" role="gridcell">{formatProcessStartTime(process.startTimeMs)}</span>
            <span class="process-number" role="gridcell">{process.threadCount ?? '—'}</span>
            <span class="process-status" role="gridcell">{process.status}</span>
            <span class="process-action" role="gridcell">
              <button
                class="kill-button"
                type="button"
                aria-label={killState.ariaLabel}
                disabled={killState.disabled}
                on:click={() => void killRow(process)}
              >
                {killState.label}
              </button>
            </span>
          </div>
        {/each}
      {:else}
        <div class="process-empty surface-state" class:loading={isLoading} class:info={!isLoading} role="status">
          {isLoading ? 'Loading processes…' : (processFilter ? 'No processes match this filter' : statusMessage)}
        </div>
      {/if}
    </div>
  </div>
</main>
