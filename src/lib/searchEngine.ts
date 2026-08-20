import { invoke } from '@tauri-apps/api/core';
import type { SearchActivationKind, SearchPanelPayload, SearchPanelResult } from './searchPanel';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const SEARCH_ENGINE_COMMAND = IPC_COMMANDS.searchEngine;
export const SEARCH_ENGINE_PROGRESS_EVENT = 'search-engine:progress';
export const SEARCH_INDEX_REFRESHED_EVENT = 'search-index:refreshed';

export const SEARCH_ENGINE_PRESENTATIONS = ['anchored', 'centered'] as const;
export type SearchEnginePresentation = (typeof SEARCH_ENGINE_PRESENTATIONS)[number];

export const SEARCH_ENGINE_RESULT_KINDS = [
  'app',
  'window',
  'folder',
  'file',
  'command',
  'setting',
  'calculator',
  'web',
  'bookmark'
] as const;
export type SearchResultKind = (typeof SEARCH_ENGINE_RESULT_KINDS)[number];

export const SEARCH_PROGRESS_PHASES = ['typing', 'local', 'provider', 'complete', 'error'] as const;
export type SearchProgressPhase = (typeof SEARCH_PROGRESS_PHASES)[number];

export const SEARCH_PROVIDER_CACHE_STATES = ['hit', 'miss', 'refresh', 'indexing', 'disabled'] as const;
export type SearchProviderCacheState = (typeof SEARCH_PROVIDER_CACHE_STATES)[number];

export const SEARCH_ACTION_KINDS = [
  'openApp',
  'focusWindow',
  'openFile',
  'openFolder',
  'runCommand',
  'openSetting',
  'runControlPanel',
  'copyText',
  'openWebUrl',
  'openBookmark'
] as const;
export type SearchActionKind = (typeof SEARCH_ACTION_KINDS)[number];

export type SearchOpenWindowContext = {
  id: string;
  title: string;
  appName?: string;
  executablePath?: string;
  iconDataUrl?: string;
};

export type SearchQueryContext = {
  openWindows?: SearchOpenWindowContext[];
  workspaceRoots?: string[];
};

export type SearchQueryRequest = {
  query: string;
  sequence: number;
  limit: number;
  presentation: SearchEnginePresentation;
  context?: SearchQueryContext;
};

export type SearchAction =
  | { kind: 'openApp'; path: string }
  | { kind: 'focusWindow'; windowId: string }
  | { kind: 'openFile'; path: string }
  | { kind: 'openFolder'; path: string }
  | { kind: 'runCommand'; commandId: string }
  | { kind: 'openSetting'; uri: string }
  | { kind: 'runControlPanel'; executable: 'control.exe'; args?: string[] }
  | { kind: 'copyText'; text: string }
  | { kind: 'openWebUrl'; url: string }
  | { kind: 'openBookmark'; url: string };

export type SearchResult = {
  id: string;
  providerId: string;
  kind: SearchResultKind;
  title: string;
  subtitle?: string;
  path?: string;
  action: SearchAction;
  terms: string[];
  aliases: string[];
  score: number;
  matchReason: string;
  recordKey: string;
  titleHighlightData?: number[];
  subtitleHighlightData?: number[];
  iconDataUrl?: string;
};

export type SearchProviderTiming = {
  providerId: string;
  startedAt: string;
  endedAt?: string;
  durationMs: number;
  cache: SearchProviderCacheState;
  cacheAgeMs?: number;
  resultCount: number;
  applied: boolean;
  discardedAsStale: boolean;
};

export type SearchProviderHealth = {
  providerId: string;
  state: 'ready' | 'degraded' | 'unavailable' | 'indexing' | 'disabled';
  reasonCode?: string;
  message?: string;
};

export type SearchIndexRefreshedPayload = {
  providerId?: string;
  entryCount?: number;
  generatedAtEpochSecs?: number;
};

export type SearchEngineResponse = {
  query: string;
  sequence: number;
  results: SearchResult[];
  providerTimings: SearchProviderTiming[];
  health: SearchProviderHealth[];
  generatedAt: string;
  diagnostics?: SearchDiagnostics;
};

export type SearchDiagnostics = {
  coordinator: string;
  legacyHotPathUsed: boolean;
  notes?: string[];
};

export type SearchProgressPayload = {
  query: string;
  sequence: number;
  phase: SearchProgressPhase;
  results: SearchResult[];
  providerTimings?: SearchProviderTiming[];
  statusMessage: string;
  generatedAt: string;
  stale?: boolean;
};

