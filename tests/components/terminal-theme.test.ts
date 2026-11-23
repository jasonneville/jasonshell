import { cleanup, render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import TerminalPanelSurface from '../../src/components/TerminalPanelSurface.svelte';
import StackTerminalPane from '../../src/components/StackTerminalPane.svelte';
import { activeTerminalTheme, TERMINAL_MINIMUM_CONTRAST_RATIO, TERMINAL_THEME_KEYS } from '../../src/lib/terminalTheme';
import { createThemeEditor } from '../../src/lib/themePreview';
import { installShellThemeSelectionAuthority, setShellTheme } from '../../src/lib/themes';
import { tauriMocks } from './setup';

// VIS-14 wiring: both production terminals apply a theme built from the active
// shell theme to the SAME xterm instance on live change and preview, never
// recreate it (scrollback is kept), and leak no subscription after teardown.
// xterm is a double; the theme itself is asserted through `options.theme`.
const terminals = vi.hoisted(() => [] as any[]);
vi.mock('@xterm/xterm', () => ({ Terminal: class {
  element: HTMLElement | null = null;
  cols = 80; rows = 24;
  buffer = { active: { baseY: 0, cursorY: 0, cursorX: 0 } };
  parser = { registerOscHandler: () => ({ dispose() {} }) };
  unicode = { activeVersion: '' };
  options: Record<string, unknown>;
  themeWrites: unknown[] = [];
  write = vi.fn((text: string) => { if (this.element) this.element.textContent += text; });
  reset = vi.fn();
  dispose = vi.fn(() => this.element?.remove());
  constructor(options: Record<string, unknown>) {
    const writes = this.themeWrites;
    this.options = new Proxy(options, { set(target, key, value) { if (key === 'theme') writes.push(value); target[key as string] = value; return true; } });
    terminals.push(this);
  }
  open(host: HTMLElement) { this.element = document.createElement('div'); this.element.className = 'xterm'; host.append(this.element); }
  loadAddon() {} registerLinkProvider() {} onData() { return { dispose() {} }; } attachCustomKeyEventHandler() {}
  focus() {} scrollToBottom() {} hasSelection() { return false; } getSelection() { return ''; }
} }));
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} dispose() {} } }));
vi.mock('@xterm/addon-search', () => ({ SearchAddon: class { dispose() {} } }));
vi.mock('@xterm/addon-serialize', () => ({ SerializeAddon: class { serialize() { return ''; } dispose() {} } }));
vi.mock('@xterm/addon-unicode11', () => ({ Unicode11Addon: class {} }));
vi.mock('@xterm/addon-web-links', () => ({ WebLinksAddon: class {} }));
vi.mock('@xterm/addon-webgl', () => ({ WebglAddon: class { onContextLoss() {} dispose() {} } }));

const listeners = new Map<string, (event: { payload: any }) => void>();
let counter = 0;
const session = (sessionId: string) => ({ sessionId, cwd: 'C:\\dev', profile: 'powershell' as const, title: 'PowerShell', running: true });
async function settle() { for (let i = 0; i < 40; i += 1) { await Promise.resolve(); await tick(); } }

async function mountSurface(surface: 'panel' | 'stack') {
  if (surface === 'panel') {
    const view = render(TerminalPanelSurface);
    await settle();
    listeners.get('terminal-panel:open')?.({ payload: {} });
    await settle();
    return view;
  }
  const view = render(StackTerminalPane, { currentPath: 'C:\\dev' });
  await settle();
  await (view.component as any).startTerminal();
  await settle();
  return view;
}
const luminanceOf = (hex: string) => {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};

let releaseAuthority: () => void = () => {};
beforeEach(() => {
  releaseAuthority = installShellThemeSelectionAuthority();
  vi.useFakeTimers(); terminals.length = 0; listeners.clear(); counter = 0;
  localStorage.clear();
  document.documentElement.removeAttribute('data-theme');
  document.documentElement.removeAttribute('data-custom-theme');
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(480);
  vi.spyOn(console, 'error').mockImplementation(() => {});
  tauriMocks().listen.mockImplementation(async (name, callback) => { listeners.set(name, callback); return () => listeners.delete(name); });
  tauriMocks().invoke.mockImplementation(async (command: string, args?: any) => {
    if (command === 'list_stack_terminals') return [];
    if (command === 'start_persistent_terminal' || command === 'start_stack_terminal') return session(`session-${++counter}`);
    if (command === 'poll_stack_terminal_session') return { sessionId: args.sessionId, cwd: 'C:\\dev', running: true, chunks: [] };
    if (['resize_stack_terminal', 'write_stack_terminal', 'stop_stack_terminal'].includes(command)) return;
    throw new Error(`Unexpected terminal theme IPC: ${command}`);
  });
});
afterEach(() => { releaseAuthority(); cleanup(); vi.clearAllTimers(); });

