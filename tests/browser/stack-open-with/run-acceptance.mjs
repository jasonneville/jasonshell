import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const executable = process.env.AGENT_BROWSER_EXECUTABLE ?? join(process.env.APPDATA, 'npm/node_modules/agent-browser/bin/agent-browser-win32-x64.exe');
const artifacts = join(process.env.LOCALAPPDATA, 'Temp/opencode/stack-open-with-e2e');
mkdirSync(artifacts, { recursive: true });
const results = [];
function browser(args) {
  const run = spawnSync(executable, ['--session', 'stack-open-with-acceptance', ...args, '--json'], { encoding: 'utf8', timeout: 35000, maxBuffer: 8 * 1024 * 1024 });
  if (run.status !== 0) throw new Error(`${args.join(' ')}: ${run.error?.message || run.stderr || run.stdout}`);
  const reply = JSON.parse(run.stdout);
  if (!reply.success) throw new Error(reply.error);
  return reply.data;
}
const evaluate = (code) => browser(['eval', '-b', Buffer.from(code).toString('base64')]).result;
const wait = (code) => browser(['wait', '--fn', code]);
const assert = (condition, message) => { if (!condition) throw new Error(message); };
const menuCode = `[...document.querySelectorAll('[role="menuitem"]')]`;
const itemCode = (text) => `${menuCode}.find(e=>e.textContent.trim()===${JSON.stringify(text)})`;
const triggerCode = `${menuCode}.find(e=>e.textContent.includes('Open with'))`;
function rect(code) { return evaluate(`(()=>{const e=${code};if(!e)return null;const r=e.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height,right:r.right,bottom:r.bottom,visible:getComputedStyle(e).visibility!=='hidden'&&getComputedStyle(e).display!=='none'&&r.width>0&&r.height>0};})()`); }
function move(x, y) { browser(['mouse', 'move', String(Math.round(x)), String(Math.round(y))]); }
function clickAt(x, y, button = 'left') { move(x,y); browser(['mouse','down',button]); browser(['mouse','up',button]); }
function click(code) { const r = rect(code); assert(r?.visible,'click target visible'); clickAt(r.x+r.width/2,r.y+r.height/2); }
function resolveCandidates(index, ids) { evaluate(`window.openWithFixture.pending[${index}].resolve(${JSON.stringify(ids)}.map(window.openWithFixture.candidate))`); }
function prepare(width = 1000, height = 650) {
  browser(['open','http://127.0.0.1:4186']); browser(['set','viewport',String(width),String(height)]);
  wait('Boolean(document.querySelector("[data-stack-entry-path]"))');
  browser(['dblclick','[data-stack-entry-path]']);
  wait(`Boolean(document.querySelector('[data-stack-entry-path$="draft.txt"]'))`);
}
function context(x = 40,y = 100,name = 'draft.txt') {
  // Context event anchors can be anywhere in the client; row identity remains production-owned.
  evaluate(`document.querySelector('[data-stack-entry-path$="${name}"]').dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:${x},clientY:${y}}))`);
  wait(`Boolean(${triggerCode})`);
}
function open() { click(triggerCode); }
function record(name, fn) {
  try { const evidence = fn(); results.push({ name, pass: true, evidence }); }
  catch (error) { results.push({ name, pass: false, error: String(error) }); }
  try { browser(['screenshot',resolve(artifacts,`${results.length}.png`)]); } catch {}
}
try {
  for (const [name,width,height,x,y,diagonal,scroll] of [
    ['right',1000,650,40,100,false,false], ['left',1000,650,999,100,false,false],
    ['diagonal-right',1000,650,40,100,true,false], ['diagonal-left',1000,650,999,100,true,false],
    ['scrolled-parent',1000,450,40,100,true,true], ['constrained',440,300,420,250,true,false]
  ]) record(`trusted pointer gap crossing ${name}`, () => {
    prepare(width,height); context(x,y); resolveCandidates(0,['editor','other']);
    if (scroll) evaluate(`document.querySelector('.context-menu-scroll').scrollTop=18`);
    const t = rect(triggerCode); assert(t?.visible,'trigger rendered');
    move(t.x+t.width/2,t.y+t.height/2);
    wait(`${triggerCode}?.getAttribute('aria-expanded')==='true'`);
    evaluate(`new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))`);
    const target = rect(itemCode('Choose app...')); assert(target?.visible,'chooser visible after hover');
    assert(target.x>=0&&target.y>=0&&target.right<=width+1&&target.bottom<=height+1,'chooser constrained inside client');
    const left = target.x<t.x;
    if (name==='left'||name==='diagonal-left') assert(left,'edge menu must open left');
    if (name==='right'||name==='diagonal-right') assert(!left,'roomy menu must open right');
    const endX=left?target.right-8:target.x+8;
    const startX=left?t.x+1:t.right-1;
    const startY=t.y+t.height/2;
    const points=[[t.x+t.width/2,startY]];
    if (diagonal) {
      // Traverse a neighboring parent action before reaching the gap: this is the
      // real diagonal regression, not a horizontal-only bridge check.
      points.push([left?t.x+t.width/3:t.right-t.width/3,startY+t.height+5]);
    }
    points.push([startX,diagonal?startY+t.height+5:startY],[(startX+endX)/2,diagonal?(startY+target.y+target.height/2)/2:startY],
      [endX,diagonal?target.y+target.height/2:startY],[target.x+target.width/2,target.y+target.height/2]);
    const endpoint=browser(['get','cdp-url']).cdpUrl;
    const path=spawnSync(process.execPath,['tests/browser/stack-open-with/pointer-path.mjs',endpoint,JSON.stringify(points)],{encoding:'utf8',timeout:10000});
    assert(path.status===0,path.stderr||'trusted CDP path completed');
    const trace=JSON.parse(path.stdout);
    assert(trace.every(p=>p.expanded==='true'),`path retained child: ${JSON.stringify(trace)}`);
    assert(evaluate(`${triggerCode}?.getAttribute('aria-expanded')==='true'`),'submenu survives gap');
    move(target.x+target.width/2,target.y+target.height/2);
    browser(['mouse','down']); browser(['mouse','up']);
    const evidence=evaluate(`({calls:window.openWithFixture.ledger.filter(e=>e.command==='open_stack_item_with_picker'),pointer:window.openWithFixture.pointer})`);
    assert(evidence.calls.length===1&&evidence.calls[0].args.path.endsWith('draft.txt'),'one captured-file chooser dispatch');
    assert(evidence.pointer.length>=points.length&&evidence.pointer.every(e=>e.trusted),'every observed path event is trusted browser input');
    return {...evidence,trace};
  });
  for (const mode of ['loading','empty','error']) record(`chooser reachable during ${mode}`,()=>{
    prepare(); context();
    if(mode==='empty')resolveCandidates(0,[]);
    if(mode==='error')evaluate(`window.openWithFixture.pending[0].reject(new Error('Fixture lookup rejected'))`);
    open(); const chooser=rect(itemCode('Choose app...')); assert(chooser?.visible,'chooser visible');
    assert(!evaluate(`${menuCode}.some(e=>/Notepad|Visual Studio Code/.test(e.textContent))`),'no invented apps');
    click(itemCode('Choose app...'));
    assert(evaluate(`window.openWithFixture.ledger.filter(e=>e.command==='open_stack_item_with_picker').length`)===1,'chooser dispatched once');
  });
  for(const mode of ['pointer','focus'])record(`late response does not move chooser under ${mode}`,()=>{
    prepare();context();open();
    const before=rect(itemCode('Choose app...'));assert(before?.visible,'loading chooser visible');
    if(mode==='pointer')move(before.x+before.width/2,before.y+before.height/2);
    else browser(['focus','#stack-open-with-menu [role="menuitem"]:last-child']);
    evaluate(`window.fixtureChooser=${itemCode('Choose app...')};window.fixtureFocus=document.activeElement`);
    resolveCandidates(0,['late','later']);
    const after=rect(itemCode('Choose app...'));
    assert(Math.abs(after.x-before.x)<1&&Math.abs(after.y-before.y)<1,'chooser geometry stable');
    assert(evaluate(`window.fixtureChooser===${itemCode('Choose app...')}`),'chooser DOM identity stable');
    if(mode==='focus')assert(evaluate('document.activeElement===window.fixtureFocus'),'focus remains on same action');
    clickAt(before.x+before.width/2,before.y+before.height/2);
    assert(evaluate(`window.openWithFixture.ledger.filter(e=>e.command==='open_stack_item_with_picker').length`)===1,'original action invoked');
    return {before,after};
  });
  record('same-file reopen rejects out-of-order response',()=>{
    prepare();context();browser(['press','Escape']);context();
    resolveCandidates(1,['current']);resolveCandidates(0,['retired']);open();
    wait(`Boolean(${itemCode('Associated current')})`);
    assert(!evaluate(`Boolean(${itemCode('Associated retired')})`),'retired result excluded');
  });
  record('keyboard submenu navigation and layered Escape',()=>{
    prepare();context();resolveCandidates(0,['editor']);
    browser(['focus','.submenu-trigger']);browser(['press','ArrowRight']);
    wait(`document.activeElement?.closest('.context-submenu-panel')!==null`);
    browser(['press','ArrowLeft']);assert(evaluate(`document.activeElement===${triggerCode}`),'left returns trigger focus');
    browser(['press','ArrowRight']);browser(['press','Escape']);
    assert(evaluate(`Boolean(${triggerCode})&&${triggerCode}.getAttribute('aria-expanded')==='false'`),'Escape closes only child');
    browser(['press','Escape']);assert(!evaluate(`Boolean(${triggerCode})`),'second Escape dismisses root');
  });
  record('hover Escape without trigger focus closes child before parent',()=>{
    prepare();context();
    browser(['focus','[role="grid"]']);
    const t=rect(triggerCode);move(t.x+t.width/2,t.y+t.height/2);
    wait(`${triggerCode}?.getAttribute('aria-expanded')==='true'`);
    assert(evaluate(`document.activeElement?.getAttribute('role')==='grid'`),'hover does not steal grid focus');
    browser(['press','Escape']);
    assert(evaluate(`Boolean(${triggerCode})&&${triggerCode}.getAttribute('aria-expanded')==='false'`),'first Escape preserves root');
    browser(['press','Escape']);assert(!evaluate(`Boolean(${triggerCode})`),'second Escape closes root');
  });
  record('settling over another parent action closes child',()=>{
    prepare();context();const t=rect(triggerCode);move(t.x+t.width/2,t.y+t.height/2);
    wait(`${triggerCode}?.getAttribute('aria-expanded')==='true'`);
    const other=rect(itemCode('Copy'));move(other.x+other.width/2,other.y+other.height/2);
    wait(`${triggerCode}?.getAttribute('aria-expanded')==='false'`);
    return { switchedTo: 'Copy' };
  });
  record('launch captures file, suppresses duplicate and surfaces failure without fallback',()=>{
    prepare();evaluate('window.openWithFixture.state.deferLaunch=true');context();resolveCandidates(0,['editor']);open();click(itemCode('Associated editor'));
    context(40,100,'readme.md');resolveCandidates(1,['editor']);open();
    const second=rect(itemCode('Associated editor'));if(second?.visible)click(itemCode('Associated editor'));
    const calls=evaluate(`window.openWithFixture.ledger.filter(e=>e.command==='open_stack_item_with_app')`);
    assert(calls.length===1&&calls[0].args.path.endsWith('draft.txt'),'one launch pinned to original file');
    evaluate(`window.openWithFixture.launches[0].reject(new Error('Fixture association launch failed'))`);
    wait(`document.body.textContent.includes('Fixture association launch failed')`);
    assert(!evaluate(`window.openWithFixture.ledger.some(e=>e.command==='open_stack_item_with_picker'||e.command==='open_stack_item')`),'no fallback');
    return calls;
  });
} finally {
  try { browser(['close']); } catch {}
  const report={total:results.length,passed:results.filter(e=>e.pass).length,results,boundary:'Production DOM/CSS; trusted browser pointer/keyboard; controlled IPC. No live Tauri, handler or native picker.'};
  writeFileSync(join(artifacts,'results.json'),JSON.stringify(report,null,2));
  console.log(JSON.stringify(report,null,2));process.exitCode=results.some(e=>!e.pass)?1:0;
}
