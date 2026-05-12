export type TerminalQuickSelectKind = 'url' | 'localhost' | 'windowsPath' | 'relativePath' | 'fileLine' | 'gitHash' | 'branch';

export type TerminalQuickSelectTarget = {
  kind: TerminalQuickSelectKind;
  text: string;
  target: string;
  index: number;
  line?: number;
  column?: number;
  label?: string;
};

const URL_PATTERN = /\bhttps?:\/\/[^\s"'<>]+/gi;
const LOCALHOST_PATTERN = /\b(?:localhost|127\.0\.0\.1):\d{2,5}(?:\/[^\s"'<>]*)?/gi;
const WINDOWS_PATH_PATTERN = /\b[A-Za-z]:\\[^\s\r\n<>|"?*]+/g;
const RELATIVE_PATH_PATTERN = /(?:^|[\s(["'`])((?:\.{1,2}[\\/])?(?:[\w.-]+[\\/])+[\w.-]+(?::\d+)?(?::\d+)?)/g;
const GIT_HASH_PATTERN = /\b[0-9a-f]{7,40}\b/gi;
const BRANCH_PATTERN = /(?:branch|checkout|switched to|on)\s+['"]?([A-Za-z0-9._\/-]{2,})['"]?/gi;
const LABELS = 'asdfghjklqwertyuiopzxcvbnm'.split('');

export function detectTerminalQuickSelectTargets(text: string, cwd = ''): TerminalQuickSelectTarget[] {
  const targets: TerminalQuickSelectTarget[] = [];
  addPattern(targets, text, URL_PATTERN, (match, index) => ({
    kind: isLocalhostUrl(match) ? 'localhost' : 'url',
    text: match,
    target: trimTrailingPunctuation(match),
    index
  }));
  addPattern(targets, text, LOCALHOST_PATTERN, (match, index) => ({
    kind: 'localhost',
    text: match,
    target: `http://${trimTrailingPunctuation(match)}`,
    index
  }));
  addPathPattern(targets, text, WINDOWS_PATH_PATTERN, cwd, false);
  let relativeMatch: RegExpExecArray | null;
  RELATIVE_PATH_PATTERN.lastIndex = 0;
  while ((relativeMatch = RELATIVE_PATH_PATTERN.exec(text))) {
    const raw = relativeMatch[1];
    const prefixLength = relativeMatch[0].length - raw.length;
    const parsed = parsePathLineColumn(trimTrailingPunctuation(raw));
    const normalized = parsed.path.replace(/\//g, '\\');
    const absolute = /^[A-Za-z]:\\/.test(normalized) ? normalized : joinTerminalPath(cwd, normalized);
    targets.push({
      kind: parsed.line ? 'fileLine' : 'relativePath',
      text: raw,
      target: absolute,
      index: relativeMatch.index + prefixLength,
      line: parsed.line,
      column: parsed.column
    });
  }
  addPattern(targets, text, GIT_HASH_PATTERN, (match, index) => ({ kind: 'gitHash', text: match, target: match, index }));
  let branchMatch: RegExpExecArray | null;
  BRANCH_PATTERN.lastIndex = 0;
  while ((branchMatch = BRANCH_PATTERN.exec(text))) {
    const branch = branchMatch[1];
    if (isSafeBranchName(branch)) {
      targets.push({ kind: 'branch', text: branch, target: branch, index: branchMatch.index + branchMatch[0].lastIndexOf(branch) });
    }
  }
  return assignQuickSelectLabels(dedupe(targets).filter(isSafeTerminalQuickSelectTarget).sort((a, b) => a.index - b.index));
}

export function assignQuickSelectLabels(targets: TerminalQuickSelectTarget[]) {
  return targets.slice(0, LABELS.length).map((target, index) => ({ ...target, label: LABELS[index] }));
}

export function isSafeTerminalQuickSelectTarget(target: TerminalQuickSelectTarget) {
  if (target.kind === 'url' || target.kind === 'localhost') {
    return /^https?:\/\//i.test(target.target) && !/^javascript:/i.test(target.target);
  }
  if (target.kind === 'gitHash') {
    return /^[0-9a-f]{7,40}$/i.test(target.target);
  }
  if (target.kind === 'branch') {
    return isSafeBranchName(target.target);
  }
  return /^[A-Za-z]:\\/.test(target.target) && !/[<>|"?*]/.test(target.target);
}

function addPathPattern(targets: TerminalQuickSelectTarget[], text: string, pattern: RegExp, cwd: string, relative: boolean) {
  addPattern(targets, text, pattern, (match, index) => {
    const parsed = parsePathLineColumn(trimTrailingPunctuation(match));
    return { kind: parsed.line ? 'fileLine' : relative ? 'relativePath' : 'windowsPath', text: match, target: parsed.path, index, line: parsed.line, column: parsed.column };
  });
}

function addPattern(targets: TerminalQuickSelectTarget[], text: string, pattern: RegExp, map: (match: string, index: number) => TerminalQuickSelectTarget) {
  pattern.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(text))) targets.push(map(match[0], match.index));
}

function parsePathLineColumn(value: string) {
  const match = /^(.*?)(?::(\d+))?(?::(\d+))?$/.exec(value);
  return { path: match?.[1] ?? value, line: match?.[2] ? Number(match[2]) : undefined, column: match?.[3] ? Number(match[3]) : undefined };
}

function trimTrailingPunctuation(value: string) { return value.replace(/[),.;]+$/g, ''); }
function isLocalhostUrl(value: string) { return /^https?:\/\/(?:localhost|127\.0\.0\.1)(?::|\/|$)/i.test(value); }
function joinTerminalPath(cwd: string, relativePath: string) {
  const cleanCwd = cwd.replace(/[\\/]+$/g, '');
  const cleanRelative = relativePath.replace(/^\.?[\\/]+/g, '');
  return cleanCwd ? `${cleanCwd}\\${cleanRelative}` : cleanRelative;
}
function isSafeBranchName(value: string) { return /^[A-Za-z0-9._\/-]+$/.test(value) && !value.includes('..') && !value.startsWith('-'); }
function dedupe(targets: TerminalQuickSelectTarget[]) {
  const seen = new Set<string>();
  return targets.filter((target) => {
    const key = `${target.kind}:${target.index}:${target.target}:${target.line ?? ''}:${target.column ?? ''}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
