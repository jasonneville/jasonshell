import type { SearchPanelResult } from './searchPanel';

const MAX_RESULTS = 14;
const USAGE_STORAGE_KEY = 'jasonshell.search.usage';

type UsageMap = Record<string, number>;

export function rankSearchResults(results: SearchPanelResult[], query: string): SearchPanelResult[] {
  return rankSearchResultsWithUsage(results, query, readUsage());
}

export function rankSearchResultsWithUsage(
  results: SearchPanelResult[],
  query: string,
  usage: UsageMap
): SearchPanelResult[] {
  const tokens = normalize(query).split(' ').filter(Boolean);

  return results
    .map((result) => ({
      result,
      score: result.priority + scoreMatch(result, tokens) + (usage[result.id] ?? 0)
    }))
    .filter((entry) => tokens.length === 0 || entry.score > entry.result.priority)
    .sort((left, right) => right.score - left.score || left.result.title.localeCompare(right.result.title))
    .slice(0, MAX_RESULTS)
    .map((entry) => entry.result);
}

export function recordSearchUsage(resultId: string) {
  const usage = readUsage();
  usage[resultId] = Math.min((usage[resultId] ?? 0) + 8, 80);
  window.localStorage.setItem(USAGE_STORAGE_KEY, JSON.stringify(usage));
}

function scoreMatch(result: SearchPanelResult, tokens: string[]) {
  if (tokens.length === 0) {
    return 1;
  }

  const haystack = normalize(`${result.title} ${result.subtitle} ${result.terms}`);
  if (!tokens.every((token) => haystack.includes(token))) {
    return 0;
  }

  const title = normalize(result.title);
  const exactBoost = tokens.some((token) => title === token) ? 60 : 0;
  const prefixBoost = tokens.some((token) => title.startsWith(token)) ? 36 : 0;
  return 20 + exactBoost + prefixBoost;
}

function normalize(value: string) {
  return value.trim().toLocaleLowerCase().replace(/\s+/g, ' ');
}

function readUsage(): UsageMap {
  try {
    const value = window.localStorage.getItem(USAGE_STORAGE_KEY);
    return value ? JSON.parse(value) as UsageMap : {};
  } catch (_error) {
    return {};
  }
}
