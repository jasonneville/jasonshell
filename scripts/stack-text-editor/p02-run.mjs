#!/usr/bin/env node
// Explicitly serial native P02 execution. This never launches the shell or edits
// desktop settings. Failed synthetic fixtures remain in this run for inspection.
import { spawn } from 'node:child_process';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createReadStream, createWriteStream } from 'node:fs';
import { mkdir, readFile, writeFile, copyFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { totalmem, cpus, release } from 'node:os';
import { generateCorpus, writeCorpusManifest, assertFreeSpace } from './corpusGenerator.mjs';
import { decodeByteText, encodeByteText } from './byteTextOracle.mjs';
import { DeterministicRangeIo } from './controlledIo.mjs';
import { createMeasurementRun, writeMeasurementArtifact } from './measurement.mjs';
import { persistCommandRecord } from './commandManifest.mjs';

const args = process.argv.slice(2);
const value = (name, fallback) => { const index = args.indexOf(name); return index < 0 ? fallback : args[index + 1]; };
if (!args.includes('--serial-authorized')) throw new Error('P02 execution requires coordinator serial authorization after both mutation workers stabilize');
if (process.platform !== 'win32') throw new Error('Native Windows P02 gate requires Windows, not a substitute platform');
const output = resolve(value('--output', `test-results/stack-text-editor/P02/${Date.now()}`));
const seed = Number(value('--seed', '305419896'));
if (!Number.isInteger(seed) || seed < 0 || seed > 0xffffffff) throw new Error('seed must be a uint32');
const sizes = value('--sizes', 'small').split(',');
if (sizes.some(size => size !== 'small') && !args.includes('--allow-large')) throw new Error('large fixtures require explicit --allow-large');
await mkdir(dirname(output), { recursive: true });
await mkdir(output, { recursive: false });
await mkdir(join(output, 'native-fixtures'));
const commands = [];
const observations = [];
const outcomes = [];
const testPrefix = 'stack_popup::text_document::feasibility::tests::';
const tests = [
  ['RB-02-v2', 'rb02_v2_lease_mapping_and_context'],
  ['RB-02-v2', 'rb02_v2_source_outcomes_and_errors'],
  ['RB-02-v2', 'rb02_v2_selection_barrier_replay'],
  ['RB-02-v2', 'rb02_v2_transport_credits_and_cancellation'],
  ['T02-01', 't02_01_authoritative_identity_rejection_and_races'],
  ['T02-01', 't02_01_identity_captures_reparse_metadata_and_rejects_named_streams'],
  ['T02-01', 't02_01_reparse_ancestor_and_final_refuse'],
  ['T02-01', 't02_01_symlink_refusal_or_explicit_fixture_block'],
  ['T02-01', 't02_01_native_probe_canonical_root_opens_ordinary_source'],
  ['T02-02', 't02_02_ordinary_and_preexisting_mapped_writer_native'],
  ['T02-02', 't02_02_prefix_edit_copy_handoff_and_cancellation'],
  ['T02-02', 't02_02_copy_quota_failure_retains_protected_source'],
  ['T02-03', 't02_03_paged_edits_history_eviction_and_quota_keep_dirty_root'],
  ['T02-03', 't02_03_authenticated_reopen_and_tamper_refusal'],
  ['T02-04', 't02_04_every_split_codec_map_crlf_and_invalid_tail'],
  ['T02-04', 't02_04_disk_checkpoints_and_distant_invalid'],
  ['T02-04', 't02_04_invalid_prefix_is_retained_and_exposed_as_decision_required'],
  ['T02-04', 't02_04_far_invalid_range_is_exposed_not_private_state'],
  ['T02-05', 't02_05_blocked_scan_urgent_priority_latest_and_cancellation'],
  ['T02-05', 't02_05_priority_lanes_bound_background_work_and_keep_actor_lock_io_free'],
  ['T02-05', 't02_05_prefix_caret_race_preempts_background_and_keeps_latest_seek'],
  ['T02-05', 't02_05_public_source_failure_states_are_distinct'],
  ['control', 't02_resource_control'],
];
const sourceFiles = [
  ...['mod','native','source','backing','contract','decode','hash','index','lease','rb02_v2','scheduler','session','tests'].map(name => `src-tauri/src/stack_popup/text_document/feasibility/${name}.rs`),
  'src-tauri/src/stack_popup/text_document/mod.rs', 'src-tauri/src/stack_popup/text_document/protocol.rs',
  'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock',
  'src-tauri/src/main.rs', 'src-tauri/src/stack_popup.rs',
  'src/features/stack-browser/textEditorProtocol.ts',
  'tests/fixtures/stack-text-editor-protocol.json', 'tests/fixtures/stack-text-v2.json',
  'tests/fixtures/stack-text-request-v2.json', 'tests/fixtures/stack-text-results-v2.json',
  'scripts/stack-text-editor/p02-run.mjs', 'scripts/stack-text-editor/corpusGenerator.mjs',
  'scripts/stack-text-editor/byteTextOracle.mjs', 'scripts/stack-text-editor/controlledIo.mjs', 'scripts/stack-text-editor/measurement.mjs',
  'scripts/stack-text-editor/commandManifest.mjs'
];
const sourceManifest = [];
for (const path of sourceFiles) {
  const bytes = await readFile(path);
  sourceManifest.push({ path, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') });
  const destination = join(output, 'source-snapshot', path); await mkdir(dirname(destination), { recursive: true }); await copyFile(path, destination);
}
await writeFile(join(output, 'source-manifest.json'), JSON.stringify(sourceManifest, null, 2));
await writeFile(join(output, 'environment.json'), JSON.stringify({ platform: process.platform, osRelease: release(), node: process.version, cpu: cpus()[0]?.model, logicalCpus: cpus().length, totalPhysicalMemoryBytes: String(totalmem()), seed, schemaRevision: 'stack-text-editor.v1', feasibilitySchemaRevision: 'stack-text-editor.p02-feasibility.v2', canonicalRevalidationSchemaRevision: 'stack-text-editor.v2', canonicalRevalidationScope: 'test-only-RB-02', context7: 'unavailable', lsp: 'unavailable', nativeUi: args.includes('--actual-window-probe') ? 'actual-webview-window-probe' : 'not-launched', desktopSettings: 'untouched', sourceRevision: 'source-manifest-sha256', invokedArguments: args }, null, 2));

async function runTest(group, name, env = {}, ignored = false) {
  const id = String(commands.length + 1).padStart(3, '0');
  const commandArgs = ['test', ...(args.includes('--release') ? ['--release'] : []), '--manifest-path', 'src-tauri/Cargo.toml', `${testPrefix}${name}`, '--', '--exact', '--nocapture', '--test-threads=1', ...(ignored ? ['--ignored'] : [])];
  const started = new Date().toISOString();
  const stdout = createWriteStream(join(output, `${id}-stdout.log`));
  const stderr = createWriteStream(join(output, `${id}-stderr.log`));
  let pending = ''; let matched = 0; const captured = [];
  const measurement = createMeasurementRun(); measurement.record('intent', { source: 'native' });
  function consume(text) {
    pending += text;
    if (pending.length > 2 * 1024 * 1024) throw new Error('native diagnostic line exceeds bound');
    for (;;) {
      const newline = pending.indexOf('\n'); if (newline < 0) break;
      const line = pending.slice(0, newline); pending = pending.slice(newline + 1);
      const index = line.indexOf('{"case":');
      const phaseIndex = line.indexOf('{"phase":');
      const jsonIndex = index >= 0 ? index : phaseIndex;
      if (jsonIndex >= 0) {
        try { const event = JSON.parse(line.slice(jsonIndex)); if (event.phase === 'P02') { observations.push(event); captured.push(event); if (event.case.endsWith('first-edit')) measurement.record('backendAck', { source: 'native', revision: event.observation.acceptedRevision }); } } catch { /* Full raw log remains authoritative if a producer line is not JSON. */ }
      }
      const result = /test result: ok\. (\d+) passed;/u.exec(line); if (result) matched += Number(result[1]);
    }
  }
  const child = spawn('cargo', commandArgs, { stdio: ['ignore', 'pipe', 'pipe'], env: { ...process.env, P02_FIXTURE_ROOT: join(output, 'native-fixtures'), ...env }, windowsHide: true });
  child.stdout.on('data', chunk => { stdout.write(chunk); consume(chunk.toString('utf8')); });
  child.stderr.on('data', chunk => stderr.write(chunk));
  const exitCode = await new Promise((resolveExit, reject) => { child.once('error', reject); child.once('close', resolveExit); });
  await Promise.all([new Promise(resolveEnd => stdout.end(resolveEnd)), new Promise(resolveEnd => stderr.end(resolveEnd))]);
  const command = { id, executable: 'cargo', args: commandArgs, environment: Object.fromEntries(Object.entries(env).map(([key, value]) => [key, value])), started, finished: new Date().toISOString(), exitCode, matchedPassingTests: matched, stdout: `${id}-stdout.log`, stderr: `${id}-stderr.log` };
  commands.push(command);
  const status = exitCode === 0 && matched === 1 ? 'PASS' : captured.some(event => event.observation?.gate === 'BLOCK') ? 'BLOCK' : 'FAIL';
  outcomes.push({ group, test: name, status, command: id, zeroMatchRejected: matched === 0 });
  await writeFile(join(output, 'commands.json'), JSON.stringify(commands, null, 2));
  await writeFile(join(output, 'observations.json'), JSON.stringify(observations, null, 2));
  await writeMeasurementArtifact(join(output, `${id}-measurement.json`), measurement.complete({ experiment: { phase: 'P02', noPaintClaim: true } }));
  return { status, captured };
}

async function discoverNativeTests(libtestArgs = []) {
  const commandArgs = [
    'test',
    ...(args.includes('--release') ? ['--release'] : []),
    '--manifest-path',
    'src-tauri/Cargo.toml',
    'stack_popup::text_document::feasibility::tests',
    '--',
    '--list',
    ...libtestArgs,
  ];
  const child = spawn('cargo', commandArgs, {
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, P02_FIXTURE_ROOT: join(output, 'native-fixtures') },
    windowsHide: true,
  });
  let stdout = '';
  let stderr = '';
  child.stdout.on('data', chunk => { stdout += chunk.toString('utf8'); });
  child.stderr.on('data', chunk => { stderr += chunk.toString('utf8'); });
  const exitCode = await new Promise((resolveExit, reject) => {
    child.once('error', reject);
    child.once('close', resolveExit);
  });
  if (exitCode !== 0) throw new Error(`P02 test discovery failed (${exitCode}): ${stderr.trim()}`);
  const suffix = ': test';
  return stdout
    .split(/\r?\n/u)
    .filter(line => line.startsWith(testPrefix) && line.endsWith(suffix))
    .map(line => line.slice(testPrefix.length, -suffix.length));
}

const discoveredTests = await discoverNativeTests();
const discoveredIgnoredTests = await discoverNativeTests(['--ignored']);
const explicitlyIgnoredTests = new Set([
  't02_external_byte_oracle_cases',
  't02_large_populated_storage_experiment',
]);
const describedTests = new Set([
  ...tests.map(([, name]) => name),
  ...explicitlyIgnoredTests,
]);
const discoveredSet = new Set(discoveredTests);
const ignoredSet = new Set(discoveredIgnoredTests);
const rb02Tests = tests.filter(([group]) => group === 'RB-02-v2').map(([, name]) => name);
if (rb02Tests.length === 0 || rb02Tests.some(name => !discoveredSet.has(name))) {
  throw new Error(JSON.stringify({ canonicalV2ZeroDiscoveryRejected: true, expected: rb02Tests, discovered: discoveredTests }));
}
const missingDescriptors = discoveredTests.filter(name => !describedTests.has(name));
const staleDescriptors = tests
  .map(([, name]) => name)
  .filter(name => !discoveredSet.has(name) && !ignoredSet.has(name));
const unhandledIgnoredTests = discoveredIgnoredTests.filter(name => !explicitlyIgnoredTests.has(name));
if (missingDescriptors.length > 0 || staleDescriptors.length > 0 || unhandledIgnoredTests.length > 0) {
  throw new Error(JSON.stringify({ missingDescriptors, staleDescriptors, unhandledIgnoredTests }));
}
await writeFile(join(output, 'test-discovery.json'), JSON.stringify({
  command: 'cargo test --manifest-path src-tauri/Cargo.toml stack_popup::text_document::feasibility::tests -- --list',
  discovered: discoveredTests,
  ignored: discoveredIgnoredTests,
  described: [...describedTests],
  allDiscoveredTestsHaveRunnerDescriptors: missingDescriptors.length === 0,
  allIgnoredTestsHaveExplicitRunnerHandling: unhandledIgnoredTests.length === 0,
  canonicalV2Schema: 'stack-text-editor.v2',
  canonicalV2Checks: rb02Tests,
  canonicalV2DiscoveryCount: rb02Tests.length,
  canonicalV2ZeroDiscoveryRejected: true,
}, null, 2));

function blockedOutcome(group, test, reason) {
  outcomes.push({ group, test, status: 'BLOCK', command: null, reason });
}

async function runActualWindowProbe() {
  const id = String(commands.length + 1).padStart(3, '0');
  const resultPath = join(output, 'native-caller-result.json');
  const stdoutPath = join(output, `${id}-actual-window-stdout.log`);
  const stderrPath = join(output, `${id}-actual-window-stderr.log`);
  const stdout = createWriteStream(stdoutPath);
  const stderr = createWriteStream(stderrPath);
  const started = new Date().toISOString();
  const child = spawn('cargo', ['run', '--manifest-path', 'src-tauri/Cargo.toml'], {
    stdio: ['ignore', 'pipe', 'pipe'],
    env: {
      ...process.env,
      P02_NATIVE_PROBE_ROOT: output,
      P02_FIXTURE_ROOT: join(output, 'native-fixtures'),
      RUST_BACKTRACE: '1',
    },
    windowsHide: true,
  });
  child.stdout.on('data', chunk => stdout.write(chunk));
  child.stderr.on('data', chunk => stderr.write(chunk));
  let exitCode = null;
  child.once('close', code => { exitCode = code; });
  let result;
  const deadline = Date.now() + 180_000;
  while (Date.now() < deadline && !result) {
    try {
      result = JSON.parse(await readFile(resultPath, 'utf8'));
    } catch {
      await new Promise(resolveSleep => setTimeout(resolveSleep, 500));
    }
  }
  if (!result && exitCode === null) child.kill();
  while (exitCode === null && Date.now() < deadline + 10_000) {
    await new Promise(resolveSleep => setTimeout(resolveSleep, 250));
  }
  await Promise.all([
    new Promise(resolveEnd => stdout.end(resolveEnd)),
    new Promise(resolveEnd => stderr.end(resolveEnd)),
  ]);
  await persistCommandRecord(output, commands, {
    id,
    executable: 'cargo',
    args: ['run', '--manifest-path', 'src-tauri/Cargo.toml'],
    environment: { P02_NATIVE_PROBE_ROOT: output },
    started,
    finished: new Date().toISOString(),
    exitCode,
    matchedPassingTests: result?.status === 'PASS' ? 1 : 0,
    stdout: `${id}-actual-window-stdout.log`,
    stderr: `${id}-actual-window-stderr.log`,
  });
  const passed = result?.status === 'PASS'
    && result.authorizationMode === 'actual-webview-window'
    && result.unauthorizedExistingAndMissingTargetRejected === true
    && result.contentDisclosedOnRejectedOpen === false
    && result.feasibilitySchemaRevision === 'stack-text-editor.p02-feasibility.v2';
  outcomes.push({
    group: 'T02-01',
    test: 'actual_webview_window_authorization',
    status: passed ? 'PASS' : 'BLOCK',
    command: id,
    reason: passed ? undefined : result?.message ?? 'actual WebviewWindow probe did not complete',
  });
  await writeFile(join(output, 'native-caller-result.json'), JSON.stringify(result ?? {
    status: 'BLOCK',
    message: 'actual WebviewWindow probe timed out or exited without evidence',
  }, null, 2));
}

// Independent small oracle: existing decoder owns byte maps. Canonicalize lone
// CR for the frozen lease view only; never normalize untouched source bytes.
const oracleCases = [];
for (const encoding of ['utf8', 'utf16le', 'utf16be']) {
  for (const bom of encoding === 'utf8' ? [false, true] : [true]) {
    const raw = 'first\r\n😀e\u0301\nthird\rlast';
    let bytes = Buffer.from(raw, encoding === 'utf8' ? 'utf8' : 'utf16le');
    if (encoding === 'utf16be') bytes.swap16();
    if (bom) bytes = Buffer.concat([Buffer.from(encoding === 'utf8' ? [0xef,0xbb,0xbf] : encoding === 'utf16le' ? [0xff,0xfe] : [0xfe,0xff]),bytes]);
    const oracle = decodeByteText(bytes);
    const start = 0, end = 5, insert = 'edited\n';
    const inserted = encodeByteText(insert, { encoding, bom: false, newlineStyle: oracle.newlineStyle });
    const expected = Buffer.concat([bytes.subarray(0, oracle.localToByte[start]), inserted, bytes.subarray(oracle.localToByte[end])]);
    oracleCases.push({ bytes: [...bytes], encoding: encoding === 'utf8' ? bom ? 'utf8Bom' : 'utf8' : encoding === 'utf16le' ? 'utf16Le' : 'utf16Be', bom: oracle.bomLength, text: oracle.text.replaceAll('\r','\n'), localToByte: oracle.localToByte, expectedBreaks: 3, editStart: start, editEnd: end, insert, editedBytes: [...expected] });
  }
}
const oraclePath = join(output, 'oracle-cases.json'); await writeFile(oraclePath, JSON.stringify(oracleCases));
// Reuse P01 controlled I/O to prove the input fixture's withheld remainder is not
// materialized by the setup; this is separate from native kernel proof above.
const controlled = new DeterministicRangeIo(Uint8Array.from(oracleCases[0].bytes), { blockSize: 16, withholdAfterOffset: 16, seed });
const controlledPrefix = await controlled.read('0', '16', 'prefix');
const remainderLength = oracleCases[0].bytes.length - 16;
const controlledRemainder = controlled.read('16', String(remainderLength), 'remainder');
assert.equal(controlledPrefix.bytes.length, 16);
assert.equal(controlled.bytesRead, 16);
assert.equal(controlled.pendingCount, 1);
const beforeRelease = { bytesRead: controlled.bytesRead, pending: controlled.pendingCount };
controlled.release('remainder');
assert.equal((await controlledRemainder).bytes.length, remainderLength);
await writeFile(join(output, 'oracle-notes.json'), JSON.stringify({ oracleSource: 'scripts/stack-text-editor/byteTextOracle.mjs', loneCrViewCanonicalization: true, untouchedByteSplice: true, independentlyExpectedMixedBreakCount: 3, noFullOracleReencode: true, controlledIo: { beforeRelease, afterRelease: { bytesRead: controlled.bytesRead, pending: controlled.pendingCount }, readLog: controlled.readLog }, limitations: ['Existing oracle preserves lone CR in view text.', 'Existing mutation oracle re-encodes all EOLs; only insertion encoding and byte boundary maps are reused.'] }, null, 2));
for (const [group,name] of tests) await runTest(group,name);
await runTest('T02-04','t02_external_byte_oracle_cases',{P02_ORACLE_CASES:oraclePath},true);
if (args.includes('--actual-window-probe')) await runActualWindowProbe();
else blockedOutcome('T02-01', 'actual_webview_window_authorization', 'requires --actual-window-probe and real Tauri WebviewWindow instances');
const sizeBytes = { '1MiB': 1024 ** 2, '100MiB': 100 * 1024 ** 2, '1GiB': 1024 ** 3, multiGiB: 3 * 1024 ** 3, overRam: Math.ceil(totalmem() / 65536) * 65536 + 64 * 1024 ** 2 };
const requestedScaleSizes = new Set(sizes.filter(size => size !== 'small'));
for (const size of sizes) {
  if (size === 'small') continue;
  if (!Object.hasOwn(sizeBytes,size)) throw new Error(`unknown size ${size}`);
  try {
    // Existing generator writes populated chunks <=256 KiB and rechecks each file.
    const freeSpace = assertFreeSpace(output,sizeBytes[size] * 6 + 512 * 1024 ** 2);
    const corpus = await generateCorpus({ outputDirectory: join(output,`corpus-${size}`),seed,includeLarge:true,allowLarge:true,largeBytes:sizeBytes[size],smallBytes:4096,chunkSize:65536 });
    await writeCorpusManifest(corpus,join(output,`corpus-${size}`,'manifest.json'));
    await writeFile(join(output,`admission-${size}.json`),JSON.stringify({ size,freeSpace,requestedBytes:String(sizeBytes[size]),overAvailableRam:sizeBytes[size]>totalmem(),sparse:false },null,2));
    const largeFixtures = corpus.fixtures.filter(fixture=>fixture.features.includes('large'));
    for (const fixture of largeFixtures) {
      // Independent streaming oracle for the large UTF-8 fixture's accepted "x"
      // insertion. Never materialize a large document in this driver.
      const editedHash = createHash('sha256').update('x');
      for await (const chunk of createReadStream(fixture.path, { highWaterMark: 65536 })) editedHash.update(chunk);
      await runTest('T02-02/T02-03', 't02_large_populated_storage_experiment', { P02_FIXTURE:fixture.path,P02_FIXTURE_SHA256:fixture.sha256,P02_EDITED_SHA256:editedHash.digest('hex') }, true);
    }
  } catch (error) {
    blockedOutcome('T02-02/T02-03', `populated_${size}`, error instanceof Error ? error.message : String(error));
  }
}
for (const required of ['1MiB', '100MiB', '1GiB']) {
  if (!requestedScaleSizes.has(required)) blockedOutcome('P12/NFR-6', `populated_${required}`, 'deferred P12 populated size was not requested');
}
if (![...requestedScaleSizes].some(size => sizeBytes[size] && sizeBytes[size] >= 1024 ** 2)) {
  blockedOutcome('P12/NFR-6', 'populated_high_line_fixture', 'deferred P12 populated high-line fixture was not executed');
}
if (!requestedScaleSizes.has('multiGiB')) blockedOutcome('P12/NFR-6', 'populated_multiGiB', 'deferred P12 multi-GiB populated run was not executed');
if (!requestedScaleSizes.has('overRam')) blockedOutcome('P12/NFR-6', 'populated_overRam', 'deferred P12 over-RAM populated run was not executed');
const targetPolicyMatrix = [
  {
    targetClass: 'local-ntfs-regular-unnamed-default-stream-single-link',
    open: 'PROVEN',
    edit: 'prefix-before-copy-proven',
    destructiveSave: 'UNPROVEN-P04',
    guarantee: 'authoritative identity, bounded prefix lease, encrypted private backing',
  },
  {
    targetClass: 'readonly-hardlink-reparse-ancestor-reparse-final-named-stream-device-UNC',
    open: 'REFUSED',
    edit: 'REFUSED',
    destructiveSave: 'REFUSED',
    guarantee: 'no source bytes disclosed before target-policy refusal',
  },
  {
    targetClass: 'pre-existing-writable-mapped-writer-or-share-denial',
    open: 'REFUSED',
    edit: 'REFUSED',
    destructiveSave: 'REFUSED',
    guarantee: 'protected source is not advertised when writer exclusion is unproved',
  },
  {
    targetClass: 'non-NTFS-remote-cloud-offline-sparse-compressed-encrypted-special-file',
    open: 'UNSUPPORTED',
    edit: 'READ-LIMITED-OR-REFUSED',
    destructiveSave: 'REFUSED',
    guarantee: 'no feasibility claim outside fixed local NTFS/default-stream scope',
  },
];
const stopGoRecommendation = {
  recommendation: 'STOP',
  reason: 'P02 feasibility evidence is not production promotion approval',
  provenGuarantees: [
    'authorized stack-popup WebviewWindow opens ordinary local source before copy/index',
    'unauthorized top-bar WebviewWindow rejects existing and missing targets without content disclosure',
    'valid prefix remains visible while distant invalid bytes become DecisionRequired',
    'AES-GCM page authentication rejects tampered backing',
    'demand prefix/caret reads preempt background lanes and obsolete seeks are latest-only',
  ],
  unsupportedFilesystemModes: [
    'remote/UNC/cloud/offline/removable/non-NTFS targets',
    'reparse ancestors/final reparse points, named streams, devices',
    'sparse/compressed/encrypted/special files',
  ],
  unsupportedWriterModes: [
    'pre-existing writable mapped views',
    'ordinary conflicting writers/share denial',
    'any writer mode without independently proven exclusion',
  ],
  pendingPromotionProof: [
    'independent storage/security review and coordinator signoff',
    'P04 recovery-root verification',
  ],
  postLandingDebt: ['P12 populated scale and NFR-6 acceptance/refutation'],
};
await writeFile(join(output, 'target-policy.json'), JSON.stringify(targetPolicyMatrix, null, 2));
await writeFile(join(output, 'recommendation.json'), JSON.stringify(stopGoRecommendation, null, 2));
const matrix = ['T02-01','T02-02','T02-03','T02-04','T02-05'].map(test => {
  const relevant=outcomes.filter(outcome=>outcome.group.includes(test));
  return { test,execution:relevant.some(outcome=>outcome.status==='FAIL')?'FAIL':relevant.some(outcome=>outcome.status==='BLOCK')?'BLOCK':'PASS',gate:'IN_REVIEW',evidence:relevant.map(outcome=>outcome.command).filter(command=>command !== null) };
});
const blockers = [
  'No phase acceptance: independent storage/security review and coordinator signoff required.',
  'Actual Tauri cross-window source-open authorization needs native integration review; Rust entry accepts only WebviewWindow, never a request label.',
  'SnapshotComplete is not recoveryComplete: P04 recovery-root verification remains outside P02.',
  ...(sizes.includes('overRam') ? [] : ['Over-RAM populated run not requested; retained as non-gate P12/NFR-6 debt.']),
  ...(sizes.includes('multiGiB') ? [] : ['Multi-GiB populated run not requested; retained as non-gate P12/NFR-6 debt.']),
  ...(args.includes('--actual-window-probe') ? [] : ['Actual WebviewWindow authorization probe not executed; T02-01 gate BLOCKED.']),
];
const reviewPacket = {
  scope: 'P02 test-only non-scale feasibility and RB-02 canonical-v2 compatibility',
  storageSafety: {
    status: 'PENDING-INDEPENDENT-REVIEW',
    evidence: ['authenticated encrypted backing', 'tamper refusal', 'quota refusal retains readable dirty root', 'pre-existing mapped writer refused'],
  },
  securityPrivacy: {
    status: 'PENDING-INDEPENDENT-REVIEW',
    nativeAuthorizationEvidence: outcomes.find(outcome => outcome.test === 'actual_webview_window_authorization')?.status ?? 'BLOCK',
    expectedAuthorizedWindow: 'stack-popup',
    expectedUnauthorizedWindow: 'top-bar',
    sourceTextLogged: false,
    sourceSnapshotContainsFixtures: false,
  },
  completionSemantics: {
    snapshotCompleteIsRecoveryComplete: false,
    snapshotComplete: true,
    recoveryComplete: false,
    recoveryOwner: 'P04',
  },
  scale: { status: 'DEFERRED-P12-NFR-6', gateForThisNonScaleRun: false },
  coordinatorDisposition: 'PENDING',
  gateDisposition: 'BLOCKED-PENDING-REVIEWS-COORDINATOR-AND-P04',
};
await writeFile(join(output, 'review-packet.json'), JSON.stringify(reviewPacket, null, 2));
const nonScaleOutcomes = outcomes.filter(outcome => outcome.group !== 'P12/NFR-6');
const status = nonScaleOutcomes.some(outcome=>outcome.status==='FAIL')?'FAIL':nonScaleOutcomes.some(outcome=>outcome.status==='BLOCK')?'BLOCK':'IN_REVIEW';
await writeFile(join(output,'matrix.json'),JSON.stringify({ phase:'P02',status,matrix,outcomes,blockers,seed,sourceManifest:'source-manifest.json',commands:'commands.json',observations:'observations.json',targetPolicy:'target-policy.json',recommendation:'recommendation.json',reviewPacket:'review-packet.json',schemaRevision:'stack-text-editor.v1',feasibilitySchemaRevision:'stack-text-editor.p02-feasibility.v2',canonicalRevalidationSchemaRevision:'stack-text-editor.v2',canonicalRevalidationChecks:rb02Tests,noSchemaChanges:false,schemaNote:'Legacy v1/local-v2 truth is retained; canonical v2 evidence is limited to isolated RB-02 tests and is not runtime or production proof.' },null,2));
console.log(JSON.stringify({
  output,
  outcomes: outcomes.length,
  failed: outcomes.filter(outcome=>outcome.status==='FAIL').length,
  blocked: outcomes.filter(outcome=>outcome.status==='BLOCK').length,
  status,
  blockers,
},null,2));
if(nonScaleOutcomes.some(outcome=>outcome.status!=='PASS'))process.exitCode=1;
