import assert from 'node:assert/strict';
import test from 'node:test';
import {
  TERMINAL_THEMES,
  defaultTerminalThemeId,
  normalizeTerminalThemeId,
  terminalThemeById,
  terminalThemeOptions
} from '../dist-tests/lib/terminalThemes.js';

const expectedIds = [
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
];

const requiredThemeKeys = [
  'background',
  'foreground',
  'cursor',
  'cursorAccent',
  'selectionBackground',
  'black',
  'red',
  'green',
  'yellow',
  'blue',
  'magenta',
  'cyan',
  'white',
  'brightBlack',
  'brightRed',
  'brightGreen',
  'brightYellow',
  'brightBlue',
  'brightMagenta',
  'brightCyan',
  'brightWhite'
];

test('terminal.theme registry mirrors JasonShell theme ids with xterm colors', () => {
  assert.equal(defaultTerminalThemeId, 'base-dark');
  assert.deepEqual(TERMINAL_THEMES.map((theme) => theme.id), expectedIds);
  assert.equal(new Set(TERMINAL_THEMES.map((theme) => theme.id)).size, TERMINAL_THEMES.length);

  for (const terminalTheme of TERMINAL_THEMES) {
    assert.ok(terminalTheme.label, `${terminalTheme.id} has a label`);
    assert.ok(terminalTheme.family, `${terminalTheme.id} has a family`);
    assert.match(terminalTheme.mode, /^(dark|light)$/);
    for (const key of requiredThemeKeys) {
      assert.match(terminalTheme.theme[key], /^#[0-9a-f]{3,8}$/i, `${terminalTheme.id}.${key} is a color`);
    }
  }
});

test('terminal.theme normalization falls back to the default id', () => {
  assert.equal(normalizeTerminalThemeId('dracula'), 'dracula');
  assert.equal(normalizeTerminalThemeId('unknown-theme'), defaultTerminalThemeId);
  assert.equal(normalizeTerminalThemeId(null), defaultTerminalThemeId);
  assert.equal(terminalThemeById('github-light').id, 'github-light');
  assert.equal(terminalThemeById('missing').id, defaultTerminalThemeId);
});

test('terminal.theme option helpers return clone-safe values', () => {
  const firstOptions = terminalThemeOptions();
  const secondOptions = terminalThemeOptions();
  assert.deepEqual(firstOptions.map((theme) => theme.id), expectedIds);
  assert.notEqual(firstOptions[0], secondOptions[0]);
  assert.notEqual(firstOptions[0].theme, secondOptions[0].theme);

  firstOptions[0].theme.background = '#ffffff';
  firstOptions[0].label = 'Mutated';
  const fresh = terminalThemeById(defaultTerminalThemeId);
  assert.equal(fresh.label, 'Base Dark');
  assert.notEqual(fresh.theme.background, '#ffffff');
});
