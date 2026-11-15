import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands';
import { broadcastShellSettings, type ShellSettings, type StandardHotkeyAction, type StandardHotkeySettings } from './settings';

export const HOTKEY_ACTION_NAMES: Record<StandardHotkeyAction, string> = {
  search: 'Search', terminal: 'Terminal', stackBrowser: 'Stack Browser',
  speechTranscription: 'Speech transcription', snipping: 'Screen snipping',
  processManager: 'Process Manager', settings: 'Settings'
};
export const HOTKEY_ACTIONS = Object.keys(HOTKEY_ACTION_NAMES) as StandardHotkeyAction[];

export interface StandardHotkeyStatus {
  revision: number;
  bindings: { action: StandardHotkeyAction; id: number; binding: string; active: boolean; reason: string | null; state: 'active' | 'inactive' | 'capturePaused' }[];
}
export interface SaveStandardHotkeysResponse {
  committed: true;
  settings: ShellSettings;
  status: StandardHotkeyStatus | null;
  statusError: string | null;
}
export interface HotkeyCaptureLease {
  leaseId: string;
  revision: number;
  sequence: number;
  expiresInMs: number;
  registeredBindings: string[];
}
export interface HotkeyCaptureEvent {
  leaseId: string;
  revision: number;
  sequence: number;
  action: StandardHotkeyAction;
  binding: string;
}
export function isCanonicalHotkey(binding: unknown): binding is string {
  return typeof binding === 'string' && /^(Ctrl|Alt)\+([A-Z0-9]|Space|Backquote)$/.test(binding);
}
export function hotkeyDraftError(hotkeys: StandardHotkeySettings): string {
  for (const action of HOTKEY_ACTIONS) {
    if (!isCanonicalHotkey(hotkeys[action])) return `Invalid ${HOTKEY_ACTION_NAMES[action]} shortcut. Use Ctrl or Alt with a letter, number, Space, or Backquote.`;
    const duplicate = HOTKEY_ACTIONS.find((other) => other !== action && hotkeys[other] === hotkeys[action]);
    if (duplicate) return `Duplicate shortcut: ${hotkeys[action]} is assigned to ${HOTKEY_ACTION_NAMES[action]} and ${HOTKEY_ACTION_NAMES[duplicate]}. Change either shortcut before saving.`;
  }
  return '';
}
export function getStandardHotkeyStatus(): Promise<StandardHotkeyStatus> {
  return invoke(IPC_COMMANDS.getStandardHotkeyStatus);
}
export async function saveStandardHotkeys(hotkeys: StandardHotkeySettings, revision: number) {
  const result = await invoke<SaveStandardHotkeysResponse>(IPC_COMMANDS.saveStandardHotkeys, { request: { hotkeys, revision } });
  try {
    broadcastShellSettings(result.settings);
  } catch {
    // Renderer synchronization cannot turn an already committed Save into failure.
  }
  return result;
}
export function beginStandardHotkeyCapture(revision: number): Promise<HotkeyCaptureLease> {
  return invoke(IPC_COMMANDS.beginStandardHotkeyCapture, { request: { revision } });
}
export function renewStandardHotkeyCapture(lease: HotkeyCaptureLease): Promise<HotkeyCaptureLease> {
  return invoke(IPC_COMMANDS.renewStandardHotkeyCapture, { request: { leaseId: lease.leaseId, revision: lease.revision } });
}
export function endStandardHotkeyCapture(lease: HotkeyCaptureLease): Promise<void> {
  return invoke(IPC_COMMANDS.endStandardHotkeyCapture, { request: { leaseId: lease.leaseId, revision: lease.revision } });
}
