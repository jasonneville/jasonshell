import assert from 'node:assert/strict';
import test from 'node:test';
import { THEME_PALETTE_TOKENS, createCustomTheme, setThemePaletteColor } from '../dist-tests/lib/themePalette.js';
import { applyThemeToElement } from '../dist-tests/lib/themes.js';
import { parseColor } from './support/color.mjs';
import { readRepoFile } from './support/repoSource.mjs';

function element() {
  const style = new Map();
  return { dataset: {}, props: style, style: { setProperty: (name, value) => style.set(name, value), removeProperty: (name) => { style.delete(name); } } };
}

// Opaque either at the property itself or because app.css strips alpha where it is consumed.
function assertEffectivelyOpaque(value) {
  if (parseColor(value).a === 1) return;
  assert.match(readRepoFile('src/app.css'), /--js-focus-color:\s*rgb\(from var\(--js-custom-focus-color.*\) r g b\)/, `${value} is translucent and app.css does not strip its alpha`);
}

test('custom theme applies an OPAQUE --js-custom-focus-color derived from the focus ring, without a new palette token', () => {
  const theme = createCustomTheme('base-dark', 'Focus');
  const el = element();
  applyThemeToElement(theme, el);
  const value = el.props.get('--js-custom-focus-color');
  assert.ok(value, '--js-custom-focus-color must be set for custom themes');
  assertEffectivelyOpaque(value);
  assert.ok(!THEME_PALETTE_TOKENS.includes('--js-focus-color'), 'focus colour is derived, not an editable palette token');
});

test('editing the focus ring changes the derived focus colour; built-in apply removes it', () => {
  const base = createCustomTheme('base-dark', 'Focus');
  const edited = { ...base, palette: setThemePaletteColor(base.palette, '--js-focus-ring', '#ff0000', 0.5) };
  const a = element(); const b = element();
  applyThemeToElement(base, a); applyThemeToElement(edited, b);
  assert.notEqual(a.props.get('--js-custom-focus-color'), b.props.get('--js-custom-focus-color'));
  assertEffectivelyOpaque(b.props.get('--js-custom-focus-color'));
  applyThemeToElement('base-dark', a);
  assert.equal(a.props.has('--js-custom-focus-color'), false);
});

test('no source declares outline as the shadow-only --js-focus-ring token (invalid value)', () => {
  for (const file of ['src/features/stack-browser/stackTextEditorAdapter.ts', 'src/app.css']) {
    assert.doesNotMatch(readRepoFile(file), /outline['"]?\s*:\s*['"]?var\(--js-focus-ring\)/, file);
  }
  const adapter = readRepoFile('src/features/stack-browser/stackTextEditorAdapter.ts');
  const focused = adapter.match(/'&\.cm-focused'\s*:\s*\{([^}]*)\}/)?.[1] ?? '';
  assert.match(focused, /outline:\s*'\d+(?:\.\d+)?px solid [^']+'/, 'editor focus needs a valid outline: <n>px solid <colour>');
});
