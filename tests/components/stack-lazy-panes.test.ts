import { afterEach, beforeEach, expect, it, vi } from 'vitest';

// FE-34 acceptance (mocked IPC, mocked pane modules; live native not covered).
// StackGitPanel / StackTextEditor / StackTerminalPane (and therefore xterm) must be loaded by
// dynamic import() only when first needed. Pane mocks are re-registered (vi.doMock) before every test so each dynamic import re-runs its gated factory
// (vitest caches a factory result per registration);
// a static import is observable as a factory evaluation during the surface import (loadedAtImport).
const ctl = vi.hoisted(() => {
  const state = {
    loads: { editor: 0, git: 0, terminal: 0, xterm: 0 },
    gate: { editor: null as Promise<void> | null, git: null as Promise<void> | null },
    fail: { editor: 0, git: 0 }
  };
  const factories = {
    editor: async () => {
      state.loads.editor += 1; await state.gate.editor;
      if (state.fail.editor > 0) { state.fail.editor -= 1; throw new Error('Failed to fetch dynamically imported module: editor'); }
      return { default: (await import('./fixtures/stack-lazy-editor.svelte')).default };
    },
    git: async () => {
      state.loads.git += 1; await state.gate.git;
      if (state.fail.git > 0) { state.fail.git -= 1; throw new Error('Failed to fetch dynamically imported module: git'); }
      return { default: (await import('./fixtures/stack-lazy-git-panel.svelte')).default };
    },
    terminal: async () => { state.loads.terminal += 1; return { default: (await import('./fixtures/stack-lazy-git-panel.svelte')).default }; }
  };
  return Object.assign(state, { factories });
});
vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-external-refresh/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-external-refresh/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-external-refresh/bridge'));
vi.mock('../../src/components/StackTextEditor.svelte', () => ctl.factories.editor());
vi.mock('../../src/components/StackGitPanel.svelte', () => ctl.factories.git());
vi.mock('../../src/components/StackTerminalPane.svelte', () => ctl.factories.terminal());
vi.mock('@xterm/xterm', () => { ctl.loads.xterm += 1; return { Terminal: class {} }; });

import * as bridge from '../browser/stack-external-refresh/bridge';
import * as tl from '@testing-library/svelte';
import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';
// Evidence of static imports: pane factories evaluated while the Stack graph was imported.
const loadedAtImport = { ...ctl.loads };
type Boot = { bridge: typeof bridge; tl: typeof tl };
function gate() { let open!: () => void; const promise = new Promise<void>((resolve) => { open = resolve; }); return { promise, open }; }
async function boot(): Promise<Boot> {
  bridge.resetBridge();
  tl.render(StackPopupSurface);
  await tl.screen.findByText('draft.txt');
  await tl.waitFor(() => expect(document.querySelector('.stack-path')?.getAttribute('title')).toBe(bridge.root));
  return { bridge, tl };
}
const quiet = () => new Promise((resolve) => setTimeout(resolve, 40));
function press(target: Element, button = 3, type = 'pointerdown') {
  const event = new MouseEvent(type, { button, bubbles: true, cancelable: true });
  Object.defineProperty(event, 'pointerType', { value: 'mouse' });
  target.dispatchEvent(event); return event;
}
const back = (tl: Boot['tl']) => press(tl.screen.getByRole('region', { name: 'Stack browser' }));
async function openEditor(tl: Boot['tl']) { await tl.fireEvent.dblClick(tl.screen.getByText('draft.txt')); }
async function openGit(tl: Boot['tl']) {
  const branch = await tl.screen.findByRole('button', { name: 'main', exact: true });
  await tl.waitFor(() => expect((branch as HTMLButtonElement).disabled).toBe(false));
  await tl.fireEvent.click(branch);
}
const pending = (tl: Boot['tl'], pane: RegExp) => tl.screen.getByRole('status', { name: pane });
const grid = (tl: Boot['tl']) => tl.screen.queryByRole('grid', { name: 'Folder details' });

beforeEach(() => {
  vi.doMock('../../src/components/StackTextEditor.svelte', ctl.factories.editor);
  vi.doMock('../../src/components/StackGitPanel.svelte', ctl.factories.git);
  vi.doMock('../../src/components/StackTerminalPane.svelte', ctl.factories.terminal);
  ctl.loads = { editor: 0, git: 0, terminal: 0, xterm: 0 }; ctl.gate = { editor: null, git: null }; ctl.fail = { editor: 0, git: 0 }; });
