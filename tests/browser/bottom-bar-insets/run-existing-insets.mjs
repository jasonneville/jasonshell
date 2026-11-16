import { spawnSync } from 'node:child_process';
import { writeFileSync, mkdirSync } from 'node:fs';
import { join, resolve } from 'node:path';

// Same existing static inset assertions, without Windows shell-shim argv handling.
const executable = process.env.AGENT_BROWSER_EXECUTABLE ?? join(process.env.APPDATA, 'npm/node_modules/agent-browser/bin/agent-browser-win32-x64.exe');
const artifacts = resolve('test-results/bottom-edge-final');
mkdirSync(artifacts, { recursive: true });
const session = 'bottom-edge-existing-insets';
function browser(args) {
  console.log(`existing inset runner: ${args[0]}`);
  const run = spawnSync(executable, ['--session', session, ...args, '--json'], { encoding: 'utf8', timeout: 45000, maxBuffer: 16 * 1024 * 1024 });
  if (run.status !== 0) throw new Error(run.error?.message || run.stderr || run.stdout);
  const reply = JSON.parse(run.stdout);
  if (!reply.success) throw new Error(reply.error);
  return reply.data;
}
let report;
try {
  browser(['open', 'http://127.0.0.1:5191/tests/browser/bottom-bar-insets/index.html']);
  browser(['set', 'media', 'dark']);
  report = browser(['eval', 'window.checkBottomBarInsets()']).result;
  browser(['set', 'media', 'light', 'reduced-motion']);
  for (let index = 0; index < 5; index++) {
    browser(['press', 'Tab']);
    const next = browser(['eval', 'window.checkBottomBarMotionAndFocus()']).result;
    report.total += next.total; report.failed.push(...next.failed); report.results.push(...next.results);
  }
} catch (error) {
  report ??= { total: 0, results: [], failed: [] };
  report.failed.push({ name: 'existing inset runner completes', pass: false, actual: String(error) });
} finally {
  try { browser(['close']); } catch {}
}
writeFileSync(join(artifacts, 'existing-insets-current.json'), JSON.stringify(report, null, 2));
console.log(JSON.stringify({ total: report.total, passed: report.results.filter(r => r.pass).length, failed: report.failed }, null, 2));
process.exitCode = report.failed.length ? 1 : 0;
