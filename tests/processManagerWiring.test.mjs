import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

test('process manager surface and commands are routed through app and Rust command table', () => {
  const app = readFileSync('src/App.svelte', 'utf8');
  const surfaces = readFileSync('src/lib/shellSurface.ts', 'utf8');
  const shellWindows = readFileSync('src-tauri/src/shell_windows.rs', 'utf8');
  const main = readFileSync('src-tauri/src/main.rs', 'utf8');
  const bottomBar = readFileSync('src/components/BottomBar.svelte', 'utf8');
  const processSurface = readFileSync('src/components/ProcessManagerSurface.svelte', 'utf8');
  const processState = readFileSync('src/lib/processManagerState.ts', 'utf8');
  const processManager = readFileSync('src-tauri/src/process_manager.rs', 'utf8');

  assert.match(app, /ProcessManagerSurface/);
  assert.match(surfaces, /'process-manager'/);
  assert.match(shellWindows, /PROCESS_MANAGER_LABEL/);
  assert.match(main, /process_manager::show_process_manager/);
  assert.match(main, /process_manager::list_processes/);
  assert.match(main, /process_manager::kill_process/);
  assert.match(bottomBar, /showProcessManager/);
  assert.match(bottomBar, /process-manager-button/);
  assert.match(processSurface, /sortBy\('startTimeMs'\)/);
  assert.match(processSurface, /formatProcessStartTime\(process\.startTimeMs\)/);
  assert.match(processState, /startTimeMs/);
  assert.match(processManager, /start_time_ms: process_handle\.and_then\(process_start_time_ms\)/);
});
