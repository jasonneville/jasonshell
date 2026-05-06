import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const stackCss = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');
const traySource = readFileSync(new URL('../src/components/TrayPanelSurface.svelte', import.meta.url), 'utf8');
const trayCss = readFileSync(new URL('../src/components/TrayPanelSurface.css', import.meta.url), 'utf8');
const processSource = readFileSync(new URL('../src/components/ProcessManagerSurface.svelte', import.meta.url), 'utf8');
const processCss = readFileSync(new URL('../src/components/ProcessManagerSurface.css', import.meta.url), 'utf8');
const searchCss = readFileSync(new URL('../src/components/SearchPanelSurface.css', import.meta.url), 'utf8');

function cssRule(source, selector) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = source.match(new RegExp(`${escaped}\\s*\\{(?<body>[\\s\\S]*?)\\}`));
  assert.ok(match?.groups?.body, `Missing CSS rule for ${selector}`);
  return match.groups.body;
}

function assertCssDeclaration(rule, property, valuePattern, message) {
  assert.match(
    rule,
    new RegExp(`${property}\\s*:\\s*${valuePattern}\\s*;`),
    message ?? `Expected ${property}: ${valuePattern}`
  );
}

test('stack context menu and submenu clamp to viewport with internal vertical scroll', () => {
  const contextMenuRule = cssRule(stackCss, '.context-menu');
  const submenuPanelRule = cssRule(stackCss, '.context-submenu-panel');

  assertCssDeclaration(
    contextMenuRule,
    'max-height',
    '(?:min\\(|calc\\(|var\\(--stack-context-menu-max-height\\)|[0-9.]+(?:vh|px|rem))[^;]*',
    'Root context menu must clamp height to available viewport space.'
  );
  assertCssDeclaration(
    contextMenuRule,
    'overflow-y',
    'auto',
    'Root context menu must scroll internally when opened near viewport bottom.'
  );
  assertCssDeclaration(
    submenuPanelRule,
    'max-height',
    '(?:min\\(|calc\\(|var\\(--stack-context-submenu-max-height\\)|[0-9.]+(?:vh|px|rem))[^;]*',
    'Context submenu must clamp height to available viewport space.'
  );
  assertCssDeclaration(
    submenuPanelRule,
    'overflow-y',
    'auto',
    'Context submenu must scroll internally instead of clipping bottom actions.'
  );
});

