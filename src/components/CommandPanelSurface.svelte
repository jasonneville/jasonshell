<script lang="ts">
  import { onMount, tick } from 'svelte';
  import './CommandPanelSurface.css';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import {
    QUICK_COMMAND_MODES,
    formatQuickCommandArgsTextarea,
    formatQuickCommandCommandsTextarea,
    listQuickCommandHistory,
    loadQuickCommandsSettings,
    nextDuplicateQuickCommandLabel,
    nextUniqueQuickCommandId,
    parseQuickCommandArgsTextarea,
    parseQuickCommandCommandsTextarea,
    runQuickCommand,
    saveQuickCommandsSettings,
    stopQuickCommand,
    type QuickCommandEntry,
    type QuickCommandRunHistoryEntry,
    type QuickCommandMode
  } from '../lib/quickCommands';
  import { hideCommandPanel } from '../lib/commandPanel';

  type CommandEditorModel = {
    id: string | null;
    label: string;
    mode: QuickCommandMode;
    targetPath: string;
    cwd: string;
    argsText: string;
    commandsText: string;
  };

  type CommandPanelTab = 'configuration' | 'previousRuns';

  const modeLabels: Record<QuickCommandMode, string> = {
    direct: 'Program',
    commandBlock: 'Command block'
  };

  let entries: QuickCommandEntry[] = [];
  let loading = true;
  let saving = false;
  let runningId: string | null = null;
  let stoppingId: string | null = null;
  let formErrors: string[] = [];
  let panelError = '';
  let editor: CommandEditorModel = blankEditor();
  let history: QuickCommandRunHistoryEntry[] = [];
  let historyLoading = false;
  let contextEntry: QuickCommandEntry | null = null;
  let contextMenuPosition = { x: 16, y: 112 };
  let activeRunIds = new Set<string>();
  let expandedRunIds = new Set<string>();
  let activeTab: CommandPanelTab = 'configuration';
  let listWidth = 180;
  let panelElement: HTMLDivElement;
  let contextMenuElement: HTMLDivElement;
  let contextMenuFirstAction: HTMLButtonElement;
  let resizePointerId: number | null = null;

  function blankEditor(): CommandEditorModel {
    return {
      id: null,
      label: '',
      mode: 'direct',
      targetPath: '',
      cwd: '',
      argsText: '',
      commandsText: ''
    };
  }

  function inputValue(event: Event): string {
    return (event.currentTarget as HTMLInputElement).value;
  }

  function selectValue(event: Event): string {
    return (event.currentTarget as HTMLSelectElement).value;
  }

  function selectedMode(event: Event): QuickCommandMode {
    return selectValue(event) as QuickCommandMode;
  }

  function textareaValue(event: Event): string {
    return (event.currentTarget as HTMLTextAreaElement).value;
  }

  function startNewEntry() {
    formErrors = [];
    panelError = '';
    activeTab = 'configuration';
    editor = blankEditor();
  }

  function startEditEntry(entry: QuickCommandEntry) {
    formErrors = [];
    panelError = '';
    activeTab = 'configuration';
    editor = {
      id: entry.id,
      label: entry.label,
      mode: entry.mode,
      targetPath: entry.targetPath,
      cwd: entry.cwd ?? '',
      argsText: formatQuickCommandArgsTextarea(entry.args),
      commandsText: formatQuickCommandCommandsTextarea(entry.commands)
    };
  }

  function duplicateEntry(entry: QuickCommandEntry) {
    formErrors = [];
    panelError = '';
    activeTab = 'configuration';
    editor = {
      id: null,
      label: nextDuplicateQuickCommandLabel(entry.label, entries.map((current) => current.label)),
      mode: entry.mode,
      targetPath: entry.targetPath,
      cwd: entry.cwd ?? '',
      argsText: formatQuickCommandArgsTextarea(entry.args),
      commandsText: formatQuickCommandCommandsTextarea(entry.commands)
    };
    contextEntry = null;
  }

  function sortedEntries(values: readonly QuickCommandEntry[]): QuickCommandEntry[] {
    return [...values].sort((left, right) => left.label.localeCompare(right.label));
  }

  function validateEditor(): string[] {
    const errors: string[] = [];
    if (!editor.label.trim()) {
      errors.push('Label is required.');
    }
    if (editor.mode === 'direct' && !editor.targetPath.trim()) {
      errors.push('Program is required.');
    }
    if (editor.mode === 'commandBlock' && parseQuickCommandCommandsTextarea(editor.commandsText).length === 0) {
      errors.push('Add at least one command.');
    }
    const hasInvalidEmptyArg = editor.argsText
      .split(/\r?\n/u)
      .some((line) => line.length > 0 && !line.trim());
    if (hasInvalidEmptyArg) {
      errors.push('Arguments must not include whitespace-only lines.');
    }
    return errors;
  }

  async function refreshEntries() {
    loading = true;
    panelError = '';
    try {
      const quickCommands = await loadQuickCommandsSettings();
      entries = sortedEntries(quickCommands.entries);
      if (editor.id && !entries.some((entry) => entry.id === editor.id)) {
        editor = blankEditor();
      }
    } catch (error) {
      panelError = 'Quick command settings are unavailable.';
      console.error('Failed to load quick commands', error);
    } finally {
      loading = false;
    }
  }

  async function saveEntry() {
    if (saving) return;
    formErrors = validateEditor();
    if (formErrors.length) {
      return;
    }

    const id = editor.id ?? nextUniqueQuickCommandId(editor.label, entries.map((entry) => entry.id));
    if (!id) {
      formErrors = ['Command id could not be derived from Label.'];
      return;
    }

    const nextEntry: QuickCommandEntry = {
      id,
      label: editor.label.trim(),
      mode: editor.mode,
      targetPath: editor.mode === 'direct' ? editor.targetPath.trim() : '',
      cwd: editor.cwd.trim() ? editor.cwd.trim() : null,
      args: editor.mode === 'direct' ? parseQuickCommandArgsTextarea(editor.argsText) : [],
      commands: editor.mode === 'commandBlock' ? parseQuickCommandCommandsTextarea(editor.commandsText) : []
    };

    saving = true;
    panelError = '';
    try {
      const nextEntries = [...entries.filter((entry) => entry.id !== id), nextEntry];
      const saved = await saveQuickCommandsSettings({ entries: nextEntries });
      entries = sortedEntries(saved.entries);
      startEditEntry(nextEntry);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      formErrors = [message];
    } finally {
      saving = false;
    }
  }

  async function deleteEntry(id: string) {
    saving = true;
    formErrors = [];
    panelError = '';
    try {
      const saved = await saveQuickCommandsSettings({
        entries: entries.filter((entry) => entry.id !== id)
      });
      entries = sortedEntries(saved.entries);
      if (editor.id === id) {
        editor = blankEditor();
      }
    } catch (error) {
      panelError = error instanceof Error ? error.message : String(error);
    } finally {
      saving = false;
    }
  }

  async function runEntry(id: string) {
    runningId = id;
    panelError = '';
    formErrors = [];
    try {
      await runQuickCommand({ id });
      activeRunIds = new Set([...activeRunIds, id]);
      activeTab = 'previousRuns';
      await refreshHistory();
    } catch (error) {
      panelError = error instanceof Error ? error.message : String(error);
    } finally {
      runningId = null;
    }
  }

  function commandLabelFor(commandId: string): string {
    return entries.find((entry) => entry.id === commandId)?.label ?? commandId;
  }

  function historyRunKey(run: QuickCommandRunHistoryEntry): string {
    return `${run.commandId}:${run.processId}:${run.startedAtEpochMs}`;
  }

  function historyRunStatus(run: QuickCommandRunHistoryEntry): string {
    if (run.running) return 'Running';
    if (run.exitCode === 0) return 'Completed';
    if (run.exitCode === null) return 'Ended';
    return `Exit ${run.exitCode}`;
  }

  function historyRunSummary(run: QuickCommandRunHistoryEntry): string {
    return `${commandLabelFor(run.commandId)} · ${historyRunStatus(run)}`;
  }

  function formatRunTime(epochMs: number): string {
    return new Date(epochMs).toLocaleString();
  }

  async function refreshHistory(): Promise<void> {
    historyLoading = true;
    try {
      const runs = await listQuickCommandHistory();
      history = runs;
      activeRunIds = new Set(runs.filter((run) => run.running).map((run) => run.commandId));
      const nextExpandedIds = new Set(expandedRunIds);
      for (const run of runs) {
        if (run.running) {
          nextExpandedIds.add(historyRunKey(run));
        }
      }
      expandedRunIds = nextExpandedIds;
    } catch (error) {
      panelError = error instanceof Error ? error.message : String(error);
      history = [];
    } finally {
      historyLoading = false;
    }
  }

  async function stopEntry(id: string) {
    stoppingId = id;
    panelError = '';
    try {
      const activeRun = (await listQuickCommandHistory()).find(
        (run) => run.commandId === id && run.running
      );
      if (!activeRun) {
        await refreshHistory();
        return;
      }
      await stopQuickCommand({ id, processId: activeRun.processId });
      await refreshHistory();
    } catch (error) {
      panelError = error instanceof Error ? error.message : String(error);
    } finally {
      stoppingId = null;
    }
  }

  function isRunExpanded(run: QuickCommandRunHistoryEntry): boolean {
    return expandedRunIds.has(historyRunKey(run));
  }

  function toggleRunOutput(run: QuickCommandRunHistoryEntry) {
    const id = historyRunKey(run);
    const next = new Set(expandedRunIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    expandedRunIds = next;
  }

  function startListResize(event: PointerEvent) {
    event.preventDefault();
    resizePointerId = event.pointerId;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function resizeList(event: PointerEvent) {
    if (resizePointerId !== event.pointerId || !panelElement) {
      return;
    }
    const panelLeft = panelElement.getBoundingClientRect().left;
    listWidth = Math.round(Math.min(Math.max(event.clientX - panelLeft - 16, 128), 420));
  }

  function stopListResize(event: PointerEvent) {
    if (resizePointerId === event.pointerId) {
      resizePointerId = null;
    }
  }

  function closePanel() {
    void hideCommandPanel().catch((error) => {
      console.error('Failed to hide command panel', error);
    });
  }

  function selectCommand(entry: QuickCommandEntry) {
    startEditEntry(entry);
    contextEntry = null;
  }

  function showHistory() {
    activeTab = 'previousRuns';
    contextEntry = null;
    void refreshHistory();
  }

  function editContextEntry() {
    if (contextEntry) {
      selectCommand(contextEntry);
    }
  }

  function duplicateContextEntry() {
    if (contextEntry) {
      duplicateEntry(contextEntry);
    }
  }

  function openContextMenu(event: MouseEvent, entry: QuickCommandEntry) {
    event.preventDefault();
    const panelBounds = panelElement.getBoundingClientRect();
    contextMenuPosition = {
      x: Math.max(8, Math.min(event.clientX - panelBounds.left, panelBounds.width - 172)),
      y: Math.max(8, Math.min(event.clientY - panelBounds.top, panelBounds.height - 92))
    };
    contextEntry = entry;
    void focusContextMenu();
  }

  function openKeyboardContextMenu(event: KeyboardEvent, entry: QuickCommandEntry) {
    if (event.key !== 'ContextMenu' && !(event.key === 'F10' && event.shiftKey)) {
      return;
    }
    event.preventDefault();
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const panelBounds = panelElement.getBoundingClientRect();
    contextMenuPosition = {
      x: Math.max(8, Math.min(bounds.left - panelBounds.left, panelBounds.width - 172)),
      y: Math.max(8, Math.min(bounds.bottom - panelBounds.top, panelBounds.height - 92))
    };
    contextEntry = entry;
    void focusContextMenu();
  }

  async function focusContextMenu() {
    await tick();
    contextMenuFirstAction?.focus();
  }

  function dismissContextMenu(event: MouseEvent) {
    if (contextMenuElement?.contains(event.target as Node)) {
      return;
    }
    contextEntry = null;
  }

  function dismissContextMenuOnEscape(event: KeyboardEvent) {
    if (event.key === 'Escape' && contextEntry) {
      event.preventDefault();
      contextEntry = null;
    }
  }

  function commandRowKeydown(event: KeyboardEvent, entry: QuickCommandEntry) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      selectCommand(entry);
    } else {
      openKeyboardContextMenu(event, entry);
    }
  }

  function shouldPollHistory(): boolean {
    return activeTab === 'previousRuns' || activeRunIds.size > 0;
  }

  onMount(() => {
    void refreshEntries();
    void refreshHistory();
    const interval = window.setInterval(() => {
      if (shouldPollHistory()) {
        void refreshHistory();
      }
    }, 1100);
    return () => window.clearInterval(interval);
  });
