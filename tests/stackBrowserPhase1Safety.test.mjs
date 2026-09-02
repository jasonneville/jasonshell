import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const contractsSource = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const stackPopupSource = readFileSync(new URL('../src-tauri/src/stack_popup.rs', import.meta.url), 'utf8');
const stackPopupAuthSource = readFileSync(new URL('../src-tauri/src/stack_popup/auth.rs', import.meta.url), 'utf8');
const stackPopupWrapperSource = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');

const stackPhase1Commands = [
  ['LIST_PINNED_STACK_FOLDERS', 'list_pinned_stack_folders', ['top-bar', 'stack-popup']],
  ['PIN_STACK_FOLDER', 'pin_stack_folder', ['top-bar', 'stack-popup', 'search-panel']],
  ['UNPIN_STACK_FOLDER', 'unpin_stack_folder', ['top-bar', 'stack-popup']],
  ['REORDER_PINNED_STACK_FOLDERS', 'reorder_pinned_stack_folders', ['top-bar']],
  ['SHOW_STACK_POPUP', 'show_stack_popup', ['top-bar']],
  ['HIDE_STACK_POPUP', 'hide_stack_popup', ['top-bar', 'stack-popup']],
  ['TOGGLE_STACK_POPUP', 'toggle_stack_popup', ['top-bar']],
  ['BEGIN_STACK_POPUP_FOCUS_LOSS_HOLD', 'begin_stack_popup_focus_loss_hold', ['top-bar', 'stack-popup']],
  ['END_STACK_POPUP_FOCUS_LOSS_HOLD', 'end_stack_popup_focus_loss_hold', ['top-bar', 'stack-popup']],
  ['GET_STACK_POPUP_REQUEST', 'get_stack_popup_request', ['stack-popup']],
  ['RESIZE_STACK_POPUP', 'resize_stack_popup', ['stack-popup']],
  ['READ_STACK_FOLDER', 'read_stack_folder', ['stack-popup']],
  ['SUGGEST_STACK_PATHS', 'suggest_stack_paths', ['stack-popup']],
  ['RESOLVE_STACK_ITEM_ICONS', 'resolve_stack_item_icons', ['stack-popup']],
  ['OPEN_STACK_ITEM', 'open_stack_item', ['stack-popup', 'terminal-panel']],
  ['OPEN_STACK_ITEM_WITH_PICKER', 'open_stack_item_with_picker', ['stack-popup']],
  ['LIST_STACK_OPEN_WITH_CANDIDATES', 'list_stack_open_with_candidates', ['stack-popup']],
  ['OPEN_STACK_ITEM_WITH_APP', 'open_stack_item_with_app', ['stack-popup']],
  ['RENAME_STACK_ITEM', 'rename_stack_item', ['stack-popup']],
  ['COPY_STACK_ITEMS', 'copy_stack_items', ['stack-popup']],
  ['PREPARE_STACK_FILE_DRAG', 'prepare_stack_file_drag', ['stack-popup']],
  ['CUT_STACK_ITEMS', 'cut_stack_items', ['stack-popup']],
  ['PASTE_STACK_ITEMS', 'paste_stack_items', ['stack-popup']],
  ['DELETE_STACK_ITEM', 'delete_stack_item', ['stack-popup']],
  ['NEW_STACK_FOLDER', 'new_stack_folder', ['stack-popup']],
  ['NEW_STACK_TEXT_FILE', 'new_stack_text_file', ['stack-popup']],
  ['OPEN_STACK_TERMINAL_HERE', 'open_stack_terminal_here', ['stack-popup']],
  ['REVEAL_STACK_ITEM', 'reveal_stack_item', ['stack-popup']],
  ['EXTRACT_STACK_ARCHIVE', 'extract_stack_archive', ['stack-popup']],
  ['SHOW_STACK_ITEM_PROPERTIES', 'show_stack_item_properties', ['stack-popup']],
  ['OPEN_STACK_FOLDER_IN_VSCODE', 'open_stack_folder_in_vscode', ['top-bar', 'stack-popup']],
  ['GET_STACK_GIT_STATUS', 'get_stack_git_status', ['stack-popup']],
  ['OPEN_STACK_GIT_REMOTE_URL', 'open_stack_git_remote_url', ['stack-popup']],
  ['STACK_GIT_ADD_PATHS', 'stack_git_add_paths', ['stack-popup']],
  ['STACK_GIT_COMMIT', 'stack_git_commit', ['stack-popup']],
  ['STACK_GIT_LOG', 'stack_git_log', ['stack-popup']],
  ['STACK_GIT_TREE', 'stack_git_tree', ['stack-popup']],
  ['STACK_GIT_BRANCHES', 'stack_git_branches', ['stack-popup']],
  ['STACK_GIT_FETCH', 'stack_git_fetch', ['stack-popup']],
  ['STACK_GIT_PULL', 'stack_git_pull', ['stack-popup']],
  ['STACK_GIT_PUSH', 'stack_git_push', ['stack-popup']],
  ['STACK_GIT_CHECKOUT_BRANCH', 'stack_git_checkout_branch', ['stack-popup']],
  ['STACK_GIT_CREATE_BRANCH', 'stack_git_create_branch', ['stack-popup']],
  ['START_PERSISTENT_TERMINAL', 'start_persistent_terminal', ['terminal-panel']],
  ['START_STACK_TERMINAL', 'start_stack_terminal', ['terminal-panel', 'stack-popup']],
  ['READ_STACK_TERMINAL', 'read_stack_terminal', ['session-target']],
  ['WRITE_STACK_TERMINAL', 'write_stack_terminal', ['session-target']],
  ['RESIZE_STACK_TERMINAL', 'resize_stack_terminal', ['session-target']],
  ['STOP_STACK_TERMINAL', 'stop_stack_terminal', ['session-target']],
  ['POLL_STACK_TERMINAL_SESSION', 'poll_stack_terminal_session', ['session-target']],
  ['LIST_STACK_TERMINALS', 'list_stack_terminals', ['requested-target']],
  ['RENAME_STACK_TERMINAL', 'rename_stack_terminal', ['session-target']],
  ['STOP_TERMINAL_PANEL_SESSIONS', 'stop_terminal_panel_sessions', ['terminal-panel']],
  ['GET_STACK_TERMINAL_CWD', 'get_stack_terminal_cwd', ['session-target']]
];

