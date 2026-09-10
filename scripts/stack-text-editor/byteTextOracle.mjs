import { createHash } from 'node:crypto';

export const UINT64_MAX = 18_446_744_073_709_551_615n;
export const ORACLE_SCHEMA_VERSION = 'stack-text-oracle.v1';
export const SUPPORTED_ENCODINGS = Object.freeze(['utf8', 'utf16le', 'utf16be']);

export class OracleError extends Error {
  constructor(code, message, details = undefined) {
    super(message);
    this.name = 'OracleError';
    this.code = code;
    this.details = details;
  }
}

function assertBytes(value) {
  if (value instanceof Uint8Array) return value;
  if (value instanceof ArrayBuffer) return new Uint8Array(value);
  if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
  throw new TypeError('bytes must be a Uint8Array or ArrayBuffer view');
}

export function normalizeEncoding(value) {
  const normalized = String(value ?? '').toLowerCase().replaceAll('-', '');
  if (normalized === 'utf8') return 'utf8';
  if (normalized === 'utf16le' || normalized === 'utf16') return 'utf16le';
  if (normalized === 'utf16be') return 'utf16be';
  throw new OracleError('UnsupportedEncoding', `unsupported oracle encoding: ${value}`);
}

export function detectEncoding(bytes) {
  const input = assertBytes(bytes);
  if (input.length >= 3 && input[0] === 0xef && input[1] === 0xbb && input[2] === 0xbf) {
    return { encoding: 'utf8', bomLength: 3 };
  }
  if (input.length >= 2 && input[0] === 0xff && input[1] === 0xfe) {
    return { encoding: 'utf16le', bomLength: 2 };
  }
  if (input.length >= 2 && input[0] === 0xfe && input[1] === 0xff) {
    return { encoding: 'utf16be', bomLength: 2 };
  }
  return { encoding: 'utf8', bomLength: 0 };
}

function readUtf8CodePoint(bytes, index) {
  const first = bytes[index];
  if (first <= 0x7f) return { codePoint: first, length: 1 };
  let length;
  let codePoint;
  let minimum;
  if (first >= 0xc2 && first <= 0xdf) {
    length = 2;
    codePoint = first & 0x1f;
    minimum = 0x80;
  } else if (first >= 0xe0 && first <= 0xef) {
    length = 3;
    codePoint = first & 0x0f;
    minimum = 0x800;
  } else if (first >= 0xf0 && first <= 0xf4) {
    length = 4;
    codePoint = first & 0x07;
    minimum = 0x10000;
  } else {
    return null;
  }
  if (index + length > bytes.length) return null;
  for (let offset = 1; offset < length; offset += 1) {
    const part = bytes[index + offset];
    if ((part & 0xc0) !== 0x80) return null;
    codePoint = (codePoint << 6) | (part & 0x3f);
  }
  if (codePoint < minimum || codePoint > 0x10ffff || (codePoint >= 0xd800 && codePoint <= 0xdfff)) return null;
  return { codePoint, length };
}

function decodeRawCharacters(body, encoding) {
  const rawCharacters = [];
  let invalidTailAt = null;
  if (encoding === 'utf8') {
    let index = 0;
    while (index < body.length) {
      const decoded = readUtf8CodePoint(body, index);
      if (!decoded) {
        invalidTailAt = index;
        break;
      }
      rawCharacters.push({
        text: String.fromCodePoint(decoded.codePoint),
        byteStart: index,
        byteEnd: index + decoded.length
      });
      index += decoded.length;
    }
  } else {
    let index = 0;
    while (index + 1 < body.length) {
      const codeUnit = encoding === 'utf16le'
        ? body[index] | (body[index + 1] << 8)
        : (body[index] << 8) | body[index + 1];
      if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
        if (index + 3 >= body.length) {
          invalidTailAt = index;
          break;
        }
        const next = encoding === 'utf16le'
          ? body[index + 2] | (body[index + 3] << 8)
          : (body[index + 2] << 8) | body[index + 3];
        if (next < 0xdc00 || next > 0xdfff) {
          invalidTailAt = index;
          break;
        }
        const codePoint = 0x10000 + ((codeUnit - 0xd800) << 10) + (next - 0xdc00);
        rawCharacters.push({ text: String.fromCodePoint(codePoint), byteStart: index, byteEnd: index + 4 });
        index += 4;
      } else if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
        invalidTailAt = index;
        break;
      } else {
        rawCharacters.push({ text: String.fromCharCode(codeUnit), byteStart: index, byteEnd: index + 2 });
        index += 2;
      }
    }
    if (invalidTailAt === null && index < body.length) invalidTailAt = index;
  }
  return { rawCharacters, invalidTailAt };
}

