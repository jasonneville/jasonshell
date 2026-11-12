import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { SHELL_THEMES } from '../dist-tests/lib/themes.js';

// Capture the existing CSS cascade independently of the implementation's palette
// table. Browser tests must additionally compare computed values, because aliases
// and inherited values cannot be certified by textual equality alone.
const css = readFileSync(new URL('../src/app.css', import.meta.url), 'utf8');
const paletteToken = /^--js-(?:color-|shadow-|focus-ring$|inset-highlight$|scrollbar-|bg-|stack-editor-selection)/;
function declarations(body) {
  return Object.fromEntries([...body.matchAll(/(--js-[\w-]+)\s*:\s*([^;]+);/g)]
    .filter(([, name]) => paletteToken.test(name))
    .map(([, name, value]) => [name, value.trim()]));
}
const rootBody = css.match(/^:root\s*\{([\s\S]*?)^\}/m)?.[1];
assert.ok(rootBody, 'production CSS must retain a readable root palette');
const rootTokens = declarations(rootBody);
function cssPreset(id) {
  const body = css.match(new RegExp(`:root\\[data-theme='${id}'\\]\\s*\\{([\\s\\S]*?)\\n\\}`))?.[1];
  assert.ok(body !== undefined, `missing preset selector: ${id}`);
  return { ...rootTokens, ...declarations(body) };
}

for (const theme of SHELL_THEMES) {
  test(`editable palette preserves every existing ${theme.id} token and alias`, async () => {
    const { getPresetThemePalette } = await import('../dist-tests/lib/themePalette.js');
    assert.equal(typeof getPresetThemePalette, 'function');
    assert.deepEqual(getPresetThemePalette(theme.id), cssPreset(theme.id));
  });
}

test('preset palette lookups return independent clones, including the default fallback', async () => {
  const { getPresetThemePalette } = await import('../dist-tests/lib/themePalette.js');
  const original = getPresetThemePalette('base-dark');
  const edited = getPresetThemePalette('base-dark');
  edited['--js-color-accent'] = '#123456';
  assert.deepEqual(getPresetThemePalette('base-dark'), original);
  assert.deepEqual(getPresetThemePalette('not-a-theme'), original);
});

test('the palette boundary excludes typography, geometry, timing and arbitrary CSS', async () => {
  const { getPresetThemePalette } = await import('../dist-tests/lib/themePalette.js');
  const tokens = getPresetThemePalette('base-dark');
  assert.ok(Object.keys(tokens).length >= 36);
  for (const name of Object.keys(tokens)) assert.match(name, paletteToken);
  for (const name of ['--js-font-sans', '--js-radius-md', '--js-space-1', '--js-motion-fast', 'background-image']) {
    assert.equal(Object.hasOwn(tokens, name), false, name);
  }
});

test('all builtin palettes pass the same strict validator as custom palettes', async () => {
  const { getPresetThemePalette, validateThemePalette } = await import('../dist-tests/lib/themePalette.js');
  for (const { id } of SHELL_THEMES) {
    const palette = getPresetThemePalette(id);
    assert.deepEqual(validateThemePalette(palette), palette, id);
  }
});

test('picker colors normalize shorthand, case and alpha without mutating input palettes', async () => {
  const { getPresetThemePalette, normalizeThemeColor, setThemePaletteColor } = await import('../dist-tests/lib/themePalette.js');
  assert.equal(normalizeThemeColor(' #AbC '), '#aabbcc');
  assert.equal(normalizeThemeColor('#AbC8'), '#aabbcc88');
  assert.equal(normalizeThemeColor('#abcdef', 0), '#abcdef00');
  assert.equal(normalizeThemeColor('#abcdef', 0.5), '#abcdef80');
  assert.equal(normalizeThemeColor('#abcdef', 1), '#abcdefff');
  for (const alpha of [-1, 1.01, NaN, Infinity]) assert.throws(() => normalizeThemeColor('#123456', alpha));
  const source = getPresetThemePalette('nord');
  const original = { ...source };
  const edited = setThemePaletteColor(source, '--js-color-accent', '#123');
  assert.equal(edited['--js-color-accent'], '#112233');
  assert.deepEqual(source, original);
});

test('aliases retain relationships and reset source is not materialized into computed colors', async () => {
  const { getPresetThemePalette, setThemePaletteColor } = await import('../dist-tests/lib/themePalette.js');
  const source = getPresetThemePalette('base-dark');
  assert.equal(source['--js-bg-control'], 'var(--js-color-control)');
  const changed = setThemePaletteColor(source, '--js-color-control', '#123456');
  assert.equal(changed['--js-bg-control'], 'var(--js-color-control)');
  const explicit = setThemePaletteColor(changed, '--js-bg-control', '#654321');
  assert.equal(explicit['--js-bg-control'], '#654321');
  assert.equal(explicit['--js-color-control'], '#123456');
});

