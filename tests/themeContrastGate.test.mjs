import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { BUILTIN_THEME_PALETTES } from '../dist-tests/lib/builtinThemePalettes.generated.js';
import {
  MAX_RESOLVED_THEME_COLOR_LENGTH, THEME_PALETTE_TOKENS, createCustomTheme, getPresetThemePalette,
  resolveThemePaletteColor, validateCustomTheme
} from '../dist-tests/lib/themePalette.js';
import { SHELL_THEMES } from '../dist-tests/lib/themes.js';
import { composite, contrast, hueDistance, oklabDistance, oklch, resolvePaletteColor, surfaces } from './support/color.mjs';
import { readRepoFile } from './support/repoSource.mjs';

// VIS-01 / VIS-03. Every built-in palette is gated against BOTH opaque surfaces
// (surface and surface-raised, each composited over bg), with translucent
// foregrounds composited before measuring. Numeric thresholds come from the
// finding: muted text >= 4.5, subtle text >= 3.0, accent-border >= 3.0 (WCAG
// 1.4.11) and accent-as-text >= 4.5 on surface-raised.
const ids = SHELL_THEMES.map((theme) => theme.id);
const baseline = JSON.parse(readRepoFile('tests/fixtures/theme-identity-baseline.json'));
const palette = (id) => BUILTIN_THEME_PALETTES[id];
const ratio = (id, token, surface) => contrast(resolvePaletteColor(palette(id), token), surfaces(palette(id))[surface]);
// Accent as text: the implementation chose to nudge --js-color-accent itself, or add --js-color-accent-text.
const accentTextToken = (id) => ('--js-color-accent-text' in palette(id) ? '--js-color-accent-text' : '--js-color-accent');

test('34 built-ins are covered by the gate', () => assert.equal(ids.length, 34));

for (const id of ids) {
  test(`${id}: text and state-border contrast gates hold on surface and surface-raised`, () => {
    const failures = [];
    for (const surface of ['surface', 'raised']) {
      for (const [token, min] of [['--js-color-text-muted', 4.5], ['--js-color-text-subtle', 3.0], ['--js-color-accent-border', 3.0]]) {
        const value = ratio(id, token, surface);
        if (value < min) failures.push(`${token} on ${surface}: ${value.toFixed(2)} < ${min}`);
      }
    }
    const accent = ratio(id, accentTextToken(id), 'raised');
    if (accent < 4.5) failures.push(`${accentTextToken(id)} as text on raised: ${accent.toFixed(2)} < 4.5`);
    assert.deepEqual(failures, []);
  });
}

test('base-dark accent family shares one hue, is chromatic, and selected is distinct from the surface', () => {
  const p = palette('base-dark');
  const tokens = ['--js-color-accent', '--js-color-accent-soft', '--js-color-accent-border', '--js-color-selected'];
  const lch = tokens.map((token) => ({ token, ...oklch(resolvePaletteColor(p, token)) }));
  for (const { token, C } of lch) assert.ok(C >= 0.05, `${token} must be a chromatic accent tint, got OKLCH chroma ${C.toFixed(3)}`);
  for (const a of lch) for (const b of lch) assert.ok(hueDistance(a.H, b.H) <= 12, `${a.token} (${a.H.toFixed(1)}) vs ${b.token} (${b.H.toFixed(1)}) differ by more than 12 degrees`);
  const raised = surfaces(p).raised;
  for (const surface of ['surface', 'raised']) {
    const base = surfaces(p)[surface];
    const shown = composite(resolvePaletteColor(p, '--js-color-selected'), base);
    assert.ok(oklabDistance(shown, base) >= 0.04, `selected must be visibly distinct from ${surface} (OKLab distance ${oklabDistance(shown, base).toFixed(3)})`);
  }
  assert.ok(raised.a === 1);
});

test('app.css :root defaults equal the generated base-dark block for every shared token', () => {
  const css = readRepoFile('src/app.css');
  const root = css.match(/^:root\s*\{([\s\S]*?)^\}/m)?.[1];
  assert.ok(root, 'app.css :root block');
  const declared = Object.fromEntries([...root.matchAll(/(--js-[\w-]+)\s*:\s*([^;]+);/g)].map(([, name, value]) => [name, value.trim()]));
  const mismatches = Object.entries(palette('base-dark')).filter(([name, value]) => declared[name] !== value).map(([name]) => name);
  assert.deepEqual(mismatches, []);
});

