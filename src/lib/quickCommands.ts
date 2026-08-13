import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const QUICK_COMMAND_MODES = ['direct', 'commandBlock'] as const;
export type QuickCommandMode = (typeof QUICK_COMMAND_MODES)[number];

export type QuickCommandRunId = string;

export interface QuickCommandEntry {
  id: string;
  label: string;
  mode: QuickCommandMode;
  targetPath: string;
  args: string[];
  commands: string[];
  cwd: string | null;
}

export interface QuickCommandTranscriptEntry {
  kind: string;
  body: string;
  requestId: string | null;
  prompt: string | null;
  secret: boolean;
  redacted: boolean;
  maxLength?: number;
  sequence: number;
  atEpochMs: number;
  pending: boolean;
}

export interface QuickCommandRunHistoryEntry {
  runId: QuickCommandRunId;
  commandId: string;
  startedAtEpochMs: number;
  finishedAtEpochMs: number;
  processId: number;
  exitCode: number | null;
  stdout: string;
  stderr: string;
  stdoutTruncated: boolean;
  stderrTruncated: boolean;
  running: boolean;
  transcript: QuickCommandTranscriptEntry[];
}

export interface QuickCommandsSettings {
  entries: QuickCommandEntry[];
  history: QuickCommandRunHistoryEntry[];
}

export interface RunQuickCommandRequest {
  id: string;
}

export interface QuickCommandSpawnResult {
  runId: QuickCommandRunId;
  processId: number;
}

export interface StopQuickCommandRequest {
  id: string;
  runId: QuickCommandRunId;
  processId: number;
}

export interface SendQuickCommandInputRequest {
  id: string;
  runId: QuickCommandRunId;
  processId: number;
  requestId: string;
  value: string;
  secret: boolean;
  maxLength: number;
}

export interface QuickCommandRunUpdatedEvent extends QuickCommandTranscriptEntry {
  runId: QuickCommandRunId;
  commandId: string;
  processId: number;
}

export interface QuickCommandPendingInputRequest {
  runId: QuickCommandRunId;
  commandId: string;
  processId: number;
  requestId: string;
  kind: string;
  prompt: string;
  secret: boolean;
  redacted: boolean;
  maxLength: number;
  sequence?: number;
  atEpochMs?: number;
  pending?: boolean;
}

export type ListQuickCommandHistoryRequest = { id?: string };

type ShellSettingsRecord = Record<string, unknown> & {
  quickCommands?: unknown;
};

const DEFAULT_QUICK_COMMANDS_SETTINGS: QuickCommandsSettings = {
  entries: [],
  history: []
};

const SECRET_KEY_PATTERN = /(token|secret|password|credential|api[_-]?key|authorization|cookie)/iu;
const SECRET_VALUE_PATTERN = /\b(?:bearer\s+\S+|ghp_[A-Za-z0-9_.-]+|gho_[A-Za-z0-9_.-]+|github_pat_[A-Za-z0-9_.-]+|xoxb-[A-Za-z0-9_.-]+|sk-[A-Za-z0-9_.-]+|akia[A-Za-z0-9_.-]*)\b/iu;
const DEFAULT_MAX_QUICK_COMMAND_INPUT_LENGTH = 4096;
const MIN_QUICK_COMMAND_INPUT_LENGTH = 1;
const MAX_QUICK_COMMAND_INPUT_LENGTH = 16384;

export function defaultQuickCommandsSettings(): QuickCommandsSettings {
  return { entries: [], history: [] };
}

export function coerceQuickCommandsSettings(value: unknown): QuickCommandsSettings {
  const record = asRecord(value);
  const rawEntries = Array.isArray(record?.entries) ? record.entries : [];
  const rawHistory = Array.isArray(record?.history) ? record.history : [];
  const entries = rawEntries
    .map((entry, index) => coerceQuickCommandEntry(entry, index))
    .filter((entry): entry is QuickCommandEntry => entry !== null);
  const history = rawHistory
    .map((entry, index) => coerceLegacyQuickCommandRunHistoryEntry(entry, index))
    .filter((entry): entry is QuickCommandRunHistoryEntry => entry !== null)
    .sort(sortQuickCommandHistoryEntries);

  const seen = new Set<string>();
  for (const entry of entries) {
    if (seen.has(entry.id)) {
      throw new Error(`Quick command id must be unique: ${entry.id}`);
    }
    seen.add(entry.id);
  }
  return { entries, history };
}

