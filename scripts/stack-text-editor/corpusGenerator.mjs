import { createHash } from 'node:crypto';
import { statfsSync } from 'node:fs';
import { mkdir, open, stat } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { deterministicRandom, encodeByteText } from './byteTextOracle.mjs';

export const CORPUS_SCHEMA_VERSION = 'stack-text-corpus.v1';
export const MAX_CHUNK_BYTES = 256 * 1024;
export const DEFAULT_SEED = 0x51a7e;
export const T01_02_PASSING_CRITERIA = Object.freeze([
  'seeded fixture bytes and hashes are reproducible',
  'fixture writes use chunks no larger than 256 KiB',
  'large generation is explicit and requires a passing free-space check',
  'withheld/reordered/failed reads preserve deterministic ordering and typed failure'
]);

const encoder = new TextEncoder();

function bytesForPattern(text, encoding = 'utf8', bom = false, newlineStyle = '\n') {
  return encodeByteText(text, { encoding, bom, newlineStyle });
}


function fixtureText(kind, seed) {
  const random = deterministicRandom(seed);
  const token = Math.floor(random() * 0xffffffff).toString(16).padStart(8, '0');
  switch (kind) {
    case 'utf8-basic': return `seed=${token}\nplain ASCII\naccent: café\nnonBMP: 𐐷😀\n`;
    case 'crlf-nonbmp': return `one\r\ntwo\r\nemoji 😀\r\ncombining e\u0301\r\n`;
    case 'utf16-text': return `UTF-16 seed ${token}\r\nLE and BE\r\n𐐷\r\n`;
    case 'high-line-count': return `line-${token}\n`;
    case 'giant-line': return `giant-${token}-`;
    case 'minified-json': return `{"seed":"${token}","items":[1,2,{"ok":true}],"unicode":"😀"}`;
    case 'csv-multiline': return `name,quote\r\nAda,"line one\nline two"\r\nZoë,"emoji 😀"\r\n`;
    case 'mixed-scripts': return `Latin Ελληνικά Кириллица العربية हिन्दी 日本語 한국어 עברית\n`;
    case 'source-like': return `export function seeded_${token}() {\n  return "safe text 😀";\n}\n`;
    default: throw new Error(`unknown corpus fixture kind: ${kind}`);
  }
}

