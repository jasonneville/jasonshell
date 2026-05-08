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
const contractsSettingsTest = readRepoFile('tests/contractsSettings.test.mjs');
const masterSpec = readRepoFile('master_spec.md');
const changelog = readRepoFile('changelog.md');

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

test('stack browser toolbar switches between files and CLI views with compact Melt controls', () => {
  assert.match(stackPopupSurface, /type StackBrowserViewMode = 'files' \| 'terminal';/);
  assert.match(stackPopupSurface, /let stackBrowserViewMode: StackBrowserViewMode = 'files';/);
  assert.match(stackPopupSurface, /class:terminal-mode=\{stackBrowserViewMode === 'terminal'\}/);
  assert.match(stackPopupSurface, /ariaPressed=\{stackBrowserViewMode === 'files'\}/);
  assert.match(stackPopupSurface, /ariaPressed=\{stackBrowserViewMode === 'terminal'\}/);
  assert.match(stackPopupSurface, /onClick=\{\(\) => void switchStackBrowserView\('files'\)\}/);
  assert.match(stackPopupSurface, /onClick=\{\(\) => void switchStackBrowserView\('terminal'\)\}/);
  assert.match(stackPopupSurface, /class="stack-view-toggle"/);
  assert.match(stackPopupSurface, /Files<\/MeltActionButton>/);
  assert.match(stackPopupSurface, /CLI<\/MeltActionButton>/);
  assert.doesNotMatch(stackPopupSurface, /Stack Browser embedded terminal lets you|Use this terminal to/);
});

test('stack terminal view starts in current folder, accepts Enter and Ctrl+C, polls output, and stays pinned to bottom', () => {
  assert.match(stackPopupSurface, /startStackTerminal\(currentPath, stackTerminalProfile\)/);
  assert.match(stackPopupSurface, /readStackTerminal\(stackTerminalSession\.sessionId\)/);
  assert.match(stackPopupSurface, /writeStackTerminal\(stackTerminalSession\.sessionId, command \+ '\\n'\)/);
  assert.match(stackPopupSurface, /writeStackTerminal\(stackTerminalSession\.sessionId, '\\u0003'\)/);
  assert.match(stackPopupSurface, /function handleStackTerminalKeydown\(event: KeyboardEvent\)/);
  assert.match(stackPopupSurface, /event\.key === 'Enter'/);
  assert.match(stackPopupSurface, /event\.ctrlKey && event\.key\.toLowerCase\(\) === 'c'/);
  assert.match(stackPopupSurface, /event\.key === 'Escape'[\s\S]{0,180}await closeStackPopupFromSurface\(\);/);
  assert.match(stackPopupSurface, /stackTerminalPollTimer = window\.setInterval/);
  assert.match(stackPopupSurface, /stopStackTerminal\(stackTerminalSession\.sessionId\)/);
  assert.match(stackPopupSurface, /stackTerminalOutput/);
  assert.match(stackPopupSurface, /bind:this=\{stackTerminalOutputElement\}/);
  assert.match(stackPopupSurface, /function scrollStackTerminalToBottom\(\)/);
  assert.match(stackPopupSurface, /stackTerminalOutputElement\.scrollTop = stackTerminalOutputElement\.scrollHeight/);
  assert.match(stackPopupSurface, /bind:this=\{stackTerminalInput\}/);
  assert.match(stackPopupSurface, /class="stack-terminal-output"/);
});

test('terminal cwd changes update Stack Browser path and breadcrumbs immediately', () => {
  assert.match(stackPopupSurface, /async function syncFolderToStackTerminalCwd\(\)/);
  assert.match(stackPopupSurface, /async function applyStackTerminalCwd\(cwd: string\)/);
  assert.match(stackPopupSurface, /await applyStackTerminalCwd\(result\.cwd \|\| stackTerminalCwd\)/);
  assert.match(stackPopupSurface, /stackBrowserViewMode === 'terminal'[\s\S]{0,180}await openFolder\(nextCwd\);/);
  assert.match(stackPopupSurface, /await syncFolderToStackTerminalCwd\(\);[\s\S]*stackBrowserViewMode = 'files';/);
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
  assert.match(stackPopupCss, /\.stack-terminal-command/);
  assert.doesNotMatch(stackPopupCss, /\.stack-terminal-profile/);
  assert.match(stackPopupCss, /font-family: var\(--js-font-mono/);
  assert.match(stackPopupCss, /grid-template-rows: auto minmax\(0, 1fr\) auto;/);
  assert.doesNotMatch(stackPopupCss, /\.stack-terminal[\s\S]{0,220}box-shadow: var\(--js-shadow-panel\)/);
});

test('durable docs mention Stack Browser embedded terminal behavior and validation', () => {
  assert.match(masterSpec, /Stack Browser:[\s\S]*embedded terminal/);
  assert.match(masterSpec, /`start_stack_terminal`/);
  assert.match(masterSpec, /`stackBrowser\.terminalProfile`/);
  assert.match(masterSpec, /tests\/stackBrowserTerminal\.test\.mjs/);
  assert.match(changelog, /Stack Browser embedded terminal/);
  assert.match(changelog, /tests\\stackBrowserTerminal\.test\.mjs|tests\/stackBrowserTerminal\.test\.mjs/);
});
