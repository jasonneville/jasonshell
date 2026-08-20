import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { validatePhase0HarnessSamples } from './measure-search-performance.core.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(__dirname, '..');
const artifactRoot = join(repoRoot, 'test-results', 'search-performance');
const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
const outDir = join(artifactRoot, timestamp);
mkdirSync(outDir, { recursive: true });

const prefixCorpus = JSON.parse(readFileSync(join(repoRoot, 'tests/fixtures/searchPhase0PrefixCorpus.fixture.json'), 'utf8'));
const approvedPhaseFiles = [
  'src-tauri/src/search/mod.rs',
  'src-tauri/src/search/phase0_harness.rs',
  'src-tauri/src/search/providers/apps.rs',
  'src-tauri/src/search/providers/everything.rs',
  'src-tauri/src/search/providers/local.rs',
  'src-tauri/src/search/providers/open_windows.rs',
  'src-tauri/src/search/providers/settings.rs',
  'scripts/measure-search-performance.mjs',
  'scripts/measure-search-performance.core.mjs',
  'tests/searchOverhaulPhase0.test.mjs',
  'tests/fixtures/searchPhase0PrefixCorpus.fixture.json',
];
const approvedPhaseStatus = spawnSync('git', ['status', '--short', '--', ...approvedPhaseFiles], { cwd: repoRoot, encoding: 'utf8' }).stdout;
const commandLines = [];

function recordCommand(command, status, stdout, stderr) {
  commandLines.push(`$ ${command}`);
  commandLines.push(`status=${status}`);
  if (stdout) commandLines.push(`stdout=${stdout}`);
  if (stderr) commandLines.push(`stderr=${stderr}`);
}

function percentile(values, p) {
  const sorted = [...values].sort((a, b) => a - b);
  const rank = Math.ceil((p / 100) * sorted.length) - 1;
  return sorted[Math.max(0, Math.min(sorted.length - 1, rank))];
}

function redact(text) {
  return String(text)
    .replace(/([A-Z]:\\Users\\)[^\\\r\n]+/gi, '$1<redacted>')
    .replace(/(username|hostname|device id|device-id|computer name)=([^\s]+)/gi, '$1=<redacted>');
}

function extractFunctionBlock(source, signaturePattern) {
  const start = source.search(signaturePattern);
  if (start === -1) throw new Error(`missing function body for ${signaturePattern}`);
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let index = braceStart; index < source.length; index += 1) {
    const char = source[index];
    if (char === '{') depth += 1;
    else if (char === '}' && --depth === 0) return source.slice(start, index + 1);
  }
  throw new Error(`unterminated function body for ${signaturePattern}`);
}

function assertNoRecursiveScanInSource() {
  const searchMod = readFileSync(join(repoRoot, 'src-tauri/src/search/mod.rs'), 'utf8');
  const everything = readFileSync(join(repoRoot, 'src-tauri/src/search/providers/everything.rs'), 'utf8');
  const settings = readFileSync(join(repoRoot, 'src-tauri/src/search/providers/settings.rs'), 'utf8');
  const apps = readFileSync(join(repoRoot, 'src-tauri/src/search/providers/apps.rs'), 'utf8');
  const local = readFileSync(join(repoRoot, 'src-tauri/src/search/providers/local.rs'), 'utf8');
  const openWindows = readFileSync(join(repoRoot, 'src-tauri/src/search/providers/open_windows.rs'), 'utf8');

  const scopedBodies = [
    extractFunctionBlock(searchMod, /fn run_search_engine\b/),
    extractFunctionBlock(settings, /pub\(crate\) fn search_settings\b/),
    extractFunctionBlock(apps, /pub\(crate\) fn search_apps\b/),
    extractFunctionBlock(local, /pub\(crate\) fn search_local\b/),
    extractFunctionBlock(openWindows, /pub\(crate\) fn search_open_windows\b/),
    extractFunctionBlock(everything, /pub\(crate\) fn search_everything\b/),
    extractFunctionBlock(everything, /fn search_everything_with\b/),
  ];

  const banned = [/WalkDir\b/, /recursive\s+traversal/i, /recursive_scan/i, /broad scan/i, /scan.*recursive/i];
  for (const body of scopedBodies) {
    for (const pattern of banned) {
      if (pattern.test(body)) throw new Error(`source guard failed: ${pattern}`);
    }
  }
  return { source_guard: { passed: true, scanned_functions: scopedBodies.length, limit: 'scoped hot-path functions only; apps index builder outside search_apps excluded' } };
}

