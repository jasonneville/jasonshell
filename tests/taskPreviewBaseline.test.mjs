import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const css = readFileSync(new URL('../src/components/TaskPreviewSurface.css', import.meta.url), 'utf8').replace(/\r\n/g, '\n');
function rule(selector) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const body = css.match(new RegExp(`(?:^|\\n)${escaped}\\s*\\{([^}]*)\\}`))?.[1];
  assert.ok(body, `${selector} exists`);
  return Object.fromEntries([...body.matchAll(/([\w-]+):\s*([^;]+);/g)].map(([, key, value]) => [key, value.trim()]));
}

// User-approved 02 Inset lens replaces prior compact study; behavior invariants stay.
test('inset lens captured/native chrome retains raised bevel, theme gradient, border and soft corners', () => {
  const captured = rule('.preview-surface');
  const native = { ...captured, ...rule('.preview-surface-native') };
  for (const chrome of [captured, native]) {
    assert.match(chrome.background, /var\(--preview-chrome\)/);
    assert.match(captured['--preview-chrome'], /linear-gradient\([\s\S]*var\(--js-bg-surface\)[\s\S]*var\(--js-color-accent\)/);
    assert.match(chrome['box-shadow'], /var\(--js-shadow-raised\)/);
    assert.match(chrome['box-shadow'], /(?:inset|var\(--js-inset-highlight\))/);
    assert.notEqual(chrome.border, 'none');
    assert.equal(chrome['border-radius'], '8px');
  }
  const frame = rule('.preview-surface-native .preview-frame-native');
  assert.equal(frame.background, 'transparent');
  assert.notEqual(frame['border-color'], 'transparent', 'native perimeter remains visible');
});

test('inset lens geometry is border1 padding10 header36 zero gap and fine frame border', () => {
  const surface = rule('.preview-surface');
  assert.equal(surface.padding, '10px');
  assert.equal(surface.gap, '0');
  assert.match(surface.border, /^1px solid /);
  const header = rule('.preview-header');
  assert.equal(header.height, '36px');
  assert.match(rule('.preview-frame,\n.preview-empty').border, /^1px solid /);
  for (const selector of ['.preview-title', '.preview-process']) {
    const text = rule(selector);
    assert.equal(text.overflow, 'hidden');
    assert.equal(text['text-overflow'], 'ellipsis');
    assert.equal(text['white-space'], 'nowrap');
  }
  assert.equal(rule('.preview-process').color, 'var(--js-color-text-muted)');
  const close = rule('.preview-close-button');
  assert.equal(close.position, 'absolute');
  assert.notEqual(close.background, 'none');
  assert.notEqual(close.background, 'transparent');
});

test('raised preview chrome retains keyboard focus tokens', () => {
  const focus = rule('.preview-surface:focus-visible');
  assert.match(focus['box-shadow'], /var\(--js-shadow-raised\)/);
  assert.match(focus['box-shadow'], /var\(--js-focus-ring\)/);
  assert.equal(focus['border-color'], 'var(--js-color-accent-border)');
});
