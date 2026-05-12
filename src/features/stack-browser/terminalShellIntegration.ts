export type TerminalShellMarkerKind = 'prompt' | 'command' | 'output' | 'end' | 'cwd';

export interface TerminalShellMarker {
  kind: TerminalShellMarkerKind;
  sequence: string;
  exitCode?: number;
  cwd?: string;
  timestamp: number;
}

export interface TerminalCommandRecord {
  id: string;
  sessionId: string;
  commandText?: string;
  startMarker?: number;
  outputStartMarker?: number;
  endMarker?: number;
  exitCode?: number;
  cwd?: string;
  startedAt: number;
  endedAt?: number;
}

export interface TerminalCommandState {
  sessionId: string;
  cwd?: string;
  records: TerminalCommandRecord[];
  activeCommandId?: string;
  pendingCommandText?: string;
  nextId: number;
  markerOffset: number;
}

const MAX_COMMAND_RECORDS = 200;

export function createTerminalCommandState(sessionId: string, cwd?: string): TerminalCommandState {
  return { sessionId, cwd, records: [], nextId: 1, markerOffset: 0 };
}

export function parseTerminalShellSequence(sequence: string, timestamp = Date.now()): TerminalShellMarker | null {
  const trimmed = sequence.trim();
  if (!trimmed) return null;
  const [prefix, ...rest] = trimmed.split(';');
  if (prefix === 'A') return { kind: 'prompt', sequence, timestamp };
  if (prefix === 'B') return { kind: 'command', sequence, timestamp };
  if (prefix === 'C') return { kind: 'output', sequence, timestamp };
  if (prefix === 'D') {
    const exitCode = Number.parseInt(rest[0] ?? '', 10);
    return { kind: 'end', sequence, timestamp, exitCode: Number.isFinite(exitCode) ? exitCode : undefined };
  }
  if (prefix === 'CurrentDir' || prefix === '7' || prefix.toLowerCase() === 'cwd') {
    const cwd = rest.join(';');
    return cwd ? { kind: 'cwd', sequence, timestamp, cwd } : null;
  }
  if (/^(file|[a-zA-Z]:\\|\\\\)/.test(trimmed)) {
    return { kind: 'cwd', sequence, timestamp, cwd: trimmed.replace(/^file:\/\//, '') };
  }
  return null;
}

export function beginTerminalCommandRecord(
  state: TerminalCommandState,
  commandText: string,
  xtermMarkerLine?: number,
  timestamp = Date.now()
): TerminalCommandState {
  const text = commandText.trim();
  if (!text) return state;
  const record: TerminalCommandRecord = {
    id: `${state.sessionId}:${state.nextId}`,
    sessionId: state.sessionId,
    commandText: text,
    startMarker: xtermMarkerLine,
    outputStartMarker: typeof xtermMarkerLine === 'number' ? xtermMarkerLine + 1 : undefined,
    cwd: state.cwd,
    startedAt: timestamp
  };
  const records = [...state.records, record];
  if (records.length > MAX_COMMAND_RECORDS) records.splice(0, records.length - MAX_COMMAND_RECORDS);
  return {
    ...state,
    records,
    activeCommandId: record.id,
    pendingCommandText: undefined,
    nextId: state.nextId + 1,
    markerOffset: typeof xtermMarkerLine === 'number' ? xtermMarkerLine : state.markerOffset
  };
}

export function reduceTerminalShellMarker(
  state: TerminalCommandState,
  marker: TerminalShellMarker,
  xtermMarkerLine?: number
): TerminalCommandState {
  const next: TerminalCommandState = {
    ...state,
    records: [...state.records],
    markerOffset: typeof xtermMarkerLine === 'number' ? xtermMarkerLine : state.markerOffset
  };
  if (marker.kind === 'cwd' && marker.cwd) {
    next.cwd = marker.cwd;
    const active = next.records.find((record) => record.id === next.activeCommandId);
    if (active) active.cwd = marker.cwd;
    return next;
  }
  if (marker.kind === 'command') {
    if (!state.pendingCommandText) return next;
    return beginTerminalCommandRecord(next, state.pendingCommandText, xtermMarkerLine, marker.timestamp);
  }
  const active = next.records.find((record) => record.id === next.activeCommandId);
  if (active && marker.kind === 'output') {
    active.outputStartMarker = xtermMarkerLine;
  } else if (active && marker.kind === 'end') {
    active.endMarker = xtermMarkerLine;
    active.exitCode = marker.exitCode;
    active.endedAt = marker.timestamp;
    next.activeCommandId = undefined;
    next.pendingCommandText = undefined;
  }
  return next;
}

export function fallbackCwdFromInput(currentCwd: string | undefined, input: string): string | undefined {
  const normalized = input.trim();
  const match = /^(?:cd|chdir|sl|Set-Location)\s+(.+)$/i.exec(normalized);
  if (!match || !currentCwd) return currentCwd;
  const target = match[1].replace(/^['\"]|['\"]$/g, '');
  if (!target || target === '.' || target.includes('does-not-exist')) return currentCwd;
  if (/^[a-zA-Z]:[\\/]/.test(target) || /^\\\\/.test(target)) return target;
  return `${currentCwd.replace(/[\\/]+$/, '')}\\${target}`;
}
