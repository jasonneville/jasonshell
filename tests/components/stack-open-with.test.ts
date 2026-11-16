import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import { child, ledger, overrides, resetBridge } from '../browser/stack-mouse/bridge';

vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-mouse/bridge'));
import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';

const candidate = (id: string) => ({ id, label: `Associated ${id}`, source: 'windows-association' });
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
beforeEach(() => {
  resetBridge();
  vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} });
  overrides.set('list_stack_open_with_candidates', () => []);
  overrides.set('open_stack_item_with_app', () => undefined);
  overrides.set('open_stack_item_with_picker', () => undefined);
});
afterEach(() => vi.unstubAllGlobals());
async function mountFolder() {
  render(StackPopupSurface);
  await fireEvent.dblClick(await screen.findByText('repo'));
  await screen.findByText('draft.txt');
}
async function menu(name = 'draft.txt') {
  await fireEvent.contextMenu(screen.getByText(name), { clientX: 40, clientY: 80 });
  return screen.findByRole('menuitem', { name: /Open with/ });
}
async function openSubmenu() {
  const trigger = await menu();
  await fireEvent.click(trigger);
  return trigger;
}

it('click exposes an explicit expanded submenu and chooser while association lookup is pending, without invented apps', async () => {
  overrides.set('list_stack_open_with_candidates', () => new Promise(() => {}));
  await mountFolder();
  const trigger = await openSubmenu();
  expect(trigger.getAttribute('aria-haspopup')).toBe('menu');
  expect(trigger.getAttribute('aria-expanded')).toBe('true');
  expect((screen.getByRole('menuitem', { name: 'Choose app...' }) as HTMLButtonElement).disabled).toBe(false);
  expect(screen.queryAllByRole('menuitem', { name: /Notepad|Visual Studio Code|Windows Terminal/ })).toHaveLength(0);
});

it.each(['empty', 'rejected'])('%s association lookup leaves only real candidates and an operable chooser', async (mode) => {
  overrides.set('list_stack_open_with_candidates', () => mode === 'empty' ? [] : Promise.reject(new Error('Association unavailable')));
  await mountFolder();
  await openSubmenu();
  await tick();
  expect(screen.queryAllByRole('menuitem', { name: /Notepad|Visual Studio Code|Windows Terminal/ })).toHaveLength(0);
  await fireEvent.click(screen.getByRole('menuitem', { name: 'Choose app...' }));
  await waitFor(() => expect(ledger.filter(({ command }) => command === 'open_stack_item_with_picker')).toEqual([
    { command: 'open_stack_item_with_picker', args: { path: `${child}\\draft.txt` } }
  ]));
});

it('ArrowRight enters submenu; ArrowLeft returns focus; Escape closes only submenu first', async () => {
  overrides.set('list_stack_open_with_candidates', () => [candidate('editor')]);
  await mountFolder();
  const trigger = await menu();
  await screen.findByText('Associated editor');
  trigger.focus();
  await fireEvent.keyDown(trigger, { key: 'ArrowRight' });
  await waitFor(() => expect(document.activeElement?.textContent).toMatch(/Associated editor|Choose app/));
  await fireEvent.keyDown(document.activeElement!, { key: 'ArrowLeft' });
  await waitFor(() => expect(document.activeElement).toBe(trigger));
  expect(trigger.getAttribute('aria-expanded')).toBe('false');
  await fireEvent.keyDown(trigger, { key: 'ArrowRight' });
  await fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
  expect(screen.getByRole('menuitem', { name: /Open with/ })).toBe(trigger);
  expect(trigger.getAttribute('aria-expanded')).toBe('false');
});

it('same-file reopen excludes older out-of-order response', async () => {
  const first = deferred<ReturnType<typeof candidate>[]>();
  const second = deferred<ReturnType<typeof candidate>[]>();
  let calls = 0;
  overrides.set('list_stack_open_with_candidates', () => ++calls === 1 ? first.promise : second.promise);
  await mountFolder();
  const trigger = await menu();
  await fireEvent.keyDown(trigger, { key: 'Escape' });
  await menu();
  second.resolve([candidate('current')]);
  await screen.findByText('Associated current');
  first.resolve([candidate('retired')]);
  await new Promise((resolve) => setTimeout(resolve, 0));
  await tick();
  expect(screen.queryByText('Associated retired')).toBeNull();
  expect(screen.getByText('Associated current')).toBeTruthy();
});

it('hover-open Escape keeps parent menu when folder grid retains focus; second Escape dismisses parent', async () => {
  await mountFolder();
  const trigger = await menu();
  const grid = screen.getByRole('grid', { name: 'Folder details' });
  grid.focus();
  await fireEvent.pointerEnter(trigger.closest('.context-submenu')!);
  await waitFor(() => expect(trigger.getAttribute('aria-expanded')).toBe('true'));
  expect(document.activeElement).toBe(grid);
  await fireEvent.keyDown(grid, { key: 'Escape' });
  expect(screen.getByRole('menuitem', { name: /Open with/ })).toBe(trigger);
  expect(trigger.getAttribute('aria-expanded')).toBe('false');
  await fireEvent.keyDown(grid, { key: 'Escape' });
  expect(screen.queryByRole('menuitem', { name: /Open with/ })).toBeNull();
});

it('duplicate activation while launch pending sends exactly one captured-file request', async () => {
  const pending = deferred<void>();
  overrides.set('list_stack_open_with_candidates', () => [candidate('editor')]);
  overrides.set('open_stack_item_with_app', () => pending.promise);
  await mountFolder();
  await openSubmenu();
  const app = await screen.findByText('Associated editor');
  await fireEvent.click(app);
  await menu('readme.md');
  await fireEvent.click(await screen.findByText('Associated editor'));
  expect(ledger.filter(({ command }) => command === 'open_stack_item_with_app')).toEqual([
    { command: 'open_stack_item_with_app', args: { path: `${child}\\draft.txt`, appId: 'editor' } }
  ]);
  pending.resolve();
});

it('launch failure remains visible and does not fall back to another app or chooser', async () => {
  overrides.set('list_stack_open_with_candidates', () => [candidate('editor')]);
  overrides.set('open_stack_item_with_app', () => Promise.reject(new Error('Handler no longer registered')));
  await mountFolder();
  await openSubmenu();
  await fireEvent.click(await screen.findByText('Associated editor'));
  await screen.findByText(/Handler no longer registered/);
  expect(ledger.filter(({ command }) => command === 'open_stack_item_with_app')).toHaveLength(1);
  expect(ledger.some(({ command }) => command === 'open_stack_item_with_picker' || command === 'open_stack_item')).toBe(false);
});
