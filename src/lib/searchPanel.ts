import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export const SEARCH_PANEL_LABEL = 'search-panel';
export const SEARCH_PANEL_UPDATE_EVENT = 'search-panel:update';
export const SEARCH_PANEL_ACTIVATE_EVENT = 'search-panel:activate';
export const SEARCH_PANEL_SELECT_EVENT = 'search-panel:select';
export const SEARCH_PANEL_PIN_FOLDER_EVENT = 'search-panel:pin-folder';
export const SEARCH_PANEL_INTERACTION_EVENT = 'search-panel:interaction';
export const SEARCH_PANEL_CLOSED_EVENT = 'search-panel:closed';

export type SearchPanelResultKind = 'app' | 'window' | 'folder' | 'file' | 'command';

export type SearchPanelResult = {
  id: string;
  kind: SearchPanelResultKind;
  title: string;
  subtitle: string;
  terms: string;
  priority: number;
  iconDataUrl?: string;
  path?: string;
};

export type SearchPanelPayload = {
  query: string;
  results: SearchPanelResult[];
  selectedIndex: number;
  statusMessage: string;
};

export type ShowSearchPanelRequest = {
  anchorLeft: number;
  anchorWidth: number;
};

export function showSearchPanel(request: ShowSearchPanelRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showSearchPanel, { request });
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
