import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';
import { hasFocusPaint } from './focus-paint.mjs';

const [endpoint = 'http://127.0.0.1:9295', base = 'http://127.0.0.1:5196', output = 'test-results/close-parity-terminal'] = process.argv.slice(2);
const rows = [];
const failures = [];
const form = 'document.querySelector(".stack-terminal-search, .terminal-search")';
const input = `${form}?.querySelector('input')`;
const button = `${form}?.querySelector('button:last-of-type')`;
const terminalFocused = 'document.activeElement.matches(".xterm-helper-textarea")';
await mkdir(output, { recursive: true });
for (const [theme, width] of [['dark', 680], ['light', 680], ['translucent', 360]]) {
  let reference;
  for (const panel of ['stack-terminal', 'terminal']) {
    const id = `${panel}-${theme}-${width}`;
    const check = (condition, assertion) => { if (!condition) failures.push({ id, assertion }); };
    const page = await connectPage(endpoint, 'about:blank');
    try {
      await page.send('Emulation.setDeviceMetricsOverride', { width, height: 540, deviceScaleFactor: width === 360 ? 1.25 : 1, mobile: false });
      await page.send('Page.navigate', { url: `${base}/tests/browser/close-parity/index.html?panel=${panel}&theme=${theme}` });
      await page.send('Page.bringToFront');
      await page.wait(`location.href.startsWith(${JSON.stringify(base)}) && document.getElementById('app')`);
      await page.evaluate(`import('/tests/browser/close-parity/main.ts').then(() => true)`);
      await page.wait('window.closeParityLedger');
      if (panel === 'terminal') await page.evaluate('window.openFixtureTerminal()');
      await page.wait('document.querySelector(".xterm-helper-textarea")');
      await page.evaluate('document.querySelector(".xterm-helper-textarea").focus()');
      await page.wait(terminalFocused);
      const openSearch = async () => {
        const key = { key: 'f', code: 'KeyF', windowsVirtualKeyCode: 70, modifiers: 2 };
        await page.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', ...key });
        await page.send('Input.dispatchKeyEvent', { type: 'keyUp', ...key });
        await page.wait(`${input} === document.activeElement`);
      };
      await openSearch();
      await page.send('Input.insertText', { text: 'find me' });
      await page.wait(`${input}.value === 'find me'`);
      for (let index = 0; index < 6 && !await page.evaluate(`${button} === document.activeElement`); index++) await press(page, 'Tab');
      const style = await page.evaluate(`(() => { const b = ${button}; const r = b.getBoundingClientRect(); const c = getComputedStyle(b);
        return { name: b.getAttribute('aria-label') || b.textContent.trim(), text: b.textContent.trim(), icon: b.querySelector('svg path')?.getAttribute('d') ?? null,
          width: r.width, height: r.height, fits: r.left >= 0 && r.right <= innerWidth && r.top >= 0 && r.bottom <= innerHeight,
          focus: b === document.activeElement && b.matches(':focus-visible'), ring: [c.outlineStyle, c.outlineWidth, c.outlineColor, c.boxShadow],
          point: { x: r.left+r.width/2, y: r.top+r.height/2 } }; })()`);
      const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
      await writeFile(`${output}/${id}.png`, Buffer.from(data, 'base64'));
      check(style.name === 'Close terminal search', 'Name distinguishes search from the terminal session');
      check(style.icon && !style.text, 'Icon-only close affordance');
      check(style.width >= 24 && style.height >= 24 && style.fits, '24 CSS pixel target fits');
      check(style.focus && hasFocusPaint(style.ring), 'Visible keyboard focus');
      const chrome = JSON.stringify([style.icon, style.width, style.height, style.ring]);
      if (panel === 'stack-terminal') reference = chrome;
      else check(reference === chrome, 'Stack and dedicated terminal use equal icon, target and focus styling');
      await page.evaluate('window.closeParityLedger.length = 0');
      await page.send('Input.dispatchMouseEvent', { type: 'mousePressed', button: 'left', clickCount: 1, ...style.point });
      await page.send('Input.dispatchMouseEvent', { type: 'mouseReleased', button: 'left', clickCount: 1, ...style.point });
      await page.wait(`!${form} && ${terminalFocused}`);
      await openSearch();
      check(await page.evaluate(`${input}.value === ''`), 'Click cleared the query');
      await page.send('Input.insertText', { text: 'input escape probe' });
      await page.wait(`${input}.value === 'input escape probe'`);
      await press(page, 'Escape');
      const inputEscape = await page.evaluate(`({ open: Boolean(${form}), query: ${input}?.value ?? null, terminalFocused: ${terminalFocused}, inputFocused: ${input} === document.activeElement })`);
      check(inputEscape.open && inputEscape.inputFocused && inputEscape.query === 'input escape probe', 'Escape from the search input preserves search, focus, and query');
      if (!inputEscape.open) {
        await page.evaluate('document.querySelector(".xterm-helper-textarea").focus()');
        await openSearch();
      }
      await page.evaluate(`(() => { const field = ${input}; field.focus(); field.select(); })()`);
      await page.send('Input.insertText', { text: 'xterm escape probe' });
      await page.wait(`${input}.value === 'xterm escape probe'`);
      await page.evaluate('document.querySelector(".xterm-helper-textarea").focus()');
      await press(page, 'Escape');
      const xtermEscape = await page.evaluate(`({ open: Boolean(${form}), terminalFocused: ${terminalFocused} })`);
      check(!xtermEscape.open && xtermEscape.terminalFocused, 'Escape handled by xterm closes search without losing terminal focus');
      if (!xtermEscape.open && xtermEscape.terminalFocused) {
        await openSearch();
        check(await page.evaluate(`${input}.value === ''`), 'Escape cleared the query');
      }
      const sessionActions = await page.evaluate('window.closeParityLedger.filter(name => /^(hide_|stop_|start_|write_)/.test(name))');
      check(!sessionActions.length, `Search dismissal must not dispatch session/window actions: ${sessionActions}`);
      rows.push({ id, style, inputEscape, xtermEscape, sessionActions });
    } catch (error) { failures.push({ id, setup: String(error).split('\n')[0], exceptions: page.errors.map(item => item.exception?.description ?? item.text) }); }
    finally { await page.close(); }
  }
}
await writeFile(`${output}/report.json`, JSON.stringify({ rows, failures }, null, 2));
console.log(`${rows.length} terminal states captured; ${failures.length} failures. ${output}/report.json`);
if (failures.length) process.exitCode = 1;
