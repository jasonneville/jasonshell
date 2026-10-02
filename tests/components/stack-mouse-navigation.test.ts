import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import { child, fixtureRowHeight, handlers, installLargeDirectoryFixture, largeDirectoryCount, largeFileName, ledger, overrides, resetBridge, root } from '../browser/stack-mouse/bridge';

vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-mouse/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-mouse/bridge'));
// Keep the actual editor surface/dirty guard; substitute only CodeMirror's DOM adapter in jsdom.
vi.mock('../../src/features/stack-browser/stackTextEditorAdapter', () => ({
  createStackTextEditorAdapter: ({ parent, content, onChange }: { parent: HTMLElement; content: string; onChange: (draft: string) => void }) => {
    const input = document.createElement('textarea');
    input.setAttribute('aria-label', 'Fixture draft');
    input.value = content;
    input.oninput = () => onChange(input.value);
    parent.append(input);
    return { destroy: () => input.remove(), focusAtStart: () => input.focus(), setFont: () => {} };
  }
}));

import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';

beforeEach(() => { resetBridge(); vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} }); });
afterEach(() => vi.unstubAllGlobals());

function sideEvent(target: Element | Window, button: number, type = 'pointerdown', pointerType = 'mouse') {
  const event = new MouseEvent(type, { button, buttons: type === 'pointerdown' || type === 'mousedown' ? (button === 3 ? 8 : button === 4 ? 16 : 0) : 0, bubbles: true, cancelable: true });
  Object.defineProperty(event, 'pointerType', { value: pointerType });
  Object.defineProperty(event, 'pointerId', { value: 1 });
  target.dispatchEvent(event);
  return event;
}
async function openChild() {
  const view = render(StackPopupSurface);
  await fireEvent.dblClick(await screen.findByText('repo'));
  await screen.findByText('draft.txt');
  await waitFor(() => expect((screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(child));
  return view;
}
function snapshotFolder() {
  return {
    path: (screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value,
    back: (screen.getByRole('button', { name: 'Back', exact: true }) as HTMLButtonElement).disabled,
    forward: (screen.getByRole('button', { name: 'Forward', exact: true }) as HTMLButtonElement).disabled,
    selected: [...document.querySelectorAll('[data-stack-entry-path][aria-selected="true"]')].map((row) => row.getAttribute('data-stack-entry-path')),
    reads: ledger.filter(({ command }) => command === 'read_stack_folder').length
  };
}

it('Back closes clean editor at pointerdown, preserving folder/history/selection, with compatibility events inert', async () => {
  await openChild();
  await fireEvent.click(screen.getByText('draft.txt'));
  const folder = snapshotFolder();
  await fireEvent.dblClick(screen.getByText('draft.txt'));
  const draft = await screen.findByRole('textbox', { name: 'Fixture draft' });
  const event = sideEvent(draft, 3);
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Close editor and return to folder' })).toBeNull());
  expect(event.defaultPrevented).toBe(true);
  sideEvent(screen.getByRole('region', { name: 'Stack browser' }), 3, 'mousedown');
  sideEvent(screen.getByRole('region', { name: 'Stack browser' }), 3, 'auxclick');
  expect(snapshotFolder()).toEqual(folder);
  expect(ledger.some(({ command }) => /hide|save|delete|revert/.test(command))).toBe(false);
});

it('dirty Back owns close-editor-only; Forward inert, Cancel retains draft, Discard never navigates/hides', async () => {
  await openChild();
  await fireEvent.dblClick(screen.getByText('draft.txt'));
  const draft = await screen.findByRole('textbox', { name: 'Fixture draft' });
  await fireEvent.input(draft, { target: { value: 'unsaved draft' } });
  const folder = snapshotFolder();
  expect(sideEvent(draft, 4).defaultPrevented).toBe(true);
  sideEvent(draft, 3);
  await screen.findByRole('alertdialog');
  sideEvent(window, 3, 'mousedown'); sideEvent(window, 3, 'auxclick');
  expect(screen.getAllByRole('alertdialog')).toHaveLength(1);
  expect(sideEvent(screen.getByRole('alertdialog'), 4).defaultPrevented).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect((screen.getByRole('textbox', { name: 'Fixture draft' }) as HTMLTextAreaElement).value).toBe('unsaved draft');
  sideEvent(draft, 3);
  await screen.findByRole('alertdialog');
  await fireEvent.click(screen.getByRole('button', { name: 'Discard' }));
  await screen.findByRole('grid', { name: 'Folder details' });
  expect(snapshotFolder()).toEqual(folder);
  expect(ledger.some(({ command }) => /hide|save|delete|revert/.test(command))).toBe(false);
});

it.each(['loading', 'error', 'markdown'])('Back closes %s editor without folder navigation', async (mode) => {
  await openChild();
  if (mode === 'loading') overrides.set('read_stack_basic_text_file', () => new Promise(() => {}));
  if (mode === 'error') overrides.set('read_stack_basic_text_file', () => Promise.reject(new Error('Fixture read error')));
  await fireEvent.dblClick(screen.getByText(mode === 'markdown' ? 'readme.md' : 'draft.txt'));
  const close = await screen.findByRole('button', { name: 'Close editor and return to folder' });
  if (mode === 'error') await screen.findByText('Fixture read error');
  if (mode === 'markdown') await screen.findByRole('button', { name: 'Preview', exact: true });
  const folder = snapshotFolder();
  expect(sideEvent(close, 3).defaultPrevented).toBe(true);
  await screen.findByRole('grid', { name: 'Folder details' });
  expect(snapshotFolder()).toEqual(folder);
});

it('Back cancels inline folder input even when its mousedown stops bubbling', async () => {
  await openChild();
  await fireEvent.click(screen.getByRole('button', { name: 'New folder', exact: true }));
  const input = screen.getByRole('textbox', { name: 'New folder name' });
  await fireEvent.input(input, { target: { value: 'never create' } });
  const folder = snapshotFolder();
  expect(sideEvent(input, 4).defaultPrevented).toBe(true);
  expect(screen.getByRole('textbox', { name: 'New folder name' })).toBe(input);
  sideEvent(input, 3);
  await waitFor(() => expect(screen.queryByRole('textbox', { name: 'New folder name' })).toBeNull());
  expect(snapshotFolder()).toEqual(folder);
  expect(ledger.some(({ command }) => command === 'new_stack_folder')).toBe(false);
});

it('bare folder Back/Forward visits history once per physical press; boundaries inert', async () => {
  await openChild();
  expect(sideEvent(screen.getByRole('grid'), 3).defaultPrevented).toBe(true);
  await waitFor(() => expect((screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(root));
  sideEvent(window, 3, 'mousedown'); sideEvent(window, 3, 'auxclick');
  await screen.findByText('repo');
  expect((screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(root);
  const reads = ledger.filter(({ command }) => command === 'read_stack_folder').length;
  sideEvent(screen.getByRole('grid'), 3);
  expect(ledger.filter(({ command }) => command === 'read_stack_folder')).toHaveLength(reads);
  expect(sideEvent(screen.getByRole('grid'), 4).defaultPrevented).toBe(true);
  sideEvent(window, 4, 'mousedown'); sideEvent(window, 4, 'auxclick');
  await screen.findByText('draft.txt');
  expect(ledger.filter(({ command }) => command === 'read_stack_folder')).toHaveLength(reads + 1);
});

it('folder context menu Back dismisses menu before canceling underlying inline input', async () => {
  await openChild();
  await fireEvent.click(screen.getByRole('button', { name: 'New folder', exact: true }));
  await fireEvent.contextMenu(screen.getByRole('region', { name: 'Stack browser' }));
  await screen.findByRole('menu');
  const folder = snapshotFolder();
  expect(sideEvent(screen.getByRole('menu'), 4).defaultPrevented).toBe(true);
  expect(screen.getByRole('menu')).toBeTruthy();
  sideEvent(screen.getByRole('menu'), 3);
  await waitFor(() => expect(screen.queryByRole('menu')).toBeNull());
  expect(screen.getByRole('textbox', { name: 'New folder name' })).toBeTruthy();
  expect(snapshotFolder()).toEqual(folder);
});

it('ordinary buttons/non-mouse pointers remain unowned; unmount removes navigation listener', async () => {
  const view = await openChild();
  const folder = snapshotFolder();
  for (const button of [0, 1, 2]) expect(sideEvent(screen.getByRole('grid'), button).defaultPrevented).toBe(false);
  expect(sideEvent(screen.getByRole('grid'), 3, 'pointerdown', 'pen').defaultPrevented).toBe(false);
  expect(sideEvent(screen.getByRole('grid'), 3, 'pointerdown', 'touch').defaultPrevented).toBe(false);
  expect(snapshotFolder()).toEqual(folder);
  view.unmount();
  expect(sideEvent(window, 3).defaultPrevented).toBe(false);
});

it('fixture control: actual editor dirty guard works through existing ordinary close/Cancel/Discard buttons', async () => {
  await openChild();
  await fireEvent.dblClick(screen.getByText('draft.txt'));
  await fireEvent.input(await screen.findByRole('textbox', { name: 'Fixture draft' }), { target: { value: 'control dirty draft' } });
  const folder = snapshotFolder();
  await fireEvent.click(screen.getByRole('button', { name: 'Close editor and return to folder' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Cancel', exact: true }));
  expect((screen.getByRole('textbox', { name: 'Fixture draft' }) as HTMLTextAreaElement).value).toBe('control dirty draft');
  await fireEvent.click(screen.getByRole('button', { name: 'Close editor and return to folder' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Discard', exact: true }));
  await screen.findByRole('grid', { name: 'Folder details' });
  expect(snapshotFolder()).toEqual(folder);
});

it('rapid dirty Back/compatibility sequences prompt, cancel, reprompt without losing draft; Discard restores folder focus', async () => {
  await openChild();
  await fireEvent.dblClick(screen.getByText('draft.txt'));
  const draft = await screen.findByRole('textbox', { name: 'Fixture draft' });
  await fireEvent.input(draft, { target: { value: 'rapid draft' } });
  draft.focus();
  const popup = screen.getByRole('region', { name: 'Stack browser' });
  const folder = snapshotFolder();
  for (const expectedPrompt of [true, false, true]) {
    expect(sideEvent(popup, 3).defaultPrevented).toBe(true);
    await tick();
    expect(sideEvent(popup, 3, 'mousedown').defaultPrevented).toBe(true);
    expect(sideEvent(popup, 3, 'auxclick').defaultPrevented).toBe(true);
    await tick();
    expect(Boolean(screen.queryByRole('alertdialog'))).toBe(expectedPrompt);
    expect((screen.getByRole('textbox', { name: 'Fixture draft' }) as HTMLTextAreaElement).value).toBe('rapid draft');
    expect(snapshotFolder()).toEqual(folder);
    if (!expectedPrompt) await waitFor(() => expect(document.activeElement).toBe(draft));
  }
  await fireEvent.click(screen.getByRole('button', { name: 'Discard' }));
  await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('grid', { name: 'Folder details' })));
  expect(snapshotFolder()).toEqual(folder);
  expect(ledger.filter(({ command }) => /hide|save|delete|revert/.test(command))).toEqual([]);
});

it('folder delete confirmation Back cancels only modal, restores existing grid focus, and issues zero destructive calls', async () => {
  await openChild();
  const row = screen.getByText('draft.txt').closest<HTMLElement>('[role="row"]')!;
  await fireEvent.click(row);
  row.focus();
  const folder = snapshotFolder();
  await fireEvent.click(screen.getByRole('button', { name: 'Delete selected item' }));
  const dialog = await screen.findByRole('alertdialog');
  await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Cancel', exact: true })));
  expect(sideEvent(dialog, 4).defaultPrevented).toBe(true);
  expect(screen.getByRole('alertdialog')).toBe(dialog);
  expect(sideEvent(dialog, 3).defaultPrevented).toBe(true);
  await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
  await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('grid', { name: 'Folder details' })));
  expect(snapshotFolder()).toEqual(folder);
  expect(ledger.filter(({ command }) => /hide|delete|revert/.test(command))).toEqual([]);
});

it('ordinary click selects rows; folder keyboard still selects/navigates and editor keys do not navigate underlying history', async () => {
  await openChild();
  const row = screen.getByText('draft.txt').closest<HTMLElement>('[role="row"]')!;
  expect(sideEvent(row, 0).defaultPrevented).toBe(false);
  await fireEvent.click(row);
  expect(row.getAttribute('aria-selected')).toBe('true');
  const reads = snapshotFolder().reads;
  await fireEvent.keyDown(screen.getByRole('grid'), { key: 'ArrowDown' });
  expect(screen.getByText('readme.md').closest('[role="row"]')?.getAttribute('aria-selected')).toBe('true');
  expect(snapshotFolder().reads).toBe(reads);
  await fireEvent.dblClick(row);
  const draft = await screen.findByRole('textbox', { name: 'Fixture draft' });
  const folder = snapshotFolder();
  for (const key of ['Backspace', 'ArrowUp', 'ArrowDown']) await fireEvent.keyDown(draft, { key });
  expect(screen.getByRole('textbox', { name: 'Fixture draft' })).toBe(draft);
  expect(snapshotFolder()).toEqual(folder);
  await fireEvent.click(screen.getByRole('button', { name: 'Close editor and return to folder' }));
  await fireEvent.keyDown(screen.getByRole('grid'), { key: 'Backspace' });
  await waitFor(() => expect((screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(root));
});

it('unmount/remount cleans capture handlers, request listeners and reconciliation timer; unmounted popup cannot intercept terminal target', async () => {
  const view = await openChild();
  const clearInterval = vi.spyOn(window, 'clearInterval');
  await waitFor(() => expect(handlers.get('stack-popup:close-requested')?.size).toBe(1));
  view.unmount();
  await tick();
  expect([...handlers.values()].every((subscribers) => subscribers.size === 0)).toBe(true);
  expect(clearInterval).toHaveBeenCalled();
  const terminalTarget = document.createElement('textarea');
  terminalTarget.setAttribute('aria-label', 'Isolated terminal input target');
  document.body.append(terminalTarget);
  const calls = ledger.slice();
  try {
    for (const button of [3, 4]) for (const type of ['pointerdown', 'mousedown', 'auxclick']) expect(sideEvent(terminalTarget, button, type).defaultPrevented).toBe(false);
    expect(ledger).toEqual(calls);
  } finally { terminalTarget.remove(); }
  const remount = await openChild();
  expect(handlers.get('stack-popup:close-requested')?.size).toBe(1);
  await fireEvent.dblClick(screen.getByText('draft.txt'));
  const draft = await screen.findByRole('textbox', { name: 'Fixture draft' });
  expect(sideEvent(draft, 3).defaultPrevented).toBe(true);
  await screen.findByRole('grid');
  expect(snapshotFolder().path).toBe(child);
  remount.unmount();
  await tick();
  expect(sideEvent(window, 3).defaultPrevented).toBe(false);
});

// Explicit geometry makes virtualization deterministic in jsdom; browser fixture proves actual layout separately.
function installFolderGeometry() {
  const originalRect = HTMLElement.prototype.getBoundingClientRect;
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
    return this.matches('.details-body')
      ? { x: 0, y: 100, top: 100, left: 0, right: 600, bottom: 340, width: 600, height: 240, toJSON: () => ({}) }
      : originalRect.call(this);
  });
  const originalHeight = Object.getOwnPropertyDescriptor(Element.prototype, 'clientHeight')!.get!;
  const originalScrollHeight = Object.getOwnPropertyDescriptor(Element.prototype, 'scrollHeight')!.get!;
  vi.spyOn(Element.prototype, 'clientHeight', 'get').mockImplementation(function (this: Element) { return this.matches('.details-body') ? 240 : originalHeight.call(this); });
  vi.spyOn(Element.prototype, 'scrollHeight', 'get').mockImplementation(function (this: Element) { return this.matches('.details-body') ? largeDirectoryCount * fixtureRowHeight : originalScrollHeight.call(this); });
}
function folderBody() { return document.querySelector<HTMLElement>('.details-body')!; }
async function framesSettled() {
  await tick();
  await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
  await tick();
  await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
  await tick();
}
async function openLargeFolder() {
  installLargeDirectoryFixture(); installFolderGeometry();
  const view = render(StackPopupSurface);
  await fireEvent.dblClick(await screen.findByText('repo'));
  await screen.findByRole('grid', { name: 'Folder details' });
  await waitFor(() => expect((screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(child));
  await framesSettled();
  return view;
}
async function scrollToFile(index: number) {
  const offset = index * fixtureRowHeight;
  folderBody().scrollTop = offset;
  await fireEvent.scroll(folderBody());
  const row = screen.getByText(largeFileName(index)).closest<HTMLElement>('[data-stack-entry-path]')!;
  await fireEvent.click(row);
  expect(document.querySelectorAll('[data-stack-entry-path]').length).toBeLessThan(largeDirectoryCount);
  expect(screen.queryByText(largeFileName(0))).toBeNull();
  return { offset, row, selectedPath: `${child}\\${largeFileName(index)}` };
}
async function expectRestoredFolder(offset: number, selectedPath: string, expectGridFocus = true) {
  await screen.findByRole('grid', { name: 'Folder details' });
  await framesSettled();
  expect(folderBody().scrollTop).toBe(offset);
  const row = [...document.querySelectorAll('[data-stack-entry-path]')].find((candidate) => candidate.getAttribute('data-stack-entry-path') === selectedPath);
  expect(row, 'selected file must remain inside the remounted virtual row window').toBeTruthy();
  expect(row!.getAttribute('aria-selected')).toBe('true');
  expect(screen.queryByText(largeFileName(0))).toBeNull();
  if (expectGridFocus) expect(document.activeElement).toBe(screen.getByRole('grid', { name: 'Folder details' }));
  expect(ledger.filter(({ command }) => command === 'read_stack_folder')).toHaveLength(2);
  expect(ledger.filter(({ command }) => /hide|save|delete|revert/.test(command))).toEqual([]);
}

it.each([350, 351])('viewport: clean file %s Back restores same-folder DOM offset and selected virtual row after focus', async (index) => {
  await openLargeFolder();
  const { offset, row, selectedPath } = await scrollToFile(index);
  await fireEvent.dblClick(row);
  const close = await screen.findByRole('button', { name: 'Close editor and return to folder' });
  if (index % 2 === 0) await screen.findByRole('button', { name: 'Preview', exact: true });
  await fireEvent.resize(window);
  await framesSettled();
  // Editor scrolling is unrelated to the folder checkpoint.
  document.querySelector<HTMLElement>('.stack-text-editor-body')!.scrollTop = 96;
  expect(sideEvent(close, 3).defaultPrevented).toBe(true);
  await expectRestoredFolder(offset, selectedPath);
});

it('viewport: dirty Markdown Cancel keeps editor/draft; Discard restores folder offset; next open captures latest offset', async () => {
  await openLargeFolder();
  const first = await scrollToFile(350);
  await fireEvent.dblClick(first.row);
  await fireEvent.click(await screen.findByRole('button', { name: 'Edit', exact: true }));
  const draft = await screen.findByRole('textbox', { name: 'Fixture draft' });
  await fireEvent.input(draft, { target: { value: 'viewport dirty markdown' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Preview', exact: true }));
  sideEvent(screen.getByRole('button', { name: 'Close editor and return to folder' }), 3);
  await fireEvent.click(await screen.findByRole('button', { name: 'Cancel', exact: true }));
  expect(screen.queryByRole('grid')).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Edit', exact: true }));
  expect((screen.getByRole('textbox', { name: 'Fixture draft' }) as HTMLTextAreaElement).value).toBe('viewport dirty markdown');
  sideEvent(screen.getByRole('button', { name: 'Close editor and return to folder' }), 3);
  await fireEvent.click(await screen.findByRole('button', { name: 'Discard', exact: true }));
  await expectRestoredFolder(first.offset, first.selectedPath);
  const second = await scrollToFile(100);
  await fireEvent.dblClick(second.row);
  sideEvent(await screen.findByRole('button', { name: 'Close editor and return to folder' }), 3);
  await expectRestoredFolder(second.offset, second.selectedPath);
});

it.each(['loading', 'error'])('viewport: %s editor exit restores old folder offset without copying editor scroll', async (mode) => {
  await openLargeFolder();
  const { offset, row, selectedPath } = await scrollToFile(350);
  if (mode === 'loading') overrides.set('read_stack_basic_text_file', () => new Promise(() => {}));
  else overrides.set('read_stack_basic_text_file', () => Promise.reject(new Error('Viewport fixture read failure')));
  await fireEvent.dblClick(row);
  const close = await screen.findByRole('button', { name: 'Close editor and return to folder' });
  if (mode === 'error') await screen.findByText('Viewport fixture read failure');
  sideEvent(close, 3);
  await expectRestoredFolder(offset, selectedPath);
});

it('viewport: Git replaces same virtualized grid and Back restores its DOM offset/selection', async () => {
  await openLargeFolder();
  const { offset, selectedPath } = await scrollToFile(350);
  await fireEvent.click(await screen.findByRole('button', { name: 'main', exact: true }));
  const panel = await screen.findByRole('region', { name: 'Git panel' });
  expect(screen.queryByRole('grid')).toBeNull();
  panel.scrollTop = 72;
  sideEvent(panel, 3);
  await expectRestoredFolder(offset, selectedPath, false);
});

it.each(['folder', 'search', 'sort'])('viewport: %s change must not replay an unrelated pre-editor offset', async (change) => {
  await openLargeFolder();
  const { row } = await scrollToFile(350);
  await fireEvent.dblClick(row);
  await screen.findByRole('button', { name: 'Close editor and return to folder' });
  if (change === 'folder') {
    await fireEvent.click(screen.getByRole('button', { name: 'Back', exact: true }));
    await waitFor(() => expect((screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(root));
    await screen.findByText('repo');
  } else if (change === 'search') {
    await fireEvent.input(screen.getByRole('textbox', { name: 'Search current folder' }), { target: { value: largeFileName(10) } });
    sideEvent(screen.getByRole('button', { name: 'Close editor and return to folder' }), 3);
    await screen.findByRole('grid');
  } else {
    // Sort headers are not rendered under the editor. Change sort only through the real remounted grid.
    sideEvent(screen.getByRole('button', { name: 'Close editor and return to folder' }), 3);
    await screen.findByRole('grid'); await framesSettled();
    await fireEvent.click(screen.getByRole('columnheader', { name: /Name/ }));
    folderBody().scrollTop = 48;
    await fireEvent.scroll(folderBody());
    const sortedRow = screen.getByText(largeFileName(398)).closest<HTMLElement>('[data-stack-entry-path]')!;
    await fireEvent.click(sortedRow); await fireEvent.dblClick(sortedRow);
    sideEvent(await screen.findByRole('button', { name: 'Close editor and return to folder' }), 3);
    await expectRestoredFolder(48, `${child}\\${largeFileName(398)}`);
    return;
  }
  await framesSettled();
  expect(folderBody().scrollTop).toBe(0);
  if (change === 'search') expect(screen.getByText(largeFileName(10))).toBeTruthy();
  if (change === 'folder') expect(screen.getByText('repo')).toBeTruthy();
});

it('viewport: deferred toolbar Back abandons old checkpoint and new large folder remains scroll/resize responsive', async () => {
  installLargeDirectoryFixture(); installFolderGeometry();
  const largeListing = overrides.get('read_stack_folder')!;
  type FixturePage = { items: Record<string, unknown>[]; [key: string]: unknown };
  overrides.set('read_stack_folder', async (args) => {
    if (args.path !== root) return largeListing(args);
    const source = await largeListing({ path: child }) as FixturePage;
    const initialRoot = await largeListing({ path: root }) as FixturePage;
    return { ...source, path: root, total: largeDirectoryCount + 1, items: [
      ...initialRoot.items,
      ...source.items.map((item, index) => {
        const name = `destination-${String(index).padStart(4, '0')}.md`;
        return { ...item, name, path: `${root}\\${name}` };
      })
    ] };
  });
  render(StackPopupSurface);
  await fireEvent.dblClick(await screen.findByText('repo'));
  await waitFor(() => expect((screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(child));
  await framesSettled();
  const { row } = await scrollToFile(350);
  await fireEvent.dblClick(row);
  await screen.findByRole('button', { name: 'Preview', exact: true });
  // Existing toolbar navigation dismisses clean editor, remounts grid, then commits another path.
  await fireEvent.click(screen.getByRole('button', { name: 'Back', exact: true }));
  await waitFor(() => expect((screen.getByRole('textbox', { name: 'Current folder path' }) as HTMLInputElement).value).toBe(root));
  await screen.findByRole('grid', { name: 'Folder details' });
  await framesSettled();
  expect(folderBody().scrollTop).toBe(0);
  expect(screen.getByRole('grid').getAttribute('aria-rowcount')).toBe(String(largeDirectoryCount + 2));
  expect([...document.querySelectorAll('[data-stack-entry-path]')].some((entry) => entry.getAttribute('data-stack-entry-path')?.startsWith(`${root}\\destination-`))).toBe(true);
  folderBody().scrollTop = 6000;
  await fireEvent.scroll(folderBody());
  await tick();
  expect(folderBody().scrollTop).toBe(6000);
  expect(screen.queryByText('destination-0200.md'), 'new folder virtualization must follow the new scroll event, not stay frozen behind old checkpoint').toBeTruthy();
  expect(screen.queryByText('destination-0000.md')).toBeNull();
  folderBody().scrollTop = 9000;
  await fireEvent.resize(window);
  await framesSettled();
  expect(screen.getByText('destination-0300.md')).toBeTruthy();
  expect(screen.queryByText('destination-0200.md')).toBeNull();
  // A later checkpoint belongs to the destination, not the abandoned source checkpoint.
  const destinationRow = screen.getByText('destination-0300.md').closest<HTMLElement>('[data-stack-entry-path]')!;
  await fireEvent.click(destinationRow); await fireEvent.dblClick(destinationRow);
  sideEvent(await screen.findByRole('button', { name: 'Close editor and return to folder' }), 3);
  await screen.findByRole('grid'); await framesSettled();
  expect(folderBody().scrollTop).toBe(9000);
  expect(screen.getByText('destination-0300.md').closest('[data-stack-entry-path]')?.getAttribute('aria-selected')).toBe('true');
  expect(ledger.filter(({ command }) => command === 'read_stack_folder')).toHaveLength(3);
  expect(ledger.filter(({ command }) => /hide|save|delete|revert/.test(command))).toEqual([]);
});
