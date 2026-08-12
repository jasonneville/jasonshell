export const SHELL_PREFERENCES_STORAGE_KEY = 'jasonshell.uiPreferences';
export const SHELL_PREFERENCES_CHANNEL = 'jasonshell.uiPreferences.changed';
export const SHELL_PREFERENCES_CHANGED_EVENT = 'jasonshell:ui-preferences-changed';

export type BuiltInShellFontId = 'open-sans' | 'google-sans' | 'google-sans-code' | 'segoe-ui' | 'inter' | 'aptos' | 'system' | 'cascadia';
export type ShellFontId = BuiltInShellFontId | string;

export interface ShellFontOption {
  id: ShellFontId;
  label: string;
  stack: string;
}

export interface ShellCustomFont {
  id: string;
  label: string;
  cssUrl: string;
  stack: string;
}

export interface ShellPreferences {
  fontId: ShellFontId;
  customFonts: ShellCustomFont[];
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
    id: 'google-sans',
    label: 'Google Sans',
    stack: "'Google Sans', 'Open Sans', 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif"
  },
  {
    id: 'google-sans-code',
    label: 'Google Sans Code',
    stack: "'Google Sans Code', 'Cascadia Code', 'Cascadia Mono', Consolas, 'Segoe UI Variable Text', 'Segoe UI', monospace"
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
  customFonts: [],
  dateFormat: DEFAULT_DATE_FORMAT,
  use24HourTime: false,
  showSeconds: true,
  compactDensity: false,
  strongFocusRing: false,
  reducedTransparency: false,
  showSearchShortcutHint: true
};

const FONT_IDS = new Set<string>(SHELL_FONT_OPTIONS.map((font) => font.id));

export function shellFontOptions(customFonts: unknown = []): ShellFontOption[] {
  return [
    ...SHELL_FONT_OPTIONS.map((font) => ({ ...font })),
    ...normalizeCustomFonts(customFonts).map((font) => ({ ...font }))
  ];
}

export function shellFontById(value: unknown, customFonts: unknown = []): ShellFontOption {
  const id = typeof value === 'string' ? value : defaultShellPreferences.fontId;
  const customFont = normalizeCustomFonts(customFonts).find((font) => font.id === id);
  if (customFont) {
    return customFont;
  }
  const builtInId = FONT_IDS.has(id) ? (id as BuiltInShellFontId) : defaultShellPreferences.fontId;
  return SHELL_FONT_OPTIONS.find((font) => font.id === builtInId) ?? SHELL_FONT_OPTIONS[0];
}

export interface ParsedGoogleFontLink {
  id: string;
  label: string;
  cssUrl: string;
  stack: string;
}

const GOOGLE_FONTS_SITE_ORIGIN = 'https://fonts.google.com';
const GOOGLE_FONTS_CSS2_ORIGIN = 'https://fonts.googleapis.com';
const CUSTOM_FONT_LINK_DATA_ATTRIBUTE = 'data-jasonshell-custom-google-font';

export function normalizeDateFormat(value: unknown): string {
  if (typeof value !== 'string') {
    return DEFAULT_DATE_FORMAT;
  }
  const clean = value.replace(/[\u0000-\u001f\u007f]/gu, '').trim();
  return clean ? clean.slice(0, 64) : DEFAULT_DATE_FORMAT;
}

export function parseGoogleFontLink(value: unknown): ParsedGoogleFontLink | null {
  if (typeof value !== 'string') {
    return null;
  }
  const raw = value.trim();
  if (!raw || /[\u0000-\u001f\u007f\s]/u.test(raw)) {
    return null;
  }

  let url: URL;
  try {
    url = new URL(raw);
  } catch (_error) {
    return null;
  }

  if (url.protocol !== 'https:' || url.origin !== GOOGLE_FONTS_SITE_ORIGIN || url.username || url.password) {
    return null;
  }

  const familyParams = url.searchParams.getAll('family')
    .map((family) => normalizeGoogleFontFamilyParam(family))
    .filter((family): family is string => Boolean(family));
  const specimenFamily = parseSpecimenFamily(url.pathname);
  const families = familyParams.length > 0 ? familyParams : specimenFamily ? [specimenFamily] : [];
  if (families.length === 0) {
    return null;
  }

  const cssUrl = buildGoogleFontsCss2Url(families);
  const primaryFamilyName = familyNameFromParam(families[0]);
  return {
    id: customFontIdForCssUrl(cssUrl),
    label: primaryFamilyName,
    cssUrl,
    stack: `${quoteFontFamily(primaryFamilyName)}, 'Open Sans', 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif`
  };
}

export function installGoogleFontPreference(value: unknown, current: unknown = storedShellPreferences()): ShellPreferences {
  const parsed = parseGoogleFontLink(value);
  if (!parsed) {
    throw new Error('Paste a valid https://fonts.google.com font link.');
  }
  const preferences = normalizeShellPreferences(current);
  const existingIndex = preferences.customFonts.findIndex((font) => font.cssUrl === parsed.cssUrl || font.id === parsed.id);
  const customFonts = existingIndex >= 0
    ? preferences.customFonts.map((font, index) => index === existingIndex ? parsed : font)
    : [...preferences.customFonts, parsed];
  return setShellPreferences({
    ...preferences,
    customFonts,
    fontId: parsed.id
  });
}

