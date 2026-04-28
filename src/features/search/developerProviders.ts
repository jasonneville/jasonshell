import type { ProcessInfo } from '../../lib/processManager';
import type { SearchPanelResult, SearchPanelResultKind } from '../../lib/searchPanel';

export const DEVELOPER_PROVIDER_IDS = [
  'workspace-files',
  'recent-files',
  'git-changes',
  'task-history',
  'commands',
  'settings',
  'processes',
  'saved-searches'
] as const;

export type DeveloperProviderId = (typeof DEVELOPER_PROVIDER_IDS)[number];
export type DeveloperWorkspacePolicy = 'prefer-active' | 'active-only' | 'all';
export type SavedSearchScopeKind = 'global' | 'workspace';

export type DeveloperWorkspaceContext = {
  activeWorkspaceId?: string | null;
  activeWorkspaceRoot?: string | null;
};

export type DeveloperSearchOptions = {
  perProviderLimit?: number;
  totalLimit?: number;
  workspacePolicy?: DeveloperWorkspacePolicy;
};

export type DeveloperFileItem = {
  id?: string;
  title?: string;
  path: string;
  workspaceId?: string | null;
  workspaceRoot?: string | null;
  isDirectory?: boolean;
  modifiedAtMs?: number | null;
  lastOpenedAtMs?: number | null;
  language?: string | null;
  terms?: string;
};

export type DeveloperGitChangeItem = {
  path: string;
  status: 'added' | 'deleted' | 'modified' | 'renamed' | 'untracked' | 'conflicted';
  workspaceId?: string | null;
  workspaceRoot?: string | null;
  staged?: boolean;
  previousPath?: string | null;
};

export type DeveloperTaskHistoryItem = {
  id: string;
  command: string;
  cwd?: string | null;
  workspaceId?: string | null;
  startedAtMs?: number | null;
  exitCode?: number | null;
};

export type DeveloperCommandItem = {
  id: string;
  title: string;
  subtitle?: string;
  terms?: string;
  command?: string;
  workspaceId?: string | null;
};

export type DeveloperSettingItem = {
  key: string;
  title: string;
  scope: SavedSearchScopeKind;
  workspaceId?: string | null;
  terms?: string;
};

export type DeveloperProcessItem = ProcessInfo & {
  workspaceId?: string | null;
  cwd?: string | null;
  commandLine?: string | null;
};

export type SavedSearchRecord = {
  id: string;
  name: string;
  query: string;
  scope: SavedSearchScopeKind;
  workspaceId?: string | null;
  workspaceRoot?: string | null;
  providerIds?: DeveloperProviderId[];
  createdAtMs?: number | null;
  updatedAtMs?: number | null;
};

export type DeveloperSearchSnapshot = {
  workspaceFiles?: readonly DeveloperFileItem[];
  recentFiles?: readonly DeveloperFileItem[];
  gitChanges?: readonly DeveloperGitChangeItem[];
  taskHistory?: readonly DeveloperTaskHistoryItem[];
  commands?: readonly DeveloperCommandItem[];
  settings?: readonly DeveloperSettingItem[];
  processes?: readonly DeveloperProcessItem[];
  savedSearches?: readonly SavedSearchRecord[];
};

export type DeveloperSearchProviderResult = SearchPanelResult & {
  providerId: DeveloperProviderId;
  workspaceId?: string | null;
  workspaceRoot?: string | null;
  workspaceMatch: boolean;
  persistedScope?: SavedSearchScopeKind;
};

export type DeveloperSearchProviderGroup = {
  providerId: DeveloperProviderId;
  label: string;
  results: DeveloperSearchProviderResult[];
};

export type DeveloperSearchResponse = {
  query: string;
  groups: DeveloperSearchProviderGroup[];
  results: DeveloperSearchProviderResult[];
};

export const SAVED_SEARCH_PERSISTENCE_CONTRACT = {
  settingsSchema: 'jasonshell.settings',
  settingsVersion: 1,
  globalField: 'savedSearches',
  workspaceField: 'workspaces[].savedSearches',
  allowedScopes: ['global', 'workspace'],
  secretStorageAllowed: false
} as const;

const DEFAULT_PER_PROVIDER_LIMIT = 8;
const DEFAULT_TOTAL_LIMIT = 40;

const PROVIDER_LABELS: Record<DeveloperProviderId, string> = {
  'workspace-files': 'Workspace Files',
  'recent-files': 'Recent Files',
  'git-changes': 'Git Changes',
  'task-history': 'Task History',
  commands: 'Commands',
  settings: 'Settings',
  processes: 'Processes',
  'saved-searches': 'Saved Searches'
};

