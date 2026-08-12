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
  injectCustomFontStylesheets,
  installGoogleFontPreference,
  formatShellDate,
  formatShellTime,
  installShellPreferencesSync,
  normalizeShellPreferences,
  parseGoogleFontLink,
  shellFontById,
  shellFontOptions
} from '../dist-tests/lib/shellPreferences.js';

const appCss = readFileSync(new URL('../src/app.css', import.meta.url), 'utf8');
const googleSansCss = readFileSync(new URL('../src/assets/fonts/google-sans/google-fonts.css', import.meta.url), 'utf8');
const googleSansManifest = readFileSync(new URL('../src/assets/fonts/google-sans/manifest.json', import.meta.url), 'utf8');
const googleSansCodeCss = readFileSync(new URL('../src/assets/fonts/google-sans-code/google-fonts.css', import.meta.url), 'utf8');
const mainSource = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8');

test('Open Sans is the default shell font and bundled font options are backed by local downloaded assets', () => {
  assert.equal(defaultShellPreferences.fontId, 'open-sans');
  assert.equal(shellFontById('open-sans').label, 'Open Sans');
  assert.equal(shellFontById('google-sans').label, 'Google Sans');
  assert.equal(shellFontById('google-sans-code').label, 'Google Sans Code');
  assert.ok(SHELL_FONT_OPTIONS.length >= 7);
  assert.match(appCss, /@font-face\s*{[\s\S]*font-family: 'Open Sans'/);
  assert.match(appCss, /assets\/fonts\/open-sans\/open-sans-10\.woff2/);
  assert.match(appCss, /@font-face\s*{[\s\S]*font-family: 'Google Sans'[\s\S]*font-weight: 400[\s\S]*assets\/fonts\/google-sans\/google-sans-latin\.woff2/);
  assert.match(appCss, /@font-face\s*{[\s\S]*font-family: 'Google Sans'[\s\S]*font-weight: 500[\s\S]*assets\/fonts\/google-sans\/google-sans-latin\.woff2/);
  assert.match(appCss, /@font-face\s*{[\s\S]*font-family: 'Google Sans'[\s\S]*font-weight: 700[\s\S]*assets\/fonts\/google-sans\/google-sans-latin\.woff2/);
  assert.match(googleSansCss, /font-family: 'Google Sans'/);
  assert.match(googleSansCss, /font-weight: 400/);
  assert.match(googleSansCss, /font-weight: 500/);
  assert.match(googleSansCss, /font-weight: 700/);
  assert.match(googleSansManifest, /google-sans-latin\.woff2/);
  assert.match(appCss, /@font-face\s*{[\s\S]*font-family: 'Google Sans Code'[\s\S]*font-weight: 300 800/);
  assert.match(appCss, /assets\/fonts\/google-sans-code\/google-sans-code-latin-variable\.woff2/);
  assert.match(googleSansCodeCss, /font-family: 'Google Sans Code'/);
  assert.match(shellFontById('google-sans').stack, /Google Sans/);
  assert.match(shellFontById('google-sans').stack, /Open Sans/);
  assert.match(shellFontById('google-sans-code').stack, /Google Sans Code/);
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
  assert.deepEqual(preferences.customFonts, []);
  assert.equal(preferences.dateFormat, 'yyyy-MM-dd');
  assert.equal(preferences.showSeconds, true);
  assert.equal(documentElement.dataset.shellDensity, 'compact');
  assert.equal(documentElement.dataset.shellFocus, 'strong');
  assert.equal(documentElement.dataset.shellTransparency, 'reduced');
  assert.match(documentElement.styleValues.get('--js-font-sans'), /Aptos/);
  assert.equal(JSON.parse(storage.get(SHELL_PREFERENCES_STORAGE_KEY)).fontId, 'aptos');

  assert.deepEqual(normalizeShellPreferences({ fontId: 'missing', dateFormat: '' }), defaultShellPreferences);
});

test('parses and installs strict https fonts.google.com links as custom CSS2 font preferences', () => {
  const parsedSpecimen = parseGoogleFontLink('https://fonts.google.com/specimen/Open+Sans?query=open');
  assert.equal(parsedSpecimen.label, 'Open Sans');
  assert.equal(parsedSpecimen.cssUrl, 'https://fonts.googleapis.com/css2?family=Open%2BSans&display=swap');
  assert.match(parsedSpecimen.id, /^google-font:/);
  assert.match(parsedSpecimen.stack, /'Open Sans'/);

  const parsedFamily = parseGoogleFontLink('https://fonts.google.com/?family=Roboto:wght@400;700&display=swap');
  assert.equal(parsedFamily.label, 'Roboto');
  assert.equal(parsedFamily.cssUrl, 'https://fonts.googleapis.com/css2?family=Roboto%3Awght%40400%3B700&display=swap');

  assert.equal(parseGoogleFontLink('http://fonts.google.com/specimen/Roboto'), null);
  assert.equal(parseGoogleFontLink('https://fonts.googleapis.com/css2?family=Roboto'), null);
  assert.equal(parseGoogleFontLink('https://example.com/specimen/Roboto'), null);
  assert.equal(parseGoogleFontLink('https://user@fonts.google.com/specimen/Roboto'), null);
  assert.equal(parseGoogleFontLink('https://user:pass@fonts.google.com/specimen/Roboto'), null);
  assert.equal(parseGoogleFontLink('https://fonts.google.com/?query=Roboto'), null);

  const storage = new Map();
  const originalWindow = globalThis.window;
  Object.defineProperty(globalThis, 'window', {
    configurable: true,
    value: {
      localStorage: {
        getItem: (key) => storage.get(key) ?? null,
        setItem: (key, value) => storage.set(key, value)
      },
      dispatchEvent: () => true
    }
  });
  try {
    const preferences = installGoogleFontPreference('https://fonts.google.com/specimen/Roboto', defaultShellPreferences);
    assert.equal(preferences.customFonts.length, 1);
    assert.equal(preferences.fontId, preferences.customFonts[0].id);
    assert.equal(shellFontById(preferences.fontId, preferences.customFonts).label, 'Roboto');
    assert.ok(shellFontOptions(preferences.customFonts).some((font) => font.label === 'Roboto'));
    assert.equal(JSON.parse(storage.get(SHELL_PREFERENCES_STORAGE_KEY)).customFonts[0].label, 'Roboto');
  } finally {
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: originalWindow
    });
  }
});

test('injects one safe stylesheet link per custom Google Fonts CSS URL', () => {
  const appended = [];
  const removed = [];
  const existing = {
    attributes: { 'data-jasonshell-custom-google-font': 'https://fonts.googleapis.com/css2?family=Old&display=swap' },
    getAttribute(name) {
      return this.attributes[name] ?? null;
    },
    remove() {
      removed.push(this);
    }
  };
  const documentLike = {
    querySelectorAll(selector) {
      assert.equal(selector, 'link[data-jasonshell-custom-google-font]');
      return [existing];
    },
    createElement(tagName) {
      assert.equal(tagName, 'link');
      return {
        attributes: {},
        rel: '',
        href: '',
        setAttribute(name, value) {
          this.attributes[name] = value;
        }
      };
    },
    head: {
      appendChild(node) {
        appended.push(node);
      }
    }
  };

  injectCustomFontStylesheets([
    parseGoogleFontLink('https://fonts.google.com/specimen/Roboto'),
    { label: 'Bad', cssUrl: 'https://evil.example/font.css', id: 'google-font:bad', stack: 'Bad' }
  ], documentLike);

  assert.equal(removed.length, 1);
  assert.equal(appended.length, 1);
  assert.equal(appended[0].rel, 'stylesheet');
  assert.equal(appended[0].href, 'https://fonts.googleapis.com/css2?family=Roboto&display=swap');
  assert.equal(appended[0].attributes['data-jasonshell-custom-google-font'], appended[0].href);
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
