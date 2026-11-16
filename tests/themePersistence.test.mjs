import assert from 'node:assert/strict';
import test from 'node:test';
import { applyShellTheme, storedShellThemeId, SHELL_THEME_STORAGE_KEY } from '../dist-tests/lib/themes.js';

const CANONICAL_KEY = 'jasonshell.themes.v1';
function envelope(selectedId = 'nord', customThemes = []) {
  return { version: 1, revision: 'fixture:1', selectedId, customThemes };
}
function memoryStorage(entries = []) {
  const values = new Map(entries);
  const writes = [];
  return {
    values, writes,
    getItem: key => values.get(key) ?? null,
    setItem: (key, value) => { writes.push([key, value]); values.set(key, value); }
  };
}

test('fresh renderer hydrates canonical selection before consulting a stale legacy mirror', () => {
  const storage = memoryStorage([
    [CANONICAL_KEY, JSON.stringify(envelope('nord'))],
    [SHELL_THEME_STORAGE_KEY, 'dracula']
  ]);
  assert.equal(storedShellThemeId(storage), 'nord');
});

test('legacy selection remains readable when no canonical envelope exists', () => {
  const storage = memoryStorage([[SHELL_THEME_STORAGE_KEY, 'dracula']]);
  assert.equal(storedShellThemeId(storage), 'dracula');
});

test('missing canonical selection recovers to Base Dark, not a stale legacy selection', () => {
  const storage = memoryStorage([
    [CANONICAL_KEY, JSON.stringify(envelope('custom:deleted'))],
    [SHELL_THEME_STORAGE_KEY, 'dracula']
  ]);
  assert.equal(storedShellThemeId(storage), 'base-dark');
});

test('valid saved custom selection survives fresh hydration without builtin-ID normalization', async () => {
  const { getPresetThemePalette } = await import('../dist-tests/lib/themePalette.js');
  const palette = getPresetThemePalette('nord');
  const custom = {
    id: 'custom:fixture', label: 'Saved Nord', baseBuiltinId: 'nord', mode: 'dark',
    revision: 1, startingPalette: { ...palette }, palette: { ...palette, '--js-color-accent': '#123456' }
  };
  const storage = memoryStorage([
    [CANONICAL_KEY, JSON.stringify(envelope(custom.id, [custom]))],
    [SHELL_THEME_STORAGE_KEY, 'base-dark']
  ]);
  assert.equal(storedShellThemeId(storage), custom.id);
});

for (const [name, raw] of [
  ['malformed JSON', '{broken'],
  ['wrong envelope type', '[]'],
  ['future schema', JSON.stringify({ ...envelope('nord'), version: 999 })],
  ['over 512KiB including JSON whitespace', ' '.repeat(512 * 1024) + JSON.stringify(envelope('nord'))]
]) {
  test(`${name} recovers safely and leaves stored bytes unchanged`, () => {
    const storage = memoryStorage([[CANONICAL_KEY, raw]]);
    assert.equal(storedShellThemeId(storage), 'base-dark');
    assert.equal(storage.values.get(CANONICAL_KEY), raw);
    assert.deepEqual(storage.writes, []);
  });
}

test('unavailable storage reads recover without throwing', () => {
  assert.equal(storedShellThemeId({ getItem() { throw new Error('denied'); }, setItem() { throw new Error('denied'); } }), 'base-dark');
});

test('selection storage failure never applies a falsely successful new selection', () => {
  const documentElement = { dataset: { theme: 'nord' }, style: { colorScheme: 'dark' } };
  const storage = memoryStorage([[CANONICAL_KEY, JSON.stringify(envelope('nord'))]]);
  storage.setItem = () => { throw new Error('quota'); };
  assert.throws(() => applyShellTheme('base-light', { storage, documentElement }), /save|storage|full/i);
  assert.equal(documentElement.dataset.theme, 'nord');
  assert.equal(documentElement.style.colorScheme, 'dark');
  assert.equal(storedShellThemeId(storage), 'nord');
});

test('future schema refuses mutations without touching canonical bytes or legacy mirror', async () => {
  const { writeSavedThemeState, emptyThemeState } = await import('../dist-tests/lib/themePersistence.js');
  const raw = JSON.stringify({ ...envelope(), version: 2 });
  const storage = memoryStorage([[CANONICAL_KEY, raw], [SHELL_THEME_STORAGE_KEY, 'dracula']]);
  assert.throws(() => writeSavedThemeState(emptyThemeState(), 'initial', storage), /newer|protected|schema/i);
  assert.equal(storage.values.get(CANONICAL_KEY), raw);
  assert.equal(storage.values.get(SHELL_THEME_STORAGE_KEY), 'dracula');
  assert.deepEqual(storage.writes, []);
});