const PROVIDER_PRIORITY: Record<DeveloperProviderId, number> = {
  'workspace-files': 190,
  'git-changes': 180,
  'recent-files': 165,
  'saved-searches': 150,
  commands: 135,
  'task-history': 120,
  processes: 105,
  settings: 95
};

export function buildDeveloperSearchProviders(
  snapshot: DeveloperSearchSnapshot,
  context: DeveloperWorkspaceContext,
  query: string,
  options: DeveloperSearchOptions = {}
): DeveloperSearchResponse {
  const tokens = tokenize(query);
  const perProviderLimit = positiveInteger(options.perProviderLimit, DEFAULT_PER_PROVIDER_LIMIT);
  const totalLimit = positiveInteger(options.totalLimit, DEFAULT_TOTAL_LIMIT);
  const workspacePolicy = options.workspacePolicy ?? 'prefer-active';
  const resultsByProvider = new Map<DeveloperProviderId, DeveloperSearchProviderResult[]>();

  for (const result of [
    ...workspaceFileResults(snapshot.workspaceFiles ?? [], context),
    ...recentFileResults(snapshot.recentFiles ?? [], context),
    ...gitChangeResults(snapshot.gitChanges ?? [], context),
    ...taskHistoryResults(snapshot.taskHistory ?? [], context),
    ...commandResults(snapshot.commands ?? [], context),
    ...settingResults(snapshot.settings ?? [], context),
    ...processResults(snapshot.processes ?? [], context),
    ...savedSearchResults(snapshot.savedSearches ?? [], context)
  ]) {
    if (!matchesWorkspacePolicy(result, context, workspacePolicy) || !matchesQuery(result, tokens)) {
      continue;
    }
    const next = resultsByProvider.get(result.providerId) ?? [];
    next.push({ ...result, priority: scoreResult(result, tokens) });
    resultsByProvider.set(result.providerId, next);
  }

  const limitedByProvider = new Map<DeveloperProviderId, DeveloperSearchProviderResult[]>();
  for (const providerId of DEVELOPER_PROVIDER_IDS) {
    const ranked = (resultsByProvider.get(providerId) ?? [])
      .sort(compareDeveloperResults)
      .slice(0, perProviderLimit);
    if (ranked.length) {
      limitedByProvider.set(providerId, ranked);
    }
  }

  const results = [...limitedByProvider.values()]
    .flat()
    .sort(compareDeveloperResults)
    .slice(0, totalLimit);
  const allowedIds = new Set(results.map((result) => result.id));
  const groups = DEVELOPER_PROVIDER_IDS
    .map((providerId) => ({
      providerId,
      label: PROVIDER_LABELS[providerId],
      results: (limitedByProvider.get(providerId) ?? []).filter((result) => allowedIds.has(result.id))
    }))
    .filter((group) => group.results.length > 0);

  return { query, groups, results };
}

export function filterSavedSearchesForScope(
  savedSearches: readonly SavedSearchRecord[],
  context: DeveloperWorkspaceContext,
  workspacePolicy: DeveloperWorkspacePolicy = 'prefer-active'
): SavedSearchRecord[] {
  return savedSearches.filter((savedSearch) => {
    if (savedSearch.scope === 'global') {
      return true;
    }
    if (workspacePolicy === 'all') {
      return true;
    }
    return isActiveWorkspace(savedSearch, context);
  });
}

function workspaceFileResults(
  items: readonly DeveloperFileItem[],
  context: DeveloperWorkspaceContext
): DeveloperSearchProviderResult[] {
  return items.map((item) => fileResult('workspace-files', item, context, 'Workspace file'));
}

function recentFileResults(
  items: readonly DeveloperFileItem[],
  context: DeveloperWorkspaceContext
): DeveloperSearchProviderResult[] {
  return items.map((item) => fileResult('recent-files', item, context, 'Recent file'));
}

function fileResult(
  providerId: 'workspace-files' | 'recent-files',
  item: DeveloperFileItem,
  context: DeveloperWorkspaceContext,
  subtitle: string
): DeveloperSearchProviderResult {
  const title = item.title ?? basename(item.path);
  const kind: SearchPanelResultKind = item.isDirectory ? 'folder' : 'file';
  return {
    id: `developer:${providerId}:${item.id ?? item.path}`,
    kind,
    path: item.path,
    providerId,
    title,
    subtitle: item.language ? `${subtitle} - ${item.language}` : subtitle,
    terms: compactTerms(title, item.path, item.terms, item.language),
    priority: PROVIDER_PRIORITY[providerId] + recencyBonus(item.lastOpenedAtMs ?? item.modifiedAtMs),
    workspaceId: item.workspaceId,
    workspaceRoot: item.workspaceRoot,
    workspaceMatch: isActiveWorkspace(item, context)
  };
}

