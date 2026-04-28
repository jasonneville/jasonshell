import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  CURRENT_SETTINGS_VERSION,
  SETTINGS_COMMANDS,
  SETTINGS_SCHEMA,
  assertNoSecretSettingKeys,
  defaultShellSettings
} from '../dist-tests/lib/settings.js';
import {
  createDiagnosticsRingBuffer,
  redactDiagnosticFields,
  redactDiagnosticMessage
} from '../dist-tests/ipc/diagnostics.js';

const commandsSource = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const eventsSource = readFileSync(new URL('../src/ipc/events.ts', import.meta.url), 'utf8');
const surfacesSource = readFileSync(new URL('../src/ipc/surfaces.ts', import.meta.url), 'utf8');
const diagnosticsSource = readFileSync(new URL('../src/ipc/diagnostics.ts', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const capabilityDir = new URL('../src-tauri/capabilities/', import.meta.url);
const capabilitySources = Object.fromEntries(
  readdirSync(capabilityDir)
    .filter((name) => name.endsWith('.json'))
    .map((name) => [name, readFileSync(new URL(name, capabilityDir), 'utf8')])
);
const capabilitySource = Object.values(capabilitySources).join('\n');
const tauriConfigSource = readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8');
const wrapperSources = [
  'runtimeMetrics.ts',
  'searchPanel.ts',
  'systemSearch.ts',
  'processManager.ts',
  'stackPopup.ts',
  'taskbarLaunchers.ts',
  'taskbarMenus.ts',
  'taskbarPreview.ts',
  'taskbarWindows.ts'
].map((name) => readFileSync(new URL(`../src/lib/${name}`, import.meta.url), 'utf8'));

test('frontend IPC contracts expose command, event, and surface constants for future wrappers', () => {
  for (const command of [
    'show_search_panel',
    'stack_popup',
    'load_shell_settings',
    'save_shell_settings',
    'record_diagnostic',
    'export_diagnostics'
  ]) {
    assert.match(commandsSource, new RegExp(command));
  }

  for (const event of ['search-index:refreshed', 'stack-pins:updated', 'process-manager:closed']) {
    assert.match(eventsSource, new RegExp(event));
  }

  for (const surface of ['top-bar', 'bottom-bar', 'search-panel', 'stack-popup', 'process-manager']) {
    assert.match(surfacesSource, new RegExp(surface));
  }

  for (const source of wrapperSources) {
    assert.doesNotMatch(source, /invoke\('[-_a-z]+/);
    assert.match(source, /IPC_COMMANDS/);
  }
});

test('settings wrapper declares versioned schema and stable command names', () => {
  assert.equal(SETTINGS_SCHEMA, 'jasonshell.settings');
  assert.equal(CURRENT_SETTINGS_VERSION, 1);
  assert.deepEqual(SETTINGS_COMMANDS, {
    load: 'load_shell_settings',
    save: 'save_shell_settings'
  });
  assert.deepEqual(defaultShellSettings(), {
    schema: 'jasonshell.settings',
    version: 1,
    ui: {
      activeWorkspaceId: null,
      enableDiagnosticsExport: false
    },
    workspaces: [],
    taskHistory: []
  });
});

test('settings wrapper refuses secret-like keys before persistence', () => {
  assert.doesNotThrow(() => assertNoSecretSettingKeys(defaultShellSettings()));
  assert.doesNotThrow(() =>
    assertNoSecretSettingKeys({
      ...defaultShellSettings(),
      workspaces: [{ id: 'workspace-a', rootPath: 'C:\\dev\\jasonshell' }],
      taskHistory: [{ id: 'task-a', command: 'npm run validate', exitCode: 0 }]
    })
  );
  assert.throws(
    () => assertNoSecretSettingKeys({ integrations: { apiToken: 'abc' } }),
    /Settings must not store secret-like key: integrations\.apiToken/
  );
});

test('frontend diagnostics contract redacts sensitive diagnostic data', () => {
  assert.match(diagnosticsSource, /SECRET_KEY_PATTERN/);
  assert.match(diagnosticsSource, /SECRET_VALUE_PATTERN/);
  assert.match(diagnosticsSource, /createDiagnosticsRingBuffer/);
  assert.match(diagnosticsSource, /\[REDACTED\]/);

  assert.equal(redactDiagnosticMessage('failed Bearer abc.def'), 'failed Bearer [REDACTED]');
  assert.deepEqual(
    redactDiagnosticFields({
      nested: { apiToken: 'abc', text: 'Bearer nested-secret' },
      list: ['Bearer list-secret']
    }),
    {
      nested: { apiToken: '[REDACTED]', text: 'Bearer [REDACTED]' },
      list: ['Bearer [REDACTED]']
    }
  );

  const buffer = createDiagnosticsRingBuffer(1);
  buffer.record({
    level: 'warn',
    source: 'settings',
    message: 'first',
    fields: { safe: true },
    timestampEpochMs: 1
  });
  buffer.record({
    level: 'error',
    source: 'settings',
    message: 'second Bearer abc',
    fields: { password: 'secret' },
    timestampEpochMs: 2
  });
  assert.deepEqual(buffer.export().entries.map((entry) => entry.message), ['second Bearer [REDACTED]']);
  assert.deepEqual(buffer.export().entries[0].fields, { password: '[REDACTED]' });
});

test('backend settings and diagnostics commands are registered with hardened app config', () => {
  for (const command of [
    'settings::load_shell_settings',
    'settings::save_shell_settings',
    'diagnostics::record_diagnostic',
    'diagnostics::export_diagnostics'
  ]) {
    assert.match(mainSource, new RegExp(command.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }

  for (const surface of ['top-bar', 'bottom-bar', 'task-preview', 'search-panel', 'stack-popup', 'process-manager']) {
    assert.match(capabilitySource, new RegExp(surface));
  }
  assert.deepEqual(
    Object.values(capabilitySources).map((source) => JSON.parse(source).windows).sort((a, b) => a[0].localeCompare(b[0])),
    [['bottom-bar'], ['process-manager'], ['search-panel'], ['stack-popup'], ['task-preview'], ['top-bar']]
  );

  assert.doesNotMatch(tauriConfigSource, /"csp": null/);
  assert.match(tauriConfigSource, /default-src 'self'/);
  assert.match(tauriConfigSource, /"devCsp"/);
});
