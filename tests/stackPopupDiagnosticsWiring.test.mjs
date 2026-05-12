import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const stackPopupSource = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');

test('stack folder listing publishes baseline diagnostics hooks for timing and payload metrics', () => {
  assert.match(stackPopupSource, /type StackFolderListingDiagnostics = \{/);
  assert.match(stackPopupSource, /folderOpenDurationMs: number;/);
  assert.match(stackPopupSource, /pageDurationMs: number;/);
  assert.match(stackPopupSource, /pageItemCount: number;/);
  assert.match(stackPopupSource, /iconResolutionCount: number;/);
  assert.match(stackPopupSource, /iconResolutionDurationMs: number;/);
  assert.match(stackPopupSource, /payloadItemCount: number;/);
  assert.match(stackPopupSource, /function emitStackFolderListingDiagnostics\(/);
  assert.match(stackPopupSource, /emitStackFolderListingDiagnostics\(\{/);
});
