<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { emitTo, listen } from '@tauri-apps/api/event';
  import './CommandPanelSurface.css';
  import MeltActionButton from './melt/MeltActionButton.svelte';
  import MaterialSymbolIcon from './icons/MaterialSymbolIcon.svelte';
  import {
    QUICK_COMMAND_MODES,
    QUICK_COMMAND_ORDER_LEGACY,
    QUICK_COMMAND_ORDER_VERSION,
    captureQuickCommandOrder,
    deriveQuickCommandPendingInputRequest,
    formatQuickCommandArgsTextarea,
    formatQuickCommandCommandsTextarea,
    listQuickCommandHistory,
    loadQuickCommandsSettings,
    mergeQuickCommandRunHistoryEntries,
    nextDuplicateQuickCommandLabel,
    nextUniqueQuickCommandId,
    moveQuickCommandById,
    openQuickCommandArtifactLocation,
    parseQuickCommandArgsTextarea,
    parseQuickCommandCommandsTextarea,
    quickCommandRunRequest,
    runQuickCommand,
    openQuickCommandUrl,
    saveQuickCommandsSettings,
    sendQuickCommandInput,
    sameQuickCommandOrder,
    sortQuickCommandsForLegacyMigration,
    stopQuickCommand,
    type QuickCommandEntry,
    type QuickCommandMode,
    type QuickCommandPendingInputRequest,
    type QuickCommandRunHistoryEntry,
    type QuickCommandRunUpdatedEvent
  } from '../lib/quickCommands';
  import { IPC_EVENTS } from '../ipc/events.js';
  import { hideCommandPanel, pickQuickCommandArtifactLocation } from '../lib/commandPanel';
  import { topBarWebviewWindowEventTarget } from '../lib/topBarPins';

  type CommandEditorModel = { id: string | null; label: string; mode: QuickCommandMode; targetPath: string; cwd: string; artifactLocation: string; argsText: string; commandsText: string };
  type CommandPanelTab = 'configuration' | 'previousRuns';
  type SettingsSnapshot = { orderVersion: typeof QUICK_COMMAND_ORDER_VERSION; entries: QuickCommandEntry[]; history: QuickCommandRunHistoryEntry[]; listWidth: number };
  type SettingsMutation = {
    revision: number;
    desired: SettingsSnapshot;
    rollback: SettingsSnapshot;
    focusId: string | null;
    resolve: (value: SettingsSnapshot) => void;
    reject: (error: unknown) => void;
  };

  const modeLabels: Record<QuickCommandMode, string> = { direct: 'Program', commandBlock: 'Command block' };
  const SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT = 'search:toggle-centered';
  const TOP_BAR_TARGET = topBarWebviewWindowEventTarget();
  const commandPanelNewIconUrl = new URL('../assets/icons/add_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg', import.meta.url).href;
  const commandPanelDeleteIconUrl = new URL('../assets/icons/delete_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg', import.meta.url).href;
  const commandPanelSaveIconUrl = new URL('../assets/icons/save_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg', import.meta.url).href;
  const commandPanelCancelIconUrl = new URL('../assets/icons/cancel_presentation_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg', import.meta.url).href;
  const COMMAND_REORDER_DRAG_THRESHOLD_PX = 6;
  const COMMAND_REORDER_AUTOSCROLL_EDGE_PX = 36;

  let entries: QuickCommandEntry[] = [];
  let loading = true;
  let saving = false;
  let runningId: string | null = null;
  let stoppingId: string | null = null;
  let formErrors: string[] = [];
  let panelError = '';
  let editor: CommandEditorModel = blankEditor();
  let history: QuickCommandRunHistoryEntry[] = [];
  let allHistory: QuickCommandRunHistoryEntry[] = [];
  let historyLoading = false;
  let historyRequestId = 0;
  let contextEntry: QuickCommandEntry | null = null;
  let deleteConfirmation: QuickCommandEntry | null = null;
  let deleteConfirmationBusy = false;
  let deleteConfirmationDialog: HTMLDivElement | null = null;
  let deleteTriggerElement: HTMLButtonElement | null = null;
  let contextMenuPosition = { x: 16, y: 112 };
  let activeRunIds = new Set<string>();
  let activeCommandIds = new Set<string>();
  let stoppingRunIds = new Set<string>();
  let expandedRunIds = new Set<string>();
  let activeTab: CommandPanelTab = 'configuration';
  let listWidth = 180;
  let panelElement: HTMLDivElement;
  let contextMenuElement: HTMLDivElement;
  let contextMenuFirstAction: HTMLButtonElement;
  let resizePointerId: number | null = null;
  let resizeStartWidth = 180;
  let commandListElement: HTMLUListElement | null = null;
  let pointerDrag: { id: string; pointerId: number; startX: number; startY: number; original: QuickCommandEntry[]; focusId: string | null; active: boolean; pointerY: number; targetIndex: number } | null = null;
  let draggingCommandId: string | null = null;
  let dropTargetCommandId: string | null = null;
  let suppressCommandClickId: string | null = null;
  let dragAutoScrollFrame: number | null = null;
  let orderLiveMessage = '';
  let orderMigrationAttempted = false;
  let mutationRevision = 0;
  let latestAcceptedRevision = 0;
  let latestAcceptedSettings: SettingsSnapshot = { orderVersion: QUICK_COMMAND_ORDER_VERSION, entries: [], history: [], listWidth: 180 };
  let mutationQueue: SettingsMutation[] = [];
  let mutationInFlight = false;
  let pendingInputRequest: QuickCommandPendingInputRequest | null = null;
  let pendingInputDraft = '';
  let pendingInputBusy = false;
  let pendingInputError = '';
  let pendingInputInput: HTMLInputElement | HTMLTextAreaElement | null = null;
  const TRANSCRIPT_SEGMENT_CACHE_LIMIT = 256;
  let historyPollTimer: number | null = null;
  let historyUpdateFrame: number | null = null;
  let historyUpdateQueued = false;
  let lastLiveHistoryUpdateAtByRun = new Map<string, number>();
  let transcriptSegmentCache = new Map<string, TranscriptSegment[]>();
  let transcriptSegmentCacheOrder: string[] = [];
  let shellSurfaceHotkeyHandled = false;
  let disposed = false;
  let structuralMutationPending = false;
  let artifactPickerBusy = false;
  let artifactOpenBusy = false;
  let selectedSavedCommand: QuickCommandEntry | null = null;

  $: structuralMutationPending = mutationInFlight || mutationQueue.length > 0;
  $: selectedSavedCommand = editor.id ? entries.find((entry) => entry.id === editor.id) ?? null : null;

  type TranscriptTokenKind = 'prompt' | 'path' | 'url' | 'level-error' | 'level-warning' | 'level-success' | 'level-info';
  type TranscriptSegment = { text: string; kind: TranscriptTokenKind | null };

  const TRANSCRIPT_PROMPT_PATTERN = /^((?:PS\s+)?(?:(?:[A-Za-z]:[\\/]|~[\\/]|\.\.[\\/]|\.\/[\\/]|[^\s:@]+@[^\s:@]+[:/]|[^\s]+:[\\/])[^^\n]*?[>$#❯])\s+)/u;
  const TRANSCRIPT_TOKEN_PATTERNS = [
    { kind: 'url' as const, pattern: /\bhttps?:\/\/[^\s<>"'`]+/iu },
    { kind: 'path' as const, pattern: /\b(?:[A-Za-z]:[\\/](?:[^\\/\s<>:"|?*]+[\\/])*[^\\/\s<>:"|?*]+|\\\\[^\s<>:"|?*]+(?:\\[^\s<>:"|?*]+)+|~[\\/][^\s<>"'`]+|\.\.[\\/][^\s<>"'`]+|\.\/[^^\s<>"'`]+)/u },
    { kind: 'level-error' as const, pattern: /\b(?:error|fatal|failed|failure|panic|crash|exception|err)\b/iu },
    { kind: 'level-warning' as const, pattern: /\b(?:warn|warning|caution|deprecated|todo)\b/iu },
    { kind: 'level-success' as const, pattern: /\b(?:success|succeeded|ok|done|complete|completed|passed|pass|ready)\b/iu },
    { kind: 'level-info' as const, pattern: /\b(?:info|debug|trace)\b/iu }
  ];

  function blankEditor(): CommandEditorModel { return { id: null, label: '', mode: 'direct', targetPath: '', cwd: '', artifactLocation: '', argsText: '', commandsText: '' }; }
  function inputValue(event: Event): string { return (event.currentTarget as HTMLInputElement).value; }
  function selectValue(event: Event): string { return (event.currentTarget as HTMLSelectElement).value; }
  function selectedMode(event: Event): QuickCommandMode { return selectValue(event) as QuickCommandMode; }
  function textareaValue(event: Event): string { return (event.currentTarget as HTMLTextAreaElement).value; }

  function cloneEntries(values: readonly QuickCommandEntry[]): QuickCommandEntry[] { return values.map((entry) => ({ ...entry, args: [...entry.args], commands: [...entry.commands] })); }
  function cloneSettings(values: { entries: readonly QuickCommandEntry[]; history: readonly QuickCommandRunHistoryEntry[]; listWidth: number }): SettingsSnapshot {
    return { orderVersion: QUICK_COMMAND_ORDER_VERSION, entries: cloneEntries(values.entries), history: [...values.history], listWidth: values.listWidth };
  }
  function currentSettingsSnapshot(): SettingsSnapshot { return { orderVersion: QUICK_COMMAND_ORDER_VERSION, entries: cloneEntries(entries), history: [...allHistory], listWidth }; }
  function structuralMutationBusy(): boolean { return structuralMutationPending; }
  function focusCommandEntry(id: string | null) { if (!id) return; void tick().then(() => { if (disposed) return; commandListElement?.querySelector<HTMLButtonElement>(`[data-command-id="${CSS.escape(id)}"] .command-select`)?.focus(); }); }

  function startNewEntry() { cancelReorderForAction(); if (structuralMutationBusy()) return; formErrors = []; panelError = ''; activeTab = 'configuration'; editor = blankEditor(); }
  function startEditEntry(entry: QuickCommandEntry) { cancelReorderForAction(); formErrors = []; panelError = ''; activeTab = 'configuration'; editor = { id: entry.id, label: entry.label, mode: entry.mode, targetPath: entry.targetPath, cwd: entry.cwd ?? '', artifactLocation: entry.artifactLocation ?? '', argsText: formatQuickCommandArgsTextarea(entry.args), commandsText: formatQuickCommandCommandsTextarea(entry.commands) }; }
  function duplicateEntry(entry: QuickCommandEntry) { cancelReorderForAction(); if (structuralMutationBusy()) return; formErrors = []; panelError = ''; activeTab = 'configuration'; editor = { id: null, label: nextDuplicateQuickCommandLabel(entry.label, entries.map((current) => current.label)), mode: entry.mode, targetPath: entry.targetPath, cwd: entry.cwd ?? '', artifactLocation: entry.artifactLocation ?? '', argsText: formatQuickCommandArgsTextarea(entry.args), commandsText: formatQuickCommandCommandsTextarea(entry.commands) }; contextEntry = null; }

  function applyAcceptedSettings(values: SettingsSnapshot) {
    entries = cloneEntries(values.entries);
    listWidth = values.listWidth;
    history = editor.id ? allHistory.filter((run) => run.commandId === editor.id) : [];
  }

  function restoreMutationFocus(id: string | null) { focusCommandEntry(id); }

  async function drainMutationQueue() {
    if (mutationInFlight || disposed) return;
    const mutation = mutationQueue.shift();
    if (!mutation) { saving = false; return; }
    mutationInFlight = true;
    try {
      const saved = await saveQuickCommandsSettings(mutation.desired);
      if (disposed) {
        mutation.resolve(cloneSettings(saved));
        return;
      }
      const accepted = cloneSettings(saved);
      latestAcceptedSettings = accepted;
      latestAcceptedRevision = mutation.revision;
      if (mutation.revision === mutationRevision) applyAcceptedSettings(accepted);
      mutation.resolve(accepted);
    } catch (error) {
      if (mutationQueue.length === 0) {
        const rollback = latestAcceptedRevision < mutation.revision ? mutation.rollback : latestAcceptedSettings;
        entries = cloneEntries(rollback.entries);
        listWidth = rollback.listWidth;
        history = editor.id ? allHistory.filter((run) => run.commandId === editor.id) : [];
        restoreMutationFocus(mutation.focusId);
      }
      mutation.reject(error);
    } finally {
      mutationInFlight = false;
      saving = mutationQueue.length > 0;
      if (mutationQueue.length > 0) void drainMutationQueue();
    }
  }

  function enqueueSettingsMutation(desired: SettingsSnapshot, rollback: SettingsSnapshot, focusId: string | null): Promise<SettingsSnapshot> {
    const revision = ++mutationRevision;
    saving = true;
    return new Promise<SettingsSnapshot>((resolve, reject) => {
      mutationQueue.push({ revision, desired, rollback, focusId, resolve, reject });
      void drainMutationQueue();
    });
  }

  function validateEditor(): string[] {
    const errors: string[] = [];
    if (!editor.label.trim()) errors.push('Label is required.');
    if (editor.mode === 'direct' && !editor.targetPath.trim()) errors.push('Program is required.');
    if (editor.mode === 'commandBlock' && parseQuickCommandCommandsTextarea(editor.commandsText).length === 0) errors.push('Add at least one command.');
    if (editor.argsText.split(/\r?\n/u).some((line) => line.length > 0 && !line.trim())) errors.push('Arguments must not include whitespace-only lines.');
    return errors;
  }

  async function pickArtifactLocation() {
    if (disposed || artifactPickerBusy) return;
    artifactPickerBusy = true;
    formErrors = [];
    try {
      const selected = await pickQuickCommandArtifactLocation();
      if (!disposed && selected !== null) editor = { ...editor, artifactLocation: selected };
    } catch (error) {
      if (!disposed) formErrors = [error instanceof Error ? error.message : String(error)];
    } finally {
      if (!disposed) artifactPickerBusy = false;
    }
  }

  async function openSelectedArtifactLocation() {
    if (!editor.id || !selectedSavedCommand?.artifactLocation || artifactOpenBusy || structuralMutationBusy()) return;
    artifactOpenBusy = true;
    panelError = '';
    try {
      await openQuickCommandArtifactLocation(editor.id);
    } catch (error) {
      if (!disposed) panelError = error instanceof Error ? error.message : String(error);
    } finally {
      if (!disposed) artifactOpenBusy = false;
    }
  }

  async function refreshEntries() {
    if (disposed) return;
    loading = true;
    panelError = '';
    try {
      const quickCommands = await loadQuickCommandsSettings();
      if (disposed) return;
      const loaded = cloneSettings(quickCommands);
      latestAcceptedSettings = loaded;
      entries = cloneEntries(quickCommands.entries);
      allHistory = [...quickCommands.history];
      listWidth = quickCommands.listWidth;
      history = editor.id ? allHistory.filter((run) => run.commandId === editor.id) : [];
      updatePendingInputFromHistory();
      if (editor.id && !entries.some((entry) => entry.id === editor.id)) editor = blankEditor();
      if (quickCommands.orderVersion === QUICK_COMMAND_ORDER_LEGACY && !orderMigrationAttempted) {
        orderMigrationAttempted = true;
        const migratedEntries = sortQuickCommandsForLegacyMigration(entries);
        entries = migratedEntries;
        const rollback = loaded;
        void enqueueSettingsMutation(currentSettingsSnapshot(), rollback, editor.id).catch((error) => {
          if (!disposed) panelError = `Quick command order migration could not be saved: ${error instanceof Error ? error.message : String(error)}`;
        });
      }
    } catch (error) {
      if (disposed) return;
      panelError = 'Quick command settings are unavailable.';
      console.error('Failed to load quick commands', error);
    } finally {
      if (disposed) return;
      loading = false;
    }
  }

  async function saveEntry() {
    if (disposed) return;
    cancelReorderForAction();
    if (saving) return;
    formErrors = validateEditor();
    if (formErrors.length) return;
    const id = editor.id ?? nextUniqueQuickCommandId(editor.label, entries.map((entry) => entry.id));
    if (!id) { formErrors = ['Command id could not be derived from Label.']; return; }
    const editorBeforeSave = { ...editor };
    const nextEntry: QuickCommandEntry = { id, label: editor.label.trim(), mode: editor.mode, targetPath: editor.mode === 'direct' ? editor.targetPath.trim() : '', cwd: editor.cwd.trim() ? editor.cwd.trim() : null, artifactLocation: editor.artifactLocation.trim() ? editor.artifactLocation.trim() : null, args: editor.mode === 'direct' ? parseQuickCommandArgsTextarea(editor.argsText) : [], commands: editor.mode === 'commandBlock' ? parseQuickCommandCommandsTextarea(editor.commandsText) : [] };
    panelError = '';
    const rollback = currentSettingsSnapshot();
    const existingIndex = editor.id ? entries.findIndex((entry) => entry.id === editor.id) : -1;
    try {
      const nextEntries = existingIndex < 0
        ? [...entries, nextEntry]
        : entries.map((entry, index) => index === existingIndex ? nextEntry : entry);
      entries = nextEntries;
      const saved = await enqueueSettingsMutation(currentSettingsSnapshot(), rollback, editor.id);
      if (disposed) return;
      if (saved.entries.some((entry) => entry.id === id) && JSON.stringify(editor) === JSON.stringify(editorBeforeSave)) startEditEntry(nextEntry);
    } catch (error) {
      if (disposed) return;
      formErrors = [error instanceof Error ? error.message : String(error)];
    }
  }

  async function deleteEntry(id: string, event?: MouseEvent) {
    if (disposed) return;
    cancelReorderForAction();
    if (structuralMutationBusy()) return;
    const entry = entries.find((current) => current.id === id);
    if (!entry || deleteConfirmationBusy) return;
    const trigger = event?.currentTarget;
    deleteTriggerElement = trigger instanceof HTMLButtonElement ? trigger : null;
    deleteConfirmation = entry;
    contextEntry = null;
    await focusDeleteConfirmation();
  }

  function getDeleteConfirmationButtons(): HTMLButtonElement[] { return Array.from(deleteConfirmationDialog?.querySelectorAll('button') ?? []) as HTMLButtonElement[]; }
  async function focusDeleteConfirmation() { if (disposed) return; await tick(); if (disposed || !deleteConfirmation || deleteConfirmationBusy) return; getDeleteConfirmationButtons()[0]?.focus(); }
  async function restoreDeleteTriggerFocus() { if (disposed) return; const trigger = deleteTriggerElement; deleteTriggerElement = null; await tick(); if (disposed) return; if (trigger?.isConnected) trigger.focus(); else panelElement?.focus(); }
  function focusDeleteConfirmationButton(direction: 1 | -1) { const buttons = getDeleteConfirmationButtons().filter((button) => !button.disabled); if (!buttons.length) { deleteConfirmationDialog?.focus(); return; } const activeButton = document.activeElement instanceof HTMLButtonElement ? document.activeElement : null; const currentIndex = activeButton ? buttons.indexOf(activeButton) : -1; const nextIndex = currentIndex < 0 ? (direction > 0 ? 0 : buttons.length - 1) : (currentIndex + direction + buttons.length) % buttons.length; buttons[nextIndex]?.focus(); }

  async function confirmDeleteEntry() {
    if (disposed) return;
    cancelReorderForAction();
    if (!deleteConfirmation || deleteConfirmationBusy || structuralMutationBusy()) return;
    const id = deleteConfirmation.id;
    deleteConfirmationBusy = true; formErrors = []; panelError = '';
    try {
      const latestRuns = await listQuickCommandHistory({ id });
      if (disposed) return;
      if (latestRuns.some((run) => run.running)) { panelError = 'Cannot delete quick command while it is running.'; return; }
      const rollback = currentSettingsSnapshot();
      entries = entries.filter((entry) => entry.id !== id);
      await enqueueSettingsMutation(currentSettingsSnapshot(), rollback, id);
      if (disposed) return;
      history = editor.id ? allHistory.filter((run) => run.commandId === editor.id) : [];
      if (editor.id === id) editor = blankEditor();
      deleteConfirmation = null;
      await restoreDeleteTriggerFocus();
    } catch (error) {
      if (disposed) return;
      panelError = error instanceof Error ? error.message : String(error);
      deleteConfirmation = null;
      await restoreDeleteTriggerFocus();
    } finally { if (!disposed) deleteConfirmationBusy = false; }
  }

  function cancelDeleteEntry() { if (deleteConfirmationBusy) return; deleteConfirmation = null; void restoreDeleteTriggerFocus(); }
  function handleDeleteConfirmationKeydown(event: KeyboardEvent) { if (event.key === 'Escape') { event.preventDefault(); cancelDeleteEntry(); return; } if (event.key !== 'Tab') return; event.preventDefault(); if (deleteConfirmationBusy) { deleteConfirmationDialog?.focus(); return; } focusDeleteConfirmationButton(event.shiftKey ? -1 : 1); }

  function announceCommandOrder(message: string) { orderLiveMessage = message; }

  function commandPosition(id: string): number {
    const index = entries.findIndex((entry) => entry.id === id);
    return index < 0 ? 0 : index + 1;
  }

  function destinationIndexForPointer(clientY: number): number {
    const rows = Array.from(commandListElement?.querySelectorAll<HTMLElement>('[data-command-id]') ?? [])
      .filter((row) => row.dataset.commandId !== pointerDrag?.id);
    return rows.reduce((index, row) => {
      const rect = row.getBoundingClientRect();
      return index + (clientY > rect.top + rect.height / 2 ? 1 : 0);
    }, 0);
  }

  function autoScrollCommandList(clientY: number) {
    const list = commandListElement;
    if (!list) return;
    const bounds = list.getBoundingClientRect();
    const edge = COMMAND_REORDER_AUTOSCROLL_EDGE_PX;
    const step = clientY < bounds.top + edge ? -Math.max(2, Math.round((bounds.top + edge - clientY) / 5)) : clientY > bounds.bottom - edge ? Math.max(2, Math.round((clientY - (bounds.bottom - edge)) / 5)) : 0;
    if (step) list.scrollTop += step;
  }

  function scheduleCommandAutoScroll() {
    if (dragAutoScrollFrame !== null) return;
    const tickAutoScroll = () => {
      dragAutoScrollFrame = null;
      if (!pointerDrag?.active) return;
      autoScrollCommandList(pointerDrag.pointerY);
      applyPointerDragPosition(pointerDrag.pointerY);
      dragAutoScrollFrame = window.requestAnimationFrame(tickAutoScroll);
    };
    dragAutoScrollFrame = window.requestAnimationFrame(tickAutoScroll);
  }

  function stopCommandAutoScroll() {
    if (dragAutoScrollFrame !== null) window.cancelAnimationFrame(dragAutoScrollFrame);
    dragAutoScrollFrame = null;
  }

  function applyPointerDragPosition(clientY: number) {
    if (!pointerDrag?.active) return;
    const targetIndex = destinationIndexForPointer(clientY);
    pointerDrag.targetIndex = targetIndex;
    const candidateRows = Array.from(commandListElement?.querySelectorAll<HTMLElement>('[data-command-id]') ?? [])
      .filter((row) => row.dataset.commandId !== pointerDrag?.id);
    const targetId = candidateRows[Math.min(targetIndex, Math.max(candidateRows.length - 1, 0))]?.dataset.commandId ?? null;
    const moved = moveQuickCommandById(pointerDrag.original, pointerDrag.id, targetIndex);
    const position = moved.findIndex((entry) => entry.id === pointerDrag?.id) + 1;
    announceCommandOrder(`Moving ${moved.find((entry) => entry.id === pointerDrag?.id)?.label ?? 'command'} to position ${position} of ${moved.length}.`);
    dropTargetCommandId = targetId === pointerDrag.id ? null : targetId;
  }

  function startCommandPointerDrag(event: PointerEvent, entry: QuickCommandEntry) {
    const pointerTarget = event.target instanceof Element ? event.target : null;
    if (event.button !== 0 || structuralMutationBusy() || pointerTarget?.closest('.command-row-actions')) return;
    const target = event.currentTarget as HTMLElement;
    pointerDrag = { id: entry.id, pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, original: cloneEntries(entries), focusId: entry.id, active: false, pointerY: event.clientY, targetIndex: entries.findIndex((current) => current.id === entry.id) };
    target.setPointerCapture(event.pointerId);
  }

  function updateCommandPointerDrag(event: PointerEvent, entry: QuickCommandEntry) {
    if (!pointerDrag || pointerDrag.id !== entry.id || pointerDrag.pointerId !== event.pointerId) return;
    pointerDrag.pointerY = event.clientY;
    if (!pointerDrag.active) {
      const distance = Math.hypot(event.clientX - pointerDrag.startX, event.clientY - pointerDrag.startY);
      if (distance < COMMAND_REORDER_DRAG_THRESHOLD_PX) return;
      pointerDrag.active = true;
      draggingCommandId = entry.id;
      event.preventDefault();
      announceCommandOrder(`Grabbed ${entry.label}. Drag to reorder, then release.`);
      scheduleCommandAutoScroll();
    }
    event.preventDefault();
    autoScrollCommandList(event.clientY);
    applyPointerDragPosition(event.clientY);
  }

  function finishCommandPointerDrag(event: PointerEvent, cancelled = false) {
    if (!pointerDrag || pointerDrag.pointerId !== event.pointerId) return;
    const drag = pointerDrag;
    const target = event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
    stopCommandAutoScroll();
    if (drag.active) {
      event.preventDefault();
      suppressNextCommandClick(drag.id);
      if (cancelled) {
        restoreReorderEntries(drag.original);
        announceCommandOrder(`Cancelled reorder for ${entries.find((entry) => entry.id === drag.id)?.label ?? 'command'}.`);
      } else {
        const moved = moveQuickCommandById(drag.original, drag.id, drag.targetIndex);
        if (sameQuickCommandOrder(moved, captureQuickCommandOrder(drag.original))) {
          announceCommandOrder('Command order unchanged.');
        } else {
          entries = moved;
          commitCommandReorder(drag.original, drag.focusId);
        }
      }
      restoreMutationFocus(drag.focusId ?? drag.id);
    }
    pointerDrag = null;
    draggingCommandId = null;
    dropTargetCommandId = null;
    if (target?.hasPointerCapture(event.pointerId)) target.releasePointerCapture(event.pointerId);
  }

  function cancelCommandPointerDrag(event: PointerEvent) { finishCommandPointerDrag(event, true); }

  function commitCommandReorder(original: QuickCommandEntry[], focusId: string | null) {
    const rollback = { ...currentSettingsSnapshot(), entries: cloneEntries(original) };
    const desired = currentSettingsSnapshot();
    void enqueueSettingsMutation(desired, rollback, focusId).then(() => {
      if (!disposed) announceCommandOrder('Command order saved.');
    }).catch((error) => {
      if (!disposed) {
        panelError = `Quick command order could not be saved: ${error instanceof Error ? error.message : String(error)}`;
        announceCommandOrder('Command order save failed. Previous order restored.');
      }
    });
  }

  function handleCommandReorderKeydown(event: KeyboardEvent, entry: QuickCommandEntry): boolean {
    if (!event.altKey || structuralMutationBusy() || pointerDrag) return false;
    const currentIndex = entries.findIndex((current) => current.id === entry.id);
    const targetIndex = event.key === 'ArrowUp' ? currentIndex - 1
      : event.key === 'ArrowDown' ? currentIndex + 1
      : event.key === 'Home' ? 0
      : event.key === 'End' ? entries.length - 1
      : currentIndex;
    if (!['ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) return false;
    event.preventDefault();
    const original = cloneEntries(entries);
    const moved = moveQuickCommandById(entries, entry.id, Math.min(Math.max(targetIndex, 0), entries.length - 1));
    if (sameQuickCommandOrder(moved, captureQuickCommandOrder(entries))) {
      announceCommandOrder(`${entry.label} is already at position ${commandPosition(entry.id)} of ${entries.length}.`);
      restoreMutationFocus(entry.id);
      return true;
    }
    entries = moved;
    announceCommandOrder(`${entry.label} moved to position ${commandPosition(entry.id)} of ${entries.length}. Saving.`);
    commitCommandReorder(original, entry.id);
    restoreMutationFocus(entry.id);
    return true;
  }

  function suppressCommandRowClick(event: MouseEvent, entry: QuickCommandEntry) {
    if (event.target instanceof Element && event.target.closest('.command-row-actions')) return;
    if (suppressCommandClickId !== entry.id) return;
    event.preventDefault();
    event.stopPropagation();
    suppressCommandClickId = null;
  }

  function suppressNextCommandClick(id: string) {
    suppressCommandClickId = id;
    window.setTimeout(() => { if (suppressCommandClickId === id) suppressCommandClickId = null; }, 500);
  }

  function restoreReorderEntries(original: readonly QuickCommandEntry[]) {
    const liveEntriesById = new Map(entries.map((entry) => [entry.id, entry]));
    const originalIds = new Set(original.map((entry) => entry.id));
    const restored = original
      .map((entry) => liveEntriesById.get(entry.id))
      .filter((entry): entry is QuickCommandEntry => entry !== undefined);
    entries = [...restored, ...entries.filter((entry) => !originalIds.has(entry.id))];
  }

  async function runEntry(entry: QuickCommandEntry) {
    if (disposed) return;
    runningId = entry.id; panelError = ''; formErrors = [];
    try {
      const { runId } = await runQuickCommand(quickCommandRunRequest(entry.id));
      if (disposed) return;
      activeCommandIds = new Set([...activeCommandIds, entry.id]);
      activeRunIds = new Set([...activeRunIds, runId]);
      expandedRunIds = new Set([...expandedRunIds, runId]);
      selectCommand(entry);
      activeTab = 'previousRuns';
      await refreshHistory();
    } catch (error) { if (disposed) return; panelError = error instanceof Error ? error.message : String(error); } finally { if (!disposed) runningId = null; }
  }

  function commandLabelFor(commandId: string): string { return entries.find((entry) => entry.id === commandId)?.label ?? commandId; }
  function historyRunKey(run: QuickCommandRunHistoryEntry): string { return run.runId || `${run.commandId}:${run.processId}:${run.startedAtEpochMs}`; }
  function historyRunStatus(run: QuickCommandRunHistoryEntry): string { if (run.running) return 'Running'; if (run.exitCode === 0) return 'Completed'; if (run.exitCode === null) return 'Ended'; return `Exit ${run.exitCode}`; }
  function historyRunSummary(run: QuickCommandRunHistoryEntry): string { return `${commandLabelFor(run.commandId)} · ${historyRunStatus(run)}`; }
  function formatRunTime(epochMs: number): string { return new Date(epochMs).toLocaleString(); }

  async function refreshHistory(): Promise<void> {
    if (disposed) return;
    const requestId = ++historyRequestId;
    const selectedId = editor.id;
    historyLoading = true;
    try {
      const allRuns = await listQuickCommandHistory();
      if (disposed) return;
      if (requestId !== historyRequestId || editor.id !== selectedId) return;
      allHistory = mergeQuickCommandRunHistoryEntries(allHistory, allRuns);
      history = selectedId ? allHistory.filter((run) => run.commandId === selectedId) : [];
      activeRunIds = new Set(allHistory.filter((run) => run.running).map((run) => run.runId));
      activeCommandIds = new Set(allHistory.filter((run) => run.running).map((run) => run.commandId));
      const nextExpandedIds = new Set(expandedRunIds);
      for (const run of allHistory) if (run.commandId === selectedId && run.running) nextExpandedIds.add(historyRunKey(run));
      expandedRunIds = nextExpandedIds;
      updatePendingInputFromHistory();
    } catch (error) { if (disposed) return; if (requestId !== historyRequestId || editor.id !== selectedId) return; panelError = error instanceof Error ? error.message : String(error); } finally { if (!disposed && requestId === historyRequestId && editor.id === selectedId) historyLoading = false; }
  }

  async function stopEntry(id: string) {
    if (disposed) return;
    stoppingId = id; panelError = '';
    try {
      const activeRun = (await listQuickCommandHistory()).find((run) => run.commandId === id && run.running);
      if (disposed) return;
      if (!activeRun) { await refreshHistory(); return; }
      await stopQuickCommand({ id, processId: activeRun.processId, runId: activeRun.runId });
      if (disposed) return;
      await refreshHistory();
    } catch (error) { if (disposed) return; panelError = error instanceof Error ? error.message : String(error); } finally { if (!disposed) stoppingId = null; }
  }

  function latestRunControlKind(run: QuickCommandRunHistoryEntry): string | null { return [...run.transcript].reverse().find((line) => line.kind === 'stopping' || line.kind === 'stop-failed' || line.kind === 'stopped' || line.kind === 'exit')?.kind ?? null; }
  function isRunStopping(run: QuickCommandRunHistoryEntry): boolean { return run.running && (latestRunControlKind(run) === 'stopping' || stoppingRunIds.has(run.runId)); }
  function isCommandStopping(commandId: string): boolean { return allHistory.some((run) => run.commandId === commandId && isRunStopping(run)); }
  function isRunExpanded(run: QuickCommandRunHistoryEntry): boolean { return expandedRunIds.has(historyRunKey(run)); }
  function handleHistoryRunToggle(event: Event, run: QuickCommandRunHistoryEntry) { const details = event.currentTarget as HTMLDetailsElement | null; if (!details) return; if (run.running) { details.open = true; return; } const id = historyRunKey(run); const next = new Set(expandedRunIds); if (details.open) next.add(id); else next.delete(id); expandedRunIds = next; }
  function startListResize(event: PointerEvent) { if (structuralMutationBusy() || pointerDrag) return; event.preventDefault(); resizeStartWidth = listWidth; resizePointerId = event.pointerId; (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId); }
  function resizeList(event: PointerEvent) { if (resizePointerId !== event.pointerId || !panelElement) return; const panelLeft = panelElement.getBoundingClientRect().left; listWidth = Math.round(Math.min(Math.max(event.clientX - panelLeft - 16, 128), 420)); }
  function stopListResize(event: PointerEvent) {
    if (resizePointerId !== event.pointerId) return;
    resizePointerId = null;
    const rollback = { ...currentSettingsSnapshot(), listWidth: resizeStartWidth };
    if (listWidth === resizeStartWidth) return;
    void enqueueSettingsMutation(currentSettingsSnapshot(), rollback, editor.id).catch((error) => {
      if (!disposed) panelError = `Quick command pane width could not be saved: ${error instanceof Error ? error.message : String(error)}`;
    });
  }
  function cancelListResize(event: PointerEvent) { if (resizePointerId === event.pointerId) { listWidth = resizeStartWidth; resizePointerId = null; } }
  function closePanel() { void hideCommandPanel().catch((error) => console.error('Failed to hide command panel', error)); }
  function selectCommand(entry: QuickCommandEntry) { startEditEntry(entry); contextEntry = null; activeTab = 'configuration'; }
  function showHistory() { cancelReorderForAction(); if (contextEntry) selectCommand(contextEntry); activeTab = 'previousRuns'; contextEntry = null; void refreshHistory(); }
  function editContextEntry() { if (contextEntry) selectCommand(contextEntry); }
  function duplicateContextEntry() { if (contextEntry) duplicateEntry(contextEntry); }
  function openContextMenu(event: MouseEvent, entry: QuickCommandEntry) { event.preventDefault(); cancelReorderForAction(); const panelBounds = panelElement.getBoundingClientRect(); contextMenuPosition = { x: Math.max(8, Math.min(event.clientX - panelBounds.left, panelBounds.width - 172)), y: Math.max(8, Math.min(event.clientY - panelBounds.top, panelBounds.height - 92)) }; contextEntry = entry; void focusContextMenu(); }
  function openKeyboardContextMenu(event: KeyboardEvent, entry: QuickCommandEntry) { if (event.key !== 'ContextMenu' && !(event.key === 'F10' && event.shiftKey)) return; event.preventDefault(); cancelReorderForAction(); const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect(); const panelBounds = panelElement.getBoundingClientRect(); contextMenuPosition = { x: Math.max(8, Math.min(bounds.left - panelBounds.left, panelBounds.width - 172)), y: Math.max(8, Math.min(bounds.bottom - panelBounds.top, panelBounds.height - 92)) }; contextEntry = entry; void focusContextMenu(); }
  async function focusContextMenu() { await tick(); contextMenuFirstAction?.focus(); }
  function dismissContextMenu(event: MouseEvent) { if (contextMenuElement?.contains(event.target as Node)) return; contextEntry = null; }
  function cancelActivePointerReorder(restoreFocus = true) {
    if (!pointerDrag) return false;
    const drag = pointerDrag;
    stopCommandAutoScroll();
    const row = commandListElement?.querySelector<HTMLElement>(`[data-command-id="${CSS.escape(drag.id)}"]`);
    if (drag.active) {
      restoreReorderEntries(drag.original);
      suppressNextCommandClick(drag.id);
      announceCommandOrder('Reorder cancelled. Previous order restored.');
      if (restoreFocus) restoreMutationFocus(drag.focusId ?? drag.id);
    }
    pointerDrag = null;
    draggingCommandId = null;
    dropTargetCommandId = null;
    if (row?.hasPointerCapture(drag.pointerId)) row.releasePointerCapture(drag.pointerId);
    return true;
  }
  function cancelReorderForAction() {
    const pointerCancelled = cancelActivePointerReorder(false);
    return pointerCancelled;
  }
  function dismissContextMenuOnEscape(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    if (cancelActivePointerReorder()) { event.preventDefault(); return; }
    if (contextEntry) { event.preventDefault(); contextEntry = null; }
  }
  function getSelectionWithinShell(shell: HTMLElement): Selection | null { const selection = window.getSelection(); if (!selection || selection.isCollapsed || selection.rangeCount === 0) return null; const range = selection.getRangeAt(0); if (!shell.contains(range.commonAncestorContainer)) return null; const anchor = selection.anchorNode; const focus = selection.focusNode; if (!anchor || !focus) return null; const anchorShell = (anchor instanceof Element ? anchor : anchor.parentElement)?.closest('.command-transcript-shell'); const focusShell = (focus instanceof Element ? focus : focus.parentElement)?.closest('.command-transcript-shell'); return anchorShell === shell && focusShell === shell ? selection : null; }
  function handleTranscriptKeydown(event: KeyboardEvent) { if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'c') return; const shell = event.currentTarget as HTMLElement; if (!getSelectionWithinShell(shell)) return; try { if (document.execCommand('copy')) { event.preventDefault(); event.stopPropagation(); } } catch { /* native default */ } }
  function handleTranscriptContextMenu(event: MouseEvent) { const shell = event.currentTarget as HTMLElement; if (getSelectionWithinShell(shell)) event.stopPropagation(); }
  function handleTranscriptUrlAuxClick(event: MouseEvent, url: string) { event.preventDefault(); event.stopPropagation(); void openTranscriptUrl(url); }
  function commandRowKeydown(event: KeyboardEvent, entry: QuickCommandEntry) { if (handleCommandReorderKeydown(event, entry)) return; if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); selectCommand(entry); } else { openKeyboardContextMenu(event, entry); } }
  function shouldPollHistory(): boolean {
    if (pendingInputRequest !== null) return true;
    if (activeRunIds.size === 0) return false;
    const now = Date.now();
    // Contract: activeCommandIds holds command IDs; freshness map keys must match the same ID domain.
    for (const runId of activeRunIds) {
      const lastLiveAt = lastLiveHistoryUpdateAtByRun.get(runId);
      if (lastLiveAt === undefined || now - lastLiveAt >= 1100) return true;
    }
    return false;
  }
  function pruneLiveHistoryUpdateTimes() {
    if (!lastLiveHistoryUpdateAtByRun.size) return;
    const activeIds = new Set(allHistory.filter((run) => run.running).map((run) => run.runId));
    for (const runId of lastLiveHistoryUpdateAtByRun.keys()) if (!activeIds.has(runId)) lastLiveHistoryUpdateAtByRun.delete(runId);
  }
  function pruneTranscriptSegmentCache() {
    while (transcriptSegmentCacheOrder.length > TRANSCRIPT_SEGMENT_CACHE_LIMIT) {
      const removed = transcriptSegmentCacheOrder.shift();
      if (removed) transcriptSegmentCache.delete(removed);
    }
  }
  function normalizeDraftLength(value: string, maxLength: number): string { return Array.from(value).slice(0, maxLength).join(''); }
  function currentPendingInputMaxLength(): number { return pendingInputRequest?.maxLength ?? 4096; }
  function isCtrlSpaceHotkey(event: KeyboardEvent) { return event.code === 'Space' && event.ctrlKey && !event.altKey && !event.metaKey; }
  function handleCtrlSpaceHotkey(event: KeyboardEvent) {
    if (!isCtrlSpaceHotkey(event)) return;
    event.preventDefault();
    event.stopPropagation();
    if (!shellSurfaceHotkeyHandled && !event.repeat) {
      shellSurfaceHotkeyHandled = true;
      void emitTo(TOP_BAR_TARGET, SEARCH_HOTKEY_TOGGLE_SEARCH_EVENT);
    }
  }

  function transcriptLineClass(kind: string): string {
    if (kind === 'stderr') return 'command-transcript-line--stderr';
    if (kind === 'system') return 'command-transcript-line--system';
    if (kind === 'input-request') return 'command-transcript-line--prompt';
    if (kind === 'input-submitted') return 'command-transcript-line--command';
    if (kind === 'confirm') return 'command-transcript-line--success';
    return '';
  }

  async function openTranscriptUrl(url: string) {
    try {
      await openQuickCommandUrl(url);
    } catch (error) {
      panelError = error instanceof Error ? error.message : String(error);
    }
  }

  function handleTranscriptUrlClick(event: MouseEvent, url: string) {
    event.preventDefault();
    void openTranscriptUrl(url);
  }

  function handleTranscriptUrlContextMenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
  }

  function transcriptBodySegments(runId: string, sequence: number | null | undefined, fallbackKey: string, body: string): TranscriptSegment[] {
    if (!body) return [];
    const cacheKey = sequence === undefined || sequence === null ? `${runId}:fallback:${fallbackKey}` : `${runId}:${sequence}`;
    const cached = transcriptSegmentCache.get(cacheKey);
    if (cached) return cached;
    const promptMatch = body.match(TRANSCRIPT_PROMPT_PATTERN);
    const segments: TranscriptSegment[] = [];
    let remainder = body;
    if (promptMatch?.[1]) {
      segments.push({ text: promptMatch[1], kind: 'prompt' });
      remainder = body.slice(promptMatch[1].length);
    }
    let index = 0;
    while (index < remainder.length) {
      let nextMatch: { kind: TranscriptTokenKind; start: number; end: number; text: string } | null = null;
      for (const candidate of TRANSCRIPT_TOKEN_PATTERNS) {
        const match = remainder.slice(index).match(candidate.pattern);
        if (!match || match.index === undefined) continue;
        const start = index + match.index;
        const text = match[0];
        const end = start + text.length;
        if (!nextMatch || start < nextMatch.start || (start === nextMatch.start && end > nextMatch.end)) {
          nextMatch = { kind: candidate.kind, start, end, text };
        }
      }
      if (!nextMatch) {
        segments.push({ text: remainder.slice(index), kind: null });
        break;
      }
      if (nextMatch.start > index) {
        segments.push({ text: remainder.slice(index, nextMatch.start), kind: null });
      }
      segments.push({ text: nextMatch.text, kind: nextMatch.kind });
      index = nextMatch.end;
    }
    transcriptSegmentCache.set(cacheKey, segments);
    transcriptSegmentCacheOrder.push(cacheKey);
    pruneTranscriptSegmentCache();
    return segments;
  }

  function updatePendingInputFromHistory() {
    const run = pendingInputRequest ? allHistory.find((entry) => entry.runId === pendingInputRequest?.runId) : allHistory.find((entry) => deriveQuickCommandPendingInputRequest(entry));
    pendingInputRequest = run ? deriveQuickCommandPendingInputRequest(run) : null;
    if (!pendingInputRequest) { pendingInputDraft = ''; pendingInputError = ''; }
  }

  function handleQuickCommandRunUpdated(payload: QuickCommandRunUpdatedEvent) {
    if (disposed) return;
    const existing = allHistory.find((run) => run.runId === payload.runId) ?? null;
    const nextHistory = mergeQuickCommandRunHistoryEntries(allHistory, [{
      runId: payload.runId,
      commandId: payload.commandId,
      startedAtEpochMs: existing?.startedAtEpochMs ?? payload.atEpochMs ?? Date.now(),
      finishedAtEpochMs: payload.kind === 'exit' || payload.kind === 'stopped' ? payload.atEpochMs ?? Date.now() : (existing?.finishedAtEpochMs ?? payload.atEpochMs ?? Date.now()),
      processId: payload.processId,
      exitCode: payload.kind === 'exit' ? (() => { const code = Number.parseInt(payload.body, 10); return Number.isFinite(code) ? code : null; })() : existing?.exitCode ?? null,
      stdout: existing?.stdout ?? '',
      stderr: existing?.stderr ?? '',
      stdoutTruncated: existing?.stdoutTruncated ?? false,
      stderrTruncated: existing?.stderrTruncated ?? false,
      running: payload.kind !== 'exit' && payload.kind !== 'stopped',
      transcript: [payload]
    }]);
    allHistory = nextHistory;
    if (payload.kind === 'stopping') stoppingRunIds = new Set([...stoppingRunIds, payload.runId]);
    else if (payload.kind === 'stop-failed' || payload.kind === 'stopped' || payload.kind === 'exit') stoppingRunIds = new Set([...stoppingRunIds].filter((runId) => runId !== payload.runId));
    if ((payload.kind === 'stop-failed' || payload.kind === 'stopped' || payload.kind === 'exit') && stoppingId === payload.commandId) stoppingId = null;
    if (payload.kind === 'stopped' || payload.kind === 'exit') {
      activeRunIds = new Set([...activeRunIds].filter((runId) => runId !== payload.runId));
      activeCommandIds = new Set([...activeCommandIds].filter((commandId) => commandId !== payload.commandId));
    }
    lastLiveHistoryUpdateAtByRun.set(payload.runId, Date.now());
    pruneLiveHistoryUpdateTimes();
    if (historyUpdateFrame !== null) return;
    historyUpdateQueued = true;
    historyUpdateFrame = window.requestAnimationFrame(() => {
      if (disposed) return;
      historyUpdateFrame = null;
      if (!historyUpdateQueued) return;
      historyUpdateQueued = false;
      history = editor.id ? allHistory.filter((run) => run.commandId === editor.id) : [];
      updatePendingInputFromHistory();
    });
  }

  async function submitPendingInput() {
    if (disposed) return;
    if (!pendingInputRequest || pendingInputBusy) return;
    pendingInputBusy = true; pendingInputError = '';
    try {
      const maxLength = currentPendingInputMaxLength();
      const value = normalizeDraftLength(pendingInputDraft, maxLength);
      pendingInputDraft = value;
      await sendQuickCommandInput({ id: pendingInputRequest.commandId, runId: pendingInputRequest.runId, processId: pendingInputRequest.processId, requestId: pendingInputRequest.requestId, value, secret: pendingInputRequest.secret, maxLength });
      if (disposed) return;
      pendingInputDraft = '';
      await refreshHistory();
    } catch (error) { if (disposed) return; pendingInputError = error instanceof Error ? error.message : String(error); } finally { if (!disposed) pendingInputBusy = false; }
  }

  function clearPendingInputDraft() { pendingInputDraft = ''; }
  function handlePendingInputKeydown(event: KeyboardEvent) { if (event.key === 'Escape') { event.preventDefault(); clearPendingInputDraft(); return; } if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); void submitPendingInput(); } }

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    disposed = false;
    function registerAsyncUnlistener(registration: Promise<() => void>) {
      void registration.then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlisteners.push(unlisten);
      }).catch((error) => {
        if (!disposed) console.error('Failed to register command panel listener', error);
      });
    }

    void refreshEntries();
    void refreshHistory();
    registerAsyncUnlistener(listen(IPC_EVENTS.quickCommandRunUpdated, (event: { payload: unknown }) => {
      if (disposed) return;
      handleQuickCommandRunUpdated(event.payload as QuickCommandRunUpdatedEvent);
    }));
    const keydownHandler = (event: KeyboardEvent) => handleCtrlSpaceHotkey(event);
    const keyupHandler = (event: KeyboardEvent) => {
      if (event.code === 'Space' && shellSurfaceHotkeyHandled) {
        event.preventDefault();
        event.stopPropagation();
        shellSurfaceHotkeyHandled = false;
      }
    };
    window.addEventListener('keydown', keydownHandler, true);
    window.addEventListener('keyup', keyupHandler, true);
    historyPollTimer = window.setInterval(() => { if (shouldPollHistory()) void refreshHistory(); }, 1100);
    return () => { disposed = true; stopCommandAutoScroll(); pointerDrag = null; if (historyPollTimer !== null) window.clearInterval(historyPollTimer); if (historyUpdateFrame !== null) window.cancelAnimationFrame(historyUpdateFrame); historyUpdateQueued = false; window.removeEventListener('keydown', keydownHandler, true); window.removeEventListener('keyup', keyupHandler, true); while (unlisteners.length) { try { unlisteners.pop()?.(); } catch (error) { console.error('Failed to dispose command panel listener', error); } } };
  });
</script>

<svelte:window on:click={dismissContextMenu} on:keydown={dismissContextMenuOnEscape} />

<div bind:this={panelElement} class="command-panel" id="command-panel" role="dialog" tabindex="-1" aria-labelledby="command-panel-title" style={`--command-list-width: ${listWidth}px`} on:pointermove={resizeList} on:pointerup={stopListResize} on:pointercancel={cancelListResize}>
  <header class="command-panel-header"><h1 id="command-panel-title">Quick Commands</h1><MeltActionButton class="command-panel-close-button" ariaLabel="Close quick commands" onClick={closePanel}><MaterialSymbolIcon name="close" /></MeltActionButton></header>
  {#if panelError}<p class="command-panel-error" role="alert">{panelError}</p>{/if}
  {#if pendingInputError}<p class="command-panel-error" role="alert">{pendingInputError}</p>{/if}
  <p class="command-order-live-region" aria-live="polite" aria-atomic="true">{orderLiveMessage}</p>
  <section class="command-panel-layout">
    <aside class="command-list" aria-label="Saved commands">
      <div class="command-list-header"><h2>Saved</h2><MeltActionButton class="command-text-button command-create-button" ariaLabel="Create command" onClick={startNewEntry}><img class="command-new-icon" src={commandPanelNewIconUrl} alt="" aria-hidden="true" draggable="false" /></MeltActionButton></div>
      {#if loading}<p class="command-list-state">Loading commands…</p>{:else if !entries.length}<p class="command-list-state">No quick commands saved.</p>{:else}
        <ul bind:this={commandListElement} class:command-list-reordering={draggingCommandId !== null} aria-label="Saved quick command order">
          {#each entries as entry (entry.id)}
            <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-noninteractive-element-interactions -->
            <li data-selected={editor.id === entry.id} data-command-id={entry.id} data-dragging={draggingCommandId === entry.id} data-drop-target={dropTargetCommandId === entry.id} on:contextmenu={(event) => openContextMenu(event, entry)} on:pointerdown={(event) => startCommandPointerDrag(event, entry)} on:pointermove={(event) => updateCommandPointerDrag(event, entry)} on:pointerup={(event) => finishCommandPointerDrag(event)} on:pointercancel={cancelCommandPointerDrag} on:lostpointercapture={cancelCommandPointerDrag} on:click|capture={(event) => suppressCommandRowClick(event, entry)}>
              <div class="command-row">
                <button class="command-select" type="button" aria-label={`Edit ${entry.label}`} aria-keyshortcuts="Alt+ArrowUp Alt+ArrowDown Alt+Home Alt+End" on:click={() => selectCommand(entry)} on:keydown={(event) => commandRowKeydown(event, entry)}><strong>{entry.label}</strong></button>
                <button class="command-context-trigger" type="button" aria-label={`More options for ${entry.label}`} aria-haspopup="menu" on:click|stopPropagation={() => selectCommand(entry)} on:keydown={(event) => openKeyboardContextMenu(event, entry)}></button>
                <!-- svelte-ignore a11y-no-static-element-interactions -->
                <div class="command-row-actions" on:pointerdown|stopPropagation>
                  {#if runningId === entry.id || activeCommandIds.has(entry.id)}<span class="command-spinner" aria-label={`${entry.label} is running`}></span>{/if}
                  <MeltActionButton class={`command-icon-button ${activeCommandIds.has(entry.id) || isCommandStopping(entry.id) ? 'command-stop-button' : 'command-run-button'}`} ariaLabel={isCommandStopping(entry.id) ? `${entry.label} is stopping` : activeCommandIds.has(entry.id) ? `Stop ${entry.label}` : `Run ${entry.label}`} disabled={Boolean(runningId || stoppingId === entry.id || isCommandStopping(entry.id))} onClick={() => void (activeCommandIds.has(entry.id) ? stopEntry(entry.id) : runEntry(entry))}>{#if activeCommandIds.has(entry.id) || isCommandStopping(entry.id)}{#if isCommandStopping(entry.id)}Stopping...{:else}<svg viewBox="0 0 16 16" aria-hidden="true"><rect x="3.5" y="3.5" width="9" height="9" /></svg>{/if}{:else}<svg viewBox="0 0 16 16" aria-hidden="true"><path d="M4 2.5v11L13 8 4 2.5Z" /></svg>{/if}</MeltActionButton>
                  <MeltActionButton class="command-icon-button command-delete-button" ariaLabel={`Delete ${entry.label}`} disabled={Boolean(runningId || structuralMutationPending || activeCommandIds.has(entry.id))} onClick={(event) => void deleteEntry(entry.id, event)}><img class="command-delete-icon" src={commandPanelDeleteIconUrl} alt="" aria-hidden="true" draggable="false" /></MeltActionButton>
                </div>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </aside>
    <button class="command-list-resize-grip" type="button" aria-label="Resize saved commands pane" on:pointerdown={startListResize}></button>
    <section class="command-editor" aria-label="Command editor">
      <div class="command-pane-header">{#if activeTab === 'previousRuns'}<MeltActionButton class="command-icon-button command-artifact-open-button" ariaLabel="Open artifact folder" disabled={artifactOpenBusy || structuralMutationPending || !selectedSavedCommand?.artifactLocation} onClick={() => void openSelectedArtifactLocation()}><MaterialSymbolIcon name="folder" /></MeltActionButton>{:else}<div><p>DETAILS</p><h2>{editor.id ? editor.label : 'New command'}</h2></div>{/if}<div class="command-pane-tabs" role="tablist" aria-label="Quick command panels"><MeltActionButton class={`command-pane-tab ${activeTab === 'configuration' ? 'active' : ''}`} role="tab" ariaSelected={activeTab === 'configuration'} ariaControls="command-panel-configuration" onClick={() => { cancelReorderForAction(); activeTab = 'configuration'; }}>Configuration</MeltActionButton><MeltActionButton class={`command-pane-tab ${activeTab === 'previousRuns' ? 'active' : ''}`} role="tab" ariaSelected={activeTab === 'previousRuns'} ariaControls="command-panel-previous-runs" onClick={() => { cancelReorderForAction(); activeTab = 'previousRuns'; void refreshHistory(); }}>Previous runs</MeltActionButton></div></div>
      {#if pendingInputRequest}
        {@const pending = pendingInputRequest}
        <section class="command-input-panel" aria-label="Backend input required">
          <div class="command-input-meta"><div><p>{pending.kind}</p><strong>{pending.prompt || 'Input required'}</strong></div><span>{pending.secret ? 'Password' : 'Input'}</span></div>
          <label class="command-input-field"><span>{pending.secret ? 'Secret input' : 'Input'}</span>{#if pending.secret}<input bind:this={pendingInputInput} type="password" value={pendingInputDraft} disabled={pendingInputBusy} maxlength={currentPendingInputMaxLength()} on:input={(event) => (pendingInputDraft = normalizeDraftLength(inputValue(event), currentPendingInputMaxLength()))} on:keydown={handlePendingInputKeydown} />{:else}<textarea bind:this={pendingInputInput} rows="3" value={pendingInputDraft} disabled={pendingInputBusy} maxlength={currentPendingInputMaxLength()} on:input={(event) => (pendingInputDraft = normalizeDraftLength(textareaValue(event), currentPendingInputMaxLength()))} on:keydown={handlePendingInputKeydown}></textarea>{/if}</label>
          <div class="command-editor-actions"><MeltActionButton class="command-text-button" ariaLabel="Submit input" disabled={pendingInputBusy} onClick={() => void submitPendingInput()}>Send</MeltActionButton><MeltActionButton class="command-text-button" ariaLabel="Clear draft" disabled={pendingInputBusy} onClick={clearPendingInputDraft}>Clear draft</MeltActionButton></div>
        </section>
      {/if}
      {#if activeTab === 'configuration'}
        <div id="command-panel-configuration" class="command-pane" role="tabpanel">{#if formErrors.length}<ul class="command-form-errors" role="alert">{#each formErrors as error (error)}<li>{error}</li>{/each}</ul>{/if}<label><span>Label</span><input value={editor.label} maxlength="96" spellcheck="false" on:input={(event) => (editor = { ...editor, label: inputValue(event) })} /></label><label><span>Mode</span><select value={editor.mode} on:change={(event) => (editor = { ...editor, mode: selectedMode(event) })}>{#each QUICK_COMMAND_MODES as mode}<option value={mode}>{modeLabels[mode]}</option>{/each}</select></label>{#if editor.mode === 'direct'}<label><span>Program</span><input value={editor.targetPath} spellcheck="false" placeholder="git.exe" on:input={(event) => (editor = { ...editor, targetPath: inputValue(event) })} /></label>{/if}<label><span>Working directory</span><input value={editor.cwd} spellcheck="false" placeholder="Optional absolute path" on:input={(event) => (editor = { ...editor, cwd: inputValue(event) })} /></label><label><span>Artifact location</span><div class="command-artifact-input-shell"><input value={editor.artifactLocation} spellcheck="false" placeholder="Optional absolute Windows path" on:input={(event) => (editor = { ...editor, artifactLocation: inputValue(event) })} /><MeltActionButton class="command-icon-button command-artifact-picker-button" ariaLabel="Pick artifact folder" disabled={artifactPickerBusy} onClick={() => void pickArtifactLocation()}><MaterialSymbolIcon name="folder" /></MeltActionButton></div></label>{#if editor.mode === 'direct'}<label><span>Arguments (one per line)</span><textarea rows="5" spellcheck="false" value={editor.argsText} on:input={(event) => (editor = { ...editor, argsText: textareaValue(event) })}></textarea></label>{:else}<label><span>Commands (one per line)</span><textarea rows="8" spellcheck="false" value={editor.commandsText} placeholder={'cd C:\\dev\\my-app\npython app.py'} on:input={(event) => (editor = { ...editor, commandsText: textareaValue(event) })}></textarea></label>{/if}<div class="command-editor-actions"><MeltActionButton class="command-text-button command-editor-icon-button" ariaLabel="Save command" disabled={saving || Boolean(runningId)} onClick={() => void saveEntry()}><img class="command-save-icon" src={commandPanelSaveIconUrl} alt="" aria-hidden="true" draggable="false" />{saving ? 'Saving…' : ''}</MeltActionButton><MeltActionButton class="command-text-button command-editor-icon-button" ariaLabel="Cancel command editing" disabled={saving || Boolean(runningId)} onClick={startNewEntry}><img class="command-cancel-icon" src={commandPanelCancelIconUrl} alt="" aria-hidden="true" draggable="false" /></MeltActionButton></div></div>
      {:else}
        <div id="command-panel-previous-runs" class="command-pane" role="tabpanel" aria-busy={historyLoading}><div class="command-history-host">{#if historyLoading && !history.length}<p class="command-list-state command-history-loading">Loading output…</p>{:else if !editor.id}<p class="command-list-state">Select a command to view runs.</p>{:else if !history.length}<p class="command-list-state">No runs yet.</p>{/if}<div class="command-history-list">{#if history.length}{#each history as run (historyRunKey(run))}<details class="command-history-run" open={run.running || isRunExpanded(run)} on:toggle={(event) => handleHistoryRunToggle(event, run)}><summary class:running={run.running} aria-label={historyRunSummary(run)} ><div class="command-history-meta"><strong>{commandLabelFor(run.commandId)}</strong><span>{historyRunStatus(run)} · {formatRunTime(run.startedAtEpochMs)} · PID {run.processId}</span></div>{#if run.running}<span class="command-history-live">Live</span>{/if}</summary><div class="command-history-body"><!-- svelte-ignore a11y-no-noninteractive-tabindex a11y-no-static-element-interactions a11y-no-noninteractive-element-interactions --><div class="command-transcript-shell" role="region" tabindex="0" aria-label="Merged transcript" on:keydown={handleTranscriptKeydown} on:contextmenu={handleTranscriptContextMenu}>{#if run.transcript.length}{#each run.transcript as line (line.sequence ?? `${line.kind}:${line.requestId ?? line.body}:${line.atEpochMs ?? ''}`)}<div class={`command-transcript-line ${transcriptLineClass(line.kind)} ${line.secret ? 'secret' : ''} ${line.redacted ? 'redacted' : ''}`} data-kind={line.kind}>{#each transcriptBodySegments(run.runId, line.sequence, `${line.kind}:${line.requestId ?? line.body}:${line.atEpochMs ?? ''}`, line.body) as segment, segmentIndex (segmentIndex)}{#if segment.kind === 'url'}<a class={`command-transcript-token command-transcript-token--${segment.kind}`} href={segment.text} rel="noreferrer noopener" on:click={(event) => handleTranscriptUrlClick(event, segment.text)} on:auxclick={(event) => handleTranscriptUrlAuxClick(event, segment.text)} on:contextmenu={(event) => handleTranscriptUrlContextMenu(event)}>{segment.text}</a>{:else if segment.kind}<span class={`command-transcript-token command-transcript-token--${segment.kind}`}>{segment.text}</span>{:else}{segment.text}{/if}{/each}</div>{/each}{:else if run.stdout || run.stderr}<pre class="command-transcript-body">{run.stdout}{run.stderr}</pre>{:else}<p class="command-list-state">Waiting for transcript…</p>{/if}</div></div></details>{/each}{/if}</div></div></div>
      {/if}
    </section>
  </section>
  {#if contextEntry}<div bind:this={contextMenuElement} class="command-context-menu" role="menu" style={`left: ${contextMenuPosition.x}px; top: ${contextMenuPosition.y}px`}><button bind:this={contextMenuFirstAction} type="button" role="menuitem" on:click={showHistory}>View output history</button><button type="button" role="menuitem" on:click={editContextEntry}>Edit command</button><button type="button" role="menuitem" on:click={duplicateContextEntry}>Duplicate command</button></div>{/if}
  {#if deleteConfirmation}<div class="delete-confirm-backdrop" role="presentation" on:click|stopPropagation><div bind:this={deleteConfirmationDialog} class="delete-confirm-dialog" role="alertdialog" tabindex="-1" aria-modal="true" aria-labelledby="command-delete-confirm-title" aria-describedby="command-delete-confirm-message" on:keydown={handleDeleteConfirmationKeydown}><h2 id="command-delete-confirm-title">Confirm Delete</h2><p id="command-delete-confirm-message">Delete quick command “{deleteConfirmation.label}”? This cannot be undone.</p><div class="delete-confirm-actions"><MeltActionButton class="command-text-button" disabled={deleteConfirmationBusy} onClick={cancelDeleteEntry}>Cancel</MeltActionButton><MeltActionButton class="command-text-button danger" disabled={deleteConfirmationBusy} onClick={() => void confirmDeleteEntry()}>Delete</MeltActionButton></div></div></div>{/if}
</div>

