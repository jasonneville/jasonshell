import '../../../src/app.css';
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import { mount, tick } from 'svelte';
import { surfaceComponentLoaders } from '../../../src/lib/surfaceLoader';
import { defaultShellSettings } from '../../../src/lib/settings';
import { applyThemeToElement } from '../../../src/lib/themes';
import { createCustomTheme } from '../../../src/lib/themePalette';
import type { AudioState } from '../../../src/lib/audio';
import type { SpeechHistoryEntry } from '../../../src/ipc/events';
import type { SystemTrayIconSnapshot } from '../../../src/lib/systemTray';

const query = new URLSearchParams(location.search);
const label = query.get('surface') as keyof typeof surfaceComponentLoaders;
if (!Object.hasOwn(surfaceComponentLoaders, label)) throw new Error(`Unknown surface: ${label}`);
const calls: string[] = [];
const missing = new Set<string>();
const requests: { command: string; args: unknown }[] = [];
const populated = query.has('populated');
let audio: AudioState = { masterVolumePercent: 42, inputDevices: [], outputDevices: [], sessions: [] };
if (populated) audio = { ...audio, outputDevices: [{ id: 'desk', name: 'Desk speakers' }, { id: 'headset', name: 'USB headset' }], inputDevices: [{ id: 'mic', name: 'Desk microphone' }], defaultOutputDeviceId: 'desk', defaultInputDeviceId: 'mic', sessions: [{ id: 'editor', name: 'Fixture editor', processId: 101, volumePercent: 35 }] };
const icons: SystemTrayIconSnapshot[] = ['Fixture sync', 'Fixture battery'].map((name, index) => ({ id: `icon-${index}`, commandId: index + 1, index, label: name, iconDataUrl: `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><circle cx="8" cy="8" r="6" fill="dodgerblue"/></svg>')}` }));
const history: SpeechHistoryEntry[] = [{ nonce: 1, transcript: 'Fixture transcript: the selected microphone picks up this sentence. A second sentence exercises wrapping at narrow widths.', outcome: 'copied' }];
mockWindows(label);
mockIPC((command, args) => {
  calls.push(command);
  requests.push({ command, args });
  switch (command) {
    case 'load_shell_settings': return query.has('commands') ? { ...defaultShellSettings(), quickCommands: { orderVersion: 1, listWidth: 180, history: [], entries: [{ id: 'interactive', label: 'Interactive fixture', mode: 'direct', targetPath: 'C:/fixture/mock.exe', args: [], commands: [], cwd: null }] } } : defaultShellSettings();
    case 'get_speech_history': return populated ? history : [];
    case 'list_system_tray_icons': return populated ? icons : [];
    case 'list_quick_command_history':
    case 'list_processes': case 'list_open_task_windows': case 'list_pinned_taskbar_apps':
    case 'list_pinned_stack_folders': case 'list_workspaces': case 'list_stack_terminals': return [];
    case 'get_audio_state': return audio;
    case 'set_default_audio_output_device': {
      const { deviceId } = (args as { request: { deviceId: string } }).request;
      return audio = { ...audio, defaultOutputDeviceId: deviceId };
    }
    case 'set_master_volume': {
      const { volumePercent } = (args as { request: { volumePercent: number } }).request;
      return audio = { ...audio, masterVolumePercent: volumePercent };
    }
    case 'hide_command_panel': case 'hide_calendar_panel': case 'report_shell_surface_runtime_metrics': return;
    case 'hide_context_menu_overlay': case 'resize_shell_bar': case 'hide_task_gallery_on_focus_loss':
    case 'hide_quick_launch_panel_on_focus_loss': case 'hide_quick_launch_panel': return;
    case 'get_search_panel_payload': return { query: 'fixture', presentation: 'centered', selectedIndex: 0, results: [
      { id: 'fixture-app', kind: 'app', providerId: 'apps', title: 'Fixture application', subtitle: 'Mocked application' },
      { id: 'fixture-folder', kind: 'folder', providerId: 'local', title: 'Fixture folder', subtitle: 'Mocked local folder', path: 'C:\\fixture' }
    ] };
    case 'resize_search_panel': return;
    default: missing.add(command); throw new Error(`Fixture has no response for ${command}`);
  }
}, { shouldMockEvents: true });
const theme = query.get('theme');
const custom = createCustomTheme('base-dark', 'Translucent renderer audit', 'custom:ui-audit');
applyThemeToElement(theme === 'translucent' ? { ...custom, palette: { ...custom.palette,
  '--js-color-surface': '#10182699', '--js-color-surface-raised': '#26364bb3', '--js-color-surface-overlay': '#17253980'
} } : theme === 'light' ? 'base-light' : 'base-dark');
const { default: Component } = await surfaceComponentLoaders[label]();
mount(Component, { target: document.getElementById('app')! });
await tick();
Object.assign(window, { audit: { calls, requests, emit, missing: () => [...missing] } });
