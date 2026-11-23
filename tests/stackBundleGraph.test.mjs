import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

// FE-34 build-output contract: the Stack surface chunk must not statically pull CodeMirror
// (stack-editor-core) or xterm; the editor, Git panel and terminal panes are dynamic imports only.
// Real vite build into a private temp outDir (~10 s). Never touches dist/ or dist-tests/.
const root = fileURLToPath(new URL('..', import.meta.url));
const outDir = mkdtempSync(path.join(tmpdir(), 'jasonshell-stack-bundle-'));
test.after(() => rmSync(outDir, { recursive: true, force: true }));

const build = spawnSync(process.execPath, [path.join(root, 'node_modules/vite/bin/vite.js'), 'build', '--outDir', outDir, '--emptyOutDir', '--manifest'], { cwd: root, encoding: 'utf8', timeout: 180_000 });
assert.equal(build.status, 0, `vite build failed:\n${build.stdout}\n${build.stderr}`);
const manifest = JSON.parse(readFileSync(path.join(outDir, '.vite', 'manifest.json'), 'utf8'));
const byFile = new Map(Object.entries(manifest).map(([key, chunk]) => [chunk.file, { key, ...chunk }]));
const surface = manifest['src/components/StackPopupSurface.svelte'];
const PANES = ['StackTextEditor', 'StackGitPanel', 'StackTerminalPane'];
// Markers that only exist in the vendor runtimes (class names are emitted as string literals).
const XTERM = /xterm-(?:screen|helper-textarea|rows|viewport)/;
const CODEMIRROR = /cm-(?:editor|scroller|content|gutters)/;

function staticClosure(entry) {
  const seen = new Set(); const stack = [entry];
  while (stack.length) {
    const chunk = stack.pop(); if (!chunk || seen.has(chunk)) continue; seen.add(chunk);
    for (const key of chunk.imports ?? []) stack.push(manifest[key]);
  }
  return [...seen];
}
const source = (chunk) => readFileSync(path.join(outDir, chunk.file), 'utf8');

test('Stack surface is its own lazily loaded chunk', () => {
  assert.ok(surface, 'manifest must contain the Stack popup surface entry');
  assert.equal(surface.isDynamicEntry, true);
});

test('the Stack surface static import closure contains no xterm and no CodeMirror runtime', () => {
  const closure = staticClosure(surface);
  const offenders = closure.filter((chunk) => /stack-editor-core/.test(chunk.file) || XTERM.test(source(chunk)) || CODEMIRROR.test(source(chunk))).map((chunk) => chunk.file);
  assert.deepEqual(offenders, [], `first Stack render must not statically load: ${offenders.join(', ')}`);
});

test('editor, Git panel and terminal panes are reachable only through dynamic imports from the Stack surface', () => {
  const dynamicTargets = new Set((surface.dynamicImports ?? []).map((key) => manifest[key]?.src ?? key));
  for (const pane of PANES) {
    if (pane === 'StackTerminalPane') continue; // currently dead code in the surface; may be deleted instead of lazified
    assert.ok(dynamicTargets.has(`src/components/${pane}.svelte`), `${pane} must be a dynamicImport of the surface`);
    assert.ok(!staticClosure(surface).some((chunk) => chunk.src === `src/components/${pane}.svelte`), `${pane} must not be in the static closure`);
  }
  assert.ok(!staticClosure(surface).some((chunk) => chunk.src === 'src/components/StackTerminalPane.svelte'));
});

test('editor core chunk is only reached through the lazy editor pane', () => {
  const core = [...byFile.values()].find((chunk) => /stack-editor-core/.test(chunk.file));
  assert.ok(core, 'vite.config manualChunks must still emit stack-editor-core');
  const staticImporters = [...byFile.values()].filter((chunk) => (chunk.imports ?? []).includes(core.key)).map((chunk) => chunk.src ?? chunk.file);
  const surfaceStatic = new Set(staticClosure(surface).map((chunk) => chunk.file));
  assert.ok(!surfaceStatic.has(core.file), `stack-editor-core statically reachable via ${staticImporters.join(', ')}`);
});
