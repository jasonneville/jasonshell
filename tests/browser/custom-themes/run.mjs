import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { connectPage, click, input, press, selectTheme } from './cdp.mjs';

const endpoint = process.argv[2];
if (!endpoint) throw new Error('Usage: node tests/browser/custom-themes/run.mjs http://127.0.0.1:CDP_PORT');
const base = 'http://127.0.0.1:4186/';
const pages = [];
const report = { checks: [], failures: [], screenshots: [], equivalence: null };
async function page(surface) {
  const result = await connectPage(endpoint, `${base}?surface=${surface}`);
  pages.push(result);
  await result.wait('!!window.themeFixture');
  return result;
}
const snap = p => p.evaluate('themeFixture.snapshot()');
async function check(name, run) {
  if (await run() === false) { report.failures.push(name); console.error(`FAIL ${name}`); }
  else { report.checks.push(name); console.log(`PASS ${name}`); }
}
async function color(p, value) { await p.wait(`themeFixture.snapshot().accent === ${JSON.stringify(value)}`); }
async function newDraft(p, name, accent) {
  await click(p, 'Create copy');
  await p.wait(`!!document.querySelector('input[aria-label="Accent hex"]')`);
  await input(p, 'Theme name', name);
  await input(p, 'Accent hex', accent);
}
async function screenshot(p, name) {
  const file = `C:/Users/jnev1/AppData/Local/Temp/opencode/${name}.png`;
  const { data } = await p.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
  await writeFile(file, Buffer.from(data, 'base64')); report.screenshots.push(file);
}