test('WP0 command inventory exposes Stack Browser Phase 1 parity blocker before auth work', () => {
  assert.match(mainSource, /stack_popup::open_stack_folder_in_vscode/);
  assert.match(stackPopupWrapperSource, /openStackFolderInVscode[\s\S]*IPC_COMMANDS\.openStackFolderInVscode/);
  assert.match(contractsSource, /pub const OPEN_STACK_FOLDER_IN_VSCODE: &str = "open_stack_folder_in_vscode"/);
  assert.match(contractsSource, /OPEN_STACK_FOLDER_IN_VSCODE[\s\S]*commands::ALL/);
});

test('WP0 auth matrix source contract classifies every current Stack Browser Phase 1 command', () => {
  assert.match(stackPopupAuthSource, /enum\s+StackCommandAuth\s*\{/);
  assert.match(stackPopupAuthSource, /fn\s+allowed_stack_command_callers\s*\([^)]*StackCommandAuth/);

  for (const [constantName, commandName, allowedLabels] of stackPhase1Commands) {
    assert.match(stackPopupAuthSource, new RegExp(`\\b${constantName}\\b|"${commandName}"`), `${commandName} missing from auth command enum/matrix`);
    if (allowedLabels.includes('session-target') || allowedLabels.includes('requested-target')) {
      assert.match(stackPopupAuthSource, new RegExp(`TerminalSessionTarget[\\s\\S]*${constantName}`), `${commandName} missing terminal session-target variant`);
      continue;
    }
    for (const label of allowedLabels) {
      assert.match(stackPopupAuthSource, new RegExp(`"${label}"|${label.replace(/-/g, '_').toUpperCase()}`), `${commandName} missing allowed caller ${label}`);
    }
  }
});

test('WP0 scoped Phase 1 handlers authorize before side effects with window injection', () => {
  const checks = [
    ['open_stack_item_with_picker', 'OPEN_STACK_ITEM_WITH_PICKER'],
    ['list_stack_open_with_candidates', 'LIST_STACK_OPEN_WITH_CANDIDATES'],
    ['open_stack_item_with_app', 'OPEN_STACK_ITEM_WITH_APP'],
    ['rename_stack_item', 'RENAME_STACK_ITEM'],
    ['prepare_stack_file_drag', 'PREPARE_STACK_FILE_DRAG'],
    ['extract_stack_archive', 'EXTRACT_STACK_ARCHIVE'],
    ['show_stack_item_properties', 'SHOW_STACK_ITEM_PROPERTIES'],
  ];
  for (const [fnName, commandConst] of checks) {
    const fnStart = stackPopupSource.indexOf(`fn ${fnName}`);
    assert.ok(fnStart >= 0, `${fnName} missing`);
    const nextFn = stackPopupSource.indexOf('\n#[tauri::command]', fnStart + 1);
    const body = stackPopupSource.slice(fnStart, nextFn > 0 ? nextFn : undefined);
    assert.match(body, /window:\s*WebviewWindow/);
    assert.match(body, new RegExp(`authorize_stack_command\\(\\s*\\&window,\\s*StackCommandAuth::[\\s\\S]*${commandConst}`));
    assert.ok(body.indexOf('authorize_stack_command') < body.indexOf('shell_paths::open_shell_path_with_picker') || !body.includes('shell_paths::open_shell_path_with_picker'));
  }
});

test('Stack Browser toggle authorizes top-bar before changing native visibility', () => {
  const fnStart = stackPopupSource.indexOf('fn toggle_stack_popup');
  const nextFn = stackPopupSource.indexOf('\n#[tauri::command]', fnStart + 1);
  const body = stackPopupSource.slice(fnStart, nextFn > 0 ? nextFn : undefined);

  assert.match(body, /window:\s*WebviewWindow/);
  assert.match(body, /authorize_stack_command\([\s\S]*TOGGLE_STACK_POPUP/);
  assert.ok(body.indexOf('authorize_stack_command') < body.indexOf('.is_visible()'));
});

test('WP0 phase 1 file mutation auth excludes terminal-panel for copy-cut-paste-delete-create', () => {
  for (const constName of ['COPY_STACK_ITEMS', 'CUT_STACK_ITEMS', 'PASTE_STACK_ITEMS', 'DELETE_STACK_ITEM', 'NEW_STACK_FOLDER', 'NEW_STACK_TEXT_FILE']) {
    const authLine = stackPopupAuthSource.split('\n').find((line) => line.includes(constName));
    assert.ok(authLine, `${constName} missing from auth matrix`);
    assert.doesNotMatch(authLine, /TERMINAL_PANEL/);
  }
});

test('WP0 auth helper fails closed for unknown commands with stable IPC error', () => {
  assert.match(stackPopupAuthSource, /fn\s+authorize_stack_command\s*\(/);
  assert.match(stackPopupAuthSource, /Unauthorized caller for command \{command\}/);
  assert.match(stackPopupAuthSource, /Err\(CallerAuthError::Unauthorized/);
  assert.doesNotMatch(stackPopupAuthSource, /JASONSHELL_.*AUTH.*DISABLE|AUTH.*KILL_SWITCH/i);
});

test('WP0 disallows cross-surface terminal target spoofing by contract', () => {
  assert.match(stackPopupSource, /caller.*label|window\.label\(\)/s);
  assert.match(stackPopupAuthSource, /session.*target.*label|target.*session/s);
  assert.match(stackPopupSource, /read_stack_terminal[\s\S]*authorize_stack_command/);
  assert.match(stackPopupSource, /write_stack_terminal[\s\S]*authorize_stack_command/);
  assert.match(stackPopupSource, /resize_stack_terminal[\s\S]*authorize_stack_command/);
  assert.match(stackPopupSource, /poll_stack_terminal_session[\s\S]*authorize_stack_command/);
});
