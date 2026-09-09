import { mkdir, open, writeFile, readFile } from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import { resolve, sep, join } from 'node:path';
import { createHash } from 'node:crypto';
import { writeFixture } from './harness.mjs';

const root = resolve(import.meta.dirname, '../..');
const output = resolve(process.argv[2] ?? '');
if (!output.toLowerCase().startsWith(`${join(root, 'test-results')}${sep}`.toLowerCase())) throw Error('Output must be under test-results');
// Caller chooses a new run directory. Never reuse an old evidence manifest.
await mkdir(output);
const baseline = process.memoryUsage(), peak = { ...baseline };
let memorySamples = 0;
const sampleMemory = () => {
  const usage = process.memoryUsage(); memorySamples++;
  for (const key of Object.keys(peak)) peak[key] = Math.max(peak[key], usage[key]);
};
const manifest = { schema: 'stack-text-corpus.v2', seed: 1, fixtures: [], nativePresentation: 'unobserved' };
for (const kind of ['utf8', 'utf16le', 'utf16be', 'minified', 'giant-line', 'many-lines', 'combining', 'invalid-tail']) {
  const requested = kind === 'giant-line' ? 134217728 : 1048576;
  const path = join(output, `${kind}.bin`), start = performance.now();
  const record = await writeFixture(path, { kind, seed: manifest.seed, bytes: requested }, sampleMemory);
  const hash = createHash('sha256'); let verifiedBytes = 0;
  for await (const chunk of createReadStream(path, { highWaterMark: 65536 })) { hash.update(chunk); verifiedBytes += chunk.length; sampleMemory(); }
  if (hash.digest('hex') !== record.sha256 || String(verifiedBytes) !== record.actualBytes) throw Error('FixtureMismatch');
  const handle = await open(path, 'r');
  try {
    const prefix = Buffer.alloc(65536);
    const { bytesRead } = await handle.read(prefix, 0, prefix.length, 0);
    record.prefixReadBytes = bytesRead;
    record.eofObservedByPrefixReader = bytesRead >= verifiedBytes;
  } finally { await handle.close(); }
  manifest.fixtures.push({ ...record, elapsedMs: performance.now() - start });
  sampleMemory();
}
manifest.memory = { baseline, peakObserved: peak, baselineAdjustedPeakObserved: Object.fromEntries(Object.keys(peak).map(key => [key, peak[key] - baseline[key]])), samples: memorySamples,
  basis: 'sampled after each generation/write and verification chunk; transient peaks between samples remain unobserved; not a production memory-budget proof' };
const production = JSON.parse(await readFile(join(root, 'src-tauri/tauri.conf.json')));
const probe = JSON.parse(await readFile(join(root, 'src-tauri/examples/text-probe/tauri.conf.json')));
if (production.app.security.csp !== probe.app.security.csp) throw Error('CspMismatch');
manifest.productionCspUnchanged = true;
await writeFile(join(output, 'manifest.json'), JSON.stringify(manifest, null, 2), { flag: 'wx' });
console.log(JSON.stringify({ fixtures: manifest.fixtures.length, maxChunk: Math.max(...manifest.fixtures.map(f => f.maxChunk)), giantBytes: manifest.fixtures.find(f => f.kind === 'giant-line').actualBytes, productionCspUnchanged: true }));
