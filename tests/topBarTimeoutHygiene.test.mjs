import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');

test('top bar tracks and clears delayed rail scroll updates', () => {
  assert.match(topBarSource, /let railScrollUpdateTimeout: number \| null = null/);
  assert.match(topBarSource, /function scheduleRailScrollButtonUpdate\(\)/);
  assert.match(topBarSource, /window\.clearTimeout\(railScrollUpdateTimeout\)/);
  assert.match(topBarSource, /railScrollUpdateTimeout = window\.setTimeout\(\(\) => \{/);
  assert.match(topBarSource, /railScrollUpdateTimeout = null;[\s\S]*updateRailScrollButtons\(\)/);
  assert.match(topBarSource, /function cancelRailScrollButtonUpdate\(\)/);
  assert.match(topBarSource, /cancelRailScrollButtonUpdate\(\);[\s\S]*if \(pinDropStatusTimer !== null\)/);
  assert.doesNotMatch(topBarSource, /(?<!window\.)setTimeout\(updateRailScrollButtons, 160\)/);
});
