export type StackTerminalDetectedLinkKind = 'url' | 'localhost' | 'windowsPath' | 'relativePath' | 'gitHash';

export type StackTerminalDetectedLink = {
  kind: StackTerminalDetectedLinkKind;
  text: string;
  target: string;
  index: number;
};

const URL_PATTERN = /\bhttps?:\/\/[^\s"'<>]+/gi;
const LOCALHOST_PATTERN = /\b(?:localhost|127\.0\.0\.1):\d{2,5}(?:\/[^\s"'<>]*)?/gi;
const WINDOWS_PATH_PATTERN = /\b[A-Za-z]:\\[^\r\n:*?"<>|]+/g;
const RELATIVE_PATH_PATTERN = /(?:^|[\s(["'`])((?:\.{1,2}[\\/])?(?:[\w.-]+[\\/])+[\w.-]+(?::\d+)?(?::\d+)?)/g;
const GIT_HASH_PATTERN = /\b[0-9a-f]{7,40}\b/gi;

export function detectStackTerminalLinks(text: string, cwd = ''): StackTerminalDetectedLink[] {
  const links: StackTerminalDetectedLink[] = [];
  addPatternLinks(links, text, URL_PATTERN, (match, index) => ({
    kind: match.toLowerCase().startsWith('http://localhost') || match.toLowerCase().startsWith('http://127.0.0.1')
      ? 'localhost'
      : 'url',
    text: match,
    target: trimTrailingPunctuation(match),
    index
  }));
  addPatternLinks(links, text, LOCALHOST_PATTERN, (match, index) => ({
    kind: 'localhost',
    text: match,
    target: `http://${trimTrailingPunctuation(match)}`,
    index
  }));
  addPatternLinks(links, text, WINDOWS_PATH_PATTERN, (match, index) => ({
    kind: 'windowsPath',
    text: match,
    target: trimPathLineColumn(trimTrailingPunctuation(match)),
    index
  }));
  let relativeMatch: RegExpExecArray | null;
  RELATIVE_PATH_PATTERN.lastIndex = 0;
  while ((relativeMatch = RELATIVE_PATH_PATTERN.exec(text))) {
    const raw = relativeMatch[1];
    const prefixLength = relativeMatch[0].length - raw.length;
    const normalized = raw.replace(/\//g, '\\');
    links.push({
      kind: 'relativePath',
      text: raw,
      target: cwd ? joinTerminalPath(cwd, trimPathLineColumn(normalized)) : trimPathLineColumn(normalized),
      index: relativeMatch.index + prefixLength
    });
  }
  addPatternLinks(links, text, GIT_HASH_PATTERN, (match, index) => ({
    kind: 'gitHash',
    text: match,
    target: match,
    index
  }));
  return dedupeLinks(links).sort((left, right) => left.index - right.index);
}

export function isSafeStackTerminalOpenTarget(link: StackTerminalDetectedLink): boolean {
  if (link.kind === 'gitHash') {
    return /^[0-9a-f]{7,40}$/i.test(link.target);
  }
  if (link.kind === 'url' || link.kind === 'localhost') {
    return /^https?:\/\//i.test(link.target);
  }
  return /^[A-Za-z]:\\/.test(link.target) && !/[<>|"?*]/.test(link.target);
}

function addPatternLinks(
  links: StackTerminalDetectedLink[],
  text: string,
  pattern: RegExp,
  map: (match: string, index: number) => StackTerminalDetectedLink
) {
  pattern.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(text))) {
    links.push(map(match[0], match.index));
  }
}

function trimTrailingPunctuation(value: string) {
  return value.replace(/[),.;:]+$/g, '');
}

function trimPathLineColumn(value: string) {
  return value.replace(/(?::\d+){1,2}$/g, '');
}

function joinTerminalPath(cwd: string, relativePath: string) {
  const cleanedCwd = cwd.replace(/[\\/]+$/g, '');
  const cleanedRelative = relativePath.replace(/^\.?[\\/]+/g, '');
  return `${cleanedCwd}\\${cleanedRelative}`;
}

function dedupeLinks(links: StackTerminalDetectedLink[]) {
  const seen = new Set<string>();
  return links.filter((link) => {
    const key = `${link.kind}:${link.index}:${link.target}`;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}
