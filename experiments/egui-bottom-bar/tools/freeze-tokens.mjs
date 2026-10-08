// Read-only, explicit source audit. This tool is NOT part of the fixture runtime.
// Emits resolved CSS declarations (not browser/compositor resolved pixels).
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
const root = new URL('../../../', import.meta.url);
const paths = ['src/app.css', 'src/lib/themes.ts', 'src/components/BottomBar.css',
  'src/components/TaskPreviewSurface.css', 'src/components/TaskGallerySurface.css'];
const files = Object.fromEntries(paths.map(path => [path, readFileSync(new URL(path, root), 'utf8')]));
const css = files['src/app.css'];
const declarations = block => Object.fromEntries([...block.matchAll(/(--[\w-]+):\s*([^;]+);/g)].map(m => [m[1], m[2]]));
const base = declarations(css.match(/:root \{([\s\S]*?)\n\}/)[1]);
const resolve = (value, tokens) => value.replace(/var\((--[\w-]+)\)/g, (_, key) => resolve(tokens[key] ?? `UNRESOLVED:${key}`, tokens));
const themes = {};
for (const [, id, block] of css.matchAll(/:root\[data-theme='([^']+)'\] \{([\s\S]*?)\n\}/g)) {
  const tokens = { ...base, ...declarations(block) };
  themes[id] = Object.fromEntries(Object.entries(tokens).map(([key, value]) => [key, resolve(value, tokens)]));
}
const supported = [...files['src/lib/themes.ts'].matchAll(/\{ id: '([^']+)'/g)].map(m => m[1]);
if (supported.some(id => !themes[id]) || supported.length !== Object.keys(themes).length) throw Error('theme inventory mismatch');
console.log(JSON.stringify({
  schema: 1, date: '2026-10-07', kind: 'source-declaration freeze, not observed browser pixels',
  sources: paths.map(path => ({ path, sha256: createHash('sha256').update(files[path]).digest('hex') })),
  sourceRoot: fileURLToPath(root),
  supported, themes,
  limits: 'Default CSS root declarations and theme overrides only; user density/focus/transparency/custom-font overrides not applied. No browser root-font or native DPI observation.',
}, null, 2));