export function searchEngine(request: SearchQueryRequest): Promise<SearchEngineResponse> {
  if (!isSearchQueryRequest(request)) {
    return Promise.reject(new Error('Invalid search engine request'));
  }
  return invoke<SearchEngineResponse>(SEARCH_ENGINE_COMMAND, { request }).then((response) => {
    if (!isSearchEngineResponse(response)) {
      throw new Error('Invalid search engine response');
    }
    return response;
  });
}

export function searchEngineResultToPanelResult(result: SearchResult): SearchPanelResult {
  return {
    id: result.id,
    providerId: panelProviderId(result.providerId),
    kind: result.kind,
    title: result.title,
    subtitle: result.subtitle ?? result.matchReason,
    terms: [...result.terms, ...result.aliases, result.title, result.subtitle ?? '', result.path ?? '']
      .filter(Boolean)
      .join(' '),
    priority: Math.round(result.score),
    score: result.score,
    iconDataUrl: result.iconDataUrl,
    path: panelResultPath(result),
    url: panelResultUrl(result),
    actionId: panelActionId(result.action),
    actionArgs: result.action.kind === 'runControlPanel' ? result.action.args : undefined,
    copyText: result.action.kind === 'copyText' ? result.action.text : undefined,
    titleHighlightData: result.titleHighlightData,
    subtitleHighlightData: result.subtitleHighlightData,
    recordKey: result.recordKey
  };
}

export function searchEngineResponseToPanelResults(response: SearchEngineResponse): SearchPanelResult[] {
  return response.results.map(searchEngineResultToPanelResult);
}

export function searchEngineResponseToPanelPayload(
  response: SearchEngineResponse,
  selectedIndex = 0,
  presentation: SearchEnginePresentation = 'centered'
): SearchPanelPayload {
  const results = searchEngineResponseToPanelResults(response);
  return {
    query: response.query,
    results,
    selectedIndex: Math.min(selectedIndex, Math.max(results.length - 1, 0)),
    statusMessage: searchPanelStatusMessageFromResponse(response, results),
    presentation
  };
}

export function searchEngineProgressToPanelPayload(
  payload: SearchProgressPayload,
  selectedIndex = 0,
  presentation: SearchEnginePresentation = 'centered'
): SearchPanelPayload {
  const results = payload.results.map(searchEngineResultToPanelResult);
  return {
    query: payload.query,
    results,
    selectedIndex: Math.min(selectedIndex, Math.max(results.length - 1, 0)),
    statusMessage: searchPanelStatusMessageFromProgress(payload, results),
    presentation,
    phase: payload.phase,
    sequence: payload.sequence
  };
}

export function searchPanelStatusMessageFromResponse(
  response: Pick<SearchEngineResponse, 'health' | 'providerTimings'>,
  results: readonly SearchPanelResult[]
): string {
  if (response.providerTimings.some((timing) => timing.providerId === 'apps' && timing.cache === 'indexing')) {
    return 'Apps index warming — results may update';
  }

  const everythingHealth = response.health.find((health) => health.providerId === 'everything');
  if (everythingHealth) {
    return searchPanelEverythingStatusMessage(everythingHealth, results.length);
  }

  return results.length ? '' : 'No search results matched';
}

export function searchPanelStatusMessageFromProgress(
  payload: Pick<SearchProgressPayload, 'statusMessage' | 'providerTimings'>,
  results: readonly SearchPanelResult[]
): string {
  if (payload.providerTimings?.some((timing) => timing.providerId === 'apps' && timing.cache === 'indexing')) {
    return 'Apps index warming — results may update';
  }

  const normalized = payload.statusMessage.trim();
  if (!normalized) {
    return results.length ? '' : 'No search results matched';
  }
  if (normalized === 'Showing search results' && results.length > 0) {
    return '';
  }

  const sanitized = searchPanelEverythingStatusMessageFromText(normalized);
  if (sanitized) {
    return sanitized;
  }

  return normalized;
}