function buildLocalProjection(rawCharacters, bomLength, invalidTailAt, bodyLength) {
  const tokens = [];
  for (let index = 0; index < rawCharacters.length; index += 1) {
    const current = rawCharacters[index];
    const next = rawCharacters[index + 1];
    if (current.text === '\r' && next?.text === '\n') {
      tokens.push({ text: '\n', byteStart: current.byteStart, byteEnd: next.byteEnd, sourceNewline: '\r\n' });
      index += 1;
    } else {
      tokens.push({
        text: current.text,
        byteStart: current.byteStart,
        byteEnd: current.byteEnd,
        sourceNewline: current.text === '\r' ? '\r' : current.text === '\n' ? '\n' : null
      });
    }
  }

  let text = '';
  const localToByte = [bomLength];
  const safeBoundary = [true];
  const unitTokens = [];
  for (const token of tokens) {
    text += token.text;
    const unitLength = token.text.length;
    if (unitLength === 2) {
      unitTokens.push({ ...token, unitStart: unitTokens.length, unitLength });
      localToByte.push(null);
      safeBoundary.push(false);
      localToByte.push(bomLength + token.byteEnd);
      safeBoundary.push(true);
    } else {
      unitTokens.push({ ...token, unitStart: unitTokens.length, unitLength });
      localToByte.push(bomLength + token.byteEnd);
      safeBoundary.push(true);
    }
  }

  const lineStarts = [0];
  for (let index = 0; index < text.length; index += 1) {
    if (text[index] === '\n') lineStarts.push(index + 1);
  }
  const firstNewline = tokens.find((token) => token.sourceNewline);
  const newlineStyle = firstNewline?.sourceNewline ?? '\r\n';
  const invalidRange = invalidTailAt === null
    ? null
    : { start: bomLength + invalidTailAt, end: bomLength + bodyLength, kind: 'invalid-tail' };
  return { text, localToByte, safeBoundary, unitTokens, lineStarts, newlineStyle, invalidRange };
}

export function decodeByteText(bytes, options = {}) {
  const input = assertBytes(bytes);
  const detected = options.encoding ? { encoding: normalizeEncoding(options.encoding), bomLength: options.bomLength ?? 0 } : detectEncoding(input);
  const encoding = detected.encoding;
  const bomLength = Math.max(0, Math.min(input.length, Number(detected.bomLength ?? 0)));
  const body = input.subarray(bomLength);
  const raw = decodeRawCharacters(body, encoding);
  const projection = buildLocalProjection(raw.rawCharacters, bomLength, raw.invalidTailAt, body.length);
  const digest = createHash('sha256').update(input).digest('hex');
  return {
    schemaVersion: ORACLE_SCHEMA_VERSION,
    encoding,
    bomLength,
    byteLength: input.length,
    sha256: digest,
    text: projection.text,
    utf16Length: projection.text.length,
    lineCount: projection.lineStarts.length,
    lineStarts: projection.lineStarts,
    newlineStyle: projection.newlineStyle,
    localToByte: projection.localToByte,
    safeBoundary: projection.safeBoundary,
    unitTokens: projection.unitTokens,
    invalidRange: projection.invalidRange,
    validPrefixByteLength: projection.invalidRange?.start ?? input.length,
    hasFinalNewline: projection.text.endsWith('\n'),
    sourceBytes: new Uint8Array(input)
  };
}

function encodeCodePointUtf8(codePoint, output) {
  if (codePoint <= 0x7f) output.push(codePoint);
  else if (codePoint <= 0x7ff) output.push(0xc0 | (codePoint >> 6), 0x80 | (codePoint & 0x3f));
  else if (codePoint <= 0xffff) output.push(0xe0 | (codePoint >> 12), 0x80 | ((codePoint >> 6) & 0x3f), 0x80 | (codePoint & 0x3f));
  else output.push(0xf0 | (codePoint >> 18), 0x80 | ((codePoint >> 12) & 0x3f), 0x80 | ((codePoint >> 6) & 0x3f), 0x80 | (codePoint & 0x3f));
}

