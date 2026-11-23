// Contrast gate for built-in palettes (WCAG 2.x). Translucent foregrounds are
// composited over each opaque surface (itself composited over bg) first.
const SURFACES = ['--js-color-surface', '--js-color-surface-raised'];
export const CONTRAST_RULES = Object.freeze([
  { token: '--js-color-text-muted', min: 4.5, surfaces: SURFACES },
  { token: '--js-color-text-subtle', min: 3, surfaces: SURFACES },
  { token: '--js-color-accent-border', min: 3, surfaces: SURFACES },
  // Accent doubles as text (eyebrows, highlights) on raised surfaces.
  { token: '--js-color-accent', min: 4.5, surfaces: ['--js-color-surface-raised'] }
]);

/** Accepts only the literal forms gated tokens use: hex and rgb()/rgba(). */
export function parseColor(value) {
  const hex = value.match(/^#([\da-f]{3,8})$/i)?.[1];
  if (hex && [3, 4, 6, 8].includes(hex.length)) {
    const digits = hex.length <= 4 ? [...hex].map((digit) => digit + digit).join('') : hex;
    const at = (index) => parseInt(digits.slice(index, index + 2), 16);
    return { r: at(0), g: at(2), b: at(4), a: digits.length === 8 ? at(6) / 255 : 1 };
  }
  const parts = value.match(/^rgba?\(([^()]*)\)$/)?.[1].split(',').map(Number);
  if (parts && (parts.length === 3 || parts.length === 4) && parts.every(Number.isFinite)) {
    return { r: parts[0], g: parts[1], b: parts[2], a: parts[3] ?? 1 };
  }
  throw new Error(`Contrast gate cannot read color: ${value}`);
}

function resolve(palette, token) {
  const value = palette[token];
  const alias = value.match(/^var\((--js-[\w-]+)\)$/);
  return alias ? resolve(palette, alias[1]) : parseColor(value);
}

export function composite(fg, bg) {
  const mix = (key) => fg[key] * fg.a + bg[key] * (1 - fg.a);
  return { r: mix('r'), g: mix('g'), b: mix('b'), a: 1 };
}

function luminance(color) {
  const [r, g, b] = [color.r, color.g, color.b].map((channel) => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrastRatio(fg, bg) {
  const [high, low] = [luminance(composite(fg, bg)), luminance(bg)].sort((a, b) => b - a);
  return (high + 0.05) / (low + 0.05);
}

export function paletteContrast(palette, token, surface) {
  return contrastRatio(resolve(palette, token), composite(resolve(palette, surface), resolve(palette, '--js-color-bg')));
}

export function contrastFailures(palettes) {
  const failures = [];
  for (const [id, palette] of Object.entries(palettes)) {
    for (const { token, min, surfaces } of CONTRAST_RULES) {
      for (const surface of surfaces) {
        const ratio = paletteContrast(palette, token, surface);
        if (ratio < min) failures.push(`${id}: ${token} on ${surface} is ${ratio.toFixed(2)}:1, needs ${min}:1`);
      }
    }
  }
  return failures;
}
