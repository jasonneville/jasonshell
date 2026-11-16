import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

// Browser-only, production-mounted IPC fixture. Never starts Tauri/native shell.
const baseline = process.argv.includes('--baseline');
const statesOnly = process.argv.includes('--states-only');
const overflowOnly = process.argv.includes('--overflow-only');
const option = name => process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : undefined;
const artifacts = resolve(option('--artifacts') ?? 'test-results/bottom-edge');
mkdirSync(artifacts, { recursive: true });
const executable = process.env.AGENT_BROWSER_EXECUTABLE ?? join(process.env.APPDATA, 'npm/node_modules/agent-browser/bin/agent-browser-win32-x64.exe');
const session = option('--session') ?? 'bottom-edge-regression';
const results = [], geometry = {};
const previous = !baseline && process.argv.includes('--compare') ? JSON.parse(readFileSync(join(artifacts, 'baseline.json'), 'utf8')).geometry : null;
function browser(args) {
  const run = spawnSync(executable, ['--session', session, ...args, '--json'], { encoding: 'utf8', timeout: 45000, maxBuffer: 32 * 1024 * 1024 });
  if (run.status !== 0) throw new Error(run.error?.message || run.stderr || run.stdout);
  const reply = JSON.parse(run.stdout);
  if (!reply.success) throw new Error(reply.error);
  return reply.data;
}
const evaluate = code => browser(['eval', '-b', Buffer.from(code).toString('base64')]).result;
const check = (name, pass, actual) => results.push({ name, pass: Boolean(pass), actual });
const count = names => evaluate(`window.edgeFixture.ledger.filter(x => ${JSON.stringify(names)}.includes(x.name)).length`);
const pause = ms => evaluate(`new Promise(resolve => setTimeout(() => resolve(true), ${ms}))`);
const clear = () => evaluate('window.edgeFixture.clear()');
// Installed agent-browser mouse CLI accepts integer coordinates only.
const move = (x, y) => browser(['mouse', 'move', String(Math.round(x)), String(Math.round(y))]);
const click = (button = 'left') => { browser(['mouse', 'down', button]); browser(['mouse', 'up', button]); };
const actionNames = { quick: ['show_quick_launch_panel', 'hide_quick_launch_panel'], direct: ['activate_task_window'],
  group: ['show_task_gallery', 'hide_task_gallery'], process: ['show_process_manager'] };
