import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const stackPopupApi = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');
const stackPopupSurface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const taskbarMenusTs = readFileSync(new URL('../src/lib/taskbarMenus.ts', import.meta.url), 'utf8');
const taskbarMenuRs = readFileSync(new URL('../src-tauri/src/taskbar_menu.rs', import.meta.url), 'utf8');
const shellPathsRs = readFileSync(new URL('../src-tauri/src/shell_paths.rs', import.meta.url), 'utf8');
const stackPopupRs = readFileSync(new URL('../src-tauri/src/stack_popup.rs', import.meta.url), 'utf8');

test('phase 4 adds VS Code folder-open IPC command wrappers', () => {
  assert.match(commandsSource, /openStackFolderInVscode: 'open_stack_folder_in_vscode'/);
  assert.match(stackPopupApi, /openStackFolderInVscode\(path: string\): Promise<void>/);
  assert.match(stackPopupApi, /invoke\(IPC_COMMANDS\.openStackFolderInVscode, \{ path \}\)/);
});

test('phase 4 stack browser context menus expose folder and current-path Open in VS Code', () => {
  assert.match(
    stackPopupSurface,
    /<MeltActionButton role="menuitem" disabled=\{selectedEntry\?\.entryType !== 'Folder'\} onClick=\{\(\) => void openSelectedFolderInVscode\(\)\}>Open in VS Code<\/MeltActionButton>/
  );
  assert.match(
    stackPopupSurface,
    /<MeltActionButton role="menuitem" disabled=\{!currentPath\} onClick=\{\(\) => void openCurrentFolderInVscode\(\)\}>Open in VS Code<\/MeltActionButton>/
  );
});

test('phase 4 top-bar pin menu includes Open in VS Code and dispatches action payload', () => {
  assert.match(taskbarMenusTs, /action: 'open' \| 'openInVscode' \| 'unpin'/);
  assert.match(taskbarMenuRs, /"Open in VS Code"/);
  assert.match(topBarSource, /event\.payload\.action === 'openInVscode'/);
  assert.match(topBarSource, /openStackFolderInVscode\(event\.payload\.path\)/);
});

test('phase 4 backend includes shared VS Code resolver with safe missing-install error', () => {
  assert.match(shellPathsRs, /pub fn open_folder_in_vscode/);
  assert.match(shellPathsRs, /resolve_vscode_executable/);
  const resolverBody = shellPathsRs.slice(
    shellPathsRs.indexOf('fn resolve_executable_candidate'),
    shellPathsRs.indexOf('fn expand_environment')
  );
  assert.doesNotMatch(resolverBody, /PATH|split_paths\(|code\.cmd/);
  assert.match(shellPathsRs, /Visual Studio Code was not found/);
  assert.match(stackPopupRs, /pub fn open_stack_folder_in_vscode/);
});

test('phase 4 rejects bare executable PATH candidates for VS Code and terminal launch', () => {
  const resolverBody = shellPathsRs.slice(
    shellPathsRs.indexOf('const VSCODE_EXECUTABLE_CANDIDATES'),
    shellPathsRs.indexOf('#[cfg(target_os = "windows")]')
  );
  assert.doesNotMatch(resolverBody, /Command::new\("code\.(cmd|exe)"\)|split_paths\(|PATH/);
  assert.doesNotMatch(stackPopupRs, /Command::new\("wt\.exe"\)|Command::new\("powershell\.exe"\)|Command::new\("cmd\.exe"\)/);
});
