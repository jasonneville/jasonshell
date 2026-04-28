import { invoke } from '@tauri-apps/api/core';

export const SETTINGS_SCHEMA = 'jasonshell.settings';
export const CURRENT_SETTINGS_VERSION = 1;

export interface ShellUiSettings {
  activeWorkspaceId: string | null;
  enableDiagnosticsExport: boolean;
}

export type ShellWorkspaceSettings = Record<string, unknown>;
export type ShellTaskHistoryEntry = Record<string, unknown>;

export interface ShellSettings {
  schema: typeof SETTINGS_SCHEMA;
  version: typeof CURRENT_SETTINGS_VERSION;
  ui: ShellUiSettings;
  workspaces: ShellWorkspaceSettings[];
  taskHistory: ShellTaskHistoryEntry[];
}

export const SETTINGS_COMMANDS = {
  load: 'load_shell_settings',
  save: 'save_shell_settings'
} as const;

export function defaultShellSettings(): ShellSettings {
  return {
    schema: SETTINGS_SCHEMA,
    version: CURRENT_SETTINGS_VERSION,
    ui: {
      activeWorkspaceId: null,
      enableDiagnosticsExport: false
    },
    workspaces: [],
    taskHistory: []
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