test('other themes keep identity: surfaces unchanged and text/accent hue shift <= 6 degrees', () => {
  const problems = [];
  for (const id of ids.filter((value) => value !== 'base-dark')) {
    const p = palette(id);
    const was = baseline[id];
    for (const [key, token] of [['bg', '--js-color-bg'], ['surface', '--js-color-surface'], ['surfaceRaised', '--js-color-surface-raised']]) {
      if (p[token] !== was[key]) problems.push(`${id} ${token} changed: ${was[key]} -> ${p[token]} (only lightness of foreground tokens may be nudged)`);
    }
    for (const [key, token] of [['accent', '--js-color-accent'], ['textMuted', '--js-color-text-muted'], ['textSubtle', '--js-color-text-subtle']]) {
      const now = oklch(resolvePaletteColor(p, token));
      if (was[key].C < 0.02 || now.C < 0.02) continue; // Hue of near-neutral colors is not meaningful.
      const shift = hueDistance(was[key].H, now.H);
      if (shift > 6) problems.push(`${id} ${token} hue shifted ${shift.toFixed(1)} degrees`);
    }
  }
  assert.deepEqual(problems, []);
});

test('custom-theme contracts: bounded token set, linked aliases, 4,096-char expansion, immutable built-ins', () => {
  const tokens = Object.keys(palette('base-dark'));
  assert.deepEqual([...THEME_PALETTE_TOKENS], tokens);
  for (const id of ids) {
    assert.deepEqual(Object.keys(palette(id)), tokens, `${id} token set`);
    assert.ok(Object.isFrozen(palette(id)), `${id} frozen`);
    const theme = validateCustomTheme(createCustomTheme(id, `Copy of ${id}`));
    for (const token of tokens) {
      const resolved = resolveThemePaletteColor(theme.palette, token);
      assert.ok(resolved.length <= MAX_RESOLVED_THEME_COLOR_LENGTH, `${id} ${token} expands to ${resolved.length} chars`);
    }
    const copy = getPresetThemePalette(id);
    copy['--js-color-accent'] = '#000000';
    assert.notEqual(palette(id)['--js-color-accent'], '#000000');
  }
  assert.equal(MAX_RESOLVED_THEME_COLOR_LENGTH, 4096);
  // Aliases that were linked stay linked (only literal values may change).
  for (const [name, value] of Object.entries(palette('base-dark'))) {
    if (name.startsWith('--js-bg-')) assert.match(value, /^var\(--js-color-[\w-]+\)$/, `${name} must remain an alias`);
  }
});

// Generator gate: enforced in generate AND --check, before the stale comparison.
const repository = fileURLToPath(new URL('../', import.meta.url));
function generatorFixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'theme-contrast-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const files = ['scripts/generate-theme-palettes.mjs', 'src/app.css', 'src/lib/themes.ts', 'src/lib/builtinThemePalettes.generated.ts'];
  if (existsSync(join(repository, 'scripts/themeContrast.mjs'))) files.push('scripts/themeContrast.mjs'); // Optional helper module name the generator may import.
  for (const relative of files) {
    mkdirSync(dirname(join(root, relative)), { recursive: true });
    copyFileSync(join(repository, relative), join(root, relative));
  }
  return root;
}
function run(root, check) {
  const result = spawnSync(process.execPath, [join(root, 'scripts/generate-theme-palettes.mjs'), ...(check ? ['--check'] : [])], { cwd: root, encoding: 'utf8', timeout: 20_000 });
  assert.equal(result.error, undefined, result.error?.message);
  return result;
}
function edit(root, mutate) {
  const target = join(root, 'src/app.css');
  const before = readFileSync(target, 'utf8');
  const after = mutate(before);
  assert.notEqual(after, before, 'mutation must change app.css');
  writeFileSync(target, after);
}

test('generator accepts the real repository inputs in generate and --check modes', (t) => {
  const root = generatorFixture(t);
  assert.equal(run(root, false).status, 0, run(root, false).stderr);
  assert.equal(run(root, true).status, 0, run(root, true).stderr);
});

const mutations = {
  'muted text below 4.5 in :root': (css) => css.replace(/(:root\s*\{[\s\S]*?--js-color-text-muted:\s*)[^;]+;/, '$1rgba(222, 230, 248, 0.3);'),
  'subtle text below 3.0 in :root': (css) => css.replace(/(:root\s*\{[\s\S]*?--js-color-text-subtle:\s*)[^;]+;/, '$1rgba(222, 230, 248, 0.2);'),
  'accent-border below 3.0 in a preset': (css) => css.replace(/(:root\[data-theme='nord'\]\s*\{[\s\S]*?)(\n\})/, '$1\n  --js-color-accent-border: rgba(136, 192, 208, 0.08);$2')
};
for (const [name, mutate] of Object.entries(mutations)) {
  for (const check of [false, true]) {
    test(`generator rejects ${name} (${check ? '--check' : 'generate'}) with a contrast diagnostic`, (t) => {
      const root = generatorFixture(t);
      edit(root, mutate);
      const result = run(root, check);
      assert.notEqual(result.status, 0, 'gate must fail generation');
      assert.match(result.stderr, /contrast/i);
      assert.doesNotMatch(result.stderr, /palette data stale/i, 'contrast gate must run before the stale comparison');
    });
  }
}
