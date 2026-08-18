import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const taskbarPinsSource = readFileSync(new URL('../src/lib/taskbarPins.ts', import.meta.url), 'utf8');
const taskbarMenusSource = readFileSync(new URL('../src/lib/taskbarMenus.ts', import.meta.url), 'utf8');
const quickLaunchPanelSource = readFileSync(new URL('../src/components/QuickLaunchPanelSurface.svelte', import.meta.url), 'utf8');
const quickLaunchLibSource = readFileSync(new URL('../src/lib/quickLaunchPanel.ts', import.meta.url), 'utf8');

test('Explorer pin launch failures never remove or hide launcher buttons', () => {
  assert.match(bottomBarSource, /async function launchApp\(launcher: PinnedTaskbarLauncher\)/);
  assert.match(bottomBarSource, /await launchPinnedTaskbarLauncher\(launcher\.shortcutPath\)/);
  const launchFn = bottomBarSource.match(
    /async function launchApp\(launcher: PinnedTaskbarLauncher\) \{[\s\S]*?^  \}/m
  )?.[0] ?? '';
  assert.doesNotMatch(launchFn, /launchers = launchers\.filter/);
  assert.doesNotMatch(launchFn, /launchers = \[\]/);
  assert.doesNotMatch(launchFn, /saveShellSettings/);
  assert.doesNotMatch(launchFn, /unpin/);
});

test('app-managed quick icon frontend path is retired', () => {
  assert.doesNotMatch(taskbarPinsSource, /invoke|listQuickIcons|launchQuickIcon|QuickIcon/);
  assert.doesNotMatch(taskbarMenusSource, /QuickIcon|showQuickIconContextMenu|QUICK_ICON_MENU_ACTIONS/);
  assert.doesNotMatch(bottomBarSource, /quickIcons|quick-icon|Pinned quick icons|quick-launch-menu/);
});

