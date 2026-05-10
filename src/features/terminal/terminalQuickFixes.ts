export type TerminalQuickFixKind = 'gitUpstream' | 'portInUse' | 'commandNotFound' | 'testFailurePath';

export type TerminalQuickFixSuggestion = {
  kind: TerminalQuickFixKind;
  title: string;
  detail?: string;
  insertText?: string;
  target?: string;
  pid?: number;
  autoExecute: false;
  destructive?: boolean;
};

export function detectTerminalQuickFixes(output: string, commandText = ''): TerminalQuickFixSuggestion[] {
  const fixes: TerminalQuickFixSuggestion[] = [];
  const upstream = /git push --set-upstream origin ([A-Za-z0-9._\/-]+)/i.exec(output);
  if (upstream?.[1]) {
    fixes.push({ kind: 'gitUpstream', title: `Set upstream for ${upstream[1]}`, insertText: `git push --set-upstream origin ${upstream[1]}`, autoExecute: false });
  }
  const port = /(?:EADDRINUSE|address already in use).*?(?:port\s*)?(\d{2,5})(?:.*?PID\s*(\d+))?/is.exec(output);
  if (port?.[1]) {
    fixes.push({ kind: 'portInUse', title: `Port ${port[1]} already in use`, detail: port[2] ? `PID ${port[2]}` : 'Open Process Manager or choose another port.', pid: port[2] ? Number(port[2]) : undefined, autoExecute: false, destructive: true });
  }
  const missing = /(?:command not found|is not recognized as (?:an internal|the name of)|not recognized).*?:?\s*['"]?([A-Za-z0-9._-]+)['"]?/i.exec(output);
  if (missing?.[1]) {
    fixes.push({ kind: 'commandNotFound', title: `Command not found: ${missing[1]}`, detail: 'Check PATH or install the tool.', autoExecute: false });
  }
  const failurePath = /([A-Za-z]:\\[^\r\n<>|"?*]+|(?:\.{1,2}[\\/])?(?:[\w.-]+[\\/])+[\w.-]+):(\d+)(?::(\d+))?/i.exec(output || commandText);
  if (failurePath?.[1]) {
    fixes.push({ kind: 'testFailurePath', title: 'Open failing file', target: failurePath[1], detail: failurePath[2] ? `Line ${failurePath[2]}${failurePath[3] ? `:${failurePath[3]}` : ''}` : undefined, autoExecute: false });
  }
  return dedupe(fixes).map((fix) => ({ ...fix, autoExecute: false as const }));
}

function dedupe(fixes: TerminalQuickFixSuggestion[]) {
  const seen = new Set<string>();
  return fixes.filter((fix) => {
    const key = `${fix.kind}:${fix.insertText ?? fix.target ?? fix.title}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
