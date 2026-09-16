import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import {
  PROJECTION_LIMITS, FileRangeProvider, ProjectionSession, decodeSegment, decodeSplitSegments, localAtByte,
} from '../dist-tests/features/stack-browser/textEditorProjectionExperiment.js';

test('T03-01 decodes BOM UTF-8/16, surrogate and mixed EOL mappings without overflow', () => {
  const vectors = [
    [Buffer.from([0xef, 0xbb, 0xbf, 0x41, 0xf0, 0x9f, 0x98, 0x80, 13, 10, 0x42, 13, 0x43, 10]), 'utf8', 3, 'A😀\nB\nC\n', [3, 5, 7]],
    [Buffer.from([0xff, 0xfe, 0x41, 0, 0x3d, 0xd8, 0, 0xde, 13, 0, 10, 0]), 'utf16le', 2, 'A😀\n', [3]],
    [Buffer.from([0xfe, 0xff, 0, 0x41, 0xd8, 0x3d, 0xde, 0, 0, 13, 0, 10]), 'utf16be', 2, 'A😀\n', [3]],
  ];
  for (const [bytes, encoding, bom, text, newlineOffsets] of vectors) {
    const decoded = decodeSegment(bytes, encoding, bom);
    assert.equal(decoded.text, text);
    assert.deepEqual(decoded.newlines.map((line) => line.localOffset), newlineOffsets);
    assert.equal(decoded.boundaries.at(-1).byte, bytes.length);
    assert.equal(decoded.boundaries.some((point) => point.local === 2), false);
    for (const boundary of decoded.boundaries) {
      assert.equal(localAtByte(decoded, boundary.byte), boundary.local);
      assert.equal(decoded.boundaries.find(point => point.local === boundary.local).byte, boundary.byte);
    }
  }
  const split = decodeSplitSegments([Buffer.from('left\r'), Buffer.from('\n😀right')], 'utf8');
  assert.equal(split.text, 'left\n😀right');
  assert.deepEqual(split.newlines, [{ localOffset: 4, kind: 'crlf' }]);
  assert.equal(localAtByte(split, 6), 5);
  assert.throws(() => localAtByte(split, 7), /InvalidTextBoundary/);
  assert.throws(() => decodeSegment(Buffer.alloc((PROJECTION_LIMITS.units + 1) * 2, 65), 'utf16le', 0), /ResourceLimit/);
});

test('T03-02 cold unknown-line lease is real and editable', async () => {
  const session = ProjectionSession.fromBytes(Buffer.from('cold 😀 text'), { lineState: 'indexing' });
  await session.seek(0);
  assert.deepEqual(session.line, { state: 'indexing', count: null });
  session.edit(4, 5, ' far ');
  assert.equal(session.visibleText, 'cold far 😀 text');
});

test('T03-03 caps two active/four total credits and retains only latest seek through stale release', async () => {
  const waits = [];
  const provider = { read(start) { return new Promise(resolve => waits.push({ start, resolve })); } };
  const session = new ProjectionSession(provider);
  const held1 = session.seek(0); waits.shift().resolve(Buffer.from('abcdef')); const lease1 = await held1;
  const held2 = session.seek(1); waits.shift().resolve(Buffer.from('bcdef')); const lease2 = await held2;
  const stale3 = session.seek(2); const stale4 = session.seek(3);
  assert.equal(session.metrics.activeReads, 2);
  assert.equal(session.metrics.totalCredits, 4);
  const cancelled = session.seek(4);
  const latest = session.seek(5);
  await assert.rejects(cancelled, /Cancelled/);
  assert.equal(session.metrics.pendingSeeks, 1);
  waits.find(wait => wait.start === 2).resolve(Buffer.from('cdef'));
  await assert.rejects(stale3, /StaleRevision/);
  assert.equal(waits.some(wait => wait.start === 5), true);
  waits.find(wait => wait.start === 3).resolve(Buffer.from('def'));
  await assert.rejects(stale4, /StaleRevision/);
  waits.find(wait => wait.start === 5).resolve(Buffer.from('f'));
  const newest = await latest;
  assert.equal(newest.segments[0].text, 'f');
  assert.equal(session.metrics.maxActiveReads, 2);
  assert.equal(session.metrics.maxTotalCredits, 4);
  session.releaseLease(lease1.leaseId); session.releaseLease(lease2.leaseId);
  assert.equal(session.metrics.totalCredits, 1);
  session.releaseLease(lease1.leaseId);
  assert.equal(session.metrics.totalCredits, 1);
  assert.throws(() => session.acceptLease({ ...newest, viewGeneration: '1' }), /StaleRevision/);
});

