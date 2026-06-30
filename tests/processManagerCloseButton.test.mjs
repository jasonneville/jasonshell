import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const processManagerSource = readFileSync(new URL('../src/components/ProcessManagerSurface.svelte', import.meta.url), 'utf8');
const processManagerCss = readFileSync(new URL('../src/components/ProcessManagerSurface.css', import.meta.url), 'utf8');
const processManagerWrapper = readFileSync(new URL('../src/lib/processManager.ts', import.meta.url), 'utf8');

function functionBody(source, name) {
  const start = source.indexOf(`function ${name}`);
  assert.notEqual(start, -1, `${name} exists`);
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let index = braceStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    if (char === '}') depth -= 1;
    if (depth === 0) return source.slice(braceStart + 1, index);
  }
  assert.fail(`${name} body closes`);
}

function cssRule(source, selector) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = source.match(new RegExp(`${escapedSelector}\\s*\\{([\\s\\S]*?)\\n\\}`));
  assert.ok(match, `${selector} rule exists`);
  return match[1];
}

test('process manager has an accessible task-preview-style red X close button', () => {
  const closeBody = functionBody(processManagerSource, 'requestClose');
  const closeButtonRule = cssRule(processManagerCss, '.process-manager-close-button');
  const headerRule = cssRule(processManagerCss, '.process-manager-header');

  assert.match(processManagerSource, /class="process-manager-close-button"/);
  assert.match(processManagerSource, /ariaLabel="Close process manager"/);
  assert.match(processManagerSource, />×<|>✕</);
  assert.match(processManagerSource, /onClick=\{\(\) => void requestClose\(\)\}/);
  assert.match(closeBody, /closeSurface\(\)/);
  assert.match(closeBody, /await hideProcessManager\(\)/);
  assert.match(processManagerWrapper, /export function hideProcessManager\(\): Promise<void>/);

  assert.match(headerRule, /position:\s*relative/);
  assert.match(headerRule, /padding:\s*0\.45rem 3rem 0\.45rem 0\.6rem/);
  assert.match(closeButtonRule, /position:\s*absolute/);
  assert.match(closeButtonRule, /top:\s*0\.42rem/);
  assert.match(closeButtonRule, /right:\s*0\.42rem/);
  assert.match(closeButtonRule, /background:\s*#dc2626/);
  assert.match(closeButtonRule, /border-radius:\s*var\(--js-radius-xs\)/);
  assert.doesNotMatch(closeButtonRule, /border-radius:\s*999px/);
  assert.match(closeButtonRule, /min-width:\s*2\.1rem/);
});