test('tray panel keeps root fixed and gives icon content an internal scroller', () => {
  assert.match(traySource, /class="tray-grid"/, 'Tray icon grid must remain present.');
  assert.match(
    traySource,
    /Loading notification icons[\s\S]*No notification icons are currently available[\s\S]*class="tray-content"[\s\S]*class="tray-grid"/,
    'Tray loading/error/empty states must stay outside the icon-list scroller.'
  );

  const rootRule = cssRule(trayCss, '.tray-panel');
  assertCssDeclaration(rootRule, 'height', '100%', 'Tray panel root must fill fixed Tauri popup height.');
  assertCssDeclaration(rootRule, 'overflow', 'hidden', 'Tray panel root must not become the page scroller.');

  const hasDedicatedScroller = /class="[^"]*tray-(?:content|body|scroller)[^"]*"[\s\S]*class="tray-grid"/.test(traySource);
  const scrollerRule = hasDedicatedScroller
    ? trayCss.match(/\.tray-(?:content|body|scroller)[^{]*\{(?<body>[\s\S]*?)\}/)?.groups?.body
    : cssRule(trayCss, '.tray-grid');

  assert.ok(scrollerRule, 'Tray grid or wrapping content region must have CSS.');
  assertCssDeclaration(scrollerRule, 'min-height', '0', 'Tray scroll region needs min-height: 0 inside fixed popup.');
  assert.match(scrollerRule, /overflow(?:-y)?\s*:\s*auto\s*;/, 'Tray scroll region needs internal overflow:auto.');
  assert.match(scrollerRule, /flex\s*:\s*1\s+1\s+auto\s*;/, 'Tray icon scroller must take remaining panel height.');
});

test('process manager keeps header and body in one horizontal scroll context', () => {
  const headerIndex = processSource.indexOf('class="process-row process-row-head"');
  assert.notEqual(headerIndex, -1, 'Process manager header row must exist.');

  const bodyIndex = processSource.indexOf('class="process-table-body"');
  const sharedScrollClass = /class="[^"]*process-(?:table-)?(?:scroll|viewport|scroller)[^"]*"/.test(processSource);
  const headerInsideBodyScroller = bodyIndex !== -1 && headerIndex > bodyIndex;
  const sharedScrollerWrapsHeaderAndBody = /class="process-table-scroll"[\s\S]*class="process-row process-row-head"[\s\S]*class="process-table-body"/.test(processSource);
  const sharedScrollerIsFocusable = /class="process-table-scroll"[^>]*role="grid"[^>]*tabindex="0"/.test(processSource);
  assert.ok(
    (sharedScrollClass && sharedScrollerWrapsHeaderAndBody) || headerInsideBodyScroller,
    'Process header row must live inside the same horizontal scroller as process rows, or an explicit shared scroller must wrap both.'
  );
  assert.ok(sharedScrollerIsFocusable, 'Shared Process Manager scroller must be keyboard-focusable for horizontal scroll reachability.');

  const tableRule = cssRule(processCss, '.process-table');
  const scrollRule = cssRule(processCss, '.process-table-scroll');
  const contentRule = cssRule(processCss, '.process-table-content');
  const headerRule = cssRule(processCss, '.process-row-head');
  const bodyRule = cssRule(processCss, '.process-table-body');
  const headerIsSticky = /position\s*:\s*sticky\s*;/.test(headerRule) && /top\s*:\s*0\s*;/.test(headerRule);
  const bodyOwnsOnlyVerticalOverflow = /overflow-y\s*:\s*auto\s*;/.test(bodyRule)
    && !/overflow\s*:\s*auto\s*;/.test(bodyRule)
    && !/overflow-x\s*:\s*auto\s*;/.test(bodyRule);

  assert.ok(
    headerIsSticky || bodyOwnsOnlyVerticalOverflow,
    'Avoid split horizontal scrolling: use a sticky header in the shared scroller or keep horizontal overflow on a shared wrapper.'
  );
  assertCssDeclaration(tableRule, 'overflow', 'hidden', 'Process table root must not become a second horizontal scroller.');
  assert.match(scrollRule, /overflow\s*:\s*auto\s*;/, 'Shared Process Manager scroller must own both horizontal and vertical overflow.');
  assertCssDeclaration(
    contentRule,
    'min-width',
    '(?:4[5-9]|[5-9][0-9])(?:rem|px)',
    'Process table content must be wider than the default 720px popup so rightmost status/action columns are reachable by horizontal scroll.'
  );
  assert.doesNotMatch(bodyRule, /overflow(?:-x)?\s*:\s*auto\s*;/, 'Process table body must not own an independent horizontal scroller.');
});

test('search panel uses flex column with internal result overflow and no approximate result max-height', () => {
  const rootRule = cssRule(searchCss, '.search-panel');
  const headerRule = cssRule(searchCss, '.search-panel-header');
  const resultListRule = cssRule(searchCss, '.result-list');
  const emptyStateRule = cssRule(searchCss, '.empty-state');

  assertCssDeclaration(rootRule, 'display', 'flex', 'Search panel root must be a flex column container.');
  assertCssDeclaration(rootRule, 'flex-direction', 'column', 'Search panel root must stack header above result scroller.');
  assertCssDeclaration(rootRule, 'min-height', '0', 'Search panel root needs min-height: 0 for constrained popup sizing.');
  assertCssDeclaration(rootRule, 'overflow', 'hidden', 'Search panel root must keep scrolling inside content regions.');

  assertCssDeclaration(headerRule, 'flex', '0\\s+0\\s+auto', 'Search header/input must stay outside the result scroller.');
  assertCssDeclaration(resultListRule, 'flex', '1\\s+1\\s+auto', 'Search results must take remaining height inside the flex column.');
  assertCssDeclaration(resultListRule, 'min-height', '0', 'Search results need min-height: 0 to shrink before scrolling.');
  assertCssDeclaration(resultListRule, 'overflow-y', 'auto', 'Search results must scroll internally.');
  assert.doesNotMatch(
    resultListRule,
    /max-height\s*:\s*calc\(/,
    'Search results must not use approximate max-height: calc(...) sizing.'
  );
  assertCssDeclaration(emptyStateRule, 'flex', '1\\s+1\\s+auto', 'Search empty/status state should use the same remaining-height lane.');
  assertCssDeclaration(emptyStateRule, 'min-height', '0', 'Search empty/status state must not force panel overflow.');
});
