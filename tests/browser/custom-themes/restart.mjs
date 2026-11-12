import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { connectPage, click, input } from './cdp.mjs';

// Isolated ordinary Chrome profile only: never Tauri, AppBars or Windows hooks.
const chrome = process.argv[2];
if (!chrome) throw new Error('Usage: node tests/browser/custom-themes/restart.mjs PATH_TO_EXISTING_CHROME');
const profile = await mkdtemp('C:/Users/jnev1/AppData/Local/Temp/opencode/theme-restart-');
let child;
let exited;
async function launch() {
  await rm(`${profile}/DevToolsActivePort`, { force: true });
  child = spawn(chrome, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`,
    '--no-first-run', '--no-default-browser-check', '--disable-background-networking', 'about:blank'], { stdio: 'ignore' });
  exited = new Promise(resolve => child.once('exit', resolve));
  let endpoint;
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    try {
      const port = (await readFile(`${profile}/DevToolsActivePort`, 'utf8')).split(/\r?\n/)[0];
      if (port) { endpoint = `http://127.0.0.1:${port}`; break; }
    } catch { /* Browser has not written its bounded startup affordance yet. */ }
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  if (!endpoint) throw new Error('Isolated Chrome did not expose CDP within 15s');
  const page = await connectPage(endpoint, 'http://127.0.0.1:4186/?surface=settings');
  await page.wait('!!window.themeFixture');
  return page;
}
async function stop(page) {
  await page.send('Browser.close').catch(() => {});
  page.disconnect();
  await Promise.race([exited, new Promise((_, reject) => setTimeout(() => reject(new Error('Chrome did not exit')), 10_000))]);
}
try {
  const first = await launch();
  await click(first, 'Create copy');
  await input(first, 'Theme name', 'Restart saved');
  await input(first, 'Accent hex', '#13579b');
  await click(first, 'Save & apply');
  await first.wait('themeFixture.snapshot().accent === "rgb(19, 87, 155)"');
  const saved = await first.evaluate('themeFixture.snapshot().storage');
  await click(first, 'Edit theme'); await input(first, 'Accent hex', '#abcdef');
  await first.wait('themeFixture.snapshot().accent === "rgb(171, 205, 239)"');
  assert.deepEqual(await first.evaluate('themeFixture.snapshot().storage'), saved);
  await stop(first);
  const second = await launch();
  const restored = await second.evaluate('themeFixture.snapshot()');
  assert.equal(restored.accent, 'rgb(19, 87, 155)');
  assert.match(restored.bootstrap.accent, /13579b/i);
  assert.deepEqual(restored.storage, saved);
  assert.equal(await second.evaluate('!!document.querySelector(".theme-name input")'), false);
  const report = { result: 'PASS', claim: 'real browser-process restart retains saved custom only; all storage keys unchanged by unsaved preview', bootstrap: restored.bootstrap };
  console.log(JSON.stringify(report));
  await writeFile('C:/Users/jnev1/AppData/Local/Temp/opencode/custom-theme-restart-report.json', JSON.stringify(report, null, 2));
  await stop(second);
} finally {
  if (child && child.exitCode === null) child.kill();
  await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
}
