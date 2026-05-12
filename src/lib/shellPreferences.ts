export const SHELL_PREFERENCES_STORAGE_KEY = 'jasonshell.uiPreferences';
export const SHELL_PREFERENCES_CHANNEL = 'jasonshell.uiPreferences.changed';
export const SHELL_PREFERENCES_CHANGED_EVENT = 'jasonshell:ui-preferences-changed';

export type ShellFontId = 'open-sans' | 'segoe-ui' | 'inter' | 'aptos' | 'system' | 'cascadia';

export interface ShellFontOption {
  id: ShellFontId;
  label: string;
  stack: string;
}

export interface ShellPreferences {
  fontId: ShellFontId;
  dateFormat: string;
  use24HourTime: boolean;
  showSeconds: boolean;
  compactDensity: boolean;
  strongFocusRing: boolean;
  reducedTransparency: boolean;
  showSearchShortcutHint: boolean;
}

export interface PreferenceStorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

export interface PreferenceDocumentElementLike {
  dataset: Record<string, string | undefined>;
  style: {
    setProperty(name: string, value: string): void;
  };
}

export const SHELL_FONT_OPTIONS: readonly ShellFontOption[] = [
  {
    id: 'open-sans',
    label: 'Open Sans',
    stack: "'Open Sans', 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif"
  },
  {
    id: 'segoe-ui',
    label: 'Segoe UI',
    stack: "'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif"
  },
  {
    id: 'inter',
    label: 'Inter',
    stack: "'Inter', 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif"
  },
  {
    id: 'aptos',
    label: 'Aptos',
    stack: "'Aptos', 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif"
  },
  {
    id: 'system',
    label: 'System UI',
    stack: "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
  },
  {
    id: 'cascadia',
    label: 'Cascadia UI',
    stack: "'Cascadia Code', 'Cascadia Mono', 'Segoe UI Variable Text', 'Segoe UI', monospace"
  }
] as const;

export const DEFAULT_DATE_FORMAT = 'EEE, MMM d';

export const defaultShellPreferences: ShellPreferences = {
  fontId: 'open-sans',
  dateFormat: DEFAULT_DATE_FORMAT,
  use24HourTime: false,
  showSeconds: true,
  compactDensity: false,
  strongFocusRing: false,
  reducedTransparency: false,
  showSearchShortcutHint: true
};

const FONT_IDS = new Set<string>(SHELL_FONT_OPTIONS.map((font) => font.id));

export function shellFontOptions(): ShellFontOption[] {
  return SHELL_FONT_OPTIONS.map((font) => ({ ...font }));
}

export function shellFontById(value: unknown): ShellFontOption {
  const id = typeof value === 'string' && FONT_IDS.has(value)
    ? (value as ShellFontId)
    : defaultShellPreferences.fontId;
  return SHELL_FONT_OPTIONS.find((font) => font.id === id) ?? SHELL_FONT_OPTIONS[0];
}

export function normalizeDateFormat(value: unknown): string {
  if (typeof value !== 'string') {
    return DEFAULT_DATE_FORMAT;
  }
  const clean = value.replace(/[\u0000-\u001f\u007f]/gu, '').trim();
  return clean ? clean.slice(0, 64) : DEFAULT_DATE_FORMAT;
}

export function normalizeShellPreferences(value: unknown): ShellPreferences {
  const input = value && typeof value === 'object' ? value as Partial<ShellPreferences> : {};
  return {
    fontId: shellFontById(input.fontId).id,
    dateFormat: normalizeDateFormat(input.dateFormat),
    use24HourTime: Boolean(input.use24HourTime),
    showSeconds: input.showSeconds === undefined ? defaultShellPreferences.showSeconds : Boolean(input.showSeconds),
    compactDensity: Boolean(input.compactDensity),
    strongFocusRing: Boolean(input.strongFocusRing),
    reducedTransparency: Boolean(input.reducedTransparency),
    showSearchShortcutHint: input.showSearchShortcutHint === undefined
      ? defaultShellPreferences.showSearchShortcutHint
      : Boolean(input.showSearchShortcutHint)
  };
}

export function storedShellPreferences(
  storage: PreferenceStorageLike | null = browserStorage()
): ShellPreferences {
  try {
    const raw = storage?.getItem(SHELL_PREFERENCES_STORAGE_KEY);
    return normalizeShellPreferences(raw ? JSON.parse(raw) : defaultShellPreferences);
  } catch (_error) {
    return { ...defaultShellPreferences };
  }
}

export function getInitialShellPreferences(): ShellPreferences {
  return storedShellPreferences();
}

export function applyShellPreferences(
  value: unknown,
  options: {
    documentElement?: PreferenceDocumentElementLike | null;
    storage?: PreferenceStorageLike | null;
    dispatch?: boolean;
  } = {}
): ShellPreferences {
  const preferences = normalizeShellPreferences(value);
  const documentElement = options.documentElement ?? browserDocumentElement();
  if (documentElement) {
    documentElement.style.setProperty('--js-font-sans', shellFontById(preferences.fontId).stack);
    documentElement.dataset.shellDensity = preferences.compactDensity ? 'compact' : undefined;
    documentElement.dataset.shellFocus = preferences.strongFocusRing ? 'strong' : undefined;
    documentElement.dataset.shellTransparency = preferences.reducedTransparency ? 'reduced' : undefined;
  }

  if (options.storage !== null) {
    try {
      (options.storage ?? browserStorage())?.setItem(
        SHELL_PREFERENCES_STORAGE_KEY,
        JSON.stringify(preferences)
      );
    } catch (_error) {
      // UI preferences are cosmetic; failed persistence must not break shell surfaces.
    }
  }

  if (options.dispatch !== false) {
    dispatchShellPreferencesChanged(preferences);
  }

  return preferences;
}

