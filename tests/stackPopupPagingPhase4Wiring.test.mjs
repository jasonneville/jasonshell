import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const stackPopupSurfaceSource = readFileSync(
  new URL('../src/components/StackPopupSurface.svelte', import.meta.url),
  'utf8'
);
const stackPopupViewModelSource = readFileSync(
  new URL('../src/lib/stackPopupViewModel.ts', import.meta.url),
  'utf8'
);

test('stack popup emits visible-row window updates for virtualization-aware progressive loading', () => {
  assert.match(stackPopupSurfaceSource, /STACK_BROWSER_FRONTEND_EVENTS\.folderRowsWindowChanged/);
  assert.match(stackPopupSurfaceSource, /emit\(STACK_BROWSER_FRONTEND_EVENTS\.folderRowsWindowChanged/);
});

test('stack popup progressive page handling uses a guarded focus helper instead of unconditional per-page focus', () => {
  assert.match(stackPopupSurfaceSource, /maybeFocusDetailsGridAfterPageAppend\(/);
  const onPageCallback = stackPopupSurfaceSource.match(
    /listStackFolder\(folderPath,\s*async \(page\) => \{([\s\S]*?)\n\s*\}\);/
  );
  assert.ok(onPageCallback, 'listStackFolder onPage callback not found');
  assert.match(onPageCallback[1], /maybeFocusDetailsGridAfterPageAppend\(\);/);
  assert.doesNotMatch(onPageCallback[1], /focusDetailsGrid\(\);/);
});

test('stack popup virtual window contract remains explicit for progressive append compatibility', () => {
  assert.match(stackPopupViewModelSource, /export type StackBrowserVirtualWindow/);
  assert.match(stackPopupViewModelSource, /beforeHeight: number;/);
  assert.match(stackPopupViewModelSource, /afterHeight: number;/);
});