for (const [name, value] of [
  ['CSS URL', 'url(https://example.invalid/)'],
  ['declaration injection', '#123456; color:red'],
  ['CSS expression', 'expression(alert(1))'],
  ['unknown alias', 'var(--foreign-color)'],
  ['fallback alias', 'var(--js-color-text, red)'],
  ['unbounded rgba', 'rgba(256,0,0,1)'],
  ['invalid alpha', 'rgba(0,0,0,1.1)'],
  ['nonfinite rgb', 'rgb(NaN,0,0)'],
  ['inheritance escape', 'inherit'],
  ['oversized input', '#'.repeat(513)]
]) {
  test(`palette validator rejects ${name}`, async () => {
    const { getPresetThemePalette, validateThemePalette } = await import('../dist-tests/lib/themePalette.js');
    assert.throws(() => validateThemePalette({ ...getPresetThemePalette('base-dark'), '--js-color-accent': value }));
  });
}

test('validator rejects missing/extra tokens, malformed palettes and direct/indirect alias cycles', async () => {
  const { getPresetThemePalette, validateThemePalette } = await import('../dist-tests/lib/themePalette.js');
  for (const value of [null, [], 'red', 1]) assert.throws(() => validateThemePalette(value));
  const palette = getPresetThemePalette('base-dark');
  const missing = { ...palette }; delete missing['--js-color-text'];
  assert.throws(() => validateThemePalette(missing));
  assert.throws(() => validateThemePalette({ ...palette, '--js-font-sans': 'Comic Sans MS' }));
  assert.throws(() => validateThemePalette({ ...palette, '--js-color-accent': 'var(--js-color-accent)' }));
  assert.throws(() => validateThemePalette({ ...palette, '--js-color-accent': 'var(--js-color-text)', '--js-color-text': 'var(--js-color-accent)' }));
});

test('composite tint edits retain each preset geometry and reject arbitrary shadow structure', async () => {
  const { getPresetThemePalette, setThemePaletteColor, validateThemePalette } = await import('../dist-tests/lib/themePalette.js');
  for (const { id } of SHELL_THEMES) {
    const source = getPresetThemePalette(id);
    for (const token of ['--js-shadow-raised', '--js-shadow-panel', '--js-focus-ring', '--js-inset-highlight']) {
      const changed = setThemePaletteColor(source, token, '#123456');
      // Replace only the first tint; SynthWave's focus ring has a second shadow
      // whose geometry AND color must remain untouched.
      const expected = source[token].replace(/rgba?\([^)]*\)|#[a-f\d]{3,8}\b|var\(--js-[\w-]+\)/i, '#123456');
      assert.equal(changed[token], expected, `${id} ${token}`);
    }
  }
  assert.throws(() => validateThemePalette({ ...getPresetThemePalette('base-dark'), '--js-shadow-panel': '0 0 9999px #123456' }));
});

test('builtin/custom copies are independent and custom duplication starts from effective edited colors', async () => {
  const { createCustomTheme, getPresetThemePalette, setThemePaletteColor } = await import('../dist-tests/lib/themePalette.js');
  const original = getPresetThemePalette('nord');
  const first = createCustomTheme('nord', '  First  ', 'custom:first');
  assert.equal(first.label, 'First');
  first.palette = setThemePaletteColor(first.palette, '--js-color-accent', '#123456');
  assert.deepEqual(first.startingPalette, original);
  const second = createCustomTheme(first, 'Second', 'custom:second');
  assert.deepEqual(second.startingPalette, first.palette);
  assert.deepEqual(second.palette, first.palette);
  first.palette['--js-color-accent'] = '#654321';
  first.startingPalette['--js-color-bg'] = '#654321';
  assert.equal(second.palette['--js-color-accent'], '#123456');
  assert.deepEqual(getPresetThemePalette('nord'), original);
});

test('custom record validation trims names and rejects identity, size, mode and revision violations', async () => {
  const { createCustomTheme, validateCustomTheme } = await import('../dist-tests/lib/themePalette.js');
  const source = createCustomTheme('nord', 'Valid', 'custom:fixture');
  assert.equal(validateCustomTheme({ ...source, label: '  Valid  ' }).label, 'Valid');
  for (const patch of [
    { id: 'nord' }, { id: 'custom:' }, { label: '' }, { label: ' '.repeat(10) },
    { label: 'a'.repeat(81) }, { label: 'bad\u0000name' }, { baseBuiltinId: 'missing' },
    { mode: 'system' }, { revision: -1 }, { revision: 0.1 }, { revision: Number.MAX_SAFE_INTEGER + 1 }
  ]) assert.throws(() => validateCustomTheme({ ...source, ...patch }), JSON.stringify(patch));
});

test('adversarial repeated-alias DAG is rejected or resolves with bounded output, never exponential CSS expansion', async () => {
  const { getPresetThemePalette, resolveThemePaletteColor } = await import('../dist-tests/lib/themePalette.js');
  const palette = getPresetThemePalette('base-dark');
  const tokens = Object.keys(palette).filter(name => name.startsWith('--js-color-')).slice(0, 16);
  palette[tokens[0]] = '#123456';
  for (let i = 1; i < tokens.length; i++) {
    palette[tokens[i]] = `color-mix(in srgb, var(${tokens[i - 1]}) 50%, var(${tokens[i - 1]}) 50%)`;
  }
  let resolved;
  try { resolved = resolveThemePaletteColor(palette, tokens.at(-1)); }
  catch (error) {
    assert.match(error.message, /limit|large|complex|bound|alias|expand/i);
    return;
  }
  assert.ok(resolved.length <= 4096, `2372-byte alias input expanded into ${resolved.length} characters`);
});

