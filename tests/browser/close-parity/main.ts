import '../../../src/app.css';
import { mockIPC } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import { mount } from 'svelte';
import Calendar from '../../../src/components/CalendarPanelSurface.svelte';
import Audio from '../../../src/components/AudioPanelSurface.svelte';
import Tray from '../../../src/components/TrayPanelSurface.svelte';
import Speech from '../../../src/components/SpeechHistoryPanelSurface.svelte';
import StackTerminal from '../../../src/components/StackTerminalPane.svelte';
import Terminal from '../../../src/components/TerminalPanelSurface.svelte';
import { applyThemeToElement } from '../../../src/lib/themes';
import { createCustomTheme } from '../../../src/lib/themePalette';

const panels = { calendar: Calendar, audio: Audio, tray: Tray, speech: Speech };
const query = new URLSearchParams(location.search);
const panel = query.get('panel') ?? 'calendar';
const ledger: string[] = [];
const session = { sessionId: 'close-fixture', cwd: 'C:\\fixture', profile: 'powershell', title: 'PowerShell', running: true };
mockIPC(command => {
  ledger.push(command);
  switch (command) {
    case 'get_audio_state': return { masterVolumePercent: 42, inputDevices: [], outputDevices: [], sessions: [] };
    case 'get_speech_history': case 'list_system_tray_icons': return [];
    case 'hide_audio_panel': case 'hide_calendar_panel': case 'hide_tray_panel': case 'hide_speech_history_panel': return;
    case 'list_stack_terminals': return [];
    case 'start_persistent_terminal': case 'start_stack_terminal': return session;
    case 'poll_stack_terminal_session': return { ...session, chunks: [{ sequence: 1, text: 'find me\r\n', stream: 'stdout' }] };
    case 'resize_stack_terminal': return;
    default: throw new Error(`Unexpected mock IPC: ${command}`);
  }
}, { shouldMockEvents: true });

const theme = query.get('theme');
const custom = createCustomTheme('base-dark', 'Close parity translucent', 'custom:close-parity');
applyThemeToElement(theme === 'translucent' ? {
  ...custom, palette: { ...custom.palette, '--js-color-surface-overlay': '#10182699', '--js-color-surface-raised': '#26364bb3' }
} : theme === 'light' ? 'base-light' : 'base-dark');
const target = document.getElementById('app')!;
if (panel === 'stack-terminal') {
  const pane = mount(StackTerminal, { target, props: { currentPath: session.cwd } });
  void pane.startTerminal(true);
} else if (panel === 'terminal') {
  mount(Terminal, { target });
  Object.assign(window, { openFixtureTerminal: () => emit('terminal-panel:open') });
} else {
  if (!Object.hasOwn(panels, panel)) throw new Error(`Unknown fixture panel: ${panel}`);
  mount(panels[panel as keyof typeof panels], { target });
}
Object.assign(window, { closeParityLedger: ledger });
