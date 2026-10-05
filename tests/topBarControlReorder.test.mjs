import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  DEFAULT_TOP_BAR_CONTROL_ORDER,
  normalizeTopBarControlOrder,
  resolveTopBarControlPointerCaptureTarget,
  resolveTopBarControlPointerRelease,
  shouldSuppressTopBarControlClick,
  topBarControlOrderFromDisplacement
} from '../dist-tests/lib/topBarControls.js';

const topBarSource = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');

const order = ['terminal', 'command', 'tray', 'mic', 'sound'];
const rects = order.map((key, index) => ({ key, left: index * 40, width: 40 }));

test('reorders top-bar controls in both directions after crossing control centers', () => {
  assert.deepEqual(
    topBarControlOrderFromDisplacement('terminal', order, rects, 200),
    ['command', 'tray', 'mic', 'sound', 'terminal']
  );
  assert.deepEqual(
    topBarControlOrderFromDisplacement('sound', order, rects, -200),
    ['sound', 'terminal', 'command', 'tray', 'mic']
  );
});

test('keeps order unchanged for missing controls, invalid rects, and movement before a center', () => {
  assert.equal(topBarControlOrderFromDisplacement('missing', order, rects, 80), order);
  assert.equal(topBarControlOrderFromDisplacement('terminal', order, [], 80), order);
  assert.equal(topBarControlOrderFromDisplacement('terminal', order, rects, 39), order);
});

test('normalizes malformed, stale, and duplicate persisted order to all known controls', () => {
  assert.deepEqual(normalizeTopBarControlOrder(null), DEFAULT_TOP_BAR_CONTROL_ORDER);
  assert.deepEqual(normalizeTopBarControlOrder('{bad json'), DEFAULT_TOP_BAR_CONTROL_ORDER);
  assert.deepEqual(normalizeTopBarControlOrder(JSON.stringify(['sound', 'sound', 'old', 'tray'])), [
    'sound',
    'tray',
    'terminal',
    'command',
    'mic',
    'snip'
  ]);
  assert.deepEqual(normalizeTopBarControlOrder(JSON.stringify({ order })), DEFAULT_TOP_BAR_CONTROL_ORDER);
});

test('suppresses only click released after a real drag', () => {
  assert.deepEqual(resolveTopBarControlPointerRelease('terminal', false), { suppressClickId: null });
  assert.deepEqual(resolveTopBarControlPointerRelease('terminal', true), { suppressClickId: 'terminal' });
  assert.deepEqual(resolveTopBarControlPointerRelease(null, true), { suppressClickId: null });
});

test('pending drag suppression ignores keyboard activation and blocks only the matching pointer click', () => {
  assert.equal(shouldSuppressTopBarControlClick('terminal', 'terminal', { detail: 0 }), false);
  assert.equal(shouldSuppressTopBarControlClick('terminal', 'command', { detail: 1 }), false);
  assert.equal(shouldSuppressTopBarControlClick('terminal', 'terminal', { detail: 1 }), true);
});

test('captures a control pointer on the pressed button so a below-threshold release still clicks it', () => {
  const button = {};
  const wrapper = {
    contains(candidate) {
      return candidate === button;
    }
  };
  const nestedButtonContent = {
    closest(selector) {
      assert.equal(selector, 'button');
      return button;
    }
  };

  assert.equal(resolveTopBarControlPointerCaptureTarget(nestedButtonContent, wrapper), button);
  assert.equal(resolveTopBarControlPointerCaptureTarget(null, wrapper), wrapper);
  assert.match(topBarSource, /topBarControlDragElement = resolveTopBarControlPointerCaptureTarget\(\s*event\.target,\s*event\.currentTarget as HTMLElement\s*\)/);
  assert.doesNotMatch(topBarSource, /topBarControlDragElement = event\.currentTarget as HTMLElement/);
});

test('TopBar wires six controls to pointer reorder and renderer persistence', () => {
  assert.deepEqual(DEFAULT_TOP_BAR_CONTROL_ORDER, [...order, 'snip']);
  assert.match(topBarSource, /TOP_BAR_CONTROL_ORDER_STORAGE_KEY/);
  assert.match(topBarSource, /normalizeTopBarControlOrder\(\s*localStorage\.getItem\(TOP_BAR_CONTROL_ORDER_STORAGE_KEY\)\s*\)/);
  assert.match(topBarSource, /localStorage\.setItem\(TOP_BAR_CONTROL_ORDER_STORAGE_KEY, JSON\.stringify\(topBarControlOrder\)\)/);
  assert.match(topBarSource, /startTopBarControlPointerDrag\(control\.id, event\)/);
  assert.match(topBarSource, /moveTopBarControlPointerDrag/);
  assert.match(topBarSource, /finishTopBarControlPointerDrag/);
  assert.match(topBarSource, /resolveTopBarControlPointerRelease/);
  assert.match(topBarSource, /data-top-bar-control=/);
  assert.match(topBarSource, /on:dragstart=\{preventTopBarControlNativeDrag\}/);
  assert.doesNotMatch(topBarSource, /data-top-bar-control="(?:time|search)"/);
});

test('TopBar keeps stable popup anchors and all requested control renderers', () => {
  for (const id of [...order, 'snip']) {
    assert.match(topBarSource, new RegExp(`id: '${id}'`));
  }
  assert.match(topBarSource, /bind:this=\{terminalControl\}/);
  assert.match(topBarSource, /bind:this=\{commandControl\}/);
  assert.match(topBarSource, /bind:this=\{trayControl\}/);
  assert.match(topBarSource, /bind:this=\{soundControl\}/);
  assert.match(topBarSource, /<TopBarMicControl bind:this=\{micControl\} \/>/);
});
