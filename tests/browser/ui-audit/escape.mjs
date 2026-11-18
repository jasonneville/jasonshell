import { mkdir, writeFile } from 'node:fs/promises';
import { connectPage, press } from '../custom-themes/cdp.mjs';

const base = process.argv[2] ?? 'http://127.0.0.1:5199';
const output = 'test-results/ui-audit-escape';
await mkdir(output, { recursive: true });
const rows = [];
for (const [surface, label, hide, width, height, ready] of [
  ['calendar-panel', 'Close calendar', 'hide_calendar_panel', 360, 440, 'button[aria-label="Close calendar"]'],
  ['command-panel', 'Close quick commands', 'hide_command_panel', 840, 600, '.command-list-state']
]) {
  const page = await connectPage('http://127.0.0.1:9295', 'about:blank');
  try {
    await page.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
    await page.send('Page.navigate', { url: `${base}/tests/browser/ui-audit/index.html?surface=${surface}` });
    await page.send('Page.bringToFront');
    await page.wait('window.audit');
    await page.wait(`document.querySelector(${JSON.stringify(ready)})`);
    await press(page, 'Tab');
    const focus = await page.evaluate('document.activeElement.outerHTML.slice(0, 240)');
    const { data } = await page.send('Page.captureScreenshot', { format: 'png' });
    await writeFile(`${output}/${surface}.png`, Buffer.from(data, 'base64'));
    await page.evaluate('window.audit.calls.length = 0');
    await press(page, 'Escape');
    const escape = await page.evaluate('window.audit.calls.slice()');
    await page.evaluate(`document.querySelector(${JSON.stringify(`button[aria-label="${label}"]`)}).click()`);
    const click = await page.evaluate(`window.audit.calls.slice(${escape.length})`);
    rows.push({ surface, focus, escape, click, expected: [hide], missing: await page.evaluate('window.audit.missing()') });
  } finally { await page.close(); }
}
await writeFile(`${output}/report.json`, JSON.stringify(rows, null, 2));
console.log(rows);
if (rows.some(row => row.missing.length || [row.escape, row.click].some(actual => JSON.stringify(actual) !== JSON.stringify(row.expected)))) process.exitCode = 1;
