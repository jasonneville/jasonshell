// Option A acceptance: existing, visible CDP browser and production-mounted IPC fixture.
import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

const output = process.argv[2] ?? path.join(process.env.LOCALAPPDATA, 'Temp/opencode/metric-strip-e2e');
const port = process.env.HEADER_CDP_PORT ?? '9224';
await mkdir(output, { recursive: true });
const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
const target = targets.find(page => page.url.startsWith('http://127.0.0.1:4181'));
if (!target) throw new Error('Open production-mounted fixture in a visible browser first');
const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
const pending = new Map(); let sequence = 0;
socket.onmessage = event => {
  const message = JSON.parse(event.data), request = pending.get(message.id);
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
const settle = () => evaluate('(async()=>{await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));})()');
async function click(selector) {
  const point = await evaluate(`(()=>{const e=document.querySelector(${JSON.stringify(selector)});e.scrollIntoView({block:'nearest'});const r=e.getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2};})()`);
  await command('Input.dispatchMouseEvent', { type: 'mousePressed', ...point, button: 'left', clickCount: 1 });
  await command('Input.dispatchMouseEvent', { type: 'mouseReleased', ...point, button: 'left', clickCount: 1 });
  await settle();
}
async function screenshot(name) {
  await settle();
  const image = await command('Page.captureScreenshot', { format: 'png', captureBeyondViewport: false });
  await writeFile(path.join(output, name), Buffer.from(image.data, 'base64'));
}
const results = { checks: [], layouts: {}, screenshots: [], port, viewportMethod: 'Visible browser, CDP device metrics, not native WebView2 certification' };
function check(pass, name, detail) { results.checks.push({ pass: !!pass, name, detail }); }
async function capture(name) { await screenshot(name); results.screenshots.push(name); }
try {
  await command('Page.navigate', { url: 'http://127.0.0.1:4181' });
  for (let attempt = 0; attempt < 100; attempt++) {
    if (await evaluate('!!window.__metricStripFixture && document.querySelector("#ledger")?.textContent.includes("get_process_manager_context")')) break;
    await new Promise(resolve => setTimeout(resolve, 30));
  }
  for (const width of [900, 360]) {
    await command('Emulation.setDeviceMetricsOverride', { width, height: 800, deviceScaleFactor: 1, mobile: false });
    await evaluate(`window.__metricStripFixture.close();window.__metricStripFixture.metricStripFixture(true);window.__metricStripFixture.open();`);
    await settle();
    await click('header input'); await command('Input.insertText', { text: '202' }); await settle();
    await click('#manager-tab-system_info');
    await evaluate(`(async()=>{const end=performance.now()+4000;while(!document.querySelector('[aria-label="System information details"]')?.textContent.includes('Fixture secondary GPU')){if(performance.now()>end)throw new Error('rich snapshot missing');await new Promise(r=>setTimeout(r,20));}})()`);
    const layout = await evaluate(`(()=>{
      const region=document.querySelector('[aria-label="System information details"]');region.scrollTop=0;
      const overview=document.querySelector('#system-overview-title').closest('section');
      const box=e=>{const r=e.getBoundingClientRect();return {left:r.left,right:r.right,top:r.top,bottom:r.bottom,width:r.width,height:r.height};};
      const terms=[...overview.querySelectorAll('dt')];
      const metric=re=>{const term=terms.find(e=>re.test(e.textContent));const value=term?.parentElement.querySelector('dd strong');return value?{label:term.textContent,value:value.textContent,fontSize:parseFloat(getComputedStyle(value).fontSize),...box(value)}:null;};
      const metadata=[...region.querySelectorAll('details')].map(e=>({summary:e.querySelector('summary')?.textContent,text:e.textContent,open:e.open}));
      return {width:innerWidth,visible:document.visibilityState,region:{...box(region),clientWidth:region.clientWidth,scrollWidth:region.scrollWidth},overview:box(overview),
        metrics:[metric(/CPU usage|whole.machine CPU/i),metric(/RAM used|memory used/i),metric(/busiest engine|GPU/i),metric(/uptime/i)],
        sections:[...region.querySelectorAll('h2')].map(e=>e.textContent.trim()),metadata,text:region.textContent,
        clipping:[...region.querySelectorAll('dt,dd,h3,summary')].filter(e=>e.clientWidth>0&&e.scrollWidth>e.clientWidth+1).map(e=>e.textContent),outerScrollWidth:document.documentElement.scrollWidth};
    })()`);
    results.layouts[width] = layout;
    check(layout.width === width && layout.visible === 'visible', `${width}: visible document at requested viewport`);
    check(['Overview', 'Storage', 'Sensors', 'Network', 'Hardware'].every(name => layout.sections.includes(name)), `${width}: all five sections retained`);
    check(layout.metrics.every(metric => metric && metric.fontSize >= 18), `${width}: CPU/RAM/GPU/uptime are prominent readings`, layout.metrics);
    if (width === 900) {
      check(layout.metrics.every(Boolean) && Math.max(...layout.metrics.map(metric => metric.top)) - Math.min(...layout.metrics.map(metric => metric.top)) <= 6, '900: four primary readings form horizontal metric strip', layout.metrics);
      check(layout.overview.height <= 240, '900: compact overview leaves room for quiet section rows', layout.overview.height);
    }
    check(layout.region.scrollWidth <= layout.region.clientWidth + 1 && layout.outerScrollWidth <= width + 1 && layout.clipping.length === 0, `${width}: no horizontal overflow or clipped field text`, layout.clipping);
    for (const token of ['73%', '75%', '47%', '100 GiB', '300 GiB', '25 GiB', '64 GiB', '16 GiB', 'ReFS', 'Fixture VPN', 'Fixture Ethernet with a deliberately long adapter name', '198.51.100.24', '198.51.100.25', '2001:db8:1234:5678:90ab:cdef:1234:5678', '198.51.100.1', 'Fixture secondary GPU', 'Fixture CPU', 'Fixture Board', 'Fixture BIOS']) {
      check(layout.text.includes(token), `${width}: dynamic fact retained: ${token}`);
    }
    check(/busiest engine, not total GPU/i.test(layout.text) && /quota-aware/.test(layout.text) && /not summed/i.test(layout.text), `${width}: GPU/quota/per-adapter caveats retained`);
    check(/Unavailable/.test(layout.text) && !/0\s*°C|MAC address|serial number|UUID|product key/i.test(layout.text), `${width}: unavailable sensors not fabricated; inventory privacy retained`);
    check(layout.metadata.length > 0 && layout.metadata.every(details => details.summary?.trim() && !details.open), `${width}: subdued, named progressive sampling details closed by default`, layout.metadata.map(details => details.summary));
    const provenance = layout.metadata.map(details => details.text).join(' ');
    check(['PDH whole-machine', 'GetDiskFreeSpaceExW', 'Fixture mapped capacity', 'Fixture Ethernet receive baseline', 'Fixture Ethernet stale send', 'Fixture secondary GPU unavailable'].every(source => provenance.includes(source)) && /provider not verified/i.test(provenance) && /baseline/i.test(provenance) && /stale/i.test(provenance) && /Updated|sampled|timestamp/i.test(provenance), `${width}: provenance, reasons, state and timestamps reachable in details`);
    check([/ready/i, /partial/i, /warming[ _-]up/i, /unavailable/i, /percent|%/i, /bytes|GiB|KiB|B\/s/i].every(pattern => pattern.test(provenance)), `${width}: sample states and units remain explicit in provenance`);
    await capture(`metric-strip-${width}-overview.png`);
    for (const section of ['storage', 'network', 'hardware']) {
      await evaluate(`document.querySelector('#system-${section}-title').scrollIntoView({block:'start'})`);
      await capture(`metric-strip-${width}-${section}.png`);
    }
    const summaries = await evaluate(`document.querySelectorAll('[aria-label="System information details"] details > summary').length`);
    for (let index = 0; index < summaries; index++) {
      await evaluate(`(()=>{const e=document.querySelectorAll('[aria-label="System information details"] details > summary')[${index}];e.scrollIntoView({block:'center'});e.focus();})()`);
      // Include Enter's text to produce keypress/default activation, as a real
      // browser keyboard does; a textless keyDown skips summary activation.
      await command('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r', unmodifiedText: '\r' });
      await command('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13 }); await settle();
      check(await evaluate(`(()=>{const e=document.querySelectorAll('[aria-label="System information details"] details > summary')[${index}];return e.parentElement.open&&document.activeElement===e;})()`), `${width}: disclosure ${index + 1} opens using real keyboard Enter`);
    }
    if (summaries) await capture(`metric-strip-${width}-provenance.png`);
    await click('#manager-tab-processes');
    check(await evaluate(`document.querySelector('header input').value==='202'&&document.querySelector('[role="grid"]').textContent.includes('Beta')&&!document.querySelector('[role="grid"]').textContent.includes('Alpha')`), `${width}: real main-tab roundtrip preserves PID filter and process rows`);
  }
  results.status = results.checks.every(item => item.pass) ? 'PASS' : 'FAIL';
  await writeFile(path.join(output, 'metric-strip-results.json'), JSON.stringify(results, null, 2));
  console.log(JSON.stringify({ status: results.status, passed: results.checks.filter(item => item.pass).length, failed: results.checks.filter(item => !item.pass), output }, null, 2));
  process.exitCode = results.status === 'PASS' ? 0 : 1;
} finally {
  await evaluate('window.__metricStripFixture?.metricStripFixture(false)').catch(() => {});
  await command('Emulation.clearDeviceMetricsOverride').catch(() => {}); socket.close();
}
