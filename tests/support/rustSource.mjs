import { readFileSync } from 'node:fs';

export function readRust(relativeFromRepoRoot) {
  return readFileSync(new URL(`../../${relativeFromRepoRoot}`, import.meta.url), 'utf8').replace(/\r\n/g, '\n');
}

const IDENT = /[A-Za-z0-9_]/;

/** End index (exclusive) of a string/raw-string/char literal starting at i, else -1. */
function literalEnd(text, i) {
  const ch = text[i];
  if (ch === '"') {
    let j = i + 1;
    while (j < text.length && text[j] !== '"') j += text[j] === '\\' ? 2 : 1;
    return j + 1;
  }
  if ((ch === 'r' || (ch === 'b' && text[i + 1] === 'r')) && !(i > 0 && IDENT.test(text[i - 1]))) {
    const m = /^b?r(#*)"/.exec(text.slice(i, i + 40));
    if (m) {
      const close = `"${m[1]}`;
      const end = text.indexOf(close, i + m[0].length);
      return end === -1 ? text.length : end + close.length;
    }
  }
  if (ch === "'") {
    const m = /^'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^\\'\n])'/.exec(text.slice(i, i + 14));
    if (m) return i + m[0].length;
  }
  return -1;
}

/** Removes `//` line comments (incl. doc comments) and nested block comments; literals are preserved. */
export function stripRustComments(source) {
  let out = '';
  let i = 0;
  while (i < source.length) {
    if (source[i] === '/' && source[i + 1] === '/') {
      while (i < source.length && source[i] !== '\n') i += 1;
      continue;
    }
    if (source[i] === '/' && source[i + 1] === '*') {
      let depth = 1;
      i += 2;
      while (i < source.length && depth > 0) {
        if (source[i] === '/' && source[i + 1] === '*') { depth += 1; i += 2; }
        else if (source[i] === '*' && source[i + 1] === '/') { depth -= 1; i += 2; }
        else i += 1;
      }
      out += ' ';
      continue;
    }
    const end = literalEnd(source, i);
    if (end !== -1) {
      out += source.slice(i, end);
      i = end;
      continue;
    }
    out += source[i];
    i += 1;
  }
  return out;
}

function matchingBrace(text, open) {
  let depth = 0;
  for (let i = open; i < text.length; i += 1) {
    const end = literalEnd(text, i);
    if (end !== -1) {
      i = end - 1;
      continue;
    }
    if (text[i] === '{') depth += 1;
    else if (text[i] === '}') {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  return -1;
}

/**
 * Production code only, comments stripped: removes `#[cfg(test)] mod name { ... }`
 * blocks (brace matched) and `#[cfg(test)] #[path] mod name;` inclusion lines.
 */
export function productionPart(source) {
  let out = stripRustComments(source);
  out = out.replace(/^#\[cfg\(test\)\]\n(?:#\[path[^\n]*\n)?mod \w+;\n?/gm, '');
  const re = /^#\[cfg\(test\)\]\nmod \w+ \{/gm;
  let match;
  while ((match = re.exec(out)) !== null) {
    const open = out.indexOf('{', match.index);
    const close = matchingBrace(out, open);
    if (close === -1) break;
    out = out.slice(0, match.index) + out.slice(close + 1);
    re.lastIndex = match.index;
  }
  return out;
}

/** Body of `fn name` (brace matched, literal-aware). Returns null when absent. */
export function functionBody(source, name) {
  const match = new RegExp(`\\bfn ${name}\\b`).exec(source);
  if (!match) return null;
  const open = source.indexOf('{', source.indexOf(')', match.index));
  if (open === -1) return null;
  const close = matchingBrace(source, open);
  return close === -1 ? null : source.slice(open, close + 1);
}

/** Count of occurrences of `token` in text. */
export function countOf(text, token) {
  return text.split(token).length - 1;
}
