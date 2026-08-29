import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  hasTaskbarLauncherDragStarted,
  orderTaskbarLaunchers,
  reconcileTaskbarLauncherOrder,
  resolveTaskbarLauncherPointerRelease,
  taskbarLauncherOrderFromDisplacement,
  normalizeTaskbarPinTargetKey,
  preserveExplorerTaskbarPins
} from '../dist-tests/lib/taskbarPins.js';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const bottomBarCss = readFileSync(new URL('../src/components/BottomBar.css', import.meta.url), 'utf8');
const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const settingsSource = readFileSync(new URL('../src/lib/settings.ts', import.meta.url), 'utf8');
const taskbarMenusSource = readFileSync(new URL('../src/lib/taskbarMenus.ts', import.meta.url), 'utf8');
const quickLaunchRustSource = readFileSync(new URL('../src-tauri/src/quick_launch_panel.rs', import.meta.url), 'utf8');
const quickLaunchPanelSurfaceSource = readFileSync(new URL('../src/components/QuickLaunchPanelSurface.svelte', import.meta.url), 'utf8');

const chrome = {
  id: 'launcher-chrome',
  name: 'Chrome',
  shortcutPath: 'C:\\Pins\\Chrome.lnk',
  targetPath: 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
  iconDataUrl: 'data:image/png;base64,aaa'
};

const code = {
  id: 'launcher-code',
  name: 'Code',
  shortcutPath: 'C:\\Pins\\Code.lnk',
  targetPath: 'C:\\Users\\me\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe',
  iconDataUrl: 'data:image/png;base64,bbb'
};

const terminal = {
  id: 'launcher-terminal',
  name: 'Terminal',
  shortcutPath: 'C:\\Pins\\Terminal.lnk',
  targetPath: 'C:\\Users\\me\\AppData\\Local\\Microsoft\\WindowsApps\\wt.exe',
  iconDataUrl: 'data:image/png;base64,ccc'
};

test('normalizes Windows-like taskbar pin target keys for diagnostics', () => {
  assert.equal(
    normalizeTaskbarPinTargetKey('C:/Program Files/Google/Chrome/Application/chrome.exe'),
    'c:\\program files\\google\\chrome\\application\\chrome.exe'
  );
  assert.equal(
    normalizeTaskbarPinTargetKey('  \\\\Server\\Share\\Tool.exe  '),
    '\\\\server\\share\\tool.exe'
  );
});

test('preserves Explorer launchers without app-managed quick icon dedupe', () => {
  const launchers = [chrome, code];

  const filtered = preserveExplorerTaskbarPins(launchers);
  assert.deepEqual(filtered.map((launcher) => launcher.name), ['Chrome', 'Code']);
});

test('reconciles local Explorer launcher order across native refreshes', () => {
  assert.deepEqual(
    reconcileTaskbarLauncherOrder(
      [terminal.shortcutPath, chrome.shortcutPath],
      [chrome, code, terminal]
    ),
    [terminal.shortcutPath, chrome.shortcutPath, code.shortcutPath]
  );
});

test('orders Explorer launchers by local drag order without mutating launcher data', () => {
  assert.deepEqual(
    orderTaskbarLaunchers([chrome, code, terminal], [terminal.shortcutPath, chrome.shortcutPath])
      .map((launcher) => launcher.name),
    ['Terminal', 'Chrome', 'Code']
  );
});

test('uses task-window-style launcher drag threshold and displacement ordering', () => {
  const order = [chrome.shortcutPath, code.shortcutPath, terminal.shortcutPath];
  const rects = [
    { key: chrome.shortcutPath, left: 0, width: 40 },
    { key: code.shortcutPath, left: 40, width: 40 },
    { key: terminal.shortcutPath, left: 80, width: 40 }
  ];

  assert.equal(hasTaskbarLauncherDragStarted(100, 105), false);
  assert.equal(hasTaskbarLauncherDragStarted(100, 106), true);
  assert.deepEqual(taskbarLauncherOrderFromDisplacement(chrome.shortcutPath, order, rects, 81), [
    code.shortcutPath,
    terminal.shortcutPath,
    chrome.shortcutPath
  ]);
  assert.deepEqual(taskbarLauncherOrderFromDisplacement(terminal.shortcutPath, order, rects, -81), [
    terminal.shortcutPath,
    chrome.shortcutPath,
    code.shortcutPath
  ]);
});