const allActions = Object.values(actionNames).flat();
// A focused gallery open may retire its previous hover nonce first. That
// cleanup hide is not a second activation. Reject duplicate show OR hide.
const actionCount = name => name === 'group' ? evaluate(`(() => {const l=window.edgeFixture.ledger;return Math.max(l.filter(x=>x.name==='show_task_gallery' && x.args.args.focusGallery===true).length,l.filter(x=>x.name==='hide_task_gallery').length)})()`) : count(actionNames[name]);
function open(height = 32.4, theme = 'dark', width = 960, unlocked = false) {
  browser(['set', 'viewport', String(width), '600']);
  browser(['open', `http://127.0.0.1:5192/edge.html?height=${height}&theme=${theme}&unlocked=${unlocked ? 1 : 0}`]);
  browser(['wait', '--fn', 'Boolean(window.edgeFixture && document.querySelector(".task-capsule") && document.querySelector(".task-group-direct .task-button"))']);
  evaluate('document.fonts.ready.then(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve(true)))))');
  move(2, 20);
}
function point(name, position = 'center', edge = true) {
  return evaluate(`(() => { const b=document.querySelector(window.edgeFixture.selectors[${JSON.stringify(name)}]);const r=b.getBoundingClientRect();return {x:${position === 'left' ? 'r.left+1' : position === 'right' ? 'r.right-1' : '(r.left+r.right)/2'},y:${edge ? 'innerHeight-1' : '(r.top+r.bottom)/2'}}})()`);
}
const moveTo = p => move(p.x, p.y);
try {
  if (!overflowOnly) {
  for (const name of Object.keys(actionNames)) {
    open();
    moveTo(point(name, 'center', false)); pause(400);
    if (name === 'group') browser(['press', 'Escape']);
    clear(); click();
    check(`${name}: positive control face press reaches production IPC once`, actionCount(name) === 1, evaluate('window.edgeFixture.ledger'));
  }
  for (const height of statesOnly || overflowOnly ? [] : [24, 32.4, 48]) for (const theme of ['dark', 'light']) for (const width of [960, 320]) {
    const key = `${height}/${theme}/${width}`;
    open(height, theme, width);
    const metrics = evaluate('({ buttons: window.edgeFixture.measure(), bar: (() => {const r=document.querySelector(".bottom-bar").getBoundingClientRect();return {top:r.top,bottom:r.bottom,height:r.height,width:r.width}})() })');
    geometry[key] = metrics;
    check(`${key}: bar flush with viewport; configured logical height`, Math.abs(metrics.bar.bottom - 600) < .02 && Math.abs(metrics.bar.height - height) < .02, metrics.bar);
    for (const [name, value] of Object.entries(metrics.buttons)) {
      check(`${key}/${name}: visible face keeps 2px bottom inset`, Math.abs(600 - value.face.bottom - 2) < .02, value.face);
      check(`${key}/${name}: last addressable pixel hits SAME native button`, value.edgeHit, value);
    }
    if (previous?.[key]) {
      check(`${key}: visible geometry unchanged from pre-fix baseline`, JSON.stringify(metrics.bar) === JSON.stringify(previous[key].bar)
        && Object.keys(metrics.buttons).every(name => JSON.stringify(metrics.buttons[name].face) === JSON.stringify(previous[key].buttons[name].face)
        && JSON.stringify(metrics.buttons[name].children) === JSON.stringify(previous[key].buttons[name].children)), { before: previous[key], after: metrics });
    }
    browser(['screenshot', join(artifacts, `${baseline ? 'baseline' : 'current'}-${height}-${theme}-${width}.png`)]);
    for (const name of Object.keys(actionNames)) for (const position of ['left', 'center', 'right']) {
      moveTo(point(name, position));
      check(`${key}/${name}/${position}: edge engages native hover state`, evaluate(`document.querySelector(window.edgeFixture.selectors[${JSON.stringify(name)}]).matches(':hover')`), null);
      // Let supported hover intent settle, then isolate press+release from hover IPC.
      if (name === 'group') pause(400);
      clear(); click();
      check(`${key}/${name}/${position}: trusted bottom-edge action exactly once`, actionCount(name) === 1, evaluate('window.edgeFixture.ledger'));
    }
    for (const [name, command] of [['direct', 'show_task_window_preview'], ['group', 'show_task_gallery']]) {
      browser(['press', 'Escape']); move(2, 20); pause(450); clear();
      moveTo(point(name)); pause(850);
      check(`${key}/${name}: edge hover invokes production preview/gallery`, count([command]) === 1, evaluate('window.edgeFixture.ledger'));
      clear(); moveTo(point(name, 'center', false)); moveTo(point(name)); pause(150);
      check(`${key}/${name}: face to extension and back keeps hover session`, count(['hide_task_window_preview', 'hide_task_gallery', command]) === 0, evaluate('window.edgeFixture.ledger'));
    }
    for (const name of ['direct', 'group']) {
      moveTo(point(name)); clear(); click('right');
      check(`${key}/${name}: edge context menu emitted once`, count(['show_context_menu_overlay']) === 1, evaluate('window.edgeFixture.ledger'));
    }
    // Inactive horizontal gutters and margins must never become surrogate buttons.
    browser(['press', 'Escape']); move(2, 20); pause(450);
    const inactive = evaluate(`(() => {const f=window.edgeFixture; const r=s=>document.querySelector(s).getBoundingClientRect();const q=r(f.selectors.quick),d=r(f.selectors.direct),g=r(f.selectors.group),p=r(f.selectors.process);return [0,innerWidth-1,(q.right+d.left)/2,(d.right+g.left)/2,(g.right+p.left)/2]})()`);
    for (const x of inactive) {
      clear(); move(x, 599); click();
      check(`${key}: inactive bottom x=${x} stays inert`, count(allActions) === 0, evaluate('window.edgeFixture.ledger'));
    }
  }
  }
  // Real horizontal overflow, rather than labeling ordinary 320px reflow overflow.
  for (const height of statesOnly ? [] : [24, 32.4, 48]) for (const theme of ['dark', 'light']) {
    open(height, theme, 320);
    evaluate(`window.edgeFixture.snapshot([...window.edgeFixture.rows,...Array.from({length:90},(_,i)=>({...window.edgeFixture.rows[0],hwnd:String(10000+i),processId:10000+i,processName:'Overflow '+i,isActive:false,isMinimized:false}))])`);
    // Enter narrow overflow through a real host resize, not just a snapshot
    // whose unchanged strip width does not trigger production's width observer.
    browser(['set', 'viewport', '321', '600']);
    browser(['set', 'viewport', '320', '600']);
    pause(150);
    const overflow = evaluate(`(() => {const s=document.querySelector('.task-strip');return {width:s.clientWidth,scrollWidth:s.scrollWidth,overflow:s.classList.contains('task-strip-overflow'),status:document.querySelector('.taskbar-overflow-status').classList.contains('visible')}})()`);
    const overflowKey = `overflow/${height}/${theme}`;
    geometry[overflowKey] = evaluate('window.edgeFixture.measure()');
    if (previous?.[overflowKey]) check(`${overflowKey}: overflow face/content geometry unchanged`, Object.keys(geometry[overflowKey]).every(name =>
      JSON.stringify(geometry[overflowKey][name].face) === JSON.stringify(previous[overflowKey][name].face)
      && JSON.stringify(geometry[overflowKey][name].children) === JSON.stringify(previous[overflowKey][name].children)), { before: previous[overflowKey], after: geometry[overflowKey] });
    browser(['screenshot', join(artifacts, `${baseline ? 'baseline' : 'current'}-overflow-${height}-${theme}.png`)]);
    check(`${height}/${theme}: actual narrow task-strip overflow exercised`, overflow.scrollWidth > overflow.width && overflow.overflow && overflow.status, overflow);
    for (const name of ['quick', 'process']) {
      moveTo(point(name)); clear(); click();
      check(`${height}/${theme}/${name}: overflow cannot steal edge action`, actionCount(name) === 1, evaluate('window.edgeFixture.ledger'));
    }
    clear(); move(319,599); click();
    check(`${height}/${theme}: overflow cannot leak hit targets into right outer margin`, count(allActions) === 0, evaluate('window.edgeFixture.ledger'));
  }
  if (!overflowOnly) {
  open();
  evaluate('window.edgeFixture.snapshot(window.edgeFixture.rows.map((row,index)=>index ? {...row,processName:"Windows Terminal",activityState:"busy"} : row))');
  check('busy group cue exists while a busy member is not minimized', evaluate('Boolean(document.querySelector(".task-group-busy"))'), null);
  evaluate('window.edgeFixture.snapshot(window.edgeFixture.rows.map(row=>({...row,isMinimized:true,isActive:false})))');
  check('inactive attention cue exists in minimized mounted state', evaluate('Boolean(document.querySelector(".task-window-attention"))'), null);
  check('fully minimized group remains an edge target without changing its face', evaluate(`(() => {const b=document.querySelector('.task-capsule');const r=b.getBoundingClientRect();return b.closest('.task-group').classList.contains('task-group-minimized') && Math.abs(innerHeight-r.bottom-2)<.02 && document.elementFromPoint((r.left+r.right)/2,innerHeight-1)?.closest('button')===b})()`), null);
  // Stateful regressions use fresh mounts so prior hover/menu state cannot mask them.
  open();
  for (const name of ['direct', 'group']) {
    const start = point(name), finish = { x: start.x + 100, y: start.y };
    moveTo(start); pause(400); browser(['press', 'Escape']); clear(); browser(['mouse', 'down', 'left']); moveTo(finish); browser(['mouse', 'up', 'left']);
    check(`${name}: edge drag suppresses action`, actionCount(name) === 0, evaluate('window.edgeFixture.ledger'));
    moveTo(point(name)); pause(400); browser(['press', 'Escape']); clear(); browser(['mouse', 'down', 'left']); moveTo({ x: point(name).x + 30, y: 599 }); browser(['press', 'Escape']); browser(['mouse', 'up', 'left']);
    check(`${name}: Escape cancels edge drag without activation`, actionCount(name) === 0, evaluate('window.edgeFixture.ledger'));
    moveTo(point(name)); clear(); click();
    check(`${name}: fresh press after cancellation works once`, actionCount(name) === 1, evaluate('window.edgeFixture.ledger'));
  }
  open();
  evaluate('window.edgeFixture.control.deferActivation=true');
  moveTo(point('direct', 'center', false)); click();
  browser(['wait', '--fn', 'document.querySelector(".task-group-direct .task-button").disabled']);
  moveTo(point('direct')); clear(); click();
  check('disabled direct tile: edge cannot invoke again', count(['activate_task_window']) === 0, evaluate('window.edgeFixture.ledger'));
  evaluate('window.edgeFixture.control.release()');
  // Keyboard acceptance starts from its own clean session, not a deliberately
  // deferred activation/pointer-suppression state from the disabled stress case.
  open();
  for (const name of Object.keys(actionNames)) {
    evaluate(`document.querySelector(window.edgeFixture.selectors[${JSON.stringify(name)}]).focus()`);
    clear(); browser(['press', 'Enter']);
    check(`${name}: keyboard Enter retains exactly-once native action`, actionCount(name) === 1, evaluate('window.edgeFixture.ledger'));
    evaluate(`document.querySelector(window.edgeFixture.selectors[${JSON.stringify(name)}]).focus()`);
    clear(); browser(['press', 'Space']);
    check(`${name}: keyboard Space retains exactly-once native action`, actionCount(name) === 1, evaluate('window.edgeFixture.ledger'));
  }
  evaluate('document.querySelector(window.edgeFixture.selectors.direct).focus()');
  browser(['press', 'End']);
  check('toolbar End focuses last task (no extension tabstop)', evaluate('document.activeElement === document.querySelector(window.edgeFixture.selectors.group)'), evaluate('document.activeElement.outerHTML'));
  browser(['press', 'Home']);
  check('toolbar Home focuses first task', evaluate('document.activeElement === document.querySelector(window.edgeFixture.selectors.direct)'), null);
  open(); moveTo(point('direct')); pause(1100);
  const tickerState = () => evaluate(`(() => {const t=document.querySelector('.task-group-direct .overflow-ticker');const label=t?.firstElementChild;return t ? {moving:t.classList.contains('moving'),text:t.textContent,transform:getComputedStyle(label).transform,animationTime:label.getAnimations()[0]?.currentTime??null,clip:getComputedStyle(t).overflow} : null})()`);
  check('edge hover preserves overflowing title text', Boolean(tickerState()?.text?.includes('Direct')), tickerState());
  check('edge hover naturally starts ticker after engagement delay', tickerState()?.moving && tickerState()?.transform !== 'none', tickerState());
  const beforeCrossing = tickerState(); moveTo(point('direct', 'center', false)); moveTo(point('direct')); pause(200);
  check('face-extension crossing does not restart ticker', tickerState()?.animationTime > beforeCrossing?.animationTime, { before: beforeCrossing, after: tickerState() });
  evaluate('document.querySelector(window.edgeFixture.selectors.direct).focus()'); move(2,20); pause(200);
  check('focus keeps ticker alive after pointer leaves', tickerState()?.moving, tickerState());
  evaluate('document.activeElement.blur()'); pause(100);
  check('leaving hover AND focus resets ticker', !tickerState()?.moving && tickerState()?.transform === 'none', tickerState());
  browser(['set', 'media', 'dark', 'reduced-motion']);
  moveTo(point('direct')); pause(800);
  check('reduced motion browser preference active', evaluate('matchMedia("(prefers-reduced-motion: reduce)").matches'), null);
  check('reduced motion edge hover leaves ticker still and clipped', !tickerState()?.moving && tickerState()?.transform === 'none' && tickerState()?.clip === 'hidden', tickerState());
  open(32.4, 'dark', 960, true);
  const resize = evaluate('(() => {const r=document.querySelector(".bar-resize-handle").getBoundingClientRect();return {top:r.top,bottom:r.bottom}})()');
  // Absolute top is inside the bar's 1px top border, not its outer border box.
  check('unlocked resize handle is TOP ONLY', resize.top >= 600 - 32.4 && resize.top - (600 - 32.4) <= 1.02 && resize.bottom < 600 - 20, resize);
  clear(); move(500, 599); browser(['mouse', 'down', 'left']); move(500, 585); browser(['mouse', 'up', 'left']);
  check('bottom blank drag cannot resize bar', count(['resize_shell_bar', 'save_shell_bar_height']) === 0, evaluate('window.edgeFixture.ledger'));
  clear(); move(500, resize.top + 1); browser(['mouse', 'down', 'left']); move(500, resize.top - 12); browser(['mouse', 'up', 'left']);
  check('top handle still resizes and persists', count(['resize_shell_bar']) > 0 && count(['save_shell_bar_height']) === 1, evaluate('window.edgeFixture.ledger'));
  }
} catch (error) {
  check('runner completes all planned assertions', false, String(error.stack ?? error));
} finally {
  try { browser(['close']); } catch {}
}
const report = { phase: baseline ? 'baseline' : 'current', geometry, results, failed: results.filter(r => !r.pass),
  evidenceBoundary: 'Trusted browser mouse/keyboard; production BottomBar, native button handlers, mocked IPC. Not native Tauri/WebView2/AppBar/DWM certification.' };
writeFileSync(join(artifacts, `${report.phase}.json`), JSON.stringify(report, null, 2));
console.log(JSON.stringify({ total: results.length, passed: results.filter(r => r.pass).length, failed: report.failed.map(r => r.name), artifacts }, null, 2));
process.exitCode = report.failed.length ? 1 : 0;
