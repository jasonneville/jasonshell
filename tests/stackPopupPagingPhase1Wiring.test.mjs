import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const pagingSource = readFileSync(new URL('../src-tauri/src/stack_popup/paging.rs', import.meta.url), 'utf8');
const stackPopupSurfaceSource = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');

test('stack folder first-paint path materializes metadata rows without shell icon extraction calls', () => {
  assert.match(pagingSource, /stack_item_metadata_from_path\(/);
  assert.doesNotMatch(pagingSource, /stack_item_from_path\(/);
});

test('stack popup rows render fallback icon shape when iconDataUrl is absent', () => {
  assert.match(stackPopupSurfaceSource, /\{#if entry\.iconDataUrl\}/);
  assert.match(stackPopupSurfaceSource, /\{:else\}/);
  assert.match(stackPopupSurfaceSource, /stackFileIconForEntry\(entry\)/);
});
