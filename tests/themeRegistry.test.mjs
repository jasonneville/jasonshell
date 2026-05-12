import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  SHELL_THEME_STORAGE_KEY,
  SHELL_THEME_CHANNEL,
  SHELL_THEMES,
  applyShellTheme,
  defaultShellThemeId,
  installShellThemeSync,
  normalizeShellThemeId,
  shellThemeOptions
} from '../dist-tests/lib/themes.js';

const appCss = readFileSync(new URL('../src/app.css', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');

test('theme registry exposes base themes and popular editor palettes', () => {
  assert.equal(defaultShellThemeId, 'base-dark');
  assert.ok(SHELL_THEMES.length >= 15);

  for (const id of [
    'base-dark',
    'base-light',
    'monokai',
    'atom-one-dark',
    'atom-one-light',
    'nord',
    'dracula',
    'solarized-dark',
    'solarized-light',
    'github-dark',
    'github-light',
    'gruvbox-dark',
    'gruvbox-light',
    'tokyo-night',
    'catppuccin-mocha',
    'ayu-dark'
  ]) {
    assert.ok(SHELL_THEMES.some((theme) => theme.id === id), `missing ${id}`);
    assert.match(appCss, new RegExp(`data-theme=['"]${id}['"]`));
  }

  assert.equal(new Set(SHELL_THEMES.map((theme) => theme.id)).size, SHELL_THEMES.length);
  assert.deepEqual(shellThemeOptions().map((theme) => theme.id), SHELL_THEMES.map((theme) => theme.id));
});

test('theme normalization and DOM application are storage-safe', () => {
  assert.equal(SHELL_THEME_STORAGE_KEY, 'jasonshell.theme');
  assert.equal(normalizeShellThemeId('dracula'), 'dracula');
  assert.equal(normalizeShellThemeId('unknown-theme'), 'base-dark');
  assert.equal(normalizeShellThemeId(null), 'base-dark');

  const documentElement = {
    dataset: {},
    style: {
      colorScheme: ''
    }
  };
  const storage = new Map();
  const stored = applyShellTheme('github-light', {
    documentElement,
    storage: {
      getItem: (key) => storage.get(key) ?? null,
      setItem: (key, value) => storage.set(key, value)
    }
  });

  assert.equal(stored.id, 'github-light');
  assert.equal(documentElement.dataset.theme, 'github-light');
  assert.equal(documentElement.style.colorScheme, 'light');
  assert.equal(storage.get(SHELL_THEME_STORAGE_KEY), 'github-light');
});

test('theme is applied before Svelte mounts to reduce shell paint mismatch', () => {
  assert.match(mainSource, /applyShellTheme\(storedShellThemeId\(\), \{ storage: null \}\)/);
  assert.ok(mainSource.indexOf('applyShellTheme(storedShellThemeId(), { storage: null })') < mainSource.indexOf('mount(App'));
});

test('theme sync listens for cross-webview updates and releases listeners', () => {
  const originalWindow = globalThis.window;
  const originalDocument = globalThis.document;
  const originalBroadcastChannel = globalThis.BroadcastChannel;
  const storage = new Map([[SHELL_THEME_STORAGE_KEY, 'base-light']]);
  const documentElement = {
    dataset: {},
    style: {
      colorScheme: ''
    }
  };
  const listeners = new Map();
  const removedListeners = [];
  const channels = [];

  class MockBroadcastChannel {
    constructor(name) {
      this.name = name;
      this.onmessage = null;
      this.closed = false;
      channels.push(this);
    }

    postMessage(data) {
      for (const channel of channels) {
        if (channel !== this && !channel.closed) {
          channel.onmessage?.({ data });
        }
      }
    }

    close() {
      this.closed = true;
    }
  }

  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      localStorage: {
        getItem: (key) => storage.get(key) ?? null,
        setItem: (key, value) => storage.set(key, value)
      },
      addEventListener: (type, listener) => listeners.set(type, listener),
      removeEventListener: (type, listener) => removedListeners.push([type, listener])
    }
  });
  Object.defineProperty(globalThis, 'document', {
    configurable: true,
    value: { documentElement }
  });
  Object.defineProperty(globalThis, 'BroadcastChannel', {
    configurable: true,
    value: MockBroadcastChannel
  });

  try {
    const cleanup = installShellThemeSync();
    assert.equal(channels[0].name, SHELL_THEME_CHANNEL);
    assert.equal(documentElement.dataset.theme, 'base-light');
    assert.equal(documentElement.style.colorScheme, 'light');

    listeners.get('storage')?.({ key: SHELL_THEME_STORAGE_KEY, newValue: 'nord' });
    assert.equal(documentElement.dataset.theme, 'nord');
    assert.equal(documentElement.style.colorScheme, 'dark');

    channels[0].onmessage?.({ data: 'dracula' });
    assert.equal(documentElement.dataset.theme, 'dracula');

    cleanup();
    assert.equal(channels[0].closed, true);
    assert.deepEqual(removedListeners, [['storage', listeners.get('storage')]]);
  } finally {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: originalWindow
    });
    Object.defineProperty(globalThis, 'document', {
      configurable: true,
      value: originalDocument
    });
    Object.defineProperty(globalThis, 'BroadcastChannel', {
      configurable: true,
      value: originalBroadcastChannel
    });
  }
});
