import { readFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { deriveQuickCommandPendingInputRequest, isQuickCommandRunEventRetired, mergeQuickCommandRunHistoryEntries, mergeQuickCommandTranscripts,
  type QuickCommandRunHistoryEntry, type QuickCommandTranscriptEntry } from '../../src/lib/quickCommands';

// Independent oracle, cross-checked against native below; never derive expected limits from renderer code.
export const HISTORY_WINDOW = 20;
export const TRANSCRIPT_WINDOW = 256;
export function line(sequence: number, kind = 'stdout'): QuickCommandTranscriptEntry {
  return { sequence, kind, body: `row-${String(sequence).padStart(6, '0')}`, atEpochMs: sequence,
    requestId: null, prompt: null, secret: false, redacted: false, pending: false };
}
export function run(index: number, running = false, commandId = index % 2 ? 'alpha' : 'beta'): QuickCommandRunHistoryEntry {
  return { runId: `run-${index}`, commandId, processId: index + 100, startedAtEpochMs: index * 1000,
    finishedAtEpochMs: index * 1000 + 600, running, exitCode: running ? null : 0,
    stdout: '', stderr: '', stdoutTruncated: false, stderrTruncated: false, transcript: [] };
}
function metrics(history: QuickCommandRunHistoryEntry[]) {
  return { runs: history.length, rows: history.reduce((n, item) => n + item.transcript.length, 0),
    bodyCharacters: history.reduce((n, item) => n + item.transcript.reduce((sum, row) => sum + row.body.length, 0), 0) };
}

it('retention oracle matches native global history and transcript windows', () => {
  const native = readFileSync('src-tauri/src/quick_commands.rs', 'utf8');
  expect(native).toMatch(/const HISTORY_LIMIT: usize = 20;/);
  expect(native).toMatch(/const TRANSCRIPT_LIMIT: usize = 256;/);
  expect(native).toMatch(/history\.truncate\(HISTORY_LIMIT\)/);
});

it('600 sequenced live rows converge to native recent256 without stale rows or duplicate replay', () => {
  let rows: QuickCommandTranscriptEntry[] = [];
  for (let sequence = 1; sequence <= 600; sequence++) rows = mergeQuickCommandTranscripts(rows, [line(sequence)]);
  const snapshot = Array.from({ length: TRANSCRIPT_WINDOW }, (_, index) => line(345 + index));
  rows = mergeQuickCommandTranscripts(rows, snapshot);
  rows = mergeQuickCommandTranscripts(rows, [line(1), line(500), line(600)]);
  const retained = metrics([{ ...run(1), transcript: rows }]);
  process.stdout.write(`memory-retention transcript fixture ${JSON.stringify(retained)}\n`);
  expect(retained).toEqual({ runs: 1, rows: 256, bodyCharacters: 2560 });
  expect(rows.map(row => row.sequence)).toEqual(snapshot.map(row => row.sequence));
});

it('40 completed runs cap globally at recent20 while preserving genuinely active old runs', () => {
  const active = { ...run(1, true, 'interactive'), transcript: [{ ...line(1, 'text'), requestId: 'pending',
    prompt: 'Continue?', maxLength: 24, pending: true }] };
  let history = [active];
  for (let index = 2; index <= 41; index++) {
    let item = run(index, true);
    for (let sequence = 1; sequence <= 600; sequence++) {
      item = { ...item, transcript: [line(sequence + index * 1000)] };
      history = mergeQuickCommandRunHistoryEntries(history, [item]);
    }
    history = mergeQuickCommandRunHistoryEntries(history, [{ ...item, running: false, exitCode: 0,
      transcript: [line(index * 1000 + 601, 'exit')] }]);
  }
  const retained = metrics(history);
  process.stdout.write(`memory-retention history fixture ${JSON.stringify(retained)}\n`);
  expect(retained).toEqual({ runs: 21, rows: 5121, bodyCharacters: 51210 });
  expect(history.filter(item => !item.running).map(item => item.runId)).toEqual(
    Array.from({ length: HISTORY_WINDOW }, (_, index) => `run-${41 - index}`));
  expect(history.find(item => item.runId === active.runId)?.running).toBe(true);
  expect(deriveQuickCommandPendingInputRequest(history.find(item => item.runId === active.runId)!)).toMatchObject({ requestId: 'pending', processId: 101 });
  expect(history.every(item => item.transcript.length <= TRANSCRIPT_WINDOW)).toBe(true);
});

it.each(['stopped', 'exit'])('%s remains terminal after stale running snapshot and replay', kind => {
  const ended = { ...run(10), transcript: [line(900, kind)] };
  const stale = { ...run(10, true), transcript: [line(899)] };
  const [merged] = mergeQuickCommandRunHistoryEntries([ended], [stale, stale]);
  expect(merged.running).toBe(false);
  expect(merged.transcript.at(-1)?.kind).toBe(kind);
  expect(new Set(merged.transcript.map(row => row.sequence)).size).toBe(merged.transcript.length);
});

it.each([0, 1, 256, 257])('transcript boundary %i retains ordered unique latest rows', count => {
  const input = Array.from({ length: count }, (_, i) => line(i + 1));
  const rows = mergeQuickCommandTranscripts([...input].reverse(), input);
  expect(rows.map(row => row.sequence)).toEqual(input.slice(-TRANSCRIPT_WINDOW).map(row => row.sequence));
});

it('same-sequence control update replaces request marker without duplicating or corrupting order', () => {
  const pending = { ...line(10, 'text'), requestId: 'request', pending: true };
  const updated = { ...pending, pending: false };
  const rows = mergeQuickCommandTranscripts([line(11), pending], [line(9), updated, line(11)]);
  expect(rows.map(row => row.sequence)).toEqual([9, 10, 11]);
  expect(rows[1].pending).toBe(false);
});

it('retired chronology survives successive merges without rejecting known old active controls', () => {
  const active = run(1, true, 'interactive');
  let history = [active];
  for (let index = 2; index <= 100; index++) history = mergeQuickCommandRunHistoryEntries(history, [run(index)]);
  for (let index = 2; index <= 80; index++) {
    history = mergeQuickCommandRunHistoryEntries(history, [run(index), run(index, true)]);
    expect(history.length).toBe(21);
  }
  const retired = { ...line(1), runId: 'run-2', commandId: 'beta', processId: 102 };
  expect(isQuickCommandRunEventRetired(history, retired)).toBe(true);
  expect(isQuickCommandRunEventRetired(history, { ...retired, runId: active.runId })).toBe(false);
  expect(history.filter(item => !item.running).map(item => item.runId)).toEqual(
    Array.from({ length: 20 }, (_, i) => `run-${100 - i}`));
});

it('resolved input metadata survives transcript eviction and rejects replayed old request snapshot', () => {
  const pending = { ...line(10, 'text'), requestId: 'request', prompt: 'Continue?', pending: true };
  let [active] = mergeQuickCommandRunHistoryEntries([], [{ ...run(1, true), transcript: [pending] }]);
  for (let sequence = 11; sequence <= 610; sequence++) {
    [active] = mergeQuickCommandRunHistoryEntries([active], [{ ...active, transcript: [line(sequence)] }]);
  }
  expect(active.transcript.some(row => row.sequence === 10)).toBe(false);
  expect(deriveQuickCommandPendingInputRequest(active)?.requestId).toBe('request');
  [active] = mergeQuickCommandRunHistoryEntries([active], [{ ...active,
    transcript: [{ ...line(611, 'input-submitted'), requestId: 'request' }] }]);
  for (let sequence = 612; sequence <= 912; sequence++) {
    [active] = mergeQuickCommandRunHistoryEntries([active], [{ ...active, transcript: [line(sequence)] }]);
  }
  [active] = mergeQuickCommandRunHistoryEntries([active], [{ ...run(1, true), transcript: [pending] }]);
  expect(active.transcript.length).toBe(256);
  expect(deriveQuickCommandPendingInputRequest(active)).toBeNull();
});