function gitChangeResults(
  items: readonly DeveloperGitChangeItem[],
  context: DeveloperWorkspaceContext
): DeveloperSearchProviderResult[] {
  return items.map((item) => ({
    id: `developer:git-changes:${item.path}`,
    kind: 'file',
    path: item.path,
    providerId: 'git-changes',
    title: basename(item.path),
    subtitle: `${gitStatusLabel(item.status)}${item.staged ? ' - staged' : ''}`,
    terms: compactTerms(item.path, item.previousPath, item.status, item.staged ? 'staged' : 'unstaged', 'git change'),
    priority: PROVIDER_PRIORITY['git-changes'] + (item.staged ? 8 : 0),
    workspaceId: item.workspaceId,
    workspaceRoot: item.workspaceRoot,
    workspaceMatch: isActiveWorkspace(item, context)
  }));
}

function taskHistoryResults(
  items: readonly DeveloperTaskHistoryItem[],
  context: DeveloperWorkspaceContext
): DeveloperSearchProviderResult[] {
  return items.map((item) => ({
    id: `developer:task-history:${item.id}`,
    kind: 'command',
    path: item.cwd ?? undefined,
    providerId: 'task-history',
    title: item.command,
    subtitle: item.exitCode === null || item.exitCode === undefined ? 'Task history' : `Exit ${item.exitCode}`,
    terms: compactTerms(item.command, item.cwd, item.exitCode),
    priority: PROVIDER_PRIORITY['task-history'] + recencyBonus(item.startedAtMs),
    workspaceId: item.workspaceId,
    workspaceMatch: isActiveWorkspace(item, context)
  }));
}

function commandResults(
  items: readonly DeveloperCommandItem[],
  context: DeveloperWorkspaceContext
): DeveloperSearchProviderResult[] {
  return items.map((item) => ({
    id: `developer:commands:${item.id}`,
    kind: 'command',
    providerId: 'commands',
    title: item.title,
    subtitle: item.subtitle ?? 'Developer command',
    terms: compactTerms(item.title, item.subtitle, item.command, item.terms),
    priority: PROVIDER_PRIORITY.commands,
    workspaceId: item.workspaceId,
    workspaceMatch: isActiveWorkspace(item, context)
  }));
}

function settingResults(
  items: readonly DeveloperSettingItem[],
  context: DeveloperWorkspaceContext
): DeveloperSearchProviderResult[] {
  return items.map((item) => ({
    id: `developer:settings:${item.key}`,
    kind: 'command',
    providerId: 'settings',
    title: item.title,
    subtitle: `${item.scope === 'workspace' ? 'Workspace' : 'Global'} setting`,
    terms: compactTerms(item.key, item.title, item.scope, item.terms),
    priority: PROVIDER_PRIORITY.settings,
    workspaceId: item.workspaceId,
    workspaceMatch: isActiveWorkspace(item, context),
    persistedScope: item.scope
  }));
}

function processResults(
  items: readonly DeveloperProcessItem[],
  context: DeveloperWorkspaceContext
): DeveloperSearchProviderResult[] {
  return items.map((item) => ({
    id: `developer:processes:${item.pid}`,
    kind: 'window',
    path: item.executablePath ?? item.cwd ?? undefined,
    providerId: 'processes',
    title: item.name,
    subtitle: `PID ${item.pid}${item.status ? ` - ${item.status}` : ''}`,
    terms: compactTerms(item.name, item.pid, item.parentPid, item.executablePath, item.cwd, item.commandLine),
    priority: PROVIDER_PRIORITY.processes + (item.isKillable ? 0 : -5),
    workspaceId: item.workspaceId,
    workspaceMatch: isActiveWorkspace(item, context)
  }));
}

function savedSearchResults(
  items: readonly SavedSearchRecord[],
  context: DeveloperWorkspaceContext
): DeveloperSearchProviderResult[] {
  return filterSavedSearchesForScope(items, context).map((item) => ({
    id: `developer:saved-searches:${item.id}`,
    kind: 'command',
    providerId: 'saved-searches',
    title: item.name,
    subtitle: `Saved search - ${item.query}`,
    terms: compactTerms(item.name, item.query, item.providerIds?.join(' '), item.scope),
    priority: PROVIDER_PRIORITY['saved-searches'] + recencyBonus(item.updatedAtMs ?? item.createdAtMs),
    workspaceId: item.workspaceId,
    workspaceRoot: item.workspaceRoot,
    workspaceMatch: isActiveWorkspace(item, context),
    persistedScope: item.scope
  }));
}

