import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readSource, setOracle, surfaceFunction, trackerClass } from './terminalRetention.fixture.mjs';

// Reproducible Set decision oracle vs CURRENT actual-source tracker/writer.
// Historical production copy counts remain in terminalRetention.evidence.json;
// they were observed before migration, not inferred from this oracle.
for (const path of ['src/lib/terminalSequenceTracker.ts', 'src/components/TerminalPanelSurface.svelte', 'src/components/StackTerminalPane.svelte', 'src/lib/stackPopup.ts', 'src/lib/persistentTerminal.ts']) {
  console.info(JSON.stringify({ path, sha256: createHash('sha256').update(readSource(path)).digest('hex') }));
}
for (const chunks of [100, 1000, 10000]) {
  let copies = 0;
  let copiedKeys = 0;
  let constructions = 0;
  class MeasuredSet extends Set {
    constructor(input) { super(input); if (input) { copies += 1; copiedKeys += this.size; } }
  }
  const Tracker = trackerClass({ Set: MeasuredSet });
  class MeasuredTracker extends Tracker { constructor() { super(); constructions += 1; } }
  const cache = new Map();
  const lookup = surfaceFunction('src/components/TerminalPanelSurface.svelte', 'sequenceTrackerForSession', { TerminalSequenceTracker: MeasuredTracker, renderedSequenceKeysBySession: cache }).invoke;
  const tracker = lookup('baseline');
  const runtime = { session: { sessionId: 'baseline' }, renderedSequences: tracker };
  let writes = '';
  const writer = surfaceFunction('src/components/TerminalPanelSurface.svelte', 'writeTerminalChunkForRuntime', {
    Set: MeasuredSet, isRuntimeCurrent: () => true, writeTerminalOutputForRuntime: (_runtime, text) => { writes += text; }
  });
  const hidden = surfaceFunction('src/components/TerminalPanelSurface.svelte', 'rememberTerminalChunkForSession', {
    Set: MeasuredSet, sequenceTrackerForSession: lookup, appendSessionReplayBuffer: (_session, text) => { writes += text; }
  });
  const oracle = setOracle();
  let oracleWrites = '';
  for (let sequence = 0; sequence < chunks; sequence += 1) {
    const chunk = { sessionId: 'baseline', sequence, stream: 'stdout', text: 'x' };
    writer.invoke(runtime, chunk);
    if (oracle(chunk.sessionId, chunk.stream, chunk.sequence)) oracleWrites += chunk.text;
  }
  for (let sequence = 0; sequence < chunks * 2; sequence += 1) {
    const chunk = { sessionId: 'baseline', sequence, stream: 'stdout', text: 'h' };
    hidden.invoke(chunk);
    hidden.invoke(chunk);
    if (oracle(chunk.sessionId, chunk.stream, chunk.sequence)) oracleWrites += chunk.text;
  }
  // Replacement runtime gets the same canonical session tracker, not a copy.
  const replacement = { session: runtime.session, renderedSequences: lookup('baseline') };
  assert.equal(replacement.renderedSequences, tracker);
  writer.invoke(replacement, { sessionId: 'baseline', stream: 'stdout', sequence: 0, text: 'duplicate' });
  assert.equal(writes, oracleWrites);
  assert.equal(constructions, 1);
  assert.equal(copies, 0);
  assert.equal(copiedKeys, 0);
  assert.equal(tracker.stats().rangeCount, 1);
  assert.equal(tracker.stats().fallbackKeyCount, 0);
  console.info(JSON.stringify({ chunks, uniqueChunks: chunks * 2, trackerConstructions: constructions, copies, copiedKeys, ...tracker.stats(), identicalOracleWrites: writes.length }));
}

const chunks = Array.from({ length: 100 }, (_, sequence) => ({ sessionId: 'session-1', sequence, text: 'x'.repeat(100), stream: 'stdout' }));
const nativePoll = { sessionId: 'session-1', cwd: 'C:\\dev', running: true, chunks };
const calls = [];
const adapter = surfaceFunction('src/lib/stackPopup.ts', 'pollStackTerminal', {
  IPC_COMMANDS: { pollStackTerminalSession: 'poll_stack_terminal_session' },
  invoke: async (...args) => { calls.push(args); return nativePoll; }
});
const result = await adapter.invoke('session-1');
assert.equal(result.chunks, chunks);
assert.equal(result.exited, false);
assert.equal('output' in result, false);
assert.deepEqual(JSON.parse(JSON.stringify(calls)), [['poll_stack_terminal_session', { sessionId: 'session-1' }]]);
const legacy = { sessionId: 'session-1', cwd: 'C:\\dev', output: chunks.map((chunk) => chunk.text).join(''), chunks, exited: false, exitCode: null };
const pollBytes = Buffer.byteLength(JSON.stringify(nativePoll));
const legacyBytes = Buffer.byteLength(JSON.stringify(legacy));
assert.ok(pollBytes < legacyBytes);
console.info(JSON.stringify({ chunks: chunks.length, pollBytes, legacyBytes, savedBytes: legacyBytes - pollBytes }));
