export const SHELL_THEME_STORAGE_KEY = 'jasonshell.theme';
export const SHELL_THEME_CHANNEL = 'jasonshell.theme.changed';

export type ShellThemeMode = 'dark' | 'light';

export interface ShellTheme {
  id: string;
  label: string;
  family: string;
  mode: ShellThemeMode;
  accent: string;
}

export interface ThemeStorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

export interface ThemeDocumentElementLike {
  dataset: Record<string, string | undefined>;
  style?: {
    colorScheme?: string;
  };
}

export const SHELL_THEMES = [
  { id: 'base-dark', label: 'Base Dark', family: 'JasonShell', mode: 'dark', accent: '#8aa7ff' },
  { id: 'base-light', label: 'Base Light', family: 'JasonShell', mode: 'light', accent: '#2563eb' },
  { id: 'monokai', label: 'Monokai', family: 'Editor', mode: 'dark', accent: '#a6e22e' },
  { id: 'atom-one-dark', label: 'Atom One Dark', family: 'Editor', mode: 'dark', accent: '#61afef' },
  { id: 'atom-one-light', label: 'Atom One Light', family: 'Editor', mode: 'light', accent: '#4078f2' },
  { id: 'nord', label: 'Nord', family: 'Editor', mode: 'dark', accent: '#88c0d0' },
  { id: 'dracula', label: 'Dracula', family: 'Editor', mode: 'dark', accent: '#bd93f9' },
  { id: 'solarized-dark', label: 'Solarized Dark', family: 'Editor', mode: 'dark', accent: '#268bd2' },
  { id: 'solarized-light', label: 'Solarized Light', family: 'Editor', mode: 'light', accent: '#268bd2' },
  { id: 'github-dark', label: 'GitHub Dark', family: 'Application', mode: 'dark', accent: '#2f81f7' },
  { id: 'github-light', label: 'GitHub Light', family: 'Application', mode: 'light', accent: '#0969da' },
  { id: 'gruvbox-dark', label: 'Gruvbox Dark', family: 'Editor', mode: 'dark', accent: '#fabd2f' },
  { id: 'gruvbox-light', label: 'Gruvbox Light', family: 'Editor', mode: 'light', accent: '#b57614' },
  { id: 'tokyo-night', label: 'Tokyo Night', family: 'Editor', mode: 'dark', accent: '#7aa2f7' },
  { id: 'catppuccin-mocha', label: 'Catppuccin Mocha', family: 'Editor', mode: 'dark', accent: '#cba6f7' },
  { id: 'ayu-dark', label: 'Ayu Dark', family: 'Editor', mode: 'dark', accent: '#ffcc66' },
  { id: 'night-owl', label: 'Night Owl', family: 'Editor', mode: 'dark', accent: '#82aaff' },
  { id: 'palenight', label: 'Palenight', family: 'Editor', mode: 'dark', accent: '#c792ea' },
  { id: 'one-dark-pro', label: 'One Dark Pro', family: 'Editor', mode: 'dark', accent: '#61afef' },
  { id: 'material-ocean', label: 'Material Ocean', family: 'Editor', mode: 'dark', accent: '#80cbc4' },
  { id: 'everforest-dark', label: 'Everforest Dark', family: 'Editor', mode: 'dark', accent: '#a7c080' },
  { id: 'rose-pine-moon', label: 'Rose Pine Moon', family: 'Editor', mode: 'dark', accent: '#ebbcba' },
  { id: 'oceanic-next', label: 'Oceanic Next', family: 'Editor', mode: 'dark', accent: '#74b9ff' },
  { id: 'shades-of-purple', label: 'Shades of Purple', family: 'Editor', mode: 'dark', accent: '#a599e9' },
  { id: 'kanagawa-wave', label: 'Kanagawa Wave', family: 'Editor', mode: 'dark', accent: '#7e9cd8' },
  { id: 'vitesse-dark', label: 'Vitesse Dark', family: 'Editor', mode: 'dark', accent: '#e2b776' },
  { id: 'aura-dark', label: 'Aura Dark', family: 'Editor', mode: 'dark', accent: '#a277ff' },
  { id: 'horizon', label: 'Horizon', family: 'Editor', mode: 'dark', accent: '#e95678' },
  { id: 'moonlight-ii', label: 'Moonlight II', family: 'Editor', mode: 'dark', accent: '#82aaff' },
  { id: 'synthwave-84', label: "SynthWave '84", family: 'Editor', mode: 'dark', accent: '#ff7edb' },
  { id: 'github-dark-dimmed', label: 'GitHub Dark Dimmed', family: 'Application', mode: 'dark', accent: '#539bf5' },
  { id: 'tomorrow-night', label: 'Tomorrow Night', family: 'Editor', mode: 'dark', accent: '#81a2be' },
  { id: 'noctis', label: 'Noctis', family: 'Editor', mode: 'dark', accent: '#7f9cff' },
  { id: 'andromeda', label: 'Andromeda', family: 'Editor', mode: 'dark', accent: '#04d9c4' }
] as const satisfies readonly ShellTheme[];