test('T03-05 delayed ACK keeps immediate echo, one flight, bounded queue, no hydration history', async () => {
  const session = ProjectionSession.fromBytes(Buffer.from('x'));
  await session.seek(0);
  session.hydrate(session.lease);
  assert.equal(session.historyDepth, 0);
  session.edit(1, 1, 'a'); session.edit(2, 2, 'b');
  assert.equal(session.visibleText, 'xab');
  assert.equal(session.inFlightCount, 1);
  assert.equal(session.pendingOperationCount, 1);
  for (let i = 0; i < PROJECTION_LIMITS.operations - 2; i++) session.edit(session.visibleText.length, session.visibleText.length, 'z');
  assert.throws(() => session.edit(0, 0, 'q'), /ResourceLimit/);
  session.ack();
  assert.equal(session.inFlightCount, 1);

  const byteSession = ProjectionSession.fromBytes(Buffer.from('x'));
  await byteSession.seek(0);
  const exactLimit = 'é'.repeat(PROJECTION_LIMITS.bytes / 2);
  byteSession.edit(1, 1, exactLimit);
  const retained = byteSession.documentText();
  assert.equal(byteSession.pendingInputBytes, PROJECTION_LIMITS.bytes);
  assert.throws(() => byteSession.edit(0, 0, 'x'), /ResourceLimit/);
  assert.equal(byteSession.documentText(), retained);
  assert.equal(byteSession.pendingInputBytes, PROJECTION_LIMITS.bytes);
  byteSession.ack();
  byteSession.edit(0, 0, 'x');
});

test('T03-06 canonical cross-lease endpoints survive history/ACK paging and reject forged/stale/released/expired owners', async () => {
  const session = ProjectionSession.fromBytes(Buffer.from('left|right'));
  const loaded = await session.seek(0);
  const left = { ...loaded, leaseId: 'left', segments: [{ encoding: 'utf8', byteStart: '0', text: 'left|', newlines: [] }] };
  const right = { ...loaded, leaseId: 'right', segments: [{ encoding: 'utf8', byteStart: '5', text: 'right', newlines: [] }] };
  session.pinLease(left); session.pinLease(right);
  const point = (lease, offset) => ({ lease, segment: 0, offset, affinity: 'before' });
  session.select('selection1', point(left, 2), point(right, 3));
  assert.equal(session.exportSelection('selection1'), 'ft|rig');
  session.replaceSelection('selection1', 'X');
  assert.equal(session.documentText(), 'leXht');
  session.undo(); assert.equal(session.documentText(), 'left|right');
  session.redo(); assert.equal(session.documentText(), 'leXht');
  session.hydrate(right);
  assert.equal(session.documentText(), 'leXht');
  session.ack();
  assert.equal(session.documentText(), 'leXht');
  assert.throws(() => session.select('forged', point({ ...left, sessionId: 'other' }, 0), point(right, 1)), /Unauthorized/);
  session.releaseLease(left.leaseId);
  assert.throws(() => session.exportSelection('selection1'), /Unauthorized/);
  session.pinLease(left);
  session.releaseSelection('selection1');
  assert.throws(() => session.exportSelection('selection1'), /Unauthorized/);
  session.select('selection2', point(left, 0), point(left, 1)); session.expireSelections();
  assert.throws(() => session.exportSelection('selection2'), /StaleRevision/);
});

test('T03-06 authoritative pinned lease rejects changed source generation exactly', async () => {
  const session = ProjectionSession.fromBytes(Buffer.from('text'));
  const lease = await session.seek(0);
  lease.sourceGeneration = 'source2';
  const point = { lease, segment: 0, offset: 0, affinity: 'before' };
  assert.throws(() => session.select('changed-source', point, point), error => error.message === 'SourceChanged');
});

