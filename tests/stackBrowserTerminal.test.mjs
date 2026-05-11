import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

function readRepoFile(path) {
  const url = new URL(`../${path}`, import.meta.url);
  return existsSync(url) ? readFileSync(url, 'utf8') : '';
}

const commandsSource = readRepoFile('src/ipc/commands.ts');
const stackPopupApi = readRepoFile('src/lib/stackPopup.ts');
const stackPopupSurface = readRepoFile('src/components/StackPopupSurface.svelte');
const stackPopupCss = readRepoFile('src/components/StackPopupSurface.css');
const stackTerminalPane = readRepoFile('src/components/StackTerminalPane.svelte');
const stackTerminalPaneCss = readRepoFile('src/components/StackTerminalPane.css');
const terminalPanelSurface = readRepoFile('src/components/TerminalPanelSurface.svelte');
const terminalPanelCss = readRepoFile('src/components/TerminalPanelSurface.css');
const stackTerminalViewModel = readRepoFile('src/features/stack-browser/terminalViewModel.ts');
const settingsPanelSurface = readRepoFile('src/components/SettingsPanelSurface.svelte');
const settingsSource = readRepoFile('src/lib/settings.ts');
const rustStackPopup = readRepoFile('src-tauri/src/stack_popup.rs');
const rustTerminal = readRepoFile('src-tauri/src/stack_popup/terminal.rs');
const rustMain = readRepoFile('src-tauri/src/main.rs');
const rustContracts = readRepoFile('src-tauri/src/contracts.rs');
const stackPopupCapability = readRepoFile('src-tauri/capabilities/stack-popup.json');
const contractsSettingsTest = readRepoFile('tests/contractsSettings.test.mjs');
const masterSpec = readRepoFile('master_spec.md');
const changelog = readRepoFile('changelog.md');
const packageJson = readRepoFile('package.json');

test('stack terminal output chunks require sequenced dedupe identity', () => {
  assert.match(stackPopupApi, /export type StackTerminalOutputChunk = \{[\s\S]*sequence: number;[\s\S]*\};/);
  assert.doesNotMatch(stackPopupApi, /sequence\?: number/);
  assert.match(stackTerminalPane, /type StackTerminalOutputPayload = \{[\s\S]*sequence: number;[\s\S]*stream\?: 'stdout' \| 'stderr' \| 'system'[\s\S]*\}/);
});

