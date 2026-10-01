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
  assert.match(stackPopupSurfaceSource, /class="stack-popup"[\s\S]*on:pointerdown=\{beginMarqueeSelection\}/);
  assert.match(stackPopupSurfaceSource, /data-stack-marquee-start="body"/);
  assert.match(stackPopupSurfaceSource, /data-stack-marquee-start="spacer"/);
  assert.doesNotMatch(stackPopupSurfaceSource, /on:pointerdown=\{\(event\) => beginMarqueeSelection\(event, entry\)\}/);
});

test('stack browser marquee preserves row drag and resize pointer ownership', () => {
  const rowStart = stackPopupSurfaceSource.indexOf('<button', stackPopupSurfaceSource.indexOf('{#each virtualEntries.rows'));
  const row = stackPopupSurfaceSource.slice(rowStart, stackPopupSurfaceSource.indexOf('>', stackPopupSurfaceSource.indexOf('on:drop=', rowStart)) + 1);
  assert.match(row, /<button[\s\S]*on:pointerdown=\{\(event\) => beginRowDrag\(event, entry\)\}[\s\S]*on:pointermove=\{\(event\) => moveRowDrag\(event, entry\)\}[\s\S]*on:pointerup=\{endRowDrag\}[\s\S]*on:pointercancel=\{endRowDrag\}/);
  assert.doesNotMatch(row, /on:dragstart=|\bdraggable=/, 'row native drag remains pointer-owned');
  const marqueeStart = stackPopupSurfaceSource.slice(stackPopupSurfaceSource.indexOf('function beginMarqueeSelection('), stackPopupSurfaceSource.indexOf('function ', stackPopupSurfaceSource.indexOf('function beginMarqueeSelection(') + 1));
  assert.match(marqueeStart, /isStackMarqueeScrollbarTarget\(event\)[\s\S]*!isStackMarqueeStartTarget\(event\.target\)/, 'marquee excludes scrollbar and non-background targets');
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
