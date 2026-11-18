import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';

const output = process.argv[2] ?? 'test-results/ui-audit-command-nested';
await mkdir(output, { recursive: true });
const page = await connectPage('http://127.0.0.1:9295', 'about:blank');
const rows = {};
try {
  await page.send('Emulation.setDeviceMetricsOverride', { width: 460, height: 420, deviceScaleFactor: 1, mobile: false });
  await page.send('Page.navigate', { url: 'http://127.0.0.1:5199/tests/browser/ui-audit/index.html?surface=command-panel&commands' });
  await page.send('Page.bringToFront');
  const edit = `document.querySelector('[aria-label="Edit Interactive fixture"]')`;
  const menu = `document.querySelector('.command-context-menu')`;
  const dialog = `document.querySelector('[role=alertdialog]')`;
  await page.wait(`window.audit && ${edit}`);
  await page.evaluate(`${edit}.focus()`);
  for (const type of ['keyDown', 'keyUp']) await page.send('Input.dispatchKeyEvent', { type, key: 'ContextMenu', code: 'ContextMenu', windowsVirtualKeyCode: 93 });
  async function capture(key, selector) {
    await page.wait(selector);
    rows[key] = await page.evaluate(`(() => { const n = ${selector}, c = getComputedStyle(n), r = n.getBoundingClientRect(); return { text:n.innerText, focus:document.activeElement.textContent.trim().slice(0,100), bounds:r.toJSON(), fits:r.left>=0 && r.top>=0 && r.right<=innerWidth && r.bottom<=innerHeight, color:c.color, background:c.backgroundColor }; })()`);
    const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
    await writeFile(`${output}/${key}.png`, Buffer.from(data, 'base64'));
  }
  await page.wait(`document.activeElement.textContent.trim() === 'View output history'`);
  await capture('menu', menu);
  const scroll = () => page.evaluate(`(() => { const values = [scrollX, scrollY]; for (let n = ${edit}; n; n = n.parentElement) values.push(n.scrollLeft,n.scrollTop); return values; })()`);
  const beforeEscape = await scroll();
  await press(page, 'Escape');
  rows.menuScroll = { before: beforeEscape, after: await scroll() };
  rows.menuAfter = await page.evaluate(`({ open:Boolean(${menu}), editFocused:${edit}===document.activeElement, focusTag: document.activeElement.tagName })`);
  await page.evaluate(`document.querySelector('[aria-label="Delete Interactive fixture"]').click()`);
  await capture('confirmation', dialog);
  await press(page, 'Escape');
  rows.confirmationAfter = await page.evaluate(`({ open:Boolean(${dialog}), entryKept:Boolean(${edit}), focus:document.activeElement.getAttribute('aria-label') })`);
  await page.evaluate(`window.audit.emit('quick-command:run-updated', { runId:'fixture-run', commandId:'interactive', processId:101, sequence:1, atEpochMs:Date.now(), kind:'text', body:'Fixture prompt', requestId:'fixture-request', prompt:'Continue?', pending:true, maxLength:24, secret:false, redacted:false })`);
  const input = `document.querySelector('.command-input-panel textarea')`;
  await page.wait(input);
  await page.evaluate(`${input}.focus()`);
  await page.send('Input.insertText', { text:'fixture draft' });
  await page.wait(`${input}.value === 'fixture draft'`);
  await capture('input', `document.querySelector('.command-input-panel')`);
  await press(page, 'Escape');
  rows.inputAfter = await page.evaluate(`({ focused:${input}===document.activeElement, value:${input}?.value, open:Boolean(${input}), deleteDisabled:document.querySelector('[aria-label="Delete Interactive fixture"]').disabled })`);
  rows.calls = await page.evaluate('window.audit.calls.slice()');
  assert.ok(rows.menu.fits && !rows.menuAfter.open && rows.confirmation.fits && !rows.confirmationAfter.open && rows.confirmationAfter.entryKept && rows.confirmationAfter.focus === 'Delete Interactive fixture');
  assert.ok(rows.inputAfter.open && rows.inputAfter.focused && rows.inputAfter.value === '' && rows.inputAfter.deleteDisabled);
  assert.ok(!rows.calls.some(call => /^(hide_command_panel|send_quick_command_input|save_quick_commands_settings|run_quick_command)$/.test(call)));
  assert.ok(rows.menuAfter.editFocused, 'Context menu Escape must return focus to the originating command row');
  assert.deepEqual(rows.menuScroll.after, rows.menuScroll.before, 'Restoring menu origin must not scroll');
} catch (error) { rows.failure = String(error).split('\n')[0]; process.exitCode = 1; }
finally { await writeFile(`${output}/report.json`, JSON.stringify(rows,null,2)); await page.close(); }