function matchesWorkspacePolicy(
  result: DeveloperSearchProviderResult,
  context: DeveloperWorkspaceContext,
  policy: DeveloperWorkspacePolicy
): boolean {
  if (policy === 'all' || !hasActiveWorkspace(context)) {
    return true;
  }
  if (!hasWorkspaceScope(result)) {
    return true;
  }
  if (policy === 'active-only') {
    return result.workspaceMatch;
  }
  return true;
}

function matchesQuery(result: DeveloperSearchProviderResult, tokens: readonly string[]): boolean {
  if (!tokens.length) {
    return true;
  }
  const haystack = normalizeText(compactTerms(result.title, result.subtitle, result.path, result.terms));
  return tokens.every((token) => haystack.includes(token));
}

function scoreResult(result: DeveloperSearchProviderResult, tokens: readonly string[]): number {
  let score = result.priority + (result.workspaceMatch ? 35 : 0);
  const normalizedTitle = normalizeText(result.title);
  const normalizedTerms = normalizeText(result.terms);

  for (const token of tokens) {
    if (normalizedTitle === token) {
      score += 45;
    } else if (normalizedTitle.startsWith(token)) {
      score += 30;
    } else if (normalizedTitle.includes(token)) {
      score += 16;
    } else if (normalizedTerms.includes(token)) {
      score += 8;
    }
  }

  return score;
}

function compareDeveloperResults(
  left: DeveloperSearchProviderResult,
  right: DeveloperSearchProviderResult
): number {
  if (left.priority !== right.priority) {
    return right.priority - left.priority;
  }
  return left.title.localeCompare(right.title, undefined, { numeric: true, sensitivity: 'base' });
}

function isActiveWorkspace(
  item: { workspaceId?: string | null; workspaceRoot?: string | null; path?: string | null; cwd?: string | null },
  context: DeveloperWorkspaceContext
): boolean {
  if (context.activeWorkspaceId && item.workspaceId && item.workspaceId === context.activeWorkspaceId) {
    return true;
  }

  if (!context.activeWorkspaceRoot) {
    return false;
  }

  const activeRoot = normalizePath(context.activeWorkspaceRoot);
  const candidateRoot = normalizePath(item.workspaceRoot ?? item.cwd ?? item.path ?? '');
  return Boolean(candidateRoot && (candidateRoot === activeRoot || candidateRoot.startsWith(`${activeRoot}\\`)));
}

function hasWorkspaceScope(result: DeveloperSearchProviderResult): boolean {
  return Boolean(result.workspaceId || result.workspaceRoot || result.path);
}

function hasActiveWorkspace(context: DeveloperWorkspaceContext): boolean {
  return Boolean(context.activeWorkspaceId || context.activeWorkspaceRoot);
}

function tokenize(query: string): string[] {
  return normalizeText(query).split(' ').filter(Boolean);
}

function normalizeText(value: string): string {
  return value.trim().toLocaleLowerCase().replace(/\s+/g, ' ');
}

function normalizePath(value: string): string {
  return value.trim().replace(/\//g, '\\').replace(/\\+$/g, '').toLocaleLowerCase();
}

function basename(path: string): string {
  const normalized = path.replace(/\//g, '\\').replace(/\\+$/g, '');
  return normalized.split('\\').filter(Boolean).at(-1) ?? normalized;
}

function compactTerms(...parts: Array<string | number | null | undefined>): string {
  return parts
    .filter((part) => part !== null && part !== undefined && `${part}`.trim().length > 0)
    .join(' ');
}

function positiveInteger(value: number | undefined, fallback: number): number {
  return value !== undefined && Number.isFinite(value) && value > 0 ? Math.floor(value) : fallback;
}

function recencyBonus(epochMs: number | null | undefined): number {
  if (typeof epochMs !== 'number' || !Number.isFinite(epochMs)) {
    return 0;
  }
  const ageMs = Math.max(0, Date.now() - epochMs);
  const dayMs = 24 * 60 * 60 * 1000;
  return Math.max(0, 20 - Math.floor(ageMs / dayMs));
}

function gitStatusLabel(status: DeveloperGitChangeItem['status']): string {
  switch (status) {
    case 'added':
      return 'Added';
    case 'deleted':
      return 'Deleted';
    case 'modified':
      return 'Modified';
    case 'renamed':
      return 'Renamed';
    case 'untracked':
      return 'Untracked';
    case 'conflicted':
      return 'Conflicted';
  }
}
