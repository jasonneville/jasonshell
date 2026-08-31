import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const launchersRs = readFileSync(new URL('../src-tauri/src/launchers.rs', import.meta.url), 'utf8');
const taskWindowWindowsRs = readFileSync(new URL('../src-tauri/src/task_windows/windows.rs', import.meta.url), 'utf8');
const taskWindowHelperRs = readFileSync(new URL('../src-tauri/src/task_windows/helper.rs', import.meta.url), 'utf8');
const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
const mainRs = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

function extractFunction(source, name, occurrence = 0) {
  const pattern = new RegExp(`(?:pub\\s+)?(?:async\\s+)?function\\s+${name}|(?:pub\\s+)?fn\\s+${name}`, 'g');
  let match = null;
  for (let index = 0; index <= occurrence; index += 1) {
    match = pattern.exec(source);
    if (!match) {
      assert.fail(`${name} occurrence ${occurrence} should exist`);
    }
  }
  const start = match.index;
  const braceStart = source.indexOf('{', start);
  assert.notEqual(braceStart, -1, `${name} should have a body`);
  let depth = 0;
  for (let index = braceStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return source.slice(braceStart + 1, index);
      }
    }
  }
  assert.fail(`${name} body should close`);
}

test('Explorer taskbar pin listing does not hide shortcuts just because Resolve fails', () => {
  const listBody = extractFunction(launchersRs, 'list_pinned_taskbar_apps', 1);

  assert.doesNotMatch(listBody, /if\s+!\s*shortcut_resolves[\s\S]*continue/);
  assert.match(listBody, /fallback_launcher_icon_data_url/);
});

test('launcher icon extraction never falls back to the .lnk file association icon', () => {
  const iconBody = extractFunction(launchersRs, 'extract_icon_data_url');

  assert.match(iconBody, /explicit_icon_location\(&shell_link\)/);
  assert.match(iconBody, /apps_folder_shell_item_icon_data_url\(&shell_link\)/);
  assert.match(iconBody, /resolved_shortcut_target\(&shell_link\)/);
  assert.doesNotMatch(iconBody, /extract_file_icon_data_url\(shortcut_path\)/);
  assert.doesNotMatch(iconBody, /PKEY_AppUserModel_RelaunchIconResource|SHGetPropertyStoreFromParsingName|IPropertyStore/);
});

test('target icon stage only skips exact safe AppsFolder explorer proxies', () => {
  const iconBody = extractFunction(launchersRs, 'extract_icon_data_url');
  assert(iconBody.includes('should_extract_target_icon(arguments(&shell_link), target_path.as_path())'));
});

test('target icon decision contract stays positive for ordinary shortcuts and negative only for safe AppsFolder explorer proxies', () => {
  const helperBody = extractFunction(launchersRs, 'should_extract_target_icon');

  assert(helperBody.includes('Err(_) => true'));
  assert(helperBody.includes('Ok(arguments) => !is_apps_folder_proxy(&arguments, target_path)'));
  assert.doesNotMatch(helperBody, /!should_fallback_to_target_icon_for_apps_folder_proxy/);
  assert.match(launchersRs, /target_icon_stage_only_blocks_exact_apps_folder_explorer_proxies/);
});