export type ShellThemeId = (typeof SHELL_THEMES)[number]['id'];

export const defaultShellThemeId: ShellThemeId = 'base-dark';

const THEME_IDS = new Set<string>(SHELL_THEMES.map((theme) => theme.id));

export function shellThemeOptions(): ShellTheme[] {
  return SHELL_THEMES.map((theme) => ({ ...theme }));
}

export function normalizeShellThemeId(value: unknown): ShellThemeId {
  return typeof value === 'string' && THEME_IDS.has(value)
    ? (value as ShellThemeId)
    : defaultShellThemeId;
}

export function shellThemeById(value: unknown): ShellTheme {
  const id = normalizeShellThemeId(value);
  return SHELL_THEMES.find((theme) => theme.id === id) ?? SHELL_THEMES[0];
}

export function storedShellThemeId(storage: ThemeStorageLike | null = browserStorage()): ShellThemeId {
  try {
    return normalizeShellThemeId(storage?.getItem(SHELL_THEME_STORAGE_KEY));
  } catch (_error) {
    return defaultShellThemeId;
  }
}

export function getInitialShellThemeId(): ShellThemeId {
  return storedShellThemeId();
}

export function applyShellTheme(
  value: unknown,
  options: {
    documentElement?: ThemeDocumentElementLike | null;
    storage?: ThemeStorageLike | null;
  } = {}
): ShellTheme {
  const theme = shellThemeById(value);
  const documentElement = options.documentElement ?? browserDocumentElement();
  if (documentElement) {
    documentElement.dataset.theme = theme.id;
    if (documentElement.style) {
      documentElement.style.colorScheme = theme.mode;
    }
  }

  if (options.storage !== null) {
    try {
      (options.storage ?? browserStorage())?.setItem(SHELL_THEME_STORAGE_KEY, theme.id);
    } catch (_error) {
      // Theme preference is cosmetic; failed persistence must not break shell surfaces.
    }
  }

  return theme;
}

export function setShellTheme(value: unknown): ShellTheme {
  const theme = applyShellTheme(value);
  broadcastTheme(theme.id);
  return theme;
}

export function installShellThemeSync(): () => void {
  const theme = applyShellTheme(storedShellThemeId(), { storage: null });
  const channel = browserBroadcastChannel();

  const applyRemoteTheme = (value: unknown) => {
    applyShellTheme(value, { storage: null });
  };

  const handleStorage = (event: StorageEvent) => {
    if (event.key === SHELL_THEME_STORAGE_KEY) {
      applyRemoteTheme(event.newValue);
    }
  };

  if (channel) {
    channel.onmessage = (event) => applyRemoteTheme(event.data);
  }

  if (typeof window !== 'undefined') {
    window.addEventListener('storage', handleStorage);
  }

  applyRemoteTheme(theme.id);

  return () => {
    if (channel) {
      channel.close();
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('storage', handleStorage);
    }
  };
}

function broadcastTheme(id: string): void {
  const channel = browserBroadcastChannel();
  try {
    channel?.postMessage(id);
  } finally {
    channel?.close();
  }
}

function browserStorage(): ThemeStorageLike | null {
  return typeof window === 'undefined' ? null : window.localStorage;
}

function browserDocumentElement(): ThemeDocumentElementLike | null {
  return typeof document === 'undefined' ? null : document.documentElement;
}

function browserBroadcastChannel(): BroadcastChannel | null {
  if (typeof BroadcastChannel === 'undefined') {
    return null;
  }
  try {
    return new BroadcastChannel(SHELL_THEME_CHANNEL);
  } catch (_error) {
    return null;
  }
}