export function mergeSearchPanelResultsByStableKey(
  current: readonly SearchPanelResult[],
  incoming: readonly SearchPanelResult[]
): SearchPanelResult[] {
  const merged = new Map<string, SearchPanelResult>();
  const order: string[] = [];

  for (const result of current) {
    const key = stablePanelResultKey(result);
    if (!merged.has(key)) {
      order.push(key);
    }
    merged.set(key, result);
  }

  for (const result of incoming) {
    const key = stablePanelResultKey(result);
    if (!merged.has(key)) {
      order.push(key);
    }
    const currentResult = merged.get(key);
    merged.set(key, currentResult ? mergePanelResult(currentResult, result) : result);
  }

  return order.map((key) => merged.get(key)).filter((result): result is SearchPanelResult => Boolean(result));
}

export function isSearchQueryRequest(value: unknown): value is SearchQueryRequest {
  const record = asRecord(value);
  const sequence = record?.sequence;
  const limit = record?.limit;
  return Boolean(
    record &&
      typeof record.query === 'string' &&
      typeof sequence === 'number' &&
      Number.isInteger(sequence) &&
      sequence >= 0 &&
      typeof limit === 'number' &&
      Number.isInteger(limit) &&
      limit >= 1 &&
      limit <= 100 &&
      enumValue(SEARCH_ENGINE_PRESENTATIONS, record.presentation) &&
      (record.context === undefined || isSearchQueryContext(record.context))
  );
}

export function isSearchEngineResponse(value: unknown): value is SearchEngineResponse {
  const record = asRecord(value);
  return Boolean(
    record &&
      typeof record.query === 'string' &&
      Number.isInteger(record.sequence) &&
      Array.isArray(record.results) &&
      record.results.every(isSearchResult) &&
      Array.isArray(record.providerTimings) &&
      record.providerTimings.every(isSearchProviderTiming) &&
      Array.isArray(record.health) &&
      record.health.every(isSearchProviderHealth) &&
      typeof record.generatedAt === 'string' &&
      isIsoDate(record.generatedAt) &&
      (record.diagnostics === undefined || isSearchDiagnostics(record.diagnostics))
  );
}

export function isSearchProgressPayload(value: unknown): value is SearchProgressPayload {
  const record = asRecord(value);
  return Boolean(
    record &&
      typeof record.query === 'string' &&
      Number.isInteger(record.sequence) &&
      enumValue(SEARCH_PROGRESS_PHASES, record.phase) &&
      Array.isArray(record.results) &&
      record.results.every(isSearchResult) &&
      (record.providerTimings === undefined ||
        (Array.isArray(record.providerTimings) &&
          record.providerTimings.every(isSearchProviderTiming))) &&
      typeof record.statusMessage === 'string' &&
      typeof record.generatedAt === 'string' &&
      isIsoDate(record.generatedAt) &&
      (record.stale === undefined || typeof record.stale === 'boolean')
  );
}

export function isSearchResult(value: unknown): value is SearchResult {
  const record = asRecord(value);
  const iconDataUrl = record?.iconDataUrl;
  return Boolean(
    record &&
      typeof record.id === 'string' &&
      typeof record.providerId === 'string' &&
      enumValue(SEARCH_ENGINE_RESULT_KINDS, record.kind) &&
      typeof record.title === 'string' &&
      (record.subtitle === undefined || typeof record.subtitle === 'string') &&
      (record.path === undefined || typeof record.path === 'string') &&
      isSearchAction(record.action) &&
      Array.isArray(record.terms) &&
      record.terms.every((term) => typeof term === 'string') &&
      Array.isArray(record.aliases) &&
      record.aliases.every((alias) => typeof alias === 'string') &&
      typeof record.score === 'number' &&
      Number.isFinite(record.score) &&
      typeof record.matchReason === 'string' &&
      typeof record.recordKey === 'string' &&
      isOptionalHighlightData(record.titleHighlightData) &&
      isOptionalHighlightData(record.subtitleHighlightData) &&
      (iconDataUrl === undefined || (typeof iconDataUrl === 'string' && isSafeDataUrl(iconDataUrl)))
  );
}

export function isSearchProviderTiming(value: unknown): value is SearchProviderTiming {
  const record = asRecord(value);
  const resultCount = record?.resultCount;
  return Boolean(
    record &&
      typeof record.providerId === 'string' &&
      typeof record.startedAt === 'string' &&
      isIsoDate(record.startedAt) &&
      (record.endedAt === undefined ||
        (typeof record.endedAt === 'string' && isIsoDate(record.endedAt))) &&
      typeof record.durationMs === 'number' &&
      Number.isFinite(record.durationMs) &&
      record.durationMs >= 0 &&
      enumValue(SEARCH_PROVIDER_CACHE_STATES, record.cache) &&
      (record.cacheAgeMs === undefined ||
        (typeof record.cacheAgeMs === 'number' &&
          Number.isFinite(record.cacheAgeMs) &&
          record.cacheAgeMs >= 0)) &&
      typeof resultCount === 'number' &&
      Number.isInteger(resultCount) &&
      resultCount >= 0 &&
      typeof record.applied === 'boolean' &&
      typeof record.discardedAsStale === 'boolean'
  );
}