test('stack terminal IPC command names are stable and backend-registered', () => {
  assert.match(commandsSource, /startStackTerminal:\s*'start_stack_terminal'/);
  assert.match(commandsSource, /readStackTerminal:\s*'read_stack_terminal'/);
  assert.match(commandsSource, /writeStackTerminal:\s*'write_stack_terminal'/);
  assert.match(commandsSource, /stopStackTerminal:\s*'stop_stack_terminal'/);
  assert.match(rustContracts, /START_STACK_TERMINAL:\s*&str\s*=\s*"start_stack_terminal"/);
  assert.match(rustContracts, /READ_STACK_TERMINAL:\s*&str\s*=\s*"read_stack_terminal"/);
  assert.match(rustContracts, /WRITE_STACK_TERMINAL:\s*&str\s*=\s*"write_stack_terminal"/);
  assert.match(rustContracts, /STOP_STACK_TERMINAL:\s*&str\s*=\s*"stop_stack_terminal"/);
  assert.match(rustStackPopup, /mod terminal;/);
  assert.match(rustStackPopup, /pub async fn start_stack_terminal\(/);
  assert.match(rustStackPopup, /pub fn read_stack_terminal\(/);
  assert.match(rustStackPopup, /pub async fn write_stack_terminal\(/);
  assert.match(rustStackPopup, /pub fn stop_stack_terminal\(/);
  assert.match(rustMain, /stack_popup::start_stack_terminal/);
  assert.match(rustMain, /stack_popup::read_stack_terminal/);
  assert.match(rustMain, /stack_popup::write_stack_terminal/);
  assert.match(rustMain, /stack_popup::stop_stack_terminal/);
  assert.match(rustTerminal, /MAX_STACK_TERMINAL_SESSIONS/);
  assert.match(rustTerminal, /validate_stack_terminal_session_id/);
  assert.match(rustTerminal, /cwd_after_terminal_input/);
});

test('phase 1 terminal startup has visible nonblank lifecycle state before first output', () => {
  const terminalPanelSurface = readRepoFile('src/components/TerminalPanelSurface.svelte');
  const terminalPanelCss = readRepoFile('src/components/TerminalPanelSurface.css');
  assert.match(terminalPanelSurface, /type TerminalLifecycleState\s*=/);
  assert.match(terminalPanelSurface, /'starting'/);
  assert.match(terminalPanelSurface, /'waiting'/);
  assert.match(terminalPanelSurface, /'failed'/);
  assert.match(terminalPanelSurface, /let outputReceived\s*=/);
  assert.match(terminalPanelSurface, /let status\s*=/);
  assert.match(terminalPanelSurface, /class="terminal-panel-status"/);
  assert.match(terminalPanelSurface, /session\?\.cwd/);
  assert.match(terminalPanelCss, /\.terminal-panel-status/);
  assert.doesNotMatch(
    terminalPanelSurface,
    /session\.output/,
    'startStackTerminal snapshot has no output field; first bytes must come from event/poll contract'
  );
});

test('terminal control-only output does not hide startup overlay as first visible output', () => {
  assert.match(stackTerminalPane, /function terminalOutputHasVisibleText\(value: string\)/);
  assert.match(stackTerminalPane, /stripTerminalAnsiControls/);
  assert.match(stackTerminalPane, /if \(nextOutputChunk && terminalOutputHasVisibleText\(nextOutputChunk\)\)/);
  assert.doesNotMatch(
    stackTerminalPane,
    /if \(nextOutputChunk\) \{\s*firstOutputReceived = true/,
    'ANSI clear/cursor-only chunks must not hide startup overlay'
  );
});

test('terminal poll fallback keeps writing chunks after first output with sequence dedupe', () => {
  assert.match(stackTerminalPane, /if \(result\.chunks\?\.length\) \{/);
  assert.doesNotMatch(stackTerminalPane, /if \(result\.chunks\?\.length && !firstOutputReceived\)/);
  assert.match(stackTerminalPane, /for \(const chunk of result\.chunks\) \{\s*enqueueOutputChunk\(chunk\);/);
  assert.match(stackTerminalPane, /const sequenceKey = `\$\{chunk\.sessionId\}:\$\{chunk\.stream \?\? 'stdout'\}:\$\{chunk\.sequence\}`/);
  assert.doesNotMatch(stackTerminalPane, /writeOutput\(result\.output\)/);
  assert.match(stackTerminalPane, /renderedSequences\.has\(sequenceKey\)/);
  assert.match(stackTerminalPane, /if \(result\.sessionId !== session\?\.sessionId\) \{\s*return;/);
});

test('phase 1 terminal output uses stack-terminal:output listener with persistent-surface cleanup', () => {
  assert.match(stackTerminalPane, /listen\(/);
  assert.match(stackTerminalPane, /stack-terminal:output|STACK_TERMINAL_OUTPUT/);
  assert.match(stackTerminalPane, /let unlisteners:\s*Array<\(\) => void>/);
  assert.match(stackTerminalPane, /let listenersDisposed\s*=/);
  assert.match(stackTerminalPane, /listenersDisposed = true/);
  assert.match(stackTerminalPane, /unlisten\(\)/);
  assert.match(stackTerminalPane, /if \(listenersDisposed\)/);
  assert.match(stackTerminalPane, /event\.payload\.sessionId !== session\?\.sessionId/);
  assert.match(stackTerminalPane, /return;/);
  assert.match(stackTerminalPane, /watchdog/i);
  assert.doesNotMatch(
    stackTerminalPane,
    /pollTimer = window\.setInterval\([\s\S]{0,160}700/,
    'polling should be watchdog/fallback cadence, not normal high-frequency output path'
  );
});

test('phase 1 terminal resize command contract is exposed across IPC, capabilities, and fitted xterm geometry', () => {
  assert.match(commandsSource, /resizeStackTerminal:\s*'resize_stack_terminal'/);
  assert.match(stackPopupApi, /export function resizeStackTerminal\(\s*sessionId: string,\s*cols: number,\s*rows: number,\s*pixelWidth\?: number,\s*pixelHeight\?: number\s*\)/);
  assert.match(stackPopupApi, /IPC_COMMANDS\.resizeStackTerminal/);
  assert.match(rustContracts, /RESIZE_STACK_TERMINAL:\s*&str\s*=\s*"resize_stack_terminal"/);
  assert.match(rustStackPopup, /pub (async )?fn resize_stack_terminal\(/);
  assert.match(rustMain, /stack_popup::resize_stack_terminal/);
  assert.match(stackPopupCapability, /"core:default"/);
  assert.match(stackPopupCapability, /"core:window:default"/);
  assert.match(stackTerminalPane, /fitAddon\.fit\(\)/);
  assert.match(stackTerminalPane, /terminal\.cols/);
  assert.match(stackTerminalPane, /terminal\.rows/);
  assert.match(stackTerminalPane, /const sessionId = session\.sessionId/);
  assert.match(stackTerminalPane, /const cols = terminal\.cols/);
  assert.match(stackTerminalPane, /const rows = terminal\.rows/);
  assert.match(stackTerminalPane, /resizeStackTerminal\(sessionId, cols, rows/);
});

test('terminal TUI input stays low-latency and transparent', () => {
  assert.match(stackTerminalPane, /let writeQueue: Promise<void> = Promise\.resolve\(\)/);
  assert.match(stackTerminalPane, /function enqueueTerminalWrite/);
  assert.match(stackTerminalPane, /enqueueTerminalWrite\(\(\) => writeStackTerminal\(sessionId, data\)\)/);
  assert.doesNotMatch(
    stackTerminalPane,
    /async function writeTerminalData[\s\S]{0,360}pollOutput\(\)/,
    'normal keystrokes must not wait for a read/poll roundtrip'
  );
  assert.match(stackTerminalPane, /convertEol:\s*false/);
  assert.match(stackTerminalPane, /windowsPty:\s*\{\s*backend:\s*'conpty'\s*\}/);
  assert.doesNotMatch(
    stackTerminalPane,
    /event\.key === 'Escape'[\s\S]{0,180}onCloseRequest\(\)/,
    'Escape must pass through to full-screen TUIs unless Stack terminal search is open'
  );
});

test('phase 1 terminal poll and resize paths serialize frontend reads for one active session', () => {
  assert.match(stackTerminalPane, /let pollInFlight\s*=/);
  assert.match(stackTerminalPane, /let pollQueued\s*=/);
  assert.match(stackTerminalPane, /async function pollOutput/);
  assert.match(stackTerminalPane, /if \(pollInFlight\)/);
  assert.match(stackTerminalPane, /pollQueued = true/);
  assert.match(stackTerminalPane, /pollInFlight = true/);
  assert.match(stackTerminalPane, /pollInFlight = false/);
  assert.match(stackTerminalPane, /if \(pollQueued\)/);
  assert.match(stackTerminalPane, /void pollOutput\(\)/);
  assert.match(stackTerminalPane, /let operationQueue: Promise<void> = Promise\.resolve\(\)/);
  assert.match(stackTerminalPane, /function enqueueOperation/);
  assert.match(stackTerminalPane, /enqueueOperation\(\(\) => readStackTerminal\(sessionId\)\)/);
  assert.match(stackTerminalPane, /enqueueOperation\(\(\) => resizeStackTerminal/);
});

test('phase 3 terminal output is push-first and batched before xterm writes', () => {
  assert.match(stackTerminalPane, /let pendingChunks:\s*Array<StackTerminalOutputPayload \| StackTerminalOutputChunk>/);
  assert.match(stackTerminalPane, /let flushFrame:\s*number \| null = null/);
  assert.match(stackTerminalPane, /function enqueueOutputChunk/);
  assert.match(stackTerminalPane, /window\.requestAnimationFrame\(\(\) => \{\s*flushFrame = null;\s*flushOutputChunks\(\);/);
  assert.match(stackTerminalPane, /\.filter\(\(chunk\) => chunk\.sessionId === activeSessionId\)/);
  assert.match(stackTerminalPane, /\.sort\(\(left, right\) => \(left\.sequence \?\? 0\) - \(right\.sequence \?\? 0\)\)/);
  assert.match(stackTerminalPane, /enqueueOutputChunk\(event\.payload\)/);
  assert.match(stackTerminalPane, /enqueueOutputChunk\(chunk\)/);
  assert.match(stackTerminalPane, /const STACK_TERMINAL_REPLAY_LIMIT = 256 \* 1024/);
  assert.match(stackTerminalPane, /compacted\.slice\(compacted\.length - STACK_TERMINAL_REPLAY_LIMIT\)/);
  assert.doesNotMatch(
    stackTerminalPane,
    /listen<StackTerminalOutputPayload>\('stack-terminal:output'[\s\S]{0,220}writeChunk\(event\.payload\)/,
    'output listener should enqueue chunks instead of writing directly'
  );
});

test('phase 4 extracts Stack terminal pane and modern xterm addons from parent surface', () => {
  assert.match(stackPopupSurface, /import StackTerminalPane from '\.\/StackTerminalPane\.svelte'/);
  assert.match(stackPopupSurface, /<StackTerminalPane/);
  assert.doesNotMatch(stackPopupSurface, /from '@xterm\/xterm'/);
  assert.doesNotMatch(stackPopupSurface, /from '@xterm\/addon-fit'/);
  assert.match(stackTerminalPane, /import \{ WebglAddon \} from '@xterm\/addon-webgl'/);
  assert.match(stackTerminalPane, /import \{ WebLinksAddon \} from '@xterm\/addon-web-links'/);
  assert.match(stackTerminalPane, /import \{ SearchAddon \} from '@xterm\/addon-search'/);
  assert.match(stackTerminalPane, /import \{ SerializeAddon \} from '@xterm\/addon-serialize'/);
  assert.match(stackTerminalPane, /import \{ Unicode11Addon \} from '@xterm\/addon-unicode11'/);
  assert.match(stackTerminalPane, /addon\.onContextLoss\(\(\) => \{/);
  assert.match(stackTerminalPane, /renderer = 'fallback'/);
  assert.match(stackTerminalPane, /nextTerminal\.registerLinkProvider/);
  assert.match(stackTerminalPane, /searchAddon\?\.findNext/);
  assert.match(stackTerminalPane, /nextTerminal\.unicode\.activeVersion = '11'/);
  assert.match(stackTerminalPane, /STACK_TERMINAL_FONT_FAMILY/);
  assert.match(stackTerminalPane, /fontFamily: STACK_TERMINAL_FONT_FAMILY/);
  assert.match(stackTerminalPane, /letterSpacing: 0/);
  assert.match(stackTerminalPane, /lineHeight: 1\.25/);
  assert.match(stackTerminalPaneCss, /Cascadia Mono/);
});

test('stack terminal text metrics stay monospace and PTY redraw sequences stay xterm-owned', () => {
  assert.doesNotMatch(stackTerminalPane, /function anchorCommandLineToLastRow\(\)/);
  assert.doesNotMatch(stackTerminalPane, /terminal\.write\(`\\x1b\[\$\{terminal\.rows\};1H`\)/);
  assert.doesNotMatch(stackTerminalPane, /terminalOutputHasClear/);
  assert.doesNotMatch(stackTerminalPane, /terminal\?\.reset\(\);[\s\S]{0,180}terminal\?\.write\(output\)/);
  assert.doesNotMatch(
    stackTerminalPane,
    /if \(terminalOutputHasClear\(nextOutputChunk\)\)[\s\S]{0,260}return;/,
    'clear/readline/autocomplete redraw bytes must pass through xterm instead of being intercepted'
  );
  assert.match(stackTerminalPane, /terminal\?\.write\(nextOutputChunk\);/);
  assert.doesNotMatch(
    stackTerminalPane,
    /function writeOutput[\s\S]{0,520}terminal\?\.scrollToBottom\(\)/,
    'full-screen TUI redraws must not be followed by forced scroll pinning'
  );
  assert.match(stackTerminalPaneCss, /font-feature-settings: "liga" 0, "calt" 0, "tnum" 1;/);
  assert.match(stackTerminalPaneCss, /font-variant-ligatures: none;/);
  assert.match(stackTerminalPaneCss, /letter-spacing: 0;/);
  assert.match(stackTerminalPaneCss, /\.stack-terminal-output :global\(\.xterm-rows\)/);
  assert.doesNotMatch(stackTerminalPaneCss, /\.stack-terminal-output :global\(\.xterm-screen\)[\s\S]{0,80}height: 100% !important;/);
});

test('phase 4 terminal link detector recognizes safe URL, path, localhost, and git hash targets', () => {
  assert.match(stackTerminalViewModel, /export function detectStackTerminalLinks/);
  assert.match(stackTerminalViewModel, /export function isSafeStackTerminalOpenTarget/);
  const sample = 'open http://localhost:5173 and C:\\\\dev\\\\jasonshell\\\\src\\\\main.ts:12 plus abc1234';
  assert.match(sample, /localhost/);
  assert.match(stackTerminalViewModel, /LOCALHOST_PATTERN/);
  assert.match(stackTerminalViewModel, /WINDOWS_PATH_PATTERN/);
  assert.match(stackTerminalViewModel, /GIT_HASH_PATTERN/);
  assert.match(stackTerminalViewModel, /!\s*\/\[<>\|"\?\*\]\//);
});

test('phase 3 backend keeps live sessions registered and records PTY size metadata', () => {
  assert.match(stackPopupApi, /cols\?: number/);
  assert.match(stackPopupApi, /rows\?: number/);
  assert.match(rustTerminal, /pub cols: u16/);
  assert.match(rustTerminal, /pub rows: u16/);
  assert.match(rustTerminal, /size: StackTerminalSize/);
  assert.match(rustTerminal, /session\.size = StackTerminalSize \{/);
  assert.match(rustTerminal, /mpsc::sync_channel\(1024\)/);
  assert.match(rustTerminal, /decode_terminal_output_chunk\(&mut pending_utf8, &buffer\[..count\]\)/);
  assert.doesNotMatch(
    rustTerminal,
    /let mut session = take_terminal_session\(state, &request\.session_id\)\?/,
    'write path must not remove live session from registry'
  );
  assert.doesNotMatch(
    rustTerminal,
    /let mut session = take_terminal_session\(state, &session_id\)\?/,
    'poll path must not remove live session from registry'
  );
});

test('phase 1 terminal PowerShell launch plan uses trusted path explicitly and has fallback coverage', () => {
  assert.match(rustTerminal, /fn powershell_cmd_launch_line\(powershell: PathBuf\) -> String/);
  assert.match(rustTerminal, /powershell\.to_string_lossy\(\)/);
  assert.doesNotMatch(rustTerminal, /fn powershell_cmd_launch_line\(_powershell: PathBuf\)/);
  assert.doesNotMatch(rustTerminal, /"pwsh\.exe \{\}"/);
  assert.match(rustTerminal, /powershell_encoded_command\(&powershell_startup_script\(\)\)/);
  assert.match(rustTerminal, /"-EncodedCommand"\.to_string\(\)/);
  assert.match(rustTerminal, /"-NoProfile"\.to_string\(\)/);
  assert.match(rustTerminal, /trusted_powershell_candidates/);
  assert.match(rustTerminal, /WindowsPowerShell/);
  assert.match(rustTerminal, /falls_back_to_windows_powershell|fallback/i);
});

test('stack popup API exposes typed terminal profiles and command wrappers', () => {
  assert.match(stackPopupApi, /export type StackTerminalProfile = 'windowsTerminal' \| 'gitBash' \| 'powershell';/);
  assert.match(stackPopupApi, /export const STACK_TERMINAL_PROFILE_OPTIONS/);
  assert.match(stackPopupApi, /value: 'windowsTerminal', label: 'Windows Terminal'/);
  assert.match(stackPopupApi, /value: 'gitBash', label: 'Git Bash'/);
  assert.match(stackPopupApi, /value: 'powershell', label: 'PowerShell'/);
  assert.match(stackPopupApi, /export function normalizeStackTerminalProfile\(value: unknown\): StackTerminalProfile/);
  assert.match(stackPopupApi, /export type StackTerminalSession = \{/);
  assert.match(stackPopupApi, /sessionId: string;/);
  assert.match(stackPopupApi, /cwd: string;/);
  assert.match(stackPopupApi, /profile: StackTerminalProfile;/);
  assert.match(stackPopupApi, /export type StackTerminalReadResult = \{/);
  assert.match(stackPopupApi, /exited: boolean;/);
  assert.match(stackPopupApi, /export function startStackTerminal\(folderPath: string, profile: StackTerminalProfile\): Promise<StackTerminalSession>/);
  assert.match(stackPopupApi, /request: \{ folderPath, profile \}/);
  assert.match(stackPopupApi, /export function readStackTerminal\(sessionId: string\): Promise<StackTerminalReadResult>/);
  assert.match(stackPopupApi, /export function writeStackTerminal\(sessionId: string, input: string\): Promise<void>/);
  assert.match(stackPopupApi, /export function stopStackTerminal\(sessionId: string\): Promise<void>/);
});

test('shell settings include stack browser terminal profile defaults', () => {
  assert.match(settingsSource, /export interface StackBrowserSettings \{/);
  assert.match(settingsSource, /terminalProfile: StackTerminalProfile;/);
  assert.match(settingsSource, /stackBrowser: StackBrowserSettings;/);
  assert.match(settingsSource, /export function defaultStackBrowserSettings\(\): StackBrowserSettings/);
  assert.match(settingsSource, /terminalProfile: 'windowsTerminal'/);
  assert.match(contractsSettingsTest, /stackBrowser:\s*\{\s*terminalProfile: 'windowsTerminal'\s*\}/);
});

test('stack browser no longer exposes embedded CLI toggle controls', () => {
  assert.doesNotMatch(stackPopupSurface, /stack-view-toggle/);
  assert.doesNotMatch(stackPopupSurface, /CLI<\/MeltActionButton>/);
  assert.doesNotMatch(stackPopupSurface, /Stack Browser embedded terminal lets you|Use this terminal to/);
});

test('persistent terminal starts with app, accepts input, polls output, and stays pinned to bottom', () => {
  const terminalPanelSurface = readRepoFile('src/components/TerminalPanelSurface.svelte');
  assert.match(packageJson, /"@xterm\/xterm"/);
  assert.match(packageJson, /"@xterm\/addon-fit"/);
  assert.match(terminalPanelSurface, /import '@xterm\/xterm\/css\/xterm\.css';/);
  assert.match(terminalPanelSurface, /import \{ Terminal \} from '@xterm\/xterm';/);
  assert.match(terminalPanelSurface, /import \{ FitAddon \} from '@xterm\/addon-fit';/);
  assert.match(terminalPanelSurface, /startPersistentTerminal\(\)/);
  assert.match(terminalPanelSurface, /void startTerminal\(\)/);
  assert.match(terminalPanelSurface, /await startTerminal\(\)/);
  assert.match(terminalPanelSurface, /readStackTerminal\(sessionId\)/);
  assert.match(terminalPanelSurface, /new Terminal\(\{/);
  assert.match(terminalPanelSurface, /fontFamily: TERMINAL_PANEL_FONT_FAMILY/);
  assert.match(terminalPanelSurface, /fontSize: 13/);
  assert.match(terminalPanelSurface, /lineHeight: 1\.25/);
  assert.match(terminalPanelSurface, /scrollback: 8000/);
  assert.match(terminalPanelSurface, /letterSpacing: 0/);
  assert.match(terminalPanelSurface, /fitAddon = new FitAddon\(\);/);
  assert.match(terminalPanelSurface, /terminal\.loadAddon\(fitAddon\)/);
  assert.match(terminalPanelSurface, /terminal\.onData\(\(data\) => \{/);
  assert.match(terminalPanelSurface, /terminal\.attachCustomKeyEventHandler/);
  assert.match(terminalPanelSurface, /event\.ctrlKey && event\.key\.toLowerCase\(\) === 'c' && terminal\?\.hasSelection\(\)/);
  assert.match(terminalPanelSurface, /navigator\.clipboard\?\.writeText\(selection\)/);
  assert.match(terminalPanelSurface, /event\.ctrlKey && event\.key\.toLowerCase\(\) === 'v'/);
  assert.match(terminalPanelSurface, /navigator\.clipboard\?\.readText\(\)/);
  assert.match(terminalPanelSurface, /void writeTerminalData\(data\);/);
  assert.match(terminalPanelSurface, /async function writeTerminalData\(data: string\)/);
  assert.match(terminalPanelSurface, /writeStackTerminal\(sessionId, data\)/);
  assert.match(terminalPanelSurface, /pollTimer = window\.setInterval/);
  assert.match(terminalPanelSurface, /stopStackTerminal\(oldSession\)/);
  assert.match(terminalPanelSurface, /use:bindPaneHost=\{pane\}/);
  assert.doesNotMatch(terminalPanelSurface, /writeTerminalOutput\(result\.output\)/);
  assert.doesNotMatch(terminalPanelSurface, /function anchorCommandLineToLastRow\(\)/);
  assert.doesNotMatch(terminalPanelSurface, /terminal\.write\(`\\x1b\[\$\{terminal\.rows\};1H`\)/);
  assert.doesNotMatch(terminalPanelSurface, /terminalOutputHasClear/);
  assert.match(terminalPanelSurface, /function writeTerminalOutput\(output: string\)/);
  assert.match(terminalPanelSurface, /terminalOutputHasVisibleText\(output\)/);
  assert.match(terminalPanelSurface, /terminal\?\.write\(output\)/);
  assert.match(terminalPanelSurface, /function focusTerminal\(\)/);
  assert.match(terminalPanelSurface, /terminal\?\.focus\(\)/);
  assert.match(terminalPanelSurface, /class="terminal-panel-output"/);
  assert.doesNotMatch(stackPopupSurface, /<input[\s\S]*aria-label="Terminal command"/);
  assert.doesNotMatch(stackPopupSurface, /stackTerminalPrompt/);
  assert.doesNotMatch(stackPopupSurface, /PS \$\{cwd\}>/);
  assert.doesNotMatch(stackPopupSurface, /<span>\{stackTerminalPrompt\(\)\}<\/span>/);
  assert.doesNotMatch(stackPopupSurface, /stackTerminalInputForKeydown/);
  assert.doesNotMatch(stackPopupSurface, /stackTerminalInputDraft/);
  assert.match(terminalPanelSurface, /terminal && terminal\.element && host\.contains\(terminal\.element\)/);
});

test('stack terminal supports xterm selection and copy', () => {
  const terminalPanelSurface = readRepoFile('src/components/TerminalPanelSurface.svelte');
  const terminalPanelCss = readRepoFile('src/components/TerminalPanelSurface.css');
  assert.match(terminalPanelSurface, /terminal\.onData/);
  assert.match(stackTerminalPane, /nextTerminal\.attachCustomKeyEventHandler/);
  assert.match(stackTerminalPane, /event\.ctrlKey && event\.key\.toLowerCase\(\) === 'c' && nextTerminal\.hasSelection\(\)/);
  assert.match(stackTerminalPane, /navigator\.clipboard\?\.writeText\(selection\)/);
  assert.match(stackTerminalPane, /event\.ctrlKey && event\.key\.toLowerCase\(\) === 'v'/);
  assert.match(stackTerminalPane, /navigator\.clipboard\?\.readText\(\)/);
  assert.match(stackTerminalPane, /on:contextmenu=\{openTerminalContextMenu\}/);
  assert.match(stackTerminalPane, /class="stack-terminal-context-menu"/);
  assert.match(stackTerminalPane, /copySelectionFromContextMenu/);
  assert.match(stackTerminalPane, /pasteClipboardFromContextMenu/);
  assert.match(terminalPanelCss, /user-select: text;/);
  assert.match(terminalPanelSurface, /on:contextmenu=\{\(event\) => \{ activatePane\(pane\.paneId\); openTerminalContextMenu\(event\); \}\}/);
  assert.match(terminalPanelSurface, /class="terminal-panel-context-menu"/);
  assert.match(terminalPanelSurface, /copySelectionFromContextMenu/);
  assert.match(terminalPanelSurface, /pasteClipboardFromContextMenu/);
  assert.match(terminalPanelCss, /\.terminal-panel-context-menu/);
  assert.match(terminalPanelCss, /\.terminal-panel-output :global\(\.xterm\)/);
  assert.match(terminalPanelCss, /\.terminal-panel-output :global\(\.xterm-helper-textarea\)/);
  assert.match(terminalPanelCss, /opacity: 0 !important;/);
  assert.match(terminalPanelCss, /caret-color: transparent !important;/);
  assert.match(terminalPanelCss, /height: 1px !important;/);
});

test('terminal removes xterm assistive mirrors from visible layout', () => {
  assert.match(terminalPanelSurface, /screenReaderMode:\s*false/);
  assert.match(terminalPanelSurface, /windowsPty:\s*\{\s*backend:\s*'conpty'\s*\}/);
  assert.match(stackTerminalPane, /screenReaderMode:\s*false/);
  assert.match(stackTerminalPane, /windowsPty:\s*\{\s*backend:\s*'conpty'\s*\}/);
  assert.match(terminalPanelCss, /\.terminal-panel-output :global\(\.xterm-accessibility\)/);
  assert.match(terminalPanelCss, /\.terminal-panel-output :global\(\.xterm-accessibility-tree\)/);
  assert.match(terminalPanelCss, /\.terminal-panel-output :global\(\.live-region\)/);
  assert.match(terminalPanelCss, /display: none !important;/);
  assert.match(stackTerminalPaneCss, /\.stack-terminal-output :global\(\.xterm-accessibility\)/);
  assert.match(stackTerminalPaneCss, /\.stack-terminal-output :global\(\.xterm-accessibility-tree\)/);
  assert.match(stackTerminalPaneCss, /\.stack-terminal-output :global\(\.live-region\)/);
});

test('terminal cwd changes update Stack Browser path and breadcrumbs immediately', () => {
  assert.match(stackTerminalPane, /export async function syncFolderToTerminalCwd\(\)/);
  assert.match(stackTerminalPane, /async function applyCwd\(nextCwd: string\)/);
  assert.match(stackTerminalPane, /await applyCwd\(result\.cwd \|\| cwd\)/);
  assert.match(stackTerminalPane, /await onCwdChange\(normalized\)/);
  assert.match(stackPopupSurface, /async function handleStackTerminalCwdChange\(cwd: string\)/);
  assert.match(stackPopupSurface, /stackBrowserViewMode === 'terminal'[\s\S]{0,220}await openFolder\(cwd, \{ warmTerminal: false \}\);/);
  assert.match(stackPopupSurface, /await stackTerminalPane\?\.syncFolderToTerminalCwd\(\);[\s\S]*stackBrowserViewMode = 'files';/);
});

test('stack terminal PowerShell profile loads normal shell affordances and normalizes extended cwd paths', () => {
  assert.match(rustTerminal, /"-ExecutionPolicy"\.to_string\(\),\s*"Bypass"\.to_string\(\)/);
  assert.match(rustTerminal, /"-NoExit"\.to_string\(\)/);
  assert.match(rustTerminal, /"-EncodedCommand"\.to_string\(\)/);
  assert.match(rustTerminal, /fn powershell_encoded_command\(script: &str\) -> String/);
  assert.match(rustTerminal, /powershell_startup_script\(\)/);
  assert.match(rustTerminal, /powershell_augmented_path\(\)/);
  assert.match(rustTerminal, /command\.env\("PATH", path\)/);
  assert.match(rustTerminal, /powershell\.to_string_lossy\(\)/);
  assert.ok(rustTerminal.includes('InlinePrediction = \\"`e[38;5;240m\\"'));
  assert.ok(rustTerminal.includes('ListPrediction = \\"`e[38;5;244m\\"'));
  assert.match(rustTerminal, /Set-PSReadLineKeyHandler -Key RightArrow -Function AcceptSuggestion/);
  assert.match(rustTerminal, /Set-PSReadLineKeyHandler -Key Tab -Function TabCompleteNext/);
  assert.match(rustTerminal, /Set-PSReadLineKeyHandler -Key Shift\+Tab -Function TabCompletePrevious/);
  assert.match(rustTerminal, /\$ErrorActionPreference = 'Continue'/);
  assert.doesNotMatch(rustTerminal, /\$ErrorActionPreference = 'SilentlyContinue'/);
  assert.match(rustTerminal, /Set-Alias -Name ls -Value Get-ChildItem -Force -ErrorAction SilentlyContinue/);
  assert.match(rustTerminal, /Set-Alias -Name clear -Value Clear-Host -Force -ErrorAction SilentlyContinue/);
  assert.match(rustTerminal, /function which \{ Get-Command @args \}/);
  assert.doesNotMatch(rustTerminal, /write_all\(startup_script\.as_bytes\(\)\)/);
  assert.doesNotMatch(rustTerminal, /"Clear-Host",/);
  assert.match(rustTerminal, /"-NoProfile"\.to_string\(\)/);
  assert.match(rustTerminal, /use portable_pty::\{native_pty_system, Child as PtyChild, CommandBuilder, MasterPty, PtySize\}/);
  assert.match(rustTerminal, /native_pty_system\(\)/);
  assert.match(rustTerminal, /\.openpty\(PtySize/);
  assert.match(rustTerminal, /apply_terminal_capability_environment\(&mut command\)/);
  assert.match(rustTerminal, /command\.env\("TERM", "xterm-256color"\)/);
  assert.match(rustTerminal, /command\.env\("COLORTERM", "truecolor"\)/);
  assert.match(rustTerminal, /command\.env\("TERM_PROGRAM", "JasonShell"\)/);
  assert.match(rustTerminal, /\.slave\.spawn_command\(command\)/);
  assert.match(rustTerminal, /master: Option<Arc<Mutex<Box<dyn MasterPty \+ Send>>>>/);
  assert.match(rustTerminal, /master: Some\(Arc::new\(Mutex::new\(pty_pair\.master\)\)\)/);
  assert.doesNotMatch(rustTerminal, /stdin\(Stdio::piped\(\)\)/);
  assert.match(rustTerminal, /fn display_stack_terminal_path\(path: &Path\) -> String/);
  assert.ok(rustTerminal.includes('strip_prefix("\\\\\\\\?\\\\")'));
  assert.ok(rustTerminal.includes('strip_prefix("\\\\\\\\?\\\\UNC\\\\")'));
  assert.match(rustTerminal, /format!\("\\\\\\\\\{rest\}"\)/);
  assert.match(rustTerminal, /stack_terminal_cwd_string\(cwd: &Path\)[\s\S]*display_stack_terminal_path\(cwd\)/);
});

test('stack terminal profile uses JSON shell settings from settings panel, not Stack Browser toolbar', () => {
  assert.match(stackPopupSurface, /loadShellSettings/);
  assert.match(stackPopupSurface, /normalizeStackTerminalProfile\(settings\.stackBrowser\?\.terminalProfile\)/);
  assert.doesNotMatch(stackPopupSurface, /saveShellSettings/);
  assert.doesNotMatch(stackPopupSurface, /class="stack-terminal-profile"/);
  assert.doesNotMatch(stackPopupSurface, /aria-label="Terminal profile"/);
  assert.match(settingsPanelSurface, /STACK_TERMINAL_PROFILE_OPTIONS/);
  assert.match(settingsPanelSurface, /loadShellSettings/);
  assert.match(settingsPanelSurface, /saveShellSettings/);
  assert.match(settingsPanelSurface, /normalizeStackTerminalProfile\(settings\.stackBrowser\?\.terminalProfile\)/);
  assert.match(settingsPanelSurface, /label="Stack Browser terminal"/);
  assert.match(settingsPanelSurface, /terminalProfile: selectedStackTerminalProfile/);
});

test('stack terminal layout is flat, dense, and isolated from file grid interactions', () => {
  assert.match(stackPopupCss, /\.stack-popup\.terminal-mode/);
  assert.match(stackPopupCss, /\.stack-view-toggle/);
  assert.match(stackPopupCss, /\.stack-terminal/);
  assert.match(stackPopupCss, /\.stack-terminal-output/);
  assert.doesNotMatch(stackPopupCss, /\.stack-terminal-command/);
  assert.doesNotMatch(stackPopupCss, /\.stack-terminal-profile/);
  assert.match(stackPopupCss, /font-family: var\(--js-font-mono/);
  assert.match(stackPopupCss, /grid-template-rows: minmax\(0, 1fr\);/);
  assert.match(stackPopupCss, /\.stack-terminal \{[\s\S]*grid-row: 3 \/ -1;[\s\S]*grid-template-rows: minmax\(0, 1fr\);/);
  assert.match(stackPopupCss, /\.stack-terminal \{[\s\S]*border: none;[\s\S]*border-radius: 0;/);
  assert.match(stackPopupCss, /\.stack-terminal-output \{[\s\S]*background: #06080b;[\s\S]*padding: 0;/);
  assert.match(stackPopupCss, /\.stack-terminal-output :global\(\.xterm-helper-textarea\) \{[\s\S]*caret-color: transparent !important;[\s\S]*left: -10000px !important;[\s\S]*opacity: 0 !important;/);
  assert.doesNotMatch(stackPopupCss, /\.stack-terminal-command span/);
  assert.doesNotMatch(stackPopupCss, /\.stack-terminal[\s\S]{0,220}box-shadow: var\(--js-shadow-panel\)/);
});

test('durable docs mention persistent terminal panel behavior and validation', () => {
  assert.match(masterSpec, /Persistent terminal panel/);
  assert.match(masterSpec, /`terminal-panel`/);
  assert.match(masterSpec, /`start_persistent_terminal`/);
  assert.match(masterSpec, /`stackBrowser\.terminalProfile`/);
  assert.match(masterSpec, /xterm\.js|xterm/);
  assert.match(masterSpec, /tests\/stackBrowserTerminal\.test\.mjs/);
  assert.match(masterSpec, /tests\/persistentTerminalPanel\.test\.mjs/);
  assert.match(changelog, /Persistent terminal panel/);
  assert.match(changelog, /xterm\.js|xterm/);
  assert.match(changelog, /tests\\persistentTerminalPanel\.test\.mjs|tests\/persistentTerminalPanel\.test\.mjs/);
});
