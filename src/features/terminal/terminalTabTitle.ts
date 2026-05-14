import type { TerminalCommandState } from '../stack-browser/terminalShellIntegration';

export interface TerminalTabTitleInput {
  profileTitle?: string;
  manualTitle?: string;
  cwd?: string;
  currentInputText?: string;
  recentOutputText?: string;
  commandState?: TerminalCommandState | null;
}

const MAX_TITLE_LENGTH = 48;
const AI_HARNESS_COMMANDS = new Set(['pi', 'codex', 'claude', 'gemini', 'aider', 'opencode']);

export function buildTerminalTabTitle(input: TerminalTabTitleInput): string {
  const manualTitle = sanitizeTitlePart(input.manualTitle);
  if (manualTitle) return truncateTitle(manualTitle);

  const commandRecordCwd = latestCommandCwd(input.commandState);
  const cwd = normalizedDirectoryPath(input.cwd || commandRecordCwd || input.commandState?.cwd);
  const cwdName = directoryBasename(cwd);
  const submittedCommandText = latestCommandText(input.commandState);
  const commandText = submittedCommandText || (input.commandState ? '' : sanitizeTitlePart(input.currentInputText));
  const commandTitle = commandText ? titleForCommand(commandText, cwdName) : '';
  if (commandTitle) return truncateTitle(commandTitle);

  if (cwd) return truncateTitle(cwd);

  return truncateTitle(sanitizeTitlePart(input.profileTitle) || 'Terminal');
}

export function sanitizeTitlePart(value?: string): string {
  return (value ?? '')
    .replace(/\x1b\][^\x07]*(?:\x07|\x1b\\)/g, '')
    .replace(/\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])/g, '')
    .replace(/[\x00-\x1f\x7f-\x9f]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

function latestCommandRecord(commandState?: TerminalCommandState | null) {
  const active = commandState?.activeCommandId
    ? commandState.records.find((record) => record.id === commandState.activeCommandId)
    : undefined;
  return active ?? commandState?.records.at(-1);
}

function latestCommandText(commandState?: TerminalCommandState | null): string {
  return sanitizeTitlePart(latestCommandRecord(commandState)?.commandText);
}

function latestCommandCwd(commandState?: TerminalCommandState | null): string {
  return sanitizeTitlePart(latestCommandRecord(commandState)?.cwd);
}

function titleForCommand(commandText: string, cwdName: string): string {
  const tokens = shellTokens(commandText);
  const executableToken = firstExecutableToken(tokens);
  if (!executableToken) return '';

  const executable = commandProgramName(executableToken);
  const lowerExecutable = executable.toLowerCase();
  const args = executableToken === tokens[0] ? tokens.slice(1) : tokens.slice(tokens.indexOf(executableToken) + 1);

  if (lowerExecutable === 'mvn' || lowerExecutable === 'mvnw') {
    const goal = args.find((arg) => !arg.startsWith('-') && arg.toLowerCase() !== 'clean');
    return `maven${goal ? ` ${goal}` : ''}`;
  }

  if (AI_HARNESS_COMMANDS.has(lowerExecutable)) {
    return cwdName ? `${lowerExecutable} - ${cwdName}` : lowerExecutable;
  }

  if (args.length > 0 && isShortDeveloperCommand(lowerExecutable)) {
    const firstArg = args.find((arg) => !arg.startsWith('-'));
    if (firstArg && !looksLikePath(firstArg)) return `${lowerExecutable} ${firstArg}`;
  }

  return cwdName ? `${lowerExecutable} - ${cwdName}` : lowerExecutable;
}

function shellTokens(commandText: string): string[] {
  const tokens: string[] = [];
  const pattern = /"([^"]*)"|'([^']*)'|(\S+)/gu;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(commandText))) {
    tokens.push(match[1] ?? match[2] ?? match[3] ?? '');
  }
  return tokens.filter(Boolean);
}

function firstExecutableToken(tokens: string[]): string {
  for (const token of tokens) {
    const lower = token.toLowerCase();
    if (lower === '&' || lower === 'start' || lower === 'cmd' || lower === '/c' || lower === '/k') continue;
    if (lower === 'sudo' || lower === 'env' || /^[A-Z_][A-Z0-9_]*=/i.test(token)) continue;
    return token;
  }
  return '';
}

function commandProgramName(commandToken: string): string {
  const withoutPath = commandToken.replace(/^\.\//, '').split(/[\\/]/).pop() ?? commandToken;
  return withoutPath.replace(/\.(exe|cmd|bat|ps1)$/i, '') || commandToken;
}

function isShortDeveloperCommand(command: string): boolean {
  return ['npm', 'pnpm', 'yarn', 'bun', 'cargo', 'go', 'python', 'node', 'dotnet'].includes(command);
}

function looksLikePath(value: string): boolean {
  return /[\\/]/.test(value) || /^[A-Za-z]:/.test(value) || /^\.{1,2}$/.test(value);
}

function normalizedDirectoryPath(value?: string): string {
  const cleaned = sanitizeTitlePart(value).replace(/\\/g, '/').replace(/\/+$/g, '');
  if (!cleaned) return '';
  return cleaned.replace(/^([A-Z]):/, (_, drive: string) => `${drive.toLowerCase()}:`);
}

function directoryBasename(value?: string): string {
  const cleaned = normalizedDirectoryPath(value);
  if (!cleaned) return '';
  const match = /(?:^|\/)([^\/]+)$/.exec(cleaned);
  return match?.[1] || cleaned;
}

function truncateTitle(value: string, maxLength = MAX_TITLE_LENGTH): string {
  if (value.length <= maxLength) return value;
  return `${value.slice(0, Math.max(0, maxLength - 1)).trimEnd()}…`;
}
