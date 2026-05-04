import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { reorderPinnedFolders } from '../dist-tests/lib/topBarPins.js';

const alpha = { id: 'a', name: 'Work', path: 'C:\\Alpha\\Work' };
const beta = { id: 'b', name: 'Work', path: 'D:\\Beta\\Work' };
const gamma = { id: 'c', name: 'Gamma', path: 'C:\\Gamma' };

test('reorders first pinned folder to the end', () => {
  assert.deepEqual(reorderPinnedFolders([alpha, beta, gamma], alpha.path, 2), [beta, gamma, alpha]);
});

test('reorders last pinned folder to the beginning', () => {
  assert.deepEqual(reorderPinnedFolders([alpha, beta, gamma], gamma.path, 0), [gamma, alpha, beta]);
});

test('reorders a middle pinned folder by path not display name', () => {
  assert.deepEqual(reorderPinnedFolders([alpha, beta, gamma], beta.path, 0), [beta, alpha, gamma]);
});

test('keeps same array for same slot and missing source', () => {
  const pins = [alpha, beta, gamma];
  assert.equal(reorderPinnedFolders(pins, beta.path, 1), pins);
  assert.equal(reorderPinnedFolders(pins, 'C:\\Missing', 1), pins);
});

test('TopBar pinned folders wire drag reorder through existing persistence path', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  assert.match(source, /reorderPinnedFolders/);
  assert.match(source, /onDragStart=\{\(event\) => handlePinDragStart\(event, pin\)\}/);
  assert.match(source, /onDragOver=\{\(event\) => handlePinDragOver\(event, pin\)\}/);
  assert.match(source, /onDrop=\{\(event\) => void handlePinDrop\(event, pin\)\}/);
  assert.match(source, /await applyStackPins\(await reorderStackPins\(nextPins\.map\(\(pin\) => pin\.path\)\)\)/);
});

test('TopBar keeps click and context menu behavior below drag threshold', () => {
  const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
  assert.match(source, /const PIN_REORDER_DRAG_THRESHOLD_PX = \d+/);
  assert.match(source, /let suppressNextPinClickPath: string \| null = null/);
  assert.match(source, /if \(suppressNextPinClickPath === pin\.path\) \{[\s\S]*event\.preventDefault\(\);[\s\S]*return;/);
  assert.match(source, /openStackPath\(pin\.path, event\.currentTarget\)/);
  assert.match(source, /function handlePinContextMenu\(event: MouseEvent, pin: StackPin\)/);
});
