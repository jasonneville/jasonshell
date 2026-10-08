import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

// User-consented headed mock fixture only. Do not remove --headed.
const session = 'task-gallery-bottom-bar-alignment-visible';
const executable = process.env.AGENT_BROWSER_EXECUTABLE ?? join(process.env.APPDATA, 'npm/node_modules/agent-browser/bin/agent-browser-win32-x64.exe');
const artifacts = resolve('test-results/task-gallery');
mkdirSync(artifacts, { recursive: true });
function browser(args) {
  const run = spawnSync(executable, ['--session', session, '--headed', ...args, '--json'], { encoding: 'utf8', timeout: 45000, maxBuffer: 16 * 1024 * 1024 });
  if (run.status !== 0) throw new Error(run.error?.message || run.stderr || run.stdout);
  const reply = JSON.parse(run.stdout);
  if (!reply.success) throw new Error(reply.error);
  return reply.data;
}
let report;
try {
  browser(['open', 'http://127.0.0.1:4179/']);
  browser(['set', 'viewport', '640', '600']);
  browser(['wait', '--fn', 'Boolean(window.galleryFixture && document.querySelector(".task-button"))']);
  const code = readFileSync(new URL('./bottom-bar-alignment-checks.js', import.meta.url), 'utf8');
  report = browser(['eval', '-b', Buffer.from(code).toString('base64')]).result;
  browser(['screenshot', resolve(artifacts, 'bottom-bar-alignment-red.png')]);
} catch (error) {
  report = { failed: [{ name: 'visible alignment runner completed', pass: false, actual: String(error) }], results: [] };
} finally {
  try { browser(['close']); } catch {}
}
writeFileSync(resolve(artifacts, 'bottom-bar-alignment.json'), JSON.stringify(report, null, 2));
console.log(JSON.stringify({ total: report.results.length, passed: report.results.filter(result => result.pass).length,
  failed: report.failed.map(result => result.name), artifacts }, null, 2));
process.exitCode = report.failed.length ? 1 : 0;
