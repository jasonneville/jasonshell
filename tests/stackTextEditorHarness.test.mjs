import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { corpus, writeFixture, ByteOracle, ControlledReads, publish, Credits, Measurements } from '../scripts/stack-text-editor/harness.mjs';

test('seeded corpus preserves complete UTF-8/UTF-16 patterns/BOM and JSON framing', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'stack-p01-'));
  try {
    for (const kind of ['utf8', 'utf16le', 'utf16be', 'minified', 'giant-line', 'many-lines', 'combining', 'invalid-tail']) {
      const a = join(dir, `${kind}-a`), b = join(dir, `${kind}-b`);
      const first = await writeFixture(a, { kind, seed: 305419896, bytes: 65537 });
      const second = await writeFixture(b, { kind, seed: 305419896, bytes: 65537 });
      assert.equal(first.sha256, second.sha256); assert.ok(first.maxChunk <= 65536);
      const bytes = await readFile(a);
      assert.equal(first.sha256, createHash('sha256').update(bytes).digest('hex'));
      if (kind === 'minified') JSON.parse(bytes.toString('utf8'));
      else if (kind !== 'invalid-tail') {
        const encoding = kind === 'utf16le' || kind === 'utf16be' ? kind.replace('utf16', 'utf-16') : 'utf-8';
        new TextDecoder(encoding, { fatal: true }).decode(bytes);
      } else assert.throws(() => new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    }
  } finally { await rm(dir, { recursive: true }); }
});
test('corpus metadata and first chunk independent of requested file size', () => {
  const small = corpus({ kind: 'giant-line', seed: 1, bytes: 100000 });
  const huge = corpus({ kind: 'giant-line', seed: 1, bytes: 2 ** 40 });
  assert.deepEqual(small.next().value, huge.next().value);
  assert.throws(() => corpus({ kind: 'bad', seed: 1, bytes: 1 }).next());
});
test('small byte oracle random operations match independent splice and preserve rejection', () => {
  const oracle = new ByteOracle(Buffer.from('A😀\r\nZ'));
  assert.throws(() => oracle.replace(2, 3, Buffer.from('x')));
  let expected = Buffer.from('A😀\r\nZ');
  for (let n = 0; n < 100; n++) {
    const insert = Buffer.from(String(n % 10));
    oracle.replace(0, 0, insert); expected = Buffer.concat([insert, expected]);
    assert.deepEqual(oracle.bytes(), expected);
  }
  oracle.undo(); expected = expected.subarray(1); assert.deepEqual(oracle.bytes(), expected);
  oracle.redo(); assert.equal(oracle.bytes().length, expected.length + 1);
});
test('controlled range I/O withholds EOF, reorders completions and reports failures', async () => {
  const io = new ControlledReads(Buffer.from('abcdef'));
  const first = io.read('r1', 0, 2), last = io.read('r2', 4, 2);
  io.release('r1'); assert.deepEqual(await first, Buffer.from('ab')); assert.equal(io.eof, false);
  io.fail('r2', 'IoFailure'); await assert.rejects(last, /IoFailure/); assert.equal(io.eof, false);
  const a = io.read('r3', 2, 2), b = io.read('r4', 4, 2);
  io.release('r4'); await b; io.release('r3'); await a; assert.equal(io.eof, true);
  assert.deepEqual(io.order, ['r1', 'r2', 'r4', 'r3']);
});
test('oracle preserves UTF BOMs and rejects encoded scalar/CRLF interiors', () => {
  for (const encoding of ['utf-8', 'utf-16le', 'utf-16be']) {
    const bom = Buffer.from(encoding === 'utf-8' ? [239, 187, 191] : encoding === 'utf-16le' ? [255, 254] : [254, 255]);
    const encode = text => { const bytes = Buffer.from(text, encoding === 'utf-8' ? 'utf8' : 'utf16le'); return encoding === 'utf-16be' ? bytes.swap16() : bytes; };
    const original = Buffer.concat([bom, encode('A\u{1f600}\r\nZ')]);
    const oracle = new ByteOracle(original, encoding);
    assert.throws(() => oracle.replace(0, bom.length, Buffer.alloc(0)), /InvalidTextBoundary/);
    const emoji = bom.length + encode('A').length;
    assert.throws(() => oracle.replace(emoji + 1, emoji + 2, encode('x')), /InvalidTextBoundary/);
    const crlf = emoji + encode('\u{1f600}\r').length;
    assert.throws(() => oracle.replace(crlf, crlf, encode('x')), /InvalidTextBoundary/);
    oracle.replace(emoji, emoji + encode('\u{1f600}').length, encode('B'));
    assert.deepEqual(oracle.bytes(), Buffer.concat([bom, encode('AB\r\nZ')]));
    oracle.undo(); assert.deepEqual(oracle.bytes(), original);
  }
  assert.throws(() => new ByteOracle(Buffer.from('x'), 'windows-1252'), /InvalidEncoding/);
});
test('publication failpoints never disguise prepublication failure or postpublication ambiguity', async () => {
  for (const phase of ['write', 'flush', 'publish', 'inspect', 'journalCommit', 'cleanup']) {
    const sink = { bytes: Buffer.from('old') };
    const outcome = await publish(sink, Buffer.from('new'), p => { if (p === phase) throw Error('DiskFull'); });
    const published = ['inspect', 'journalCommit', 'cleanup'].includes(phase);
    assert.equal(sink.bytes.toString(), published ? 'new' : 'old');
    assert.equal(outcome.state, published ? 'ambiguous' : 'failed'); assert.equal(outcome.durable, false);
    assert.equal(outcome.cancellation, published ? 'ambiguous' : 'none');
  }
  const sink = { bytes: Buffer.from('old') };
  assert.equal((await publish(sink, Buffer.from('new'), p => p === 'write' ? 'cancel' : undefined)).state, 'cancelled');
  assert.equal(sink.bytes.toString(), 'old');
  assert.equal((await publish(sink, Buffer.from('new'), p => p === 'cleanup' ? 'cancel' : undefined)).cancellation, 'already-published');
});
test('credits bound blocked work, keep latest demand and reject unavailable workers', () => {
  const flow = new Credits();
  assert.equal(flow.request('a'), 'started'); assert.equal(flow.request('b'), 'started');
  assert.equal(flow.request('c'), 'queued'); assert.equal(flow.request('d'), 'queued'); assert.equal(flow.pending, 'd');
  flow.complete('a'); assert.equal(flow.pending, null); assert.ok(flow.active.has('d'));
  flow.complete('b'); flow.complete('d'); assert.equal(flow.held.size, 3);
  flow.request('e'); flow.complete('e'); assert.equal(flow.request('f'), 'queued');
  flow.release('a'); assert.ok(flow.active.has('f')); flow.release('a');
  flow.close(); assert.equal(flow.active.size + flow.held.size, 0);
  assert.throws(() => flow.request('g'), /Cancelled/);
  assert.throws(() => new Credits(false).request('a'), /WorkerUnavailable/);
});
test('metadata allowlist rejects source text at record and serialization boundaries', () => {
  const log = new Measurements();
  log.record('intent', 0); log.record('firstPaintCallback', 10, { bytesRead: 128 }); log.record('localMutation', 12);
  log.record('ack', 1000, { revision: '1' }); log.record('durable', 1001, { revision: '1' });
  assert.equal(log.summary().nativeEvidence, 'blocked'); assert.equal(log.summary().ackDelayMs, 988);
  for (const metadata of [{ sourceText: 'secret' }, { bytesRead: NaN }, { phase: 'secret' }, { revision: '01' }, { nested: {} }, { requestId: 'prose words' }]) {
    assert.throws(() => log.record('ack', 1002, metadata));
  }
  assert.throws(() => log.record('unknown', 1002)); assert.throws(() => log.record('eof', 1));
  assert.ok(!log.json().includes('secret')); assert.equal(log.summary().eofObserved, false);
  assert.throws(() => log.record('ack', 1002, { bytesRead: 128 }));
  assert.throws(() => log.record('intent', 1002, { revision: '1' }));
  assert.throws(() => log.record('firstPaintCallback', 1002));
});
