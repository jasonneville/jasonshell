import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('control-plane is routed as a hidden persistent shell surface with safe IPC', () => {
  const app = readFileSync('src/App.svelte', 'utf8');
  const shellSurface = readFileSync('src/lib/shellSurface.ts', 'utf8');
  const ipcSurfaces = readFileSync('src/ipc/surfaces.ts', 'utf8');
  const ipcCommands = readFileSync('src/ipc/commands.ts', 'utf8');
  const wrapper = readFileSync('src/lib/controlPlane.ts', 'utf8');
  const shellWindows = readFileSync('src-tauri/src/shell_windows.rs', 'utf8');
  const main = readFileSync('src-tauri/src/main.rs', 'utf8');
  const controlPlane = readFileSync('src-tauri/src/control_plane.rs', 'utf8');
  const contracts = readFileSync('src-tauri/src/contracts.rs', 'utf8');
  const capability = JSON.parse(readFileSync('src-tauri/capabilities/control-plane.json', 'utf8'));

  assert.match(app, /ControlPlaneSurface/);
  assert.match(app, /surface === 'control-plane'/);
  assert.match(shellSurface, /'control-plane'/);
  assert.match(ipcSurfaces, /controlPlane: 'control-plane'/);
  assert.match(ipcCommands, /showControlPlane: 'show_control_plane'/);
  assert.match(ipcCommands, /hideControlPlane: 'hide_control_plane'/);
  assert.match(wrapper, /invoke\(IPC_COMMANDS\.showControlPlane\)/);
  assert.match(wrapper, /invoke\(IPC_COMMANDS\.hideControlPlane\)/);
  assert.match(shellWindows, /CONTROL_PLANE_LABEL: &str = "control-plane"/);
  assert.match(shellWindows, /build_control_plane_window\(app\)/);
  const controlPlaneWindow = shellWindows.match(/fn build_control_plane_window[\s\S]+?\.build\(\)\?\)/)?.[0] ?? '';
  assert.match(controlPlaneWindow, /\.visible\(false\)/);
  assert.match(controlPlaneWindow, /\.skip_taskbar\(true\)/);
  assert.match(main, /mod control_plane;/);
  assert.match(main, /control_plane::show_control_plane/);
  assert.match(main, /control_plane::hide_control_plane/);
  assert.match(contracts, /SHOW_CONTROL_PLANE/);
  assert.match(contracts, /HIDE_CONTROL_PLANE/);
  assert.deepEqual(capability.windows, ['control-plane']);
  assert.deepEqual(capability.permissions, ['core:default', 'core:window:default']);
  assert.doesNotMatch(controlPlane, /kill_process|TerminateProcess|ShellExecute|spawn_workspace_task|parse_automation_cli/);
});