try {
  const owner = await page('settings');
  await owner.evaluate('localStorage.clear(); location.reload()');
  await owner.wait('!!window.themeFixture');
  const observer = await page('observer');
  await check('all 34 clones preserve all computed colors/composite geometry under default/strong/reduced preferences', async () => {
    report.equivalence = await observer.evaluate('themeFixture.cloneEquivalence()');
    // Keep the remaining critical journeys runnable when this independent
    // regression probe fails; final exit status below still fails the suite.
    if (report.equivalence.failures.length) console.error(`FAIL computed clone equivalence: ${report.equivalence.failures.length} mismatches`);
    assert.equal(report.equivalence.presets, 34);
    assert.equal(report.equivalence.tokens, 38);
    return report.equivalence.failures.length === 0;
  });
  const original = await snap(observer);
  await check('valid preview propagates to separate real BroadcastChannel page without persistence', async () => {
    await newDraft(owner, 'Browser custom', '#123456');
    await color(observer, 'rgb(18, 52, 86)');
    assert.equal((await snap(observer)).saved, null);
    assert.deepEqual((await snap(observer)).storage, original.storage);
  });
  await screenshot(owner, 'custom-theme-editor-preview');
  await check('hide/show retains draft and invalid input retains last valid preview', async () => {
    await click(owner, 'Hide mounted surface');
    await click(owner, 'Show mounted surface');
    assert.equal(await owner.evaluate('document.querySelector(".theme-name input").value'), 'Browser custom');
    await input(owner, 'Accent hex', '#oops');
    assert.equal(await owner.evaluate(`document.querySelector('input[aria-label="Accent hex"]').getAttribute('aria-invalid')`), 'true');
    assert.equal((await snap(observer)).accent, 'rgb(18, 52, 86)');
    assert.deepEqual((await snap(owner)).storage, original.storage);
    await input(owner, 'Accent hex', '#123456');
  });
  const late = await page('observer');
  await check('late renderer joins current in-memory preview', async () => { await color(late, 'rgb(18, 52, 86)'); });
  await check('Cancel restores saved appearance everywhere and persists no draft', async () => {
    await click(owner, 'Cancel');
    await color(observer, original.accent); await color(late, original.accent);
    assert.equal((await snap(owner)).saved, null);
  });
  await check('320/360px color rows fit without alpha/reset overlap or viewport overflow', async () => {
    await newDraft(owner, 'Responsive fields', '#123456');
    for (const width of [360, 320]) {
      await owner.send('Emulation.setDeviceMetricsOverride', { width, height: 900, deviceScaleFactor: 1, mobile: false });
      const geometry = await owner.evaluate(`(() => { const overlap = (a,b) => a.left < b.right-.5 && a.right > b.left+.5 && a.top < b.bottom-.5 && a.bottom > b.top+.5; const fields=[...document.querySelectorAll('.theme-color-field')].filter(x=>x.getBoundingClientRect().height); return { viewport:innerWidth, page:document.documentElement.scrollWidth, bad:fields.flatMap(field=>{ const alpha=field.querySelector('.theme-alpha').getBoundingClientRect(), reset=field.querySelector('button').getBoundingClientRect(), input=field.querySelector('.theme-alpha input').getBoundingClientRect(), row=field.getBoundingClientRect(); return overlap(alpha,reset)||input.right>row.right+.5||reset.right>row.right+.5 ? [field.querySelector('strong').textContent] : []; }) }; })()`);
      assert.ok(geometry.page <= width, JSON.stringify(geometry));
      assert.deepEqual(geometry.bad, [], JSON.stringify(geometry));
      await screenshot(owner, `custom-theme-editor-${width}px`);
    }
    await owner.send('Emulation.clearDeviceMetricsOverride');
    await click(owner, 'Cancel');
  });
  await check('keyboard discard confirmation contains focus; Escape returns stable Theme button without hiding Settings', async () => {
    await newDraft(owner, 'Keyboard copy', '#123456');
    await owner.evaluate(`document.querySelector('[role="combobox"][aria-label="Theme"]').focus()`);
    for (const key of [' ', 'Home', 'ArrowDown', 'Enter']) await press(owner, key);
    await owner.wait('!!document.querySelector("[role=alertdialog]")');
    await owner.wait('document.activeElement?.textContent.trim() === "Keep editing"');
    await press(owner, 'Tab');
    assert.equal(await owner.evaluate('document.activeElement.textContent.trim()'), 'Discard & select');
    await press(owner, 'Escape');
    await owner.wait('!document.querySelector("[role=alertdialog]")');
    const focus = await owner.evaluate('({role:document.activeElement.getAttribute("role"),label:document.activeElement.getAttribute("aria-label"),tag:document.activeElement.tagName,hidden:document.querySelector("#app").hidden})');
    assert.deepEqual(focus, { role: 'combobox', label: 'Theme', tag: 'BUTTON', hidden: false });
    assert.deepEqual((await snap(owner)).storage, original.storage);
    await click(owner, 'Cancel');
  });
  await check('Control Plane selection requests Settings confirmation; decline retains draft and confirm converges both pickers', async () => {
    const control = await page('control');
    await newDraft(owner, 'Remote confirmation', '#123456');
    const before = (await snap(owner)).storage;
    await selectTheme(control, 'Nord');
    await owner.wait('!!document.querySelector("[role=alertdialog]")');
    await click(owner, 'Keep editing', 'document.querySelector("[role=alertdialog]")');
    assert.deepEqual((await snap(owner)).storage, before);
    assert.equal((await snap(observer)).accent, 'rgb(18, 52, 86)');
    await selectTheme(control, 'Nord');
    await owner.wait('!!document.querySelector("[role=alertdialog]")');
    await click(owner, 'Discard & select', 'document.querySelector("[role=alertdialog]")');
    await owner.wait('!document.querySelector(".theme-name input")');
    await observer.wait('themeFixture.snapshot().theme === "nord"');
    await control.wait('document.querySelector("[role=combobox][aria-label=Theme]").textContent.includes("Nord")');
    await selectTheme(owner, 'Base Dark');
    await color(observer, original.accent);
  });
  await check('Save commits collection plus selection and fresh page hydrates saved custom before mount', async () => {
    await newDraft(owner, 'Saved browser custom', '#13579b');
    await click(owner, 'Save & apply');
    await color(observer, 'rgb(19, 87, 155)');
    const state = JSON.parse((await snap(observer)).saved);
    assert.equal(state.customThemes.length, 1);
    assert.equal(state.selectedId, state.customThemes[0].id);
    const fresh = await page('observer');
    await color(fresh, 'rgb(19, 87, 155)');
    assert.match((await snap(fresh)).bootstrap.accent, /13579b/i);
  });
  await check('renderer restart discards unsaved draft and restores saved-only custom colors', async () => {
    await click(owner, 'Edit theme');
    await input(owner, 'Accent hex', '#abcdef');
    await color(observer, 'rgb(171, 205, 239)');
    await owner.send('Page.reload');
    await owner.wait('!!window.themeFixture && !document.querySelector(".theme-name input")');
    await color(owner, 'rgb(19, 87, 155)'); await color(observer, 'rgb(19, 87, 155)');
    assert.equal(JSON.parse((await snap(owner)).saved).customThemes.length, 1);
  });
  await check('quota failure retains draft/preview, emits truthful error and never changes canonical bytes', async () => {
    const before = (await snap(owner)).saved;
    await click(owner, 'Edit theme');
    await input(owner, 'Accent hex', '#2468ac');
    await color(observer, 'rgb(36, 104, 172)');
    await owner.evaluate(`window.originalStorageSet = Storage.prototype.setItem; Storage.prototype.setItem = function(key,value) { if(key === 'jasonshell.themes.v1') throw new DOMException('quota','QuotaExceededError'); return originalStorageSet.call(this,key,value); }`);
    try {
      await click(owner, 'Save & apply');
      await owner.wait('!!document.querySelector(".theme-editor [role=alert]")');
      assert.equal((await snap(owner)).saved, before);
      assert.ok(await owner.evaluate('!!document.querySelector(".theme-name input")'));
      assert.equal((await snap(observer)).accent, 'rgb(36, 104, 172)');
    } finally { await owner.evaluate('Storage.prototype.setItem = originalStorageSet; delete window.originalStorageSet'); }
    await click(owner, 'Cancel'); await color(observer, 'rgb(19, 87, 155)');
  });
  await check('strong focus/reduced transparency remain authoritative during custom preview; reset uses original palette', async () => {
    await click(owner, 'Strong focus rings'); await click(owner, 'Reduce transparency');
    await click(owner, 'Edit theme'); await input(owner, 'Accent hex', '#123456');
    await owner.wait('themeFixture.snapshot().focus.includes("4px")');
    const actual = await snap(owner);
    assert.equal(actual.overlay, actual.control);
    await click(owner, 'Reset colors');
    // Editing a saved custom retains its original source-copy baseline, not
    // the colors from its most recent Save.
    await color(owner, original.accent);
    const reset = await snap(owner);
    assert.ok(reset.focus.includes('4px')); assert.equal(reset.overlay, reset.control);
    await click(owner, 'Cancel');
  });
  await screenshot(owner, 'custom-theme-settings-saved');
  await check('presentation Reset clears preview/default preferences but preserves saved custom collection', async () => {
    const collection = JSON.parse((await snap(owner)).saved).customThemes;
    await click(owner, 'Edit theme'); await input(owner, 'Accent hex', '#123456');
    await click(owner, 'Reset');
    await owner.wait('!document.querySelector(".theme-name input")');
    await color(observer, original.accent);
    const state = JSON.parse((await snap(owner)).saved);
    assert.equal(state.selectedId, 'base-dark'); assert.deepEqual(state.customThemes, collection);
    const preferences = JSON.parse((await snap(owner)).storage['jasonshell.uiPreferences']);
    assert.equal(preferences.strongFocusRing, false); assert.equal(preferences.reducedTransparency, false);
    await selectTheme(owner, 'Saved browser custom');
    await color(observer, 'rgb(19, 87, 155)');
  });
  await check('active delete Cancel is byte-identical; confirmation atomically restores Base Dark across pages', async () => {
    const before = (await snap(owner)).saved;
    await click(owner, 'Delete theme');
    await owner.wait('!!document.querySelector("[role=alertdialog]")');
    await click(owner, 'Cancel', 'document.querySelector("[role=alertdialog]")');
    assert.equal((await snap(owner)).saved, before);
    await click(owner, 'Delete theme');
    await click(owner, 'Delete theme', 'document.querySelector("[role=alertdialog]")');
    await observer.wait('JSON.parse(themeFixture.snapshot().saved).selectedId === "base-dark"');
    assert.deepEqual(JSON.parse((await snap(observer)).saved).customThemes, []);
    await color(observer, original.accent); await color(late, original.accent);
  });
  await check('independent recovery UI remains readable under same-color text/background custom preview', async () => {
    await newDraft(owner, 'Hostile but valid', '#000000');
    await input(owner, 'Background hex', '#000000'); await input(owner, 'Text hex', '#000000');
    const recovery = await owner.evaluate(`(() => { const editor = document.querySelector('.theme-editor'); const cancel = [...editor.querySelectorAll('button')].find(x=>x.textContent.trim()==='Cancel'); return {background:getComputedStyle(editor).backgroundColor,text:getComputedStyle(cancel).color}; })()`);
    assert.notEqual(recovery.background, recovery.text);
    assert.notEqual(recovery.text, 'rgb(0, 0, 0)');
    await screenshot(owner, 'custom-theme-recovery-hostile');
    await click(owner, 'Cancel');
  });
  await check('owner disappears without end packet: visible observer probes twice and restores saved colors', async () => {
    const doomed = await page('settings');
    await newDraft(doomed, 'Lost owner', '#345678');
    await color(observer, 'rgb(52, 86, 120)');
    // Simulate transport loss at teardown, not normal pagehide disposal. No
    // production state seam: only this renderer's outgoing end packet is lost.
    await doomed.evaluate(`window.actualPost = BroadcastChannel.prototype.postMessage; BroadcastChannel.prototype.postMessage = function(packet) { if(packet?.type !== 'end') return actualPost.call(this,packet); }`);
    await doomed.close();
    await observer.send('Page.bringToFront');
    await observer.evaluate('themeFixture.queryPresence()');
    await observer.wait('!themeFixture.snapshot().custom', 40_000);
    assert.equal((await snap(observer)).accent, original.accent);
  });
  assert.ok(pages.every(p => p.errors.length === 0), JSON.stringify(pages.flatMap(p => p.errors)));
  console.log(JSON.stringify(report, null, 2));
  if (report.equivalence.failures.length) process.exitCode = 1;
} finally {
  await writeFile('C:/Users/jnev1/AppData/Local/Temp/opencode/custom-theme-browser-report.json', JSON.stringify(report, null, 2));
  for (const p of pages.reverse()) await p.close().catch(() => {});
}
