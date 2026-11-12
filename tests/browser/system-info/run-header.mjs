// Production-mounted header acceptance in an existing visible CDP browser.
// No new browser, native shell, dependency, or real process termination.
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const output = process.argv[2] ?? path.join(process.env.LOCALAPPDATA, 'Temp/opencode/process-header-e2e');
await mkdir(output, { recursive: true });
const port = process.env.HEADER_CDP_PORT ?? '9222';
const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const target = targets.find(page => page.url.startsWith('http://127.0.0.1:4181'));
if (!target) throw new Error('Open the fixture in visible wmux first');
const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
let sequence = 0;
const pending = new Map();
socket.onmessage = event => {
  const message = JSON.parse(event.data);
  const request = pending.get(message.id);
  if (!request) return;
  pending.delete(message.id); clearTimeout(request.timer);
  if (message.error) request.reject(new Error(JSON.stringify(message.error))); else request.resolve(message.result);
};
function command(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP deadline: ${method}`)); }, 10000);
    pending.set(id, { resolve, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const value = await command('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (value.exceptionDetails) throw new Error(JSON.stringify(value.exceptionDetails));
  return value.result.value;
}
async function settle() {
  await evaluate('(async()=>{await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));})()');
}
async function click(selector) {
  const point = await evaluate(`(()=>{const e=document.querySelector(${JSON.stringify(selector)});if(!e)throw new Error('missing click target');const r=e.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()`);
  await command('Input.dispatchMouseEvent', { type: 'mousePressed', ...point, button: 'left', clickCount: 1 });
  await command('Input.dispatchMouseEvent', { type: 'mouseReleased', ...point, button: 'left', clickCount: 1 });
  await settle();
}
const checks = [];
function check(condition, name, detail) { checks.push({ name, pass: !!condition, detail }); }
async function geometry() {
  return evaluate(`(()=>{
    const main=document.querySelector('main');const header=main.querySelector('header');
    const rect=e=>{const r=e.getBoundingClientRect();return {left:r.left,right:r.right,top:r.top,bottom:r.bottom,width:r.width,height:r.height,centerY:r.y+r.height/2};};
    const controls={title:header.querySelector('strong'),filter:header.querySelector('input'),refresh:header.querySelector('[aria-label="Refresh process manager"]'),close:header.querySelector('[aria-label="Close process manager"]')};
    return {viewport:{width:innerWidth,height:innerHeight},surface:rect(main),header:rect(header),tabs:rect(main.querySelector('[role="tablist"]')),controls:Object.fromEntries(Object.entries(controls).map(([k,e])=>[k,rect(e)])),outerScrollWidth:document.documentElement.scrollWidth,visible:document.visibilityState};
  })()`);
}
async function screenshot(name) {
  await settle();
  const image = await command('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
  await writeFile(path.join(output, name), Buffer.from(image.data, 'base64'));
}
const results = { checks, layouts: {}, viewportMethod: 'CDP Emulation.setDeviceMetricsOverride in existing visible browser page', port };
try {
  await command('Page.navigate', { url: 'http://127.0.0.1:4181' });
  for (let attempt = 0; attempt < 100; attempt++) {
    if (await evaluate('!!window.__headerFixture && !!document.querySelector("main")')) break;
    await new Promise(resolve => setTimeout(resolve, 30));
  }
  for (const width of [900, 360]) {
    await command('Emulation.setDeviceMetricsOverride', { width, height: 800, deviceScaleFactor: 1, mobile: false });
    await evaluate(`(async()=>{const end=performance.now()+3000;while(!document.querySelector('#ledger')?.textContent.includes('get_process_manager_context')){if(performance.now()>end)throw new Error('mount listeners not ready');await new Promise(r=>setTimeout(r,20));}await new Promise(r=>setTimeout(r,50));})()`);
    await evaluate(`(async()=>{document.querySelector('#surface').style.width='100%';const b=window.__headerFixture;b.close();b.setRows([...b.defaultRows]);b.open();const end=performance.now()+3000;while(!document.querySelector('[role="grid"]').textContent.includes('Alpha')){if(performance.now()>end)throw new Error('rows missing');await new Promise(r=>setTimeout(r,20));}})()`);
    await settle();
    const layout = results.layouts[width] = await geometry();
    const entries = Object.entries(layout.controls);
    check(layout.viewport.width === width && layout.visible === 'visible', `${width}: genuine visible document viewport`, layout.viewport);
    for (const [name, box] of entries) {
      check(box.width > 0 && box.height > 0 && box.left >= 0 && box.right <= width + 1 && box.top >= 0 && box.bottom <= layout.tabs.top + 1, `${width}: ${name} onscreen above tabs`, box);
    }
    for (let i = 0; i < entries.length; i++) for (let j = i + 1; j < entries.length; j++) {
      const [a, x] = entries[i], [b, y] = entries[j];
      check(Math.min(x.right, y.right) <= Math.max(x.left, y.left) + 1 || Math.min(x.bottom, y.bottom) <= Math.max(x.top, y.top) + 1, `${width}: ${a}/${b} do not overlap`);
    }
    check(layout.outerScrollWidth <= width + 1, `${width}: no outer horizontal overflow`, layout.outerScrollWidth);
    check(layout.tabs.top >= layout.header.bottom - 1, `${width}: tabs below header`);
    if (width === 900) {
      const centers = entries.map(([, box]) => box.centerY);
      check(Math.max(...centers) - Math.min(...centers) <= 6, '900: title/filter/refresh/close share one vertical center (6px tolerance)', centers);
      check(layout.header.height <= 72, '900: compact header at most 72px', layout.header.height);
      check(layout.controls.filter.left > width / 2 && layout.controls.filter.right <= layout.controls.refresh.left + 1, '900: filter occupies right side before Refresh');
    }
    await screenshot(`header-${width}.png`);
    // Real CDP input, not a bound-value assignment or mocked filtering algorithm.
    await click('header input');
    await evaluate(`document.querySelector('header input').addEventListener('input',event=>{window.__headerTrustedInput=event.isTrusted;},{once:true})`);
    await command('Input.insertText', { text: '202' }); await settle();
    const filtered = await evaluate(`({value:document.querySelector('header input').value,text:document.querySelector('[role="grid"]').textContent})`);
    check(filtered.value === '202' && filtered.text.includes('Beta') && !filtered.text.includes('Alpha'), `${width}: real PID input filters to Beta`, filtered.value);
    check(await evaluate('window.__headerTrustedInput===true'), `${width}: browser delivered trusted input`);
    await evaluate(`document.querySelector('header input').select()`);
    await command('Input.insertText', { text: 'Alpha' }); await settle();
    check(await evaluate(`(()=>{const text=document.querySelector('[role="grid"]').textContent;return text.includes('Alpha')&&!text.includes('Beta');})()`), `${width}: real name input filters to Alpha`);
    await evaluate(`document.querySelector('header input').select()`);
    await command('Input.insertText', { text: '202' }); await settle();
    await command('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
    await command('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Tab', code: 'Tab', windowsVirtualKeyCode: 9 });
    check(await evaluate(`document.activeElement?.getAttribute('aria-label')==='Refresh process manager'`), `${width}: keyboard Tab reaches Refresh from filter`);
    await click('[id="manager-tab-system_info"]');
    const system = await evaluate(`({input:!!document.querySelector('header input'),refresh:!!document.querySelector('header [aria-label="Refresh process manager"]'),status:!!document.querySelector('header [role="status"]'),close:!!document.querySelector('header [aria-label="Close process manager"]')})`);
    check(!system.input && !system.refresh && !system.status && system.close, `${width}: Processes-only controls/status; shared Close`, system);
    check(await evaluate(`(()=>{const r=document.querySelector('header [aria-label="Close process manager"]').getBoundingClientRect();return r.width>0&&r.left>=0&&r.right<=innerWidth&&r.bottom<=document.querySelector('[role="tablist"]').getBoundingClientRect().top;})()`), `${width}: shared Close remains onscreen above tabs in System Info`);
    await screenshot(`system-header-${width}.png`);
    await click('[id="manager-tab-processes"]');
    check(await evaluate(`document.querySelector('header input').value==='202'`), `${width}: PID filter preserved across tabs`);
  }
  results.status = checks.every(item => item.pass) ? 'PASS' : 'FAIL';
  await writeFile(path.join(output, 'header-results.json'), JSON.stringify(results, null, 2));
  console.log(JSON.stringify({ status: results.status, passed: checks.filter(item => item.pass).length, failed: checks.filter(item => !item.pass), output }, null, 2));
  process.exitCode = results.status === 'PASS' ? 0 : 1;
} finally {
  await command('Emulation.clearDeviceMetricsOverride').catch(() => {});
  socket.close();
}
