<script lang="ts">
  import './ProcessManagerSurface.css';
  import { onMount } from 'svelte';
  import { listen, type Event } from '@tauri-apps/api/event';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import MeltProgress from './melt/MeltProgress.svelte';
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
    aggregateProcessMetrics,
    formatProcessCpu,
    formatProcessGpu,
    formatProcessMemory,
    formatProcessMemoryPercent,
    formatProcessPorts,
    formatProcessStartTime,
    formatProcessThreadCount,
    isVolatileProcessSortColumn,
    nextProcessSortState,
    orderProcessRefresh,
    processDeveloperSummary,
    sortProcesses,
    type ProcessGroupId,
    type ProcessSortColumn,
    type ProcessSortState
  } from '../lib/processManagerState';
  import { listTaskbarProcessWindows } from '../lib/taskbarWindows';
  import {
    buildProcessGroups,
    buildProcessKillPlan,
    enrichProcessesWithTaskbarWindows,
    filterProcesses,
    isProcessGroupExpanded,
    killConfirmationFromPlan,
    processKillErrorMessage,
    processMetricPercent,
    safeKillButtonState,
    taskbarActiveProcessIds,
    toggleProcessGroupExpansion,
    type ProcessGroupExpansionState
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
  let taskbarProcessPids: number[] = [];
  let processGroupExpansionState: ProcessGroupExpansionState = {};

  $: visibleProcesses = filterProcesses(processes, processFilter);
  $: metricAggregates = aggregateProcessMetrics(visibleProcesses);
  $: processGroups = buildProcessGroups(visibleProcesses, { taskbarActivePids: taskbarProcessPids });
  $: visibleProcessCount = processGroups.reduce((total, group) => total + group.rows.length, 0);

  function sortBy(column: ProcessSortColumn) {
    sortState = nextProcessSortState(sortState, column);
    processes = sortProcesses(processes, sortState, { taskbarActivePids: taskbarProcessPids });
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

  async function refreshProcesses(options: { preserveVolatileOrder?: boolean } = {}) {
    if (isLoading) {
      return;
    }

    const requestId = ++inFlightRequest;
    isLoading = true;
    const previousProcesses = processes;
    const shouldPreserveOrder = options.preserveVolatileOrder !== false
      && isVolatileProcessSortColumn(sortState.column)
      && previousProcesses.length > 0;
    try {
      const [nextProcesses, taskbarWindows] = await Promise.all([
        listProcesses(),
        listTaskbarProcessWindows().catch((error) => {
          console.warn('Failed to load taskbar process metadata', error);
          return [];
        })
      ]);
      if (requestId !== inFlightRequest) {
        return;
      }
      const nextTaskbarProcessPids = taskbarActiveProcessIds(taskbarWindows);
      const enrichedProcesses = enrichProcessesWithTaskbarWindows(nextProcesses, taskbarWindows);
      taskbarProcessPids = nextTaskbarProcessPids;
      processes = orderProcessRefresh(previousProcesses, enrichedProcesses, sortState, {
        preserveExistingOrder: shouldPreserveOrder,
        taskbarActivePids: nextTaskbarProcessPids
      });
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

  function openSurface(event?: Event<number | null>) {
    const focusPid = typeof event?.payload === 'number' ? event.payload : null;
    isOpen = true;
    if (focusPid !== null) {
      processFilter = String(focusPid);
      sortState = { column: 'pid', direction: 'asc' };
      statusMessage = `Focused PID ${focusPid}`;
    }
    startRefreshTimer();
    void refreshProcesses({ preserveVolatileOrder: false });
  }

  function closeSurface() {
    isOpen = false;
    armedKillPid = null;
    inFlightRequest += 1;
    isLoading = false;
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
      await refreshProcesses({ preserveVolatileOrder: false });
    } catch (error) {
      console.error(`Failed to kill process ${process.pid}`, error);
      statusMessage = `Could not kill ${process.name} (${process.pid}): ${processKillErrorMessage(error)}`;
      await refreshProcesses({ preserveVolatileOrder: false });
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

  function processRowStyle(rowDepth: number) {
    return `--process-depth: ${rowDepth};`;
  }

  function processGroupBodyId(groupId: ProcessGroupId) {
    return `process-group-${groupId}-details`;
  }

  function isGroupExpanded(groupId: ProcessGroupId) {
    return isProcessGroupExpanded(groupId, processGroupExpansionState);
  }

  function toggleGroup(groupId: ProcessGroupId) {
    processGroupExpansionState = toggleProcessGroupExpansion(groupId, processGroupExpansionState);
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    let disposed = false;
    const registerAsyncUnlistener = (registration: Promise<() => void>) => {
      void registration.then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlisteners.push(unlisten);
      });
    };

    registerAsyncUnlistener(listen(PROCESS_MANAGER_OPEN_EVENT, openSurface));
    registerAsyncUnlistener(listen(PROCESS_MANAGER_CLOSED_EVENT, closeSurface));

    return () => {
      disposed = true;
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
      <MeltActionButton onClick={() => void refreshProcesses({ preserveVolatileOrder: false })}>
        Refresh
      </MeltActionButton>
    </div>
    <MeltActionButton
      class="process-manager-close-button"
      ariaLabel="Close process manager"
      onClick={() => void requestClose()}
    >×</MeltActionButton>
  </header>

  <div class="process-table">
    <div class="process-table-scroll" role="grid" tabindex="0" aria-label="Running processes" aria-live={isOpen ? 'polite' : 'off'}>
      <div class="process-table-content" role="presentation">
        <div class="process-row process-row-head" role="row">
          <MeltActionButton role="columnheader" ariaSort={ariaSort('name')} onClick={() => sortBy('name')}>Name{sortIndicator('name')}</MeltActionButton>
          <MeltActionButton role="columnheader" ariaSort={ariaSort('pid')} onClick={() => sortBy('pid')}>PID{sortIndicator('pid')}</MeltActionButton>
          <MeltActionButton role="columnheader" ariaSort={ariaSort('cpuPercent')} onClick={() => sortBy('cpuPercent')}>
            <span class="process-header-metric">
              <strong>{formatProcessCpu(metricAggregates.cpuPercent)}</strong>
              <span>CPU{sortIndicator('cpuPercent')}</span>
            </span>
          </MeltActionButton>
          <MeltActionButton role="columnheader" ariaSort={ariaSort('memoryBytes')} onClick={() => sortBy('memoryBytes')}>
            <span class="process-header-metric">
              <strong>{formatProcessMemoryPercent(metricAggregates.memoryPercent)}</strong>
              <span>Memory{sortIndicator('memoryBytes')}</span>
            </span>
          </MeltActionButton>
          <MeltActionButton role="columnheader" ariaSort={ariaSort('gpuPercent')} onClick={() => sortBy('gpuPercent')}>
            <span class="process-header-metric">
              <strong>{formatProcessGpu(metricAggregates.gpuPercent)}</strong>
              <span>GPU{sortIndicator('gpuPercent')}</span>
            </span>
          </MeltActionButton>
          <MeltActionButton role="columnheader" ariaSort={ariaSort('startTimeMs')} onClick={() => sortBy('startTimeMs')}>Start Time{sortIndicator('startTimeMs')}</MeltActionButton>
          <MeltActionButton role="columnheader" ariaSort={ariaSort('threadCount')} onClick={() => sortBy('threadCount')}>
            <span class="process-header-metric">
              <strong>{formatProcessThreadCount(metricAggregates.threadCount)}</strong>
              <span>Threads{sortIndicator('threadCount')}</span>
            </span>
          </MeltActionButton>
          <span role="columnheader">Status</span>
          <span role="columnheader">Action</span>
        </div>

        <div class="process-table-body">
          {#if visibleProcessCount || processes.length}
            {#each processGroups as group (group.id)}
              {@const groupExpanded = isGroupExpanded(group.id)}
              <section class="process-group" role="rowgroup" aria-label={group.label}>
                <div class="process-row process-group-row" role="row">
                  <span class="process-group-title" role="gridcell">
                    <MeltActionButton
                      class="process-group-toggle"
                      ariaExpanded={groupExpanded}
                      ariaControls={processGroupBodyId(group.id)}
                      title={`${groupExpanded ? 'Collapse' : 'Expand'} ${group.label}`}
                      onClick={() => toggleGroup(group.id)}
                    >
                      <span class="process-group-caret" aria-hidden="true">{groupExpanded ? '▾' : '▸'}</span>
                      <strong>{group.label}</strong>
                      <span>{group.rows.length}</span>
                    </MeltActionButton>
                  </span>
                </div>
                <div class="process-group-detail" id={processGroupBodyId(group.id)} hidden={!groupExpanded}>
                  {#if group.rows.length}
                    {#each group.rows as row (row.process.pid)}
                      {@const process = row.process}
                      {@const killState = safeKillButtonState(process, armedKillPid, killingPid)}
                      {@const developerSummary = processDeveloperSummary(process)}
                      {@const portsLabel = formatProcessPorts(process.listeningPorts)}
                      {@const memoryPercent = formatProcessMemoryPercent(process.memoryPercent)}
                      <div
                        class:process-row-armed={killState.isArmed}
                        class="process-row"
                        role="row"
                        title={process.commandLine ?? process.executablePath ?? process.name}
                        style={processRowStyle(row.depth)}
                      >
                        <span class="process-name-cell" role="gridcell">
                          <span class="process-name">
                            <span class="process-icon-shell" aria-hidden="true">
                              {#if process.iconDataUrl}
                                <img class="process-icon" src={process.iconDataUrl} alt="" draggable="false" />
                              {:else}
                                <span class="process-icon process-icon-fallback"></span>
                              {/if}
                            </span>
                            <span class="process-tree-indent" aria-hidden="true"></span>
                            <span class="process-name-copy">{process.name}</span>
                            {#if process.workspaceHint}
                              <span class="process-workspace" aria-label={`Workspace hint ${process.workspaceHint.label}`}>{process.workspaceHint.label}</span>
                            {/if}
                            {#if row.childCount}
                              <span class="process-child-count" aria-label={`${row.childCount} visible child processes`}>{row.childCount}</span>
                            {/if}
                          </span>
                          {#if developerSummary}
                            <span class="process-meta">{developerSummary}</span>
                          {/if}
                        </span>
                          <span class="process-number" role="gridcell">{process.pid}</span>
                          <span class="process-number process-meter" role="gridcell">
                            <MeltProgress
                              value={processMetricPercent(process.cpuPercent, 100)}
                              label={`CPU ${formatProcessCpu(process.cpuPercent)}`}
                            />
                            <strong>{formatProcessCpu(process.cpuPercent)}</strong>
                          </span>
                          <span class="process-number process-meter memory" role="gridcell">
                            <MeltProgress
                              value={processMetricPercent(process.memoryPercent, 100)}
                              label={`Memory ${memoryPercent} (${formatProcessMemory(process.memoryBytes)})`}
                              tone="memory"
                            />
                            <span class="process-meter-value">
                              <strong>{memoryPercent}</strong>
                              <small>{formatProcessMemory(process.memoryBytes)}</small>
                            </span>
                          </span>
                          <span class="process-number process-meter gpu" role="gridcell">
                            <MeltProgress
                              value={processMetricPercent(process.gpuPercent, 100)}
                              label={`GPU ${formatProcessGpu(process.gpuPercent)}`}
                              tone="gpu"
                            />
                            <strong>{formatProcessGpu(process.gpuPercent)}</strong>
                          </span>
                          <span class="process-number" role="gridcell">{formatProcessStartTime(process.startTimeMs)}</span>
                          <span class="process-number" role="gridcell">{process.threadCount ?? '—'}</span>
                          <span class="process-status" role="gridcell">{process.status}</span>
                          <span class="process-action" role="gridcell">
                            <MeltActionButton
                              class="kill-button"
                              ariaLabel={killState.ariaLabel}
                              disabled={killState.disabled}
                              onClick={() => void killRow(process)}
                            >
                              {killState.label}
                            </MeltActionButton>
                          </span>
                        </div>
                      {/each}
                  {:else}
                    <div class="process-row process-group-empty" role="row">
                      <span role="gridcell">{processFilter ? `No matching ${group.label.toLocaleLowerCase()}` : group.emptyMessage}</span>
                    </div>
                  {/if}
                </div>
              </section>
            {/each}
          {:else}
            <div class="process-empty surface-state" class:loading={isLoading} class:info={!isLoading} role="status">
              {isLoading ? 'Loading processes…' : (processFilter ? 'No processes match this filter' : statusMessage)}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</main>