export function parseQuickCommandArgsTextarea(value: string): string[] {
  return value.split(/\r?\n/u).map((line) => line.trim()).filter(Boolean);
}

export function formatQuickCommandArgsTextarea(args: readonly string[]): string {
  return args.map((value) => value.trim()).filter(Boolean).join('\n');
}

export function parseQuickCommandCommandsTextarea(value: string): string[] {
  return value.split(/\r?\n/u).map((line) => line.trim()).filter(Boolean);
}

export function formatQuickCommandCommandsTextarea(commands: readonly string[]): string {
  return commands.map((value) => value.trim()).filter(Boolean).join('\n');
}

export function nextDuplicateQuickCommandLabel(label: string, existingLabels: readonly string[]): string {
  const base = label.trim();
  const normalized = new Set(existingLabels.map((value) => value.trim().toLowerCase()));
  let suffix = 1;
  let candidate = `${base} (${suffix})`;
  while (normalized.has(candidate.trim().toLowerCase())) {
    suffix += 1;
    candidate = `${base} (${suffix})`;
  }
  return candidate;
}

export function nextUniqueQuickCommandId(label: string, existingIds: readonly string[]): string {
  const base = asSlug(label);
  if (!base) return '';
  const ids = new Set(existingIds.map((value) => value.trim().toLowerCase()));
  let suffix = 0;
  let candidate = base;
  while (ids.has(candidate.toLowerCase())) {
    suffix += 1;
    candidate = `${base}-${suffix}`;
  }
  return candidate;
}

export async function loadQuickCommandsSettings(): Promise<QuickCommandsSettings> {
  const settings = await invoke<ShellSettingsRecord>(IPC_COMMANDS.loadShellSettings);
  return coerceQuickCommandsSettings(settings.quickCommands ?? DEFAULT_QUICK_COMMANDS_SETTINGS);
}

export async function saveQuickCommandsSettings(quickCommands: QuickCommandsSettings): Promise<QuickCommandsSettings> {
  const normalized = coerceQuickCommandsSettings(quickCommands);
  const saved = await invoke<QuickCommandsSettings>(IPC_COMMANDS.saveQuickCommandsSettings, {
    quickCommands: normalized
  });
  return coerceQuickCommandsSettings(saved);
}

export function quickCommandRunRequest(id: string): RunQuickCommandRequest {
  return { id: normalizeNonEmpty(id, 'Quick command id') };
}

export function runQuickCommand(request: RunQuickCommandRequest): Promise<QuickCommandSpawnResult> {
  const normalized = quickCommandRunRequest(request.id);
  return invoke<QuickCommandSpawnResult>(IPC_COMMANDS.runQuickCommand, { request: normalized });
}

export function listQuickCommandHistory(request: ListQuickCommandHistoryRequest = {}): Promise<QuickCommandRunHistoryEntry[]> {
  const normalized = request.id === undefined ? undefined : { id: request.id.trim() };
  return invoke<QuickCommandRunHistoryEntry[]>(IPC_COMMANDS.listQuickCommandHistory, { request: normalized });
}

export function sendQuickCommandInput(request: SendQuickCommandInputRequest): Promise<void> {
  const normalized = normalizeQuickCommandInputRequest(request);
  return invoke<void>(IPC_COMMANDS.sendQuickCommandInput, { request: normalized });
}

export function stopQuickCommand(request: StopQuickCommandRequest): Promise<void> {
  const normalized = normalizeQuickCommandStopRequest(request);
  return invoke<void>(IPC_COMMANDS.stopQuickCommand, { request: normalized });
}