test('Explorer taskbar pin launch ShellExecutes the shortcut path without Resolve preflight', () => {
  assert.doesNotMatch(launchersRs, /shortcut_resolves/);
  assert.match(launchersRs, /shell_execute_shortcut\(\s*shortcut_path\.clone\(\),\s*None,\s*&\[SE_ERR_ACCESSDENIED\],\s*"launch pinned shortcut"/);
  assert.doesNotMatch(extractFunction(launchersRs, 'shell_execute_shortcut'), /resolved_shortcut_target_path/);
});

test('Explorer taskbar pin launch retries with elevation on access denied', () => {
  assert.match(launchersRs, /SE_ERR_ACCESSDENIED/);
  assert.match(launchersRs, /shell_execute_shortcut\(\s*shortcut_path\.clone\(\),\s*Some\("runas"\),\s*&\[3, SE_ERR_ACCESSDENIED, SE_ERR_FNF_NOASSOC\]/);
  assert.match(extractFunction(launchersRs, 'shell_execute_shortcut'), /code <= 32 && !preserved_codes\.contains\(&code\)/);
  const launchBody = extractFunction(launchersRs, 'launch_pinned_taskbar_app', 2);
  assert.match(launchBody, /Ok\(SE_ERR_ACCESSDENIED\) => launch_pinned_taskbar_app_as_admin\(shortcut_path\)/);
});

test('WindowsApps admin launch falls back to Explorer only for AppX targets', () => {
  assert.match(launchersRs, /fn is_windowsapps_path\(path: &Path\) -> bool/);
  assert.match(launchersRs, /eq_ignore_ascii_case\("WindowsApps"\)/);
  assert.match(launchersRs, /fn launch_windowsapps_target_as_admin_or_explorer_fallback\(/);
  assert.match(launchersRs, /code == 3 \|\| code == SE_ERR_ACCESSDENIED \|\| code == SE_ERR_FNF_NOASSOC/);
  assert.match(launchersRs, /windows_explorer_path\(\)\?/);
  assert.match(launchersRs, /Command::new\(&explorer_path\)\s*\.arg\(shortcut_path\)/);
  assert.match(launchersRs, /WINDIR is unavailable/);
});

test('explicit admin launch hands code 3 or access denied shortcut results to AppX fallback helper', () => {
  const adminBody = extractFunction(launchersRs, 'launch_pinned_taskbar_app_as_admin');

  assert.match(adminBody, /SE_ERR_FNF_NOASSOC/);
  assert.match(adminBody, /Ok\(code\) if code == 3 \|\| code == SE_ERR_ACCESSDENIED \|\| code == SE_ERR_FNF_NOASSOC => \{/);
  assert.match(adminBody, /launch_windowsapps_target_as_admin_or_explorer_fallback\(/);
  assert.doesNotMatch(adminBody, /shell_execute_target\(PathBuf::from\(target_path\), Some\("runas"\), context\.as_str\(\)\)/);
});

test('elevated launcher helper has no status-file handoff and no helper argv path', () => {
  assert.doesNotMatch(launchersRs, /write_helper_status|wait_for_helper_status|launch_status_token|status_path/);
  assert.match(taskWindowHelperRs, /helper_exit_code_for_shell_execute_result/);
  assert.match(taskWindowHelperRs, /UAC canceled/);
});

test('Explorer launch failure does not remove the visible launcher row', () => {
  const launchAppBody = extractFunction(bottomBarSource, 'launchApp');

  assert.doesNotMatch(launchAppBody, /launchers\s*=\s*launchers\.filter/);
  assert.match(launchAppBody, /Launch unavailable/);
});

test('app-managed quick icon backend commands are no longer registered', () => {
  assert.doesNotMatch(mainRs, /mod quick_icons/);
  assert.doesNotMatch(mainRs, /quick_icons::/);
});

test('Explorer launcher context menu exposes Windows taskbar unpin only through validated lnk path', () => {
  const taskbarMenuRs = readFileSync(new URL('../src-tauri/src/taskbar_menu.rs', import.meta.url), 'utf8');

  assert.match(taskbarMenuRs, /LAUNCHER_MENU_PREFIX\}:unpin/);
  assert.match(taskbarMenuRs, /"Unpin from taskbar"/);
  assert.match(taskbarMenuRs, /"unpin"\s*=>\s*launchers::unpin_pinned_taskbar_app/);
  assert.match(launchersRs, /pub fn unpin_pinned_taskbar_app\(shortcut_path: String\) -> Result<\(\), String>/);
  assert.match(launchersRs, /let shortcut_path = validate_shortcut_path\(&shortcut_path\)\?/);
  assert.match(launchersRs, /fs::remove_file\(&shortcut_path\)/);
});

test('active task window context menu exposes PID lookup and taskbar pin actions', () => {
  const taskbarMenuRs = readFileSync(new URL('../src-tauri/src/taskbar_menu.rs', import.meta.url), 'utf8');
  const taskbarMenuTs = readFileSync(new URL('../src/lib/taskbarMenus.ts', import.meta.url), 'utf8');
  const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');
  const processManagerRust = readFileSync(new URL('../src-tauri/src/process_manager.rs', import.meta.url), 'utf8');
  const processManagerSurface = readFileSync(new URL('../src/components/ProcessManagerSurface.svelte', import.meta.url), 'utf8');

  assert.match(taskbarMenuTs, /processId: number \| null/);
  assert.match(bottomBarSource, /processId: normalizeTaskGalleryProcessId\(taskWindow\.processId\)/);
  assert.match(taskbarMenuRs, /"Pin to taskbar"/);
  assert.match(taskbarMenuRs, /launchers::can_pin_task_window_to_taskbar\(&request\.hwnd\)/);
  assert.match(taskbarMenuRs, /"pin"\s*=>\s*launchers::pin_task_window_to_taskbar/);
  assert.match(taskbarMenuRs, /PID \{pid\} - open in Process Manager/);
  assert.match(taskbarMenuRs, /process_manager::show_process_manager/);
  assert.match(processManagerRust, /pub focus_pid: Option<u32>/);
  assert.match(processManagerRust, /emit\(PROCESS_MANAGER_OPEN_EVENT, request\.focus_pid\)/);
  assert.match(processManagerSurface, /processFilter = String\(focusPid\)/);
  assert.match(processManagerSurface, /sortState = \{ column: 'pid', direction: 'asc' \}/);
});

test('active taskbar pin creates a taskbar shortcut instead of reviving app-managed quick icons', () => {
  const mainRs = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.doesNotMatch(mainRs, /quick_icons::/);
  assert.match(launchersRs, /pub fn pin_task_window_to_taskbar\(hwnd: String\) -> Result<\(\), String>/);
  assert.match(launchersRs, /crate::task_windows::task_window_process_path\(&hwnd\)/);
  assert.match(launchersRs, /fn taskbar_target_already_pinned\(target_path: &Path\) -> Result<bool, String>/);
  assert.match(launchersRs, /fn create_taskbar_shortcut\(target_path: &Path\) -> Result<\(\), String>/);
  assert.match(launchersRs, /\.SetPath\(PCWSTR\(target_wide\.as_ptr\(\)\)\)/);
  assert.match(launchersRs, /\.Save\(PCWSTR\(shortcut_wide\.as_ptr\(\)\), true\)/);
});