export function isSearchAction(value: unknown): value is SearchAction {
  const record = asRecord(value);
  if (!record || !enumValue(SEARCH_ACTION_KINDS, record.kind)) {
    return false;
  }
  switch (record.kind) {
    case 'openApp':
    case 'openFile':
    case 'openFolder':
      return typeof record.path === 'string' && record.path.trim().length > 0;
    case 'focusWindow':
      return typeof record.windowId === 'string' && record.windowId.trim().length > 0;
    case 'runCommand':
      return typeof record.commandId === 'string' && /^[a-z0-9:_-]+$/iu.test(record.commandId);
    case 'openSetting':
      return typeof record.uri === 'string' && isSafeMsSettingsUri(record.uri);
    case 'runControlPanel':
      return isSafeControlPanelAction(record);
    case 'copyText':
      return typeof record.text === 'string';
    case 'openWebUrl':
    case 'openBookmark':
      return typeof record.url === 'string' && isSafeHttpUrl(record.url);
  }
}

export function validateSearchResultActionSafety(result: Pick<SearchResult, 'kind' | 'action'>): boolean {
  if (result.kind === 'setting') {
    return (
      (result.action.kind === 'openSetting' && isSafeMsSettingsUri(result.action.uri)) ||
      (result.action.kind === 'runControlPanel' && isSafeControlPanelAction(result.action))
    );
  }
  return isSearchAction(result.action);
}

export function isAppSearchIndexRefreshedPayload(value: unknown): value is SearchIndexRefreshedPayload {
  const record = asRecord(value);
  return Boolean(
    record &&
      record.providerId === 'apps' &&
      typeof record.entryCount === 'number' &&
      Number.isInteger(record.entryCount) &&
      record.entryCount >= 0 &&
      typeof record.generatedAtEpochSecs === 'number' &&
      Number.isInteger(record.generatedAtEpochSecs) &&
      record.generatedAtEpochSecs > 0
  );
}

export function isSafeMsSettingsUri(uri: string): boolean {
  return /^ms-settings:[a-z0-9_.-]*$/iu.test(uri);
}

export function isSafeControlPanelAction(action: unknown): action is Extract<SearchAction, { kind: 'runControlPanel' }> {
  const record = asRecord(action);
  return Boolean(
    record &&
      record.kind === 'runControlPanel' &&
      typeof record.executable === 'string' &&
      record.executable.toLowerCase() === 'control.exe' &&
      (record.args === undefined ||
        (Array.isArray(record.args) &&
          record.args.every((arg) => typeof arg === 'string' && /^[a-z0-9_.{},-]+$/iu.test(arg))))
  );
}

function isSearchQueryContext(value: unknown): value is SearchQueryContext {
  const record = asRecord(value);
  return Boolean(
    record &&
      (record.openWindows === undefined ||
        (Array.isArray(record.openWindows) && record.openWindows.every(isSearchOpenWindowContext))) &&
      (record.workspaceRoots === undefined ||
        (Array.isArray(record.workspaceRoots) &&
          record.workspaceRoots.every((root) => typeof root === 'string')))
  );
}

function isSearchOpenWindowContext(value: unknown): value is SearchOpenWindowContext {
  const record = asRecord(value);
  const iconDataUrl = record?.iconDataUrl;
  return Boolean(
    record &&
      typeof record.id === 'string' &&
      typeof record.title === 'string' &&
      (record.appName === undefined || typeof record.appName === 'string') &&
      (record.executablePath === undefined || typeof record.executablePath === 'string') &&
      (iconDataUrl === undefined || (typeof iconDataUrl === 'string' && isSafeDataUrl(iconDataUrl)))
  );
}

function isSearchProviderHealth(value: unknown): value is SearchProviderHealth {
  const record = asRecord(value);
  return Boolean(
    record &&
      typeof record.providerId === 'string' &&
      (record.state === 'ready' ||
        record.state === 'degraded' ||
        record.state === 'unavailable' ||
        record.state === 'indexing' ||
        record.state === 'disabled') &&
      (record.reasonCode === undefined || typeof record.reasonCode === 'string') &&
      (record.message === undefined || typeof record.message === 'string')
  );
}

