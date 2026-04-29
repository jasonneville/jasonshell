import type { SearchPanelResult } from './searchPanel';

const MAX_RESULTS = 14;
const USAGE_STORAGE_KEY = 'jasonshell.search.usage';
const MAX_SCORE = 1_000_000;
const MAX_USAGE_BOOST = 80;
const TOP_MOST_BOOST = 10_000;

type UsageMap = Record<string, number>;
type RankableSearchPanelResult = SearchPanelResult & {
  topMost?: boolean;
  runCount?: number;
};

export function rankSearchResults(results: SearchPanelResult[], query: string): SearchPanelResult[] {
  return rankSearchResultsWithUsage(results, query, cachedUsage());
}

export function rankSearchResultsWithUsage(
  results: SearchPanelResult[],
  query: string,
  usage: UsageMap
): SearchPanelResult[] {
  const tokens = queryTokens(query);

  return collapseDuplicateResults(results)
    .map((result) => ({
      result,
      score: scoreSearchResult(result, tokens, usage)
    }))
    .filter((entry) => tokens.length === 0 || entry.score > entry.result.priority)
    .sort(compareRankedEntries)
    .slice(0, MAX_RESULTS)
    .map((entry) => entry.result);
}

export function recordSearchUsage(resultId: string) {
  const usage = cachedUsage();
  usage[resultId] = Math.min((usage[resultId] ?? 0) + 8, MAX_USAGE_BOOST);
  writeUsage(usage);
}

export function recordSearchResultUsage(result: SearchPanelResult) {
  recordSearchUsage(searchResultRecordKey(result));
}

export function setSearchUsageForTest(usage: UsageMap | null) {
  usageCache = usage ? { ...usage } : null;
}

export function searchResultRecordKey(result: SearchPanelResult): string {
  if (result.recordKey) {
    return result.recordKey;
  }
  if (result.path) {
    return `${result.kind}:${normalizePath(result.path)}`;
  }
  if (result.url) {
    return `${result.kind}:${result.url.trim().toLocaleLowerCase()}`;
  }
  return result.id;
}

export function collapseDuplicateResults(results: SearchPanelResult[]): SearchPanelResult[] {
  const byKey = new Map<string, SearchPanelResult>();
  for (const result of results) {
    const key = searchResultRecordKey(result);
    const current = byKey.get(key);
    if (!current || preferredDuplicate(result, current) === result) {
      byKey.set(key, result);
    }
  }
  return [...byKey.values()];
}

export function scoreSearchResult(
  result: SearchPanelResult,
  tokens: string[],
  usage: UsageMap
): number {
  const rankable = result as RankableSearchPanelResult;
  return capScore(
    result.priority +
      scoreMatch(result, tokens) +
      providerPriority(result.providerId) +
      resultTypePriority(result.kind) +
      usageBoost(result, usage) +
      everythingRunCountBoost(rankable) +
      (rankable.topMost ? TOP_MOST_BOOST : 0)
  );
}

function scoreMatch(result: SearchPanelResult, tokens: string[]) {
  if (tokens.length === 0) {
    return 1;
  }

  const haystack = normalize(`${result.title} ${result.subtitle} ${result.terms} ${result.path ?? ''}`);
  if (!tokens.every((token) => haystack.includes(token))) {
    return 0;
  }

  const title = normalize(result.title);
  const queryText = tokens.join(' ');
  const titleNoExtension = normalize(stripKnownExtension(result.title));
  const exactBoost = title === queryText || titleNoExtension === queryText ? 120 : 0;
  const tokenExactBoost = tokens.some((token) => title === token || titleNoExtension === token) ? 60 : 0;
  const prefixBoost = title.startsWith(queryText) || titleNoExtension.startsWith(queryText) ? 48 : 0;
  const containsBoost = tokens.some((token) => title.includes(token)) ? 20 : 0;
  return 20 + exactBoost + tokenExactBoost + prefixBoost + containsBoost;
}

function queryTokens(query: string): string[] {
  return normalize(query).split(' ').filter(Boolean);
}

function providerPriority(providerId: string | undefined): number {
  switch (providerId) {
    case 'everything':
      return 60;
    case 'apps':
      return 45;
    case 'openWindows':
      return 40;
    case 'commands':
      return 24;
    case 'warmedCache':
      return 4;
    case 'windowsSearch':
      return -20;
    default:
      return 0;
  }
}

function resultTypePriority(kind: SearchPanelResult['kind']): number {
  switch (kind) {
    case 'app':
      return 35;
    case 'window':
      return 30;
    case 'folder':
      return 26;
    case 'file':
      return 20;
    case 'command':
    case 'setting':
      return 16;
    case 'calculator':
      return 14;
    case 'web':
    case 'bookmark':
      return 8;
  }
}

function usageBoost(result: SearchPanelResult, usage: UsageMap): number {
  return Math.min(
    Math.max(usage[searchResultRecordKey(result)] ?? usage[result.id] ?? 0, 0),
    MAX_USAGE_BOOST
  );
}

function everythingRunCountBoost(result: RankableSearchPanelResult): number {
  if (result.providerId !== 'everything' || typeof result.runCount !== 'number') {
    return 0;
  }
  return Math.min(Math.max(Math.trunc(result.runCount), 0), 100);
}

function preferredDuplicate(
  left: SearchPanelResult,
  right: SearchPanelResult
): SearchPanelResult {
  const leftProvider = providerPriority(left.providerId);
  const rightProvider = providerPriority(right.providerId);
  if (leftProvider !== rightProvider) {
    return leftProvider > rightProvider ? left : right;
  }
  if (left.priority !== right.priority) {
    return left.priority > right.priority ? left : right;
  }
  return left.id.localeCompare(right.id) <= 0 ? left : right;
}

function compareRankedEntries(
  left: { result: SearchPanelResult; score: number },
  right: { result: SearchPanelResult; score: number }
): number {
  return (
    right.score - left.score ||
    left.result.title.localeCompare(right.result.title) ||
    left.result.id.localeCompare(right.result.id)
  );
}

function capScore(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.min(Math.trunc(value), MAX_SCORE));
}

function stripKnownExtension(value: string): string {
  return value.replace(/\.[a-z0-9]{1,8}$/iu, '');
}

function normalizePath(value: string): string {
  return value.trim().replace(/\//gu, '\\').toLocaleLowerCase();
}

function normalize(value: string) {
  return value.trim().toLocaleLowerCase().replace(/[_\-.]+/gu, ' ').replace(/\s+/gu, ' ');
}

let usageCache: UsageMap | null = null;

function cachedUsage(): UsageMap {
  if (usageCache === null) {
    usageCache = readUsage();
  }
  return usageCache;
}

function readUsage(): UsageMap {
  try {
    if (typeof window === 'undefined' || !window.localStorage) {
      return {};
    }
    const value = window.localStorage.getItem(USAGE_STORAGE_KEY);
    return value ? JSON.parse(value) as UsageMap : {};
  } catch (_error) {
    return {};
  }
}

function writeUsage(usage: UsageMap) {
  usageCache = { ...usage };
  try {
    if (typeof window !== 'undefined' && window.localStorage) {
      window.localStorage.setItem(USAGE_STORAGE_KEY, JSON.stringify(usageCache));
    }
  } catch (_error) {
    // Best-effort local ranking boost; search must keep working without storage.
  }
}