afterEach(() => { tl.cleanup(); });

it('harness: a rejected mocked pane import can be retried (validates the failure/retry tests below)', async () => {
  ctl.fail.editor = 1;
  await expect(import('../../src/components/StackTextEditor.svelte')).rejects.toThrow(/dynamically imported|error when mocking/);
  await expect(import('../../src/components/StackTextEditor.svelte')).resolves.toHaveProperty('default');
});

it('first popup render imports no pane module, no xterm', async () => {
  await boot(); await quiet();
  expect(loadedAtImport, 'a static import evaluated a pane module with the Stack surface').toEqual({ editor: 0, git: 0, terminal: 0, xterm: 0 });
  expect(ctl.loads).toEqual({ editor: 0, git: 0, terminal: 0, xterm: 0 });
});

it('opening a text file shows an accessible pending state, then the editor, and imports nothing else', async () => {
  const { tl } = await boot(); const held = gate(); ctl.gate.editor = held.promise;
  await openEditor(tl);
  await tl.waitFor(() => expect(pending(tl, /loading.*editor/i)).toBeTruthy());
  expect(tl.screen.queryByRole('region', { name: 'Lazy editor fixture' })).toBeNull();
  held.open();
  await tl.screen.findByRole('region', { name: 'Lazy editor fixture' });
  expect(tl.screen.queryByRole('status', { name: /loading.*editor/i })).toBeNull();
  expect(ctl.loads).toEqual({ editor: 1, git: 0, terminal: 0, xterm: 0 });
});

it('opening the Git panel shows a pending state then the panel; mouse Back after load closes it', async () => {
  const { tl } = await boot(); const held = gate(); ctl.gate.git = held.promise;
  await openGit(tl);
  await tl.waitFor(() => expect(pending(tl, /loading.*git/i)).toBeTruthy());
  held.open();
  await tl.screen.findByRole('region', { name: 'Lazy git panel fixture' });
  expect(tl.screen.queryByRole('status', { name: /loading.*git/i })).toBeNull();
  expect(back(tl).defaultPrevented).toBe(true);
  await tl.waitFor(() => expect(tl.screen.queryByRole('region', { name: 'Lazy git panel fixture' })).toBeNull());
  expect(grid(tl)).toBeTruthy();
  expect(ctl.loads).toMatchObject({ editor: 0, terminal: 0, xterm: 0 });
});

it.each([['editor', openEditor, /loading.*editor/i, 'Lazy editor fixture'], ['git', openGit, /loading.*git/i, 'Lazy git panel fixture']] as const)(
  'mouse Back while the %s pane is still loading returns to the file list and the late pane never mounts',
  async (kind, open, label, region) => {
    const { tl } = await boot(); const held = gate(); ctl.gate[kind] = held.promise;
    await open(tl);
    await tl.waitFor(() => expect(pending(tl, label)).toBeTruthy());
    expect(back(tl).defaultPrevented).toBe(true);
    await tl.waitFor(() => expect(tl.screen.queryByRole('status', { name: label })).toBeNull());
    expect(grid(tl)).toBeTruthy();
    held.open(); await quiet();
    expect(tl.screen.queryByRole('region', { name: region })).toBeNull();
    expect(grid(tl)).toBeTruthy();
  }
);

it('closing the popup while a pane loads hides it and the late pane never mounts', async () => {
  const { tl, bridge } = await boot(); const held = gate(); ctl.gate.git = held.promise;
  await openGit(tl);
  await tl.waitFor(() => expect(pending(tl, /loading.*git/i)).toBeTruthy());
  await bridge.emit('stack-popup:close-requested');
  await tl.waitFor(() => expect(bridge.ledger.some((entry) => entry.command === 'hide_stack_popup')).toBe(true));
  held.open(); await quiet();
  expect(tl.screen.queryByRole('region', { name: 'Lazy git panel fixture' })).toBeNull();
});