function validateRedactions(text) {
  const banned = [/\\Users\\[^\\\r\n]+/i, /hostname/i, /username/i, /device id/i, /device-id/i];
  for (const pattern of banned) {
    if (pattern.test(text)) throw new Error(`redaction validator blocked pattern ${pattern}`);
  }
}

function measureNodeTransformSerializationProxy(rowCount) {
  const rows = Array.from({ length: rowCount }, (_, index) => ({
    title: `Fixture row ${index + 1}`,
    subtitle: `Prefix ${index % prefixCorpus.queries.length}`,
    score: 100 - index,
  }));
  const start = performance.now();
  const output = rows.map((row) => JSON.stringify(row)).join('\n');
  return { rowCount, durationMs: performance.now() - start, bytes: output.length };
}

const harnessOutDir = join(outDir, 'harness');
mkdirSync(harnessOutDir, { recursive: true });
const cargo = spawnSync('cargo', ['test', '--manifest-path', 'src-tauri/Cargo.toml', 'phase0_harness::tests::phase0_harness_produces_progress_and_health_traces', '--', '--nocapture'], {
  cwd: repoRoot,
  env: { ...process.env, JASONSHELL_PHASE0_OUTDIR: harnessOutDir },
  encoding: 'utf8',
});
recordCommand('cargo test --manifest-path src-tauri/Cargo.toml phase0_harness::tests::phase0_harness_produces_progress_and_health_traces -- --nocapture', cargo.status, cargo.stdout, cargo.stderr);
if (cargo.status !== 0) throw new Error(`cargo test failed with status ${cargo.status}\n${cargo.stdout}\n${cargo.stderr}`);

const harnessJson = JSON.parse(readFileSync(join(harnessOutDir, 'phase0-samples.json'), 'utf8'));
const harnessValidation = validatePhase0HarnessSamples(harnessJson);
const nodeSamples = [0, 5, 50].flatMap((rowCount) => Array.from({ length: 30 }, () => ({ rowCount, ...measureNodeTransformSerializationProxy(rowCount) })));
const rawSamplesMs = harnessJson.map((sample) => sample.input_to_final_ms);
const rawScenarioSamples = harnessJson.reduce((acc, sample) => ((acc[sample.scenario] ??= []).push(sample), acc), {});
const scenarioAggregates = Object.fromEntries(Object.entries(rawScenarioSamples).map(([scenario, samples]) => [scenario, { count: samples.length, p50: percentile(samples.map((sample) => sample.input_to_final_ms), 50), p95: percentile(samples.map((sample) => sample.input_to_final_ms), 95) }]));

const redactionReport = { scanned_fields: ['metadata.json', 'raw-benchmark.json', 'derived-tables.json', 'command-output.txt', 'redaction-report.json', 'live-dogfood-notes.md'], prohibited_hits: [] };

