import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';
import { hasFocusPaint } from '../close-parity/focus-paint.mjs';

const output = process.argv[2] ?? 'test-results/ui-audit-active-panels';
const rows = [];
await mkdir(output, { recursive: true });
for (const [surface, normal, narrow, height, selector] of [
  ['audio-panel', 320, 280, 430, 'select[aria-label="Output audio device"], select[aria-label="Input audio device"], input[type=range]'],
  ['tray-panel', 252, 210, 220, '.tray-icon-button'],
  ['speech-history-panel', 360, 300, 430, '.speech-history-copy']
]) for (const theme of ['dark', 'light', 'translucent']) {
  const width = theme === 'translucent' ? narrow : normal;
  const id = `${surface}-${theme}-${width}`;
  const page = await connectPage('http://127.0.0.1:9295', 'about:blank');
  try {
    await page.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
    await page.send('Page.navigate', { url: `http://127.0.0.1:5199/tests/browser/ui-audit/index.html?surface=${surface}&populated&theme=${theme}` });
    await page.send('Page.bringToFront');
    await page.wait(`location.port === '5199' && document.getElementById('app')`);
    await page.wait('window.audit');
    await page.evaluate(`window.audit.emit('${surface}:open')`);
    const controls = `Array.from(document.querySelectorAll(${JSON.stringify(selector)}))`;
    await page.wait(`window.audit && ${controls}.length`);
    const focused = {};
    for (let i = 0; i < 18; i++) {
      await press(page, 'Tab');
      const row = await page.evaluate(`(async () => {
        const el = document.activeElement;
        if (!${controls}.includes(el)) return null;
        getComputedStyle(el).boxShadow;
        await Promise.allSettled(el.getAnimations().filter(a => Number.isFinite(a.effect?.getComputedTiming().endTime)).map(a => a.finished));
        const r = el.getBoundingClientRect(), c = getComputedStyle(el);
        let left = 0, right = innerWidth, top = 0, bottom = innerHeight;
        for (let a = el.parentElement; a; a = a.parentElement) {
          const s = getComputedStyle(a), b = a.getBoundingClientRect();
          if (/hidden|clip|auto|scroll/.test(s.overflowX)) { left = Math.max(left, b.left+a.clientLeft); right = Math.min(right, b.left+a.clientLeft+a.clientWidth); }
          if (/hidden|clip|auto|scroll/.test(s.overflowY)) { top = Math.max(top, b.top+a.clientTop); bottom = Math.min(bottom, b.top+a.clientTop+a.clientHeight); }
        }
        const scroller = el.closest('.sound-apps');
        const outline = c.outlineStyle === 'none' ? 0 : Math.max(0, parseFloat(c.outlineWidth) + parseFloat(c.outlineOffset));
        const shadow = Math.max(0, ...c.boxShadow.split(/,(?![^()]*[)]) /).map(layer => {
          if (layer.includes('inset')) return 0;
          const [x=0,y=0,blur=0,spread=0] = (layer.match(/-?[\\d.]+px/g) ?? []).map(parseFloat);
          return Math.max(Math.abs(x),Math.abs(y)) + blur + spread;
        }));
        const inset = Math.max(outline,shadow);
        const appSlider = scroller && { noHorizontalOverflow: scroller.scrollWidth === scroller.clientWidth, scrollWidth: scroller.scrollWidth, clientWidth: scroller.clientWidth,
          outset: inset, paintFits: r.left-inset >= left-.01 && r.right+inset <= right+.01 && r.top-inset >= top-.01 && r.bottom+inset <= bottom+.01,
          ring: [c.outlineStyle,c.outlineWidth,c.outlineColor,c.boxShadow] };
        return { label: el.getAttribute('aria-label'), rect: r.toJSON(), clip: [left,top,right,bottom], fits: r.left >= left-.01 && r.right <= right+.01 && r.top >= top-.01 && r.bottom <= bottom+.01, appSlider,
          focus: el.matches(':focus-visible'), color: c.color, background: c.backgroundColor, border: c.border, radius: c.borderRadius, font: c.fontSize, current: el.selectedOptions?.[0]?.textContent ?? el.value };
      })()`);
      if (row) focused[row.label] = row;
      if (row && (!row.fits || row.appSlider)) {
        const { data } = await page.send('Page.captureScreenshot', { format:'png' });
        await writeFile(`${output}/${id}-focus.png`, Buffer.from(data,'base64'));
      }
      if (Object.keys(focused).length === await page.evaluate(`${controls}.length`)) break;
    }
    let audio;
    if (surface === 'audio-panel') {
      const master = `document.querySelector('[aria-label="Master volume"]')`;
      await page.evaluate(`${master}.focus()`);
      const key = { key:'ArrowRight', code:'ArrowRight', windowsVirtualKeyCode:39 };
      for (const type of ['keyDown','keyUp']) await page.send('Input.dispatchKeyEvent', { type, ...key });
      await page.wait(`${master}.value === '43' && window.audit.calls.includes('set_master_volume')`);
      const outputDevice = `document.querySelector('[aria-label="Output audio device"]')`;
      await page.evaluate(`${outputDevice}.focus()`);
      await press(page, 'ArrowDown');
      await page.wait(`${outputDevice}.value === 'headset' && window.audit.calls.includes('set_default_audio_output_device')`);
      audio = await page.evaluate(`({ volume:${master}.value, selected:${outputDevice}.selectedOptions[0].textContent, requests: window.audit.requests.filter(r=>r.command.startsWith('set_')) })`);
    }
    const { data } = await page.send('Page.captureScreenshot', { format:'png' });
    await writeFile(`${output}/${id}.png`, Buffer.from(data,'base64'));
    const missing = await page.evaluate('window.audit.missing()');
    const expected = await page.evaluate(`${controls}.length`);
    rows.push({ id, expected, focused, audio, missing });
  } catch (error) { rows.push({ id, setup: String(error).split('\n')[0], observed: await page.evaluate(`({ text:document.body.innerText.slice(0,300), calls:window.audit?.calls, missing:window.audit?.missing() })`).catch(() => null) }); }
  finally { await page.close(); }
}
await writeFile(`${output}/report.json`, JSON.stringify(rows,null,2));
const failures = rows.filter(row => row.setup || row.missing.length || Object.keys(row.focused).length !== row.expected || Object.values(row.focused).some(control => !control.fits || !control.focus) || (row.audio && (row.audio.requests.length !== 2 || !Object.values(row.focused).some(({ appSlider: s }) => s?.noHorizontalOverflow && s.paintFits && hasFocusPaint(s.ring)))));
console.log(`${rows.length} states; ${failures.length} failures`, failures.map(row=>row.id));
if (failures.length) process.exitCode = 1;
