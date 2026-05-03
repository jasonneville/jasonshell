import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const settingsPanelSource = readFileSync(new URL('../src/components/SettingsPanelSurface.svelte', import.meta.url), 'utf8');
const settingsPanelCss = readFileSync(new URL('../src/components/SettingsPanelSurface.css', import.meta.url), 'utf8');
const settingsPanelWrapper = readFileSync(new URL('../src/lib/settingsPanel.ts', import.meta.url), 'utf8');
const ipcCommandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const systemPowerSource = readFileSync(new URL('../src-tauri/src/system_power.rs', import.meta.url), 'utf8');

test('settings panel exposes three power actions behind in-panel confirmation', () => {
  for (const label of ['Sleep', 'Restart', 'Turn Off']) {
    assert.match(settingsPanelSource, new RegExp(`>${label}<`));
  }

  assert.match(settingsPanelSource, /power-heading/);
  assert.match(settingsPanelSource, /aria-labelledby="power-confirm-heading"/);
  assert.match(settingsPanelSource, /confirmPowerAction/);
  assert.match(settingsPanelSource, /cancelPowerAction/);
  assert.match(settingsPanelSource, /triggerSystemPowerAction/);
  assert.match(settingsPanelSource, /powerError/);
  assert.doesNotMatch(settingsPanelSource, /confirm\(/);
  assert.match(settingsPanelCss, /settings-power-actions/);
  assert.match(settingsPanelCss, /settings-power-confirm/);
});

test('settings power action wrapper accepts enum-only request shape', () => {
  assert.match(ipcCommandsSource, /triggerSystemPowerAction: 'trigger_system_power_action'/);
  assert.match(settingsPanelWrapper, /export type SystemPowerAction = 'sleep' \| 'restart' \| 'shutdown'/);
  assert.match(settingsPanelWrapper, /SystemPowerActionRequest/);
  assert.match(settingsPanelWrapper, /if \(!SYSTEM_POWER_ACTIONS\.includes\(request\.action\)\)/);
  assert.match(settingsPanelWrapper, /invoke\(IPC_COMMANDS\.triggerSystemPowerAction, \{ request \}\)/);
});

test('Rust power command validates enum and builds non-shell execution plans', () => {
  assert.match(mainSource, /mod system_power;/);
  assert.match(mainSource, /system_power::trigger_system_power_action/);
  assert.match(systemPowerSource, /enum SystemPowerAction/);
  assert.match(systemPowerSource, /Sleep/);
  assert.match(systemPowerSource, /Restart/);
  assert.match(systemPowerSource, /Shutdown/);
  assert.match(systemPowerSource, /PowerActionPlan/);
  assert.match(systemPowerSource, /std::process::Command::new\(plan\.program\)/);
  assert.doesNotMatch(systemPowerSource, /cmd\.exe/);
  assert.doesNotMatch(systemPowerSource, /powershell/i);
});
