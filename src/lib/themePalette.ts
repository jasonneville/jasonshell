import { BUILTIN_THEME_PALETTES } from './builtinThemePalettes.generated.js';
import { normalizeShellThemeId, shellThemeById, type ShellThemeId, type ShellThemeMode } from './themes.js';

export type ThemePalette = Record<string, string>;
export interface CustomTheme {
  id: `custom:${string}`;
  label: string;
  baseBuiltinId: ShellThemeId;
  mode: ShellThemeMode;
  revision: number;
  startingPalette: ThemePalette;
  palette: ThemePalette;
}
export const THEME_PALETTE_TOKENS = Object.freeze(Object.keys(BUILTIN_THEME_PALETTES['base-dark']));
export const MAX_RESOLVED_THEME_COLOR_LENGTH = 4096;
const allowedTokens = new Set(THEME_PALETTE_TOKENS);
const compositeToken = /^--js-(?:shadow-|focus-ring$|inset-highlight$)/;
const hex = /^#(?:[\da-f]{3}|[\da-f]{4}|[\da-f]{6}|[\da-f]{8})$/i;

export function getPresetThemePalette(id: unknown): ThemePalette {
  return { ...BUILTIN_THEME_PALETTES[normalizeShellThemeId(id)] };
}

/** Canonical picker storage is #rrggbb or #rrggbbaa; alpha is 0..1. */
export function normalizeThemeColor(value: unknown, alpha?: number): string {
  if (typeof value !== 'string' || !hex.test(value.trim())) throw new Error('Use a hexadecimal color');
  let digits = value.trim().slice(1).toLowerCase();
  if (digits.length <= 4) digits = [...digits].map((digit) => digit + digit).join('');
  if (alpha !== undefined) {
    if (!Number.isFinite(alpha) || alpha < 0 || alpha > 1) throw new Error('Alpha must be between 0 and 1');
    digits = digits.slice(0, 6) + Math.round(alpha * 255).toString(16).padStart(2, '0');
  }
  return `#${digits}`;
}

function splitArguments(value: string): string[] {
  let depth = 0;
  let start = 0;
  const parts: string[] = [];
  for (let index = 0; index < value.length; index++) {
    if (value[index] === '(') depth++;
    if (value[index] === ')') depth--;
    if (depth < 0) return [];
    if (value[index] === ',' && depth === 0) {
      parts.push(value.slice(start, index).trim());
      start = index + 1;
    }
  }
  return depth === 0 ? [...parts, value.slice(start).trim()] : [];
}

/** Deliberately small color grammar; never delegates untrusted CSS to the browser. */
function safeColor(value: string, depth = 0): boolean {
  if (depth > 8 || value.length > 512) return false;
  if (hex.test(value) || value === 'transparent') return true;
  const alias = value.match(/^var\((--js-[\w-]+)\)$/);
  if (alias) return allowedTokens.has(alias[1]) && !compositeToken.test(alias[1]);
  const rgb = value.match(/^(rgb|rgba)\(([^()]*)\)$/);
  if (rgb) {
    const numbers = rgb[2].split(',').map((part) => part.trim());
    const count = rgb[1] === 'rgb' ? 3 : 4;
    return numbers.length === count && numbers.every((part, index) => {
      if (!/^(?:\d+(?:\.\d+)?|\.\d+)$/.test(part)) return false;
      const number = Number(part);
      return number >= 0 && number <= (index === 3 ? 1 : 255);
    });
  }
  const mix = value.match(/^color-mix\((.*)\)$/);
  if (!mix) return false;
  const parts = splitArguments(mix[1]);
  return parts.length === 3 && parts[0] === 'in srgb' && parts.slice(1).every((part) => {
    const weighted = part.match(/^(.*)\s+(\d+(?:\.\d+)?)%$/);
    return weighted ? Number(weighted[2]) <= 100 && safeColor(weighted[1], depth + 1) : safeColor(part, depth + 1);
  });
}

