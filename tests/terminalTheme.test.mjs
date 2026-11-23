import assert from 'node:assert/strict';
import test from 'node:test';
import { BUILTIN_THEME_PALETTES } from '../dist-tests/lib/builtinThemePalettes.generated.js';
import { THEME_PALETTE_TOKENS } from '../dist-tests/lib/themePalette.js';
import { SHELL_THEMES } from '../dist-tests/lib/themes.js';
import {
  TERMINAL_MINIMUM_CONTRAST_RATIO, TERMINAL_THEME_KEYS, terminalThemeFor
} from '../dist-tests/lib/terminalTheme.js';
import { composite, contrast, luminance, oklabDistance, parseColor, resolvePaletteColor, surfaces } from './support/color.mjs';
import { readRepoFile } from './support/repoSource.mjs';

// VIS-14 pure mapping: applied theme -> complete xterm ITheme. The module
// contract is terminalThemeFor(themeId, palette, mode) with `palette` a resolved
// or var()-linked --js-* palette (built-in generated palette or a custom theme
// palette) and mode 'dark' | 'light'. It is pure and never throws for an
// unknown or custom id; it derives a palette instead.
const ANSI = ['black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white'];
const KEYS = ['background', 'foreground', 'cursor', 'cursorAccent', 'selectionBackground', ...ANSI, ...ANSI.map((name) => `bright${name[0].toUpperCase()}${name.slice(1)}`)];
const theme = (id) => terminalThemeFor(id, BUILTIN_THEME_PALETTES[id], SHELL_THEMES.find((item) => item.id === id).mode);
const opaque = (value) => parseColor(value);

test('contract constants', () => {
  assert.equal(TERMINAL_MINIMUM_CONTRAST_RATIO, 4.5);
  assert.deepEqual([...TERMINAL_THEME_KEYS].sort(), [...KEYS].sort());
  assert.equal(KEYS.length, 21);
});

for (const item of SHELL_THEMES) {
  test(`${item.id}: complete, parseable, readable xterm theme`, () => {
    const result = theme(item.id);
    assert.deepEqual(Object.keys(result).sort(), [...KEYS].sort());
    for (const key of KEYS) {
      assert.equal(typeof result[key], 'string', key);
      const color = opaque(result[key]); // throws when unparseable
      if (key !== 'selectionBackground') assert.equal(color.a, 1, `${key} must be opaque`);
    }
    const bg = opaque(result.background);
    assert.ok(contrast(opaque(result.foreground), bg) >= 4.5, 'foreground/background must be >= 4.5:1');
    assert.ok(contrast(opaque(result.cursor), bg) >= 3, 'cursor must be visible against the background');
    assert.ok(oklabDistance(composite(opaque(result.selectionBackground), bg), bg) >= 0.03, 'selection must be visible');
    const darkTheme = item.mode === 'dark';
    if (darkTheme) { assert.ok(luminance(bg) < 0.2, 'dark theme gets a dark terminal'); assert.ok(luminance(opaque(result.foreground)) > 0.3); }
    else { assert.ok(luminance(bg) > 0.5, 'light theme gets a light terminal'); assert.ok(luminance(opaque(result.foreground)) < 0.4); assert.ok(luminance(opaque(result.black)) < 0.3, 'light-theme ANSI black stays dark'); }
  });
}

const canonical = {
  dracula: { background: '#282a36', red: '#ff5555', green: '#50fa7b', yellow: '#f1fa8c' },
  nord: { background: '#2e3440', red: '#bf616a', green: '#a3be8c', yellow: '#ebcb8b' },
  'tokyo-night': { background: '#1a1b26', red: '#f7768e', green: '#9ece6a', yellow: '#e0af68' },
  'catppuccin-mocha': { background: '#1e1e2e', red: '#f38ba8', green: '#a6e3a1', yellow: '#f9e2af' },
  'solarized-dark': { background: '#002b36', red: '#dc322f', green: '#859900', yellow: '#b58900' },
  'solarized-light': { background: '#fdf6e3', red: '#dc322f', green: '#859900', yellow: '#b58900' },
  'gruvbox-dark': { background: '#282828', red: '#cc241d', green: '#98971a', yellow: '#d79921' },
  monokai: { background: '#272822', red: '#f92672', green: '#a6e22e' }
};
for (const [id, expected] of Object.entries(canonical)) {
  test(`${id} uses its canonical terminal palette`, () => {
    const result = theme(id);
    for (const [key, value] of Object.entries(expected)) assert.equal(result[key].toLowerCase(), value, `${id} ${key}`);
  });
}

test('terminal themes of different built-ins are distinct (not one shared default)', () => {
  const backgrounds = new Set(SHELL_THEMES.map((item) => theme(item.id).background));
  assert.ok(backgrounds.size >= 12, `only ${backgrounds.size} distinct terminal backgrounds`);
  assert.notDeepEqual(theme('dracula'), theme('nord'));
  assert.notDeepEqual(theme('base-dark'), theme('base-light'));
});

