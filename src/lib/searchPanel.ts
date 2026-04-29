import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';
import { isEverythingSetupConsentAllowed } from './searchSettings.js';
import type { EverythingSetupConsentRequest, EverythingSetupResult } from './searchSettings';

export const SEARCH_PANEL_LABEL = 'search-panel';
export const SEARCH_PANEL_UPDATE_EVENT = 'search-panel:update';
export const SEARCH_PANEL_ACTIVATE_EVENT = 'search-panel:activate';
export const SEARCH_PANEL_SELECT_EVENT = 'search-panel:select';
export const SEARCH_PANEL_PIN_FOLDER_EVENT = 'search-panel:pin-folder';
export const SEARCH_PANEL_INTERACTION_EVENT = 'search-panel:interaction';
export const SEARCH_PANEL_CLOSED_EVENT = 'search-panel:closed';

export const SEARCH_RESULT_KINDS = [
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
export type SearchPanelResultKind = (typeof SEARCH_RESULT_KINDS)[number];

export const SEARCH_PROVIDER_IDS = [
  'apps',
  'openWindows',
  'everything',
  'windowsSearch',
  'warmedCache',
  'commands',
  'calculator',
  'web',
  'bookmarks'
] as const;
export type SearchProviderId = (typeof SEARCH_PROVIDER_IDS)[number];

export const SEARCH_PROVIDER_HEALTH_STATES = [
  'ready',
  'degraded',
  'unavailable',
  'indexing',
  'adminRequired',
  'disabled'
] as const;
export type ProviderHealthState = (typeof SEARCH_PROVIDER_HEALTH_STATES)[number];

export const SEARCH_PROVIDER_HEALTH_REASON_CODES = [
  'sdkMissing',
  'ipcUnavailable',
  'serviceUnavailable',
  'notInstalled',
  'notRunning',
  'userDisabled',
  'checksumBlocked',
  'licenseBlocked',
  'fallbackActive'
] as const;
export type ProviderHealthReasonCode = (typeof SEARCH_PROVIDER_HEALTH_REASON_CODES)[number];

export const SEARCH_ACTIVATION_KINDS = [
  'openApp',
  'focusWindow',
  'openFile',
  'openFolder',
  'runCommand',
  'openSetting',
  'copyCalculatorResult',
  'openWebUrl',
  'openBookmark'
] as const;
export type SearchActivationKind = (typeof SEARCH_ACTIVATION_KINDS)[number];

export const CENTERED_SEARCH_CLOSE_REASONS = [
  'escape',
  'outsideClick',
  'focusLoss',
  'activation',
  'settingsChanged'
] as const;
export type CenteredSearchCloseReason = (typeof CENTERED_SEARCH_CLOSE_REASONS)[number];

export type ProviderHealthContract = {
  providerId: SearchProviderId;
  state: ProviderHealthState;
  reasonCode?: ProviderHealthReasonCode;
  message: string;
  canRequestSetup: boolean;
  checkedAtIso: string;
};

export type SearchActivationRequest = {
  resultId: string;
  providerId: SearchProviderId;
  actionId: string;
  kind: SearchActivationKind;
  recordKey: string;
  payload: Record<string, string | number | boolean>;
  requiresConfirmation: boolean;
};

export type SearchActivationResult = {
  resultId: string;
  handled: boolean;
  message?: string;
};

export type CenteredSearchSurfaceContract = {
  label: 'search-panel' | 'centered-search';
  mode: 'centeredHotkey';
  requestId: string;
  query: string;
  sequence: number;
  anchor: 'screenCenter';
  closeReasons: CenteredSearchCloseReason[];
  accessibility: {
    role: 'combobox';
    listboxId: string;
    activeOptionId?: string;
  };
};

export type SearchPanelResult = {
  id: string;
  providerId?: string;
  kind: SearchPanelResultKind;
  title: string;
  subtitle: string;
  terms: string;
  priority: number;
  score?: number;
  iconDataUrl?: string;
  path?: string;
  url?: string;
  actionId?: string;
  copyText?: string;
  autoCompleteText?: string;
  titleHighlightData?: number[];
  subtitleHighlightData?: number[];
  recordKey?: string;
  runCount?: number;
  topMost?: boolean;
  providerHealth?: ProviderHealthState;
};

export type SearchPanelPayload = {
  query: string;
  results: SearchPanelResult[];
  selectedIndex: number;
  statusMessage: string;
  sequence?: number;
};

export type ShowSearchPanelRequest = {
  anchorLeft: number;
  anchorWidth: number;
};

export function showSearchPanel(request: ShowSearchPanelRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showSearchPanel, { request });
}

