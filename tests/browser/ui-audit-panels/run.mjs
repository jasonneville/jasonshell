import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';

const output = process.argv[2] ?? 'test-results/ui-audit-panels';
const rows = [];
await mkdir(output, { recursive: true });

function focusGeometry() {
  const el = document.activeElement, style = getComputedStyle(el), r = el.getBoundingClientRect();
  let left = 0, top = 0, right = innerWidth, bottom = innerHeight;
  for (let a = el.parentElement; a; a = a.parentElement) {
    const s = getComputedStyle(a), b = a.getBoundingClientRect();
    if (s.overflowX !== 'visible') { left = Math.max(left, b.left+a.clientLeft); right = Math.min(right, b.left+a.clientLeft+a.clientWidth); }
    if (s.overflowY !== 'visible') { top = Math.max(top, b.top+a.clientTop); bottom = Math.min(bottom, b.top+a.clientTop+a.clientHeight); }
  }
  const clip = [left, top, right, bottom];
  const outline = style.outlineStyle !== 'none' && style.outlineColor !== 'rgba(0, 0, 0, 0)' ? parseFloat(style.outlineWidth) : 0;
  const outset = outline ? Math.max(0, outline + parseFloat(style.outlineOffset)) : 0;
  const fits = n => r.left-n >= left-.05 && r.right+n <= right+.05 && r.top-n >= top-.05 && r.bottom+n <= bottom+.05;
  const owner = el.closest('.process-filter') ?? el;
  const shadow = getComputedStyle(owner).boxShadow;
  const visibleShadow = shadow.replace(/rgba\([^)]*,\s*0\)/g, '').includes('rgb');
  const spread = owner === el && visibleShadow ? Number(shadow.match(/\) 0px 0px 0px ([\d.]+)px$/)?.[1] ?? 0) : 0;
  return { rect: r.toJSON(), clip, fits: fits(0), ringFits: fits(Math.max(outset, spread)), focusVisible: el.matches(':focus-visible'), outline: style.outline, shadow, focusPaint: Boolean(outline || visibleShadow), radius: style.borderRadius, border: style.border, font: style.fontSize };
}

