import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const stackPopupSource = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const stackPopupCss = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');

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

test('stack browser has an accessible token-styled close button wired to the surface close path', () => {
  const closeBody = functionBody(stackPopupSource, 'closeStackPopupFromSurface');
  const closeButtonRule = cssRule(stackPopupCss, '.stack-browser-close-button');
  const toolbarRule = cssRule(stackPopupCss, '.stack-toolbar');

  assert.match(stackPopupSource, /import MaterialSymbolIcon from '\.\/icons\/MaterialSymbolIcon\.svelte'/);
  assert.match(stackPopupSource, /class="stack-browser-close-button"/);
  assert.match(stackPopupSource, /ariaLabel="Close stack browser"/);
  assert.match(stackPopupSource, /<MaterialSymbolIcon name="close" \/>/);
  assert.match(stackPopupSource, /onClick=\{\(\) => void closeStackPopupFromSurface\(\)\}/);
  // FE-34: stopCurrentStackTerminal was a no-op (the pane was never mounted) and was removed.
  assert.doesNotMatch(closeBody, /stopCurrentStackTerminal/);
  // FE-34: stackBrowserViewMode could only become 'terminal' via the unreachable switchStackBrowserView, so the reset was dead and removed.
  assert.match(closeBody, /closeGitStatusPopup\(\)/); // a still-loading Git pane must not mount into a hidden popup
  assert.match(closeBody, /await hideStackPopup\(\)/);

  assert.match(toolbarRule, /padding-right:\s*3rem/);
  assert.match(closeButtonRule, /position:\s*absolute/);
  assert.match(closeButtonRule, /top:\s*0\.42rem/);
  assert.match(closeButtonRule, /right:\s*0\.42rem/);
  assert.match(closeButtonRule, /background:\s*none/);
  assert.match(closeButtonRule, /color:\s*var\(--js-color-text\)/);
  const hoverAndFocusRule = cssRule(stackPopupCss, '.stack-browser-close-button:focus-visible');
  assert.match(hoverAndFocusRule, /background:\s*var\(--js-color-accent-soft\)/);
  assert.match(hoverAndFocusRule, /border-color:\s*var\(--js-color-accent-border\)/);
  assert.match(closeButtonRule, /border-radius:\s*var\(--js-radius-xs\)/);
  assert.doesNotMatch(closeButtonRule, /border-radius:\s*999px/);
  assert.match(closeButtonRule, /min-width:\s*2\.1rem/);
});