test('canonical write atomically publishes collection and selection, then compatibility mirror', async () => {
  const { writeSavedThemeState } = await import('../dist-tests/lib/themePersistence.js');
  const { createCustomTheme } = await import('../dist-tests/lib/themePalette.js');
  const custom = createCustomTheme('nord', 'Custom', 'custom:fixture');
  const storage = memoryStorage();
  const saved = writeSavedThemeState(envelope(custom.id, [custom]), 'initial', storage);
  assert.equal(storage.writes.length, 2);
  assert.equal(storage.writes[0][0], CANONICAL_KEY);
  const canonical = JSON.parse(storage.writes[0][1]);
  assert.equal(canonical.selectedId, custom.id);
  assert.equal(canonical.customThemes[0].id, custom.id);
  assert.notEqual(canonical.revision, 'fixture:1');
  assert.deepEqual(saved, canonical);
  assert.deepEqual(storage.writes[1], [SHELL_THEME_STORAGE_KEY, custom.id]);
  custom.palette['--js-color-accent'] = '#654321';
  assert.notEqual(saved.customThemes[0].palette['--js-color-accent'], '#654321');
});

test('compatibility mirror failure does not negate successful canonical save', async () => {
  const { writeSavedThemeState } = await import('../dist-tests/lib/themePersistence.js');
  const storage = memoryStorage();
  const write = storage.setItem;
  storage.setItem = (key, value) => { if (key === SHELL_THEME_STORAGE_KEY) throw new Error('legacy quota'); write(key, value); };
  const saved = writeSavedThemeState(envelope('nord'), 'initial', storage);
  assert.equal(storedShellThemeId(storage), 'nord');
  assert.equal(saved.selectedId, 'nord');
});

test('stale envelope revision and record edit/delete revisions reject without writes', async () => {
  const { writeSavedThemeState, persistCustomTheme, persistDeleteCustomTheme } = await import('../dist-tests/lib/themePersistence.js');
  const { createCustomTheme } = await import('../dist-tests/lib/themePalette.js');
  const storage = memoryStorage();
  const custom = createCustomTheme('nord', 'Custom', 'custom:fixture');
  const saved = persistCustomTheme(custom, null, storage);
  const before = storage.values.get(CANONICAL_KEY);
  storage.writes.length = 0;
  assert.throws(() => writeSavedThemeState(envelope('dracula'), 'outdated', storage), /changed|another window/i);
  assert.throws(() => persistCustomTheme(custom, null, storage), /changed|deleted/i);
  assert.throws(() => persistDeleteCustomTheme(custom.id, 0, storage), /changed|deleted/i);
  assert.equal(storage.values.get(CANONICAL_KEY), before);
  assert.deepEqual(storage.writes, []);
  assert.equal(saved.customThemes[0].revision, 1);
});

test('editing retains baseline, increments record revision; active deletion selects Base Dark atomically', async () => {
  const { persistCustomTheme, persistDeleteCustomTheme } = await import('../dist-tests/lib/themePersistence.js');
  const { createCustomTheme } = await import('../dist-tests/lib/themePalette.js');
  const storage = memoryStorage();
  const first = persistCustomTheme(createCustomTheme('nord', 'Custom', 'custom:fixture'), null, storage).customThemes[0];
  const originalBaseline = structuredClone(first.startingPalette);
  const edited = { ...first, label: 'Edited', palette: { ...first.palette, '--js-color-accent': '#123456' } };
  const second = persistCustomTheme(edited, first.revision, storage);
  assert.equal(second.customThemes[0].revision, 2);
  assert.deepEqual(second.customThemes[0].startingPalette, originalBaseline);
  storage.writes.length = 0;
  const deleted = persistDeleteCustomTheme(first.id, 2, storage);
  assert.equal(deleted.selectedId, 'base-dark');
  assert.deepEqual(deleted.customThemes, []);
  const atomic = JSON.parse(storage.writes[0][1]);
  assert.equal(atomic.selectedId, 'base-dark');
  assert.deepEqual(atomic.customThemes, []);
});

test('inactive deletion preserves selection and unrelated custom records', async () => {
  const { persistCustomTheme, persistDeleteCustomTheme } = await import('../dist-tests/lib/themePersistence.js');
  const { createCustomTheme } = await import('../dist-tests/lib/themePalette.js');
  const storage = memoryStorage();
  persistCustomTheme(createCustomTheme('nord', 'First', 'custom:first'), null, storage);
  const second = persistCustomTheme(createCustomTheme('dracula', 'Second', 'custom:second'), null, storage);
  const result = persistDeleteCustomTheme('custom:first', 1, storage);
  assert.equal(result.selectedId, 'custom:second');
  assert.deepEqual(result.customThemes, second.customThemes.filter(theme => theme.id === 'custom:second'));
});

test('saved envelope rejects duplicates, oversized collection, invalid revision and malformed custom records', async () => {
  const { validateSavedThemeState } = await import('../dist-tests/lib/themePersistence.js');
  const { createCustomTheme } = await import('../dist-tests/lib/themePalette.js');
  const custom = createCustomTheme('nord', 'Custom', 'custom:fixture');
  assert.throws(() => validateSavedThemeState(envelope(custom.id, [custom, custom])), /Duplicate/i);
  assert.throws(() => validateSavedThemeState(envelope('nord', Array.from({ length: 101 }, (_, i) => ({ ...custom, id: `custom:${i}` })))), /100/);
  assert.throws(() => validateSavedThemeState({ ...envelope(), revision: '' }), /revision/i);
  assert.throws(() => validateSavedThemeState(envelope('nord', [{ ...custom, palette: {} }])));
});
