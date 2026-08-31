import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');

test('bottom bar allocates preview request ids from shared native state', () => {
  assert.match(bottomBarSource, /allocateTaskPreviewRequestId/);
  assert.match(bottomBarSource, /const requestId = await allocateTaskPreviewRequestId\(\);/);
  assert.doesNotMatch(bottomBarSource, /let previewRequestId\s*=/);
  assert.doesNotMatch(bottomBarSource, /function nextPreviewRequestId\(/);
});
