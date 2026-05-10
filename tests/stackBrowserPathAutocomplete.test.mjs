import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import {
  getNextStackPathCompletionCycleIndex,
  getStackPathAutocompleteQuery,
  getStackPathInlineCompletion
} from '../dist-tests/lib/stackPopupViewModel.js';

const surface = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const styles = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');
const api = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');
const commands = readFileSync(new URL('../src/ipc/commands.ts', import.meta.url), 'utf8');
const pathKeydownSource = surface.slice(surface.indexOf('function handlePathKeydown'), surface.indexOf('async function refreshPathSuggestions'));

test('parses Windows path autocomplete parent and segment at caret', () => {
  assert.deepEqual(getStackPathAutocompleteQuery('C:\\dev\\ja', 9), {
    parentPath: 'C:\\dev',
    segment: 'ja'
  });
  assert.deepEqual(getStackPathAutocompleteQuery('C:\\', 3), {
    parentPath: 'C:\\',
    segment: ''
  });
  assert.deepEqual(getStackPathAutocompleteQuery('C:\\dev\\', 7), {
    parentPath: 'C:\\dev',
    segment: ''
  });
});

test('parses UNC paths and rejects relative or empty autocomplete inputs', () => {
  assert.deepEqual(getStackPathAutocompleteQuery('\\\\server\\share\\pro', 18), {
    parentPath: '\\\\server\\share',
    segment: 'pro'
  });
  assert.equal(getStackPathAutocompleteQuery('', 0), null);
  assert.equal(getStackPathAutocompleteQuery('dev\\ja', 6), null);
});