function defaultDescriptors(seed, options = {}) {
  const smallBytes = Math.max(1, Number(options.smallBytes ?? 4096));
  const descriptors = [
    { id: 'utf8-basic', kind: 'utf8-basic', encoding: 'utf8', bom: false, targetBytes: smallBytes, features: ['utf8', 'nonBMP', 'CRLF-capable'] },
    { id: 'utf8-bom-crlf', kind: 'crlf-nonbmp', encoding: 'utf8', bom: true, targetBytes: smallBytes, features: ['utf8', 'BOM', 'CRLF', 'nonBMP'] },
    { id: 'utf16le-bom', kind: 'utf16-text', encoding: 'utf16le', bom: true, targetBytes: smallBytes, features: ['utf16le', 'BOM', 'nonBMP'] },
    { id: 'utf16be-bom', kind: 'utf16-text', encoding: 'utf16be', bom: true, targetBytes: smallBytes, features: ['utf16be', 'BOM', 'nonBMP'] },
    { id: 'invalid-utf8-tail', kind: 'utf8-basic', encoding: 'utf8', bom: false, targetBytes: smallBytes, invalidTail: [0xe2, 0x28, 0xa1], features: ['utf8', 'invalid-tail'] },
    { id: 'invalid-utf16le-tail', kind: 'utf16-text', encoding: 'utf16le', bom: true, targetBytes: smallBytes + 1, invalidTail: [0x00], features: ['utf16le', 'invalid-tail'] },
    { id: 'invalid-utf16be-tail', kind: 'utf16-text', encoding: 'utf16be', bom: true, targetBytes: smallBytes + 1, invalidTail: [0x00], features: ['utf16be', 'invalid-tail'] },
    { id: 'high-line-count', kind: 'high-line-count', encoding: 'utf8', bom: false, targetBytes: smallBytes, features: ['high-line-count'] },
    { id: 'giant-line', kind: 'giant-line', encoding: 'utf8', bom: false, targetBytes: smallBytes, features: ['giant-line', 'no-final-newline'] },
    { id: 'minified-json', kind: 'minified-json', encoding: 'utf8', bom: false, targetBytes: smallBytes, features: ['minified-json'] },
    { id: 'csv-multiline', kind: 'csv-multiline', encoding: 'utf8', bom: false, targetBytes: smallBytes, features: ['csv', 'quoted-multiline'] },
    { id: 'mixed-scripts', kind: 'mixed-scripts', encoding: 'utf8', bom: false, targetBytes: smallBytes, features: ['mixed-scripts', 'nonBMP'] },
    { id: 'source-like', kind: 'source-like', encoding: 'utf8', bom: false, targetBytes: smallBytes, features: ['source-like'] }
  ];
  if (options.includeLarge) {
    const largeBytes = Math.max(MAX_CHUNK_BYTES + 1, Number(options.largeBytes ?? 1024 * 1024));
    descriptors.push(
      { id: 'large-high-line-count', kind: 'high-line-count', encoding: 'utf8', bom: false, targetBytes: largeBytes, features: ['large', 'high-line-count'] },
      { id: 'large-giant-line', kind: 'giant-line', encoding: 'utf8', bom: false, targetBytes: largeBytes, features: ['large', 'giant-line', 'no-final-newline'] }
    );
  }
  return descriptors.map((descriptor, index) => {
    const seeded = { ...descriptor, seed: (Number(seed) + index * 0x9e3779b9) >>> 0 };
    const { pattern, prefix } = fixtureLayout(seeded);
    const tailBytes = seeded.invalidTail?.length ?? 0;
    // Complete patterns avoid accidental invalid scalars/newlines in valid fixtures.
    const repeats = Math.max(1, Math.ceil((seeded.targetBytes - prefix.length - tailBytes) / pattern.length));
    return { ...seeded, targetBytes: prefix.length + repeats * pattern.length + tailBytes };
  });
}

export function buildCorpusPlan(seed = DEFAULT_SEED, options = {}) {
  if (!Number.isInteger(Number(seed)) || Number(seed) < 0) throw new TypeError('seed must be a non-negative integer');
  return {
    schemaVersion: CORPUS_SCHEMA_VERSION,
    seed: Number(seed) >>> 0,
    maxChunkBytes: MAX_CHUNK_BYTES,
    descriptors: defaultDescriptors(seed, options)
  };
}

export function getAvailableBytes(directory) {
  try {
    const result = statfsSync(directory);
    return Number(result.bavail) * Number(result.bsize);
  } catch {
    return null;
  }
}

export function assertFreeSpace(directory, requiredBytes, options = {}) {
  const required = Math.max(0, Number(requiredBytes));
  const available = typeof options.freeSpaceCheck === 'function'
    ? Number(options.freeSpaceCheck(directory, required))
    : getAvailableBytes(directory);
  if (!Number.isFinite(available)) throw new Error('free-space check unavailable; explicit large-fixture generation is refused');
  if (available < required) throw new Error(`insufficient free space for requested corpus (${available} < ${required})`);
  return { checked: true, availableBytes: available, requiredBytes: required };
}

function fixtureLayout(descriptor) {
  const text = fixtureText(descriptor.kind, descriptor.seed);
  const pattern = descriptor.kind === 'minified-json'
    ? encoder.encode(`${text},`)
    : bytesForPattern(text, descriptor.encoding, false, descriptor.encoding !== 'utf8' || descriptor.kind === 'crlf-nonbmp' ? '\r\n' : '\n');
  const prefix = descriptor.kind === 'minified-json' ? encoder.encode('[')
    : descriptor.bom
      ? descriptor.encoding === 'utf8' ? Uint8Array.from([0xef, 0xbb, 0xbf]) : descriptor.encoding === 'utf16le' ? Uint8Array.from([0xff, 0xfe]) : Uint8Array.from([0xfe, 0xff])
      : new Uint8Array();
  return { pattern, prefix };
}

