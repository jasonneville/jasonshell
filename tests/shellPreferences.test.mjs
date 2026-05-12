import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import {
  SHELL_FONT_OPTIONS,
  SHELL_PREFERENCES_CHANNEL,
  SHELL_PREFERENCES_STORAGE_KEY,
  addShellPreferencesChangeListener,
  applyShellPreferences,
  defaultShellPreferences,
  formatShellDate,
  formatShellTime,
  installShellPreferencesSync,
  normalizeShellPreferences,
  shellFontById
} from '../dist-tests/lib/shellPreferences.js';

const appCss = readFileSync(new URL('../src/app.css', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');

test('Open Sans is the default shell font and is backed by local downloaded assets', () => {
  assert.equal(defaultShellPreferences.fontId, 'open-sans');
  assert.equal(shellFontById('open-sans').label, 'Open Sans');
  assert.ok(SHELL_FONT_OPTIONS.length >= 5);
  assert.match(appCss, /@font-face\s*{[\s\S]*font-family: 'Open Sans'/);
  assert.match(appCss, /assets\/fonts\/open-sans\/open-sans-10\.woff2/);
  assert.match(appCss, /--js-font-sans: 'Open Sans'/);
  assert.ok(mainSource.indexOf('applyShellPreferences(storedShellPreferences(), { storage: null, dispatch: false })') < mainSource.indexOf('mount(App'));
});

test('normalizes shell preferences and applies dataset plus CSS font variables', () => {
  const documentElement = {
    dataset: {},
    styleValues: new Map(),
    style: {
      setProperty(name, value) {
        documentElement.styleValues.set(name, value);
      }
    }
  };
  const storage = new Map();
  const preferences = applyShellPreferences(
    {
      fontId: 'aptos',
      dateFormat: 'yyyy-MM-dd',
      compactDensity: true,
      strongFocusRing: true,
      reducedTransparency: true,
      showSearchShortcutHint: false
    },
    {
      documentElement,
      storage: {
        getItem: (key) => storage.get(key) ?? null,
        setItem: (key, value) => storage.set(key, value)
      },
      dispatch: false
    }
  );

  assert.equal(preferences.fontId, 'aptos');
  assert.equal(preferences.dateFormat, 'yyyy-MM-dd');
  assert.equal(preferences.showSeconds, true);
  assert.equal(documentElement.dataset.shellDensity, 'compact');
  assert.equal(documentElement.dataset.shellFocus, 'strong');
  assert.equal(documentElement.dataset.shellTransparency, 'reduced');
  assert.match(documentElement.styleValues.get('--js-font-sans'), /Aptos/);
  assert.equal(JSON.parse(storage.get(SHELL_PREFERENCES_STORAGE_KEY)).fontId, 'aptos');

  assert.deepEqual(normalizeShellPreferences({ fontId: 'missing', dateFormat: '' }), defaultShellPreferences);
});

test('formats custom date strings and clock options for the top bar', () => {
  const date = new Date('2026-04-28T18:05:09');
  assert.equal(formatShellDate(date, 'yyyy-MM-dd'), '2026-04-28');
  assert.match(formatShellDate(date, 'EEE, MMM d'), /, .* 28/);
  assert.doesNotMatch(formatShellTime(date, { ...defaultShellPreferences, showSeconds: false }), /:09/);
});

test('preferences sync handles storage, BroadcastChannel updates, cleanup, and local listeners', () => {
  const originalWindow = globalThis.window;
  const originalDocument = globalThis.document;
  const originalBroadcastChannel = globalThis.BroadcastChannel;
  const storage = new Map([[SHELL_PREFERENCES_STORAGE_KEY, JSON.stringify({ fontId: 'open-sans' })]]);
  const listeners = new Map();
  const removedListeners = [];
  const dispatched = [];
  const documentElement = {
    dataset: {},
    style: {
      setProperty() {}
    }
  };
  const channels = [];

  class MockBroadcastChannel {
    constructor(name) {
      this.name = name;
      this.onmessage = null;
      this.closed = false;
      channels.push(this);
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
      removeEventListener: (type, listener) => removedListeners.push([type, listener]),
      dispatchEvent: (event) => {
        dispatched.push(event);
        return true;
      }
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
    const seen = [];
    const cleanup = installShellPreferencesSync((preferences) => seen.push(preferences));
    assert.equal(channels[0].name, SHELL_PREFERENCES_CHANNEL);
    assert.equal(seen[0].fontId, 'open-sans');

    listeners.get('storage')?.({
      key: SHELL_PREFERENCES_STORAGE_KEY,
      newValue: JSON.stringify({ fontId: 'segoe-ui', showSeconds: false })
    });
    assert.equal(seen.at(-1).fontId, 'segoe-ui');
    assert.equal(seen.at(-1).showSeconds, false);

    channels[0].onmessage?.({ data: { fontId: 'cascadia', compactDensity: true } });
    assert.equal(seen.at(-1).fontId, 'cascadia');
    assert.equal(seen.at(-1).compactDensity, true);

    const localChanges = [];
    const removeLocal = addShellPreferencesChangeListener((preferences) => localChanges.push(preferences));
    listeners.get('jasonshell:ui-preferences-changed')?.({ detail: { fontId: 'aptos' } });
    assert.equal(localChanges[0].fontId, 'aptos');
    removeLocal();

    cleanup();
    assert.equal(channels[0].closed, true);
    assert.ok(removedListeners.some(([type]) => type === 'storage'));
    assert.ok(dispatched.length >= 2);
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
