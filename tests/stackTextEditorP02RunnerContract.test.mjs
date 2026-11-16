import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { EventEmitter } from 'node:events';
import { persistCommandRecord } from '../scripts/stack-text-editor/commandManifest.mjs';

const runner = await readFile('scripts/stack-text-editor/p02-run.mjs', 'utf8');

test('P02 runner keeps deferred P12 scale debt outside non-scale T02 gate', () => {
  assert.match(runner, /blockedOutcome\('P12\/NFR-6'/u);
  assert.match(runner, /const nonScaleOutcomes = outcomes\.filter/u);
  assert.match(runner, /:'IN_REVIEW'/u);
  assert.match(runner, /retained as non-gate P12\/NFR-6 debt/u);
});

test('P02 runner emits explicit review and coordinator disposition packet', () => {
  assert.match(runner, /review-packet\.json/u);
  assert.match(runner, /snapshotCompleteIsRecoveryComplete: false/u);
  assert.match(runner, /coordinatorDisposition: 'PENDING'/u);
});

test('P02 runner behaviorally persists actual native probe command record', async () => {
  const output = await mkdtemp(join(tmpdir(), 'p02-command-manifest-'));
  const commands = [{ id: '024', args: ['test'], exitCode: 0 }];
  const native = {
    id: '025',
    executable: 'cargo',
    args: ['run', '--manifest-path', 'src-tauri/Cargo.toml', '--features', 'text-document-feasibility'],
    exitCode: 0,
    stdout: '025-actual-window-stdout.log',
    stderr: '025-actual-window-stderr.log',
  };

  await persistCommandRecord(output, commands, native);

  const persisted = JSON.parse(await readFile(join(output, 'commands.json'), 'utf8'));
  assert.deepEqual(persisted, [commands[0], native]);
});

// Execute only the extracted probe driver with mocked capabilities. Never import
// the top-level runner: it performs native/fixture work requiring human consent.
async function isolatedProbeInvocation() {
  const declaration = runner.match(/async function runActualWindowProbe\(\) \{[\s\S]*?\n\}/)?.[0];
  assert.ok(declaration, 'actual-window probe function must be available for isolated acceptance');
  const calls = [];
  const records = [];
  const spawn = (executable, args, options) => {
    calls.push({ executable, args, options });
    const child = new EventEmitter();
    child.stdout = new EventEmitter();
    child.stderr = new EventEmitter();
    child.kill = () => assert.fail('successful isolated fixture must not require termination');
    queueMicrotask(() => child.emit('close', 0));
    return child;
  };
  const result = {
    status: 'PASS', authorizationMode: 'actual-webview-window',
    unauthorizedExistingAndMissingTargetRejected: true,
    contentDisclosedOnRejectedOpen: false,
    feasibilitySchemaRevision: 'stack-text-editor.p02-feasibility.v2',
  };
  const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
  const invoke = new AsyncFunction(
    'spawn', 'join', 'createWriteStream', 'readFile', 'writeFile',
    'persistCommandRecord', 'process',
    `const output = 'isolated-fixture'; const commands = []; const outcomes = [];
     ${declaration}
     await runActualWindowProbe(); return outcomes;`,
  );
  const outcomes = await invoke(
    spawn, join, () => ({ write() {}, end(done) { done(); } }),
    async () => JSON.stringify(result), async () => {},
    async (_output, _commands, record) => { records.push(record); }, { env: {} },
  );
  assert.equal(outcomes[0]?.status, 'PASS');
  assert.equal(calls.length, 1);
  assert.equal(records.length, 1);
  return { call: calls[0], record: records[0] };
}

function assertFeasibilityFeature(args) {
  const featureIndex = args.indexOf('--features');
  assert.ok(featureIndex >= 0, 'native probe cargo invocation must explicitly enable its nondefault feature');
  assert.ok(args[featureIndex + 1]?.split(',').includes('text-document-feasibility'));
}

test('actual-window probe execution explicitly enables text-document-feasibility', async () => {
  const { call } = await isolatedProbeInvocation();
  assert.equal(call.executable, 'cargo');
  assert.equal(call.args[0], 'run');
  assertFeasibilityFeature(call.args);
  assert.equal(call.options.env.P02_NATIVE_PROBE_ROOT, 'isolated-fixture');
});

test('actual-window probe recorded cargo args truthfully include the enabled feature', async () => {
  const { call, record } = await isolatedProbeInvocation();
  assertFeasibilityFeature(record.args);
  assert.deepEqual(record.args, call.args);
  assert.equal(record.executable, call.executable);
});

test('unsupported P02 probe request fails closed before Tauri or ordinary shell startup', async () => {
  const main = (await readFile('src-tauri/src/main.rs', 'utf8')).replaceAll('\r\n', '\n');
  const windowsMain = main.slice(main.indexOf('#[cfg(target_os = "windows")]\nfn main()'));
  const builderIndex = windowsMain.indexOf('tauri::Builder::default()');
  assert.ok(builderIndex > 0, 'Windows entry must have an identifiable builder boundary');
  const beforeBuilder = windowsMain.slice(0, builderIndex);
  assert.match(beforeBuilder, /#\[cfg\(not\(all\(debug_assertions, feature = "text-document-feasibility"\)\)\)\]\s*if std::env::var_os\("P02_NATIVE_PROBE_ROOT"\)\.is_some\(\)\s*\{[\s\S]*?std::process::exit\([1-9]\d*\);[\s\S]*?\n    \}/,
    'unsupported probe requests must exit nonzero before any Tauri startup, not fall through into AppBars/hooks');
});
