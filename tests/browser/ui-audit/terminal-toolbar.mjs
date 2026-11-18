import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';
import { hasFocusPaint } from '../close-parity/focus-paint.mjs';

const output = process.argv[2] ?? 'test-results/ui-audit-terminal-toolbar';
await mkdir(output, { recursive: true });
const rows = [];
const menuOnly = process.argv.includes('--menu-only');
for (const width of menuOnly ? [] : [860, 480, 360, 320]) {
  const page = await connectPage('http://127.0.0.1:9295', 'about:blank');
  try {
    await page.send('Emulation.setDeviceMetricsOverride', { width, height: 520, deviceScaleFactor: 1, mobile: false });
    await page.send('Page.navigate', { url: 'http://127.0.0.1:5198/tests/browser/close-parity/index.html?panel=terminal' });
    await page.send('Page.bringToFront');
    await page.wait('window.openFixtureTerminal');
    await page.evaluate('window.openFixtureTerminal()');
    await page.wait('document.querySelector(".xterm-helper-textarea")');
    await page.evaluate(`document.querySelector('.terminal-toolbar button:not(:disabled)').focus()`);
    const order = [];
    for (let i = 0; i < 30; i++) {
      if (i) await press(page, 'Tab');
      const state = await page.evaluate(`(async () => {
        const n = document.activeElement, toolbar = n.closest('.terminal-toolbar');
        if (!toolbar) return null;
        getComputedStyle(n).boxShadow;
        await Promise.allSettled(n.getAnimations().filter(animation => Number.isFinite(animation.effect?.getComputedTiming().endTime)).map(animation => animation.finished));
        const c = getComputedStyle(n);
        const r = n.getBoundingClientRect(); let left = 0, right = innerWidth;
        for (let a = n.parentElement; a; a = a.parentElement) if (/hidden|clip|auto|scroll/.test(getComputedStyle(a).overflowX)) {
          const b = a.getBoundingClientRect(); left = Math.max(left,b.left+a.clientLeft); right = Math.min(right,b.left+a.clientLeft+a.clientWidth);
        }
        return { label:n.getAttribute('aria-label') || n.title || n.textContent.trim(), left:r.left, right:r.right, clipLeft:left, clipRight:right, fits:r.left>=left-0.01 && r.right<=right+0.01 && r.top>=0 && r.bottom<=innerHeight, focus:n.matches(':focus-visible'), ring:[c.outlineStyle,c.outlineWidth,c.outlineColor,c.boxShadow] };
      })()`);
      if (!state || order.some(item => item.label === state.label)) continue;
      order.push({ ...state, painted: hasFocusPaint(state.ring) });
      if (!state.fits || state.label === 'Open terminal actions menu') {
        const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
        await writeFile(`${output}/${width}-${i}.png`, Buffer.from(data, 'base64'));
      }
    }
    rows.push({ width, order });
  } finally { await page.close(); }
}
const page = await connectPage('http://127.0.0.1:9295', 'about:blank');
let menu;
try {
  await page.send('Emulation.setDeviceMetricsOverride', { width: 320, height: 200, deviceScaleFactor: 1, mobile: false });
  await page.send('Page.navigate', { url: 'http://127.0.0.1:5198/tests/browser/close-parity/index.html?panel=terminal' });
  await page.send('Page.bringToFront');
  await page.wait('window.openFixtureTerminal');
  await page.evaluate('window.openFixtureTerminal()');
  await page.wait('document.querySelector(".xterm-helper-textarea")');
  const trigger = `document.querySelector('[aria-label="Open terminal actions menu"]')`;
  const popup = `document.querySelector('.terminal-toolbar-menu:not(.terminal-recent-menu)')`;
  const captureScroll = owner => page.evaluate(`(() => {
      const nodes = new Set();
      for (let node = ${owner}; node; node = node.parentElement) nodes.add(node);
      const viewport = document.querySelector('.xterm-helper-textarea')?.closest('.xterm')?.querySelector('.xterm-viewport');
      if (!viewport) throw new Error('Active xterm viewport missing');
      nodes.add(viewport);
      window.auditEscapeScrollNodes = [...nodes];
      return { nodes: ['window', ...window.auditEscapeScrollNodes.map(node => node.className || node.tagName)], offsets: [[scrollX, scrollY], ...window.auditEscapeScrollNodes.map(node => [node.scrollLeft, node.scrollTop])] };
    })()`);
  const escapeFrom = async (owner, dropdown, before) => {
    const offsets = `[[scrollX, scrollY], ...window.auditEscapeScrollNodes.map(node => node.isConnected ? [node.scrollLeft, node.scrollTop] : null)]`;
    const opened = await page.evaluate(offsets);
    await page.evaluate('window.closeParityLedger.length = 0');
    await press(page, 'Escape');
    return page.evaluate(`(() => {
      const before = ${JSON.stringify(before)};
      const after = ${offsets};
      const trigger = ${owner}, rect = trigger.getBoundingClientRect();
      let left=0, top=0, right=innerWidth, bottom=innerHeight;
      for (let node=trigger.parentElement; node; node=node.parentElement) {
        const s=getComputedStyle(node), r=node.getBoundingClientRect();
        if (/hidden|clip|auto|scroll/.test(s.overflowX)) { left=Math.max(left,r.left+node.clientLeft); right=Math.min(right,r.left+node.clientLeft+node.clientWidth); }
        if (/hidden|clip|auto|scroll/.test(s.overflowY)) { top=Math.max(top,r.top+node.clientTop); bottom=Math.min(bottom,r.top+node.clientTop+node.clientHeight); }
      }
      return { open:Boolean(${dropdown}), triggerFocused:trigger.isConnected && trigger===document.activeElement,
        triggerFits:rect.width>0 && rect.height>0 && rect.left>=left-.01 && rect.top>=top-.01 && rect.right<=right+.01 && rect.bottom<=bottom+.01,
        scroll: { nodes: before.nodes, before: before.offsets, opened: ${JSON.stringify(opened)}, after, unchanged:JSON.stringify(after)===JSON.stringify(before.offsets) },
        sessionIPC:window.closeParityLedger.filter(command => /^(hide_|start_|stop_|write_)/.test(command)) };
    })()`);
  };
  await page.evaluate(`${trigger}.focus()`);
  const beforeActions = await captureScroll(trigger);
  await press(page, ' ');
  await page.wait(popup);
  const eligible = `Array.from(${popup}.querySelectorAll('button:not(:disabled)')).at(-1)`;
  for (let i = 0; i < 25 && !await page.evaluate(`${eligible} === document.activeElement`); i++) await press(page, 'Tab');
  menu = await page.evaluate(`(() => {
    const n = ${eligible}, p = ${popup}, r = n.getBoundingClientRect();
    let left = 0, right = innerWidth, top = 0, bottom = innerHeight;
    for (let a = n.parentElement; a; a = a.parentElement) {
      const b = a.getBoundingClientRect(), s = getComputedStyle(a);
      if (/hidden|clip|auto|scroll/.test(s.overflowX)) { left = Math.max(left,b.left+a.clientLeft); right = Math.min(right,b.left+a.clientLeft+a.clientWidth); }
      if (/hidden|clip|auto|scroll/.test(s.overflowY)) { top = Math.max(top,b.top+a.clientTop); bottom = Math.min(bottom,b.top+a.clientTop+a.clientHeight); }
    }
    return { label:n.textContent.trim(), focused: n === document.activeElement && n.matches(':focus-visible'), row:r.toJSON(), clip:[left,top,right,bottom], popup:p.getBoundingClientRect().toJSON(), scrollTop:p.scrollTop,
      fits:r.left>=left-.01 && r.top>=top-.01 && r.right<=right+.01 && r.bottom<=bottom+.01 };
  })()`);
  const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
  await writeFile(`${output}/menu-320x200.png`, Buffer.from(data, 'base64'));
  menu.escape = await escapeFrom(trigger, popup, beforeActions);
  menu.additional = [];
  for (const [kind, owner, dropdown, focusPopup] of [
    ['actions-trigger', trigger, popup, false],
    ['recent-trigger', `document.querySelector('[aria-label="Open recent terminal commands"]')`, `document.querySelector('.terminal-recent-menu')`, false],
    ['recent-popup', `document.querySelector('[aria-label="Open recent terminal commands"]')`, `document.querySelector('.terminal-recent-menu')`, true]
  ]) {
    await page.evaluate(`${owner}.focus()`);
    const before = await captureScroll(owner);
    await press(page, ' ');
    await page.wait(dropdown);
    const source = focusPopup ? `${dropdown}.querySelector('button[role="menuitem"]:not(:disabled)')` : owner;
    if (focusPopup) {
      await page.wait(source);
      for (let i = 0; i < 12 && !await page.evaluate(`${source} === document.activeElement`); i++) await press(page, 'Tab');
    }
    await page.wait(`${source} === document.activeElement`);
    const role = await page.evaluate(`document.activeElement.getAttribute('role')`);
    const label = await page.evaluate(`document.activeElement.getAttribute('aria-label') || document.activeElement.textContent.trim()`);
    menu.additional.push({ kind, role, label, ...await escapeFrom(owner, dropdown, before) });
  }
} catch (error) { menu = { ...menu, setup:String(error).split('\n')[0] }; }
finally { await page.close(); }
await writeFile(`${output}/report.json`, JSON.stringify({ rows, menu },null,2));
console.log(menu);
const escapedLocally = state => !state.open && state.triggerFocused && state.triggerFits && state.scroll.unchanged && !state.sessionIPC.length;
if (menu.setup || !menu.focused || !menu.fits || !menu.additional.some(state => state.kind === 'recent-popup' && state.role === 'menuitem' && state.label) || ![menu.escape, ...menu.additional].every(escapedLocally) || rows.some(row => row.order.length !== 6 || row.order.some(item => !item.fits || !item.focus || !item.painted))) process.exitCode = 1;
