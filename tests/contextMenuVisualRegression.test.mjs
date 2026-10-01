import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { positionScrollableContextMenuInViewport } from '../dist-tests/lib/contextMenuPosition.js';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const overlay = read('src/components/ContextMenuOverlaySurface.svelte');
const shared = read('src/components/ContextMenu.svelte');
const item = read('src/components/ContextMenuItem.svelte');
const stack = read('src/components/StackPopupSurface.css');
const theme = read('src/app.css');

function rule(source, selector) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = source.match(new RegExp(`${escaped}\\s*\\{([^}]*)\\}`));
  assert.ok(match, `Missing CSS rule ${selector}`);
  return match[1];
}

test('overlay measures intrinsic menu height before first-open placement, even when menu kind changes', () => {
  const handler = overlay.slice(overlay.indexOf('async function positionMenu()'), overlay.indexOf('function dismiss()'));
  assert.match(handler, /await tick\(\)/);
  assert.match(handler, /(?:scrollHeight|offsetHeight|intrinsicHeight|naturalHeight)/,
    'animated getBoundingClientRect height is zero or scaled on the first frame; measure unbounded intrinsic content');
  assert.doesNotMatch(overlay, /maxHeight:\s*0\b/,
    'first render must not constrain the menu to zero before measuring it');
  assert.match(handler, /request\.source[\s\S]*?bounds\.height|request\.source[\s\S]*?menuHeight/,
    'top and bottom placement must depend on the freshly measured menu height');
});

test('top and bottom anchors keep short and oversized menus inside overlay edges', () => {
  const viewport = { width: 360, height: 360 };
  for (const height of [96, 264, 700]) {
    for (const source of ['top-bar', 'bottom-bar']) {
      const y = source === 'bottom-bar' ? Math.max(8, viewport.height - height - 8) : 8;
      const placed = positionScrollableContextMenuInViewport(
        { x: 8, y }, { width: 240, height }, viewport
      );
      assert.ok(placed.y >= 8 && placed.y + Math.min(height, placed.maxHeight) <= viewport.height - 8,
        `${source} height ${height} stays vertically bounded`);
      assert.ok(placed.x >= 8 && placed.x + 240 <= viewport.width - 8);
    }
  }
});

test('shared menu is opaque throughout opening animation, including bars', () => {
  const panel = rule(shared, ':global(.js-context-menu)');
  const background = panel.match(/(?:^|;)\s*background(?:-color)?\s*:\s*([^;]+)/)?.[1]?.trim();
  assert.ok(background, 'menu needs a background');
  const token = background.match(/^var\((--[\w-]+)\)$/)?.[1];
  const colors = token
    ? [...theme.matchAll(new RegExp(`${token}\\s*:\\s*([^;]+);`, 'g'))].map((match) => match[1].trim())
    : [background];
  assert.ok(colors.length, `background token ${token} must resolve in app.css`);
  for (const color of colors) {
    assert.match(color, /^(?:#[0-9a-f]{3,8}|rgb\([^)]*\)|rgba\([^)]*\)|(?:white|black|Canvas))$/i,
      `menu background must resolve to a directly auditable solid color: ${color}`);
    const rgba = color.match(/^rgba\([^)]*,\s*(0?(?:\.\d+)?|1(?:\.0+)?)\s*\)$/i);
    if (rgba) assert.equal(Number(rgba[1]), 1, `menu background must be fully opaque: ${color}`);
    if (color.startsWith('#')) assert.ok(![4, 8].includes(color.length - 1) || parseInt(color.slice(-2), 16) === 255,
      `menu background hex alpha must be opaque: ${color}`);
  }
  assert.doesNotMatch(rule(shared, '@keyframes js-context-menu-in'), /opacity\s*:\s*[0.][0-9]*\s*;/,
    'opacity animation exposes transparent menu background while opening');
});

