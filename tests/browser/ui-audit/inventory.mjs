import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';

const base = process.argv[2] ?? 'http://127.0.0.1:5199';
const output = process.argv[3] ?? 'test-results/ui-audit-inventory';
const bounds = {
  'top-bar': [1280, 24], 'bottom-bar': [1280, 33], 'snip-overlay': [1280, 720], 'snip-preview': [320, 264],
  'task-gallery': [384, 208], 'task-preview': [332, 228], 'search-panel': [420, 320], 'stack-popup': [980, 430],
  'process-manager': [720, 520], 'quick-launch-panel': [360, 360], 'control-plane': [860, 620], 'settings-panel': [440, 520],
  'tray-panel': [252, 220], 'terminal-panel': [860, 520], 'command-panel': [460, 420], 'audio-panel': [320, 430],
  'speech-history-panel': [360, 430], 'calendar-panel': [360, 430], 'speech-indicator': [280, 44], 'context-menu-overlay': [360, 360]
};
const loader = await readFile(new URL('../../../src/lib/surfaceLoader.ts', import.meta.url), 'utf8');
const labels = [...loader.matchAll(/^\s*'([^']+)': \(\) => import/gm)].map(match => match[1]);
if (labels.length !== Object.keys(bounds).length || labels.some(label => !bounds[label])) throw new Error('Surface inventory and loader differ');
await mkdir(output, { recursive: true });
const rows = [];
for (const surface of labels) for (const theme of ['dark', 'light', 'translucent']) {
  const [normalWidth, height] = bounds[surface];
  const narrow = theme === 'translucent' && surface !== 'speech-indicator';
  const width = narrow ? Math.round(normalWidth * 0.85) : normalWidth;
  const id = `${surface}-${theme}`;
  const page = await connectPage('http://127.0.0.1:9295', 'about:blank');
  try {
    await page.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: narrow ? 1.25 : 1, mobile: false });
    await page.send('Page.navigate', { url: `${base}/tests/browser/ui-audit/index.html?surface=${surface}&theme=${theme}` });
    await page.send('Page.bringToFront');
    await page.wait(`location.href.startsWith(${JSON.stringify(base)}) && document.getElementById('app')`);
    await page.evaluate(`import('/tests/browser/ui-audit/main.ts').then(() => true)`);
    await page.wait('window.audit');
    await press(page, 'Tab');
    const state = await page.evaluate(`(async () => {
      await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      const app = document.getElementById('app');
      const controls = [...app.querySelectorAll('button,input,select,[role=option],[role=menuitem],[role=separator]')];
      const describe = node => {
        const c = getComputedStyle(node), r = node.getBoundingClientRect();
        return { tag: node.tagName, role: node.getAttribute('role'), label: node.getAttribute('aria-label') || (node.innerText ?? '').trim().slice(0,100),
          rect: [r.x, r.y, r.width, r.height], inViewport: r.left >= 0 && r.top >= 0 && r.right <= innerWidth && r.bottom <= innerHeight,
          color: c.color, background: c.backgroundColor, border: c.border, radius: c.borderRadius, shadow: c.boxShadow, outline: c.outline,
          spacing: [c.padding, c.gap], font: [c.fontSize, c.fontWeight, c.lineHeight], display: c.display, visibility: c.visibility,
          disabled: node.matches(':disabled,[aria-disabled=true]'), pressed: node.getAttribute('aria-pressed'), selected: node.getAttribute('aria-selected'), expanded: node.getAttribute('aria-expanded') };
      };
      return { root: app.firstElementChild && describe(app.firstElementChild), text: app.innerText.slice(0,1200),
        focus: document.activeElement === document.body ? null : describe(document.activeElement),
        controls: controls.slice(0,80).map(describe), calls: [...window.audit.calls], missing: window.audit.missing() };
    })()`);
    const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
    await writeFile(`${output}/${id}.png`, Buffer.from(data, 'base64'));
    rows.push({ id, width, height, ...state });
  } catch (error) {
    rows.push({ id, width, height, setup: String(error).split('\n')[0] });
  } finally { await page.close(); }
}
await writeFile(`${output}/report.json`, JSON.stringify(rows, null, 2));
console.log(`${rows.length} states; ${rows.filter(row => row.setup).length} setup failures; ${rows.filter(row => row.missing?.length).length} mock gaps`);