test('quick launch button closes on pointerdown and suppresses the racing reopen click', () => {
  assert.match(bottomBarSource, /let suppressQuickLaunchClick = false;/);
  assert.match(bottomBarSource, /function handleQuickLaunchPointerDown\(event: PointerEvent\)/);
  assert.match(bottomBarSource, /if \(event\.button !== 0 \|\| !quickLaunchPanelOpen\) \{/);
  assert.match(bottomBarSource, /suppressQuickLaunchClick = true;/);
  assert.match(bottomBarSource, /void hideQuickLaunchPanel\(\);/);
  assert.match(bottomBarSource, /function handleQuickLaunchClick\(\)/);
  const closeFn = bottomBarSource.match(
    /listen<QuickLaunchClosedPayload>\(QUICK_LAUNCH_CLOSED_EVENT, \(event\) => \{[\s\S]*?^    \}\)\);/m
  )?.[0] ?? '';
  assert.doesNotMatch(closeFn, /suppressQuickLaunchClick = false;/);
  assert.match(bottomBarSource, /if \(suppressQuickLaunchClick\) \{\s+suppressQuickLaunchClick = false;\s+return;/);
  assert.match(bottomBarSource, /onPointerDown=\{handleQuickLaunchPointerDown\}/);
  assert.match(bottomBarSource, /onClick=\{handleQuickLaunchClick\}/);
});

test('quick launch close handler leaves suppression for trailing click consumption only', () => {
  const closeFn = bottomBarSource.match(
    /listen<QuickLaunchClosedPayload>\(QUICK_LAUNCH_CLOSED_EVENT, \(event\) => \{[\s\S]*?^    \}\)\);/m
  )?.[0] ?? '';
  assert.match(closeFn, /quickLaunchSessionNonce = null;/);
  assert.match(closeFn, /quickLaunchPanelOpen = false;/);
  assert.doesNotMatch(closeFn, /suppressQuickLaunchClick = false;/);
  const clickFn = bottomBarSource.match(/function handleQuickLaunchClick\(\) \{[\s\S]*?^  \}/m)?.[0] ?? '';
  assert.match(clickFn, /if \(suppressQuickLaunchClick\) \{/);
  assert.match(clickFn, /suppressQuickLaunchClick = false;/);
});

test('quick launch panel exposes only admin right-click action', () => {
  assert.match(quickLaunchPanelSource, /showQuickLaunchPanelContextMenu/);
  assert.match(quickLaunchPanelSource, /on:contextmenu\|preventDefault=/);
  assert.doesNotMatch(quickLaunchPanelSource, /suppressNextNativeMenuBlurClose/);
  assert.match(quickLaunchPanelSource, /invoke\('hide_quick_launch_panel_on_focus_loss'\)/);
  assert.match(quickLaunchLibSource, /showQuickLaunchPanelContextMenu/);
});

test('quick launch selected row and focus-visible stay background-only', () => {
  const focusedRule = quickLaunchPanelSource.match(/\.rows button:focus-visible \{[\s\S]*?^  \}/m)?.[0] ?? '';
  const selectedRule = quickLaunchPanelSource.match(/\.rows button\.focused \{[\s\S]*?^  \}/m)?.[0] ?? '';
  const baseRule = quickLaunchPanelSource.match(/\.rows button \{[\s\S]*?^  \}/m)?.[0] ?? '';
  assert.match(baseRule, /border: 1px solid transparent;/);
  assert.match(selectedRule, /background: var\(--js-color-selected\);/);
  assert.doesNotMatch(selectedRule, /outline|box-shadow|border-color/);
  assert.match(focusedRule, /background: var\(--js-color-selected\);/);
  assert.match(focusedRule, /outline:\s*none;/);
  assert.match(focusedRule, /outline-offset:\s*0;/);
  assert.match(focusedRule, /box-shadow:\s*none;/);
  assert.match(focusedRule, /border-color:\s*transparent;/);
});

test('quick launch panel ignores stale closed nonce and clears state on valid close', () => {
  const closeFn = quickLaunchPanelSource.match(/listen<\{ nonce: string \| null \}>\(QUICK_LAUNCH_CLOSED_EVENT, \(event\) => \{[\s\S]*?^    \}\);/m)?.[0] ?? '';
  assert.match(closeFn, /if \(event\.payload\.nonce !== null && event\.payload\.nonce !== quickLaunchNonce\) \{/);
  assert.match(closeFn, /focusedIndex = 0;/);
  assert.match(closeFn, /launchers = \[\];/);
  assert.match(closeFn, /quickLaunchNonce = null;/);
  assert.match(closeFn, /quickLaunchSelectionInFlight = false;/);
});

test('quick launch backend owns blur hold and release around native menu', () => {
  const quickLaunchBackendSource = readFileSync(new URL('../src-tauri/src/quick_launch_panel.rs', import.meta.url), 'utf8');
  const taskbarMenuSource = readFileSync(new URL('../src-tauri/src/taskbar_menu.rs', import.meta.url), 'utf8');
  assert.match(quickLaunchBackendSource, /focus_loss_hold_count/);
  assert.match(quickLaunchBackendSource, /begin_quick_launch_panel_focus_hold\(\);/);
  assert.match(quickLaunchBackendSource, /impl Drop for FocusHoldGuard/);
  assert.match(quickLaunchBackendSource, /hide_quick_launch_panel_on_focus_loss/);
  assert.match(taskbarMenuSource, /run_quick_launch_panel_as_admin/);
  assert.match(taskbarMenuSource, /show_quick_launch_panel_context_menu/);
});

test('access-denied launch retries original shortcut with runas and AppX target fallback stays elevated', () => {
  const launchersSource = readFileSync(new URL('../src-tauri/src/launchers.rs', import.meta.url), 'utf8');
  const adminFn = launchersSource.match(/fn launch_pinned_taskbar_app_as_admin\(shortcut_path: PathBuf\) -> Result<\(\), String> \{[\s\S]*?^    \}/m)?.[0] ?? '';
  assert.match(launchersSource, /Ok\(SE_ERR_ACCESSDENIED\) => launch_pinned_taskbar_app_as_admin\(shortcut_path\)/);
  assert.match(launchersSource, /SE_ERR_FNF_NOASSOC/);
  assert.match(adminFn, /Some\("runas"\)/);
  assert.doesNotMatch(adminFn, /Some\("runas"\)[\s\S]*shell_execute_target\(PathBuf::from\(target_path\), None/);
});

test('quick launch command surface is registered in native IPC and contracts', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
  const contractsSource = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');
  assert.match(mainSource, /show_quick_launch_panel_context_menu/);
  assert.match(contractsSource, /SHOW_QUICK_LAUNCH_PANEL_CONTEXT_MENU/);
  assert.match(
    contractsSource,
    /SHOW_LAUNCHER_CONTEXT_MENU,[\s\S]*SHOW_QUICK_LAUNCH_PANEL_CONTEXT_MENU,[\s\S]*SHOW_TOP_BAR_PIN_CONTEXT_MENU/
  );
});
