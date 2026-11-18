import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage } from '../custom-themes/cdp.mjs';

const output = process.argv[2] ?? 'test-results/ui-audit-stack-menu';
await mkdir(output, { recursive: true });
const rows = [];
for (const fail of [false, true]) {
  const page = await connectPage('http://127.0.0.1:9295', 'about:blank');
  try {
    await page.send('Emulation.setDeviceMetricsOverride', { width: 440, height: 300, deviceScaleFactor: 1, mobile: false });
    await page.send('Page.navigate', { url: 'http://127.0.0.1:5201/' });
    await page.send('Page.bringToFront');
    const trigger = `[...document.querySelectorAll('[role=menuitem]')].find(e=>e.textContent.includes('Open with'))`;
    const file = `document.querySelector('[data-stack-entry-path$="draft.txt"]')`;
    await page.wait('window.openWithFixture && document.querySelector("[data-stack-entry-path]")');
    await page.evaluate(`document.querySelector('[data-stack-entry-path]').dispatchEvent(new MouseEvent('dblclick',{bubbles:true}))`);
    await page.wait(file);
    await page.evaluate(`${file}.dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:420,clientY:250}))`);
    await page.wait(trigger);
    await page.evaluate(`${trigger}.focus()`);
    const key = async (key, windowsVirtualKeyCode) => {
      for (const type of ['keyDown', 'keyUp']) await page.send('Input.dispatchKeyEvent', { type, key, code: key, windowsVirtualKeyCode });
    };
    await key('ArrowRight', 39);
    await page.wait(`${trigger}.getAttribute('aria-expanded') === 'true' && window.openWithFixture.pending.length`);
    const snapshot = () => page.evaluate(`(() => [...document.querySelectorAll('[role=menu]')].map(menu => ({ text:menu.innerText, focus:document.activeElement.textContent.trim().slice(0,100), items:[...menu.querySelectorAll('[role=menuitem]')].map(item=>({text:item.textContent.trim(),disabled:item.disabled})), bounds:menu.getBoundingClientRect().toJSON(), background:getComputedStyle(menu).backgroundColor })))()`);
    const pending = await snapshot();
    await page.evaluate(fail ? `window.openWithFixture.pending[0].reject(new Error('Fixture association lookup failed'))` : `window.openWithFixture.pending[0].resolve([window.openWithFixture.candidate('editor')])`);
    if (!fail) {
      await page.wait(`document.body.innerText.includes('Apps ready. Reopen to view.') || document.body.innerText.includes('Associated editor')`);
      if (await page.evaluate(`document.body.innerText.includes('Apps ready. Reopen to view.')`)) {
        await key('Escape', 27);
        await key('ArrowRight', 39);
      }
    }
    await page.wait(fail ? `document.body.innerText.includes('Apps unavailable. Choose an app below.')` : `[...document.querySelectorAll('[role=menuitem]')].some(item => item.textContent.includes('Associated editor'))`);
    const settled = await snapshot();
    const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
    await writeFile(`${output}/${fail ? 'error' : 'loaded'}.png`, Buffer.from(data, 'base64'));
    await key('Escape', 27);
    const returned = await page.evaluate(`Boolean(${trigger} && ${trigger}.getAttribute('aria-expanded') === 'false' && ${trigger} === document.activeElement)`);
    rows.push({ fail, pending, settled, returned, fits: settled.every(({bounds:r})=>r.left>=0 && r.top>=0 && r.right<=440.01 && r.bottom<=300.01) });
  } catch (error) { rows.push({ fail, setup: String(error).split('\n')[0] }); }
  finally { await page.close(); }
}
await writeFile(`${output}/report.json`, JSON.stringify(rows,null,2));
if (rows.some(row => row.setup || !row.returned || !row.fits)) process.exitCode = 1;
