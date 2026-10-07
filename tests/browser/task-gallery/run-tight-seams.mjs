import { spawnSync } from 'node:child_process';
const session = 'task-gallery-tight-seams';
function browser(args) {
  const result = spawnSync('agent-browser', ['--session', session, ...args, '--json'], { encoding: 'utf8', shell: process.platform === 'win32' });
  if (result.status !== 0) throw new Error(result.stderr || result.stdout);
  const payload = JSON.parse(result.stdout);
  if (!payload.success) throw new Error(payload.error);
  return payload.data;
}
browser(['open', 'http://127.0.0.1:4179/']);
browser(['wait', '.task-gallery-tile']);
const report = browser(['eval', 'window.checkGalleryTightSeams()']).result;
console.log(JSON.stringify(report, null, 2));
if (!report || !Array.isArray(report.failed)) throw new Error('Missing gallery browser acceptance results');
process.exitCode = report.failed.length ? 1 : 0;
