import { invoke } from '@tauri-apps/api/core';
import type { WorkspaceProfile } from './workspaces';
import { defaultSearchSettings } from './searchSettings.js';
import type { SearchMode, SearchSettingsContract } from './searchSettings';
import { defaultQuickCommandsSettings, type QuickCommandsSettings } from './quickCommands.js';
import type { StackTerminalProfile } from './stackPopup';

export const SETTINGS_SCHEMA = 'jasonshell.settings';
export const CURRENT_SETTINGS_VERSION = 1;

export interface ShellUiSettings {
  activeWorkspaceId: string | null;
  enableDiagnosticsExport: boolean;
  searchMode: SearchMode;
}

export type ShellTaskHistoryEntry = Record<string, unknown>;

export interface StackBrowserSettings {
  terminalProfile: StackTerminalProfile;
}

export interface ShellSettings {
  schema: typeof SETTINGS_SCHEMA;
  version: typeof CURRENT_SETTINGS_VERSION;
  ui: ShellUiSettings;
  search: SearchSettingsContract['search'];
  workspaces: WorkspaceProfile[];
  taskHistory: ShellTaskHistoryEntry[];
  quickCommands: QuickCommandsSettings;
  stackBrowser: StackBrowserSettings;
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
      searchMode: searchSettings.ui.searchMode
    },
    search: searchSettings.search,
    workspaces: [],
    taskHistory: [],
    quickCommands: defaultQuickCommandsSettings(),
    stackBrowser: defaultStackBrowserSettings()
  };
}

export function defaultStackBrowserSettings(): StackBrowserSettings {
  return {
    terminalProfile: 'windowsTerminal'
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
  return invoke<ShellSettings>(SETTINGS_COMMANDS.save, { settings });
}
