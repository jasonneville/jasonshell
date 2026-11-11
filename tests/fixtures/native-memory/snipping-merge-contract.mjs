// Merge-only source/ownership guard. Run from the target checkout root.
// Runtime codec/golden coverage remains in the untouched snipping_png_codec.rs.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const source=readFileSync('src-tauri/src/snipping/clipboard.rs','utf8').replaceAll('\r\n','\n');
assert.doesNotMatch(source,/^(?:<<<<<<<|=======|>>>>>>>)/m,'unresolved clipboard conflict');
assert.equal([...source.matchAll(/\b(?:static|const)\s+\w+\s*:\s*\[u32;\s*256\]/g)].length,1,'one 1KiB production CRC table only');
assert.equal([...source.matchAll(/\bconst fn crc_table\(/g)].length,1);
assert.match(source,/static CRC_TABLE:\s*\[u32;\s*256\]\s*=\s*crc_table\(\)/);
const crc=source.slice(source.indexOf('fn crc(bytes:'),source.indexOf('\n}',source.indexOf('fn crc(bytes:'))+2);
assert.match(crc,/\^ CRC_TABLE\[/);
assert.doesNotMatch(crc,/\bstatic\b/,'CRC must use the module table, not a duplicate local table');
const start=source.indexOf('fn prepare('),end=source.indexOf('\n}',start);
const prepare=source.slice(start,end);
const boundaries=['let dib_global = Global::copy(&dib, false)?;','drop(dib);','drop(_dib_staged);','let png = Global::copy(&output, false)?;'].map(text=>prepare.indexOf(text));
assert(boundaries.every((offset,index)=>offset>=0&&(index===0||offset>boundaries[index-1])),'DIB allocation/accounting must release before PNG HGLOBAL allocation');
assert.match(source,/fn update_adler\(/);
assert.match(source,/pub fn png_rgba_transport\(/);
console.log('PASS: resolved single module CRC table; DIB Vec/accounting release order; master codec entrypoints retained.');
