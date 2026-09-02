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
import { defaultSearchSettings } from '../dist-tests/lib/searchSettings.js';
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
const rustContractsSource = readFileSync(new URL('../src-tauri/src/contracts.rs', import.meta.url), 'utf8');
const shellWindowsSource = readFileSync(new URL('../src-tauri/src/shell_windows.rs', import.meta.url), 'utf8');
const shellSurfaceSource = readFileSync(new URL('../src/lib/shellSurface.ts', import.meta.url), 'utf8');
const appSource = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const surfaceLoaderSource = readFileSync(new URL('../src/lib/surfaceLoader.ts', import.meta.url), 'utf8');
const capabilityDir = new URL('../src-tauri/capabilities/', import.meta.url);
const capabilitySources = Object.fromEntries(
  readdirSync(capabilityDir)
    .filter((name) => name.endsWith('.json'))
    .map((name) => [name, readFileSync(new URL(name, capabilityDir), 'utf8')])
);
const capabilitySource = Object.values(capabilitySources).join('\n');
const tauriConfigSource = readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8');
const tauriConfig = JSON.parse(tauriConfigSource);
const wrapperSources = [
  'runtimeMetrics.ts',
  'audio.ts',
  'commandPanel.ts',
  'quickCommands.ts',
  'trayPanel.ts',
  'controlPlane.ts',
  'settingsPanel.ts',
  'shellBarResize.ts',
  'devTools.ts',
  'searchPanel.ts',
  'processManager.ts',
  'stackPopup.ts',
  'taskbarLaunchers.ts',
  'taskbarMenus.ts',
  'taskbarPreview.ts',
  'taskbarWindows.ts'
].map((name) => readFileSync(new URL(`../src/lib/${name}`, import.meta.url), 'utf8'));

function uniqueSorted(values) {
  return [...new Set(values)].sort();
}

function regexCaptureAll(source, regex) {
  return uniqueSorted([...source.matchAll(regex)].map((match) => match[1]));
}

function parseShellWindowLabels(source) {
  return regexCaptureAll(source, /pub const [A-Z_]+_LABEL: &str = "([^"]+)"/g);
}

function parseShellSurfaceTypeLabels(source) {
  const typeMatch = source.match(/export type ShellSurface =([\s\S]*?);/);
  assert.ok(typeMatch, 'src/lib/shellSurface.ts must export ShellSurface union');
  return uniqueSorted(regexCaptureAll(typeMatch[1], /\| '([^']+)'/g).filter((label) => label !== 'unknown'));
}

function parseIpcSurfaceLabels(source) {
  const objectMatch = source.match(/export const SHELL_SURFACES = \{([\s\S]*?)\} as const;/);
  assert.ok(objectMatch, 'src/ipc/surfaces.ts must export SHELL_SURFACES');
  return regexCaptureAll(objectMatch[1], /:\s*'([^']+)'/g);
}

function parseAppSurfaceRoutes(source) {
  assert.match(appSource, /loadSurfaceComponent\(surface\)/);
  return regexCaptureAll(source, /'([^']+)': \(\) => import\('\.\.\/components\/[A-Za-z]+\.svelte'\)/g);
}

function parseRustSurfaceContractLabels(source) {
  const moduleMatch = source.match(/pub mod surfaces \{([\s\S]*?)\npub mod commands/);
  assert.ok(moduleMatch, 'src-tauri/src/contracts.rs must define surfaces module before commands module');
  return regexCaptureAll(moduleMatch[1], /pub const [A-Z_]+: &str = "([^"]+)"/g);
}

function parseIpcEventNames(source) {
  const objectMatch = source.match(/export const IPC_EVENTS = \{([\s\S]*?)\} as const;/);
  assert.ok(objectMatch, 'src/ipc/events.ts must export IPC_EVENTS');
  return regexCaptureAll(objectMatch[1], /:\s*'([^']+)'/g);
}

function parseRustEventContractLabels(source) {
  const moduleMatch = source.match(/pub mod events \{([\s\S]*?)\n\}/);
  assert.ok(moduleMatch, 'src-tauri/src/contracts.rs must define events module');
  return regexCaptureAll(moduleMatch[1], /pub const [A-Z_]+: &str = "([^"]+)"/g);
}

function parseCapabilityWindows(sources) {
  const labelsToFiles = new Map();

  for (const [name, source] of Object.entries(sources)) {
    const parsed = JSON.parse(source);
    for (const label of parsed.windows ?? []) {
      labelsToFiles.set(label, [...(labelsToFiles.get(label) ?? []), `src-tauri/capabilities/${name}`]);
    }
  }

  return labelsToFiles;
}

function missingFrom(expected, actual) {
  return expected.filter((label) => !actual.includes(label));
}

function formatMissingSurfaceLabels(entries) {
  return entries
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => `${entry.file}: ${entry.missing.join(', ')}`)
    .join('\n');
}

