import type { SearchPanelResult } from './searchPanel';

type SelectableResult = Pick<SearchPanelResult, 'id' | 'kind' | 'providerId' | 'matchReason'>;

const WEB_SEARCH_ID = 'web:search';
const WEB_OPEN_URL_ID = 'web:open-url';
const QUICK_COMMAND_ID_PREFIX = 'quick:';

function hasWebRowId(result: Pick<SearchPanelResult, 'id' | 'providerId'> | undefined, id: string): boolean {
  return result?.providerId === 'web' && (result.id === id || result.id.startsWith(`${id}:`));
}

/** The web fallback is always supplied last and must stay last. */
export function isWebSearchFallback(result: Pick<SearchPanelResult, 'id' | 'providerId'> | undefined): boolean {
  return hasWebRowId(result, WEB_SEARCH_ID);
}

/** Saved command id of a Quick Commands row; built-in command rows return null. */
export function quickCommandIdForResult(
  result: Pick<SearchPanelResult, 'id' | 'kind' | 'providerId'>
): string | null {
  return result.kind === 'command' &&
    result.providerId === 'commands' &&
    result.id.startsWith(QUICK_COMMAND_ID_PREFIX)
    ? result.id.slice(QUICK_COMMAND_ID_PREFIX.length) || null
    : null;
}

/**
 * Whether a plain Enter may activate this row without the user having chosen it.
 * `interim` means other providers are still pending.
 */
export function isImplicitlySelectable(result: SelectableResult, interim = false): boolean {
  if (quickCommandIdForResult(result) !== null || (interim && result.kind === 'bookmark')) {
    return result.matchReason === 'exact' || result.matchReason === 'prefix';
  }
  if (hasWebRowId(result, WEB_OPEN_URL_ID)) {
    return result.matchReason === 'url';
  }
  return !isWebSearchFallback(result);
}

/**
 * Index selected without any user choice, or -1 for none. Web rows qualify as a group only when
 * nothing else matched; a weak quick command never does.
 */
export function implicitSearchSelectionIndex(results: readonly SelectableResult[], interim = false): number {
  const strong = results.findIndex((result) => isImplicitlySelectable(result, interim));
  if (strong >= 0) {
    return strong;
  }
  return results.length > 0 && results.every((result) => result.providerId === 'web') ? 0 : -1;
}
