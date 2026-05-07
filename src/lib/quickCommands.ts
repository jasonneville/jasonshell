import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const QUICK_COMMAND_MODES = ['direct', 'commandBlock'] as const;
export type QuickCommandMode = (typeof QUICK_COMMAND_MODES)[number];

export interface QuickCommandEntry {
  id: string;
  label: string;
  mode: QuickCommandMode;
  targetPath: string;
  args: string[];
  commands: string[];
  cwd: string | null;
}

export interface QuickCommandsSettings {
  entries: QuickCommandEntry[];
}

export interface RunQuickCommandRequest {
  id: string;
}

export interface QuickCommandSpawnResult {
  processId: number;
}

type ShellSettingsRecord = Record<string, unknown> & {
  quickCommands?: unknown;
};

const DEFAULT_QUICK_COMMANDS_SETTINGS: QuickCommandsSettings = {
  entries: []
};

const SECRET_KEY_PATTERN = /(token|secret|password|credential|api[_-]?key|authorization|cookie)/iu;
const SECRET_VALUE_PATTERN = /\b(?:bearer\s+\S+|ghp_[A-Za-z0-9_.-]+|gho_[A-Za-z0-9_.-]+|github_pat_[A-Za-z0-9_.-]+|xoxb-[A-Za-z0-9_.-]+|sk-[A-Za-z0-9_.-]+|akia[A-Za-z0-9_.-]*)\b/iu;

export function defaultQuickCommandsSettings(): QuickCommandsSettings {
  return {
    entries: []
  };
}

export function coerceQuickCommandsSettings(value: unknown): QuickCommandsSettings {
  const record = asRecord(value);
  const rawEntries = Array.isArray(record?.entries) ? record.entries : [];
  const entries = rawEntries
    .map((entry, index) => coerceQuickCommandEntry(entry, index))
    .filter((entry): entry is QuickCommandEntry => entry !== null);

  const seen = new Set<string>();
  for (const entry of entries) {
    if (seen.has(entry.id)) {
      throw new Error(`Quick command id must be unique: ${entry.id}`);
    }
    seen.add(entry.id);
  }
  return { entries };
}

