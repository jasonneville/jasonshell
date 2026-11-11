import { readFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import test from 'node:test';

// Opt-in artifact check: never executes or loads the shell, and never builds it.
// JASONSHELL_TEST_EXE=src-tauri/target/debug/jason-shell.exe node --test tests/windowsExecutableManifest.test.mjs
test('built Windows shell embeds its Common Controls v6 activation manifest', {
  skip: !process.env.JASONSHELL_TEST_EXE && 'set JASONSHELL_TEST_EXE to inspect a built PE artifact',
}, () => {
  const pe = readFileSync(process.env.JASONSHELL_TEST_EXE);
  assert.equal(pe.toString('ascii', 0, 2), 'MZ');
  const header = pe.readUInt32LE(0x3c);
  assert.equal(pe.toString('ascii', header, header + 4), 'PE\0\0');
  const optional = header + 24;
  assert.equal(pe.readUInt16LE(optional), 0x20b, 'expected x64 PE32+ executable');
  const resourceRva = pe.readUInt32LE(optional + 112 + 16);
  assert.notEqual(resourceRva, 0, 'missing PE resources: loader cannot activate Common Controls v6');
  const sectionTable = optional + pe.readUInt16LE(header + 20);
  const sectionCount = pe.readUInt16LE(header + 6);
  const offset = (rva) => {
    for (let i = 0; i < sectionCount; i++) {
      const section = sectionTable + i * 40;
      const start = pe.readUInt32LE(section + 12);
      const size = pe.readUInt32LE(section + 16);
      if (rva >= start && rva < start + size) return pe.readUInt32LE(section + 20) + rva - start;
    }
    assert.fail(`resource RVA ${rva} has no file-backed section`);
  };
  const root = offset(resourceRva);
  const entries = (relative) => {
    const directory = root + relative;
    const count = pe.readUInt16LE(directory + 12) + pe.readUInt16LE(directory + 14);
    return Array.from({ length: count }, (_, i) => {
      const entry = directory + 16 + i * 8;
      return { id: pe.readUInt32LE(entry), child: pe.readUInt32LE(entry + 4) };
    });
  };
  const type = entries(0).find(({ id }) => id === 24); // RT_MANIFEST
  assert.ok(type && (type.child >>> 31), 'missing RT_MANIFEST directory');
  const manifest = entries(type.child & 0x7fffffff).find(({ id }) => id === 1);
  assert.ok(manifest && (manifest.child >>> 31), 'missing process activation manifest (resource ID 1)');
  const languages = entries(manifest.child & 0x7fffffff);
  assert.ok(languages.length, 'manifest has no language resource');
  for (const language of languages) {
    assert.equal(language.child >>> 31, 0, 'expected manifest data entry');
    const data = root + language.child;
    const start = offset(pe.readUInt32LE(data));
    const xmlBytes = pe.subarray(start, start + pe.readUInt32LE(data + 4));
    const xml = xmlBytes.toString(xmlBytes[0] === 0xff && xmlBytes[1] === 0xfe ? 'utf16le' : 'utf8');
    assert.match(xml, /<assemblyIdentity\b[^>]*\bname=["']Microsoft\.Windows\.Common-Controls["'][^>]*>/);
    const identity = xml.match(/<assemblyIdentity\b[^>]*\bname=["']Microsoft\.Windows\.Common-Controls["'][^>]*>/)[0];
    assert.match(identity, /\bversion=["']6\.0\.0\.0["']/);
    assert.match(identity, /\bpublicKeyToken=["']6595b64144ccf1df["']/);
  }
});
