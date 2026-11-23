import { getPresetThemePalette, type ThemePalette } from './themePalette.js';
import { currentPreviewTheme, subscribeThemePreview } from './themePreview.js';
import { getSavedThemeState, shellThemeById, subscribeShellThemes, type ShellThemeMode } from './themes.js';

/** Passed to xterm, which lifts unreadable ANSI/background pairs at paint time. */
export const TERMINAL_MINIMUM_CONTRAST_RATIO = 4.5;

const ANSI_KEYS = [
  'black', 'red', 'green', 'yellow', 'blue', 'magenta', 'cyan', 'white',
  'brightBlack', 'brightRed', 'brightGreen', 'brightYellow', 'brightBlue', 'brightMagenta', 'brightCyan', 'brightWhite'
] as const;
export const TERMINAL_THEME_KEYS = Object.freeze([
  'background', 'foreground', 'cursor', 'cursorAccent', 'selectionBackground', ...ANSI_KEYS
] as const);
export type TerminalTheme = Record<(typeof TERMINAL_THEME_KEYS)[number], string>;

// Each theme's own published terminal palette: background, foreground, cursor,
// selection, then the 16 ANSI colors in xterm order. Adjusted only where the
// published value is unreadable as plain text: Solarized uses base1/base01
// foregrounds, Gruvbox Light keeps a dark ANSI black.
const CANONICAL: Readonly<Record<string, string>> = {
  monokai: '272822 f8f8f2 f8f8f0 49483e  272822 f92672 a6e22e e6db74 66d9ef ae81ff a1efe4 f8f8f2  75715e f92672 a6e22e e6db74 66d9ef ae81ff a1efe4 f9f8f5',
  'atom-one-dark': '282c34 abb2bf 528bff 3e4451  282c34 e06c75 98c379 e5c07b 61afef c678dd 56b6c2 abb2bf  5c6370 e06c75 98c379 d19a66 61afef c678dd 56b6c2 ffffff',
  'atom-one-light': 'fafafa 383a42 526fff e5e5e6  383a42 e45649 50a14f c18401 4078f2 a626a4 0184bc a0a1a7  696c77 e45649 50a14f c18401 4078f2 a626a4 0184bc ffffff',
  nord: '2e3440 d8dee9 d8dee9 434c5e  3b4252 bf616a a3be8c ebcb8b 81a1c1 b48ead 88c0d0 e5e9f0  4c566a bf616a a3be8c ebcb8b 81a1c1 b48ead 8fbcbb eceff4',
  dracula: '282a36 f8f8f2 f8f8f2 44475a  21222c ff5555 50fa7b f1fa8c bd93f9 ff79c6 8be9fd f8f8f2  6272a4 ff6e6e 69ff94 ffffa5 d6acff ff92df a4ffff ffffff',
  'solarized-dark': '002b36 93a1a1 93a1a1 073642  073642 dc322f 859900 b58900 268bd2 d33682 2aa198 eee8d5  002b36 cb4b16 586e75 657b83 839496 6c71c4 93a1a1 fdf6e3',
  'solarized-light': 'fdf6e3 586e75 586e75 eee8d5  073642 dc322f 859900 b58900 268bd2 d33682 2aa198 eee8d5  002b36 cb4b16 586e75 657b83 839496 6c71c4 93a1a1 fdf6e3',
  'github-dark': '0d1117 e6edf3 2f81f7 264f78  484f58 ff7b72 3fb950 d29922 58a6ff bc8cff 39c5cf b1bac4  6e7681 ffa198 56d364 e3b341 79c0ff d2a8ff 56d4dd ffffff',
  'github-light': 'ffffff 1f2328 0969da bbdfff  24292f cf222e 116329 4d2d00 0969da 8250df 1b7c83 6e7781  57606a a40e26 1a7f37 633c01 218bff a475f9 3192aa 8c959f',
  'gruvbox-dark': '282828 ebdbb2 ebdbb2 504945  282828 cc241d 98971a d79921 458588 b16286 689d6a a89984  928374 fb4934 b8bb26 fabd2f 83a598 d3869b 8ec07c ebdbb2',
  'gruvbox-light': 'fbf1c7 3c3836 3c3836 d5c4a1  3c3836 cc241d 98971a d79921 458588 b16286 689d6a 7c6f64  928374 9d0006 79740e b57614 076678 8f3f71 427b58 504945',
  'tokyo-night': '1a1b26 c0caf5 c0caf5 283457  15161e f7768e 9ece6a e0af68 7aa2f7 bb9af7 7dcfff a9b1d6  414868 f7768e 9ece6a e0af68 7aa2f7 bb9af7 7dcfff c0caf5',
  'catppuccin-mocha': '1e1e2e cdd6f4 f5e0dc 585b70  45475a f38ba8 a6e3a1 f9e2af 89b4fa f5c2e7 94e2d5 bac2de  585b70 f38ba8 a6e3a1 f9e2af 89b4fa f5c2e7 94e2d5 a6adc8',
  'ayu-dark': '0f1419 e6e1cf f29718 253340  000000 ff3333 b8cc52 e7c547 36a3d9 f07178 95e6cb ffffff  323232 ff6565 eafe84 fff779 68d5ff ffa3aa c7fffd ffffff',
  'night-owl': '011627 d6deeb 80a4c2 1d3b53  011627 ef5350 22da6e addb67 82aaff c792ea 21c7a8 ffffff  575656 ef5350 22da6e ffeb95 82aaff c792ea 7fdbca ffffff',
  palenight: '292d3e bfc7d5 ffcc00 3c435e  292d3e ff5572 a9c77d ffcb6b 82aaff c792ea 89ddff ffffff  676e95 ff5572 c3e88d ffcb6b 82aaff c792ea 89ddff ffffff',
  'one-dark-pro': '282c34 abb2bf 528bff 404859  3f4451 e05561 8cc265 d18f52 4aa5f0 c162de 42b3c2 d7dae0  4f5666 ff616e a5e075 f0a45d 4dc4ff de73ff 4cd1e0 e6e6e6',
  'material-ocean': '0f111a a6accd ffcc00 2b3048  000000 f07178 c3e88d ffcb6b 82aaff c792ea 89ddff ffffff  464b5d f07178 c3e88d ffcb6b 82aaff c792ea 89ddff ffffff',
  'everforest-dark': '2d353b d3c6aa d3c6aa 475258  475258 e67e80 a7c080 dbbc7f 7fbbb3 d699b6 83c092 d3c6aa  859289 e67e80 a7c080 dbbc7f 7fbbb3 d699b6 83c092 d3c6aa',
  'rose-pine-moon': '232136 e0def4 e0def4 44415a  393552 eb6f92 3e8fb0 f6c177 9ccfd8 c4a7e7 ea9a97 e0def4  6e6a86 eb6f92 3e8fb0 f6c177 9ccfd8 c4a7e7 ea9a97 e0def4',
  'oceanic-next': '1b2b34 cdd3de cdd3de 4f5b66  1b2b34 ec5f67 99c794 fac863 6699cc c594c5 5fb3b3 c0c5ce  65737e ec5f67 99c794 fac863 6699cc c594c5 5fb3b3 d8dee9',
  'shades-of-purple': '2d2b55 ffffff fad000 b362ff55  000000 ec3a37 3ad900 fad000 6943ff ff2c70 80fcff ffffff  5c5c61 ec3a37 3ad900 fad000 6943ff fb94ff 80fcff ffffff',
  'kanagawa-wave': '1f1f28 dcd7ba c8c093 2d4f67  16161d c34043 76946a c0a36e 7e9cd8 957fb8 6a9589 c8c093  727169 e82424 98bb6c e6c384 7fb4ca 938aa9 7aa89f dcd7ba',
  'vitesse-dark': '121212 dbd7ca dbd7ca 2a2a2a  393a34 cb7676 4d9375 e6cc77 6394bf d9739f 5eaab5 dbd7ca  777777 cb7676 4d9375 e6cc77 6394bf d9739f 5eaab5 ffffff',
  'aura-dark': '15141b edecee a277ff 3d375e  110f18 ff6767 61ffca ffca85 a277ff f694ff 61ffca edecee  6d6d6d ff6767 61ffca ffca85 a277ff f694ff 61ffca edecee',
  horizon: '1c1e26 d5d8da e95678 2e303e  16161c e95678 29d398 fab795 26bbd9 ee64ac 59e1e3 d5d8da  5b5858 ec6a88 3fdaa4 fbc3a7 3fc4de f075b5 6be4e6 ffffff',
  'moonlight-ii': '222436 c8d3f5 c8d3f5 2d3f76  1b1d2b ff757f c3e88d ffc777 82aaff c099ff 86e1fc c8d3f5  444a73 ff8d94 c7fb6d ffd8ab 9ab8ff caabff b2ebff ffffff',
  'synthwave-84': '262335 ffffff ff7edb 463465  241b2f fe4450 72f1b8 fede5d 03edf9 ff7edb 03edf9 ffffff  614d85 fe4450 72f1b8 fede5d 03edf9 ff7edb 03edf9 ffffff',
  'github-dark-dimmed': '22272e adbac7 539bf5 3b5070  545d68 f47067 57ab5a c69026 539bf5 b083f0 39c5cf 909dab  636e7b ff938a 6bc46d daaa3f 6cb6ff dcbdfb 56d4dd cdd9e5',
  'tomorrow-night': '1d1f21 c5c8c6 c5c8c6 373b41  1d1f21 cc6666 b5bd68 f0c674 81a2be b294bb 8abeb7 c5c8c6  969896 cc6666 b5bd68 f0c674 81a2be b294bb 8abeb7 ffffff',
  andromeda: '23262e d5ced9 d5ced9 3d4352  23262e ee5d43 96e072 ffe66d 7cb7ff c74ded 00e8c6 d5ced9  746f77 ee5d43 96e072 ffe66d 7cb7ff c74ded 00e8c6 d5ced9'
};
/** Shared ANSI sets for JasonShell's own, custom, and unknown themes. */
const ANSI_BY_MODE: Readonly<Record<ShellThemeMode, string>> = {
  dark: '2a3142 ff7a8a 7fd8a4 f0c674 8aa7ff c8a2ff 7cd4e0 d7deee  5b6680 ff9aa6 a3e8bf ffdd94 b0c4ff dcc1ff a3e6ef fbfdff',
  light: '1e293b b42334 1c7a45 8a5a00 1d4ed8 8b2fb0 0e7490 5b677a  475569 c8384a 23804c 966000 2f6bff a246cc 0f7f97 64748b'
};

