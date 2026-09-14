import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const appSource = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const surfaceLoaderSource = readFileSync(new URL('../src/lib/surfaceLoader.ts', import.meta.url), 'utf8');
const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const topBarCss = readFileSync(new URL('../src/components/TopBar.css', import.meta.url), 'utf8');
const commandPanelSource = readFileSync(new URL('../src/components/CommandPanelSurface.svelte', import.meta.url), 'utf8');
const commandPanelCss = readFileSync(new URL('../src/components/CommandPanelSurface.css', import.meta.url), 'utf8');
const commandPanelNewIconPath = new URL('../src/assets/icons/add_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg', import.meta.url);
const commandPanelDeleteIconPath = new URL('../src/assets/icons/delete_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg', import.meta.url);
const commandPanelSaveIconPath = new URL('../src/assets/icons/save_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg', import.meta.url);
const commandPanelCancelIconPath = new URL('../src/assets/icons/cancel_presentation_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24.svg', import.meta.url);
const shellSurfaceSource = readFileSync(new URL('../src/lib/shellSurface.ts', import.meta.url), 'utf8');
const ipcSurfacesSource = readFileSync(new URL('../src/ipc/surfaces.ts', import.meta.url), 'utf8');
const ipcEventsSource = readFileSync(new URL('../src/ipc/events.ts', import.meta.url), 'utf8');
const ipcCommandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const commandPanelWrapper = readFileSync(new URL('../src/lib/commandPanel.ts', import.meta.url), 'utf8');
const shellWindowsSource = readFileSync(new URL('../src-tauri/src/shell_windows.rs', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const commandPanelRs = readFileSync(new URL('../src-tauri/src/command_panel.rs', import.meta.url), 'utf8');
const contractsSource = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');
const capabilitySource = readFileSync(new URL('../src-tauri/capabilities/command-panel.json', import.meta.url), 'utf8');

test('command panel is routed as a dedicated auxiliary shell surface', () => {
  assert.match(appSource, /loadSurfaceComponent\(surface\)/);
  assert.match(surfaceLoaderSource, /'command-panel': \(\) => import\('\.\.\/components\/CommandPanelSurface\.svelte'\)/);
  assert.match(shellSurfaceSource, /\| 'command-panel'/);
  assert.match(shellWindowsSource, /COMMAND_PANEL_LABEL: &str = "command-panel"/);
  assert.match(shellWindowsSource, /COMMAND_PANEL_WIDTH_LOGICAL: f64 = 460\.0/);
  assert.match(shellWindowsSource, /COMMAND_PANEL_HEIGHT_LOGICAL: f64 = 420\.0/);
  assert.match(shellWindowsSource, /build_command_panel_window\(app\)/);
  assert.match(shellWindowsSource, /fn build_command_panel_window\(app: &App\)[\s\S]*?\.resizable\(true\)/);
  assert.match(mainSource, /mod command_panel;/);
  assert.match(mainSource, /command_panel::show_command_panel/);
  assert.match(mainSource, /command_panel::hide_command_panel/);
  assert.match(mainSource, /shell_windows::COMMAND_PANEL_LABEL[\s\S]*WindowEvent::Focused\(false\)/);
  assert.match(mainSource, /emit_to\(\s*shell_windows::TOP_BAR_LABEL,\s*command_panel::COMMAND_PANEL_CLOSED_EVENT/);
  assert.match(commandPanelRs, /pub fn show_command_panel/);
  assert.match(commandPanelRs, /pub fn hide_command_panel/);
  assert.match(commandPanelRs, /pub fn save_command_panel_size/);
  assert.match(capabilitySource, /"command-panel"/);
});

test('command panel contracts and wrappers use constant-backed IPC and event names', () => {
  assert.match(ipcCommandsSource, /showCommandPanel: 'show_command_panel'/);
  assert.match(ipcCommandsSource, /hideCommandPanel: 'hide_command_panel'/);
  assert.match(ipcCommandsSource, /saveCommandPanelSize: 'save_command_panel_size'/);
  assert.match(ipcEventsSource, /commandPanelClosed: 'command-panel:closed'/);
  assert.match(ipcSurfacesSource, /commandPanel: 'command-panel'/);
  assert.match(contractsSource, /COMMAND_PANEL/);
  assert.match(contractsSource, /SHOW_COMMAND_PANEL/);
  assert.match(contractsSource, /HIDE_COMMAND_PANEL/);
  assert.match(contractsSource, /COMMAND_PANEL_CLOSED/);
  assert.match(commandPanelWrapper, /invoke\(IPC_COMMANDS\.showCommandPanel/);
  assert.match(commandPanelWrapper, /invoke\(IPC_COMMANDS\.hideCommandPanel/);
  assert.match(commandPanelWrapper, /invoke\(IPC_COMMANDS\.saveCommandPanelSize/);
  assert.match(commandPanelWrapper, /invoke<string \| null>\(IPC_COMMANDS\.pickQuickCommandArtifactLocation/);
  assert.doesNotMatch(commandPanelWrapper, /invoke\('show_command_panel'/);
  assert.doesNotMatch(commandPanelWrapper, /invoke\('hide_command_panel'/);
});

test('top bar command button is left of tray button and enforces popup exclusivity', () => {
  assert.match(topBarSource, /from '\.\.\/lib\/commandPanel'/);
  assert.match(topBarSource, /COMMAND_PANEL_CLOSED_EVENT/);
  assert.match(topBarSource, /class="command-control"[\s\S]*class="tray-control"/);
  assert.match(topBarSource, /class="command-button"[\s\S]*ariaControls=\{COMMAND_PANEL_ID\}/);
  assert.match(topBarSource, /ariaLabel="Open quick commands"/);
  assert.match(topBarSource, /await closePanel\(\);[\s\S]*await closeAudioPanel\(\);[\s\S]*await closeTrayPanel\(\);[\s\S]*await showCommandPanel\(\{/);
  assert.match(topBarSource, /if \(commandOpen && \(!target \|\| !commandControl\?\.contains\(target\)\)\) \{[\s\S]*void closeCommandPanel\(\);/);
  assert.match(topBarSource, /(?:void listen|registerAsyncUnlistener\(listen)\(COMMAND_PANEL_CLOSED_EVENT, \(\) => \{[\s\S]*commandOpen = false;/);
  assert.match(topBarCss, /\.top-bar \.command-button \{/);
  assert.match(topBarSource, /<MaterialSymbolIcon name="code_blocks" \/>/);
});

test('command panel surface includes compact list actions, resize controls, and command-block editor flow', () => {
  assert.match(commandPanelSource, /id="command-panel"[\s\S]*role="dialog"/);
  assert.match(commandPanelSource, /function selectCommand\(entry: QuickCommandEntry\) \{[\s\S]*startEditEntry\(entry\);[\s\S]*contextEntry = null;[\s\S]*\}/);
  assert.match(commandPanelSource, /data-selected=\{editor\.id === entry\.id\}/);
  assert.doesNotMatch(commandPanelSource, /role="listbox"/);
  assert.doesNotMatch(commandPanelSource, /role="option"/);
  assert.doesNotMatch(commandPanelSource, /aria-selected=\{editor\.id === entry\.id\}/);
  assert.match(commandPanelSource, /on:click=\{\(\) => selectCommand\(entry\)\}/);
  assert.match(commandPanelSource, /on:keydown=\{\(event\) => commandRowKeydown\(event, entry\)\}/);
  assert.match(commandPanelSource, /command-run-button/);
  assert.match(commandPanelSource, /Edit/);
  assert.match(commandPanelSource, /command-delete-button/);
  assert.match(commandPanelSource, /IPC_EVENTS\.quickCommandRunUpdated/);
  assert.match(commandPanelSource, /sendQuickCommandInput/);
  assert.match(commandPanelSource, /normalizeDraftLength/);
  assert.match(commandPanelSource, /pendingInputRequest/);
  assert.match(commandPanelSource, /type="password"/);
  assert.match(commandPanelSource, /Escape/);
  assert.match(commandPanelSource, /Merged transcript/);
  assert.match(commandPanelSource, /class="command-panel-close-button"/);
  assert.doesNotMatch(commandPanelSource, /<p>JasonShell<\/p>/);
  assert.match(commandPanelSource, /class="command-text-button"/);
  assert.match(commandPanelSource, /Saving…/);
  assert.match(commandPanelSource, /Clear/);
  assert.match(commandPanelSource, /Mode/);
  assert.match(commandPanelSource, /Program/);
  assert.match(commandPanelSource, /Working directory/);
  assert.match(commandPanelSource, /Arguments \(one per line\)/);
  assert.match(commandPanelSource, /Commands \(one per line\)/);
  assert.match(commandPanelSource, /parseQuickCommandArgsTextarea/);
  assert.match(commandPanelSource, /formatQuickCommandArgsTextarea/);
  assert.match(commandPanelSource, /parseQuickCommandCommandsTextarea/);
  assert.match(commandPanelSource, /formatQuickCommandCommandsTextarea/);
  assert.match(commandPanelSource, /saveQuickCommandsSettings/);
  assert.match(commandPanelSource, /runQuickCommand/);
  assert.match(commandPanelSource, /quickCommandRunRequest/);
  assert.match(commandPanelSource, /stopQuickCommand/);
  assert.match(commandPanelSource, /listQuickCommandHistory/);
  assert.match(commandPanelSource, /listQuickCommandHistory\(\)/);
  assert.match(commandPanelSource, /const allRuns = await listQuickCommandHistory\(\);/);
  assert.match(commandPanelSource, /const selectedId = editor\.id;/);
  assert.match(commandPanelSource, /Configuration/);
  assert.match(commandPanelSource, /Previous runs/);
  assert.match(commandPanelSource, /\{#each history as run \(historyRunKey\(run\)\)\}/);
  assert.match(commandPanelSource, /class="command-history-list"[\s\S]*\{#if history\.length\}/);
  assert.match(commandPanelSource, /aria-busy=\{historyLoading\}/);
  assert.doesNotMatch(commandPanelSource, /history\.slice\(0, 12\)/);
  assert.match(commandPanelSource, /historyRunStatus/);
  assert.match(commandPanelSource, /if \(saving\) return;/);
  assert.match(commandPanelSource, /const requestId = \+\+historyRequestId;/);
  assert.match(commandPanelSource, /if \(requestId !== historyRequestId \|\| editor\.id !== selectedId\) return;/);
  assert.doesNotMatch(commandPanelSource, /history = \[\];/);
  assert.match(commandPanelSource, /historyLoading = false;/);
  assert.match(commandPanelSource, /on:contextmenu/);
  assert.match(commandPanelSource, /openKeyboardContextMenu/);
  assert.match(commandPanelSource, /on:keydown=\{dismissContextMenuOnEscape\}/);
  assert.match(commandPanelSource, /View output history/);
  assert.match(commandPanelSource, /function showHistory\(\)/);
  assert.match(commandPanelSource, /activeTab = 'previousRuns'/);
  assert.match(commandPanelSource, /Merged transcript/);
  assert.match(commandPanelSource, /command-transcript-line/);
  assert.doesNotMatch(commandPanelSource, /aria-label=\{`Transcript \$\{line\.kind\}`\}/);
  assert.match(commandPanelSource, /transcriptBodySegments/);
  assert.match(commandPanelSource, /segment\.kind === 'url'/);
  assert.match(commandPanelSource, /openQuickCommandUrl/);
  assert.match(commandPanelSource, /href=\{segment\.text\}/);
  assert.match(commandPanelSource, /TRANSCRIPT_PROMPT_PATTERN/);
  assert.match(commandPanelSource, /TRANSCRIPT_SEGMENT_CACHE_LIMIT/);
  assert.match(commandPanelSource, /transcriptBodySegments\(run\.runId, line\.sequence, `\$\{line\.kind\}:\$\{line\.requestId \?\? line\.body\}:\$\{line\.atEpochMs \?\? ''\}`, line\.body\)/);
  assert.match(commandPanelSource, /sequence === undefined \|\| sequence === null \? `\$\{runId\}:fallback:\$\{fallbackKey\}` : `\$\{runId\}:\$\{sequence\}`/);
  assert.doesNotMatch(commandPanelSource, /TRANSCRIPT_SEGMENT_CACHE_LIMIT \+ 1|> 256/);
  assert.match(commandPanelSource, /historyUpdateFrame/);
  assert.match(commandPanelSource, /requestAnimationFrame/);
  assert.match(commandPanelSource, /cancelAnimationFrame/);
  assert.match(commandPanelCss, /\[data-kind='stderr'\]/);
  assert.match(commandPanelCss, /\[data-kind='system'\]/);
  assert.match(commandPanelCss, /\[data-kind='input-request'\]/);
  assert.match(commandPanelCss, /command-transcript-token--url/);
  assert.match(commandPanelCss, /cursor: pointer/);
  assert.match(commandPanelCss, /:focus-visible/);
  assert.match(commandPanelCss, /command-transcript-token--level-error/);
  assert.match(commandPanelCss, /display: block;/);
  assert.match(commandPanelSource, /command-list-resize-grip/);
  assert.match(commandPanelSource, /historyRunStatus/);
  assert.match(commandPanelSource, /activeTab === 'previousRuns'/);
  assert.match(commandPanelSource, /command-history-run/);
  assert.match(commandPanelSource, /<details class="command-history-run" open=\{run\.running \|\| isRunExpanded\(run\)\} on:toggle=\{\(event\) => handleHistoryRunToggle\(event, run\)\}><summary class:running=\{run\.running\} aria-label=\{historyRunSummary\(run\)\}\s*>\s*<div class="command-history-meta">/);
  assert.doesNotMatch(commandPanelSource, /<summary[^>]*on:(?:click|keydown)=/);
  assert.match(commandPanelSource, /command-transcript-shell/);
  assert.match(commandPanelSource, /command-transcript-line/);
  assert.match(commandPanelCss, /user-select:\s*text/);
  assert.match(commandPanelCss, /\.command-transcript-shell \{[^}]*max-height: 18rem;[^}]*overflow: auto;/s);
  assert.match(commandPanelSource, /document\.execCommand\('copy'\)/);
  assert.match(commandPanelSource, /handleTranscriptContextMenu/);
  assert.match(commandPanelSource, /role="region"/);
  assert.match(commandPanelSource, /on:keydown=\{handleTranscriptKeydown\}/);
  assert.match(commandPanelSource, /on:click=\{\(event\) => handleTranscriptUrlClick\(event, segment\.text\)\}/);
  assert.match(commandPanelSource, /on:auxclick=\{\(event\) => handleTranscriptUrlAuxClick\(event, segment\.text\)\}/);
  assert.match(commandPanelSource, /on:contextmenu=\{\(event\) => handleTranscriptUrlContextMenu\(event\)\}/);
  assert.doesNotMatch(commandPanelSource, /handleTranscriptUrlActivate/);
  assert.match(commandPanelSource, /Duplicate command/);
  assert.match(commandPanelSource, /nextDuplicateQuickCommandLabel/);
  assert.match(commandPanelSource, /nextUniqueQuickCommandId/);
  assert.match(commandPanelSource, /\? `Stop \$\{entry\.label\}` : `Run \$\{entry\.label\}`/);
  assert.match(commandPanelSource, /command-stop-button/);
  assert.match(commandPanelSource, /IPC_EVENTS\.quickCommandRunUpdated/);
  assert.match(commandPanelSource, /sendQuickCommandInput/);
  assert.match(commandPanelSource, /pendingInputRequest/);
  assert.match(commandPanelSource, /type="password"/);
  assert.match(commandPanelSource, /Escape/);
  assert.match(commandPanelSource, /Merged transcript/);
  assert.match(commandPanelSource, /Select a command to view runs\./);
  assert.match(commandPanelSource, /command-transcript-line/);
  assert.match(commandPanelCss, /command-transcript-token--path/);
  assert.match(commandPanelCss, /command-transcript-token--level-success/);
  assert.match(commandPanelCss, /\[data-kind='stderr'\]/);
  assert.match(commandPanelCss, /\[data-kind='system'\]/);
  assert.match(commandPanelCss, /\[data-kind='input-request'\]/);
  assert.match(commandPanelCss, /display: block;/);
  assert.match(commandPanelSource, /delete-confirm-backdrop/);
  assert.match(commandPanelSource, /role="alertdialog"/);
  assert.match(commandPanelSource, /bind:this=\{deleteConfirmationDialog\}/);
  assert.match(commandPanelSource, /on:keydown=\{handleDeleteConfirmationKeydown\}/);
  assert.match(commandPanelSource, /deleteConfirmation/);
  assert.match(commandPanelSource, /Cancel/);
  assert.match(commandPanelSource, /Confirm Delete/);
  assert.match(commandPanelSource, /deleteConfirmationBusy/);
  assert.match(commandPanelSource, /focusDeleteConfirmation\(\)/);
  assert.match(commandPanelSource, /restoreDeleteTriggerFocus\(\)/);
  assert.match(commandPanelSource, /const latestRuns = await listQuickCommandHistory\(\{ id \}\);/);
  assert.match(commandPanelSource, /if \(latestRuns\.some\(\(run\) => run\.running\)\) \{[\s\S]*Cannot delete quick command while it is running\./);
  assert.match(commandPanelSource, /event\.key === 'Escape'/);
  assert.match(commandPanelSource, /event\.key !== 'Tab'/);
  assert.match(commandPanelSource, /focusDeleteConfirmationButton\(event\.shiftKey \? -1 : 1\)/);
  assert.match(commandPanelSource, /disabled=\{deleteConfirmationBusy\}/);
  assert.match(commandPanelSource, /let activeRunIds = new Set<string>\(\);/);
  assert.match(commandPanelSource, /let activeCommandIds = new Set<string>\(\);/);
  assert.match(commandPanelSource, /activeRunIds = new Set\(allHistory\.filter\(\(run\) => run\.running\)\.map\(\(run\) => run\.runId\)\);/);
  assert.match(commandPanelSource, /activeCommandIds = new Set\(allHistory\.filter\(\(run\) => run\.running\)\.map\(\(run\) => run\.commandId\)\);/);
  assert.match(commandPanelSource, /Contract: activeCommandIds holds command IDs; freshness map keys must match the same ID domain\./);
  assert.match(commandPanelSource, /function shouldPollHistory\(\): boolean \{[\s\S]*pendingInputRequest !== null[\s\S]*activeRunIds\.size === 0[\s\S]*lastLiveAt === undefined \|\| now - lastLiveAt >= 1100[\s\S]*\}/);
  assert.match(commandPanelSource, /lastLiveHistoryUpdateAtByRun/);
  assert.match(commandPanelSource, /pruneLiveHistoryUpdateTimes\(\)/);
  assert.match(commandPanelSource, /window\.setInterval\(/);
  assert.match(commandPanelSource, /shouldPollHistory\(\)/);
  assert.match(commandPanelSource, /void refreshHistory\(\)/);
  assert.match(commandPanelSource, /historyPollTimer/);
  assert.match(commandPanelCss, /grid-template-columns: minmax\(8rem, var\(--command-list-width\)\)/);
  assert.match(commandPanelCss, /animation: command-spin 1100ms linear infinite/);
  assert.match(commandPanelSource, /hideCommandPanel/);
  assert.match(commandPanelCss, /\.command-panel \{/);
});

test('quick command artifact folder can be typed or picked and is opened by saved command id', () => {
  assert.match(ipcCommandsSource, /pickQuickCommandArtifactLocation: 'pick_quick_command_artifact_location'/);
  assert.match(ipcCommandsSource, /openQuickCommandArtifactLocation: 'open_quick_command_artifact_location'/);
  assert.match(contractsSource, /PICK_QUICK_COMMAND_ARTIFACT_LOCATION/);
  assert.match(contractsSource, /OPEN_QUICK_COMMAND_ARTIFACT_LOCATION/);
  assert.match(mainSource, /command_panel::pick_quick_command_artifact_location/);
  assert.match(mainSource, /quick_commands::open_quick_command_artifact_location/);
  assert.match(commandPanelRs, /begin_command_panel_focus_loss_hold/);
  assert.match(commandPanelRs, /FocusHoldGuard/);
  assert.match(commandPanelRs, /picker_lifecycle_nonce/);
  assert.match(commandPanelRs, /command_panel_focus_loss_nonce_is_current\(picker_lifecycle_nonce\)/);
  assert.match(commandPanelRs, /FOS_PICKFOLDERS/);
  assert.match(commandPanelSource, /Artifact location/);
  assert.match(commandPanelSource, /class="command-artifact-input-shell"><input value=\{editor\.artifactLocation\}[^>]*on:input=\{\(event\) => \(editor = \{ \.\.\.editor, artifactLocation: inputValue\(event\) \}\)\} \/><MeltActionButton class="command-icon-button command-artifact-picker-button" ariaLabel="Pick artifact folder"/);
  assert.doesNotMatch(commandPanelSource, /artifactLocation\}[^>]*readonly/);
  assert.doesNotMatch(commandPanelSource, /ariaLabel="Clear artifact folder"|>Clear<|Picking…|>Pick folder</);
  assert.match(commandPanelSource, /pickQuickCommandArtifactLocation/);
  assert.match(commandPanelSource, /openQuickCommandArtifactLocation\(editor\.id\)/);
  assert.doesNotMatch(commandPanelSource, /openQuickCommandArtifactLocation\([^)]*artifactLocation/);
  assert.match(commandPanelSource, /class="command-pane-header">\{#if activeTab === 'previousRuns'\}<MeltActionButton class="command-icon-button command-artifact-open-button"/);
  assert.doesNotMatch(commandPanelSource, /command-history-header-actions/);
  assert.match(commandPanelCss, /\.command-artifact-input-shell \{[^}]*display: grid;[^}]*grid-template-columns: minmax\(0, 1fr\) auto;[^}]*position: relative;/);
  assert.match(commandPanelCss, /\.command-artifact-input-shell > input \{[^}]*padding-right:/);
  assert.match(commandPanelCss, /\.command-artifact-picker-button \{[^}]*grid-column: 2;[^}]*justify-self: end;/);
});

test('artifact actions preserve panel lifecycle and settings mutation authority', () => {
  assert.match(commandPanelRs, /fn artifact_picker_restore_requires_current_open_lifecycle/);
  assert.match(commandPanelRs, /artifact_picker_completion_does_not_restore_after_explicit_hide/);
  assert.match(commandPanelSource, /function openSelectedArtifactLocation\(\)[\s\S]*structuralMutationBusy\(\)/);
  assert.match(commandPanelSource, /ariaLabel="Open artifact folder" disabled=\{artifactOpenBusy \|\| structuralMutationPending \|\| !selectedSavedCommand\?\.artifactLocation\}/);
});

test('quick command ordering uses versioned migration, authoritative arrays, and one mutation pipeline', () => {
  assert.match(commandPanelSource, /QUICK_COMMAND_ORDER_LEGACY/);
  assert.match(commandPanelSource, /QUICK_COMMAND_ORDER_VERSION/);
  assert.match(commandPanelSource, /sortQuickCommandsForLegacyMigration/);
  assert.match(commandPanelSource, /orderMigrationAttempted/);
  assert.match(commandPanelSource, /Quick command order migration could not be saved/);
  assert.doesNotMatch(commandPanelSource, /function sortedEntries/);
  assert.match(commandPanelSource, /entries = cloneEntries\(quickCommands\.entries\)/);
  assert.match(commandPanelSource, /const existingIndex = editor\.id \? entries\.findIndex/);
  assert.match(commandPanelSource, /existingIndex < 0[\s\S]*\[\.\.\.entries, nextEntry\]/);
  assert.match(commandPanelSource, /index === existingIndex \? nextEntry : entry/);
  assert.match(commandPanelSource, /function enqueueSettingsMutation/);
  assert.match(commandPanelSource, /\$: structuralMutationPending = mutationInFlight \|\| mutationQueue\.length > 0;/);
  assert.match(commandPanelSource, /mutationInFlight/);
  assert.match(commandPanelSource, /latestAcceptedSettings/);
  assert.match(commandPanelSource, /saveQuickCommandsSettings\(mutation\.desired\)/);
  assert.match(commandPanelSource, /function applyAcceptedSettings\(values: SettingsSnapshot\) \{[\s\S]*entries = cloneEntries\(values\.entries\);[\s\S]*listWidth = values\.listWidth;/);
  assert.doesNotMatch(commandPanelSource, /allHistory = saved\.history/);
  assert.match(commandPanelSource, /function restoreReorderEntries/);
  assert.match(commandPanelSource, /liveEntriesById/);
  assert.match(commandPanelSource, /disabled=\{Boolean\(runningId \|\| stoppingId === entry\.id/);
  assert.match(commandPanelSource, /disabled=\{Boolean\(runningId \|\| structuralMutationPending \|\| activeCommandIds\.has\(entry\.id\)\)\}/);
  assert.match(commandPanelSource, /moveQuickCommandById/);
  assert.match(commandPanelSource, /setPointerCapture/);
  assert.match(commandPanelSource, /Math\.hypot\(/);
  assert.match(commandPanelSource, /getBoundingClientRect\(\)/);
  assert.match(commandPanelSource, /requestAnimationFrame\(tickAutoScroll\)/);
  assert.match(commandPanelSource, /cancelCommandPointerDrag/);
  assert.match(commandPanelSource, /suppressCommandClickId/);
  assert.doesNotMatch(commandPanelSource, /command-reorder-handle|⠿/);
  assert.match(commandPanelSource, /pointerTarget\?\.closest\('\.command-row-actions'\)/);
  assert.match(commandPanelSource, /event\.target instanceof Element && event\.target\.closest\('\.command-row-actions'\)/);
  assert.match(commandPanelSource, /class="command-row-actions" on:pointerdown\|stopPropagation/);
  assert.match(commandPanelSource, /\.filter\(\(row\) => row\.dataset\.commandId !== pointerDrag\?\.id\)/);
  assert.match(commandPanelSource, /filter\(\(row\) => row\.dataset\.commandId !== pointerDrag\?\.id\)[\s\S]*clientY > rect\.top \+ rect\.height \/ 2/);
  assert.doesNotMatch(commandPanelSource, /sourceIndex|insertionIndex > sourceIndex/);
  assert.match(commandPanelSource, /pointerDrag\.targetIndex = targetIndex;/);
  assert.match(commandPanelSource, /const moved = moveQuickCommandById\(drag\.original, drag\.id, drag\.targetIndex\);/);
  const pointerPreviewSource = commandPanelSource.slice(
    commandPanelSource.indexOf('function applyPointerDragPosition'),
    commandPanelSource.indexOf('function startCommandPointerDrag')
  );
  assert.doesNotMatch(pointerPreviewSource, /entries = moved;/);
  assert.match(commandPanelSource, /event\.altKey[\s\S]*event\.key === 'ArrowUp'/);
  assert.match(commandPanelSource, /aria-keyshortcuts="Alt\+ArrowUp Alt\+ArrowDown Alt\+Home Alt\+End"/);
  assert.match(commandPanelSource, /ArrowUp/);
  assert.match(commandPanelSource, /ArrowDown/);
  assert.match(commandPanelSource, /cancelReorderForAction/);
  assert.match(commandPanelSource, /if \(!pointerDrag\) return false;/);
  assert.match(commandPanelSource, /row\?\.hasPointerCapture\(drag\.pointerId\)/);
  assert.match(commandPanelSource, /target\?\.hasPointerCapture\(event\.pointerId\)/);
  assert.match(commandPanelSource, /aria-live="polite"/);
  assert.match(commandPanelCss, /command-list-reordering/);
  assert.match(commandPanelCss, /data-drop-target='true'/);
  assert.match(commandPanelCss, /\.command-list li \{[^}]*touch-action: none;/s);
  assert.match(commandPanelSource, /class="command-icon-button command-delete-button"[\s\S]*disabled=\{Boolean\(runningId \|\| structuralMutationPending \|\| activeCommandIds\.has\(entry\.id\)\)\}[\s\S]*onClick=\{\(event\) => void deleteEntry\(entry\.id, event\)\}/);
});

test('quick command create action uses copied add icon while retaining accessible name and behavior', () => {
  const addIconSource = readFileSync(commandPanelNewIconPath, 'utf8');

  assert.match(addIconSource, /<svg[^>]*viewBox="0 -960 960 960"[^>]*fill="#e3e3e3"/);
  assert.match(addIconSource, /<path d="M440-440H200v-80h240v-240h80v240h240v80H520v240h-80v-240Z"\/>/);
  assert.match(commandPanelSource, /const commandPanelNewIconUrl = new URL\('\.\.\/assets\/icons\/add_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24\.svg', import\.meta\.url\)\.href;/);
  assert.match(commandPanelSource, /<MeltActionButton class="command-text-button command-create-button" ariaLabel="Create command" onClick=\{startNewEntry\}><img class="command-new-icon" src=\{commandPanelNewIconUrl\} alt="" aria-hidden="true" draggable="false" \/><\/MeltActionButton>/);
  assert.doesNotMatch(commandPanelSource, /ariaLabel="Create command"[^>]*>New<\/MeltActionButton>/);
  assert.match(commandPanelCss, /\.command-create-button \{[^}]*align-items: center;[^}]*display: inline-flex;[^}]*justify-content: center;[^}]*min-height: 24px;[^}]*min-width: 24px;[^}]*padding: 0;/s);
  assert.match(commandPanelCss, /\.command-new-icon \{[^}]*display: block;[^}]*height: 16px;[^}]*width: 16px;/s);
});

test('quick command saved-command actions use copied icons while retaining names and handlers', () => {
  const deleteIconSource = readFileSync(commandPanelDeleteIconPath, 'utf8');
  const saveIconSource = readFileSync(commandPanelSaveIconPath, 'utf8');
  const cancelIconSource = readFileSync(commandPanelCancelIconPath, 'utf8');

  assert.match(deleteIconSource, /<svg[^>]*viewBox="0 -960 960 960"[^>]*fill="#e3e3e3"/);
  assert.match(deleteIconSource, /<path d="M280-120q-33 0-56\.5-23\.5T200-200v-520h-40v-80h200v-40h240v40h200v80h-40v520q0 33-23\.5 56\.5T680-120H280Z/);
  assert.match(saveIconSource, /<svg[^>]*viewBox="0 -960 960 960"[^>]*fill="#e3e3e3"/);
  assert.match(saveIconSource, /<path d="M840-680v480q0 33-23\.5 56\.5T760-120H200/);
  assert.match(cancelIconSource, /<svg[^>]*viewBox="0 -960 960 960"[^>]*fill="#e3e3e3"/);
  assert.match(cancelIconSource, /<path d="m376-320 104-104 104 104/);
  assert.match(commandPanelSource, /const commandPanelDeleteIconUrl = new URL\('\.\.\/assets\/icons\/delete_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24\.svg', import\.meta\.url\)\.href;/);
  assert.match(commandPanelSource, /const commandPanelSaveIconUrl = new URL\('\.\.\/assets\/icons\/save_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24\.svg', import\.meta\.url\)\.href;/);
  assert.match(commandPanelSource, /const commandPanelCancelIconUrl = new URL\('\.\.\/assets\/icons\/cancel_presentation_24dp_E3E3E3_FILL0_wght400_GRAD0_opsz24\.svg', import\.meta\.url\)\.href;/);
  assert.match(commandPanelSource, /class="command-icon-button command-delete-button"[\s\S]*ariaLabel=\{`Delete \$\{entry\.label\}`\}[\s\S]*onClick=\{\(event\) => void deleteEntry\(entry\.id, event\)\}[\s\S]*<img class="command-delete-icon" src=\{commandPanelDeleteIconUrl\} alt="" aria-hidden="true" draggable="false" \/>/);
  assert.match(commandPanelSource, /class="command-text-button command-editor-icon-button" ariaLabel="Save command"[\s\S]*onClick=\{\(\) => void saveEntry\(\)\}[\s\S]*<img class="command-save-icon" src=\{commandPanelSaveIconUrl\} alt="" aria-hidden="true" draggable="false" \/>/);
  assert.match(commandPanelSource, /class="command-text-button command-editor-icon-button" ariaLabel="Cancel command editing"[\s\S]*onClick=\{startNewEntry\}[\s\S]*<img class="command-cancel-icon" src=\{commandPanelCancelIconUrl\} alt="" aria-hidden="true" draggable="false" \/>/);
  assert.match(commandPanelCss, /\.command-icon-button \{[^}]*min-height: 24px;[^}]*min-width: 24px;/s);
  assert.match(commandPanelCss, /\.command-editor-icon-button \{[^}]*min-height: 24px;[^}]*min-width: 24px;/s);
  assert.match(commandPanelCss, /\.command-(?:delete|save|cancel)-icon \{[^}]*display: block;[^}]*height: 16px;[^}]*width: 16px;/s);
});

test('command panel Rust placement clamps inside monitor work area and shrinks only when needed', () => {
  assert.match(commandPanelRs, /places_command_panel_within_work_area_and_keeps_width_when_it_fits/);
  assert.match(commandPanelRs, /shrinks_command_panel_before_positioning_when_work_area_is_too_narrow/);
  assert.match(commandPanelRs, /clamps_saved_command_panel_size_to_monitor_work_area/);
  assert.match(commandPanelRs, /current_monitor\(\)[\s\S]*?work_area\(\)/);
  assert.match(commandPanelRs, /let target_size = PhysicalSize::new\(panel_width as u32, panel_height as u32\)/);
  assert.match(commandPanelRs, /set_size\(target_size\)/);
  assert.match(commandPanelRs, /emit_to\(TOP_BAR_LABEL, COMMAND_PANEL_CLOSED_EVENT/);
});

test('command panel close lifecycle avoids resize and minimize/maximize disappearance', () => {
  assert.match(mainSource, /COMMAND_PANEL_LABEL[\s\S]*Focused\(false\)/);
  assert.match(mainSource, /sleep\(Duration::from_millis\(150\)\)/);
  assert.match(mainSource, /is_focused\(\)/);
  assert.match(mainSource, /is_maximized\(\)/);
  assert.match(mainSource, /is_minimized\(\)/);
  assert.match(mainSource, /COMMAND_PANEL_LABEL[\s\S]*WindowEvent::Resized/);
  assert.doesNotMatch(mainSource, /COMMAND_PANEL_LABEL[\s\S]*WindowEvent::Maximized/);
  assert.doesNotMatch(mainSource, /COMMAND_PANEL_LABEL[\s\S]*WindowEvent::Minimized/);
});

test('command panel transcript host remains a labelled read-only focusable log with copy and context handlers', () => {
  assert.match(commandPanelSource, /<!-- svelte-ignore a11y-no-noninteractive-tabindex a11y-no-static-element-interactions(?: a11y-no-noninteractive-element-interactions)? -->/);
  assert.match(
    commandPanelSource,
    /<div class="command-transcript-shell"[^>]*role="region"[^>]*tabindex="0"[^>]*aria-label="Merged transcript"[^>]*on:keydown=\{handleTranscriptKeydown\}[^>]*on:contextmenu=\{handleTranscriptContextMenu\}/,
    'transcript shell should be a focusable labelled region and keep copy/context handlers'
  );
  assert.doesNotMatch(commandPanelSource, /on:contextmenu\|preventDefault\|stopPropagation/);
});

test('quick command transcripts tail output until the user scrolls away from bottom', () => {
  assert.match(commandPanelSource, /let transcriptTailAttached = new WeakMap<HTMLElement, boolean>\(\);/);
  assert.match(commandPanelSource, /let transcriptScrollIntent = new WeakSet<HTMLElement>\(\);/);
  assert.match(commandPanelSource, /let transcriptPointerScrollShell: HTMLElement \| null = null;/);
  assert.match(commandPanelSource, /function markTranscriptScrollIntent\(event: Event\)/);
  assert.match(commandPanelSource, /if \(!transcriptScrollIntent\.has\(shell\) && transcriptPointerScrollShell !== shell\) return;/);
  assert.match(commandPanelSource, /transcriptScrollIntent\.delete\(shell\);/);
  assert.match(commandPanelSource, /function isTranscriptAtBottom\(shell: HTMLElement\)/);
  assert.match(commandPanelSource, /shell\.scrollHeight - shell\.scrollTop - shell\.clientHeight <= 2/);
  assert.match(commandPanelSource, /function handleTranscriptScroll\(event: Event\)/);
  assert.match(commandPanelSource, /transcriptTailAttached\.set\(shell, isTranscriptAtBottom\(shell\)\)/);
  assert.match(commandPanelSource, /function scheduleTranscriptTail\(\)/);
  assert.match(commandPanelSource, /shell\.scrollTop = shell\.scrollHeight/);
  assert.match(commandPanelSource, /on:scroll\|capture=\{handleTranscriptScroll\}/);
  assert.match(commandPanelSource, /on:wheel\|capture=\{markTranscriptScrollIntent\}/);
  assert.match(commandPanelSource, /on:pointerdown\|capture=\{markTranscriptScrollIntent\}/);
  assert.match(commandPanelSource, /on:keydown\|capture=\{markTranscriptScrollIntent\}/);
  assert.match(commandPanelSource, /on:pointerup\|capture=\{endTranscriptPointerScroll\}/);
});

test('command panel keeps stopping run visible with disabled stop affordance', () => {
  assert.match(commandPanelSource, /latestRunControlKind\(|isRunStopping\(|isCommandStopping\(/);
  assert.match(commandPanelSource, /return run\.running && \(latestRunControlKind\(run\) === 'stopping' \|\| stoppingRunIds\.has\(run\.runId\)\)/);
  assert.match(commandPanelSource, /Stopping…|Stopping\.\.\./);
  assert.match(commandPanelSource, /disabled=\{[^}]*stoppingRunIds\.has|disabled=\{[^}]*isCommandStopping\(/);
  assert.match(commandPanelSource, /<details class="command-history-run" open=\{run\.running \|\| isRunExpanded\(run\)\}/);
});

test('starting a quick command closes prior expanded runs for only that command', () => {
  assert.match(commandPanelSource, /const priorRunIds = new Set\(allHistory\.filter\(\(run\) => run\.commandId === entry\.id\)\.map\(historyRunKey\)\)/);
  assert.match(commandPanelSource, /expandedRunIds = new Set\(\[\.\.\.expandedRunIds\]\.filter\(\(id\) => !priorRunIds\.has\(id\)\)\.concat\(runId\)\)/);
});

test('saved-command selection preserves Previous runs and refreshes the newly selected command history', () => {
  assert.match(commandPanelSource, /function selectCommand\(entry: QuickCommandEntry\) \{[\s\S]*const preservePreviousRuns = activeTab === 'previousRuns';[\s\S]*startEditEntry\(entry\);[\s\S]*if \(preservePreviousRuns\) \{[\s\S]*activeTab = 'previousRuns';[\s\S]*void refreshHistory\(\);[\s\S]*\}/);
  assert.match(commandPanelSource, /function startEditEntry\(entry: QuickCommandEntry\) \{[\s\S]*activeTab = 'configuration';/);
});

test('context Edit command always opens Configuration without changing ordinary row selection behavior', () => {
  assert.match(commandPanelSource, /function editContextEntry\(\) \{[\s\S]*if \(!contextEntry\) return;[\s\S]*startEditEntry\(contextEntry\);[\s\S]*contextEntry = null;[\s\S]*\}/);
  assert.doesNotMatch(commandPanelSource, /function editContextEntry\(\) \{[^}]*selectCommand\(contextEntry\)/);
});

test('context Previous runs selects its command and refreshes history exactly once', () => {
  const showHistorySource = commandPanelSource.slice(
    commandPanelSource.indexOf('function showHistory()'),
    commandPanelSource.indexOf('function editContextEntry()')
  );
  assert.match(showHistorySource, /if \(!contextEntry\) return;/);
  assert.match(showHistorySource, /startEditEntry\(contextEntry\);/);
  assert.match(showHistorySource, /activeTab = 'previousRuns';/);
  assert.match(showHistorySource, /contextEntry = null;/);
  assert.equal(showHistorySource.match(/refreshHistory\(\)/g)?.length, 1);
  assert.doesNotMatch(showHistorySource, /selectCommand\(/);
});

test('saved-command row left click selects except actions and suppressed post-drag click', () => {
  assert.match(commandPanelSource, /function handleCommandRowClick\(event: MouseEvent, entry: QuickCommandEntry\) \{[\s\S]*closest\('\.command-row-actions'\)[\s\S]*suppressCommandClickId === entry\.id[\s\S]*event\.preventDefault\(\);[\s\S]*event\.stopPropagation\(\);[\s\S]*suppressCommandClickId = null;[\s\S]*return;[\s\S]*selectCommand\(entry\);[\s\S]*\}/);
  assert.match(commandPanelSource, /<li[\s\S]*?on:click\|capture=\{\(event\) => handleCommandRowClick\(event, entry\)\}/);
});

test('command panel terminal events clear active quick command state immediately', () => {
  assert.match(commandPanelSource, /payload\.kind === 'stopped' \|\| payload\.kind === 'exit'/);
  assert.match(commandPanelSource, /activeRunIds = new Set\(\[\.\.\.activeRunIds\]\.filter\(\(runId\) => runId !== payload\.runId\)\)/);
  assert.match(commandPanelSource, /activeCommandIds = new Set\(\[\.\.\.activeCommandIds\]\.filter\(\(commandId\) => commandId !== payload\.commandId\)\)/);
  assert.match(commandPanelSource, /payload\.kind === 'stop-failed' \|\| payload\.kind === 'stopped' \|\| payload\.kind === 'exit'\) && stoppingId === payload\.commandId\) stoppingId = null/);
});

