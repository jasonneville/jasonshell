import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

import {
  positionContextMenuInViewport,
  positionScrollableContextMenuInViewport
} from '../dist-tests/lib/contextMenuPosition.js';

const stackPopupSource = readFileSync(new URL('../src/components/StackPopupSurface.svelte', import.meta.url), 'utf8');
const stackPopupCss = readFileSync(new URL('../src/components/StackPopupSurface.css', import.meta.url), 'utf8');

function cssRule(source, selector) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = source.match(new RegExp(`${escaped}\\s*\\{(?<body>[\\s\\S]*?)\\}`));
  assert.ok(match?.groups?.body, `Missing CSS rule for ${selector}`);
  return match.groups.body;
}

test('keeps a context menu at the requested point when it fits', () => {
  assert.deepEqual(
    positionContextMenuInViewport(
      { x: 40, y: 50 },
      { width: 120, height: 90 },
      { width: 400, height: 300 }
    ),
    { x: 40, y: 50 }
  );
});

test('flips and clamps a context menu into visible viewport space', () => {
  assert.deepEqual(
    positionContextMenuInViewport(
      { x: 390, y: 290 },
      { width: 120, height: 90 },
      { width: 400, height: 300 }
    ),
    { x: 270, y: 200 }
  );
});

test('uses padding when the menu is larger than the available viewport', () => {
  assert.deepEqual(
    positionContextMenuInViewport(
      { x: 5, y: 5 },
      { width: 500, height: 400 },
      { width: 300, height: 200 },
      10
    ),
    { x: 10, y: 10 }
  );
});

test('returns scrollable placement when menu is taller than the available viewport', () => {
  assert.deepEqual(
    positionScrollableContextMenuInViewport(
      { x: 920, y: 405 },
      { width: 220, height: 720 },
      { width: 980, height: 430 },
      8
    ),
    { x: 700, y: 8, maxHeight: 414 }
  );
});

test('stack popup wires computed context menu max-height CSS variables', () => {
  assert.match(
    stackPopupSource,
    /--stack-context-menu-max-height:\$\{contextMenuMaxHeightCss\(rowMenu\)\}/,
    'Row context menu must receive computed max-height from placement helper.'
  );
  assert.match(
    stackPopupSource,
    /--stack-context-submenu-max-height:\$\{contextSubmenuMaxHeightCss\(rowMenu\)\}/,
    'Row submenu must receive computed max-height from available viewport.'
  );
  assert.doesNotMatch(
    stackPopupSource,
    /contextSubmenuTopCss|--stack-context-submenu-top/,
    'Submenu must stay attached to its trigger instead of fixed-jumping away from pointer path.'
  );
});

test('stack submenu stays attached to trigger and scrolls internally', () => {
  const submenuPanelRule = cssRule(stackPopupCss, '.context-submenu-panel');

  assert.match(
    submenuPanelRule,
    /position\s*:\s*absolute\s*;/,
    'Submenu panel must stay attached to the trigger hover area so pointer travel remains reachable.'
  );
  assert.match(
    submenuPanelRule,
    /top\s*:\s*0\s*;/,
    'Submenu panel top must align with the trigger row.'
  );
  assert.doesNotMatch(
    submenuPanelRule,
    /position\s*:\s*fixed\s*;/,
    'Fixed submenus can detach from the trigger hover path.'
  );
  assert.match(
    submenuPanelRule,
    /max-height\s*:\s*(?:min\()?var\(--stack-context-submenu-max-height[^;]*;/,
    'Submenu panel must clamp height to available viewport.'
  );
  assert.match(
    submenuPanelRule,
    /overflow-y\s*:\s*auto\s*;/,
    'Submenu panel must scroll internally.'
  );
});