it('navigating to another folder while the editor loads cancels the pending pane', async () => {
  const { tl, bridge } = await boot(); const held = gate(); ctl.gate.editor = held.promise;
  await openEditor(tl);
  await tl.waitFor(() => expect(pending(tl, /loading.*editor/i)).toBeTruthy());
  const input = tl.screen.getByRole('textbox', { name: 'Current folder path' });
  await tl.fireEvent.input(input, { target: { value: bridge.other } }); await tl.fireEvent.submit(input.closest('form')!);
  await tl.waitFor(() => expect(document.querySelector('.stack-path')?.getAttribute('title')).toBe(bridge.other));
  held.open(); await quiet();
  expect(tl.screen.queryByRole('region', { name: 'Lazy editor fixture' })).toBeNull();
  expect(grid(tl)).toBeTruthy();
});

it('editor import failure shows an alert with Retry; Retry re-imports and mounts the editor', async () => {
  const { tl } = await boot(); ctl.fail.editor = 1;
  await openEditor(tl);
  const alert = await tl.screen.findByRole('alert');
  expect(alert.textContent).toMatch(/editor/i);
  expect(alert.textContent).toMatch(/fail|couldn.?t|unable/i);
  await tl.fireEvent.click(tl.screen.getByRole('button', { name: /retry/i }));
  await tl.screen.findByRole('region', { name: 'Lazy editor fixture' });
  expect(tl.screen.queryByRole('alert')).toBeNull();
  expect(ctl.loads.editor).toBe(2);
});

it('git panel import failure never strands navigation: Back leaves the error and the list stays usable', async () => {
  const { tl, bridge } = await boot(); ctl.fail.git = 1;
  await openGit(tl);
  await tl.screen.findByRole('alert');
  expect(tl.screen.getByRole('button', { name: /retry/i })).toBeTruthy();
  expect(back(tl).defaultPrevented).toBe(true);
  await tl.waitFor(() => expect(tl.screen.queryByRole('alert')).toBeNull());
  expect(grid(tl)).toBeTruthy();
  await tl.fireEvent.dblClick(tl.screen.getByText('src'));
  await tl.waitFor(() => expect(document.querySelector('.stack-path')?.getAttribute('title')).toBe(`${bridge.root}\\src`));
});

it('dirty-editor guard works through the lazy pane: native close and mouse Back both require confirmation', async () => {
  const { tl, bridge } = await boot();
  await openEditor(tl);
  await tl.fireEvent.click(await tl.screen.findByRole('button', { name: /Make lazy draft dirty/ }));
  await bridge.emit('stack-popup:close-requested');
  const dialog = await tl.screen.findByRole('alertdialog');
  expect(bridge.ledger.some((entry) => entry.command === 'hide_stack_popup')).toBe(false);
  await tl.fireEvent.click(tl.screen.getByRole('button', { name: 'Cancel' }));
  expect(tl.screen.queryByRole('alertdialog')).toBeNull();
  expect(tl.screen.getByRole('region', { name: 'Lazy editor fixture' })).toBeTruthy();
  expect(dialog).toBeTruthy();
  expect(back(tl).defaultPrevented).toBe(true);
  await tl.screen.findByRole('alertdialog');
  expect(tl.screen.getByRole('region', { name: 'Lazy editor fixture' })).toBeTruthy();
  await tl.fireEvent.click(tl.screen.getByRole('button', { name: 'Discard' }));
  await tl.waitFor(() => expect(tl.screen.queryByRole('region', { name: 'Lazy editor fixture' })).toBeNull());
  expect(grid(tl)).toBeTruthy();
});

it('a clean lazy editor closes via Back and returns focus to the folder grid', async () => {
  const { tl } = await boot();
  await openEditor(tl);
  await tl.screen.findByRole('region', { name: 'Lazy editor fixture' });
  await tl.fireEvent.click(tl.screen.getByRole('button', { name: 'Editor back' }));
  await tl.waitFor(() => expect(grid(tl)).toBeTruthy());
  await tl.waitFor(() => expect(document.activeElement).toBe(grid(tl)));
});

it('Terminal and xterm stay unloaded after exercising editor and Git panes', async () => {
  const { tl } = await boot();
  await openEditor(tl); await tl.screen.findByRole('region', { name: 'Lazy editor fixture' });
  await tl.fireEvent.click(tl.screen.getByRole('button', { name: 'Editor back' }));
  await tl.waitFor(() => expect(grid(tl)).toBeTruthy());
  await openGit(tl); await tl.screen.findByRole('region', { name: 'Lazy git panel fixture' });
  expect(ctl.loads.terminal).toBe(0);
  expect(ctl.loads.xterm).toBe(0);
});
