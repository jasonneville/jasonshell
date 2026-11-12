<script lang="ts">
  import './ProcessManagerSurface.css';
  import { onMount } from 'svelte';
  import { listen, type Event } from '@tauri-apps/api/event';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import MeltProgress from './melt/MeltProgress.svelte';
  import SystemInfoView from './SystemInfoView.svelte';
  import { getSystemInfoSnapshot, type SystemInfoSnapshot } from '../lib/systemInfo';
  import {
    hideProcessManager,
    killProcess,
    listProcesses,
    PROCESS_MANAGER_CLOSED_EVENT,
    PROCESS_MANAGER_OPEN_EVENT,
    getProcessManagerContext,
    setProcessManagerTab,
    type ProcessManagerContext,
    type ProcessManagerTab,
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

  let processes = $state.raw<ProcessInfo[]>([]);
  let sortState = $state<ProcessSortState>({ column: 'cpuPercent', direction: 'desc' });
  let statusMessage = $state('Waiting for process snapshot…');
  let isOpen = $state(false);
  let isLoading = $state(false);
  let refreshTimer: number | null = null;
  let killingPid = $state<number | null>(null);
  let armedKillPid = $state<number | null>(null);
  let processFilter = $state('');
  let inFlightRequest = 0;
  let taskbarProcessPids = $state.raw<number[]>([]);
  let processGroupExpansionState = $state<ProcessGroupExpansionState>({});
  let activeTab = $state<ProcessManagerTab>('processes');
  let context: ProcessManagerContext = { visible: false, activeTab: 'processes', openGeneration: 0, tabRevision: 0, focusPid: null };
  let snapshot = $state.raw<SystemInfoSnapshot | null>(null);
  let systemLoading = $state(false);
  let systemError = $state('');
  let disposed = false;
  let processOutstanding = false;
  let pendingRefresh: { preserveVolatileOrder?: boolean; announce?: boolean } | null = null;
  let systemOutstanding = false;
  let tabIntent = 0;
  let tabOutstanding = false;

  let visibleProcesses = $derived(filterProcesses(processes, processFilter));
  let metricAggregates = $derived(aggregateProcessMetrics(visibleProcesses));
  let processGroups = $derived(buildProcessGroups(visibleProcesses, { taskbarActivePids: taskbarProcessPids }));
  let visibleProcessCount = $derived(processGroups.reduce((total, group) => total + group.rows.length, 0));

  function current(request: number, tab: ProcessManagerTab) {
    return !disposed && isOpen && activeTab === tab && request === inFlightRequest;
  }

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

  async function refreshProcesses(options: { preserveVolatileOrder?: boolean; announce?: boolean } = {}) {
    if (!isOpen || activeTab !== 'processes' || context.activeTab !== 'processes' || disposed || document.visibilityState === 'hidden') return;
    if (processOutstanding) {
      pendingRefresh = {
        preserveVolatileOrder: pendingRefresh?.preserveVolatileOrder === false || options.preserveVolatileOrder === false ? false : undefined,
        announce: pendingRefresh?.announce === true || options.announce !== false
      };
      isLoading = true;
      return;
    }

    const requestId = ++inFlightRequest;
    processOutstanding = true;
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
      if (!current(requestId, 'processes')) {
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
      if (options.announce !== false) {
        statusMessage = `${nextProcesses.length} processes • ${formatProcessMemory(nextTotalMemoryBytes)} working set`;
      }
    } catch (error) {
      console.error('Failed to refresh processes', error);
      if (current(requestId, 'processes')) {
        statusMessage = 'Process list unavailable';
      }
    } finally {
      processOutstanding = false;
      if (current(requestId, 'processes')) {
        isLoading = false;
      }
      const pending = pendingRefresh;
      pendingRefresh = null;
      if (pending) void refreshProcesses(pending);
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
      if (document.visibilityState === 'hidden') return;
      if (activeTab === 'processes') void refreshProcesses({ announce: false });
      else void refreshSystemInfo();
    }, activeTab === 'processes' ? REFRESH_INTERVAL_MS : 2_000);
  }

  function reconcile(next: ProcessManagerContext) {
    if (disposed || next.openGeneration < context.openGeneration || next.tabRevision < context.tabRevision) return;
    const opening = next.openGeneration > context.openGeneration;
    if (!opening && next.tabRevision === context.tabRevision && next.visible === isOpen && next.activeTab === activeTab) return;
    context = next;
    ++tabIntent;
    ++inFlightRequest;
    pendingRefresh = null;
    isLoading = false;
    armedKillPid = null;
    snapshot = null;
    systemError = '';
    systemLoading = false;
    isOpen = next.visible;
    activeTab = next.activeTab;
    stopRefreshTimer();
    if (!isOpen) return;
    if (opening) {
      const focusPid = next.focusPid ?? null;
      processFilter = focusPid === null ? '' : String(focusPid);
      sortState = focusPid === null ? { column: 'cpuPercent', direction: 'desc' } : { column: 'pid', direction: 'asc' };
      statusMessage = focusPid === null ? 'Loading processes…' : `Focused PID ${focusPid}`;
      processes = [];
    }
    startRefreshTimer();
    if (activeTab === 'processes') void refreshProcesses({ preserveVolatileOrder: !opening, announce: !next.focusPid });
    else void refreshSystemInfo();
  }

  function openSurface(event: Event<{ focusPid: number | null; context: ProcessManagerContext }>) {
    if (event.payload?.context) reconcile({ ...event.payload.context, focusPid: event.payload.focusPid });
  }

  async function recoverContext() {
    const intent = tabIntent;
    try {
      const next = await getProcessManagerContext();
      if (intent === tabIntent) reconcile(next);
    }
    catch {
      if (disposed || intent !== tabIntent) return;
      if (activeTab === 'system_info') systemError = 'System information context unavailable. Reopen Process Manager to retry.';
      else statusMessage = 'Process manager context unavailable. Reopen to retry.';
    }
  }

  async function selectTab(tab: ProcessManagerTab) {
    if (!isOpen || activeTab === tab) return;
    ++tabIntent;
    ++inFlightRequest;
    pendingRefresh = null;
    armedKillPid = null;
    isLoading = false;
    systemLoading = tab === 'system_info';
    systemError = '';
    snapshot = null;
    activeTab = tab;
    stopRefreshTimer();
    await commitTab();
  }

  async function commitTab() {
    if (tabOutstanding || disposed || !isOpen) return;
    tabOutstanding = true;
    const intent = tabIntent;
    const generation = context.openGeneration;
    const revision = context.tabRevision + 1;
    const tab = activeTab;
    try {
      const next = await setProcessManagerTab({ openGeneration: generation, tabRevision: revision, tab });
      if (disposed || generation !== context.openGeneration || !isOpen) return;
      if (intent === tabIntent || next.activeTab === activeTab) reconcile(next);
      else if (next.tabRevision >= context.tabRevision) context = next;
    } catch {
      if (intent === tabIntent && !disposed) await recoverContext();
    } finally {
      tabOutstanding = false;
      if (!disposed && isOpen && intent !== tabIntent && activeTab !== context.activeTab) void commitTab();
    }
  }

  async function refreshSystemInfo() {
    if (!isOpen || activeTab !== 'system_info' || context.activeTab !== 'system_info' || systemOutstanding || disposed || document.visibilityState === 'hidden') return;
    const request = inFlightRequest;
    const epoch = { openGeneration: context.openGeneration, tabRevision: context.tabRevision };
    systemOutstanding = true;
    systemLoading = snapshot === null;
    try {
      const next = await getSystemInfoSnapshot({ ...epoch, afterRevision: snapshot?.snapshotRevision ?? null });
      if (!current(request, 'system_info')) return;
      if (!next || next.openGeneration !== epoch.openGeneration || next.tabRevision !== epoch.tabRevision
        || next.snapshotRevision <= (snapshot?.snapshotRevision ?? -1)) return;
      if (next && next.schemaVersion !== 1) {
        snapshot = null;
        systemError = 'System information unavailable: unsupported snapshot version.';
        return;
      }
      snapshot = next;
      systemError = '';
    } catch {
      if (current(request, 'system_info')) systemError = 'System information unavailable. Retrying automatically.';
    } finally {
      systemOutstanding = false;
      if (current(request, 'system_info')) systemLoading = false;
      else if (!disposed && isOpen && activeTab === 'system_info') void refreshSystemInfo();
    }
  }

  function closeSurface() {
    isOpen = false;
    armedKillPid = null;
    inFlightRequest += 1;
    ++tabIntent;
    pendingRefresh = null;
    snapshot = null;
    systemLoading = false;
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
    const killEpoch = { generation: context.openGeneration, revision: context.tabRevision };
    const canAnnounceKill = () => !disposed && isOpen && activeTab === 'processes'
      && context.openGeneration === killEpoch.generation && context.tabRevision === killEpoch.revision;
    try {
      await killProcess(process.pid, killConfirmationFromPlan(killPlan));
      if (canAnnounceKill()) {
        statusMessage = `Killed ${process.name} (${process.pid})`;
        await refreshProcesses({ preserveVolatileOrder: false, announce: false });
      }
    } catch (error) {
      console.error(`Failed to kill process ${process.pid}`, error);
      if (canAnnounceKill()) {
        statusMessage = `Could not kill ${process.name} (${process.pid}): ${processKillErrorMessage(error)}`;
        await refreshProcesses({ preserveVolatileOrder: false, announce: false });
      }
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

  function tabKeydown(event: KeyboardEvent) {
    let tab: ProcessManagerTab;
    if (event.key === 'Home') tab = 'processes';
    else if (event.key === 'End') tab = 'system_info';
    else if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') tab = activeTab === 'processes' ? 'system_info' : 'processes';
    else return;
    event.preventDefault();
    document.getElementById(`manager-tab-${tab}`)?.focus();
    void selectTab(tab);
  }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    const registerAsyncUnlistener = (registration: Promise<() => void>) => {
      return registration.then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlisteners.push(unlisten);
      });
    };

    void Promise.all([
      registerAsyncUnlistener(listen(PROCESS_MANAGER_OPEN_EVENT, openSurface)),
      registerAsyncUnlistener(listen<{ context: ProcessManagerContext }>(PROCESS_MANAGER_CLOSED_EVENT, (event) => {
        // Native close carries its retired epoch; never retire newer work for a delayed event.
        if (event.payload?.context && !event.payload.context.visible) reconcile(event.payload.context);
      }))
    ]).then(() => { if (!disposed) void recoverContext(); }).catch(() => {
      if (!disposed) statusMessage = 'Process manager events unavailable';
    });

    return () => {
      disposed = true;
      closeSurface();
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  });
</script>

