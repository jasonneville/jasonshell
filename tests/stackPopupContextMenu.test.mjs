import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const stackPopupApi = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');

test('row context menu exposes working Open with picker labels only', () => {
  assert.equal(surface.includes('Open width'), false);
  assert.equal(surface.includes('Default width'), false);
  assert.match(surface, />Open with ▸<\/button>/);
  assert.match(surface, />Choose app\.\.\.<\/button>/);
  assert.match(surface, /openSelectedWithPicker\(\)/);
});

test('Open with picker is backed by a Tauri command wrapper', () => {
  assert.match(stackPopupApi, /openStackItemWithPicker\(path: string\): Promise<void>/);
  assert.match(stackPopupApi, /invoke\('open_stack_item_with_picker', \{ path \}\)/);
});