const surfaceFilter = process.argv[3];
for (const [surface, normal, narrow, height] of [['audio-panel',320,280,430], ['tray-panel',252,210,220], ['process-manager',900,500,460]]) {
  if (surfaceFilter && surface !== surfaceFilter) continue;
  for (const theme of ['dark','light','translucent']) {
    const width = theme === 'translucent' ? narrow : normal;
    const row = { surface, theme, width, height, focus: {}, failures: [] };
    rows.push(row);
    const page = await connectPage('http://127.0.0.1:9397', 'about:blank');
    const active = selector => `document.activeElement.matches(${JSON.stringify(selector)})`;
    const count = command => `audit.calls.filter(r => r.command === '${command}').length`;
    const check = (ok, message) => { if (!ok) row.failures.push(message); };
    const screenshot = async name => {
      const { data } = await page.send('Page.captureScreenshot', { format:'png' });
      await writeFile(`${output}/${surface}-${theme}-${name}.png`, Buffer.from(data,'base64'));
    };
    const focus = async (selector, name) => {
      for (let i = 0; i < 30 && !await page.evaluate(active(selector)); i++) await press(page, 'Tab');
      assert(await page.evaluate(active(selector)), `Tab did not reach ${name}`);
      const g = row.focus[name] = await page.evaluate(`(async () => {
        await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
        const el = document.activeElement;
        await Promise.allSettled((el.closest('.process-filter') ?? el).getAnimations().filter(a => a.effect.getTiming().iterations !== Infinity).map(a => a.finished));
        return (${focusGeometry})();
      })()`);
      check(g.focusVisible && g.fits && g.ringFits && g.focusPaint, `${name}: focus/control clipped or lacks paint`);
    };
    const key = async (key, windowsVirtualKeyCode, modifiers = 0) => {
      for (const type of ['keyDown','keyUp']) await page.send('Input.dispatchKeyEvent', { type, key, code: key, windowsVirtualKeyCode, modifiers });
    };
    try {
      await page.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
      await page.send('Page.navigate', { url: `http://127.0.0.1:5397/tests/browser/ui-audit-panels/?surface=${surface}&theme=${theme}` });
      await page.send('Page.bringToFront');
      await page.wait('window.audit');
      if (surface === 'audio-panel') {
        await page.evaluate(`audit.emit('audio-panel:open')`);
        const master = '[aria-label="Master volume"]', out = '[aria-label="Output audio device"]';
        const input = '[aria-label="Input audio device"]', app = '[aria-label="Fixture editor volume"]';
        await page.wait(`document.querySelector('${app}')?.value === '35'`);
        await focus(master, 'Master volume'); await key('ArrowRight', 39);
        await page.wait(`document.querySelector('${master}').value === '43' && ${count('set_master_volume')} === 1`);
        await focus(out, 'Output device'); await screenshot('device-focus'); await press(page, 'ArrowDown');
        await page.wait(`document.querySelector('${out}').value === 'usb' && ${count('set_default_audio_output_device')} === 1`);
        await focus(input, 'Input device'); await focus(app, 'App volume'); await key('ArrowRight',39);
        await page.wait(`document.querySelector('${app}').value === '36' && ${count('set_app_volume')} === 1`);
        await screenshot('app-volume');
      } else if (surface === 'tray-panel') {
        await page.evaluate(`audit.emit('tray-panel:open')`);
        const first = '[aria-label="Fixture notification 1"]', second = '[aria-label="Fixture notification 2"]';
        await page.wait(`document.querySelectorAll('button[aria-label^="Fixture notification"]').length === 12`);
        const revision = n => `document.querySelector('[aria-label="Fixture notification 12 revision ${n}"]')`;
        await focus(first, 'First icon'); await screenshot('grid');
        await page.evaluate('window.trayOrigin = document.activeElement');
        await key('F10',121,8);
        await page.wait(revision(2));
        check(await page.evaluate(`${active(first)} && document.activeElement === window.trayOrigin`), 'Successful context relay drops originating icon focus');
        row.afterSuccessfulRelay = await page.evaluate('document.activeElement.tagName');
        await focus(first, 'First icon after relay');
        await screenshot('relay-success');
        await focus(second, 'Second icon'); await key('F10',121,8);
        await page.wait(`document.querySelector('[role=alert]')?.textContent.includes('remains open')`);
        check(await page.evaluate(`${active(second)} && document.querySelectorAll('button[aria-label^="Fixture notification"]').length === 12`), 'Failed relay loses icon focus or grid');
        assert(await page.evaluate(`${count('hide_tray_panel')} === 0 && ${count('invoke_system_tray_icon')} === 2 && audit.calls.filter(r => r.command === 'invoke_system_tray_icon').every(r => r.args.request.button === 'right')`));
        await screenshot('relay-error');

        const third = '[aria-label="Fixture notification 3"]', close = '.tray-header button';
        await focus(third, 'Delayed relay origin');
        await page.evaluate('audit.delayNextTrayRelay()');
        await key('F10',121,8);
        await page.wait(`${count('invoke_system_tray_icon')} === 3`);
        await key('F10',121,8);
        await focus(close, 'Header Close during pending relay');
        assert(await page.evaluate(`${count('invoke_system_tray_icon')} === 3 && ${count('list_system_tray_icons')} === 2`), 'Pending relay accepted a duplicate activation');
        await page.evaluate('window.trayDestination = document.activeElement; audit.releaseTrayRelay()');
        await page.wait(revision(3));
        check(await page.evaluate(`${active(close)} && document.activeElement === window.trayDestination`), 'Delayed relay stole the later Tab destination');
        check(await page.evaluate(`${count('hide_tray_panel')} === 0 && ${count('invoke_system_tray_icon')} === 3`), 'Delayed relay hid panel or produced duplicate IPC');
        await screenshot('relay-delayed-tab');
      } else {
        const filter = 'input[aria-label^="Filter processes"]', tab = '#manager-tab-processes', system = '#manager-tab-system_info';
        await page.wait(`document.querySelector('[role=status]')?.textContent.includes('2 processes') && document.body.innerText.includes('Fixture Editor') && document.body.innerText.includes('Fixture Sync')`);
        await focus(filter, 'Filter');
        await page.send('Input.insertText', { text: 'Editor' });
        await page.wait(`document.body.innerText.includes('Fixture Editor') && !document.body.innerText.includes('Fixture Sync')`);
        await screenshot('filter');
        await focus(tab, 'Processes tab'); await key('ArrowRight',39);
        await page.wait(`document.body.innerText.includes('Fixture GPU') && document.body.innerText.includes('Overview')`);
        assert(await page.evaluate(`${active(system)} && document.querySelector('${system}').getAttribute('aria-selected') === 'true'`));
        await focus(system, 'System Info tab'); await screenshot('system-info');
        await key('ArrowLeft',37);
        await page.wait(`${active(tab)} && document.querySelector('${filter}')?.value === 'Editor' && document.body.innerText.includes('Fixture Editor') && !document.body.innerText.includes('Fixture Sync')`);
      }
    } catch (error) {
      row.blocked = String(error).split('\n')[0];
      row.exceptions = page.errors.map(e => e.exception?.description ?? e.text);
      await screenshot('blocked');
    } finally {
      row.ipc = await page.evaluate('window.audit && { calls: audit.calls, missing: audit.missing }');
      await page.close();
    }
  }
}
assert(rows.length, `Unknown panel: ${surfaceFilter}`);
await writeFile(`${output}/report.json`, JSON.stringify(rows,null,2));
for (const {surface, theme, blocked, failures, ipc} of rows) {
  const missing = ipc?.missing ?? [];
  console.log(surface, theme, blocked || missing.length ? 'BLOCKED' : failures.length ? 'RED' : 'PASS', blocked ?? '', ...missing, ...failures);
  if (blocked || failures.length || missing.length) process.exitCode = 1;
}