export function showCenteredSearchPanel(): Promise<void> {
  return invoke(IPC_COMMANDS.showCenteredSearchPanel);
}

export function hideSearchPanel(): Promise<void> {
  return invoke(IPC_COMMANDS.hideSearchPanel);
}

export function publishSearchPanel(payload: SearchPanelPayload): Promise<void> {
  return invoke(IPC_COMMANDS.publishSearchPanel, { payload });
}

export function getSearchPanelPayload(): Promise<SearchPanelPayload | null> {
  return invoke(IPC_COMMANDS.getSearchPanelPayload);
}

export function openShellPath(path: string): Promise<void> {
  return invoke(IPC_COMMANDS.openShellPath, { path });
}

export function getSearchProviderHealth(): Promise<ProviderHealthContract[]> {
  return invoke(IPC_COMMANDS.getSearchProviderHealth);
}

export function requestEverythingSetup(
  request: EverythingSetupConsentRequest
): Promise<EverythingSetupResult> {
  return invoke(IPC_COMMANDS.requestEverythingSetup, { request });
}

export function searchResultActionId(result: Pick<SearchPanelResult, 'actionId' | 'kind'>): SearchActivationKind {
  if (result.actionId && SEARCH_ACTIVATION_KINDS.includes(result.actionId as SearchActivationKind)) {
    return result.actionId as SearchActivationKind;
  }
  switch (result.kind) {
    case 'app':
      return 'openApp';
    case 'window':
      return 'focusWindow';
    case 'file':
      return 'openFile';
    case 'folder':
      return 'openFolder';
    case 'setting':
      return 'openSetting';
    case 'calculator':
      return 'copyCalculatorResult';
    case 'web':
      return 'openWebUrl';
    case 'bookmark':
      return 'openBookmark';
    case 'command':
      return 'runCommand';
  }
}

export function isProviderHealthContract(value: unknown): value is ProviderHealthContract {
  const record = asRecord(value);
  return Boolean(
    record &&
      enumValue(SEARCH_PROVIDER_IDS, record.providerId) &&
      enumValue(SEARCH_PROVIDER_HEALTH_STATES, record.state) &&
      (record.reasonCode === undefined ||
        enumValue(SEARCH_PROVIDER_HEALTH_REASON_CODES, record.reasonCode)) &&
      typeof record.message === 'string' &&
      typeof record.canRequestSetup === 'boolean' &&
      typeof record.checkedAtIso === 'string' &&
      !Number.isNaN(Date.parse(record.checkedAtIso))
  );
}

export function isSearchActivationRequest(value: unknown): value is SearchActivationRequest {
  const record = asRecord(value);
  return Boolean(
    record &&
      typeof record.resultId === 'string' &&
      enumValue(SEARCH_PROVIDER_IDS, record.providerId) &&
      typeof record.actionId === 'string' &&
      enumValue(SEARCH_ACTIVATION_KINDS, record.kind) &&
      record.actionId === record.kind &&
      typeof record.recordKey === 'string' &&
      isActivationPayload(record.payload) &&
      typeof record.requiresConfirmation === 'boolean'
  );
}

export function isCenteredSearchSurfaceContract(
  value: unknown
): value is CenteredSearchSurfaceContract {
  const record = asRecord(value);
  const accessibility = asRecord(record?.accessibility);
  return Boolean(
    record &&
      (record.label === 'search-panel' || record.label === 'centered-search') &&
      record.mode === 'centeredHotkey' &&
      typeof record.requestId === 'string' &&
      typeof record.query === 'string' &&
      Number.isInteger(record.sequence) &&
      record.anchor === 'screenCenter' &&
      Array.isArray(record.closeReasons) &&
      record.closeReasons.every((reason) => enumValue(CENTERED_SEARCH_CLOSE_REASONS, reason)) &&
      accessibility &&
      accessibility.role === 'combobox' &&
      typeof accessibility.listboxId === 'string' &&
      (accessibility.activeOptionId === undefined ||
        typeof accessibility.activeOptionId === 'string')
  );
}

export { isEverythingSetupConsentAllowed };

function enumValue<const T extends readonly string[]>(values: T, value: unknown): value is T[number] {
  return typeof value === 'string' && values.includes(value);
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function isActivationPayload(value: unknown): value is Record<string, string | number | boolean> {
  const record = asRecord(value);
  return Boolean(
    record &&
      Object.values(record).every(
        (item) => typeof item === 'string' || typeof item === 'number' || typeof item === 'boolean'
      )
  );
}