export function encodeByteText(text, options = {}) {
  const encoding = normalizeEncoding(options.encoding ?? 'utf8');
  const newlineStyle = options.newlineStyle ?? '\r\n';
  if (!['\n', '\r\n', '\r'].includes(newlineStyle)) throw new OracleError('InvalidNewlineStyle', 'newlineStyle must be LF, CRLF, or CR');
  const normalized = String(text).replaceAll('\r\n', '\n').replaceAll('\r', '\n');
  const logical = newlineStyle === '\n' ? normalized : normalized.replaceAll('\n', newlineStyle);
  const bytes = [];
  if (options.bom) {
    if (encoding === 'utf8') bytes.push(0xef, 0xbb, 0xbf);
    else if (encoding === 'utf16le') bytes.push(0xff, 0xfe);
    else bytes.push(0xfe, 0xff);
  }
  if (encoding === 'utf8') {
    for (const character of logical) encodeCodePointUtf8(character.codePointAt(0), bytes);
  } else {
    for (const character of logical) {
      const codePoint = character.codePointAt(0);
      const codeUnits = codePoint > 0xffff
        ? [0xd800 + ((codePoint - 0x10000) >> 10), 0xdc00 + ((codePoint - 0x10000) & 0x3ff)]
        : [codePoint];
      for (const codeUnit of codeUnits) {
        if (encoding === 'utf16le') bytes.push(codeUnit & 0xff, codeUnit >> 8);
        else bytes.push(codeUnit >> 8, codeUnit & 0xff);
      }
    }
  }
  return Uint8Array.from(bytes);
}

function assertU64(value, field) {
  let parsed;
  try {
    parsed = typeof value === 'bigint' ? value : BigInt(String(value));
  } catch {
    throw new OracleError('InvalidU64', `${field} must be a decimal u64 string`);
  }
  if (parsed < 0n || parsed > UINT64_MAX || (typeof value === 'string' && !/^\d+$/.test(value))) {
    throw new OracleError('InvalidU64', `${field} is outside decimal u64 range`);
  }
  return parsed;
}

export function assertU64Decimal(value, field = 'value') {
  return assertU64(value, field).toString(10);
}

function validateLocalBoundary(state, offset, field) {
  if (!Number.isSafeInteger(offset) || offset < 0 || offset > state.text.length) {
    throw new OracleError('InvalidTextBoundary', `${field} is outside the bounded UTF-16 projection`);
  }
  if (!state.safeBoundary[offset]) {
    throw new OracleError('InvalidTextBoundary', `${field} splits a surrogate pair or encoding boundary`);
  }
}

export class ByteTextOracle {
  constructor(bytes, options = {}) {
    const input = assertBytes(bytes);
    this.encoding = options.encoding ? normalizeEncoding(options.encoding) : detectEncoding(input).encoding;
    this.bom = options.bom ?? (detectEncoding(input).bomLength > 0);
    const decodeOptions = { ...options, encoding: this.encoding, bomLength: options.bomLength ?? detectEncoding(input).bomLength };
    this.state = decodeByteText(input, decodeOptions);
    this.revision = 0n;
    this.originalBytes = new Uint8Array(input);
  }

  get text() { return this.state.text; }
  get utf16Length() { return this.state.utf16Length; }
  get lineCount() { return this.state.lineCount; }
  get invalidRange() { return this.state.invalidRange; }
  get newlineStyle() { return this.state.newlineStyle; }
  get documentRevision() { return this.revision.toString(10); }
  byteOffsetForLocal(localOffset) {
    validateLocalBoundary(this.state, localOffset, 'localOffset');
    return this.state.localToByte[localOffset];
  }

  localOffsetForByte(byteOffset) {
    assertU64(byteOffset, 'byteOffset');
    const numeric = Number(byteOffset);
    const index = this.state.localToByte.findIndex((value) => value === numeric);
    if (index >= 0) return index;
    throw new OracleError('InvalidTextBoundary', 'byteOffset is not a known local boundary');
  }

