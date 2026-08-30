import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const processManagerSource = readFileSync(new URL('../src/components/ProcessManagerSurface.svelte', import.meta.url), 'utf8');
const processManagerCss = readFileSync(new URL('../src/components/ProcessManagerSurface.css', import.meta.url), 'utf8');

function functionBody(source, name) {
  const start = source.indexOf(`function ${name}`);
  assert.notEqual(start, -1, `${name} exists`);
  const signatureEnd = source.indexOf(') {', start);
  assert.notEqual(signatureEnd, -1, `${name} signature closes`);
  const braceStart = source.indexOf('{', signatureEnd);
  let depth = 0;
  for (let index = braceStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) return source.slice(braceStart + 1, index);
  }
  assert.fail(`${name} body closes`);
}

function cssRuleContaining(source, selectorFragment) {
  const rule = source
    .split(/\n\}/)
    .find((candidate) => candidate.includes(selectorFragment) && candidate.includes('{'));
  assert.ok(rule, `${selectorFragment} focus rule exists`);
  return rule;
}

test('process manager keeps auto-refresh grid silent and uses dedicated status live region', () => {
  const gridTag = processManagerSource.match(/<div\s+class="process-table-scroll"[\s\S]*?>/)?.[0] ?? '';
  const roleStatusLiveRegions = processManagerSource.match(/role="status" aria-live="polite"/g) ?? [];
  const statusRoleOccurrences = processManagerSource.match(/role="status"/g) ?? [];
  const ariaLiveAttributes = processManagerSource.match(/aria-live=/g) ?? [];

  assert.match(gridTag, /role="grid"/);
  assert.doesNotMatch(gridTag, /aria-live/);
  assert.equal(statusRoleOccurrences.length, 1);
  assert.equal(roleStatusLiveRegions.length, 1);
  assert.equal(ariaLiveAttributes.length, 1);
});

test('process manager auto refresh does not announce every successful timer tick', () => {
  const refreshBody = functionBody(processManagerSource, 'refreshProcesses');
  const timerBody = functionBody(processManagerSource, 'startRefreshTimer');
  const manualRefresh = processManagerSource.match(/<MeltActionButton onClick=\{\(\) => void refreshProcesses\([\s\S]*?\)\}>\s*Refresh/)?.[0] ?? '';
  const killBody = functionBody(processManagerSource, 'killRow');

  assert.match(processManagerSource, /refreshProcesses\(options:\s*\{\s*preserveVolatileOrder\?:\s*boolean;\s*announce\?:\s*boolean\s*\}/);
  assert.match(refreshBody, /if \(options\.announce !== false\) \{/);
  assert.match(timerBody, /refreshProcesses\(\{\s*announce:\s*false\s*\}\)/);
  assert.match(manualRefresh, /announce:\s*true/);
  assert.match(killBody, /refreshProcesses\(\{\s*preserveVolatileOrder:\s*false,\s*announce:\s*false\s*\}\)/);
});

test('process manager filter and grid expose visible focus styles', () => {
  const filterFocusRule = cssRuleContaining(processManagerCss, '.process-filter:focus-within');
  const gridFocusRule = cssRuleContaining(processManagerCss, '.process-table-scroll:focus-visible');
  const buttonFocusRule = cssRuleContaining(processManagerCss, '.process-manager-close-button:focus-visible');

  assert.match(filterFocusRule, /box-shadow:\s*var\(--js-focus-ring\)/);
  assert.match(filterFocusRule, /border-color:\s*var\(--js-color-accent-border\)/);
  assert.match(gridFocusRule, /box-shadow:\s*var\(--js-focus-ring\)/);
  assert.match(gridFocusRule, /border-color:\s*var\(--js-color-accent-border\)/);
  assert.match(buttonFocusRule, /box-shadow:\s*var\(--js-focus-ring\)/);
  assert.match(buttonFocusRule, /\.process-manager-actions button:focus-visible/);
  assert.match(buttonFocusRule, /\.process-row-head button:focus-visible/);
  assert.match(buttonFocusRule, /\.process-group-toggle:focus-visible/);
  assert.match(buttonFocusRule, /\.kill-button:focus-visible/);
  assert.doesNotMatch(processManagerCss, /outline:\s*none/);
});
