import { invoke } from '@tauri-apps/api/core';
import type { WorkspaceProfile } from './workspaces';
import { defaultSearchSettings } from './searchSettings.js';
import type { SearchMode, SearchSettingsContract } from './searchSettings';
import { defaultQuickCommandsSettings, type QuickCommandsSettings } from './quickCommands.js';

export const SETTINGS_SCHEMA = 'jasonshell.settings';
export const CURRENT_SETTINGS_VERSION = 1;
export const SHELL_SETTINGS_CHANNEL = 'jasonshell.settings.changed';
export const SHELL_SETTINGS_CHANGED_EVENT = 'jasonshell:settings-changed';

export interface ShellUiSettings {
  activeWorkspaceId: string | null;
  enableDiagnosticsExport: boolean;
  searchMode: SearchMode;
  lockTopBarHeight: boolean;
  lockBottomBarHeight: boolean;
  topBarHeightLogical: number;
  bottomBarHeightLogical: number;
}

export type ShellTaskHistoryEntry = Record<string, unknown>;

export interface ShellSettings {
  schema: typeof SETTINGS_SCHEMA;
  version: typeof CURRENT_SETTINGS_VERSION;
  ui: ShellUiSettings;
  search: SearchSettingsContract['search'];
  workspaces: WorkspaceProfile[];
  taskHistory: ShellTaskHistoryEntry[];
  quickCommands: QuickCommandsSettings;
}

export const SETTINGS_COMMANDS = {
  load: 'load_shell_settings',
  save: 'save_shell_settings'
} as const;

export function defaultShellSettings(): ShellSettings {
  const searchSettings = defaultSearchSettings();
  return {
    schema: SETTINGS_SCHEMA,
    version: CURRENT_SETTINGS_VERSION,
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
    quickCommands: defaultQuickCommandsSettings()
  };
}

export function hasSecretLikeSettingKey(key: string): boolean {
  return /(token|secret|password|credential|api[_-]?key|authorization|cookie)/iu.test(key);
}

export function assertNoSecretSettingKeys(value: unknown, path: string[] = []): void {
  if (!value || typeof value !== 'object') {
    return;
  }

  for (const [key, child] of Object.entries(value)) {
    const nextPath = [...path, key];
    if (hasSecretLikeSettingKey(key)) {
      throw new Error(`Settings must not store secret-like key: ${nextPath.join('.')}`);
    }
    assertNoSecretSettingKeys(child, nextPath);
  }
}

export async function loadShellSettings(): Promise<ShellSettings> {
  return invoke<ShellSettings>(SETTINGS_COMMANDS.load);
}

export async function saveShellSettings(settings: ShellSettings): Promise<ShellSettings> {
  assertNoSecretSettingKeys(settings);
  const saved = await invoke<ShellSettings>(SETTINGS_COMMANDS.save, { settings });
  broadcastShellSettings(saved);
  return saved;
}

export function addShellSettingsChangeListener(listener: (settings: ShellSettings) => void): () => void {
  if (typeof window === 'undefined') {
    return () => undefined;
  }
  const channel = browserBroadcastChannel();
  const handleChange = (event: Event) => {
    const detail = typeof event === 'object' && event && 'detail' in event
      ? (event as CustomEvent<ShellSettings>).detail
      : null;
    if (detail) {
      listener(detail);
    }
  };
  const handleStorage = (event: StorageEvent) => {
    if (event.key !== SHELL_SETTINGS_CHANNEL || !event.newValue) {
      return;
    }
    try {
      listener(JSON.parse(event.newValue) as ShellSettings);
    } catch (_error) {
      // Ignore malformed synthetic sync payloads.
    }
  };
  if (channel) {
    channel.onmessage = (event) => listener(event.data as ShellSettings);
  }
  window.addEventListener(SHELL_SETTINGS_CHANGED_EVENT, handleChange);
  window.addEventListener('storage', handleStorage);
  return () => {
    channel?.close();
    window.removeEventListener(SHELL_SETTINGS_CHANGED_EVENT, handleChange);
    window.removeEventListener('storage', handleStorage);
  };
}

function broadcastShellSettings(settings: ShellSettings): void {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent(SHELL_SETTINGS_CHANGED_EVENT, { detail: settings }));
    try {
      window.localStorage.setItem(SHELL_SETTINGS_CHANNEL, JSON.stringify(settings));
      window.localStorage.removeItem(SHELL_SETTINGS_CHANNEL);
    } catch (_error) {
      // Settings are already persisted by Rust; broadcast is best-effort UI sync.
    }
  }
  const channel = browserBroadcastChannel();
  try {
    channel?.postMessage(settings);
  } finally {
    channel?.close();
  }
}

function browserBroadcastChannel(): BroadcastChannel | null {
  if (typeof BroadcastChannel === 'undefined') {
    return null;
  }
  try {
    return new BroadcastChannel(SHELL_SETTINGS_CHANNEL);
  } catch (_error) {
    return null;
  }
}