  applyEdits(edits) {
    if (!Array.isArray(edits)) throw new TypeError('edits must be an array');
    if (this.invalidRange) throw new OracleError('EncodingRequired', 'invalid tail must be classified before mutation');
    const normalized = edits.map((edit, index) => {
      const from = edit.from ?? edit.start;
      const to = edit.to ?? edit.end ?? from;
      validateLocalBoundary(this.state, from, `edits[${index}].from`);
      validateLocalBoundary(this.state, to, `edits[${index}].to`);
      if (to < from) throw new OracleError('InvalidTextBoundary', `edits[${index}] has reversed bounds`);
      return { from, to, text: String(edit.text ?? '') };
    }).sort((left, right) => right.from - left.from || right.to - left.to);
    for (let index = 1; index < normalized.length; index += 1) {
      if (normalized[index - 1].from < normalized[index].to) throw new OracleError('InvalidTextBoundary', 'edits overlap');
    }
    let next = this.state.text;
    for (const edit of normalized) {
      const inserted = edit.text.replaceAll('\r\n', '\n').replaceAll('\r', '\n');
      next = next.slice(0, edit.from) + inserted + next.slice(edit.to);
    }
    const encoded = encodeByteText(next, { encoding: this.encoding, bom: this.bom, newlineStyle: this.newlineStyle });
    const bomLength = this.bom ? (this.encoding === 'utf8' ? 3 : 2) : 0;
    this.state = decodeByteText(encoded, { encoding: this.encoding, bomLength });
    this.revision += 1n;
    return { revision: this.documentRevision, text: this.state.text, utf16Length: this.state.utf16Length, lineCount: this.state.lineCount };
  }

  toBytes() {
    if (this.revision === 0n) return new Uint8Array(this.originalBytes);
    return encodeByteText(this.state.text, { encoding: this.encoding, bom: this.bom, newlineStyle: this.newlineStyle });
  }

  snapshot() {
    return {
      revision: this.documentRevision,
      encoding: this.encoding,
      bom: this.bom,
      byteLength: this.toBytes().length,
      utf16Length: this.utf16Length,
      lineCount: this.lineCount,
      invalidRange: this.invalidRange
    };
  }
}

export function createTextOracle(bytes, options = {}) {
  return new ByteTextOracle(bytes, options);
}

export const createByteTextOracle = createTextOracle;

export function deterministicRandom(seed) {
  let state = (Number(seed) >>> 0) || 0x9e3779b9;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    state >>>= 0;
    return state / 0x100000000;
  };
}

export function randomOracleEdits(oracle, seed, count = 16) {
  const random = deterministicRandom(seed);
  const edits = [];
  for (let index = 0; index < count; index += 1) {
    const length = oracle.utf16Length;
    const from = length === 0 ? 0 : Math.floor(random() * (length + 1));
    const safeFrom = oracle.state.safeBoundary[from] ? from : Math.max(0, from - 1);
    const deleteLength = Math.min(Math.floor(random() * 4), oracle.utf16Length - safeFrom);
    const to = safeFrom + deleteLength;
    const alphabet = ['a', 'Z', '0', 'é', '𐐷', '\n'];
    const inserted = random() < 0.35 ? alphabet[Math.floor(random() * alphabet.length)] : '';
    edits.push({ from: safeFrom, to: oracle.state.safeBoundary[to] ? to : safeFrom, text: inserted });
    oracle.applyEdits(edits.slice(-1));
  }
  return edits;
}

export function oracleDigest(oracle) {
  return createHash('sha256').update(oracle.toBytes()).digest('hex');
}
export function runRandomizedOracle(bytes, options = {}) {
  const seed = options.seed ?? 1;
  const oracle = createTextOracle(bytes, options);
  const random = deterministicRandom(seed);
  const operations = [];
  const steps = Math.max(0, Number(options.steps ?? 32));
  for (let index = 0; index < steps; index += 1) {
    const boundaries = oracle.state.safeBoundary.flatMap((safe, position) => safe ? [position] : []);
    const from = boundaries[Math.floor(random() * boundaries.length)] ?? 0;
    const to = boundaries[Math.min(boundaries.length - 1, boundaries.indexOf(from) + Math.floor(random() * 3))] ?? from;
    const alphabet = ['a', 'Z', 'é', '𐐷', '\\n'];
    const text = random() < 0.4 ? alphabet[Math.floor(random() * alphabet.length)] : '';
    const operation = { from, to: Math.max(from, to), text };
    oracle.applyEdits([operation]);
    operations.push(operation);
  }
  return { seed, steps: operations.length, operations, digest: oracleDigest(oracle), snapshot: oracle.snapshot() };
}