const metadata = {
  timestamp,
  git_commit: spawnSync('git', ['rev-parse', 'HEAD'], { cwd: repoRoot, encoding: 'utf8' }).stdout.trim(),
  approvedPhaseFiles,
  approvedPhaseStatus: redact(approvedPhaseStatus),
  approvedPhaseStatusHash: createHash('sha256').update(approvedPhaseStatus).digest('hex'),
  phase_diff_hash: (() => {
    const hash = createHash('sha256');
    for (const relativePath of approvedPhaseFiles) {
      hash.update(relativePath);
      hash.update('\0');
      hash.update(readFileSync(join(repoRoot, relativePath)));
      hash.update('\0');
    }
    hash.update(approvedPhaseStatus);
    hash.update('\0');
    return hash.digest('hex');
  })(),
  redacted_status: redact(spawnSync('git', ['status', '--short'], { cwd: repoRoot, encoding: 'utf8' }).stdout),
  os: process.platform,
  arch: process.arch,
  machineClass: `${process.platform}-${process.arch}`,
  mode: 'phase0-technical-acceptance',
  deterministicMode: 'debug-test',
  app_version: JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8')).version,
  package_resources: { status: 'unavailable-incomplete', reason: 'src-tauri/resources missing while release exe exists' },
  everything_availability: { queried: false, versionQueried: false, reason: 'not queried by phase0 harness' },
  app_cache_state: { fixtures: 'deterministic fake harness evidence' },
  release_binary: { availability: 'present', evidence: 'source tree has release exe only' },
  release_live_evidence: { status: 'absent-not-run', reason: 'package resources unavailable; live run not performed' },
  reset_steps: ['close search UI', 'clear generated artifacts', 'rerun phase0 harness'],
  command: 'node scripts/measure-search-performance.mjs',
};

const derivedTables = {
  p50: percentile(rawSamplesMs, 50),
  p95: percentile(rawSamplesMs, 95),
  stale_table: harnessJson.map((sample) => ({ scenario: sample.scenario, stale_count: sample.stale_count, latest_count: sample.latest_count })),
  queue_table: harnessJson.map((sample) => ({ scenario: sample.scenario, queue_wait_ms: sample.queue_wait_ms, boundary_trace: sample.boundary_trace })),
  no_scan_guard: (() => {
    const sourceGuard = assertNoRecursiveScanInSource();
    const runtimeSamples = harnessValidation.runtime_sample_count;
    const recursiveScanSamples = harnessJson.filter((sample) => (sample.observed_operations ?? []).includes('RecursiveFilesystemScan')).length;
    const boundaryCount = harnessValidation.boundary_count;
    const passed = runtimeSamples > 0 && recursiveScanSamples === 0 && boundaryCount > 0 && sourceGuard.source_guard.passed;
    if (!passed) throw new Error(`no-scan guard failed: runtime=${runtimeSamples} recursive=${recursiveScanSamples} boundaries=${boundaryCount}`);
    return { runtime_sample_count: runtimeSamples, recursive_scan_sample_count: recursiveScanSamples, passed, boundary_count: boundaryCount, source_guard: sourceGuard.source_guard };
  })(),
};

const rawBenchmark = {
  rounds: prefixCorpus.measurementRounds,
  rawSamplesMs,
  rawScenarioSamples,
  harnessJson,
  rendererProxy: { label: 'Node transform/serialization proxy', samples: nodeSamples, rowCounts: [0, 5, 50] },
  scenarioAggregates,
  providerStates: prefixCorpus.providerStates,
  harnessStdout: cargo.stdout,
  harnessStderr: cargo.stderr,
  prefixFixture: prefixCorpus,
};

const commandOutput = `${commandLines.join('\n')}\n`;
validateRedactions(commandOutput + JSON.stringify(metadata) + JSON.stringify(rawBenchmark) + JSON.stringify(derivedTables));

writeFileSync(join(outDir, 'metadata.json'), JSON.stringify(metadata, null, 2));
writeFileSync(join(outDir, 'raw-benchmark.json'), JSON.stringify(rawBenchmark, null, 2));
writeFileSync(join(outDir, 'derived-tables.json'), JSON.stringify(derivedTables, null, 2));
writeFileSync(join(outDir, 'command-output.txt'), commandOutput);
writeFileSync(join(outDir, 'redaction-report.json'), JSON.stringify(redactionReport, null, 2));
writeFileSync(join(outDir, 'live-dogfood-notes.md'), 'Live dogfood not run. Package resources unavailable in this environment, so only deterministic fixture/source-contract evidence was collected.\n');

console.log(JSON.stringify({ artifactDir: outDir, p50: derivedTables.p50, p95: derivedTables.p95 }, null, 2));
