import type { TerminalCommandRecord, TerminalCommandState } from '../stack-browser/terminalShellIntegration';

export type TerminalRecentCommand = Pick<TerminalCommandRecord, 'id' | 'commandText' | 'cwd' | 'exitCode' | 'startedAt'>;

export function recentTerminalCommands(state: TerminalCommandState | null | undefined, limit = 12): TerminalRecentCommand[] {
  const seen = new Set<string>();
  return (state?.records ?? [])
    .slice()
    .reverse()
    .filter((record) => {
      const command = record.commandText?.trim();
      if (!command || seen.has(command)) return false;
      seen.add(command);
      return true;
    })
    .slice(0, limit)
    .map((record) => ({ id: record.id, commandText: record.commandText, cwd: record.cwd, exitCode: record.exitCode, startedAt: record.startedAt }));
}

export function recentTerminalDirectories(state: TerminalCommandState | null | undefined, limit = 8): string[] {
  const dirs: string[] = [];
  const seen = new Set<string>();
  for (const cwd of (state?.records ?? []).map((record) => record.cwd).concat(state?.cwd ?? '').reverse()) {
    const dir = cwd?.trim();
    if (!dir || seen.has(dir)) continue;
    seen.add(dir);
    dirs.push(dir);
    if (dirs.length >= limit) break;
  }
  return dirs;
}
