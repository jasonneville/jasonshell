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
  assert.match(stackPopupSurface, /function stackTerminalOutputHasVisibleText\(output: string\)/);
  assert.match(stackPopupSurface, /stripStackTerminalAnsiControls/);
  assert.match(stackPopupSurface, /if \(output && stackTerminalOutputHasVisibleText\(output\)\)/);
  assert.doesNotMatch(
    stackPopupSurface,
    /if \(output\) \{\s*stackTerminalFirstOutputReceived = true/,
    'ANSI clear/cursor-only chunks must not hide startup overlay'
  );
});

test('terminal poll fallback keeps writing chunks after first output with sequence dedupe', () => {
  assert.match(stackPopupSurface, /if \(result\.chunks\?\.length\) \{/);
  assert.doesNotMatch(stackPopupSurface, /if \(result\.chunks\?\.length && !stackTerminalFirstOutputReceived\)/);
  assert.match(stackPopupSurface, /for \(const chunk of result\.chunks\) \{\s*writeStackTerminalChunk\(chunk\);/);
  assert.match(stackPopupSurface, /const sequenceKey = `\$\{chunk\.sessionId\}:\$\{chunk\.sequence\}`/);
  assert.match(stackPopupSurface, /stackTerminalRenderedSequences\.has\(sequenceKey\)/);
  assert.match(stackPopupSurface, /if \(result\.sessionId !== stackTerminalSession\?\.sessionId\) \{\s*return;/);
});

test('phase 1 terminal output uses stack-terminal:output listener with persistent-surface cleanup', () => {
  assert.match(stackPopupSurface, /listen\(/);
  assert.match(stackPopupSurface, /stack-terminal:output|STACK_TERMINAL_OUTPUT/);
  assert.match(stackPopupSurface, /let stackTerminalUnlisteners:\s*Array<\(\) => void>/);
  assert.match(stackPopupSurface, /let stackTerminalListenersDisposed\s*=/);
  assert.match(stackPopupSurface, /stackTerminalListenersDisposed = true/);
  assert.match(stackPopupSurface, /unlisten\(\)/);
  assert.match(stackPopupSurface, /if \(stackTerminalListenersDisposed\)/);
  assert.match(stackPopupSurface, /event\.payload\.sessionId !== stackTerminalSession\?\.sessionId/);
  assert.match(stackPopupSurface, /return;/);
  assert.match(stackPopupSurface, /stackTerminalWatchdog|watchdog/i);
  assert.doesNotMatch(
    stackPopupSurface,
    /stackTerminalPollTimer = window\.setInterval\([\s\S]{0,160}700/,
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
  assert.match(stackPopupSurface, /fitAddon\.fit\(\)/);
  assert.match(stackPopupSurface, /stackTerminal\.cols/);
  assert.match(stackPopupSurface, /stackTerminal\.rows/);
  assert.match(stackPopupSurface, /const sessionId = stackTerminalSession\.sessionId/);
  assert.match(stackPopupSurface, /const cols = stackTerminal\.cols/);
  assert.match(stackPopupSurface, /const rows = stackTerminal\.rows/);
  assert.match(stackPopupSurface, /resizeStackTerminal\(\s*sessionId,\s*cols,\s*rows/);
});

test('phase 1 terminal poll and write paths serialize frontend reads for one active session', () => {
  assert.match(stackPopupSurface, /let stackTerminalPollInFlight\s*=/);
  assert.match(stackPopupSurface, /let stackTerminalPollQueued\s*=/);
  assert.match(stackPopupSurface, /async function pollStackTerminalOutput/);
  assert.match(stackPopupSurface, /if \(stackTerminalPollInFlight\)/);
  assert.match(stackPopupSurface, /stackTerminalPollQueued = true/);
  assert.match(stackPopupSurface, /stackTerminalPollInFlight = true/);
  assert.match(stackPopupSurface, /stackTerminalPollInFlight = false/);
  assert.match(stackPopupSurface, /if \(stackTerminalPollQueued\)/);
  assert.match(stackPopupSurface, /void pollStackTerminalOutput\(\)/);
  assert.match(stackPopupSurface, /let stackTerminalOperationQueue: Promise<void> = Promise\.resolve\(\)/);
  assert.match(stackPopupSurface, /function enqueueStackTerminalOperation/);
  assert.match(stackPopupSurface, /enqueueStackTerminalOperation\(\(\) => writeStackTerminal\(sessionId, data\)\)[\s\S]{0,260}pollStackTerminalOutput\(\)/);
  assert.match(stackPopupSurface, /enqueueStackTerminalOperation\(\(\) => readStackTerminal\(sessionId\)\)/);
  assert.match(stackPopupSurface, /enqueueStackTerminalOperation\(\(\) => resizeStackTerminal\(/);
});

test('phase 1 terminal PowerShell launch plan uses trusted path explicitly and has fallback coverage', () => {
  assert.match(rustTerminal, /fn powershell_cmd_launch_line\(powershell: PathBuf\) -> String/);
  assert.match(rustTerminal, /powershell\.to_string_lossy\(\)/);
  assert.doesNotMatch(rustTerminal, /fn powershell_cmd_launch_line\(_powershell: PathBuf\)/);
  assert.doesNotMatch(rustTerminal, /"pwsh\.exe \{\}"/);
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

test('stack browser no longer exposes embedded CLI view controls', () => {
  assert.doesNotMatch(stackPopupSurface, /class:terminal-mode/);
  assert.doesNotMatch(stackPopupSurface, /ariaPressed=\{stackBrowserViewMode === 'files'\}/);
  assert.doesNotMatch(stackPopupSurface, /ariaPressed=\{stackBrowserViewMode === 'terminal'\}/);
  assert.doesNotMatch(stackPopupSurface, /onClick=\{\(\) => void switchStackBrowserView\('files'\)\}/);
  assert.doesNotMatch(stackPopupSurface, /onClick=\{\(\) => void switchStackBrowserView\('terminal'\)\}/);
  assert.doesNotMatch(stackPopupSurface, /class="stack-view-toggle"/);
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
  assert.match(terminalPanelSurface, /cursorBlink: true/);
  assert.match(terminalPanelSurface, /fontFamily: 'var\(--js-font-sans\)'/);
  assert.match(terminalPanelSurface, /fontSize: 12/);
  assert.match(terminalPanelSurface, /fitAddon = new FitAddon\(\);/);
  assert.match(terminalPanelSurface, /terminal\.loadAddon\(fitAddon\)/);
  assert.match(terminalPanelSurface, /terminal\.onData\(\(data\) => \{/);
  assert.match(terminalPanelSurface, /void writeTerminalData\(data\);/);
  assert.match(terminalPanelSurface, /async function writeTerminalData\(data: string\)/);
  assert.match(terminalPanelSurface, /writeStackTerminal\(sessionId, data\)/);
  assert.match(terminalPanelSurface, /pollTimer = window\.setInterval/);
  assert.match(terminalPanelSurface, /stopStackTerminal\(oldSession\)/);
  assert.match(terminalPanelSurface, /bind:this=\{host\}/);
  assert.match(terminalPanelSurface, /function scrollToBottom\(\)/);
  assert.match(terminalPanelSurface, /terminal\?\.scrollToBottom\(\)/);
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
  assert.match(terminalPanelCss, /user-select: text;/);
  assert.match(terminalPanelCss, /\.terminal-panel-output :global\(\.xterm\)/);
  assert.match(terminalPanelCss, /\.terminal-panel-output :global\(\.xterm-helper-textarea\)/);
});

test('terminal cwd changes update Stack Browser path and breadcrumbs immediately', () => {
  assert.match(stackPopupSurface, /async function syncFolderToStackTerminalCwd\(\)/);
  assert.match(stackPopupSurface, /async function applyStackTerminalCwd\(cwd: string\)/);
  assert.match(stackPopupSurface, /await applyStackTerminalCwd\(result\.cwd \|\| stackTerminalCwd\)/);
  assert.match(stackPopupSurface, /stackBrowserViewMode === 'terminal'[\s\S]{0,220}await openFolder\(nextCwd, \{ warmTerminal: false \}\);/);
  assert.match(stackPopupSurface, /await openFolder\(nextCwd, \{ warmTerminal: false \}\);/);
  assert.match(stackPopupSurface, /await openFolder\(stackTerminalCwd, \{ warmTerminal: false \}\);/);
  assert.match(stackPopupSurface, /await syncFolderToStackTerminalCwd\(\);[\s\S]*stackBrowserViewMode = 'files';/);
});

test('stack terminal PowerShell profile loads normal shell affordances and normalizes extended cwd paths', () => {
  assert.match(rustTerminal, /"-ExecutionPolicy"\.to_string\(\),\s*"Bypass"\.to_string\(\)/);
  assert.match(rustTerminal, /"-NoExit"\.to_string\(\)/);
  assert.doesNotMatch(rustTerminal, /"-Command"\.to_string\(\)/);
  assert.match(rustTerminal, /powershell_startup_script\(\)/);
  assert.match(rustTerminal, /powershell_augmented_path\(\)/);
  assert.match(rustTerminal, /command\.env\("PATH", path\)/);
  assert.match(rustTerminal, /powershell\.to_string_lossy\(\)/);
  assert.match(rustTerminal, /Set-Alias -Name ls -Value Get-ChildItem -Force/);
  assert.match(rustTerminal, /Set-Alias -Name clear -Value Clear-Host -Force/);
  assert.match(rustTerminal, /function which \{ Get-Command @args \}/);
  assert.doesNotMatch(rustTerminal, /"Clear-Host",/);
  assert.doesNotMatch(rustTerminal, /"-NoProfile"\.to_string\(\)/);
  assert.match(rustTerminal, /use portable_pty::\{native_pty_system, Child as PtyChild, CommandBuilder, MasterPty, PtySize\}/);
  assert.match(rustTerminal, /native_pty_system\(\)/);
  assert.match(rustTerminal, /\.openpty\(PtySize/);
  assert.match(rustTerminal, /\.slave\.spawn_command\(command\)/);
  assert.match(rustTerminal, /master: Option<Box<dyn MasterPty \+ Send>>/);
  assert.match(rustTerminal, /master: Some\(pty_pair\.master\)/);
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
  assert.match(stackPopupCss, /\.stack-terminal-output :global\(\.xterm-helper-textarea\) \{[\s\S]*border: none !important;[\s\S]*box-shadow: none !important;[\s\S]*outline: 0 !important;/);
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
