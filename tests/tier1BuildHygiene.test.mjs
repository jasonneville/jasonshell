import assert from 'node:assert/strict';
import test from 'node:test';
import { readRepoFile } from './support/repoSource.mjs';

const eol = (text) => text.replace(/\r\n/g, '\n');
const section = (toml, header) => {
  const escaped = header.replace(/\./g, String.raw`\.`);
  const match = eol(toml).match(new RegExp(String.raw`^\[${escaped}\]\n((?:(?!\[).*\n?)*)`, 'm'));
  assert.ok(match, `[${header}] section missing`);
  return match[1];
};

// BLD-03: shipped binary profile. panic stays "unwind": the Tier 0 panic hook and thread joins rely on it.
test('BLD-03 release profile is size/perf tuned without abort-on-panic', () => {
  const release = section(readRepoFile('src-tauri/Cargo.toml'), 'profile.release');
  assert.match(release, /^lto\s*=\s*"thin"/m);
  assert.match(release, /^codegen-units\s*=\s*1\b/m);
  assert.match(release, /^strip\s*=\s*"symbols"/m);
  assert.doesNotMatch(release, /^panic\s*=\s*"abort"/m);
});

// HYG-04: one package manager.
test('HYG-04 npm is the declared package manager and engines are pinned', () => {
  const pkg = JSON.parse(readRepoFile('package.json'));
  assert.match(pkg.packageManager ?? '', /^npm@\d+\.\d+\.\d+$/);
  assert.ok(pkg.engines?.node, 'engines.node required');
  assert.match(pkg.engines.node, /\d/);
});

// HYG-02/03/09: local tool/state output must be ignored.
test('HYG-02/03/09 .gitignore covers local tool and output directories', () => {
  const lines = eol(readRepoFile('.gitignore')).split('\n').map((l) => l.trim().replace(/^\//, ''));
  for (const entry of ['.code/', '.openchamber/', 'smoke-results/', 'src-tauri/logs/', 'src-tauri/test-results/', '.zed/', '.codex']) {
    assert.ok(lines.includes(entry) || lines.includes(entry.replace(/\/$/, '')), `.gitignore missing ${entry}`);
  }
});

// BLD-04: generated ACL files must be byte-stable between platforms and builds.
test('BLD-04 .gitattributes pins src-tauri/permissions to LF', () => {
  const attrs = eol(readRepoFile('.gitattributes'));
  assert.match(attrs, /^src-tauri\/permissions\/\*\*\s+.*\beol=lf\b/m);
});

test('BLD-04 CI fails when build regenerates tracked permission files', () => {
  const workflow = eol(readRepoFile('.github/workflows/windows-ci.yml'));
  const cargoTest = workflow.indexOf('run: npm run cargo:test');
  // `git diff` ignores untracked generated files, so the check must also see them (status --porcelain).
  const drift = workflow.search(/run:.*git (?:status --porcelain|diff --exit-code)[^\n]*src-tauri\/permissions/);
  assert.ok(cargoTest >= 0 && drift > cargoTest, 'drift check must run after cargo:test');
  assert.match(workflow, /git status --porcelain[^\n]*src-tauri\/permissions/);
});

test('BLD-04 build.rs does not rewrite an unchanged legacy permission file (CRLF/LF tolerant)', () => {
  const build = eol(readRepoFile('src-tauri/build.rs'));
  assert.ok(build.includes(String.raw`.replace("\r\n", "\n")`), 'compare after CRLF normalization');
  assert.match(build, /if !permission_unchanged\([^\n]*\) \{\n\s*fs::write\(path, permission\)/, 'write must be guarded by permission_unchanged');
});