export function deriveQuickCommandPendingInputRequest(
  run: Pick<QuickCommandRunHistoryEntry, 'runId' | 'commandId' | 'processId' | 'transcript'>
): QuickCommandPendingInputRequest | null {
  for (let index = run.transcript.length - 1; index >= 0; index -= 1) {
    const entry = run.transcript[index];
    if (!entry.requestId || (entry.kind !== 'text' && entry.kind !== 'password' && entry.kind !== 'confirm' && entry.kind !== 'input-request')) {
      continue;
    }
    const stillPending = !run.transcript.slice(index + 1).some((next) => next.requestId === entry.requestId && (next.kind === 'input-submitted' || next.kind === 'input-cancelled'));
    if (!stillPending) {
      continue;
    }
    return {
      runId: run.runId,
      commandId: run.commandId,
      processId: run.processId,
      requestId: entry.requestId,
      kind: entry.kind === 'input-request' ? (entry.secret ? 'password' : 'text') : entry.kind,
      prompt: entry.prompt ?? '',
      secret: entry.secret,
      redacted: entry.redacted,
      maxLength: normalizeQuickCommandInputMaxLength(entry.maxLength),
      sequence: entry.sequence,
      atEpochMs: entry.atEpochMs,
      pending: true
    };
  }
  return null;
}

export function mergeQuickCommandRunHistoryEntry(
  existing: QuickCommandRunHistoryEntry | null | undefined,
  incoming: QuickCommandRunHistoryEntry
): QuickCommandRunHistoryEntry {
  const normalizedIncoming = coerceQuickCommandRunHistoryEntry(incoming, 0) ?? incoming;
  if (!existing) {
    return normalizedIncoming;
  }
  return {
    ...existing,
    ...normalizedIncoming,
    transcript: mergeQuickCommandTranscripts(existing.transcript, normalizedIncoming.transcript),
    running: normalizedIncoming.running,
    stdout: normalizedIncoming.stdout,
    stderr: normalizedIncoming.stderr,
    stdoutTruncated: normalizedIncoming.stdoutTruncated,
    stderrTruncated: normalizedIncoming.stderrTruncated
  };
}

export function mergeQuickCommandRunHistoryEntries(
  existing: readonly QuickCommandRunHistoryEntry[],
  incoming: readonly QuickCommandRunHistoryEntry[]
): QuickCommandRunHistoryEntry[] {
  const byRunId = new Map<string, QuickCommandRunHistoryEntry>();
  for (const entry of existing) {
    byRunId.set(entry.runId, entry);
  }
  for (const entry of incoming) {
    byRunId.set(entry.runId, mergeQuickCommandRunHistoryEntry(byRunId.get(entry.runId), entry));
  }
  return [...byRunId.values()].sort(sortQuickCommandHistoryEntries);
}

export function mergeQuickCommandTranscripts(
  existing: readonly QuickCommandTranscriptEntry[],
  incoming: readonly QuickCommandTranscriptEntry[]
): QuickCommandTranscriptEntry[] {
  const bySequence = new Map<number, QuickCommandTranscriptEntry>();
  const fallback = new Map<string, QuickCommandTranscriptEntry>();

  const ingest = (entry: QuickCommandTranscriptEntry) => {
    if (entry.sequence !== undefined && Number.isFinite(entry.sequence)) {
      bySequence.set(entry.sequence, entry);
      return;
    }
    fallback.set(transcriptFallbackKey(entry), entry);
  };

  for (const entry of existing) ingest(entry);
  for (const entry of incoming) ingest(entry);

  return [
    ...[...bySequence.entries()].sort(([leftSequence], [rightSequence]) => leftSequence - rightSequence).map(([, entry]) => entry),
    ...fallback.values()
  ];
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
    if (!isAbsoluteWindowsPath(entry.targetPath) && !isSafeCommandToken(entry.targetPath)) {
      throw new Error(`Quick command '${entry.id}' direct mode target must be an absolute path or safe command token.`);
    }
  } else if (entry.commands.length === 0) {
    throw new Error(`Quick command '${entry.id}' command block must include at least one command.`);
  }
  if (entry.cwd && !isAbsoluteWindowsPath(entry.cwd)) {
    throw new Error(`Quick command '${entry.id}' cwd must be absolute when provided.`);
  }
  for (const arg of entry.args) {
    if (isSecretLikeText(arg)) {
      throw new Error(`Quick command '${entry.id}' arguments must not include secret-like values.`);
    }
  }
  for (const command of entry.commands) {
    if (isSecretLikeText(command)) {
      throw new Error(`Quick command '${entry.id}' command block must not include secret-like values.`);
    }
  }
}

