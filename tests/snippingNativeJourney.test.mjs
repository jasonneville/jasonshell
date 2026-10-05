// Opt-in actual Tauri journey, no production main or desktop pixels. Synthetic
// clipboard replacement is intentional. Normal Node runs MUST NOT launch it.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { lstat, readFile, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { inflateSync } from 'node:zlib';
import test from 'node:test';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const enabled = process.platform === 'win32' && process.env.JASONSHELL_SNIP_JOURNEY_TESTS === '1';
const delay = ms => new Promise(r => setTimeout(r, ms));

// Independent PNG reader: not producer/png crate, no clipboard reads, no library
// dependency. Accept only bounded noninterlaced RGBA8 output; all row filters.
function decode(bytes) {
  assert.ok(bytes.length <= 1024 * 1024);
  assert.deepEqual(bytes.subarray(0, 8), Buffer.from([137,80,78,71,13,10,26,10]));
  let width, height, ended = false;
  const data = [];
  for (let p = 8; p < bytes.length;) {
    const length = bytes.readUInt32BE(p);
    assert.ok(p + 12 + length <= bytes.length);
    const type = bytes.toString('ascii', p + 4, p + 8);
    const chunk = bytes.subarray(p + 8, p + 8 + length);
    if (type === 'IHDR') {
      assert.equal(length, 13); width = chunk.readUInt32BE(0); height = chunk.readUInt32BE(4);
      assert.deepEqual([...chunk.subarray(8)], [8,6,0,0,0]);
    } else if (type === 'IDAT') data.push(chunk);
    else if (type === 'IEND') { assert.equal(length, 0); ended = true; }
    p += length + 12;
  }
  assert.ok(ended && width > 0 && height > 0 && width <= 640 && height <= 400);
  const stride = width * 4;
  const raw = inflateSync(Buffer.concat(data), { maxOutputLength: (stride + 1) * height });
  assert.equal(raw.length, (stride + 1) * height);
  const rgba = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)]; assert.ok(filter <= 4);
    for (let x = 0; x < stride; x++) {
      const p = y * stride + x;
      const a = x >= 4 ? rgba[p - 4] : 0;
      const b = y > 0 ? rgba[p - stride] : 0;
      const c = y > 0 && x >= 4 ? rgba[p - stride - 4] : 0;
      const paeth = () => { const v=a+b-c; const da=Math.abs(v-a),db=Math.abs(v-b),dc=Math.abs(v-c); return da<=db&&da<=dc?a:db<=dc?b:c; };
      rgba[p] = (raw[y * (stride + 1) + x + 1] + [0,a,b,Math.floor((a+b)/2),paeth()][filter]) & 255;
    }
  }
  return { width, height, rgba };
}
async function viteReady() {
  try { const r = await fetch('http://localhost:1420/@vite/client', { signal: AbortSignal.timeout(700) }); return r.ok && (await r.text()).includes('createHotContext'); }
  catch { return false; }
}
async function terminateOwned(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  const killer = spawn('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
  await once(killer, 'exit');
}
test('actual Tauri synthetic offscreen journey and independent saved PNG pixels', { skip: !enabled, timeout: 110_000 }, async t => {
  let vite, child, fixtureDirectory, exited;
  const messages = [];
  let protocolError, buffer = '';
  const wait = async (predicate, stage) => {
    const end = Date.now() + 14_000;
    while (Date.now() < end) {
      if (protocolError) throw protocolError;
      const i = messages.findIndex(predicate);
      if (i !== -1) return messages.splice(i, 1)[0];
      if (child && (child.exitCode !== null || child.signalCode !== null)) throw new Error(`native fixture exited before ${stage}: ${child.exitCode}`);
      await delay(20);
    }
    throw new Error(`native stage timeout: ${stage}`);
  };
  try {
    if (!await viteReady()) {
      // Existing npm dev script only; never npm tauri / production main.
      vite = spawn('pwsh', ['-NoProfile','-Command','npm run dev -- --port 1420 --strictPort'], { cwd: root, stdio: 'ignore' });
      const end = Date.now() + 12_000;
      while (!await viteReady() && Date.now() < end) await delay(100);
      assert.ok(await viteReady(), 'local Vite did not become ready');
    }
    child = spawn(join(root,'src-tauri/target/debug/examples/snip_synthetic_journey.exe'), ['--synthetic-clipboard-opt-in'], { cwd: root, stdio: ['pipe','pipe','pipe'] });
    exited = once(child, 'exit');
    child.on('error', error => { protocolError = error; });
    child.stderr.on('data', () => {}); // Never persist profile/OS diagnostics.
    child.stdout.setEncoding('utf8');
    child.stdout.on('data', text => {
      buffer += text;
      if (buffer.length > 16_384) { protocolError = new Error('bounded stdout protocol exceeded'); return; }
      let p;
      while ((p = buffer.indexOf('\n')) !== -1) {
        const line = buffer.slice(0,p).trim(); buffer = buffer.slice(p+1);
        if (!line) continue;
        try {
          const value = JSON.parse(line);
          if (value.error || value.renderer?.error) protocolError = new Error(`native protocol failure: ${value.error ?? value.renderer.error}; op=${value.op ?? value.renderer?.op ?? 'unknown'}`);
          messages.push(value);
          if (messages.length > 128) protocolError = new Error('bounded metadata queue exceeded');
        } catch { protocolError = new Error('invalid native metadata JSON'); }
      }
    });
    const ready = await wait(m => m.stage === 'ready', 'READY');
    assert.equal(ready.synthetic, true); assert.equal(ready.source, 'owned-offscreen-dib');
    assert.match(ready.fixtureId, new RegExp(`^snip-journey-${child.pid}-[0-9]+$`));
    fixtureDirectory = join(process.env.LOCALAPPDATA, 'Temp/opencode', ready.fixtureId);
    assert.ok(!(await lstat(fixtureDirectory)).isSymbolicLink());
    const send = op => child.stdin.write(`${JSON.stringify({op})}\n`);
    const action = async op => { send(op); return (await wait(m => m.renderer?.op === op, op)).renderer; };
    const inspect = async () => { send('inspect'); return wait(m => m.stage === 'inspect','inspect'); };
    const started = await action('start');
    assert.match(started.generation, /^[1-9][0-9]*$/); assert.match(started.captureId, /^[0-9a-f]{32}$/);
    const frozen = await wait(m => m.stage === 'synthetic-dib-frozen', 'offscreen DIB');
    assert.deepEqual([frozen.width,frozen.height,frozen.source],[640,400,'owned-offscreen-dib']);
    const selected = await action('select'); assert.equal(selected.generation,started.generation); assert.equal(selected.captureId,started.captureId);
    const preview = await action('preview'); assert.deepEqual([preview.width,preview.height],[126,75]);
    const baseline = await inspect();
    assert.equal(baseline.phase,'Some(Preview)'); assert.equal(baseline.publicationAttempts,1);
    assert.equal(baseline.previewVisible,true); assert.equal(baseline.foregroundIsPreview,false); assert.equal(baseline.frameBytes,0);
    assert.ok(baseline.imageBytes >= 126*75*4);
    assert.equal(baseline.publication.status,'committed'); assert.equal(baseline.publication.committed,true); assert.equal(baseline.publication.durable,true);
    assert.equal((await action('deny-start')).denied,true);
    const denied = await inspect(); assert.equal(denied.publicationAttempts,1); assert.equal(denied.phase,'Some(Preview)');
    assert.equal((await action('save')).status,'saved');
    const destination = join(fixtureDirectory,'synthetic.png');
    assert.ok(!(await lstat(destination)).isSymbolicLink());
    const png = decode(await readFile(destination)); assert.deepEqual([png.width,png.height],[126,75]);
    for (let y=0;y<75;y++) for(let x=0;x<126;x++) {
      const p=(y*126+x)*4;
      assert.deepEqual([...png.rgba.subarray(p,p+4)],[(64+x)%251,(48+y)%251,93,255],`synthetic pixel ${x},${y}`);
    }
    const saved = await inspect(); assert.equal(saved.publicationAttempts,1); assert.equal(saved.phase,'Some(Preview)');
    t.diagnostic('synthetic native PNG126x75: all9450 pixels match; first64,48,93,255; last189,122,93,255; automatic publicationAttempts=1; preview nonactivating; wrong-window start denied');
    await action('dismiss');
    const dismissed = await inspect(); assert.equal(dismissed.phase,'None'); assert.equal(dismissed.windowCount,0); assert.equal(dismissed.imageBytes,0); assert.equal(dismissed.publicationAttempts,1);
  } finally {
    if (child) {
      if (child.exitCode === null && child.signalCode === null) { child.stdin.end('{"op":"shutdown"}\n'); await Promise.race([exited,delay(6000)]); }
      await terminateOwned(child); if (exited) await exited;
      // Only the newly owned PID-bound fixture; never discover/remove other profiles.
      if (fixtureDirectory) { await rm(fixtureDirectory,{recursive:true,force:true,maxRetries:8,retryDelay:250}); }
    }
    await terminateOwned(vite);
  }
  assert.equal(child.exitCode,0,'native fixture shutdown failed');
});
