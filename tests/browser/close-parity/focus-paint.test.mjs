import assert from 'node:assert/strict';
import { test } from 'node:test';
import { hasFocusPaint } from './focus-paint.mjs';

test('rejects invisible computed focus styles', () => {
  assert.equal(hasFocusPaint(['solid', '0px', 'rgb(255, 255, 255)', 'none']), false);
  assert.equal(hasFocusPaint(['solid', '2px', 'rgba(255, 255, 255, 0)', 'none']), false);
  assert.equal(hasFocusPaint(['none', '0px', 'rgb(0, 0, 0)', 'rgba(0, 0, 0, 0) 0px 0px 0px 0px']), false);
  assert.equal(hasFocusPaint(['none', '0px', 'rgb(0, 0, 0)', 'rgb(255, 255, 255) 0px 0px 0px 0px']), false);
  assert.equal(hasFocusPaint(['none', '0px', 'rgb(0, 0, 0)', 'rgb(255 255 255 / 0%) 0px 0px 0px 2px']), false);
});

test('accepts the outline and layered shadow patterns', () => {
  assert.equal(hasFocusPaint(['solid', '2px', 'rgb(255, 255, 255)', 'none']), true);
  assert.equal(hasFocusPaint(['none', '0px', 'rgb(0, 0, 0)', 'rgba(0, 0, 0, 0) 0px 0px 0px 3px, rgba(255, 255, 255, 0.6) 0px 0px 0px 2px']), true);
});