test('frontend IPC contracts expose command, event, and surface constants for future wrappers', () => {
  for (const command of [
    'show_search_panel',
    'search_engine',
    'stack_popup',
    'load_shell_settings',
    'save_shell_settings',
    'get_audio_state',
    'set_master_volume',
    'set_app_volume',
    'set_default_audio_input_device',
    'set_default_audio_output_device',
    'list_workspaces',
    'activate_workspace',
    'build_terminal_launch_plan',
    'spawn_workspace_task',
    'get_workspace_git_status',
    'record_diagnostic',
    'export_diagnostics',
    'run_quick_command',
    'resize_stack_terminal',
    'show_control_plane',
    'hide_control_plane',
    'resize_shell_bar',
    'save_shell_bar_height',
    'save_shell_bar_lock'
  ]) {
    assert.match(commandsSource, new RegExp(command));
  }

  for (const event of ['search-index:refreshed', 'stack-pins:updated', 'process-manager:closed', 'task:output']) {
    assert.match(eventsSource, new RegExp(event));
  }

  for (const surface of [
    'top-bar',
    'bottom-bar',
    'search-panel',
    'stack-popup',
    'process-manager',
    'quick-launch-panel',
    'control-plane',
    'tray-panel',
    'command-panel',
    'audio-panel'
  ]) {
    assert.match(surfacesSource, new RegExp(surface));
  }

  for (const source of wrapperSources) {
    assert.doesNotMatch(source, /invoke\('[-_a-z]+/);
    assert.match(source, /IPC_COMMANDS/);
  }
});

test('event registry documents subset authority and excludes removed audio refresh event', () => {
  assert.match(eventsSource, /Convenience subset/);
  assert.match(eventsSource, /Rust event\s*\r?\n?\s*\/\/ authority lives in src-tauri\/src\/contracts\.rs/);
  assert.doesNotMatch(eventsSource, /audioRefresh: 'audio:refresh'/);
  assert.doesNotMatch(rustContractsSource, /AUDIO_REFRESH|audio:refresh/);
});

test('shipped shell window surfaces have matching frontend routes, IPC registry entries, and capability targets', () => {
  const shippedLabels = parseShellWindowLabels(shellWindowsSource);
  const shellSurfaceLabels = parseShellSurfaceTypeLabels(shellSurfaceSource);
  const ipcSurfaceLabels = parseIpcSurfaceLabels(surfacesSource);
  const appRouteLabels = parseAppSurfaceRoutes(surfaceLoaderSource);
  const rustContractLabels = parseRustSurfaceContractLabels(rustContractsSource);
  const capabilityWindows = parseCapabilityWindows(capabilitySources);
  const capabilityLabels = uniqueSorted([...capabilityWindows.keys()]);

  const missingEntries = [
    {
      file: 'src/lib/shellSurface.ts',
      missing: missingFrom(shippedLabels, shellSurfaceLabels)
    },
    {
      file: 'src/ipc/surfaces.ts',
      missing: missingFrom(shippedLabels, ipcSurfaceLabels)
    },
    {
      file: 'src/App.svelte',
      missing: missingFrom(shippedLabels, appRouteLabels)
    },
    {
      file: 'src-tauri/src/contracts.rs',
      missing: missingFrom(shippedLabels, rustContractLabels)
    },
    {
      file: 'src-tauri/capabilities/*.json',
      missing: missingFrom(shippedLabels, capabilityLabels)
    }
  ];
  const missingDetails = formatMissingSurfaceLabels(missingEntries);

  assert.equal(
    missingDetails,
    '',
    `Missing shipped shell surface labels/files:\n${missingDetails}\n\nShipped labels: ${shippedLabels.join(', ')}`
  );

  for (const label of shippedLabels) {
    assert.ok(
      capabilityWindows.get(label)?.length,
      `Missing capability file targeting shipped shell surface label "${label}"`
    );
  }
});

test('surface parity failure output names future missing capability targets', () => {
  assert.equal(
    formatMissingSurfaceLabels([
      {
        file: 'src-tauri/capabilities/*.json',
        missing: missingFrom(['future-panel'], [])
      }
    ]),
    'src-tauri/capabilities/*.json: future-panel'
  );
});

test('Rust event contracts are authoritative and cover frontend event constants', () => {
  const rustEvents = parseRustEventContractLabels(rustContractsSource);
  const ipcEvents = parseIpcEventNames(eventsSource);

  assert.match(
    eventsSource,
    /Convenience subset[\s\S]*Rust event\s*\r?\n?\s*\/\/ authority lives in src-tauri\/src\/contracts\.rs/,
    'src/ipc/events.ts must document that it is a convenience subset, not the exhaustive event registry'
  );

  const missingIpcEvents = missingFrom(ipcEvents, rustEvents);
  assert.deepEqual(
    missingIpcEvents,
    [],
    `src/ipc/events.ts contains events missing from Rust authority: ${missingIpcEvents.join(', ')}`
  );

  const expectedRuntimeEvents = [
    'audio-panel:open',
    'audio-panel:closed',
    'calendar-panel:open',
    'calendar-panel:closed',
    'command-panel:closed',
    'process-manager:open',
    'process-manager:closed',
    'quick-command:run-updated',
    'quick-launch-panel:closed',
    'quick-launch-panel:open',
    'search:toggle-centered',
    'search-engine:progress',
    'search-index:refreshed',
    'search-panel:activate',
    'search-panel:closed',
    'search-panel:expand-group',
    'search-panel:interaction',
    'search-panel:key',
    'search-panel:pin-folder',
    'search-panel:query',
    'search-panel:select',
    'search-panel:update',
    'stack-browser:toggle',
    'stack-popup:closed',
    'stack-popup:open',
    'stack-terminal:closed',
    'stack-terminal:cwd',
    'stack-terminal:output',
    'stack-pins:updated',
    'task-gallery:closed',
    'task-gallery:open',
    'task-preview:hide',
    'task-preview:hover-enter',
    'task-preview:update',
    'task:completed',
    'task:output',
    'task:started',
    'taskbar:refresh-launchers',
    'taskbar:refresh-windows',
    'taskbar:windows-snapshot',
    'terminal-panel:closed',
    'terminal-panel:open',
    'top-bar:pin-menu-action',
    'tray-panel:closed',
    'tray-panel:open'
  ].sort();

  assert.deepEqual(
    rustEvents,
    expectedRuntimeEvents,
    'contracts::events::ALL must remain the exhaustive Tauri shell runtime event registry'
  );
});

test('dead audio refresh event contract is not exposed without a Rust emitter', () => {
  const audioWrapper = readFileSync(new URL('../src/lib/audio.ts', import.meta.url), 'utf8');
  const audioPanelSource = readFileSync(new URL('../src/components/AudioPanelSurface.svelte', import.meta.url), 'utf8');
  const audioRustSource = readFileSync(new URL('../src-tauri/src/audio_panel.rs', import.meta.url), 'utf8');
  const joinedSource = [eventsSource, audioWrapper, audioPanelSource, audioRustSource].join('\n');

  assert.doesNotMatch(joinedSource, /audio:refresh|AUDIO_REFRESH_EVENT|AudioRefreshPayload/);
});

test('settings wrapper declares versioned schema and stable command names', () => {
  const searchSettings = defaultSearchSettings();

  assert.equal(SETTINGS_SCHEMA, 'jasonshell.settings');
  assert.equal(CURRENT_SETTINGS_VERSION, 1);
  assert.deepEqual(SETTINGS_COMMANDS, {
    load: 'load_shell_settings',
    save: 'save_shell_settings',
    saveShellBarHeight: 'save_shell_bar_height',
    saveShellBarLock: 'save_shell_bar_lock'
  });
  assert.deepEqual(defaultShellSettings(), {
    schema: 'jasonshell.settings',
    version: 1,
    ui: {
      activeWorkspaceId: null,
      enableDiagnosticsExport: false,
      searchMode: searchSettings.ui.searchMode,
      lockTopBarHeight: true,
      lockBottomBarHeight: true,
      topBarHeightLogical: 23.4,
      bottomBarHeightLogical: 32.4
    },
    search: searchSettings.search,
    workspaces: [],
    taskHistory: [],
    quickCommands: {
      entries: [],
      history: [],
      listWidth: 180
    },
    stackBrowser: {
      terminalProfile: 'windowsTerminal'
    }
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
    'diagnostics::export_diagnostics',
    'quick_commands::run_quick_command',
    'command_panel::show_command_panel',
    'command_panel::hide_command_panel',
    'build_terminal_launch_plan',
    'spawn_workspace_task',
    'get_workspace_git_status'
  ]) {
    assert.match(mainSource, new RegExp(command.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  }

  for (const surface of [
    'top-bar',
    'bottom-bar',
    'task-preview',
    'search-panel',
    'stack-popup',
    'process-manager',
    'control-plane',
    'command-panel',
    'settings-panel',
    'tray-panel',
    'audio-panel',
    'calendar-panel',
    'terminal-panel'
  ]) {
    assert.match(capabilitySource, new RegExp(surface));
  }
  assert.deepEqual(
    Object.values(capabilitySources).map((source) => JSON.parse(source).windows).sort((a, b) => a[0].localeCompare(b[0])),
    [
      ['audio-panel'],
      ['bottom-bar'],
      ['calendar-panel'],
      ['command-panel'],
      ['control-plane'],
      ['process-manager'],
      ['search-panel'],
      ['settings-panel'],
      ['stack-popup'],
      ['task-gallery'],
      ['task-preview'],
      ['terminal-panel'],
      ['top-bar', 'quick-launch-panel'],
      ['tray-panel']
    ]
  );

  assert.doesNotMatch(tauriConfigSource, /"csp": null/);
  assert.match(tauriConfigSource, /default-src 'self'/);
  assert.match(tauriConfigSource, /"devCsp"/);

  for (const [name, csp] of Object.entries({
    csp: tauriConfig.app.security.csp,
    devCsp: tauriConfig.app.security.devCsp
  })) {
    assert.match(csp, /style-src[^;]*'self'[^;]*https:\/\/fonts\.googleapis\.com/, `${name} must allow Google Fonts CSS stylesheets`);
    assert.match(csp, /font-src[^;]*'self'[^;]*https:\/\/fonts\.gstatic\.com/, `${name} must allow Google Fonts font files`);
  }
});
