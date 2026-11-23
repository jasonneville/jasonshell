import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

// Static boundary evidence only: real native ConPTY/ANSI/canvas rendering is not
// certified by source checks. VIS-14 changed this contract: the old assertions
// pinned a hardcoded Stack theme (#06080b navy) and "no theme wiring", which are
// exactly the defects. Behavior (full theme, live re-apply, no leaks) is owned by
// tests/components/terminal-theme.test.ts and tests/terminalTheme.test.mjs; this
// file keeps only the line-geometry boundary and the "no custom palette" rule.
test('Stack terminal no longer hardcodes navy theme literals and uses the shared theme module', () => {
  const source = readFileSync(new URL('../src/components/StackTerminalPane.svelte', import.meta.url), 'utf8');
  assert.doesNotMatch(source, /#06080b|#d7e2f5|#315f8c/i);
  assert.match(source, /terminalTheme/);
  assert.match(source, /minimumContrastRatio|bindTerminalTheme/);
});

test('persistent terminal constructors keep unchanged line geometry and gain the shared theme', () => {
  const source = readFileSync(new URL('../src/components/TerminalPanelSurface.svelte', import.meta.url), 'utf8');
  const constructors = [...source.matchAll(/new Terminal\(\{([\s\S]*?)\n\s*\}\)/g)];
  assert.equal(constructors.length, 2);
  for (const [, options] of constructors) {
    assert.match(options, /lineHeight:\s*1\.25/);
    assert.match(options, /letterSpacing:\s*0/);
    assert.match(options, /fontSize:\s*terminalFontSize/);
  }
  assert.match(source, /terminalTheme/);
  // The theme editor still exposes no terminal palette (spec: no custom terminal palette contract).
  assert.doesNotMatch(source, /--js-custom-/);
});
