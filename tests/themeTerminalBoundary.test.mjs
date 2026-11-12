import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

// Static boundary evidence only: real native ConPTY/ANSI/canvas rendering is not
// certified by source checks. Existing terminal component/runtime suites cover
// the unaffected execution paths in the full validation gate.
test('Stack terminal retains its existing hardcoded canvas/cursor colors and default ANSI palette', () => {
  const source = readFileSync(new URL('../src/components/StackTerminalPane.svelte', import.meta.url), 'utf8');
  const block = source.match(/theme:\s*\{([^}]+)\}/)?.[1];
  assert.ok(block);
  const colors = Object.fromEntries([...block.matchAll(/(\w+):\s*'([^']+)'/g)].map(([, key, value]) => [key, value]));
  assert.deepEqual(colors, {
    background: '#06080b', foreground: '#d7e2f5', cursor: '#d7e2f5',
    cursorAccent: '#06080b', selectionBackground: '#315f8c'
  });
  assert.doesNotMatch(source, /(?:subscribeShellThemes|subscribeThemePreview|--js-custom-|options\.theme\s*=)/);
});

test('persistent terminal constructors keep xterm theme/ANSI defaults and unchanged line geometry', () => {
  const source = readFileSync(new URL('../src/components/TerminalPanelSurface.svelte', import.meta.url), 'utf8');
  const constructors = [...source.matchAll(/new Terminal\(\{([\s\S]*?)\n\s*\}\)/g)];
  assert.equal(constructors.length, 2);
  for (const [, options] of constructors) {
    assert.doesNotMatch(options, /\btheme\s*:/);
    assert.match(options, /lineHeight:\s*1\.25/);
    assert.match(options, /letterSpacing:\s*0/);
    assert.match(options, /fontSize:\s*terminalFontSize/);
  }
  assert.doesNotMatch(source, /(?:subscribeShellThemes|subscribeThemePreview|--js-custom-|options\.theme\s*=)/);
});
