// Non-live fixture benchmark. Compile helpers: node node_modules/typescript/bin/tsc -p tsconfig.test.json
// Run: node scripts/benchmark-process-manager-sort.mjs --output <new-json-path>
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { performance } from 'node:perf_hooks';
import { cpus } from 'node:os';
import * as current from '../dist-tests/lib/processManagerState.js';
import { countedPids, legacyClassifier, legacyOrdering, processSortFixture } from '../tests/fixtures/processManagerSort.fixture.mjs';

const outputIndex = process.argv.indexOf('--output');
if (outputIndex < 0 || !process.argv[outputIndex + 1]) throw new Error('--output <new-json-path> required');
const output = process.argv[outputIndex + 1];
const rows = processSortFixture(512);
const previous = rows.slice(32).reverse();
const active = rows.filter((_, index) => index % 4 === 0).map((row) => row.pid);
const sort = { column: 'cpuPercent', direction: 'desc' };
const legacy = legacyOrdering(legacyClassifier(current.isWindowsProcess));
const hash = (data) => createHash('sha256').update(data).digest('hex');
const percentile = (values, p) => [...values].sort((a, b) => a - b)[Math.ceil(values.length * p) - 1];
const raw = {};
let sink = 0;

function invoke(implementation, scenario, membership) {
  const options = { taskbarActivePids: membership, preserveExistingOrder: true };
  return scenario === 'sort' ? implementation.sortProcesses(rows, sort, options)
    : implementation.orderProcessRefresh(previous, rows, sort, options);
}

function allocationCounts(implementation, scenario) {
  const observed = countedPids(active);
  const NativeSet = globalThis.Set;
  let setConstructions = 0;
  // Synchronous isolated probe only; restore before timing or assertions. Counts constructions, not heap bytes.
  globalThis.Set = new Proxy(NativeSet, {
    construct(target, args) { setConstructions += 1; return Reflect.construct(target, args); }
  });
  try { invoke(implementation, scenario, observed.iterable); }
  finally { globalThis.Set = NativeSet; }
  return { setConstructions, membershipIterations: observed.counts.iterations, yieldedPids: observed.counts.yielded };
}

for (const scenario of ['sort', 'refresh']) {
  assert.deepEqual(invoke(current, scenario, active), invoke(legacy, scenario, active));
  const implementations = { legacy, current };
  const results = Object.fromEntries(Object.keys(implementations).map((name) => [name, {
    counts: allocationCounts(implementations[name], scenario), samplesMs: []
  }]));
  for (const implementation of Object.values(implementations)) {
    for (let warmup = 0; warmup < 5; warmup += 1) sink ^= invoke(implementation, scenario, active)[0].pid;
  }
  // Alternate ordering across 30 paired samples; fixture generation and counting outside timing.
  for (let round = 0; round < 30; round += 1) {
    for (const name of round % 2 ? ['current', 'legacy'] : ['legacy', 'current']) {
      const start = performance.now();
      const result = invoke(implementations[name], scenario, active);
      results[name].samplesMs.push(performance.now() - start);
      sink ^= result[round % result.length].pid;
    }
  }
  for (const result of Object.values(results)) {
    result.medianMs = percentile(result.samplesMs, 0.5);
    result.p95Ms = percentile(result.samplesMs, 0.95);
  }
  raw[scenario] = results;
}

const artifact = {
  schema: 'process-manager-sort-fixture/v1', timestamp: new Date().toISOString(),
  command: process.argv, node: process.version, platform: process.platform, arch: process.arch,
  cpu: cpus()[0]?.model, rows: rows.length, activePids: active.length, warmup: 5, samples: 30,
  fixtureSha256: hash(JSON.stringify({ rows, previous, active, sort })),
  sourceSha256: hash(readFileSync(new URL('../src/lib/processManagerState.ts', import.meta.url))),
  compiledSha256: hash(readFileSync(new URL('../dist-tests/lib/processManagerState.js', import.meta.url))),
  oracleSha256: hash(readFileSync(new URL('../tests/fixtures/processManagerSort.fixture.mjs', import.meta.url))),
  evidence: 'Non-live Node fixture; observed Set constructor counts and iterable traversals, not total allocator bytes or app private bytes. Legacy freezes pre-optimization ordering and membership construction, sharing unchanged isWindowsProcess heuristic.',
  acceptance: Object.fromEntries(Object.entries(raw).map(([scenario, results]) => [scenario, {
    membershipPreparedAtMostOnce: results.current.counts.membershipIterations <= 1,
    atMostOneSetConstruction: results.current.counts.setConstructions <= 1
  }])),
  fullRuntimePrivateBytes: 'NOT MEASURED', sink, scenarios: raw
};
writeFileSync(output, `${JSON.stringify(artifact, null, 2)}\n`, { flag: 'wx' });
console.log(JSON.stringify({ output, scenarios: Object.fromEntries(Object.entries(raw).map(([scenario, results]) => [scenario,
  Object.fromEntries(Object.entries(results).map(([name, result]) => [name, { ...result, samplesMs: undefined }]))])) }, null, 2));