export function setShellPreferences(value: unknown): ShellPreferences {
  const preferences = applyShellPreferences(value);
  broadcastPreferences(preferences);
  return preferences;
}

export function patchShellPreferences(patch: Partial<ShellPreferences>): ShellPreferences {
  return setShellPreferences({
    ...storedShellPreferences(),
    ...patch
  });
}

export function installShellPreferencesSync(onChange?: (preferences: ShellPreferences) => void): () => void {
  const applyRemotePreferences = (value: unknown) => {
    const preferences = applyShellPreferences(value, { storage: null });
    onChange?.(preferences);
  };
  const preferences = applyShellPreferences(storedShellPreferences(), { storage: null });
  onChange?.(preferences);
  const channel = browserBroadcastChannel();

  const handleStorage = (event: StorageEvent) => {
    if (event.key === SHELL_PREFERENCES_STORAGE_KEY) {
      try {
        applyRemotePreferences(event.newValue ? JSON.parse(event.newValue) : null);
      } catch (_error) {
        applyRemotePreferences(null);
      }
    }
  };

  if (channel) {
    channel.onmessage = (event) => applyRemotePreferences(event.data);
  }

  if (typeof window !== 'undefined') {
    window.addEventListener('storage', handleStorage);
  }

  return () => {
    channel?.close();
    if (typeof window !== 'undefined') {
      window.removeEventListener('storage', handleStorage);
    }
  };
}

export function addShellPreferencesChangeListener(
  listener: (preferences: ShellPreferences) => void
): () => void {
  if (typeof window === 'undefined') {
    return () => undefined;
  }
  const handleChange = (event: Event) => {
    const detail = typeof event === 'object' && event && 'detail' in event
      ? (event as CustomEvent<ShellPreferences>).detail
      : null;
    listener(normalizeShellPreferences(detail));
  };
  window.addEventListener(SHELL_PREFERENCES_CHANGED_EVENT, handleChange);
  return () => window.removeEventListener(SHELL_PREFERENCES_CHANGED_EVENT, handleChange);
}

export function formatShellTime(date: Date, preferences: ShellPreferences): string {
  return new Intl.DateTimeFormat(undefined, {
    hour: 'numeric',
    hour12: preferences.use24HourTime ? false : undefined,
    minute: '2-digit',
    second: preferences.showSeconds ? '2-digit' : undefined
  }).format(date);
}

export function formatShellDate(date: Date, format: string): string {
  const normalizedFormat = normalizeDateFormat(format);
  const monthNames = monthNameParts(date);
  const weekdayNames = weekdayNameParts(date);
  const tokens: Record<string, string> = {
    yyyy: `${date.getFullYear()}`,
    yy: `${date.getFullYear()}`.slice(-2),
    MMMM: monthNames.long,
    MMM: monthNames.short,
    MM: pad2(date.getMonth() + 1),
    M: `${date.getMonth() + 1}`,
    dd: pad2(date.getDate()),
    d: `${date.getDate()}`,
    EEEE: weekdayNames.long,
    EEE: weekdayNames.short
  };
  return normalizedFormat.replace(/yyyy|yy|MMMM|MMM|MM|M|dd|d|EEEE|EEE/gu, (token) => tokens[token] ?? token);
}

function pad2(value: number): string {
  return `${value}`.padStart(2, '0');
}

function monthNameParts(date: Date): { short: string; long: string } {
  return {
    short: new Intl.DateTimeFormat(undefined, { month: 'short' }).format(date),
    long: new Intl.DateTimeFormat(undefined, { month: 'long' }).format(date)
  };
}

function weekdayNameParts(date: Date): { short: string; long: string } {
  return {
    short: new Intl.DateTimeFormat(undefined, { weekday: 'short' }).format(date),
    long: new Intl.DateTimeFormat(undefined, { weekday: 'long' }).format(date)
  };
}

function dispatchShellPreferencesChanged(preferences: ShellPreferences): void {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(shellPreferencesChangedEvent(preferences));
  }
}

function shellPreferencesChangedEvent(preferences: ShellPreferences): Event {
  if (typeof CustomEvent !== 'undefined') {
    return new CustomEvent(SHELL_PREFERENCES_CHANGED_EVENT, { detail: preferences });
  }
  const event = new Event(SHELL_PREFERENCES_CHANGED_EVENT);
  Object.defineProperty(event, 'detail', {
    configurable: true,
    value: preferences
  });
  return event;
}

function broadcastPreferences(preferences: ShellPreferences): void {
  const channel = browserBroadcastChannel();
  try {
    channel?.postMessage(preferences);
  } finally {
    channel?.close();
  }
}

function browserStorage(): PreferenceStorageLike | null {
  return typeof window === 'undefined' ? null : window.localStorage;
}

function browserDocumentElement(): PreferenceDocumentElementLike | null {
  return typeof document === 'undefined' ? null : document.documentElement;
}

function browserBroadcastChannel(): BroadcastChannel | null {
  if (typeof BroadcastChannel === 'undefined') {
    return null;
  }
  try {
    return new BroadcastChannel(SHELL_PREFERENCES_CHANNEL);
  } catch (_error) {
    return null;
  }
}