function coerceQuickCommandRunHistoryEntry(value: unknown, index: number): QuickCommandRunHistoryEntry | null {
  const record = asRecord(value);
  if (!record) {
    return null;
  }
  const transcript = Array.isArray(record.transcript)
    ? record.transcript.map((entry, transcriptIndex) => coerceQuickCommandTranscriptEntry(entry, transcriptIndex)).filter((entry): entry is QuickCommandTranscriptEntry => entry !== null)
    : [];
  const entry = {
    runId: asString(record.runId),
    commandId: asString(record.commandId),
    startedAtEpochMs: numberOrZero(record.startedAtEpochMs),
    finishedAtEpochMs: numberOrZero(record.finishedAtEpochMs),
    processId: numberOrZero(record.processId),
    exitCode: asOptionalInteger(record.exitCode),
    stdout: asString(record.stdout),
    stderr: asString(record.stderr),
    stdoutTruncated: Boolean(record.stdoutTruncated),
    stderrTruncated: Boolean(record.stderrTruncated),
    running: Boolean(record.running),
    transcript
  } satisfies QuickCommandRunHistoryEntry;
  validateQuickCommandRunHistoryEntry(entry, index);
  return entry;
}

function coerceLegacyQuickCommandRunHistoryEntry(value: unknown, index: number): QuickCommandRunHistoryEntry | null {
  const record = asRecord(value);
  if (!record || !asString(record.runId).trim() || !asString(record.commandId).trim()) {
    return null;
  }
  return coerceQuickCommandRunHistoryEntry(record, index);
}

function validateQuickCommandRunHistoryEntry(entry: QuickCommandRunHistoryEntry, index: number): void {
  if (!entry.runId.trim()) {
    throw new Error(`Quick command history entry at index ${index} must include runId.`);
  }
  if (!entry.commandId.trim()) {
    throw new Error(`Quick command history entry at index ${index} must include commandId.`);
  }
}

function coerceQuickCommandTranscriptEntry(value: unknown, index: number): QuickCommandTranscriptEntry | null {
  const record = asRecord(value);
  if (!record) {
    return null;
  }
  const entry = {
    kind: asString(record.kind),
    body: asString(record.body),
    requestId: asOptionalString(record.requestId),
    prompt: asOptionalString(record.prompt),
    secret: Boolean(record.secret),
    redacted: Boolean(record.redacted),
    maxLength: asOptionalInteger(record.maxLength) ?? undefined,
    sequence: asOptionalInteger(record.sequence) ?? 0,
    atEpochMs: asOptionalInteger(record.atEpochMs) ?? 0,
    pending: Boolean(record.pending)
  } satisfies QuickCommandTranscriptEntry;
  validateQuickCommandTranscriptEntry(entry, index);
  return entry;
}

function validateQuickCommandTranscriptEntry(entry: QuickCommandTranscriptEntry, index: number): void {
  if (!entry.kind.trim()) {
    throw new Error(`Quick command transcript entry at index ${index} must include kind.`);
  }
}

function normalizeQuickCommandInputRequest(request: SendQuickCommandInputRequest): SendQuickCommandInputRequest {
  return {
    id: normalizeNonEmpty(request.id, 'Quick command id'),
    runId: normalizeQuickCommandRunId(request.runId),
    processId: normalizePositiveInteger(request.processId, 'Quick command process id'),
    requestId: normalizeNonEmpty(request.requestId, 'Quick command request id'),
    value: normalizeQuickCommandInputValue(request.value, request.maxLength),
    secret: Boolean(request.secret),
    maxLength: normalizeQuickCommandInputMaxLength(request.maxLength)
  };
}

