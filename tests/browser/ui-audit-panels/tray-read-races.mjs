import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';

const output = process.argv[2] ?? 'test-results/ui-audit-panels-tray-races-red';
await mkdir(output, { recursive: true });
const report = [];
for (const origin of ['pointer-reopen', 'overlapping-open']) for (const reject of [false, true]) {
  const id = `${origin}-${reject ? 'reject' : 'resolve'}`;
  const row = { id };
  report.push(row);
  const page = await connectPage('http://127.0.0.1:9397', 'about:blank');
  const count = command => `audit.calls.filter(r => r.command === '${command}').length`;
  const open = () => page.evaluate(`audit.emit('tray-panel:open')`);
  const snapshot = n => `document.querySelector('[aria-label="Fixture notification 12 revision ${n}"]')`;
  const state = () => page.evaluate(`({ labels: [...document.querySelectorAll('.tray-grid button')].map(b => b.getAttribute('aria-label')), alert: document.querySelector('[role=alert]')?.textContent ?? null, loading: [...document.querySelectorAll('[role=status]')].some(s => s.textContent.includes('Loading')) })`);
  try {
    await page.send('Emulation.setDeviceMetricsOverride', { width: 210, height: 220, deviceScaleFactor: 1, mobile: false });
    await page.send('Page.navigate', { url: 'http://127.0.0.1:5397/tests/browser/ui-audit-panels/?surface=tray-panel&theme=translucent' });
    await page.send('Page.bringToFront');
    await page.wait('window.audit');
    await open();
    await page.wait(snapshot(1));
    await page.evaluate('audit.delayNextTrayRead()');
    if (origin === 'pointer-reopen') {
      const {x,y} = await page.evaluate(`(() => { const r = document.querySelector('[aria-label="Fixture notification 1"]').getBoundingClientRect(); return {x:r.x+r.width/2,y:r.y+r.height/2}; })()`);
      for (const type of ['mousePressed', 'mouseReleased']) await page.send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
    } else await open();
    await page.wait(`audit.trayReadPending() && ${count('list_system_tray_icons')} === 2`);
    if (origin === 'pointer-reopen') {
      assert(await page.evaluate(`${count('invoke_system_tray_icon')} === 1 && audit.calls.find(r => r.command === 'invoke_system_tray_icon').args.request.button === 'left'`));
      await press(page, 'Escape');
      await page.wait(`${count('hide_tray_panel')} === 1`);
      // Native acknowledges close to the top-bar owner; the Tray webview stays mounted.
      await page.evaluate(`audit.emit('tray-panel:closed')`);
    }
    await open();
    await page.wait(snapshot(3));
    row.before = await state();
    assert.equal(row.before.labels.length, 12);
    assert.equal(row.before.alert, null);
    assert.equal(row.before.loading, false);
    await page.evaluate(`audit.settleTrayRead(${reject})`);
    row.after = await state();
    row.ipc = await page.evaluate('({calls: audit.calls, missing: audit.missing})');
    assert.deepEqual(row.ipc.missing, []);
    row.pass = JSON.stringify(row.before) === JSON.stringify(row.after);
    const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
    await writeFile(`${output}/${id}.png`, Buffer.from(data, 'base64'));
  } catch (error) { row.blocked = String(error).split('\n')[0]; }
  finally { await page.close(); }
}
await writeFile(`${output}/report.json`, JSON.stringify(report, null, 2));
for (const row of report) {
  console.log(row.id, row.blocked ? `BLOCKED ${row.blocked}` : row.pass ? 'PASS' : `RED: ${row.after.labels.at(-1) ?? 'grid missing'}; alert=${row.after.alert}; loading=${row.after.loading}`);
  if (!row.pass || row.blocked) process.exitCode = 1;
}
