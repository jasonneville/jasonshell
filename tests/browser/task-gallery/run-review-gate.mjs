import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
const session = 'task-gallery-review-gate';
const executable = process.env.AGENT_BROWSER_EXECUTABLE ?? join(process.env.APPDATA, 'npm/node_modules/agent-browser/bin/agent-browser-win32-x64.exe');
const artifacts = resolve('test-results/task-gallery'); mkdirSync(artifacts, { recursive: true });
const results = [];
const check = (name, pass, actual) => results.push({ name, pass: Boolean(pass), actual });
function browser(args) {
  const run = spawnSync(executable, ['--session', session, '--headed', ...args, '--json'], { encoding: 'utf8', timeout: 45000, maxBuffer: 16 * 1024 * 1024 });
  if (run.status !== 0) throw new Error(`${args.join(' ')}: ${run.error?.message || run.stderr || run.stdout}`);
  const reply = JSON.parse(run.stdout); if (!reply.success) throw new Error(reply.error); return reply.data;
}
const evaluate = (code) => browser(['eval', '-b', Buffer.from(code).toString('base64')]).result;
async function prepare(width = 584, many = false) {
  browser(['open', `http://127.0.0.1:4179/?surfaceOnly=1&width=${width}`]);
  browser(['set', 'viewport', String(width), '328']);
  browser(['wait', '--fn', 'Boolean(window.galleryFixture && document.querySelector(".task-gallery-activate"))']);
  evaluate(`(async()=>{const f=window.galleryFixture; f.state.deferSync=false; f.state.failSync=false; f.configure(${width},600,'dark'); await f.open(true,f.manyRows.slice(0,${many ? 30 : 6})); await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));})()`);
  browser(['wait', '--fn', 'window.galleryFixture.state.liveFrames.length > 0']);
}
function rightClick() { browser(['mouse', 'move', '560', '310']); browser(['mouse', 'down', 'right']); browser(['mouse', 'up', 'right']); }
function measurement() {
  return evaluate(`(async()=>{const menu=document.querySelector('[role="menu"]'); if(menu) await Promise.all(menu.getAnimations().map(a=>a.finished.catch(()=>{}))); const host=document.querySelector('#surface').getBoundingClientRect(); const rect=e=>{const r=e.getBoundingClientRect(); return {left:r.left,top:r.top,right:r.right,bottom:r.bottom,width:r.width,height:r.height,inside:r.left>=host.left&&r.top>=host.top&&r.right<=host.right&&r.bottom<=host.bottom&&r.left>=0&&r.top>=0&&r.right<=innerWidth&&r.bottom<=innerHeight};}; return {host:rect(document.querySelector('#surface')),menu:menu?rect(menu):null,items:[...document.querySelectorAll('[role="menuitem"]')].map(e=>({name:e.textContent.trim(),...rect(e)})),live:[...window.galleryFixture.state.liveFrames],alert:document.querySelector('[role="alert"]')?.textContent??''};})()`);
}
try {
  await prepare();
  const before = evaluate('window.galleryFixture.state.liveFrames.slice()');
  evaluate('window.galleryFixture.state.deferSync=true'); rightClick();
  browser(['wait', '--fn', 'window.galleryFixture.pendingSync.some(p=>p.args.frames.length===0)']);
  let measured = measurement();
  check('pending empty-sync receipt preserves acknowledged live overlays', JSON.stringify(measured.live) === JSON.stringify(before), measured);
  check('right-click menu absent until current native clear acknowledgement', measured.menu === null, measured);
  browser(['screenshot', resolve(artifacts, 'review-pending-clear.png')]);
  evaluate(`window.galleryFixture.pendingSync.splice(0).forEach(p=>p.reject(new Error('Native thumbnail cleanup denied'))); window.galleryFixture.state.deferSync=false;`);
  measured = measurement();
  check('rejected clear preserves native registration truth', JSON.stringify(measured.live) === JSON.stringify(before), measured);
  check('rejected clear cannot paint context menu and exposes retry failure', !measured.menu && /(?:clear|cleanup|preview|thumbnail).*(?:retry|try again)/i.test(measured.alert), measured);
  browser(['screenshot', resolve(artifacts, 'review-rejected-clear.png')]);

  for (const action of ['Restore / Switch', 'Minimize']) {
    await prepare(); rightClick();
    browser(['wait', '[role="menu"]']); measured = measurement();
    const refresh = evaluate(`(async()=>{
      const f=window.galleryFixture,menu=document.querySelector('[role="menu"]'),focus=document.activeElement,start=f.ledger.length;
      const backup=()=>['201','204'].every(hwnd=>Boolean(document.querySelector('[data-gallery-hwnd="'+hwnd+'"] .task-gallery-captured-image')));
      const beforeBackup=backup(); await f.snapshot(f.state.open.windows);
      await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
      return {menuSurvives:document.querySelector('[role="menu"]')===menu,focusSurvives:document.activeElement===focus,
        beforeBackup,afterBackup:backup(),live:f.state.liveFrames.slice(),
        reRegistered:f.ledger.slice(start).some(e=>e.name==='sync_task_gallery_thumbnails'&&e.args.args.frames.length>0)};
    })()`);
    check(`same-session refresh preserves real right-click menu and action focus: ${action}`, refresh.menuSurvives && refresh.focusSurvives, refresh);
    check(`menu refresh keeps all native destinations cleared and genuine live-image backups visible: ${action}`,
      refresh.live.length===0 && !refresh.reRegistered && refresh.beforeBackup && refresh.afterBackup, refresh);
    measured = measurement();
    const contained = measured.host.width === 584 && measured.host.height === 328 && measured.menu?.inside && measured.items.length === 2 && measured.items.every(item=>item.inside && item.width>0 && item.height>0);
    check(`right-click 560,310 fully contains real menu and both actions: ${action}`, contained, measured);
    browser(['screenshot', resolve(artifacts, `review-edge-${action === 'Minimize' ? 'minimize' : 'restore'}.png`)]);
    let dispatched = false;
    if (contained) {
      browser(['find','role','menuitem','click','--name',action]);
      dispatched = evaluate(`window.galleryFixture.ledger.some(e=>e.name==='activate_task_gallery_window'&&e.args.args.hwnd==='206'&&e.args.args.minimizeIfActive===${action === 'Minimize'})`);
    }
    check(`contained ${action} is reachable by real browser click`, dispatched);
  }
  for (const width of [584,144]) for (const key of ['ContextMenu','Shift+F10']) {
    await prepare(width,true);
    browser(['focus','[data-gallery-hwnd="201"] .task-gallery-activate']); browser(['press','End']);
    browser(['wait','--fn','window.galleryFixture.measure().tiles.at(-1).closeFullyWithinViewport']);
    browser(['press',key]); browser(['wait','[role="menu"]']); measured=measurement();
    check(`real ${key} after End contains both actions in ${width}x328 native client`, measured.host.width===width && measured.host.height===328 && measured.menu?.inside && measured.items.length===2 && measured.items.every(item=>item.inside), measured);
    browser(['screenshot',resolve(artifacts,`review-key-${width}-${key.replaceAll('+','-')}.png`)]);
  }
} catch(error) { check('review gate runner completed',false,String(error)); }
finally { try { browser(['close']); } catch {} }
const failed=results.filter(row=>!row.pass);
const report={total:results.length,passed:results.length-failed.length,failed,results,evidenceBoundary:'Production DOM/CSS and real browser input, mocked acknowledged native registrations only. No native DWM certification.'};
writeFileSync(resolve(artifacts,'review-gate.json'),JSON.stringify(report,null,2));
const escape=value=>String(value).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
writeFileSync(resolve(artifacts,'review-gate.xml'),`<testsuite name="task-gallery-review-gate" tests="${results.length}" failures="${failed.length}">${results.map(row=>`<testcase name="${escape(row.name)}">${row.pass?'':`<failure message="${escape(JSON.stringify(row.actual))}"/>`}</testcase>`).join('')}</testsuite>`);
writeFileSync(resolve(artifacts,'review-gate.html'),`<!doctype html><meta charset="utf-8"><h1>${report.passed}/${report.total} passed</h1><p>${escape(report.evidenceBoundary)}</p><pre>${escape(JSON.stringify(report,null,2))}</pre>`);
console.log(JSON.stringify({total:report.total,passed:report.passed,failed,artifacts},null,2)); process.exitCode=failed.length?1:0;
