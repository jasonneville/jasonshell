import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import {
  classifyStackMarqueeStartTarget
} from '../dist-tests/lib/stackPopupViewModel.js';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');

function target({ self = false, closest = {} } = {}) {
  return {
    self,
    closest: (selector) => Boolean(closest[selector])
  };
}

test('classifies full details body and margin surfaces as marquee start zones', () => {
  assert.equal(classifyStackMarqueeStartTarget(target({ self: true })), 'body');
  assert.equal(classifyStackMarqueeStartTarget(target({ closest: { '[data-stack-marquee-start="body"]': true } })), 'body');
  assert.equal(classifyStackMarqueeStartTarget(target({ closest: { '.virtual-spacer[data-stack-marquee-start]': true } })), 'spacer');
});

test('blocks interactive row and chrome targets from marquee start', () => {
  for (const selector of [
    '[role="row"]',
    'button',
    'input',
    '.inline-editor',
    '.stack-toolbar',
    '.context-menu',
    '.stack-resize-grip'
  ]) {
    assert.equal(classifyStackMarqueeStartTarget(target({ closest: { [selector]: true } })), 'blocked');
  }
});

test('details pane owns pointerdown for full margin selection, rows and resize keep ownership', () => {
  assert.match(surface, /class:marquee-selecting=\{!!marqueeSelection\}/);
  assert.match(surface, /class="stack-popup"[\s\S]*on:pointerdown=\{beginMarqueeSelection\}/);
  assert.match(surface, /data-stack-marquee-start="body"/);
  assert.match(surface, /class="stack-resize-grip"/);
  assert.match(surface, /on:pointerdown=\{beginResize\}/);
  assert.match(surface, /function isStackMarqueeScrollbarTarget\(event: PointerEvent\)/);
  assert.match(surface, /event\.offsetX >= detailsBody\.clientWidth/);
  assert.doesNotMatch(surface, /on:pointerdown=\{\(event\) => beginMarqueeSelection\(event, entry\)\}/);
});
