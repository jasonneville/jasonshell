import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { root, other, child, resetBridge, overrides, summary, mutate, notify, page } from '../browser/stack-external-refresh/bridge';
vi.mock('@tauri-apps/api/core', async () => import('../browser/stack-external-refresh/bridge'));
vi.mock('@tauri-apps/api/event', async () => import('../browser/stack-external-refresh/bridge'));
vi.mock('@tauri-apps/api/window', async () => import('../browser/stack-external-refresh/bridge'));
import StackPopupSurface from '../../src/components/StackPopupSurface.svelte';

// FE-25 mounted-surface acceptance: shared date formatter, folder roll-up badges through the
// precomputed index, badges that follow status changes and clear on retired identity.
const rowOf = (name: string) => screen.getByText(name).closest('[data-stack-entry-path]') as HTMLElement;
const settle = () => new Promise((resolve) => setTimeout(resolve, 60));
const gitItem = (path: string, status: string) => ({ path, relativePath: path, status, staged: false, unstaged: true });

beforeEach(() => { resetBridge(); vi.stubGlobal('ResizeObserver', class { observe() {} unobserve() {} disconnect() {} }); });
afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks(); });

it('constructs at most one Intl.DateTimeFormat for a many-row folder across renders and a Git-driven re-render', async () => {
  overrides.set('read_stack_folder', ({ path }) => {
    const base = page(String(path));
    return { ...base, items: Array.from({ length: 60 }, (_, i) => ({ ...base.items[0], path: `${path}\\f${String(i).padStart(2, '0')}.txt`, name: `f${String(i).padStart(2, '0')}.txt`, kind: 'file', modifiedAt: 1_700_000_000_000 + i * 3_600_000 })) };
  });
  const real = Intl.DateTimeFormat; let constructed = 0;
  vi.spyOn(Intl, 'DateTimeFormat').mockImplementation(function (this: unknown, ...args: ConstructorParameters<typeof real>) { constructed += 1; return new real(...args); } as never);
  render(StackPopupSurface); await screen.findByText('f00.txt'); await settle();
  expect(rowOf('f00.txt').textContent).toMatch(/\d/); // date cell really rendered
  mutate(1); await notify(); await waitFor(() => expect(rowOf('f00.txt').getAttribute('data-git-status')).toBeNull()); await settle();
  expect(constructed, 'date formatter must be created once, not per row/render').toBeLessThanOrEqual(1);
});

it('folder rows roll up nested status by priority through case/slash variants and update when status changes', async () => {
  let entries = [gitItem('C:/EXTERNAL-FIXTURE/REPO/SRC/a.ts', 'modified'), gitItem(`${root}\\src\\b.ts`, 'conflict'), gitItem(`${root}\\draft.txt`, 'untracked')];
  overrides.set('get_stack_git_summary', ({ path }) => ({ ...summary(String(path)), entries }));
  render(StackPopupSurface); await screen.findByText('src'); await settle();
  mutate(1); await notify();
  await waitFor(() => expect(rowOf('src').getAttribute('data-git-status')).toBe('conflict'));
  expect(rowOf('draft.txt').getAttribute('data-git-status')).toBe('untracked');
  expect(rowOf('readme.md').getAttribute('data-git-status')).toBeNull();
  entries = [gitItem(`${root}\\src\\a.ts`, 'modified')];
  mutate(2); await notify();
  await waitFor(() => expect(rowOf('src').getAttribute('data-git-status')).toBe('modified'));
  expect(rowOf('draft.txt').getAttribute('data-git-status')).toBeNull();
  entries = [];
  mutate(3); await notify();
  await waitFor(() => expect(rowOf('src').getAttribute('data-git-status')).toBeNull());
  expect(document.querySelectorAll('.git-status-badge')).toHaveLength(0);
});

it('badges are retired when the folder identity changes and do not leak into the next folder', async () => {
  overrides.set('get_stack_git_summary', ({ path }) => ({ ...summary(String(path)), entries: [gitItem(`${root}\\draft.txt`, 'modified'), gitItem(`${child}\\draft.txt`, 'deleted')] }));
  render(StackPopupSurface); await screen.findByText('draft.txt'); await settle();
  mutate(1); await notify();
  await waitFor(() => expect(rowOf('draft.txt').getAttribute('data-git-status')).toBe('modified'));
  const input = screen.getByRole('textbox', { name: 'Current folder path' });
  await fireEvent.input(input, { target: { value: other } }); await fireEvent.submit(input.closest('form')!);
  await waitFor(() => expect(document.querySelector('.stack-path')?.getAttribute('title')).toBe(other)); await settle();
  expect(document.querySelectorAll('[data-git-status]')).toHaveLength(0);
  expect(document.querySelectorAll('.git-status-badge')).toHaveLength(0);
});

it('selection and attribute labels render correctly for rows (Set-based selection, shared labels)', async () => {
  overrides.set('read_stack_folder', ({ path }) => {
    const base = page(String(path));
    return { ...base, items: base.items.map((it, i) => ({ ...it, isHidden: i === 1, isSystem: i === 1, isSymlink: i === 2 })) };
  });
  render(StackPopupSurface); await screen.findByText('draft.txt'); await settle();
  const hidden = rowOf('draft.txt');
  expect(hidden.querySelector('.item-badges')?.getAttribute('aria-label')).toBe('Hidden, System');
  expect(rowOf('readme.md').querySelector('.item-badges')?.getAttribute('aria-label')).toBe('Link');
  expect(rowOf('src').querySelector('.item-badges')).toBeNull();
  await fireEvent.click(hidden);
  expect(hidden.getAttribute('aria-selected')).toBe('true');
  await fireEvent.click(rowOf('readme.md'), { ctrlKey: true });
  expect(hidden.getAttribute('aria-selected')).toBe('true');
  expect(rowOf('readme.md').getAttribute('aria-selected')).toBe('true');
  expect(rowOf('src').getAttribute('aria-selected')).toBe('false');
});
