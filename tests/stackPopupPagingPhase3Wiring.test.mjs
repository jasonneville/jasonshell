import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const stackPopupSource = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');
const stackPopupSurfaceSource = readFileSync(
  new URL('../src/components/StackPopupSurface.svelte', import.meta.url),
  'utf8'
);
const stackPopupRustSource = readFileSync(new URL('../src-tauri/src/stack_popup.rs', import.meta.url), 'utf8');

test('stack popup exposes an icon-resolution command path distinct from metadata paging', () => {
  assert.match(stackPopupSource, /resolveStackItemIcons/);
  assert.match(stackPopupSource, /IPC_COMMANDS\.resolveStackItemIcons/);
  assert.match(stackPopupRustSource, /pub async fn resolve_stack_item_icons\(/);
});

test('stack popup icon hydration queue is bounded and does not schedule unbounded work', () => {
  assert.match(stackPopupSurfaceSource, /STACK_ICON_RESOLVE_BATCH_SIZE = \d+/);
  assert.match(stackPopupSurfaceSource, /STACK_ICON_RESOLVE_MAX_CONCURRENCY = \d+/);
  assert.match(stackPopupSurfaceSource, /scheduleVisibleIconHydration\(/);
});

test('stack popup tracks icon hydration completion separately from metadata loading', () => {
  assert.match(stackPopupSurfaceSource, /iconHydrationStatusMessage/);
  assert.match(stackPopupSurfaceSource, /stackIconHydrationStatus\(/);
});
