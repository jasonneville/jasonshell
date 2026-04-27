import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  findAddedPinPath,
  stackPinRevealPath,
  topBarWebviewWindowEventTarget
} from '../dist-tests/topBarPins.js';

const alpha = { path: 'C:\\Alpha' };
const beta = { path: 'C:\\Beta' };
const gamma = { path: 'C:\\Gamma' };

test('does not reveal the last persisted pin during the initial load', () => {
  assert.equal(stackPinRevealPath([], [alpha, beta], null, false), null);
});

test('reveals newly added pins after the initial load', () => {
  assert.equal(stackPinRevealPath([alpha], [alpha, beta], null, true), beta.path);
});

test('prefers an explicitly requested visible pin path', () => {
  assert.equal(stackPinRevealPath([alpha], [alpha, beta], gamma.path, true), gamma.path);
});

test('detects added pins case-insensitively', () => {
  assert.equal(findAddedPinPath([{ path: 'C:\\alpha' }], [{ path: 'C:\\ALPHA' }, beta]), beta.path);
});

test('uses a webview-window scoped event target for top-bar pin updates', () => {
  assert.deepEqual(topBarWebviewWindowEventTarget(), { kind: 'WebviewWindow', label: 'top-bar' });
  assert.deepEqual(topBarWebviewWindowEventTarget('custom-top'), { kind: 'WebviewWindow', label: 'custom-top' });
});
