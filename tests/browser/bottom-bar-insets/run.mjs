import { spawnSync } from 'node:child_process';
// Requires the safe allowlisted server in a separate terminal. No dependency installation.
const session = 'bottom-bar-insets';
function browser(args) {
  const result = spawnSync('agent-browser', ['--session', session, ...args, '--json'], { encoding: 'utf8', shell: process.platform === 'win32' });
  if (result.status !== 0) throw new Error(result.stderr || result.stdout);
  const payload = JSON.parse(result.stdout);
  if (!payload.success) throw new Error(payload.error);
  return payload.data;
}
browser(['open', 'http://127.0.0.1:5191/tests/browser/bottom-bar-insets/index.html']);
browser(['set', 'media', 'dark']);
const data = browser(['eval', 'window.checkBottomBarInsets()']);
const report = data.result;
browser(['set', 'media', 'light', 'reduced-motion']);
for (let index = 0; index < 5; index++) {
  browser(['press', 'Tab']);
  const motionFocus = browser(['eval', 'window.checkBottomBarMotionAndFocus()']).result;
  report.total += motionFocus.total;
  report.failed.push(...motionFocus.failed);
  report.results.push(...motionFocus.results);
}
console.log(JSON.stringify(report, null, 2));
if (!report || !Array.isArray(report.failed)) throw new Error('Missing browser acceptance results');
process.exitCode = report.failed.length ? 1 : 0;