function customPalette(overrides) {
  return { ...BUILTIN_THEME_PALETTES['base-dark'], ...overrides };
}
const nearOpaqueSurface = (palette, color) => Object.values(surfaces(palette))
  .concat(composite(resolvePaletteColor(palette, '--js-color-surface-sunken'), surfaces(palette).bg))
  .some((surface) => oklabDistance(surface, parseColor(color)) <= 0.03);

test('custom themes get a derived palette that follows their own surfaces and stays readable', () => {
  const pink = customPalette({ '--js-color-bg': '#301020', '--js-color-surface': '#301020', '--js-color-surface-raised': '#3a1428', '--js-color-surface-sunken': '#240c18', '--js-color-text': '#ffe0f0' });
  const green = customPalette({ '--js-color-bg': '#0a2a14', '--js-color-surface': '#0a2a14', '--js-color-surface-raised': '#103a1c', '--js-color-surface-sunken': '#06200e', '--js-color-text': '#e0ffe8' });
  const one = terminalThemeFor('custom:one', pink, 'dark');
  const two = terminalThemeFor('custom:two', green, 'dark');
  assert.deepEqual(Object.keys(one).sort(), [...KEYS].sort());
  assert.ok(nearOpaqueSurface(pink, one.background), 'derived background follows the custom palette surfaces');
  assert.ok(nearOpaqueSurface(green, two.background));
  assert.notEqual(one.background, two.background);
  assert.notEqual(one.background.toLowerCase(), theme('base-dark').background.toLowerCase());
  assert.ok(contrast(opaque(one.foreground), opaque(one.background)) >= 4.5);
});

test('custom light mode derives a light terminal; unreadable custom text is corrected, not copied', () => {
  const light = customPalette({ '--js-color-bg': '#f4f1ea', '--js-color-surface': '#f4f1ea', '--js-color-surface-raised': '#ffffff', '--js-color-surface-sunken': '#ebe6da', '--js-color-text': '#2a2a2a' });
  const result = terminalThemeFor('custom:light', light, 'light');
  assert.ok(luminance(opaque(result.background)) > 0.5);
  const hostile = customPalette({ '--js-color-bg': '#202020', '--js-color-surface': '#202020', '--js-color-surface-raised': '#202020', '--js-color-surface-sunken': '#202020', '--js-color-text': '#212121', '--js-color-text-strong': '#212121' });
  const fixed = terminalThemeFor('custom:hostile', hostile, 'dark');
  assert.ok(contrast(opaque(fixed.foreground), opaque(fixed.background)) >= 4.5, 'foreground contrast is enforced for custom palettes');
});

test('mapping is pure: deterministic, no input mutation, fresh result objects, unknown ids do not throw', () => {
  const input = JSON.parse(JSON.stringify(BUILTIN_THEME_PALETTES['base-dark']));
  const snapshot = JSON.stringify(input);
  const first = terminalThemeFor('definitely-not-a-theme', input, 'dark');
  assert.equal(JSON.stringify(input), snapshot);
  assert.deepEqual(terminalThemeFor('definitely-not-a-theme', input, 'dark'), first);
  first.background = '#ff00ff';
  assert.notEqual(terminalThemeFor('definitely-not-a-theme', input, 'dark').background, '#ff00ff');
  const frozen = theme('dracula');
  assert.doesNotThrow(() => { frozen.red = '#000000'; });
  assert.equal(theme('dracula').red.toLowerCase(), '#ff5555');
});

test('there is still no custom terminal palette contract in the theme token set', () => {
  assert.equal(THEME_PALETTE_TOKENS.filter((token) => /term|ansi/i.test(token)).length, 0);
  for (const file of ['src/components/ThemeEditor.svelte', 'src/lib/themePalette.ts']) {
    assert.doesNotMatch(readRepoFile(file), /--js-(?:term|ansi)-/, `${file} must not grow terminal palette tokens`);
  }
});

// CSS chrome: no literal navy/blue chrome colours; neutral black/white alpha scrims and shadows are allowed.
for (const file of ['src/components/TerminalPanelSurface.css', 'src/components/StackTerminalPane.css']) {
  test(`${file} chrome uses theme tokens instead of literal colours`, () => {
    const css = readRepoFile(file).replace(/\/\*[\s\S]*?\*\//g, '');
    const literals = [...css.matchAll(/#[\da-f]{3,8}\b|(?:rgba?|hsla?)\([^)]*\)/gi)].map((m) => m[0])
      .filter((value) => !/^rgba?\(\s*(?:0\s*,\s*0\s*,\s*0|255\s*,\s*255\s*,\s*255)\s*,/i.test(value));
    assert.deepEqual(literals, []);
    assert.match(css, /var\(--js-color-/);
  });
}
