import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const bottomBarSource = readFileSync(new URL('../src/components/BottomBar.svelte', import.meta.url), 'utf8');

test('bottom bar seeds preview request ids from current epoch instead of zero', () => {
  assert.match(bottomBarSource, /let previewRequestId = Date\.now\(\);/);
  assert.doesNotMatch(bottomBarSource, /let previewRequestId = 0;/);
  assert.match(bottomBarSource, /function nextPreviewRequestId\(\) \{\s*previewRequestId \+= 1;\s*return previewRequestId;/);
});
