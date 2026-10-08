import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
const session = 'task-gallery-preview-chrome-visible';
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
const evaluate = code => browser(['eval', '-b', Buffer.from(code).toString('base64')]).result;
const style = selector => `(()=>{document.activeElement?.blur(); const s=getComputedStyle(document.querySelector('${selector}')); return {radius:s.borderRadius,background:s.backgroundImage,color:s.backgroundColor,shadow:s.boxShadow,borderColor:s.borderTopColor,borderWidth:s.borderTopWidth};})()`;
const results = [];
try {
  for (const theme of ['dark', 'light']) {
    browser(['open', 'http://127.0.0.1:4180/']);
    browser(['set', 'viewport', '760', '520']);
    browser(['wait', '--fn', 'window.previewReady === true']);
    evaluate(`window.previewFixture.scenario('base-${theme}','captured-image',false,true)`);
    const preview = evaluate(style('.preview-surface'));
    browser(['screenshot', resolve(artifacts, `chrome-single-${theme}.png`)]);
    browser(['open', 'http://127.0.0.1:4179/?surfaceOnly=1']);
    browser(['wait', '--fn', 'Boolean(window.galleryFixture && document.querySelector(".task-gallery-panel"))']);
    evaluate(`(async()=>{const f=window.galleryFixture;f.configure(960,600,'${theme}');await f.open(true,f.rows);await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));})()`);
    const gallery = evaluate(style('.task-gallery-panel'));
    browser(['screenshot', resolve(artifacts, `chrome-gallery-${theme}.png`)]);
    const checks = {
      rounding: preview.radius === '8px' && gallery.radius === preview.radius,
      gradient: preview.background !== 'none' && gallery.background === preview.background,
      surface: gallery.color === preview.color,
      bevel: preview.shadow !== 'none' && gallery.shadow.endsWith(preview.shadow),
      // Fractional DPR can quantize a declared 1px border to 0.8 CSS pixels.
      // Compare painted keyline color and a present positive border, not that
      // platform-dependent computed width; gallery uses inset paint for geometry.
      keyline: Number.parseFloat(preview.borderWidth) > 0 && gallery.shadow.includes(preview.borderColor)
    };
    results.push({ name: `${theme}: actual production gallery vs single-preview chrome`, pass: Object.values(checks).every(Boolean), checks, gallery, preview });
  }
} catch (error) { results.push({ name: 'visible chrome comparison completed', pass: false, actual: String(error) }); }
finally { try { browser(['close']); } catch {} }
const report = { results, failed: results.filter(result => !result.pass),
  evidenceBoundary: 'Two production components mounted in existing mocked IPC fixtures, actual computed browser paint and screenshots. No copied chrome markup/CSS; no DWM/native shell certification.' };
writeFileSync(resolve(artifacts, 'chrome-comparison.json'), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
process.exitCode = report.failed.length ? 1 : 0;