test('shared menu disables standard and WebKit backdrop filtering', () => {
  const panel = rule(shared, ':global(.js-context-menu)');
  for (const property of ['backdrop-filter', '-webkit-backdrop-filter']) {
    const values = [...panel.matchAll(new RegExp(`(?:^|;)\\s*${property}\\s*:\\s*([^;]+);`, 'g'))];
    assert.ok(values.every((match) => match[1].trim() === 'none'),
      `${property} must be absent or disabled; blur can show the underlying bar`);
  }
});

test('overlay hides each new request until its measured placement commits', () => {
  const open = overlay.slice(overlay.indexOf('const registration = listen<ContextMenuOverlayRequest>'), overlay.indexOf("window.addEventListener('resize'"));
  const positioning = overlay.slice(overlay.indexOf('async function positionMenu()'), overlay.indexOf('function dismiss()'));
  assert.match(open, /request\s*=\s*event\.payload[\s\S]*?positionMenu\(\)/);
  assert.match(positioning, /await tick\(\)/);
  assert.match(overlay, /(?:visibility\s*:\s*hidden|visibility:hidden|opacity\s*:\s*0|opacity:0)/,
    'unmeasured menu must not flash at the provisional 8,8 position');
  assert.match(overlay, /(?:visibility\s*:\s*visible|visibility:visible|opacity\s*:\s*1|opacity:1)/,
    'measured menu must become visible');
  assert.match(open, /(?:ready|visible|positioned|measured)\s*=\s*false/,
    'opening any new menu must reset visibility, including changed kinds');
  assert.match(positioning, /placement\s*=\s*positionScrollableContextMenuInViewport[\s\S]*?(?:ready|visible|positioned|measured)\s*=\s*true/,
    'only the current request becomes visible after the placement is committed');
});

test('overlay focuses only after the visible placement renders and still belongs to the same menu', () => {
  const positioning = overlay.slice(overlay.indexOf('async function positionMenu()'), overlay.indexOf('function dismiss()'));
  assert.match(positioning,
    /positioned\s*=\s*true\s*;\s*await tick\(\)\s*;\s*if\s*\([^)]*request\s*!==\s*currentRequest[^)]*\)\s*return\s*;\s*focusMenuItem\(0\)/,
    'after revealing the menu, wait for DOM commit and reject a replaced/dismissed request before focusing');
  assert.match(positioning.slice(positioning.indexOf('positioned = true')),
    /await tick\(\)[\s\S]*?if\s*\([^)]*!menuElement[^)]*\)\s*return/,
    'menu element must still exist after the asynchronous render boundary');
});

test('icon and placeholder share a centered 16px cell for every row', () => {
  assert.match(item, /class="context-menu-icon"/);
  assert.match(item, /class="context-menu-icon-placeholder"/);
  assert.match(shared, /align-items:\s*center/);
  const iconRule = rule(shared, ':global(.js-context-menu .context-menu-icon),\n  :global(.js-context-menu .context-menu-icon-placeholder)');
  assert.match(iconRule, /height:\s*16px/);
  assert.match(iconRule, /width:\s*16px/);
  assert.match(shared, /(?:\.context-menu-icon[^}]*display:\s*(?:flex|grid)|\.context-menu-icon[^}]*line-height:\s*0)/,
    'SVG inline baseline must not pull icon off the text centerline');
});

test('Stack menu rows override shared row geometry locally without shrinking other menus', () => {
  const selectors = [...stack.matchAll(/([^{}]+)\{[^{}]*min-height:\s*(?:1\.55rem|2[4-8]px)[^{}]*\}/g)]
    .flatMap((match) => match[1].split(',').map((selector) => selector.trim()));
  for (const scope of ['.context-menu-scroll', '.context-submenu-panel']) {
    assert.ok(selectors.some((selector) => selector.includes(scope)
      && selector.includes('.context-menu-item')
      && (selector.match(/\.[\w-]+/g)?.length ?? 0) > 2),
    `${scope} compact row selector must strictly outrank shared two-class selector regardless of stylesheet order`);
  }
  assert.match(stack, /min-height:\s*(?:1\.55rem|2[4-8]px)/,
    'Stack rows remain modestly smaller than shared 30px rows');
  assert.match(shared, /min-height:\s*30px/,
    'non-Stack menu sizing stays unchanged');
});
