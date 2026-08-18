import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const stackPopupSurfaceSource = readFileSync(
  new URL('../src/components/StackPopupSurface.svelte', import.meta.url),
  'utf8'
);
const stackPopupCssSource = readFileSync(
  new URL('../src/components/StackPopupSurface.css', import.meta.url),
  'utf8'
);

test('stack browser marquee starts only from details background and spacer surfaces', () => {
  assert.match(stackPopupSurfaceSource, /function beginMarqueeSelection\(event: PointerEvent\)/);
  assert.match(stackPopupSurfaceSource, /isStackMarqueeStartTarget\(event\.target\)/);
  assert.match(stackPopupSurfaceSource, /class="details-table"[\s\S]*on:pointerdown=\{beginMarqueeSelection\}/);
  assert.match(stackPopupSurfaceSource, /data-stack-marquee-start="body"/);
  assert.match(stackPopupSurfaceSource, /data-stack-marquee-start="spacer"/);
  assert.doesNotMatch(stackPopupSurfaceSource, /on:pointerdown=\{\(event\) => beginMarqueeSelection\(event, entry\)\}/);
});

test('stack browser marquee preserves row drag and resize pointer ownership', () => {
  assert.match(stackPopupSurfaceSource, /on:dragstart=\{\(event\) => handleRowDragStart\(event, entry\)\}/);
  assert.match(stackPopupSurfaceSource, /class="stack-resize-grip"/);
  assert.match(stackPopupSurfaceSource, /on:pointerdown=\{beginResize\}/);
  assert.match(stackPopupSurfaceSource, /STACK_BROWSER_BACKGROUND_CONTEXT_MENU_IGNORE_SELECTORS/);
});

test('stack browser marquee has overlay styling and modest gutter affordance', () => {
  assert.match(stackPopupCssSource, /\.details-table\.marquee-selecting/);
  assert.match(stackPopupCssSource, /\.details-body\.marquee-selecting/);
  assert.match(stackPopupCssSource, /\.stack-marquee-rect/);
  assert.match(stackPopupCssSource, /border:\s*1px solid var\(--js-color-accent-border\)/);
  assert.match(stackPopupCssSource, /padding-left:\s*0\.35rem/);
});
