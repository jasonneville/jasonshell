import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';
import type { SearchPanelResult, SearchPanelResultKind } from './searchPanel';

export const SEARCH_INDEX_REFRESHED_EVENT = 'search-index:refreshed';

type SystemSearchResult = {
  id: string;
  providerId?: string;
  kind: SearchPanelResultKind;
  title: string;
  subtitle: string;
  terms: string;
  priority: number;
  path: string;
  recordKey?: string;
  runCount?: number;
  topMost?: boolean;
};

export function searchSystem(query: string): Promise<SearchPanelResult[]> {
  return invoke<SystemSearchResult[]>(IPC_COMMANDS.searchSystem, { query }).then((results) =>
    results.map((result) => ({
      id: result.id,
      providerId: result.providerId,
      kind: result.kind,
      path: result.path,
      priority: result.priority,
      recordKey: result.recordKey,
      runCount: result.runCount,
      subtitle: result.subtitle,
      terms: result.terms,
      title: result.title,
      topMost: result.topMost
    }))
  );
}

export function isSystemPathResult(result: SearchPanelResult): boolean {
  return result.id.startsWith('system:') && Boolean(result.path);
}
