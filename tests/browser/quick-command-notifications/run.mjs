import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as wait } from 'node:timers/promises';
import { createServer } from 'vite';

// Automated runner for the renderer journey in main.ts (mocked IPC, no Tauri/Windows shell).
// VIS-20b matrix: base-dark + base-light, with and without prefers-reduced-motion.
// Run: node tests/browser/quick-command-notifications/run.mjs   (set CHROME_PATH if needed)
const chrome = [process.env.CHROME_PATH, 'C:/Program Files/Google/Chrome/Application/chrome.exe', '/usr/bin/chromium'].find((path) => path && existsSync(path));
assert(chrome, 'Set CHROME_PATH to a Chromium executable');
const server = await createServer({ configFile: fileURLToPath(new URL('./vite.config.ts', import.meta.url)), server: { host: '127.0.0.1', port: 0, strictPort: false } });
await server.listen();
const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
const profile = mkdtempSync(join(tmpdir(), 'qc-notifications-'));
const browser = spawn(chrome, ['--headless', '--remote-debugging-port=0', '--window-size=1100,1000', '--no-first-run', `--user-data-dir=${profile}`, 'about:blank']);
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
    const timer = setTimeout(() => { socket.removeEventListener('message', listener); reject(new Error(`CDP timeout: ${method}`)); }, 20000);
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
  const until = async (expression, message) => {
    for (let i = 0; i < 200; i++) { if (await evaluate(expression)) return; await wait(25); }
    throw new Error(`Timed out: ${message ?? expression}`);
  };
  await send('Page.enable');
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  const mouse = (point) => send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: point.x, y: point.y, button: 'none', buttons: 0 });

  for (const theme of ['base-dark', 'base-light']) {
    for (const motion of ['no-preference', 'reduce']) {
      const label = `${theme}/${motion}`;
      await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: motion }] });
      await send('Page.navigate', { url: `${origin}/?theme=${theme}` });
      await until("typeof window.runJourney === 'function'", `${label} fixture did not start`);
      console.log(`${label}: ${await evaluate('window.runJourney()')}`);

      // Real pointer hover pauses; real pointer leave resumes (idempotent, one pause/resume pair).
      const probe = await evaluate('window.beginHover()');
      assert.equal(probe.reducedMotion, motion === 'reduce');
      const actionsBefore = (await evaluate('window.hoverStatus()')).actions.length;
      await mouse(probe.over);
      await mouse({ x: probe.over.x + 2, y: probe.over.y });
      await until('window.hoverStatus().paused', `${label} pointer hover did not pause native`);
      const hovering = await evaluate('window.hoverStatus()');
      assert.deepEqual(hovering.actions.slice(actionsBefore), ['pause'], `${label} hover must send exactly one pause`);
      if (motion === 'no-preference') assert.equal(hovering.ring, 'paused', `${label} ring must pause on hover`);
      else assert.equal(hovering.ring, null, `${label} reduced motion has no ring animation`);
      await mouse(probe.away);
      await until('!window.hoverStatus().paused', `${label} pointer leave did not resume native`);
      const left = await evaluate('window.hoverStatus()');
      assert.deepEqual(left.actions.slice(actionsBefore), ['pause', 'resume'], `${label} leave must send exactly one resume`);
      if (motion === 'no-preference') assert.equal(left.ring, 'running', `${label} ring must resume`);
      await evaluate('window.endHover()');
    }
  }
  console.log('PASS: notification journey (hit target, live region, solid icon, pause/resume) across themes and motion preferences');
} finally {
  socket?.close();
  browser.kill();
  await server.close();
  await wait(300);
  try { rmSync(profile, { recursive: true, force: true }); } catch { /* Chromium may still hold the profile. */ }
}
