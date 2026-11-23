import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as wait } from 'node:timers/promises';
import { createServer } from 'vite';

// VIS-10 journey: with Chromium forced-colors emulation, every keyboard focus stop
// of real production controls must still paint a non-none outline (Chromium forces
// box-shadow to none, so a box-shadow-only ring disappears).
// Run: node tests/browser/forced-colors-focus/run.mjs   (set CHROME_PATH if needed)
const chrome = [process.env.CHROME_PATH, 'C:/Program Files/Google/Chrome/Application/chrome.exe', '/usr/bin/chromium'].find((path) => path && existsSync(path));
assert(chrome, 'Set CHROME_PATH to a Chromium executable');
const server = await createServer({ configFile: fileURLToPath(new URL('./vite.config.ts', import.meta.url)), server: { host: '127.0.0.1', port: 0 } });
await server.listen();
const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
const profile = mkdtempSync(join(tmpdir(), 'forced-colors-'));
const browser = spawn(chrome, ['--headless', '--remote-debugging-port=0', '--window-size=900,650', '--no-first-run', `--user-data-dir=${profile}`, 'about:blank']);
let socket;
try {
  const endpoint = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('Chromium debug endpoint unavailable')), 10000);
    let output = '';
    browser.stderr.on('data', (chunk) => {
      output += chunk;
      const match = output.match(/DevTools listening on ws:\/\/([^/\s]+)/);
      if (match) { clearTimeout(timer); resolve(`http://${match[1]}`); }
    });
    browser.once('error', (error) => { clearTimeout(timer); reject(error); });
  });
  const tabs = await (await fetch(`${endpoint}/json/list`)).json();
  socket = new WebSocket(tabs.find((tab) => tab.type === 'page').webSocketDebuggerUrl);
  await once(socket, 'open');
  let id = 0;
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const current = ++id;
    const timer = setTimeout(() => { socket.removeEventListener('message', listener); reject(new Error(`CDP timeout: ${method}`)); }, 8000);
    function listener(event) {
      const response = JSON.parse(event.data);
      if (response.id !== current) return;
      clearTimeout(timer);
      socket.removeEventListener('message', listener);
      if (response.error) reject(new Error(JSON.stringify(response.error))); else resolve(response.result);
    }
    socket.addEventListener('message', listener);
    socket.send(JSON.stringify({ id: current, method, params }));
  });
  const evaluate = async (expression) => {
    const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
    if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text);
    return result.result.value;
  };
  const until = async (expression) => {
    for (let i = 0; i < 200; i++) { if (await evaluate(expression)) return; await wait(25); }
    throw new Error(`Timed out: ${expression}`);
  };
  const tab = async () => {
    await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
    await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
  };
  const stop = () => evaluate(`(() => { const el = document.activeElement; if (!el || el === document.body) return null; const s = getComputedStyle(el);
    return { label: (el.getAttribute('aria-label') || el.textContent || el.tagName).trim().slice(0, 30), tag: el.tagName, visible: el.matches(':focus-visible'),
      wrapperOutlineWidth: el.closest('.process-filter') && el.closest('.process-filter') !== el ? (getComputedStyle(el.closest('.process-filter')).outlineStyle === 'none' ? 0 : parseFloat(getComputedStyle(el.closest('.process-filter')).outlineWidth)) : null,
      outlineStyle: s.outlineStyle, outlineWidth: parseFloat(s.outlineWidth), outlineColor: s.outlineColor, boxShadow: s.boxShadow }; })()`);

  const pages = [
    { name: 'base controls + context menu', query: '', minimumStops: 8 },
    { name: 'confirm dialog', query: '?dialog=1', minimumStops: 2 }
  ];
  const failures = [];
  await send('Page.enable');
  await send('Emulation.setEmulatedMedia', { features: [{ name: 'forced-colors', value: 'active' }] });
  for (const theme of ['base-dark', 'base-light']) {
    for (const page of pages) {
      await send('Page.navigate', { url: `${origin}/${page.query}${page.query ? '&' : '?'}theme=${theme}` });
      await until("document.documentElement.dataset.ready === 'true'");
      assert.equal(await evaluate("matchMedia('(forced-colors: active)').matches"), true, 'forced-colors emulation must be active');
      const seen = new Map();
      const first = await stop();
      if (first) seen.set(first.label, first);
      for (let i = 0; i < 14; i++) {
        await tab();
        const current = await stop();
        if (current) seen.set(current.label, current);
      }
      assert.ok(seen.size >= page.minimumStops, `${page.name}/${theme}: expected >= ${page.minimumStops} focus stops, saw ${[...seen.keys()].join(', ')}`);
      for (const info of seen.values()) {
        // Wrapper-owned input: exactly one ring (the wrapper's), never wrapper + input stacked.
        if (info.wrapperOutlineWidth !== null) {
          const inputRing = info.outlineStyle !== 'none' && info.outlineWidth > 0;
          if (!(info.wrapperOutlineWidth > 0)) failures.push(`${theme} ${page.name}: ${info.label} wrapper shows no ring in forced-colors`);
          else if (inputRing) failures.push(`${theme} ${page.name}: ${info.label} shows two rings (wrapper and input)`);
          continue;
        }
        if (!info.visible) failures.push(`${theme} ${page.name}: ${info.label} was not :focus-visible after keyboard Tab`);
        else if (info.outlineStyle === 'none' || !(info.outlineWidth > 0)) failures.push(`${theme} ${page.name}: ${info.label} (${info.tag}) has no outline in forced-colors (${info.outlineStyle} ${info.outlineWidth}px)`);
      }
    }
  }
  assert.deepEqual(failures, [], `Focus indicator vanished in forced-colors:\n${failures.join('\n')}`);
  console.log('PASS: every keyboard focus stop keeps a visible outline under forced-colors emulation');
} finally {
  socket?.close();
  browser.kill();
  await server.close();
  await wait(300);
  try { rmSync(profile, { recursive: true, force: true }); } catch { /* Chromium may still hold the profile. */ }
}
