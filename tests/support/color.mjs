// Test-owned CSS color math, deliberately independent from production code so a
// gate implementation cannot validate itself. Supports the literal forms a
// resolved palette may contain: hex, rgb()/rgba(), oklch(), transparent and
// two-colour color-mix(in srgb, ...). var() is resolved by resolvePaletteColor.
const clamp01 = (n) => Math.min(1, Math.max(0, n));
const toLinear = (c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const fromLinear = (c) => (c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055);

function splitArgs(text) {
  const parts = [];
  let depth = 0;
  let current = '';
  for (const ch of text) {
    if (ch === '(') depth += 1;
    if (ch === ')') depth -= 1;
    if (ch === ',' && depth === 0) { parts.push(current.trim()); current = ''; } else current += ch;
  }
  parts.push(current.trim());
  return parts;
}
const channel = (token) => (token.endsWith('%') ? (parseFloat(token) / 100) * 255 : parseFloat(token));
const alphaOf = (token) => (token === undefined ? 1 : token.endsWith('%') ? parseFloat(token) / 100 : parseFloat(token));

export function oklabToRgb(L, a, b) {
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const lin = [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s];
  return lin.map((c) => clamp01(fromLinear(clamp01(c))) * 255);
}

/** @returns {{r:number,g:number,b:number,a:number}} channels 0..255, alpha 0..1 */
export function parseColor(input) {
  const value = String(input).trim().toLowerCase();
  if (value === 'transparent') return { r: 0, g: 0, b: 0, a: 0 };
  let m = value.match(/^#([\da-f]{3,8})$/);
  if (m) {
    let d = m[1];
    if (d.length === 3 || d.length === 4) d = [...d].map((x) => x + x).join('');
    if (d.length !== 6 && d.length !== 8) throw new Error(`Unparseable color: ${input}`);
    return { r: parseInt(d.slice(0, 2), 16), g: parseInt(d.slice(2, 4), 16), b: parseInt(d.slice(4, 6), 16), a: d.length === 8 ? parseInt(d.slice(6), 16) / 255 : 1 };
  }
  m = value.match(/^rgba?\((.*)\)$/);
  if (m) {
    const parts = m[1].includes(',') ? splitArgs(m[1]) : m[1].replace('/', ' / ').split(/\s+/).filter((x) => x && x !== '/');
    if (parts.length < 3) throw new Error(`Unparseable color: ${input}`);
    return { r: channel(parts[0]), g: channel(parts[1]), b: channel(parts[2]), a: alphaOf(parts[3]) };
  }
  m = value.match(/^oklch\((.*)\)$/);
  if (m) {
    const [body, alpha] = m[1].split('/').map((x) => x.trim());
    const [l, c, h] = body.split(/\s+/);
    const L = l.endsWith('%') ? parseFloat(l) / 100 : parseFloat(l);
    const C = c.endsWith('%') ? (parseFloat(c) / 100) * 0.4 : parseFloat(c);
    const H = (parseFloat(h) * Math.PI) / 180;
    const [r, g, b] = oklabToRgb(L, C * Math.cos(H), C * Math.sin(H));
    return { r, g, b, a: alphaOf(alpha) };
  }
  m = value.match(/^color-mix\(\s*in\s+srgb\s*,(.*)\)$/);
  if (m) {
    const [first, second] = splitArgs(m[1]);
    const part = (text) => { const hit = text.match(/^(.*?)(?:\s+([\d.]+)%)?$/); return { color: parseColor(hit[1]), weight: hit[2] === undefined ? undefined : parseFloat(hit[2]) }; };
    const one = part(first);
    const two = part(second);
    const w1 = one.weight ?? (two.weight === undefined ? 50 : 100 - two.weight);
    const w2 = two.weight ?? 100 - w1;
    const total = w1 + w2;
    const a = (one.color.a * w1 + two.color.a * w2) / total;
    const mix = (k) => (a === 0 ? 0 : (one.color[k] * one.color.a * w1 + two.color[k] * two.color.a * w2) / total / a);
    return { r: mix('r'), g: mix('g'), b: mix('b'), a };
  }
  throw new Error(`Unparseable color: ${input}`);
}

/** Resolve var(--js-x) chains against one palette, then parse. */
export function resolvePaletteColor(palette, token, seen = new Set()) {
  if (seen.has(token)) throw new Error(`Cyclic ${token}`);
  const raw = palette[token];
  if (raw === undefined) throw new Error(`Missing ${token}`);
  const next = new Set([...seen, token]);
  const flat = raw.replace(/var\((--js-[\w-]+)\)/g, (_all, ref) => {
    const c = resolvePaletteColor(palette, ref, next);
    return `rgba(${c.r}, ${c.g}, ${c.b}, ${c.a})`;
  });
  return parseColor(flat);
}

export function composite(fg, bg) {
  if (bg.a !== 1) throw new Error('Background must be opaque');
  return { r: fg.r * fg.a + bg.r * (1 - fg.a), g: fg.g * fg.a + bg.g * (1 - fg.a), b: fg.b * fg.a + bg.b * (1 - fg.a), a: 1 };
}
export function luminance({ r, g, b }) {
  return 0.2126 * toLinear(r / 255) + 0.7152 * toLinear(g / 255) + 0.0722 * toLinear(b / 255);
}
export function contrast(fg, bg) {
  const solid = fg.a === 1 ? fg : composite(fg, bg);
  const [hi, lo] = [luminance(solid), luminance(bg)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}
export function toOklab({ r, g, b }) {
  const [lr, lg, lb] = [r, g, b].map((c) => toLinear(c / 255));
  const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
  const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
  const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
  return { L: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s, a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s, b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s };
}
export function oklch(color) {
  const { L, a, b } = toOklab(color);
  return { L, C: Math.hypot(a, b), H: ((Math.atan2(b, a) * 180) / Math.PI + 360) % 360 };
}
export const hueDistance = (x, y) => { const d = Math.abs(x - y) % 360; return d > 180 ? 360 - d : d; };
export function oklabDistance(x, y) {
  const p = toOklab(x); const q = toOklab(y);
  return Math.hypot(p.L - q.L, p.a - q.a, p.b - q.b);
}
/** Opaque surface colors of a resolved palette: bg, surface (over bg), raised (over bg). */
export function surfaces(palette) {
  const bg = resolvePaletteColor(palette, '--js-color-bg');
  const over = (token) => composite(resolvePaletteColor(palette, token), bg);
  return { bg, surface: over('--js-color-surface'), raised: over('--js-color-surface-raised') };
}