</script>

<svelte:window on:click={dismissContextMenu} on:keydown={dismissContextMenuOnEscape} />

<div
  bind:this={panelElement}
  class="command-panel"
  id="command-panel"
  role="dialog"
  tabindex="-1"
  aria-labelledby="command-panel-title"
  style={`--command-list-width: ${listWidth}px`}
  on:pointermove={resizeList}
  on:pointerup={stopListResize}
  on:pointercancel={stopListResize}
>
  <header class="command-panel-header">
    <div>
      <p>JasonShell</p>
      <h1 id="command-panel-title">Quick Commands</h1>
    </div>
    <MeltActionButton ariaLabel="Close quick commands" onClick={closePanel}>x</MeltActionButton>
  </header>

  {#if panelError}
    <p class="command-panel-error" role="alert">{panelError}</p>
  {/if}

  <section class="command-panel-layout">
    <aside class="command-list" aria-label="Saved commands">
      <div class="command-list-header">
        <h2>Saved</h2>
        <MeltActionButton ariaLabel="Create command" onClick={startNewEntry}>New</MeltActionButton>
      </div>
      {#if loading}
        <p class="command-list-state">Loading commands…</p>
      {:else if !entries.length}
        <p class="command-list-state">No quick commands saved.</p>
      {:else}
        <ul>
          {#each entries as entry (entry.id)}
            <li
              class:selected={editor.id === entry.id}
              on:contextmenu={(event) => openContextMenu(event, entry)}
            >
              <div class="command-row">
                <button
                  class="command-select"
                  type="button"
                  aria-label={`Edit ${entry.label}`}
                  on:click={() => selectCommand(entry)}
                  on:keydown={(event) => commandRowKeydown(event, entry)}
                >
                  <strong>{entry.label}</strong>
                </button>
                <button
                  class="command-context-trigger"
                  type="button"
                  aria-label={`More options for ${entry.label}`}
                  aria-haspopup="menu"
                  on:click|stopPropagation={() => selectCommand(entry)}
                  on:keydown={(event) => openKeyboardContextMenu(event, entry)}
                ></button>
                <div class="command-row-actions">
                  {#if runningId === entry.id || activeRunIds.has(entry.id)}
                    <span class="command-spinner" aria-label={`${entry.label} is running`}></span>
                  {/if}
                  <MeltActionButton
                    class={`command-icon-button ${activeRunIds.has(entry.id) ? 'command-stop-button' : 'command-run-button'}`}
                    ariaLabel={activeRunIds.has(entry.id) ? `Stop ${entry.label}` : `Run ${entry.label}`}
                    disabled={Boolean(runningId || saving || stoppingId === entry.id)}
                    onClick={() => void (activeRunIds.has(entry.id) ? stopEntry(entry.id) : runEntry(entry.id))}
                  >
                    {#if activeRunIds.has(entry.id)}
                      <svg viewBox="0 0 16 16" aria-hidden="true"><rect x="3.5" y="3.5" width="9" height="9" /></svg>
                    {:else}
                      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 2.5v11L13 8 4 2.5Z" /></svg>
                    {/if}
                  </MeltActionButton>
                  <MeltActionButton
                    class="command-icon-button command-delete-button"
                    ariaLabel={`Delete ${entry.label}`}
                    disabled={Boolean(runningId || saving || activeRunIds.has(entry.id))}
                    onClick={() => void deleteEntry(entry.id)}
                  >
                    <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 5h6v8H5V5Zm1-2h4l1 1h3v1H2V4h3l1-1Z" /></svg>
                  </MeltActionButton>
                </div>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </aside>
    <button
      class="command-list-resize-grip"
      type="button"
      aria-label="Resize saved commands pane"
      on:pointerdown={startListResize}
    ></button>

    <section class="command-editor" aria-label="Command editor">
      <div class="command-pane-header">
        <div>
          <p>DETAILS</p>
          <h2>{editor.id ? editor.label : 'New command'}</h2>
        </div>
        <div class="command-pane-tabs" role="tablist" aria-label="Quick command panels">
          <MeltActionButton
            class={`command-pane-tab ${activeTab === 'configuration' ? 'active' : ''}`}
            role="tab"
            ariaSelected={activeTab === 'configuration'}
            ariaControls="command-panel-configuration"
            onClick={() => (activeTab = 'configuration')}
          >
            Configuration
          </MeltActionButton>
          <MeltActionButton
            class={`command-pane-tab ${activeTab === 'previousRuns' ? 'active' : ''}`}
            role="tab"
            ariaSelected={activeTab === 'previousRuns'}
            ariaControls="command-panel-previous-runs"
            onClick={() => {
              activeTab = 'previousRuns';
              void refreshHistory();
            }}
          >
            Previous runs
          </MeltActionButton>
        </div>
      </div>

      {#if activeTab === 'configuration'}
        <div id="command-panel-configuration" class="command-pane" role="tabpanel">
          {#if formErrors.length}
            <ul class="command-form-errors" role="alert">
              {#each formErrors as error (error)}
                <li>{error}</li>
              {/each}
            </ul>
          {/if}

          <label>
            <span>Label</span>
            <input
              value={editor.label}
              maxlength="96"
              spellcheck="false"
              on:input={(event) => {
                editor = { ...editor, label: inputValue(event) };
              }}
            />
          </label>

          <label>
            <span>Mode</span>
            <select
              value={editor.mode}
              on:change={(event) => {
                editor = { ...editor, mode: selectedMode(event) };
              }}
            >
              {#each QUICK_COMMAND_MODES as mode}
                <option value={mode}>{modeLabels[mode]}</option>
              {/each}
            </select>
          </label>

          {#if editor.mode === 'direct'}
            <label>
              <span>Program</span>
              <input
                value={editor.targetPath}
                spellcheck="false"
                placeholder="git.exe"
                on:input={(event) => {
                  editor = { ...editor, targetPath: inputValue(event) };
                }}
              />
            </label>
          {/if}

          <label>
            <span>Working directory</span>
            <input
              value={editor.cwd}
              spellcheck="false"
              placeholder="Optional absolute path"
              on:input={(event) => {
                editor = { ...editor, cwd: inputValue(event) };
              }}
            />
          </label>

          {#if editor.mode === 'direct'}
            <label>
              <span>Arguments (one per line)</span>
              <textarea
                rows="5"
                spellcheck="false"
                value={editor.argsText}
                on:input={(event) => {
                  editor = { ...editor, argsText: textareaValue(event) };
                }}
              ></textarea>
            </label>
          {:else}
            <label>
              <span>Commands (one per line)</span>
              <textarea
                rows="8"
                spellcheck="false"
                value={editor.commandsText}
                placeholder={'cd C:\\dev\\my-app\npython app.py'}
                on:input={(event) => {
                  editor = { ...editor, commandsText: textareaValue(event) };
                }}
              ></textarea>
            </label>
          {/if}

          <div class="command-editor-actions">
            <MeltActionButton
              ariaLabel="Save command"
              disabled={saving || Boolean(runningId)}
              onClick={() => void saveEntry()}
            >
              {saving ? 'Saving…' : 'Save'}
            </MeltActionButton>
            <MeltActionButton
              ariaLabel="Cancel command editing"
              disabled={saving || Boolean(runningId)}
              onClick={startNewEntry}
            >
              Clear
            </MeltActionButton>
          </div>
        </div>
      {:else}
        <div id="command-panel-previous-runs" class="command-history-panel" role="tabpanel">
          <p class="command-history-notice">Latest runs only. Output stays local in settings. Up to 20 runs, 16 KiB per stream.</p>
          {#if historyLoading}
            <p class="command-list-state">Loading output…</p>
          {:else if !history.length}
            <p class="command-list-state">No runs yet.</p>
          {:else}
            <div class="command-history-list">
              {#each history as run (historyRunKey(run))}
                <details class="command-history-run" open={run.running || isRunExpanded(run)}>
                  <summary
                    class:running={run.running}
                    aria-label={historyRunSummary(run)}
                    on:click|preventDefault={() => toggleRunOutput(run)}
                    on:keydown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault();
                        toggleRunOutput(run);
                      }
                    }}
                  >
                    <div class="command-history-meta">
                      <strong>{commandLabelFor(run.commandId)}</strong>
                      <span>{historyRunStatus(run)} · {formatRunTime(run.startedAtEpochMs)} · PID {run.processId}</span>
                    </div>
                    {#if run.running}
                      <span class="command-history-live">Live</span>
                    {/if}
                  </summary>
                  <div class="command-history-body">
                    {#if run.stdout}
                      <section>
                        <p>stdout</p>
                        <pre class="command-output">{run.stdout}{run.stdoutTruncated ? '\n[stdout truncated]' : ''}</pre>
                      </section>
                    {/if}
                    {#if run.stderr}
                      <section>
                        <p>stderr</p>
                        <pre class="command-output command-output-error">{run.stderr}{run.stderrTruncated ? '\n[stderr truncated]' : ''}</pre>
                      </section>
                    {/if}
                    {#if run.running && !run.stdout && !run.stderr}
                      <p class="command-list-state">Waiting for output…</p>
                    {/if}
                  </div>
                </details>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </section>
  </section>

  {#if contextEntry}
    <div
      bind:this={contextMenuElement}
      class="command-context-menu"
      role="menu"
      style={`left: ${contextMenuPosition.x}px; top: ${contextMenuPosition.y}px`}
    >
      <button bind:this={contextMenuFirstAction} type="button" role="menuitem" on:click={showHistory}>View output history</button>
      <button type="button" role="menuitem" on:click={editContextEntry}>Edit command</button>
      <button type="button" role="menuitem" on:click={duplicateContextEntry}>Duplicate command</button>
    </div>
  {/if}
</div>