<svelte:document onvisibilitychange={() => { if (document.visibilityState === 'hidden') closeSurface(); }} />
<svelte:window
  onkeydown={(event) => {
    if (event.key === 'Escape' && activeTab === 'processes' && armedKillPid !== null) {
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
    <strong class="process-manager-title">Process Manager</strong>
    {#if activeTab === 'processes'}
    <div class="process-manager-actions">
      <label class="process-filter">
        <span>Filter</span>
        <input
          bind:value={processFilter}
          aria-label="Filter processes by name, PID, parent, path, command line, port, workspace, or status"
          placeholder="name, pid, port, workspace"
          oninput={() => { armedKillPid = null; }}
        />
      </label>
      <MeltActionButton
        class="process-manager-refresh-button"
        ariaLabel="Refresh process manager"
        onClick={() => void refreshProcesses({ preserveVolatileOrder: false, announce: true })}
      ><MaterialSymbolIcon name="refresh" /></MeltActionButton>
    </div>
    {/if}
    <MeltActionButton
      class="process-manager-close-button"
      ariaLabel="Close process manager"
      onClick={() => void requestClose()}
    ><MaterialSymbolIcon name="close" /></MeltActionButton>
    {#if activeTab === 'processes'}
      <span class="process-manager-status" role="status" aria-live="polite">{statusMessage}</span>
    {/if}
  </header>

  <div class="process-manager-tabs" role="tablist" aria-label="Process manager views">
    {#each [{ id: 'processes', label: 'Processes' }, { id: 'system_info', label: 'System Info' }] as tab (tab.id)}
      <button type="button" role="tab" id={`manager-tab-${tab.id}`} aria-controls={`manager-panel-${tab.id}`}
        aria-selected={activeTab === tab.id} tabindex={activeTab === tab.id ? 0 : -1}
        onclick={() => void selectTab(tab.id as ProcessManagerTab)} onkeydown={tabKeydown}>{tab.label}</button>
    {/each}
  </div>
  <div class="process-table" role="tabpanel" id="manager-panel-processes" aria-labelledby="manager-tab-processes" hidden={activeTab !== 'processes'} inert={activeTab !== 'processes'}>
    <p class="process-filtered-note">Filtered process sums · not whole-machine usage</p>
    <div class="process-table-scroll" role="grid" tabindex="0" aria-label="Running processes">
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
            <div class="process-empty surface-state" class:loading={isLoading} class:info={!isLoading}>
              {isLoading ? 'Loading processes…' : (processFilter ? 'No processes match this filter' : statusMessage)}
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
  <div class="system-info-panel" role="tabpanel" id="manager-panel-system_info" aria-labelledby="manager-tab-system_info" hidden={activeTab !== 'system_info'} inert={activeTab !== 'system_info'}>
    {#if activeTab === 'system_info'}
      <SystemInfoView {snapshot} loading={systemLoading} error={systemError} />
    {/if}
  </div>
</main>
