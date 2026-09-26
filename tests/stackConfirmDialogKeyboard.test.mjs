import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('../src/components/StackConfirmDialog.svelte', import.meta.url), 'utf8');
const keydown = source.slice(source.indexOf('function handleKeydown('), source.indexOf('\n</script>'));

test('confirm dialog arrows move between enabled actions without activating either action', () => {
  assert.match(keydown, /event\.key\s*===?\s*['"]ArrowRight['"]/, 'Right arrow handled by dialog');
  assert.match(keydown, /event\.key\s*===?\s*['"]ArrowLeft['"]/, 'Left arrow handled by dialog');
  assert.match(keydown, /event\.preventDefault\(\)/, 'arrows prevent default');
  assert.match(keydown, /cancelButton/, 'Cancel action participates in arrow navigation');
  assert.match(keydown, /confirmButton/, 'Confirm action participates in arrow navigation');
  assert.match(keydown, /\.focus\(\)/, 'arrows change focus, not activation');
  assert.match(keydown, /busy|\.disabled/, 'disabled or busy actions cannot receive arrow focus');
  assert.match(source, /on:click=\{onConfirm\}/, 'Enter on focused confirm retains native button click');
  assert.match(source, /on:click=\{cancel\}/, 'Cancel retains native button click');
  assert.match(keydown, /event\.key\s*===?\s*['"]Escape['"]/, 'Escape path remains');
  assert.match(keydown, /event\.key\s*!==?\s*['"]Tab['"]/, 'Tab trapping remains');
  assert.match(source, /origin\?\.isConnected\s*&&\s*origin\.focus\(\)/, 'unmount restores origin focus');
});
