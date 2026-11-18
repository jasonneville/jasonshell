import '../../../src/app.css';
import { mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import { mount, tick } from 'svelte';
import { surfaceComponentLoaders } from '../../../src/lib/surfaceLoader';
import { applyThemeToElement } from '../../../src/lib/themes';
import { createCustomTheme } from '../../../src/lib/themePalette';
import { context, processRow, snapshot } from '../../components/fixtures/system-info';
import type { AudioState } from '../../../src/lib/audio';

const query = new URLSearchParams(location.search);
const surface = query.get('surface');
if (surface !== 'audio-panel' && surface !== 'tray-panel' && surface !== 'process-manager') throw new Error('Unsupported audit surface');
let audio: AudioState = {
  masterVolumePercent: 42, defaultOutputDeviceId: 'desk', defaultInputDeviceId: 'mic',
  outputDevices: [{ id: 'desk', name: 'Desk speakers' }, { id: 'usb', name: 'USB headset' }],
  inputDevices: [{ id: 'mic', name: 'Desk microphone' }],
  sessions: [{ id: 'editor', name: 'Fixture editor', processId: 101, volumePercent: 35 }]
};
const iconDataUrl = `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><circle cx="8" cy="8" r="6" fill="dodgerblue"/></svg>')}`;
const icons = Array.from({ length: 12 }, (_, index) => ({ id: `icon-${index}`, commandId: index + 1, index, label: `Fixture notification ${index + 1}`, iconDataUrl, hasNativeIcon: true }));
let trayReads = 0;
let delayTrayRead = false;
let settleTrayRead: ((reject: boolean) => void) | undefined;
let releaseTrayRelay: (() => void) | undefined;
let delayTrayRelay = false;
const processes = [processRow(101, 'Fixture Editor'), processRow(102, 'Fixture Sync')];
let manager = context();
const calls: { command: string; args: unknown }[] = [];
const missing: string[] = [];
mockWindows(surface);
mockIPC((command, args) => {
  calls.push({ command, args });
  const data = args as { request?: { volumePercent: number; deviceId: string; id: string }; tab: string; openGeneration: number; tabRevision: number };
  switch (command) {
    case 'get_audio_state': return audio;
    case 'set_master_volume': return audio = { ...audio, masterVolumePercent: data.request!.volumePercent };
    case 'set_default_audio_output_device': return audio = { ...audio, defaultOutputDeviceId: data.request!.deviceId };
    case 'set_app_volume': return audio = { ...audio, sessions: audio.sessions.map(session => ({ ...session, volumePercent: data.request!.volumePercent })) };
    case 'list_system_tray_icons': {
      const revision = ++trayReads;
      const rows = icons.map((icon, index) => index === 11 ? { ...icon, label: `${icon.label} revision ${revision}` } : icon);
      if (!delayTrayRead) return rows;
      delayTrayRead = false;
      return new Promise<typeof rows>((resolve, reject) => {
        settleTrayRead = fail => fail ? reject(new Error('Stale fixture read rejected')) : resolve(rows);
      });
    }
    case 'invoke_system_tray_icon':
      if (data.request?.id === 'icon-1') throw new Error('Fixture relay rejected');
      if (delayTrayRelay) {
        delayTrayRelay = false;
        return new Promise<void>(resolve => { releaseTrayRelay = resolve; });
      }
      return;
    case 'get_process_manager_context': return manager;
    case 'list_processes': return processes;
    case 'list_taskbar_process_windows': return [];
    case 'set_process_manager_tab': return manager = context(data.openGeneration, data.tabRevision, data.tab);
    case 'get_system_info_snapshot': return snapshot(manager.openGeneration, manager.tabRevision);
    case 'hide_tray_panel': case 'hide_audio_panel': case 'hide_process_manager': return;
    default: missing.push(command); throw new Error(`Unmocked panel audit command: ${command}`);
  }
}, { shouldMockEvents: true });
const theme = query.get('theme');
const translucent = createCustomTheme('base-dark', 'Translucent renderer audit', 'custom:ui-audit-panels');
applyThemeToElement(theme === 'translucent' ? { ...translucent, palette: { ...translucent.palette,
  '--js-color-surface': '#10182699', '--js-color-surface-raised': '#26364bb3', '--js-color-surface-overlay': '#17253980'
} } : theme === 'light' ? 'base-light' : 'base-dark');
const { default: Component } = await surfaceComponentLoaders[surface]();
mount(Component, { target: document.getElementById('app')! });
await tick();
Object.assign(window, { audit: { calls, missing, emit,
  delayNextTrayRead: () => { delayTrayRead = true; },
  trayReadPending: () => Boolean(settleTrayRead),
  settleTrayRead: async (reject: boolean) => {
    if (!settleTrayRead) throw new Error('No pending mocked tray read');
    const settle = settleTrayRead;
    settleTrayRead = undefined;
    settle(reject);
    await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
    await tick();
  },
  delayNextTrayRelay: () => { delayTrayRelay = true; },
  releaseTrayRelay: () => {
    if (!releaseTrayRelay) throw new Error('No pending mocked tray relay');
    const release = releaseTrayRelay;
    releaseTrayRelay = undefined;
    release();
  }
} });