export function normalizeCustomFonts(value: unknown): ShellCustomFont[] {
  if (!Array.isArray(value)) {
    return [];
  }
  const seen = new Set<string>();
  const fonts: ShellCustomFont[] = [];
  for (const entry of value) {
    if (!entry || typeof entry !== 'object') {
      continue;
    }
    const input = entry as Partial<ShellCustomFont>;
    const cssUrl = normalizeGoogleFontsCss2Url(input.cssUrl);
    const label = normalizeFontLabel(input.label);
    if (!cssUrl || !label || seen.has(cssUrl)) {
      continue;
    }
    seen.add(cssUrl);
    const id = typeof input.id === 'string' && input.id.startsWith('google-font:')
      ? input.id.slice(0, 96)
      : customFontIdForCssUrl(cssUrl);
    fonts.push({
      id,
      label,
      cssUrl,
      stack: `${quoteFontFamily(label)}, 'Open Sans', 'Segoe UI Variable Text', 'Segoe UI', system-ui, sans-serif`
    });
    if (fonts.length >= 24) {
      break;
    }
  }
  return fonts;
}

export function injectCustomFontStylesheets(customFonts: unknown, documentLike: Document | null = browserDocument()): void {
  if (!documentLike || typeof documentLike.querySelectorAll !== 'function' || !documentLike.head) {
    return;
  }
  const fonts = normalizeCustomFonts(customFonts);
  const wanted = new Set(fonts.map((font) => font.cssUrl));
  const existing = Array.from(documentLike.querySelectorAll<HTMLLinkElement>(`link[${CUSTOM_FONT_LINK_DATA_ATTRIBUTE}]`));
  for (const link of existing) {
    const cssUrl = link.getAttribute(CUSTOM_FONT_LINK_DATA_ATTRIBUTE) ?? '';
    if (!wanted.has(cssUrl)) {
      link.remove();
    } else {
      wanted.delete(cssUrl);
    }
  }
  for (const cssUrl of wanted) {
    const link = documentLike.createElement('link');
    link.rel = 'stylesheet';
    link.href = cssUrl;
    link.setAttribute(CUSTOM_FONT_LINK_DATA_ATTRIBUTE, cssUrl);
    documentLike.head.appendChild(link);
  }
}

export function normalizeShellPreferences(value: unknown): ShellPreferences {
  const input = value && typeof value === 'object' ? value as Partial<ShellPreferences> : {};
  const customFonts = normalizeCustomFonts(input.customFonts);
  return {
    fontId: shellFontById(input.fontId, customFonts).id,
    customFonts,
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
    documentElement.style.setProperty('--js-font-sans', shellFontById(preferences.fontId, preferences.customFonts).stack);
    documentElement.dataset.shellDensity = preferences.compactDensity ? 'compact' : undefined;
    documentElement.dataset.shellFocus = preferences.strongFocusRing ? 'strong' : undefined;
    documentElement.dataset.shellTransparency = preferences.reducedTransparency ? 'reduced' : undefined;
  }

  injectCustomFontStylesheets(preferences.customFonts);

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

function browserDocument(): Document | null {
  return typeof document === 'undefined' ? null : document;
}

function parseSpecimenFamily(pathname: string): string | null {
  const match = /^\/specimen\/([^/?#]+)/u.exec(pathname);
  if (!match) {
    return null;
  }
  return normalizeGoogleFontFamilyParam(decodeURIComponent(match[1]));
}

function normalizeGoogleFontFamilyParam(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null;
  }
  const clean = value.replace(/[\u0000-\u001f\u007f]/gu, '').trim().replace(/\s+/gu, '+');
  if (!clean || clean.length > 160 || !/^[A-Za-z0-9+ :;,._@-]+$/u.test(clean)) {
    return null;
  }
  return clean;
}

function buildGoogleFontsCss2Url(families: string[]): string {
  const params = new URLSearchParams();
  for (const family of families) {
    params.append('family', family);
  }
  params.set('display', 'swap');
  return `${GOOGLE_FONTS_CSS2_ORIGIN}/css2?${params.toString()}`;
}

function normalizeGoogleFontsCss2Url(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null;
  }
  let url: URL;
  try {
    url = new URL(value);
  } catch (_error) {
    return null;
  }
  if (url.protocol !== 'https:' || url.origin !== GOOGLE_FONTS_CSS2_ORIGIN || url.pathname !== '/css2') {
    return null;
  }
  const families = url.searchParams.getAll('family')
    .map((family) => normalizeGoogleFontFamilyParam(family))
    .filter((family): family is string => Boolean(family));
  if (families.length === 0) {
    return null;
  }
  return buildGoogleFontsCss2Url(families);
}

function familyNameFromParam(value: string): string {
  return value.split(':', 1)[0].replace(/\+/gu, ' ').trim();
}

function normalizeFontLabel(value: unknown): string {
  if (typeof value !== 'string') {
    return '';
  }
  return value.replace(/[\u0000-\u001f\u007f]/gu, '').replace(/\s+/gu, ' ').trim().slice(0, 64);
}

function quoteFontFamily(value: string): string {
  return `'${value.replace(/['\\]/gu, '')}'`;
}

function customFontIdForCssUrl(cssUrl: string): string {
  let hash = 2166136261;
  for (let index = 0; index < cssUrl.length; index += 1) {
    hash ^= cssUrl.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return `google-font:${(hash >>> 0).toString(36)}`;
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
