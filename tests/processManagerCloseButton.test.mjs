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

test('process manager has an accessible token-styled close button with header and focus affordances', () => {
  const closeBody = functionBody(processManagerSource, 'requestClose');
  const closeButtonRule = cssRule(processManagerCss, '.process-manager-close-button');
  const headerRule = cssRule(processManagerCss, '.process-manager-header');

  assert.match(processManagerSource, /import MaterialSymbolIcon from '\.\/icons\/MaterialSymbolIcon\.svelte'/);
  assert.match(processManagerSource, /class="process-manager-close-button"/);
  assert.match(processManagerSource, /ariaLabel="Close process manager"/);
  assert.match(processManagerSource, /<MaterialSymbolIcon name="close" \/>/);
  assert.match(processManagerSource, /onClick=\{\(\) => void requestClose\(\)\}/);
  assert.match(closeBody, /closeSurface\(\)/);
  assert.match(closeBody, /await hideProcessManager\(\)/);
  assert.match(processManagerWrapper, /export function hideProcessManager\(\): Promise<void>/);

  assert.match(headerRule, /display:\s*grid/);
  assert.match(headerRule, /grid-template-columns:\s*minmax\(0,\s*1fr\) auto auto/);
  assert.match(headerRule, /grid-template-areas:\s*'title actions close'\s*'status status status'/);
  assert.match(headerRule, /padding:\s*0\.35rem 0\.6rem/);
  assert.match(processManagerCss, /\.process-manager-title\s*\{[^}]*grid-area:\s*title/);
  assert.match(cssRule(processManagerCss, '.process-manager-actions'), /grid-area:\s*actions/);
  assert.match(processManagerCss, /\.process-manager-header \.process-manager-status\s*\{[^}]*grid-area:\s*status/);
  assert.match(closeButtonRule, /grid-area:\s*close/);
  assert.match(closeButtonRule, /display:\s*inline-flex/);
  assert.doesNotMatch(closeButtonRule, /position:\s*absolute|(?:^|;)\s*(?:top|right):/);
  assert.match(closeButtonRule, /background:\s*none/);
  assert.match(closeButtonRule, /color:\s*var\(--js-color-text\)/);
  const hoverRule = cssRule(processManagerCss, '.process-manager-close-button:hover');
  assert.match(hoverRule, /background:\s*var\(--js-color-accent-soft\)/);
  assert.match(hoverRule, /border-color:\s*var\(--js-color-accent-border\)/);
  assert.match(processManagerCss, /\.process-manager-close-button:focus-visible\s*\{[^}]*box-shadow:\s*var\(--js-focus-ring\)/s);
  assert.match(closeButtonRule, /border-radius:\s*var\(--js-radius-xs\)/);
  assert.doesNotMatch(closeButtonRule, /border-radius:\s*999px/);
  assert.match(closeButtonRule, /height:\s*1\.8rem/);
  assert.match(closeButtonRule, /min-width:\s*1\.8rem/);
});
