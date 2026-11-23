import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) walk(full, out); else if (/\.(css|svelte)$/.test(name)) out.push(full);
  }
  return out;
}

/** Style text of a source file: whole .css, or the <style> blocks of a .svelte file. */
function styleText(file, text) {
  if (file.endsWith('.css')) return text;
  return [...text.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1]).join('\n');
}

/**
 * Minimal CSS rule extractor: yields {selector, body, media[]} for every style
 * rule, descending into @media/@supports/@layer and ignoring @keyframes/@font-face.
 */
export function extractRules(css) {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, '');
  const rules = [];
  function parse(start, end, media) {
    let i = start;
    while (i < end) {
      const open = text.indexOf('{', i);
      if (open < 0 || open >= end) break;
      let depth = 1;
      let j = open + 1;
      while (j < end && depth) { if (text[j] === '{') depth += 1; else if (text[j] === '}') depth -= 1; j += 1; }
      const prelude = text.slice(i, open).trim();
      const inner = text.slice(open + 1, j - 1);
      if (/^@(media|supports|layer|container)/.test(prelude)) parse(open + 1, j - 1, [...media, prelude]);
      else if (!prelude.startsWith('@')) rules.push({ selector: prelude.replace(/\s+/g, ' '), body: inner, media });
      i = j;
    }
  }
  parse(0, text.length, []);
  return rules;
}

export function sourceRules() {
  const out = [];
  for (const file of walk(join(root, 'src'))) {
    const rel = relative(root, file).replaceAll('\\', '/');
    for (const rule of extractRules(styleText(file, readFileSync(file, 'utf8')))) out.push({ file: rel, ...rule });
  }
  return out;
}

export function declarations(body) {
  return [...body.matchAll(/([\w-]+)\s*:\s*([^;]+?)\s*(?:;|$)/g)].map(([, name, value]) => ({ name: name.toLowerCase(), value }));
}