test('alias expansion immediately below the 4096-character cap resolves exactly; the next DAG level is rejected', async () => {
  const { getPresetThemePalette, resolveThemePaletteColor, validateThemePalette, MAX_RESOLVED_THEME_COLOR_LENGTH } = await import('../dist-tests/lib/themePalette.js');
  assert.equal(MAX_RESOLVED_THEME_COLOR_LENGTH, 4096);
  const palette = getPresetThemePalette('base-dark');
  const tokens = Object.keys(palette).filter(name => name.startsWith('--js-color-'));
  let expanded = '#123456';
  palette[tokens[0]] = expanded;
  let index = 1;
  while (true) {
    const next = `color-mix(in srgb, ${expanded} 50%, ${expanded} 50%)`;
    if (next.length > MAX_RESOLVED_THEME_COLOR_LENGTH) break;
    palette[tokens[index]] = `color-mix(in srgb, var(${tokens[index - 1]}) 50%, var(${tokens[index - 1]}) 50%)`;
    expanded = next; index++;
  }
  assert.ok(expanded.length > MAX_RESOLVED_THEME_COLOR_LENGTH / 2);
  assert.deepEqual(validateThemePalette(palette), palette);
  assert.equal(resolveThemePaletteColor(palette, tokens[index - 1]), expanded);
  palette[tokens[index]] = `color-mix(in srgb, var(${tokens[index - 1]}) 50%, var(${tokens[index - 1]}) 50%)`;
  assert.throws(() => validateThemePalette(palette), /limit|large|complex|bound|alias|expand/i);
});

test('SynthWave focus tint layers edit independently without changing either shadow geometry', async () => {
  const { getPresetThemePalette, getThemePaletteColorLayers, setThemePaletteColor, resolveThemePaletteColor } = await import('../dist-tests/lib/themePalette.js');
  const original = getPresetThemePalette('synthwave-84');
  const token = '--js-focus-ring';
  const layers = getThemePaletteColorLayers(original, token);
  assert.deepEqual(layers, ['rgba(255, 126, 219, 0.7)', 'rgba(54, 249, 246, 0.12)']);
  for (let index = 0; index < layers.length; index++) {
    const changed = setThemePaletteColor(original, token, '#123456', undefined, index);
    assert.equal(changed[token], original[token].replace(layers[index], '#123456'));
    assert.equal(resolveThemePaletteColor(changed, token, index), '#123456');
    assert.equal(getThemePaletteColorLayers(changed, token)[1 - index], layers[1 - index]);
  }
  assert.equal(setThemePaletteColor(original, token, '#123456', .5, 1)[token], original[token].replace(layers[1], '#12345680'));
  for (const index of [-1, .5, NaN, Infinity, 2]) {
    assert.throws(() => setThemePaletteColor(original, token, '#123456', undefined, index));
    assert.throws(() => resolveThemePaletteColor(original, token, index));
  }
  assert.equal(getThemePaletteColorLayers(original, '--js-color-accent').length, 1);
  assert.throws(() => setThemePaletteColor(original, '--js-color-accent', '#123456', undefined, 1));
});

test('layer reset restores exact raw baseline precision/aliases and preserves other edits', async () => {
  const { getPresetThemePalette, setThemePaletteColor, resetThemePaletteColorLayer } = await import('../dist-tests/lib/themePalette.js');
  const baseline = getPresetThemePalette('synthwave-84');
  const token = '--js-focus-ring';
  const first = setThemePaletteColor(baseline, token, '#123456', .5, 0);
  const both = setThemePaletteColor(first, token, '#abcdef', .25, 1);
  const resetFirst = resetThemePaletteColorLayer(both, baseline, token, 0);
  assert.equal(resetFirst[token], baseline[token].replace('rgba(54, 249, 246, 0.12)', '#abcdef40'));
  assert.equal(resetThemePaletteColorLayer(resetFirst, baseline, token, 1)[token], baseline[token]);
  const changedAlias = setThemePaletteColor(baseline, '--js-bg-control', '#123456');
  assert.equal(resetThemePaletteColorLayer(changedAlias, baseline, '--js-bg-control')['--js-bg-control'], baseline['--js-bg-control']);
  assert.equal(both[token], '0 0 0 2px #12345680, 0 0 0 6px #abcdef40');
  for (const index of [-1, 2, .5]) assert.throws(() => resetThemePaletteColorLayer(both, baseline, token, index));
  assert.throws(() => resetThemePaletteColorLayer(both, getPresetThemePalette('base-dark'), token, 0), /geometry|layer/i);
});