function compositeParts(value: string): { geometry: string; color: string; suffix: string } | null {
  const layers = splitArguments(value);
  const match = layers[0]?.match(/^(.*?)(#[\da-f]{3,8}|rgba?\(.*\)|var\(--js-[\w-]+\)|color-mix\(.*\)|transparent)$/i);
  return match ? { geometry: match[1], color: match[2], suffix: layers.length > 1 ? ', ' + layers.slice(1).join(', ') : '' } : null;
}
function compositeGeometry(value: string): string | null {
  const layers = splitArguments(value);
  if (!layers.length) return null;
  const parts = layers.map((layer) => compositeParts(layer));
  return parts.every((part) => part && safeColor(part.color)) ? parts.map((part) => part!.geometry).join('|') : null;
}
const compositeGeometries = new Map(THEME_PALETTE_TOKENS.filter((name) => compositeToken.test(name))
  .map((name) => [name, new Set(Object.values(BUILTIN_THEME_PALETTES).map((preset) => compositeGeometry(preset[name])))]));

export function validateThemePalette(value: unknown): ThemePalette {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid theme palette');
  const entries = Object.entries(value);
  if (entries.length !== THEME_PALETTE_TOKENS.length) throw new Error('Palette must include every editable token');
  const palette: ThemePalette = {};
  for (const [name, input] of entries) {
    if (!allowedTokens.has(name) || typeof input !== 'string' || input.length > 512) throw new Error(`Invalid palette token: ${name}`);
    const color = input.trim();
    if (compositeToken.test(name)) {
      // Geometry is owned by the preset, never user-editable. All original
      // preset geometries are accepted so copies stay byte-equivalent.
      const geometry = compositeGeometry(color);
      if (geometry === null || !compositeGeometries.get(name)?.has(geometry)) throw new Error(`Invalid composite color: ${name}`);
    } else if (!safeColor(color)) throw new Error(`Invalid color: ${name}`);
    palette[name] = color;
  }
  const expandedLengths = new Map<string, number>();
  function visit(name: string, path: Set<string>): number {
    if (!Object.hasOwn(palette, name)) throw new Error(`Missing alias: ${name}`);
    if (path.has(name)) throw new Error('Theme palette contains an alias cycle');
    const cached = expandedLengths.get(name);
    if (cached !== undefined) return cached;
    const next = new Set([...path, name]);
    let length = palette[name].length;
    // Count substitutions without materializing expanded strings. Acyclic DAGs
    // can still expand exponentially when the same alias occurs repeatedly.
    for (const [reference, ref] of palette[name].matchAll(/var\((--js-[\w-]+)\)/g)) {
      length += visit(ref, next) - reference.length;
      if (length > MAX_RESOLVED_THEME_COLOR_LENGTH) throw new Error('Theme alias expansion exceeds the resolved color limit');
    }
    expandedLengths.set(name, length);
    return length;
  }
  THEME_PALETTE_TOKENS.forEach((name) => visit(name, new Set()));
  return Object.fromEntries(THEME_PALETTE_TOKENS.map((name) => [name, palette[name]]));
}

function colorLayers(palette: ThemePalette, token: string): string[] {
  if (!allowedTokens.has(token)) throw new Error('Unknown editable token');
  return compositeToken.test(token) ? splitArguments(palette[token]).map((layer) => compositeParts(layer)!.color) : [palette[token]];
}
function checkLayerIndex(index: number, count: number): void {
  if (!Number.isSafeInteger(index) || index < 0 || index >= count) throw new Error('Invalid theme color layer index');
}

/** Raw expressions preserve aliases. Use resolveThemePaletteColor for display. */
export function getThemePaletteColorLayers(palette: ThemePalette, token: string): string[] {
  return colorLayers(validateThemePalette(palette), token);
}

export function setThemePaletteColor(palette: ThemePalette, token: string, color: string, alpha?: number, layerIndex = 0): ThemePalette {
  const validated = validateThemePalette(palette);
  checkLayerIndex(layerIndex, colorLayers(validated, token).length);
  const normalized = normalizeThemeColor(color, alpha);
  const value = compositeToken.test(token) ? splitArguments(validated[token]).map((layer, index) => {
    return index === layerIndex ? compositeParts(layer)!.geometry + normalized : layer;
  }).join(', ') : normalized;
  return validateThemePalette({ ...validated, [token]: value });
}

/** Restore the original raw tint (including aliases/alpha precision), without
 * resetting neighboring layers or accepting geometry from another baseline. */
export function resetThemePaletteColorLayer(palette: ThemePalette, startingPalette: ThemePalette, token: string, layerIndex = 0): ThemePalette {
  const validated = validateThemePalette(palette);
  const baseline = validateThemePalette(startingPalette);
  const currentColors = colorLayers(validated, token);
  const originalColors = colorLayers(baseline, token);
  checkLayerIndex(layerIndex, currentColors.length);
  checkLayerIndex(layerIndex, originalColors.length);
  if (compositeToken.test(token) && compositeGeometry(validated[token]) !== compositeGeometry(baseline[token])) throw new Error('Theme reset baseline geometry does not match');
  const value = compositeToken.test(token) ? splitArguments(validated[token]).map((layer, index) => {
    return index === layerIndex ? compositeParts(layer)!.geometry + originalColors[layerIndex] : layer;
  }).join(', ') : originalColors[layerIndex];
  return validateThemePalette({ ...validated, [token]: value });
}

/** Resolve aliases without computed-style extraction; composites return tint only. */
export function resolveThemePaletteColor(palette: ThemePalette, token: string, layerIndex = 0): string {
  const validated = validateThemePalette(palette);
  const layers = colorLayers(validated, token);
  checkLayerIndex(layerIndex, layers.length);
  const expanded = new Map<string, string>();
  function expand(name: string): string {
    const cached = expanded.get(name);
    if (cached !== undefined) return cached;
    const value = validated[name];
    if (value === undefined) throw new Error('Unknown editable token');
    const resolved = value.replace(/var\((--js-[\w-]+)\)/g, (_match, ref: string) => expand(ref));
    expanded.set(name, resolved);
    return resolved;
  }
  return layers[layerIndex].replace(/var\((--js-[\w-]+)\)/g, (_match, ref: string) => expand(ref));
}

export function createCustomTheme(source: unknown, label: string, id: `custom:${string}` = `custom:${crypto.randomUUID()}`): CustomTheme {
  const custom = typeof source === 'object' && source !== null ? validateCustomTheme(source) : null;
  const base = shellThemeById(custom?.baseBuiltinId ?? source);
  const palette = custom ? { ...custom.palette } : getPresetThemePalette(base.id);
  return validateCustomTheme({ id, label, baseBuiltinId: base.id, mode: custom?.mode ?? base.mode, revision: 0,
    startingPalette: { ...palette }, palette: { ...palette } });
}

export function validateCustomTheme(input: unknown): CustomTheme {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new Error('Invalid custom theme');
  const value = input as Record<string, unknown>;
  if (typeof value.id !== 'string' || !/^custom:[a-zA-Z0-9_-]{1,100}$/.test(value.id)) throw new Error('Invalid custom theme ID');
  if (typeof value.label !== 'string' || !value.label.trim() || value.label.trim().length > 80 || /[\u0000-\u001f\u007f]/.test(value.label)) throw new Error('Theme name must be 1–80 characters');
  if (normalizeShellThemeId(value.baseBuiltinId) !== value.baseBuiltinId) throw new Error('Invalid base theme');
  if (value.mode !== 'dark' && value.mode !== 'light') throw new Error('Invalid color scheme');
  if (!Number.isSafeInteger(value.revision) || (value.revision as number) < 0) throw new Error('Invalid theme revision');
  const startingPalette = validateThemePalette(value.startingPalette);
  const palette = validateThemePalette(value.palette);
  const builtin = getPresetThemePalette(value.baseBuiltinId);
  for (const name of compositeGeometries.keys()) {
    if (compositeGeometry(palette[name]) !== compositeGeometry(startingPalette[name]) || compositeGeometry(startingPalette[name]) !== compositeGeometry(builtin[name])) throw new Error('Theme composite geometry must match its base preset');
  }
  return { id: value.id as CustomTheme['id'], label: value.label.trim(), baseBuiltinId: value.baseBuiltinId as ShellThemeId,
    mode: value.mode, revision: value.revision as number, startingPalette, palette };
}