function normalizeQuickCommandStopRequest(request: StopQuickCommandRequest): StopQuickCommandRequest {
  return {
    id: normalizeNonEmpty(request.id, 'Quick command id'),
    runId: normalizeQuickCommandRunId(request.runId),
    processId: normalizePositiveInteger(request.processId, 'Quick command process id')
  };
}

function normalizeQuickCommandRunId(value: string): QuickCommandRunId {
  return normalizeNonEmpty(value, 'Quick command run id');
}

export function normalizeQuickCommandInputMaxLength(value: unknown): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return DEFAULT_MAX_QUICK_COMMAND_INPUT_LENGTH;
  }
  const normalized = Math.trunc(value);
  return Math.min(MAX_QUICK_COMMAND_INPUT_LENGTH, Math.max(MIN_QUICK_COMMAND_INPUT_LENGTH, normalized));
}

export function normalizeQuickCommandInputValue(value: string, maxLength: number): string {
  if (typeof value !== 'string') {
    throw new Error('Quick command input value must be a string.');
  }
  return Array.from(value).slice(0, normalizeQuickCommandInputMaxLength(maxLength)).join('');
}

function normalizeNonEmpty(value: string, label: string): string {
  const normalized = typeof value === 'string' ? value.trim() : '';
  if (!normalized) {
    throw new Error(`${label} must not be empty.`);
  }
  return normalized;
}

function normalizePositiveInteger(value: number, label: string): number {
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`${label} must be positive.`);
  }
  return value;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? (value as Record<string, unknown>) : null;
}

function asString(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

function asOptionalString(value: unknown): string | null {
  if (typeof value !== 'string') return null;
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function asOptionalInteger(value: unknown): number | null {
  if (typeof value !== 'number' || !Number.isFinite(value)) return null;
  return Math.trunc(value);
}

function numberOrZero(value: unknown): number {
  return asOptionalInteger(value) ?? 0;
}

function asSlug(value: string): string {
  return value.toLowerCase().trim().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
}

function asMode(value: unknown): QuickCommandMode {
  if (value === 'commandBlock' || value === 'powershellFile' || value === 'cmdFile') return 'commandBlock';
  return 'direct';
}

function normalizeCommandLines(
  record: Record<string, unknown>,
  mode: QuickCommandMode,
  targetPath: string,
  args: readonly string[]
): string[] {
  const explicitCommands = Array.isArray(record.commands) ? record.commands.map(asString).filter(Boolean) : [];
  if (explicitCommands.length > 0) return explicitCommands;
  if (mode !== 'commandBlock') return [];
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
  return /^[A-Za-z0-9._:/\\-]+$/u.test(value) ? value : `"${value.replace(/"/gu, '\\"')}"`;
}

function isAbsoluteWindowsPath(value: string): boolean {
  return /^[a-zA-Z]:[\\/]/u.test(value) || /^\\\\[^\\]/u.test(value) || value.startsWith('/');
}

function isSafeCommandToken(value: string): boolean {
  return /^[A-Za-z0-9._-]+$/u.test(value) && !/[\\/]/u.test(value) && !value.includes(':');
}

function isSecretLikeText(value: string): boolean {
  if (SECRET_VALUE_PATTERN.test(value)) {
    return true;
  }
  const [left] = value.split('=');
  const key = left.trim().replace(/^--?/u, '').split(/\s+/u, 1)[0];
  return SECRET_KEY_PATTERN.test(key);
}

function transcriptFallbackKey(entry: QuickCommandTranscriptEntry): string {
  return [entry.kind, entry.body, entry.requestId ?? '', entry.prompt ?? '', String(entry.secret), String(entry.redacted), String(entry.maxLength ?? ''), String(entry.sequence ?? ''), String(entry.atEpochMs ?? '')].join('::');
}

function sortQuickCommandHistoryEntries(left: QuickCommandRunHistoryEntry, right: QuickCommandRunHistoryEntry): number {
  return right.startedAtEpochMs - left.startedAtEpochMs
    || right.finishedAtEpochMs - left.finishedAtEpochMs
    || right.processId - left.processId
    || left.runId.localeCompare(right.runId);
}
