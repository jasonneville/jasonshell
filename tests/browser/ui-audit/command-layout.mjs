import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';
import { hasFocusPaint } from '../close-parity/focus-paint.mjs';

const base = process.argv[2] ?? 'http://127.0.0.1:5199';
const output = process.argv[3] ?? 'test-results/ui-audit-command-layout';
const rows = [];
await mkdir(output, { recursive: true });
for (const [theme, width] of [['dark', 460], ['light', 460], ['translucent', 391]]) {
  const page = await connectPage('http://127.0.0.1:9295', 'about:blank');
  try {
    await page.send('Emulation.setDeviceMetricsOverride', { width, height: 420, deviceScaleFactor: 1, mobile: false });
    await page.send('Page.navigate', { url: `${base}/tests/browser/ui-audit/index.html?surface=command-panel&theme=${theme}` });
    await page.send('Page.bringToFront');
    await page.wait('window.audit');
    const tab = `document.querySelector('[aria-controls="command-panel-previous-runs"]')`;
    for (let i = 0; i < 20 && !await page.evaluate(`${tab} === document.activeElement`); i++) await press(page, 'Tab');
    await page.evaluate(`(async () => { const node = ${tab}; getComputedStyle(node).boxShadow;
      await Promise.allSettled(node.getAnimations().filter(animation => Number.isFinite(animation.effect?.getComputedTiming().endTime)).map(animation => animation.finished)); })()`);
    const row = await page.evaluate(`(() => { const node = ${tab}; const c = getComputedStyle(node);
      const tabs = [...node.closest('[role=tablist]').querySelectorAll('[role=tab]')].map(item => {
        let left = 0, right = innerWidth, top = 0, bottom = innerHeight;
        for (let a = item.parentElement; a; a = a.parentElement) {
          const s = getComputedStyle(a), r = a.getBoundingClientRect();
          if (/hidden|clip|auto|scroll/.test(s.overflowX)) { left = Math.max(left, r.left + a.clientLeft); right = Math.min(right, r.left + a.clientLeft + a.clientWidth); }
          if (/hidden|clip|auto|scroll/.test(s.overflowY)) { top = Math.max(top, r.top + a.clientTop); bottom = Math.min(bottom, r.top + a.clientTop + a.clientHeight); }
        }
        const box = item.getBoundingClientRect();
        const text = document.createRange(); text.selectNodeContents(item);
        const fits = r => r.left >= left - 0.01 && r.right <= right + 0.01 && r.top >= top - 0.01 && r.bottom <= bottom + 0.01;
        return { label: item.textContent.trim(), right: box.right, clipRight: right, fits: fits(box), labelFits: [...text.getClientRects()].every(r => fits(r) && r.left >= box.left - 0.01 && r.right <= box.right + 0.01 && r.top >= box.top - 0.01 && r.bottom <= box.bottom + 0.01) };
      });
      return { tabs, focused: node === document.activeElement && node.matches(':focus-visible'), ring: [c.outlineStyle,c.outlineWidth,c.outlineColor,c.boxShadow] }; })()`);
    const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
    await writeFile(`${output}/${theme}-${width}.png`, Buffer.from(data, 'base64'));
    await press(page, ' ');
    const activated = await page.evaluate(`${tab}.getAttribute('aria-selected') === 'true' && Boolean(document.getElementById('command-panel-previous-runs'))`);
    rows.push({ theme, width, ...row, painted: hasFocusPaint(row.ring), activated });
  } finally { await page.close(); }
}
await writeFile(`${output}/report.json`, JSON.stringify(rows, null, 2));
console.log(rows);
if (rows.some(row => !row.focused || !row.painted || !row.activated || row.tabs.length !== 2 || row.tabs.some(tab => !tab.fits || !tab.labelFits))) process.exitCode = 1;
