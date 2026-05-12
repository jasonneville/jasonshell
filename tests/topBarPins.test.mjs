import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import {
  findAddedPinPath,
  stackPinRevealPath,
  topBarWebviewWindowEventTarget
} from '../dist-tests/lib/topBarPins.js';

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

test('TopBar applyStackPins does not recursively reload pins during hydration', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  const start = source.indexOf('async function applyStackPins');
  assert.notEqual(start, -1, 'TopBar applyStackPins must remain visible to source-level hydration coverage');

  const openBrace = source.indexOf('{', start);
  let depth = 0;
  let end = -1;
  for (let index = openBrace; index < source.length; index += 1) {
    if (source[index] === '{') {
      depth += 1;
    } else if (source[index] === '}') {
      depth -= 1;
      if (depth === 0) {
        end = index;
        break;
      }
    }
  }

  assert.notEqual(end, -1, 'TopBar applyStackPins body must close');
  const body = source.slice(openBrace + 1, end);
  assert.doesNotMatch(
    body,
    /\bloadStackPins\s*\(/,
    'applyStackPins is called by loadStackPins during initial hydration and must not call loadStackPins again'
  );
});
