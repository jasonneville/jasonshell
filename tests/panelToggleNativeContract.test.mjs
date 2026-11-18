import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

// Source-wiring contracts, NOT HWND execution or Windows focus-timing evidence.
const read = path => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8').replace(/\r\n/g, '\n');
const settings = read('src-tauri/src/settings_panel.rs');
const manager = read('src-tauri/src/process_manager.rs');
const main = read('src-tauri/src/main.rs');
const between = (source, start, end) => {
  const from = source.indexOf(start); const to = source.indexOf(end, from + start.length);
  assert.ok(from >= 0 && to > from, `Missing source boundary ${start} / ${end}`);
  return source.slice(from, to);
};

test('Settings toggle is opt-in, reads native visibility before show, and reuses capture-retiring hide', () => {
  assert.match(settings, /#\[serde\(default\)\]\s+pub toggle: bool/);
  const show = between(settings, 'pub fn show_settings_panel(', '#[tauri::command]\npub fn hide_settings_panel');
  assert.match(show, /if request\.toggle && panel\.is_visible\(\).*?\{\s*return hide_settings_panel\(app_handle\);/s);
  const ordinary = show.slice(show.indexOf('if request.toggle'));
  assert.ok(ordinary.indexOf('return hide_settings_panel') < ordinary.indexOf('.show()'));
  const hide = between(settings, 'pub fn hide_settings_panel(', 'pub(crate) fn install_capture_lifecycle');
  assert.ok(hide.indexOf('retire_settings_capture()') < hide.indexOf('.hide()'));
});

test('PM toggle authorizes origin, reads native visibility and hides before opening a new context', () => {
  assert.match(manager, /#\[serde\(default\)\]\s+pub toggle: bool/);
  const dispatch = between(manager, 'pub fn show_process_manager(', '/// Trusted native menu dispatcher');
  assert.match(dispatch, /same_registered_instance\(&window, BOTTOM_BAR_LABEL\)/);
  assert.match(dispatch, /if request\.toggle && request\.focus_pid\.is_none\(\)/);
  assert.match(dispatch, /popup\.is_visible\(\).*?return hide_process_manager_native\(app_handle\);/s);
  assert.ok(dispatch.indexOf('return hide_process_manager_native') < dispatch.indexOf('show_process_manager_native(app_handle, request)'));
  const hide = between(manager, 'fn hide_process_manager_native(', 'pub async fn list_processes(');
  assert.ok(hide.indexOf('retire_system_info(&app_handle)?') < hide.indexOf('.hide()'));
  assert.doesNotMatch(hide, /open_generation|\.open\(|show_process_manager_native/);
});

test('both native task-menu PID routes explicitly disable toggling', () => {
  const menu = read('src-tauri/src/taskbar_menu.rs');
  const requests = [...menu.matchAll(/ShowProcessManagerRequest\s*\{([^}]+)\}/g)];
  assert.equal(requests.length, 2);
  for (const [, request] of requests) {
    assert.match(request, /focus_pid: Some\(focus_pid\)/);
    assert.match(request, /toggle: false/);
  }
});

test('deferred panel blur binds concrete window and fences refocus/destroy with atomic revisions', () => {
  const lifecycle = between(main, '.on_window_event(|window, event| {', 'if window.label() == shell_windows::QUICK_LAUNCH_PANEL_LABEL');
  assert.match(lifecycle, /SETTINGS_PANEL_LABEL => Some\(&SETTINGS_BLUR_REVISION\)/);
  assert.match(lifecycle, /PROCESS_MANAGER_LABEL => Some\(&PROCESS_MANAGER_BLUR_REVISION\)/);
  assert.match(lifecycle, /WindowEvent::Focused\(_\) \| WindowEvent::Destroyed/);
  assert.match(lifecycle, /revision\.fetch_add\(1, std::sync::atomic::Ordering::SeqCst\)/);
  const delayed = lifecycle.slice(lifecycle.indexOf('if let (Some(revision), Some(expected))'));
  assert.match(delayed, /let panel = window\.clone\(\)/);
  assert.match(delayed, /Duration::from_millis\(150\)/);
  assert.ok(delayed.indexOf('run_on_main_thread(move ||') < delayed.indexOf('revision.load('));
  assert.match(delayed, /revision\.load\(std::sync::atomic::Ordering::SeqCst\) != expected\s*\|\| !panel\.is_visible\(\)\.unwrap_or\(false\)\s*\|\| panel\.is_focused\(\)\.unwrap_or\(false\)/);
  assert.ok(delayed.indexOf('runtime.holds_focus_loss') < delayed.indexOf('panel.hide()'));
  assert.doesNotMatch(delayed, /get_webview_window/);
});

test('PM work suspends immediately, while context retirement follows successful actual hide', () => {
  const delayed = between(main, 'if let (Some(revision), Some(expected))', 'if window.label() == shell_windows::QUICK_LAUNCH_PANEL_LABEL');
  assert.ok(delayed.indexOf('suspend_system_info(window.app_handle())') >= 0);
  assert.ok(delayed.indexOf('suspend_system_info(window.app_handle())') < delayed.indexOf('tauri::async_runtime::spawn('));
  assert.match(delayed, /if panel\.hide\(\)\.is_ok\(\) && panel\.label\(\) == shell_windows::PROCESS_MANAGER_LABEL\s*\{\s*let _ = process_manager::retire_system_info\(panel\.app_handle\(\)\)/);
});
