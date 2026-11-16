import { defaultShellThemeId, normalizeShellThemeId, SHELL_THEME_STORAGE_KEY, type ThemeStorageLike } from './themes.js';
import { validateCustomTheme, type CustomTheme } from './themePalette.js';

export const SAVED_THEMES_STORAGE_KEY = 'jasonshell.themes.v1';
export const MAX_CUSTOM_THEMES = 100;
export const MAX_THEME_STORAGE_BYTES = 512 * 1024;
export interface SavedThemeState {
  version: 1;
  revision: string;
  selectedId: string;
  customThemes: CustomTheme[];
}
export interface ThemeLoadResult {
  state: SavedThemeState;
  error: string | null;
  readOnly: boolean;
  initial: boolean;
}
export function emptyThemeState(selectedId: string = defaultShellThemeId): SavedThemeState {
  return { version: 1, revision: 'initial', selectedId, customThemes: [] };
}
export function themeStorage(): ThemeStorageLike | null {
  try { return typeof window === 'undefined' ? null : window.localStorage; } catch { return null; }
}
export function newThemeRevision(): string { return crypto.randomUUID(); }

export function validateSavedThemeState(input: unknown): SavedThemeState {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new Error('Invalid saved themes');
  const value = input as Record<string, unknown>;
  if (value.version !== 1) throw new Error('Unsupported theme schema');
  if (typeof value.revision !== 'string' || !/^[\w:.-]{1,160}$/.test(value.revision)) throw new Error('Invalid saved revision');
  if (!Array.isArray(value.customThemes) || value.customThemes.length > MAX_CUSTOM_THEMES) throw new Error('At most 100 custom themes are supported');
  const customThemes = value.customThemes.map(validateCustomTheme);
  if (new Set(customThemes.map((theme) => theme.id)).size !== customThemes.length) throw new Error('Duplicate custom theme IDs');
  const selectedId = customThemes.some((theme) => theme.id === value.selectedId)
    ? value.selectedId as string : normalizeShellThemeId(value.selectedId);
  return { version: 1, revision: value.revision, selectedId, customThemes };
}

/** Loading never writes, including migration. Unsupported future data is read-only. */
export function loadSavedThemeState(storage: ThemeStorageLike | null = themeStorage()): ThemeLoadResult {
  try {
    const raw = storage?.getItem(SAVED_THEMES_STORAGE_KEY);
    if (raw == null) return { state: emptyThemeState(normalizeShellThemeId(storage?.getItem(SHELL_THEME_STORAGE_KEY))), error: null, readOnly: !storage, initial: true };
    if (new TextEncoder().encode(raw).byteLength > MAX_THEME_STORAGE_BYTES) throw new Error('Saved themes exceed 512KiB');
    const parsed: unknown = JSON.parse(raw);
    if (parsed && typeof parsed === 'object' && 'version' in parsed && typeof parsed.version === 'number' && parsed.version > 1) {
      return { state: emptyThemeState(), error: 'Themes were saved by a newer version; existing data is protected', readOnly: true, initial: false };
    }
    return { state: validateSavedThemeState(parsed), error: null, readOnly: false, initial: false };
  } catch (error) {
    return { state: emptyThemeState(), error: error instanceof Error ? error.message : 'Cannot read saved themes', readOnly: true, initial: false };
  }
}

/** One canonical setItem publishes collection and selection together. */
export function writeSavedThemeState(state: SavedThemeState, expectedRevision: string, storage: ThemeStorageLike | null = themeStorage()): SavedThemeState {
  if (!storage) throw new Error('Theme storage is unavailable');
  const current = loadSavedThemeState(storage);
  if (current.readOnly) throw new Error(current.error ?? 'Theme storage is unavailable');
  if (current.state.revision !== expectedRevision) throw new Error('Saved themes changed in another window; reload and retry');
  const validated = validateSavedThemeState({ ...state, revision: newThemeRevision() });
  const json = JSON.stringify(validated);
  if (new TextEncoder().encode(json).byteLength > MAX_THEME_STORAGE_BYTES) throw new Error('Saved themes exceed 512KiB');
  try { storage.setItem(SAVED_THEMES_STORAGE_KEY, json); } catch { throw new Error('Could not save theme; storage is unavailable or full'); }
  // This is only a compatibility mirror. Canonical publication already succeeded.
  try { storage.setItem(SHELL_THEME_STORAGE_KEY, validated.selectedId); } catch { /* canonical data remains authoritative */ }
  return validated;
}

export function persistCustomTheme(theme: CustomTheme, expectedThemeRevision: number | null, storage: ThemeStorageLike | null = themeStorage()): SavedThemeState {
  const loaded = loadSavedThemeState(storage);
  if (loaded.readOnly) throw new Error(loaded.error ?? 'Theme storage unavailable');
  const current = loaded.state.customThemes.find((item) => item.id === theme.id);
  if ((current?.revision ?? null) !== expectedThemeRevision) throw new Error('This theme changed or was deleted in another window');
  if (current && (theme.baseBuiltinId !== current.baseBuiltinId || JSON.stringify(validateCustomTheme(theme).startingPalette) !== JSON.stringify(current.startingPalette))) throw new Error('Editing cannot replace the original theme baseline');
  const saved = validateCustomTheme({ ...theme, revision: (current?.revision ?? 0) + 1 });
  const customThemes = [...loaded.state.customThemes.filter((item) => item.id !== saved.id), saved];
  return writeSavedThemeState({ ...loaded.state, selectedId: saved.id, customThemes }, loaded.state.revision, storage);
}

export function persistDeleteCustomTheme(id: string, expectedRevision: number, storage: ThemeStorageLike | null = themeStorage()): SavedThemeState {
  const loaded = loadSavedThemeState(storage);
  const current = loaded.state.customThemes.find((item) => item.id === id);
  if (!current || current.revision !== expectedRevision) throw new Error('This theme changed or was deleted in another window');
  return writeSavedThemeState({ ...loaded.state, selectedId: loaded.state.selectedId === id ? defaultShellThemeId : loaded.state.selectedId,
    customThemes: loaded.state.customThemes.filter((item) => item.id !== id) }, loaded.state.revision, storage);
}