describe.each(['panel', 'stack'] as const)('%s terminal theming', (surface) => {
  it('constructs xterm with a full theme from the active shell theme and minimumContrastRatio 4.5', async () => {
    setShellTheme('dracula');
    await mountSurface(surface);
    expect(terminals.length).toBeGreaterThan(0);
    for (const terminal of terminals) {
      const theme = terminal.options.theme as Record<string, string>;
      expect(Object.keys(theme).sort()).toEqual([...TERMINAL_THEME_KEYS].sort());
      expect(theme).toEqual(activeTerminalTheme());
      expect(theme.background.toLowerCase()).toBe('#282a36');
      expect(terminal.options.minimumContrastRatio).toBe(TERMINAL_MINIMUM_CONTRAST_RATIO);
    }
  });

  it('re-applies on live theme change to the same instances without recreating or resetting them', async () => {
    setShellTheme('dracula');
    await mountSurface(surface);
    const instances = [...terminals];
    for (const terminal of instances) terminal.write('scrollback sentinel');
    const before = instances.map((terminal) => terminal.element.textContent);
    // Startup legitimately resets/replays the stack view; only theme changes must not.
    for (const terminal of instances) { terminal.reset.mockClear(); terminal.dispose.mockClear(); }
    setShellTheme('base-light');
    await settle();
    expect(terminals).toEqual(instances); // no new Terminal constructed
    for (const terminal of instances) {
      const theme = terminal.options.theme as Record<string, string>;
      expect(theme).toEqual(activeTerminalTheme());
      expect(luminanceOf(theme.background)).toBeGreaterThan(0.5);
      expect(terminal.options.minimumContrastRatio).toBe(4.5);
      expect(terminal.dispose).not.toHaveBeenCalled();
      expect(terminal.reset).not.toHaveBeenCalled();
    }
    expect(instances.map((terminal) => terminal.element.textContent)).toEqual(before);
    setShellTheme('nord');
    await settle();
    expect((instances[0].options.theme as Record<string, string>).background.toLowerCase()).toBe('#2e3440');
  });

  it('follows a live custom-theme preview and reverts when the editor is cancelled', async () => {
    setShellTheme('dracula');
    await mountSurface(surface);
    const instances = [...terminals];
    const editor = createThemeEditor('dracula');
    const draft = editor.getDraft();
    const dark = '#102030';
    editor.update({ ...draft, palette: { ...draft.palette, '--js-color-bg': dark, '--js-color-surface': dark, '--js-color-surface-raised': dark, '--js-color-surface-sunken': dark, '--js-color-text': '#fff4d0' } });
    await settle();
    expect(terminals).toEqual(instances);
    const previewed = instances[0].options.theme as Record<string, string>;
    expect(previewed).toEqual(activeTerminalTheme());
    expect(previewed.background.toLowerCase()).not.toBe('#282a36');
    editor.cancel();
    await settle();
    expect((instances[0].options.theme as Record<string, string>).background.toLowerCase()).toBe('#282a36');
    expect(terminals).toEqual(instances);
  });

  it('paints the terminal host background from the theme immediately on mount and after a theme change', async () => {
    const normalize = (color: string) => { const probe = document.createElement('div'); probe.style.backgroundColor = color; return probe.style.backgroundColor; };
    setShellTheme('dracula');
    await mountSurface(surface);
    const hosts = () => terminals.map((terminal) => terminal.element.parentElement as HTMLElement);
    expect(hosts().length).toBeGreaterThan(0);
    for (const host of hosts()) expect(host.style.backgroundColor).toBe(normalize(activeTerminalTheme().background));
    setShellTheme('base-light');
    await settle();
    for (const host of hosts()) expect(host.style.backgroundColor).toBe(normalize(activeTerminalTheme().background));
    expect(normalize(activeTerminalTheme().background)).not.toBe(normalize('#282a36'));
  });

  it('removes its theme subscriptions on teardown (no writes or leaks after unmount)', async () => {
    setShellTheme('dracula');
    const view = await mountSurface(surface);
    const instances = [...terminals];
    view.unmount();
    await settle();
    const writes = instances.map((terminal) => terminal.themeWrites.length);
    setShellTheme('nord');
    const editor = createThemeEditor('nord');
    editor.update({ ...editor.getDraft(), label: 'x' });
    editor.cancel();
    await settle();
    expect(instances.map((terminal) => terminal.themeWrites.length)).toEqual(writes);
  });
});