function enumValue<const T extends readonly string[]>(values: T, value: unknown): value is T[number] {
  return typeof value === 'string' && values.includes(value);
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function isSearchDiagnostics(value: unknown): value is SearchDiagnostics {
  const record = asRecord(value);
  return Boolean(
    record &&
      typeof record.coordinator === 'string' &&
      typeof record.legacyHotPathUsed === 'boolean' &&
      (record.notes === undefined ||
        (Array.isArray(record.notes) && record.notes.every((note) => typeof note === 'string')))
  );
}

function isIsoDate(value: string): boolean {
  return !Number.isNaN(Date.parse(value));
}

function isSafeDataUrl(value: string): boolean {
  return /^data:image\/(?:png|webp|jpeg|svg\+xml);base64,[a-z0-9+/=]+$/iu.test(value);
}

function isSafeHttpUrl(value: string): boolean {
  try {
    const url = new URL(value);
    return url.protocol === 'http:' || url.protocol === 'https:';
  } catch (_error) {
    return false;
  }
}

function panelProviderId(providerId: string): SearchPanelResult['providerId'] {
  if (providerId === 'localFolders') {
    return 'local';
  }
  const supported = [
    'apps',
    'settings',
    'local',
    'openWindows',
    'everything',
    'commands',
    'calculator',
    'web',
    'bookmarks'
  ];
  return supported.includes(providerId) ? providerId as SearchPanelResult['providerId'] : undefined;
}

function panelResultPath(result: SearchResult): string | undefined {
  if (result.path) {
    return result.path;
  }
  switch (result.action.kind) {
    case 'openApp':
    case 'openFile':
    case 'openFolder':
      return result.action.path;
    case 'openSetting':
      return result.action.uri;
    case 'runControlPanel':
      return result.action.executable;
    default:
      return undefined;
  }
}

function panelResultUrl(result: SearchResult): string | undefined {
  if (result.action.kind === 'openWebUrl' || result.action.kind === 'openBookmark') {
    return result.action.url;
  }
  return undefined;
}

function panelActionId(action: SearchAction): SearchActivationKind {
  switch (action.kind) {
    case 'runControlPanel':
      return 'runControlPanel';
    case 'copyText':
      return 'copyCalculatorResult';
    default:
      return action.kind;
  }
}

function stablePanelResultKey(result: SearchPanelResult): string {
  return result.recordKey?.trim() || result.id;
}

function mergePanelResult(current: SearchPanelResult, incoming: SearchPanelResult): SearchPanelResult {
  return {
    ...current,
    ...incoming,
    titleHighlightData: incoming.titleHighlightData ?? current.titleHighlightData,
    subtitleHighlightData: incoming.subtitleHighlightData ?? current.subtitleHighlightData
  };
}

function searchPanelEverythingStatusMessage(
  health: SearchProviderHealth,
  resultCount: number
): string {
  switch (health.reasonCode) {
    case 'sdkMissing':
      return 'Everything unavailable — showing local results';
    case 'ipcUnavailable':
      return health.state === 'unavailable'
        ? 'Everything not running — showing local results'
        : 'Everything unavailable — showing local results';
    case 'providerError':
      return 'Everything query failed — showing local results';
    default:
      return resultCount > 0 ? '' : 'No search results matched';
  }
}

function searchPanelEverythingStatusMessageFromText(statusMessage: string): string {
  if (/everything .*query failed/i.test(statusMessage) || /queryfailed/i.test(statusMessage)) {
    return 'Everything query failed — showing local results';
  }
  if (/everything .*not running/i.test(statusMessage) || /process is not running/i.test(statusMessage)) {
    return 'Everything not running — showing local results';
  }
  if (
    /everything .*unavailable/i.test(statusMessage) ||
    /sdk missing/i.test(statusMessage) ||
    /sdk unavailable/i.test(statusMessage) ||
    /ipc unavailable/i.test(statusMessage)
  ) {
    return 'Everything unavailable — showing local results';
  }
  return '';
}

function isOptionalHighlightData(value: unknown): value is number[] | undefined {
  return (
    value === undefined ||
    (Array.isArray(value) &&
      value.length % 2 === 0 &&
      value.every((index) => typeof index === 'number' && Number.isInteger(index) && index >= 0))
  );
}
