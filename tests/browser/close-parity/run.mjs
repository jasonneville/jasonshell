import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';
import { hasFocusPaint } from './focus-paint.mjs';

const [endpoint = 'http://127.0.0.1:9295', base = 'http://127.0.0.1:5196', output = 'test-results/close-parity'] = process.argv.slice(2);
const panels = [
  ['calendar', 'Close calendar', 'hide_calendar_panel'],
  ['speech', 'Close speech history', 'hide_speech_history_panel'],
  ['audio', 'Close sound controls', 'hide_audio_panel'],
  ['tray', 'Close notification area icons', 'hide_tray_panel']
];
const rows = [];
const failures = [];
const same = (id, property, actual, expected) => {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) failures.push({ id, property, actual, expected });
};
await mkdir(output, { recursive: true });
for (const [theme, width] of [['dark', 440], ['light', 440], ['translucent', 440], ['dark', 280], ['light', 280], ['translucent', 280]]) {
  let reference;
  for (const [panel, name, hide] of panels) {
    const id = `${panel}-${theme}-${width}`;
    const page = await connectPage(endpoint, 'about:blank');
    try {
      await page.send('Emulation.setDeviceMetricsOverride', { width, height: 540, deviceScaleFactor: width === 280 ? 1.25 : 1, mobile: false });
      await page.send('Page.navigate', { url: `${base}/tests/browser/close-parity/index.html?panel=${panel}&theme=${theme}` });
      await page.send('Page.bringToFront');
      await page.wait(`location.href.startsWith(${JSON.stringify(base)}) && document.getElementById('app')`);
      await page.evaluate(`import('/tests/browser/close-parity/main.ts').then(() => true)`);
      const selector = `button[aria-label="${name}"]`;
      const button = `document.querySelector(${JSON.stringify(selector)})`;
      await page.wait(`window.closeParityLedger && ${button}`);
      const measure = `(() => {
        const button = ${button}, r = button.getBoundingClientRect();
        const header = button.closest('header, .sound-panel-header');
        const h = header?.getBoundingClientRect();
        const headerStyle = header && getComputedStyle(header);
        const contentRight = h && h.right - parseFloat(headerStyle.borderRightWidth) - parseFloat(headerStyle.paddingRight);
        const style = getComputedStyle(button);
        return { icon: button.querySelector('svg path')?.getAttribute('d') ?? null,
          text: button.textContent.trim(), width: r.width, height: r.height,
          rightInset: h ? Math.round(contentRight - r.right) : null,
          centerOffset: h ? Math.round((r.top + r.bottom - h.top - h.bottom) / 2) : null,
          inViewport: r.left >= 0 && r.top >= 0 && r.right <= innerWidth && r.bottom <= innerHeight,
          focused: document.activeElement === button && button.matches(':focus-visible'),
          ring: [style.outlineStyle, style.outlineWidth, style.outlineColor, style.boxShadow],
          chrome: [style.backgroundColor, style.borderTopColor, style.borderRadius, style.color],
          overlay: getComputedStyle(document.documentElement).getPropertyValue('--js-color-surface-overlay').trim() };
      })()`;
      const idle = await page.evaluate(measure);
      for (let index = 0; index < 30 && !await page.evaluate(`${button} === document.activeElement`); index++) await press(page, 'Tab');
      // Measure the final focus paint, not a frame partway through its CSS transition.
      await page.evaluate(`(async () => {
        const target = ${button};
        getComputedStyle(target).boxShadow;
        await Promise.allSettled(target.getAnimations().filter(animation => Number.isFinite(animation.effect?.getComputedTiming().endTime)).map(animation => animation.finished));
      })()`);
      const focused = await page.evaluate(measure);
      const screenshot = async phase => {
        const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
        await writeFile(`${output}/${id}-${phase}.png`, Buffer.from(data, 'base64'));
      };
      await screenshot('focus');
      same(id, 'keyboard focus', focused.focused, true);
      same(id, 'painted focus change', hasFocusPaint(focused.ring) && JSON.stringify(idle.ring) !== JSON.stringify(focused.ring), true);
      same(id, 'minimum 24 CSS px', idle.width >= 24 && idle.height >= 24, true);
      same(id, 'viewport fit', focused.inViewport, true);
      const tooltip = `Array.from(document.querySelectorAll('[role=tooltip]')).some(node => node.textContent.trim() === ${JSON.stringify(name)} && node.getBoundingClientRect().height > 0 && getComputedStyle(node).visibility === 'visible')`;
      const point = await page.evaluate(`(() => { const r = ${button}.getBoundingClientRect(); return { x: r.left+r.width/2, y: r.top+r.height/2 }; })()`);
      await page.send('Input.dispatchMouseEvent', { type: 'mouseMoved', ...point });
      const tooltipVisible = await page.wait(tooltip, 1300).then(() => true, () => false);
      same(id, 'visible named tooltip', tooltipVisible, true);
      if (tooltipVisible) await screenshot('tooltip');
      if (panel === 'calendar') reference = { idle, focused };
      same(id, 'icon only', idle.text, '');
      if (reference) {
        same(id, 'Calendar Material icon', idle.icon, reference.idle.icon);
        // Ignore floating-point noise from fractional layout coordinates.
        same(id, 'Calendar width/height', [idle.width, idle.height].map(size => size.toFixed(3)), [reference.idle.width, reference.idle.height].map(size => size.toFixed(3)));
        same(id, 'Calendar resting chrome', idle.chrome, reference.idle.chrome);
        same(id, 'Calendar focus style', focused.ring, reference.focused.ring);
      }
      same(id, 'trailing edge', idle.rightInset, 0);
      same(id, 'header alignment', Math.abs(idle.centerOffset ?? Infinity) <= 1, true);
      const actions = {};
      for (const action of ['space', 'click', 'escape']) {
        await page.evaluate('window.closeParityLedger.length = 0');
        if (action === 'click') {
          await page.send('Input.dispatchMouseEvent', { type: 'mousePressed', button: 'left', clickCount: 1, ...point });
          await page.send('Input.dispatchMouseEvent', { type: 'mouseReleased', button: 'left', clickCount: 1, ...point });
        } else await press(page, action === 'space' ? ' ' : 'Escape');
        actions[action] = await page.evaluate('window.closeParityLedger.slice()');
        same(id, `${action} original IPC once`, actions[action], [hide]);
      }
      rows.push({ id, idle, focused, tooltipVisible, actions });
    } catch (error) {
      failures.push({ id, setup: String(error) });
    } finally { await page.close(); }
  }
}
await writeFile(`${output}/report.json`, JSON.stringify({ rows, failures }, null, 2));
console.log(`${rows.length} renderer states captured; ${failures.length} failed assertions. ${output}/report.json`);
if (failures.length) process.exitCode = 1;