type Rgba = [r: number, g: number, b: number, a: number];

function parseColor(value: string): Rgba | null {
  const text = value.trim();
  if (text === 'transparent') return [0, 0, 0, 0];
  let digits = text.match(/^#([\da-f]{3,8})$/i)?.[1];
  if (digits) {
    if (digits.length <= 4) digits = [...digits].map((digit) => digit + digit).join('');
    if (digits.length !== 6 && digits.length !== 8) return null;
    const bytes = digits.match(/../g)!.map((pair) => parseInt(pair, 16));
    return [bytes[0], bytes[1], bytes[2], bytes.length === 4 ? bytes[3] / 255 : 1];
  }
  const numbers = text.match(/^rgba?\(([^()]*)\)$/)?.[1].split(',').map(Number);
  if (numbers) return numbers.length >= 3 && numbers.every(Number.isFinite) ? [numbers[0], numbers[1], numbers[2], numbers[3] ?? 1] : null;
  const mix = text.match(/^color-mix\(in srgb,(.*)\)$/);
  if (!mix) return null;
  // Split on the one top-level comma; nested functions keep theirs.
  let depth = 0;
  const comma = [...mix[1]].findIndex((char) => {
    if (char === '(') depth++;
    if (char === ')') depth--;
    return char === ',' && depth === 0;
  });
  const [first, second] = [mix[1].slice(0, comma), mix[1].slice(comma + 1)].map((part) => {
    const weighted = part.trim().match(/^(.*)\s+([\d.]+)%$/);
    return { color: parseColor(weighted ? weighted[1] : part), weight: weighted ? Number(weighted[2]) : undefined };
  });
  if (comma < 0 || !first.color || !second.color) return null;
  const w1 = first.weight ?? (second.weight === undefined ? 50 : 100 - second.weight);
  const w2 = second.weight ?? 100 - w1;
  const total = w1 + w2;
  if (total <= 0) return null;
  const alpha = (first.color[3] * w1 + second.color[3] * w2) / total;
  const channel = (index: 0 | 1 | 2) => alpha === 0 ? 0
    : (first.color![index] * first.color![3] * w1 + second.color![index] * second.color![3] * w2) / total / alpha;
  return [channel(0), channel(1), channel(2), alpha];
}

/** Resolve --js-* aliases inside one palette; null when missing, cyclic, or unparseable. */
function paletteColor(palette: ThemePalette, token: string, path: readonly string[] = []): Rgba | null {
  if (path.includes(token) || typeof palette[token] !== 'string') return null;
  let failed = false;
  const literal = palette[token].replace(/var\((--js-[\w-]+)\)/g, (_match, ref: string) => {
    const color = paletteColor(palette, ref, [...path, token]);
    if (!color) failed = true;
    return color ? `rgba(${color.join(', ')})` : 'transparent';
  });
  return failed ? null : parseColor(literal);
}

function over(color: Rgba, background: Rgba): Rgba {
  const channel = (index: 0 | 1 | 2) => color[index] * color[3] + background[index] * (1 - color[3]);
  return [channel(0), channel(1), channel(2), 1];
}
function luminance(color: Rgba): number {
  const [r, g, b] = color.slice(0, 3).map((value) => {
    const channel = value / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
function contrast(left: Rgba, right: Rgba): number {
  const [high, low] = [luminance(left), luminance(right)].sort((a, b) => b - a);
  return (high + 0.05) / (low + 0.05);
}
function hex(color: Rgba): string {
  const bytes = color[3] < 1 ? [color[0], color[1], color[2], color[3] * 255] : color.slice(0, 3);
  return '#' + bytes.map((byte) => Math.round(byte).toString(16).padStart(2, '0')).join('');
}
function build(chrome: readonly string[], ansi: readonly string[]): TerminalTheme {
  const [background, foreground, cursor, selectionBackground] = chrome;
  return Object.fromEntries([
    ['background', background], ['foreground', foreground], ['cursor', cursor], ['cursorAccent', background],
    ['selectionBackground', selectionBackground], ...ANSI_KEYS.map((key, index) => [key, ansi[index]])
  ]) as TerminalTheme;
}
const colors = (list: string) => list.split(/\s+/).map((value) => `#${value}`);

/**
 * Pure map from a shell theme to a complete xterm theme. Built-ins with a
 * published terminal palette use it; JasonShell's own and custom themes follow
 * their palette surfaces, with unreadable text corrected instead of copied.
 */
export function terminalThemeFor(themeId: string, palette: ThemePalette, mode: ShellThemeMode): TerminalTheme {
  if (Object.hasOwn(CANONICAL, themeId)) {
    const values = colors(CANONICAL[themeId]);
    return build(values.slice(0, 4), values.slice(4));
  }
  const black: Rgba = [0, 0, 0, 1];
  const white: Rgba = [255, 255, 255, 1];
  const backing = mode === 'light' ? white : black;
  const base = over(paletteColor(palette, '--js-color-bg') ?? backing, backing);
  const background = over(paletteColor(palette, '--js-color-surface') ?? base, base);
  const firstReadable = (tokens: readonly string[], minimum: number): Rgba => {
    for (const token of tokens) {
      const color = paletteColor(palette, token);
      const solid = color && over(color, background);
      if (solid && contrast(solid, background) >= minimum) return solid;
    }
    return contrast(white, background) >= contrast(black, background) ? white : black;
  };
  const foreground = firstReadable(['--js-color-text', '--js-color-text-strong'], TERMINAL_MINIMUM_CONTRAST_RATIO);
  const cursor = firstReadable(['--js-color-accent', '--js-color-text'], 3);
  const selection: Rgba = [cursor[0], cursor[1], cursor[2], 0.32];
  return build([background, foreground, cursor, selection].map(hex), colors(ANSI_BY_MODE[mode]));
}

// An unedited copy of a built-in (including one just opened in the editor)
// keeps that built-in's terminal palette until its base colors really change.
const IDENTITY_TOKENS = ['--js-color-bg', '--js-color-surface', '--js-color-text'];

/** The terminal theme for what the shell shows now: editor preview, else saved selection. */
export function activeTerminalTheme(): TerminalTheme {
  const saved = getSavedThemeState();
  const custom = currentPreviewTheme() ?? saved.customThemes.find((theme) => theme.id === saved.selectedId);
  if (!custom) {
    const theme = shellThemeById(saved.selectedId);
    return terminalThemeFor(theme.id, getPresetThemePalette(theme.id), theme.mode);
  }
  const preset = getPresetThemePalette(custom.baseBuiltinId);
  const unedited = custom.mode === shellThemeById(custom.baseBuiltinId).mode
    && IDENTITY_TOKENS.every((token) => custom.palette[token] === preset[token]);
  return unedited ? terminalThemeFor(custom.baseBuiltinId, custom.palette, custom.mode)
    : terminalThemeFor(custom.id, custom.palette, custom.mode);
}

/** The slice of xterm's Terminal this module touches; keeps xterm out of the import graph. */
export interface ThemeableTerminal {
  options: { theme?: unknown };
  element?: HTMLElement | null;
}

/**
 * Keep one xterm instance on the active theme. Writes only real changes to the
 * same instance, so scrollback and selection survive. Call after open(), and
 * call the returned function when that terminal is disposed.
 */
export function bindTerminalTheme(terminal: ThemeableTerminal): () => void {
  let applied = JSON.stringify(terminal.options.theme ?? null);
  const apply = () => {
    const theme = activeTerminalTheme();
    const next = JSON.stringify(theme);
    if (next !== applied) {
      applied = next;
      terminal.options.theme = theme;
    }
    // xterm paints only whole cells; the host shows through the remainder.
    const host = terminal.element?.parentElement;
    if (host) host.style.backgroundColor = theme.background;
  };
  apply();
  const unsubscribeThemes = subscribeShellThemes(apply);
  const unsubscribePreview = subscribeThemePreview(apply);
  return () => {
    unsubscribeThemes();
    unsubscribePreview();
  };
}