test('launcher pointer release suppresses click only after a real drag', () => {
  assert.deepEqual(resolveTaskbarLauncherPointerRelease(chrome.shortcutPath, false), {
    suppressClickKey: null
  });
  assert.deepEqual(resolveTaskbarLauncherPointerRelease(chrome.shortcutPath, true), {
    suppressClickKey: chrome.shortcutPath
  });
});

test('BottomBar keeps launcher order state but no longer renders inline launcher strip', () => {
  assert.match(bottomBarSource, /TASKBAR_LAUNCHER_ORDER_STORAGE_KEY/);
  assert.match(bottomBarSource, /let launcherOrder: string\[\] = readPersistedLauncherOrder\(\)/);
  assert.match(bottomBarSource, /writePersistedLauncherOrder\(launcherOrder\)/);
  assert.match(bottomBarSource, /taskbarLauncherOrderFromDisplacement/);
  assert.match(bottomBarSource, /suppressClickLauncherKey/);
  assert.doesNotMatch(bottomBarSource, /<div class="launcher-strip-launchers"|<MeltActionButton\s+class=\{`launcher-button/);
});

test('bottom bar quick launch button opens alphabetical upward list', () => {
  assert.match(bottomBarSource, /Quick Launch/);
  assert.match(bottomBarSource, /quickLaunchPanelOpen/);
  assert.match(bottomBarSource, /quickLaunchSessionNonce/);
  assert.match(bottomBarSource, /quickLaunchSessionNonce = nonce;\s+quickLaunchPanelOpen = true;/);
  assert.match(bottomBarSource, /invoke\('show_quick_launch_panel', \{ args: \{ anchorLeft: rect\?\.left \?\? 0, anchorWidth: rect\?\.width \?\? 0, nonce, rows \} \}\)/);
  assert.match(bottomBarSource, /ariaHaspopup="dialog"/);
  assert.match(bottomBarSource, /quick-launch-panel:open/);
  assert.match(bottomBarSource, /quick-launch-panel:closed/);
  assert.match(bottomBarSource, /quickLaunchOpenInFlight/);
  assert.match(bottomBarSource, /if \(quickLaunchSessionNonce === nonce\) \{\s+quickLaunchSessionNonce = null;\s+quickLaunchPanelOpen = false;/);
});

test('quick launch protocol source contracts keep camelCase payloads and scoped events', () => {
  assert.match(quickLaunchRustSource, /#\[serde\(rename_all = "camelCase"\)\]\s+pub struct QuickLaunchPanelShowArgs/);
  assert.match(quickLaunchRustSource, /#\[serde\(rename_all = "camelCase"\)\]\s+pub struct QuickLaunchPanelSelectArgs/);
  assert.match(quickLaunchRustSource, /#\[serde\(rename_all = "camelCase"\)\]\s+pub struct QuickLaunchPanelOpenPayload/);
  assert.match(quickLaunchRustSource, /#\[serde\(rename_all = "camelCase"\)\]\s+pub struct QuickLaunchPanelRow/);
  assert.match(quickLaunchRustSource, /emit_to\(\s*QUICK_LAUNCH_PANEL_LABEL,\s*QUICK_LAUNCH_PANEL_OPEN_EVENT/);
  assert.match(quickLaunchRustSource, /emit_to\(\s*BOTTOM_BAR_LABEL,\s*QUICK_LAUNCH_PANEL_CLOSED_EVENT/);
  assert.match(quickLaunchRustSource, /emit_to\(\s*QUICK_LAUNCH_PANEL_LABEL,\s*QUICK_LAUNCH_PANEL_CLOSED_EVENT/);
  assert.match(quickLaunchRustSource, /panel\s*\.hide\(\)\s*\.map_err\(\|error\| format!\("Failed to hide quick launch panel after selection: \{error\}"\)\)\?/);
  assert.match(quickLaunchRustSource, /validate_quick_launch_panel_shortcut\(\s*args\.nonce\.as_str\(\),\s*args\.shortcut_path\.as_str\(\),?\s*\)\?/);
  assert.match(quickLaunchRustSource, /allowed_shortcuts\.contains\(&canonical_shortcut_path\)/);
  assert.match(quickLaunchRustSource, /launch_pinned_taskbar_app_internal\(\s*shortcut_path\.to_string_lossy\(\)\.into_owned\(\),?\s*\)\?/);
  assert.match(quickLaunchRustSource, /let result = crate::launchers::run_pinned_taskbar_app_as_admin\(\s*shortcut_path\.to_string_lossy\(\)\.into_owned\(\),?\s*\);\s+end_quick_launch_panel_focus_hold\(\);\s+result\?;/);
  assert.match(quickLaunchRustSource, /let result = crate::launchers::run_pinned_taskbar_app_as_admin\(\s*shortcut_path\.to_string_lossy\(\)\.into_owned\(\),?\s*\);/);
  assert.match(quickLaunchRustSource, /end_quick_launch_panel_focus_hold\(\);/);
  assert.match(quickLaunchRustSource, /result\?;\s+let panel = app_handle/);
  assert.match(quickLaunchRustSource, /panel\s*\.hide\(\)\s*\.map_err\(\|error\| format!\("Failed to hide quick launch panel: \{error\}"\)\)\?/);
  assert.match(quickLaunchRustSource, /quick_launch_panel_state_reset\(\);\s+app_handle\s*\.emit_to\(\s*BOTTOM_BAR_LABEL,\s*QUICK_LAUNCH_PANEL_CLOSED_EVENT/);
  assert.match(quickLaunchRustSource, /emit_to\(\s*QUICK_LAUNCH_PANEL_LABEL,\s*QUICK_LAUNCH_PANEL_CLOSED_EVENT/);
  assert.match(quickLaunchPanelSurfaceSource, /quickLaunchSelectionInFlight = true;/);
  assert.match(quickLaunchPanelSurfaceSource, /if \(!quickLaunchSelectionInFlight\) \{\s+void invoke\('hide_quick_launch_panel_on_focus_loss'\);\s+\}/);
  assert.match(quickLaunchPanelSurfaceSource, /on:contextmenu\|preventDefault=/);
  assert.match(quickLaunchPanelSurfaceSource, /role="option"[\s\S]*?aria-haspopup="menu"/);
  assert.match(quickLaunchPanelSurfaceSource, /await invoke\('select_quick_launch_panel', \{ args: \{ nonce: quickLaunchNonce, shortcutPath: launcher\.shortcutPath \} \}\);/);
  assert.match(quickLaunchPanelSurfaceSource, /quickLaunchSelectionInFlight = false;/);
  assert.doesNotMatch(bottomBarSource, /quick-launch-panel:select/);
});

test('bottom bar renders only Explorer taskbar pins before open windows', () => {
  assert.match(bottomBarSource, /listPinnedTaskbarLaunchers/);
  assert.match(bottomBarSource, /Pinned Explorer taskbar apps/);
  assert.doesNotMatch(bottomBarSource, /listQuickIcons|launchQuickIcon|showQuickIconContextMenu/);
  assert.doesNotMatch(bottomBarSource, /quickIcons|quick-icon|Pinned quick icons|launcher-strip-launchers/);
  assert.doesNotMatch(bottomBarCss, /quick-icon/);
});

test('frontend no longer exposes app-managed quick icon IPC or settings path', () => {
  assert.doesNotMatch(commandsSource, /listQuickIcons|pinTaskWindowQuickIcon|unpinQuickIcon|launchQuickIcon/);
  assert.doesNotMatch(commandsSource, /showQuickIconContextMenu/);
  assert.doesNotMatch(settingsSource, /quickIcons|QuickIconsSettings|defaultQuickIconsSettings/);
  assert.doesNotMatch(taskbarMenusSource, /QuickIcon|showQuickIconContextMenu|QUICK_ICON_MENU_ACTIONS/);
});