function createChunkFactory(descriptor) {
  const { pattern, prefix } = fixtureLayout(descriptor);
  return (offset, length) => {
    const output = new Uint8Array(length);
    const tailStart = descriptor.targetBytes - (descriptor.invalidTail?.length ?? 0);
    for (let index = 0; index < length; index++) {
      const absolute = offset + index;
      output[index] = absolute < prefix.length ? prefix[absolute]
        : absolute >= tailStart ? descriptor.invalidTail[absolute - tailStart]
          : descriptor.kind === 'minified-json' && absolute === descriptor.targetBytes - 1 ? 0x5d
            : pattern[(absolute - prefix.length) % pattern.length];
    }
    return output;
  };
}

async function writeDescriptor(outputDirectory, descriptor, options = {}) {
  const path = join(outputDirectory, `${descriptor.id}.bin`);
  const targetBytes = Math.max(0, Number(descriptor.targetBytes));
  const chunkSize = Math.min(MAX_CHUNK_BYTES, Math.max(1, Number(options.chunkSize ?? 64 * 1024)));
  const requiresLarge = targetBytes > MAX_CHUNK_BYTES;
  if (requiresLarge && !options.allowLarge) throw new Error(`large fixture ${descriptor.id} requires explicit allowLarge=true`);
  const freeSpace = requiresLarge ? assertFreeSpace(outputDirectory, targetBytes, options) : null;
  const handle = await open(path, 'w');
  const hash = createHash('sha256');
  const makeChunk = createChunkFactory(descriptor);
  let offset = 0;
  let maxWrittenChunk = 0;
  try {
    while (offset < targetBytes) {
      const length = Math.min(chunkSize, targetBytes - offset);
      const chunk = makeChunk(offset, length);
      if (chunk.length > MAX_CHUNK_BYTES) throw new Error('fixture chunk exceeds bounded write limit');
      let written = 0;
      while (written < chunk.length) {
        const { bytesWritten } = await handle.write(chunk, written, chunk.length - written, offset + written);
        if (bytesWritten === 0) throw new Error('fixture write made no progress');
        written += bytesWritten;
      }
      hash.update(chunk);
      maxWrittenChunk = Math.max(maxWrittenChunk, chunk.length);
      offset += chunk.length;
    }
  } finally {
    await handle.close();
  }
  const fileStat = await stat(path);
  return {
    id: descriptor.id,
    path,
    encoding: descriptor.encoding,
    bom: descriptor.bom,
    bytes: String(fileStat.size),
    sha256: hash.digest('hex'),
    seed: descriptor.seed,
    features: descriptor.features,
    maxWrittenChunk,
    freeSpaceCheck: freeSpace
  };
}

export async function generateCorpus(options = {}) {
  const outputDirectory = resolve(options.outputDirectory ?? join(process.cwd(), 'test-results', 'stack-text-editor', 'generated-corpus'));
  await mkdir(outputDirectory, { recursive: true });
  const seed = options.seed ?? DEFAULT_SEED;
  const plan = buildCorpusPlan(seed, options);
  const requiresLarge = plan.descriptors.some((descriptor) => descriptor.targetBytes > MAX_CHUNK_BYTES);
  if (requiresLarge && !options.allowLarge) throw new Error('large fixture generation requires explicit allowLarge=true');
  if (requiresLarge) {
    const requiredBytes = plan.descriptors.filter((descriptor) => descriptor.targetBytes > MAX_CHUNK_BYTES).reduce((total, descriptor) => total + Number(descriptor.targetBytes), 0);
    assertFreeSpace(outputDirectory, requiredBytes, options);
  }
  const fixtures = [];
  for (const descriptor of plan.descriptors) fixtures.push(await writeDescriptor(outputDirectory, descriptor, options));
  return {
    ...plan,
    outputDirectory,
    fixtures
  };
}

export async function writeCorpusManifest(manifest, path) {
  const { writeFile } = await import('node:fs/promises');
  await writeFile(path, `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
  return path;
}