export function parseQuickCommandArgsTextarea(value: string): string[] {
  return value
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

export function formatQuickCommandArgsTextarea(args: readonly string[]): string {
  return args.map((value) => value.trim()).filter(Boolean).join('\n');
}

export function parseQuickCommandCommandsTextarea(value: string): string[] {
  return value
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

export function formatQuickCommandCommandsTextarea(commands: readonly string[]): string {
  return commands.map((value) => value.trim()).filter(Boolean).join('\n');
}

export async function loadQuickCommandsSettings(): Promise<QuickCommandsSettings> {
  const settings = await invoke<ShellSettingsRecord>(IPC_COMMANDS.loadShellSettings);
  return coerceQuickCommandsSettings(settings.quickCommands ?? DEFAULT_QUICK_COMMANDS_SETTINGS);
}

export async function saveQuickCommandsSettings(
  quickCommands: QuickCommandsSettings
): Promise<QuickCommandsSettings> {
  const normalized = coerceQuickCommandsSettings(quickCommands);
  const current = await invoke<ShellSettingsRecord>(IPC_COMMANDS.loadShellSettings);
  const currentRecord = asRecord(current) ?? {};
  const nextSettings: ShellSettingsRecord = {
    ...currentRecord,
    quickCommands: normalized
  };
  const saved = await invoke<ShellSettingsRecord>(IPC_COMMANDS.saveShellSettings, {
    settings: nextSettings
  });
  return coerceQuickCommandsSettings(saved.quickCommands ?? DEFAULT_QUICK_COMMANDS_SETTINGS);
}

export function quickCommandRunRequest(id: string): RunQuickCommandRequest {
  const normalized = id.trim();
  if (!normalized) {
    throw new Error('Quick command id must not be empty.');
  }
  return { id: normalized };
}

export function runQuickCommand(request: RunQuickCommandRequest): Promise<QuickCommandSpawnResult> {
  const normalized = quickCommandRunRequest(request.id);
  return invoke<QuickCommandSpawnResult>(IPC_COMMANDS.runQuickCommand, { request: normalized });
}

function coerceQuickCommandEntry(value: unknown, index: number): QuickCommandEntry | null {
  const record = asRecord(value);
  if (!record) {
    return null;
  }
  const id = asString(record.id);
  const label = asString(record.label);
  const mode = asMode(record.mode);
  const targetPath = asString(record.targetPath);
  const args = Array.isArray(record.args) ? record.args.map(asString).filter(Boolean) : [];
  const commands = normalizeCommandLines(record, mode, targetPath, args);
  const cwd = asOptionalString(record.cwd);
  const normalized = {
    id: id.toLowerCase(),
    label,
    mode,
    targetPath: mode === 'direct' ? targetPath : '',
    args: mode === 'direct' ? args : [],
    commands,
    cwd
  } satisfies QuickCommandEntry;
  validateQuickCommandEntry(normalized, index);
  return normalized;
}

function validateQuickCommandEntry(entry: QuickCommandEntry, index: number): void {
  if (!/^[a-z0-9]+(?:[-_][a-z0-9]+)*$/u.test(entry.id)) {
    throw new Error(`Quick command at index ${index} has invalid slug-safe id: ${entry.id}`);
  }
  if (!entry.label.trim()) {
    throw new Error(`Quick command '${entry.id}' label must not be empty.`);
  }
  if (entry.mode === 'direct') {
    if (!entry.targetPath.trim()) {
      throw new Error(`Quick command '${entry.id}' targetPath must not be empty.`);
    }
    const isAbsolutePath = isAbsoluteWindowsPath(entry.targetPath);
    if (!isAbsolutePath && !isSafeCommandToken(entry.targetPath)) {
      throw new Error(
        `Quick command '${entry.id}' direct mode target must be an absolute path or safe command token.`
      );
    }
  } else if (entry.commands.length === 0) {
    throw new Error(`Quick command '${entry.id}' command block must include at least one command.`);
  }
  if (entry.cwd && !isAbsoluteWindowsPath(entry.cwd)) {
    throw new Error(`Quick command '${entry.id}' cwd must be absolute when provided.`);
  }
  for (const arg of entry.args) {
    if (!arg.trim()) {
      throw new Error(`Quick command '${entry.id}' arguments must not include empty lines.`);
    }
    if (/[\u0000-\u001F\u007F]/u.test(arg)) {
      throw new Error(`Quick command '${entry.id}' arguments must not include control characters.`);
    }
    if (isSecretLikeArg(arg)) {
      throw new Error(`Quick command '${entry.id}' arguments must not include secret-like values.`);
    }
  }
  for (const command of entry.commands) {
    if (!command.trim()) {
      throw new Error(`Quick command '${entry.id}' command block must not include empty commands.`);
    }
    if (/[\u0000-\u001F\u007F]/u.test(command)) {
      throw new Error(`Quick command '${entry.id}' command block must not include control characters.`);
    }
    if (isSecretLikeArg(command)) {
      throw new Error(`Quick command '${entry.id}' command block must not include secret-like values.`);
    }
  }
}

function isSecretLikeArg(value: string): boolean {
  if (SECRET_VALUE_PATTERN.test(value)) {
    return true;
  }
  const [left] = value.split('=');
  const key = left.trim().replace(/^--?/u, '').split(/\s+/u, 1)[0];
  if (SECRET_KEY_PATTERN.test(key)) {
    return true;
  }
  if (value.startsWith('--')) {
    const firstToken = value.replace(/^--?/u, '').split(/\s+/u, 1)[0];
    if (SECRET_KEY_PATTERN.test(firstToken)) {
      return true;
    }
  }
  return false;
}

function isAbsoluteWindowsPath(value: string): boolean {
  return /^[a-zA-Z]:[\\/]/u.test(value) || /^\\\\[^\\]/u.test(value) || value.startsWith('/');
}

function isSafeCommandToken(value: string): boolean {
  return /^[A-Za-z0-9._-]+$/u.test(value) && !/[\\/]/u.test(value) && !value.includes(':');
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function asString(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

function asOptionalString(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null;
  }
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function asMode(value: unknown): QuickCommandMode {
  if (value === 'commandBlock' || value === 'powershellFile' || value === 'cmdFile') {
    return 'commandBlock';
  }
  return 'direct';
}

function normalizeCommandLines(
  record: Record<string, unknown>,
  mode: QuickCommandMode,
  targetPath: string,
  args: readonly string[]
): string[] {
  const explicitCommands = Array.isArray(record.commands)
    ? record.commands.map(asString).filter(Boolean)
    : [];
  if (explicitCommands.length > 0) {
    return explicitCommands;
  }
  if (mode !== 'commandBlock') {
    return [];
  }
  const originalMode = asString(record.mode);
  if (originalMode === 'powershellFile') {
    return [`pwsh.exe -NoLogo -NoProfile -File ${quoteCommandPart(targetPath)}${formatInlineArgs(args)}`];
  }
  if (originalMode === 'cmdFile') {
    return [`cmd.exe /C ${quoteCommandPart(targetPath)}${formatInlineArgs(args)}`];
  }
  return [];
}

function formatInlineArgs(args: readonly string[]): string {
  return args.length ? ` ${args.map(quoteCommandPart).join(' ')}` : '';
}

function quoteCommandPart(value: string): string {
  if (/^[A-Za-z0-9._:/\\-]+$/u.test(value)) {
    return value;
  }
  return `"${value.replace(/"/gu, '\\"')}"`;
}
