import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';
import { hasFocusPaint } from '../close-parity/focus-paint.mjs';

const base = process.argv[2] ?? 'http://127.0.0.1:5199';
const output = process.argv[3] ?? 'test-results/ui-audit-menu';
await mkdir(output, { recursive: true });
const rows = [];
const failures = [];
for (const [theme, width, height] of [['dark', 360, 360], ['light', 360, 360], ['translucent', 320, 145]]) {
  const page = await connectPage('http://127.0.0.1:9295', 'about:blank');
  try {
    await page.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
    await page.send('Page.navigate', { url: `${base}/tests/browser/ui-audit/index.html?surface=context-menu-overlay&theme=${theme}` });
    await page.send('Page.bringToFront');
    await page.wait('window.audit');
    await page.evaluate(`window.audit.emit('context-menu-overlay:open', { source: 'bottom-bar', kind: 'task-window', token: 'mock-window', x: ${width - 1}, y: ${height - 1}, isMinimized: true })`);
    await page.wait(`document.activeElement.textContent.trim() === 'Restore'`);
    const inspect = () => page.evaluate(`(async () => {
      const menu = document.querySelector('[role=menu]');
      const edges = (node) => { const r = node.getBoundingClientRect(); return [r.left,r.top,r.right,r.bottom]; };
      const style = (node) => { const c = getComputedStyle(node); return { font: c.font, border: c.border, radius: c.borderRadius, shadow: c.boxShadow, outline: c.outline, padding: c.padding, color: c.color, background: c.backgroundColor }; };
      const active = document.activeElement;
      getComputedStyle(active).boxShadow;
      await Promise.allSettled(active.getAnimations().filter(animation => Number.isFinite(animation.effect?.getComputedTiming().endTime)).map(animation => animation.finished));
      const c = getComputedStyle(active);
      return { box: edges(menu), style: style(menu), active: active.textContent.trim().slice(0,100), activeBox: edges(active), focus: active.matches(':focus-visible'),
        ring: [c.outlineStyle, c.outlineWidth, c.outlineColor, c.boxShadow], scroll: menu.scrollTop, outer: [window.scrollX, window.scrollY, document.querySelector('#app').scrollTop],
        rows: [...menu.querySelectorAll('button,[role=separator]')].map(n => ({ label: n.textContent.trim(), box: edges(n), disabled: n.disabled, style: style(n) })) };
    })()`);
    await page.evaluate(`(async () => { const menu = document.querySelector('[role=menu]'); await Promise.allSettled(menu.getAnimations().map(animation => animation.finished)); })()`);
    // Enter keyboard modality before measuring the automatically focused item.
    await press(page, 'ArrowDown');
    const arrow = await inspect();
    for (const type of ['keyDown', 'keyUp']) await page.send('Input.dispatchKeyEvent', { type, key: 'ArrowUp', code: 'ArrowUp', windowsVirtualKeyCode: 38 });
    const back = await inspect();
    await press(page, 'Tab');
    const tab = await inspect();
    for (const type of ['keyDown', 'keyUp']) await page.send('Input.dispatchKeyEvent', { type, key: 'End', code: 'End', windowsVirtualKeyCode: 35 });
    const end = await inspect();
    const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
    await writeFile(`${output}/${theme}-${width}.png`, Buffer.from(data, 'base64'));
    await press(page, 'Home');
    const home = await inspect();
    await page.evaluate('window.audit.requests.length = 0');
    await press(page, 'Escape');
    await page.wait(`!document.querySelector('[role=menu]')`);
    const escape = await page.evaluate('window.audit.requests');
    const [left, top, right, bottom] = arrow.box;
    const [, rowTop, , rowBottom] = arrow.activeBox;
    const row = { theme, width, height, arrow, back, tab, end, home, escape, missing: await page.evaluate('window.audit.missing()') };
    rows.push(row);
    if (arrow.active !== 'Pin to taskbar' || back.active !== 'Restore' || tab.active !== 'Pin to taskbar') failures.push({ theme, reason: 'Keyboard order must skip disabled rows' });
    if (!arrow.focus || left < 0 || top < 0 || right > width || bottom > height || rowTop < top || rowBottom > bottom) failures.push({ theme, reason: 'Focused row or overlay clipped' });
    const revealed = state => state.focus && hasFocusPaint(state.ring) && state.activeBox[1] >= state.box[1] && state.activeBox[3] <= state.box[3];
    if (end.active !== 'Close window' || !revealed(end)) failures.push({ theme, reason: 'End must reach and visibly focus the last enabled item' });
    if (home.active !== 'Restore' || !revealed(home)) failures.push({ theme, reason: 'Home must reach and visibly focus the first enabled item' });
    if ([tab, end, home].some(state => JSON.stringify(state.outer) !== JSON.stringify(back.outer)) || (height === 145 && !(end.scroll > home.scroll))) failures.push({ theme, reason: 'Tab, End, and Home scrolling must stay local to the menu' });
    if (escape.length !== 1 || escape[0].command !== 'hide_context_menu_overlay' || escape[0].args?.request?.restoreOriginFocus !== true || escape[0].args?.request?.source !== 'bottom-bar') failures.push({ theme, reason: 'Escape must request origin focus restoration once' });
    if (row.missing.length) failures.push({ theme, reason: `Missing IPC: ${row.missing}` });
  } catch (error) { failures.push({ theme, setup: error.stack }); }
  finally { await page.close(); }
}
await writeFile(`${output}/report.json`, JSON.stringify({ rows, failures }, null, 2));
console.log(`${rows.length} populated overlay states; ${failures.length} failures`, failures);
if (failures.length) process.exitCode = 1;
