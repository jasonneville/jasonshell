import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
const session = 'task-gallery-ribbon-matrix';
// Use preexisting native Windows CLI directly: shell:true loses semantic names
// containing spaces and JS quoting. Override supports nonstandard installations.
const executable = process.env.AGENT_BROWSER_EXECUTABLE ?? (process.platform === 'win32'
  ? join(process.env.APPDATA ?? '', 'npm/node_modules/agent-browser/bin/agent-browser-win32-x64.exe')
  : 'agent-browser');
const artifacts = resolve('test-results/task-gallery');
mkdirSync(artifacts, { recursive: true });
function browser(args) {
  const result = spawnSync(executable, ['--session', session, '--headed', ...args, '--json'], { encoding: 'utf8', shell: false, maxBuffer: 16 * 1024 * 1024, timeout: 45000 });
  if (result.status !== 0) throw new Error(`${args[0]} ${args[0] === 'eval' ? '(base64)' : args.slice(1).join(' ')}: ${result.error?.message || result.stderr || result.stdout}`);
  const payload = JSON.parse(result.stdout);
  if (!payload.success) throw new Error(payload.error);
  return payload.data;
}
const evaluate = code => browser(['eval', '-b', Buffer.from(code).toString('base64')]).result;
function focusNamed(name) {
  const snapshot = browser(['snapshot', '-i']);
  const entry = Object.entries(snapshot.refs ?? {}).find(([, node]) => node.role === 'button' && node.name === name);
  if (!entry) throw new Error(`Missing semantic focus target ${name}; snapshot: ${snapshot.snapshot}; ledger: ${JSON.stringify(evaluate('window.galleryFixture.ledger.slice(-15)'))}`);
  browser(['focus', `@${entry[0]}`]);
}
const results = [];
const check = (name, pass, actual) => results.push({ name, pass: Boolean(pass), actual });
try {
  // Open first: this CLI version's cold set-viewport can stall daemon startup.
  browser(['open', 'http://127.0.0.1:4179/']);
  browser(['set', 'viewport', '1280', '1000']);
  browser(['wait', '--fn', 'Boolean(window.galleryFixture && document.querySelector(".task-gallery-tile"))']);
  const report = evaluate('window.checkGalleryTightSeams()');
  if (!report || !Array.isArray(report.results)) throw new Error('Missing gallery browser acceptance results');
  results.push(...report.results);
  browser(['screenshot', resolve(artifacts, 'journey.png')]);

  // Browser-generated pointer and keyboard journey complements synthetic DOM
  // events in the matrix. Production BottomBar and gallery both stay mounted.
  evaluate(`(async () => {
    const f = window.galleryFixture;
    await f.publish('task-gallery:closed', { nonce: f.state.open.nonce });
    f.configure(960, 600, 'dark'); await f.bottomSnapshot(f.rows); f.ledger.length = 0;
  })()`);
  browser(['find', 'role', 'button', 'click', '--name', 'Fixture Editor']);
  browser(['wait', '.task-gallery-activate']);
  browser(['hover', 'button[aria-label="Activate Alpha, Fixture Editor, active"]']);
  check('real hover keeps inline previews without separate popup', evaluate('window.galleryFixture.ledger.filter(e => e.name === "show_task_gallery_window_preview").length') === 0);
  // Installed CLI advertises find-role focus but returns Unknown subaction.
  // Exact aria-label remains a semantic selector for its supported focus command.
  focusNamed('Activate Alpha, Fixture Editor, active');
  browser(['press', 'Tab']);
  check('real Tab reaches sibling Close', evaluate('document.activeElement?.getAttribute("aria-label")') === 'Close Alpha');
  browser(['press', 'Enter']);
  browser(['wait', '--fn', 'window.galleryFixture.pending.length === 1']);
  const close = evaluate(`(() => {
    const f = window.galleryFixture;
    const closes = f.ledger.filter(e => e.name === 'close_task_gallery_window');
    return { requests: closes.map(e => e.args.args), focus: document.activeElement?.getAttribute('aria-label'),
      busy: document.activeElement?.getAttribute('aria-busy'), activated: f.ledger.some(e => e.name === 'activate_task_gallery_window') };
  })()`);
  check('real Enter on Close never activates, retains busy focus', close.requests.length === 1 && close.requests[0].hwnd === '101' && !close.activated && close.focus === 'Close Alpha' && close.busy === 'true', close);
  evaluate('window.galleryFixture.settle()');
  browser(['wait', '--fn', 'document.querySelector(".task-gallery-close")?.getAttribute("aria-busy") === "false"']);
  check('real Close acceptance retains cards', evaluate('document.querySelectorAll("[data-gallery-hwnd]").length') === 3);
  browser(['screenshot', resolve(artifacts, 'keyboard-close.png')]);
  browser(['press', 'Space']);
  browser(['wait', '--fn', 'window.galleryFixture.pending.length === 1']);
  check('real Space on Close dispatches sibling action without activation', evaluate('window.galleryFixture.ledger.filter(e => e.name === "close_task_gallery_window").length === 2 && !window.galleryFixture.ledger.some(e => e.name === "activate_task_gallery_window")'));
  evaluate('window.galleryFixture.settle()');
  focusNamed('Activate Beta, Fixture Editor');
  browser(['press', 'Enter']);
  browser(['wait', '--fn', 'window.galleryFixture.ledger.some(e => e.name === "activate_task_gallery_window")']);
  check('real Enter activation dispatches exact sibling HWND', evaluate('window.galleryFixture.ledger.filter(e => e.name === "activate_task_gallery_window").at(-1).args.args.hwnd') === '102');

  // No duplicated full matrix: just the distinct reduced-motion claim.
  browser(['set', 'media', 'light', 'reduced-motion']);
  evaluate(`(async () => { const f = window.galleryFixture; f.configure(960, 600, 'light'); await f.open(true, f.manyRows.slice(0, 6)); })()`);
  browser(['wait', '--fn', 'document.querySelectorAll("[data-gallery-hwnd]").length === 6']);
  const motion = evaluate(`({ reduced: matchMedia('(prefers-reduced-motion: reduce)').matches,
    running: document.querySelector('.task-gallery-panel').getAnimations({subtree:true}).filter(a => a.playState === 'running').length,
    close: [...document.querySelectorAll('.task-gallery-close')].map(e => ({width:e.getBoundingClientRect().width,height:e.getBoundingClientRect().height})) })`);
  check('reduced-motion has no running gallery animations, preserves target geometry', motion.reduced && motion.running === 0 && motion.close.every(r => r.width >= 24 && r.height >= 24), motion);
  browser(['screenshot', resolve(artifacts, 'light-reduced-motion-matrix.png')]);
  evaluate('(async () => { await window.galleryFixture.snapshot(window.galleryFixture.manyRows); })()');
  browser(['wait', '--fn', 'document.querySelectorAll("[data-gallery-hwnd]").length === 30']);
  focusNamed('Activate Window 1 — many-window long title, Fixture Editor, active');
  browser(['press', 'End']);
  browser(['wait', '--fn', 'window.galleryFixture.measure().tiles.at(-1).closeFullyWithinViewport']);
  check('real End scrolls last Close pair visible', evaluate('window.galleryFixture.measure().activeElement')?.startsWith('Activate Window 30'));
  browser(['screenshot', resolve(artifacts, 'scroll-end.png')]);
} catch (error) {
  check('browser acceptance runner completed', false, String(error));
  try { browser(['screenshot', resolve(artifacts, 'failure.png')]); } catch { /* Report original blocker. */ }
} finally {
  try { browser(['close']); } catch { /* Does not mask acceptance results. */ }
}
const failed = results.filter(result => !result.pass);
const report = { total: results.length, passed: results.length - failed.length, failed, results };
writeFileSync(resolve(artifacts, 'acceptance.json'), JSON.stringify(report, null, 2));
const escape = value => String(value).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
writeFileSync(resolve(artifacts, 'acceptance.xml'), `<testsuite name="task-gallery-mocked-browser" tests="${results.length}" failures="${failed.length}">${results.map(result => `<testcase name="${escape(result.name)}">${result.pass ? '' : `<failure message="${escape(JSON.stringify(result.actual))}"/>`}</testcase>`).join('')}</testsuite>`);
writeFileSync(resolve(artifacts, 'acceptance.html'), `<!doctype html><meta charset="utf-8"><title>Gallery DOM/IPC acceptance</title><h1>${report.passed}/${report.total} passed</h1><p>Mocked native IPC; no live DWM/Win32 certification.</p><pre>${escape(JSON.stringify(report, null, 2))}</pre>`);
console.log(JSON.stringify({ total: report.total, passed: report.passed, failed, artifacts }, null, 2));
process.exitCode = failed.length ? 1 : 0;