test('path autocomplete has IPC wrapper and stale-aware UI wiring', () => {
  assert.match(commands, /suggestStackPaths: 'suggest_stack_paths'/);
  assert.match(api, /suggestStackPaths\(request: StackPathSuggestionRequest\): Promise<StackPathSuggestion\[]>/);
  assert.match(surface, /let pathSuggestionRequestSeq = 0/);
  assert.match(surface, /suggestStackPaths\(/);
  assert.match(surface, /pathInlineCompletion/);
  assert.match(surface, /class="path-input-shell"/);
  assert.match(surface, /class="path-inline-ghost"/);
  assert.match(surface, /class="path-inline-typed"/);
  assert.match(surface, /class="path-inline-completion"/);
  assert.match(surface, /on:focus=\{\(event\) => \{/);
  assert.match(surface, /refreshPathSuggestions\(event\.currentTarget\)/);
  assert.doesNotMatch(surface, /role="listbox"/);
  assert.doesNotMatch(pathKeydownSource, /ArrowDown|ArrowUp/);
  assert.doesNotMatch(surface, /openFolder\(suggestion\.path\)/);
  assert.doesNotMatch(styles, /justify-self:\s*end/);
  assert.match(styles, /\.path-inline-typed[\s\S]*visibility:\s*hidden/);
  assert.match(styles, /\.path-inline-ghost[\s\S]*pointer-events:\s*none/);
  assert.match(pathKeydownSource, /!event\.shiftKey/);
  assert.match(surface, /clearPathSuggestions/);
  assert.match(pathKeydownSource, /ArrowRight/);
  assert.match(pathKeydownSource, /Tab/);
  assert.match(surface, /focusPathInput/);
});

test('RightArrow accepting inline path completion keeps focus and caret for chained completion', () => {
  const acceptSource = surface.slice(
    surface.indexOf('function acceptInlinePathCompletion'),
    surface.indexOf('async function focusPathInput')
  );
  assert.match(pathKeydownSource, /event\.key === 'ArrowRight'/);
  assert.match(acceptSource, /pathDraft = pathInlineCompletion\.commitPath/);
  assert.match(acceptSource, /const committedPath = pathInlineCompletion\.commitPath/);
  assert.match(acceptSource, /void focusPathInput\(committedPath\.length\)/);
  assert.doesNotMatch(acceptSource, /openFolder\(committedPath\)/);
});

test('RightArrow accept immediately refreshes suggestions for committed path', () => {
  const acceptSource = surface.slice(
    surface.indexOf('function acceptInlinePathCompletion'),
    surface.indexOf('async function focusPathInput')
  );
  assert.match(acceptSource, /void refreshPathSuggestionsForValue\(committedPath,\s*committedPath\.length\)/);
  assert.match(surface, /async function refreshPathSuggestionsForValue\(value: string,\s*caret: number\)/);
});

test('Tab cycles through matching directory completions without opening the folder', () => {
  const tabBranchMatch = pathKeydownSource.match(/if \(event\.key === 'Tab' && !event\.shiftKey[\s\S]*?return;\s*\}/);
  assert.ok(tabBranchMatch, 'Tab keydown branch is present');
  assert.match(tabBranchMatch[0], /cyclePathCompletion\(\)/);
  assert.doesNotMatch(tabBranchMatch[0], /acceptInlinePathCompletion\(\)/);
  const cycleSource = surface.slice(
    surface.indexOf('function cyclePathCompletion'),
    surface.indexOf('function acceptInlinePathCompletion')
  );
  assert.match(surface, /let pathCompletionCycleIndex = -1/);
  assert.match(cycleSource, /pathSuggestions\.length/);
  assert.match(cycleSource, /getNextStackPathCompletionCycleIndex\(pathDraft, pathSuggestions, pathCompletionCycleIndex\)/);
  assert.match(cycleSource, /const committedPath = suggestion\.path/);
  assert.match(cycleSource, /pathDraft = committedPath/);
  assert.match(cycleSource, /void focusPathInput\(committedPath\.length\)/);
  assert.doesNotMatch(cycleSource, /openFolder\(committedPath\)/);
});

test('Tab cycle skips an exact prefix directory before walking sibling completions', () => {
  const suggestions = [
    { name: 'jasonshell', path: 'C:\\dev\\jasonshell' },
    { name: 'jasonshell-cli-improvements', path: 'C:\\dev\\jasonshell-cli-improvements' },
    { name: 'jasonshell-embedded-shell', path: 'C:\\dev\\jasonshell-embedded-shell' }
  ];
  const firstTab = getNextStackPathCompletionCycleIndex('C:/DEV/jasonshell', suggestions, -1);
  assert.equal(firstTab, 1);
  const secondTab = getNextStackPathCompletionCycleIndex(suggestions[firstTab].path, suggestions, firstTab);
  assert.equal(secondTab, 2);
  assert.equal(getNextStackPathCompletionCycleIndex(suggestions[2].path, suggestions, 2), 0);
  assert.equal(getNextStackPathCompletionCycleIndex('C:\\dev\\jasonshell-c', suggestions, -1), 0);
  assert.equal(getNextStackPathCompletionCycleIndex('C:\\dev\\jasonshell', [], -1), -1);
});

test('path blur timeout cannot reset draft during chained RightArrow accept flow', () => {
  assert.match(surface, /let pathBlurResetTimer: number \| null = null/);
  const acceptSource = surface.slice(
    surface.indexOf('function acceptInlinePathCompletion'),
    surface.indexOf('async function focusPathInput')
  );
  assert.match(acceptSource, /cancelPathBlurReset\(\)/);
  assert.match(surface, /function schedulePathBlurReset\(\)/);
  assert.match(surface, /function cancelPathBlurReset\(\)/);
  assert.match(surface, /on:blur=\{\(\) => \{\s*pathInputFocused = false;\s*schedulePathBlurReset\(\);/);
});

test('builds inline path completion suffix for matching suggestion', () => {
  assert.deepEqual(
    getStackPathInlineCompletion('C:\\dev\\ja', { name: 'jasonshell', path: 'C:\\dev\\jasonshell' }),
    { displayText: 'sonshell', commitPath: 'C:\\dev\\jasonshell' }
  );
  assert.deepEqual(
    getStackPathInlineCompletion('C:/dev/ja', { name: 'jasonshell', path: 'C:\\dev\\jasonshell' }),
    { displayText: 'sonshell', commitPath: 'C:\\dev\\jasonshell' }
  );
});

test('omits inline completion when suggestion does not extend typed segment', () => {
  assert.equal(getStackPathInlineCompletion('C:\\dev\\other', { name: 'jasonshell', path: 'C:\\dev\\jasonshell' }), null);
  assert.equal(getStackPathInlineCompletion('D:\\dev\\ja', { name: 'jasonshell', path: 'C:\\dev\\jasonshell' }), null);
  assert.equal(getStackPathInlineCompletion('C:\\dev\\jasonshell', { name: 'jasonshell', path: 'C:\\dev\\jasonshell' }), null);
  assert.equal(getStackPathInlineCompletion('', { name: 'jasonshell', path: 'C:\\dev\\jasonshell' }), null);
});