test('T03-06 authoritative pinned lease rejects stale document revision exactly', async () => {
  const session = ProjectionSession.fromBytes(Buffer.from('text'));
  const lease = await session.seek(0);
  lease.documentRevision = '1';
  const point = { lease, segment: 0, offset: 0, affinity: 'before' };
  assert.throws(() => session.select('stale-revision', point, point), error => error.message === 'StaleRevision');
});

test('T03-07 file-backed complete 20,001-unit grapheme and bidi copy stays bounded', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'p03-'));
  const path = join(directory, 'giant.txt');
  const text = `a${'\u0301'.repeat(20000)} אבג`;
  await writeFile(path, text, 'utf8');
  const provider = new FileRangeProvider(path);
  try {
    const session = new ProjectionSession(provider);
    await session.seek(0);
    assert.equal(session.visibleText.slice(0, 20001).length, 20001);
    session.select('giant', { lease: session.lease, segment: 0, offset: 0, affinity: 'before' }, { lease: session.lease, segment: 0, offset: 20001, affinity: 'before' });
    assert.equal(session.exportSelection('giant'), text.slice(0, 20001));
    assert.equal(session.metrics.maxResidentBytes <= PROJECTION_LIMITS.units * 4, true);
    assert.equal(session.visibleText.includes('\n'), false);
  } finally { await provider.close(); await rm(directory, { recursive: true, force: true }); }
});

async function scanProductIsolation(root, entry) {
  const visited = new Set(), failures = [];
  async function visit(file) {
    if (visited.has(file)) return; visited.add(file);
    const source = await readFile(file, 'utf8');
    if (/textEditorProjectionExperiment/.test(source)) failures.push(`experiment reachable: ${file}`);
    if (/(aria|accessib\w*)\w*\s*=\s*(documentText|fullText|sourceText)|\.join\(['"]['"]\).*aria/i.test(source)) failures.push(`full a11y buffer: ${file}`);
    for (const match of source.matchAll(/(?:import|export)\s+(?:[^'";]+?\s+from\s+)?['"]([^'"]+)['"]/g)) {
      if (!match[1].startsWith('.')) continue;
      const base = join(file, '..', match[1]);
      for (const candidate of [base, `${base}.ts`, `${base}.js`, `${base}.svelte`, join(base, 'index.ts')]) {
        try { await visit(candidate); break; } catch (error) { if (error.code !== 'ENOENT') throw error; }
      }
    }
  }
  await visit(join(root, entry));
  return failures;
}

test('T03-09 executable product graph/a11y isolation scanner catches violations and current product stays isolated', async () => {
  const source = Buffer.alloc(PROJECTION_LIMITS.units * 8, 97);
  const session = ProjectionSession.fromBytes(source, { withholdAfter: 1024 });
  await session.seek(0);
  session.edit(0, 1, 'b');
  assert.equal(session.visibleText[0], 'b');
  assert.equal(session.metrics.bytesRead <= 1024, true);
  await assert.rejects(session.seek(2048), /WithheldRead/);

  const root = process.cwd();
  assert.deepEqual(await scanProductIsolation(root, 'src/main.ts'), []);
  const directory = await mkdtemp(join(tmpdir(), 'p03-scan-'));
  try {
    await mkdir(join(directory, 'src'));
    await writeFile(join(directory, 'src', 'main.ts'), "import './bad.js';\n");
    await writeFile(join(directory, 'src', 'bad.js'), "import './textEditorProjectionExperiment.js';\nconst ariaMirror = fullText;\n");
    await writeFile(join(directory, 'src', 'textEditorProjectionExperiment.js'), 'export {};\n');
    const failures = await scanProductIsolation(directory, 'src/main.ts');
    assert.equal(failures.some(value => value.startsWith('experiment reachable:')), true);
    assert.equal(failures.some(value => value.startsWith('full a11y buffer:')), true);
  } finally { await rm(directory, { recursive: true, force: true }); }
});
