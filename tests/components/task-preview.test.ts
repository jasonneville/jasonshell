import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, expect, it, vi } from 'vitest';

// DOM/IPC evidence only. No native shell, HWND dispatch, or visual rendering claims.
const bridge = vi.hoisted(() => ({
  handlers: new Map<string, (event: { payload: unknown }) => unknown>(),
  invoke: vi.fn(), emit: vi.fn()
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: bridge.invoke }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, handler: (event: { payload: unknown }) => unknown) => {
    bridge.handlers.set(name, handler);
    return () => bridge.handlers.delete(name);
  },
  emit: bridge.emit, emitTo: bridge.emit
}));

import TaskPreviewSurface from '../../src/components/TaskPreviewSurface.svelte';
import type { TaskPreviewPayload } from '../../src/lib/taskbarPreview';
import { TASK_PREVIEW_HIDE_REQUEST_EVENT, TASK_PREVIEW_HOVER_ENTER_EVENT, TASKBAR_REFRESH_WINDOWS_EVENT } from '../../src/lib/taskbarUi';

const base: TaskPreviewPayload = {
  hwnd: '1234', title: 'JasonShell — Editor', processName: 'editor.exe',
  iconDataUrl: '', isMinimized: false,
  previewSource: 'captured-image', imageDataUrl: 'data:image/png;base64,capture'
};
async function publish(overrides: Partial<TaskPreviewPayload> = {}) {
  await bridge.handlers.get('task-preview:update')?.({ payload: { ...base, ...overrides } });
  await tick();
}
async function open(overrides: Partial<TaskPreviewPayload> = {}) {
  const view = render(TaskPreviewSurface);
  await waitFor(() => expect(bridge.handlers.has('task-preview:update')).toBe(true));
  await publish(overrides);
  return view;
}
function calls(command: string) { return bridge.invoke.mock.calls.filter(([name]) => name === command); }
function activation() { return screen.getByRole('button', { name: /^Activate / }); }
beforeEach(() => {
  bridge.handlers.clear(); bridge.invoke.mockReset(); bridge.emit.mockReset();
  bridge.invoke.mockResolvedValue(undefined); bridge.emit.mockResolvedValue(undefined);
});

it('renders title/process hierarchy, process fallback, and full long labels for CSS truncation', async () => {
  const { container } = await open();
  expect(container.querySelector('.preview-title')?.textContent).toBe(base.title);
  expect(container.querySelector('.preview-process')?.textContent).toBe(base.processName);
  await publish({ title: '' });
  expect(container.querySelector('.preview-title')?.textContent).toBe(base.processName);
  expect(container.querySelector('.preview-process')).toBeNull();
  expect(activation().getAttribute('aria-label')).toBe(`Activate ${base.processName}`);
  await publish({ title: base.processName });
  expect(container.querySelector('.preview-process')).toBeNull();
  const title = 'Very long document title '.repeat(30);
  const processName = 'long-process-name'.repeat(20);
  await publish({ title, processName });
  expect(container.querySelector('.preview-title')?.textContent).toBe(title);
  expect(container.querySelector('.preview-process')?.textContent).toBe(processName);
  expect(activation().getAttribute('aria-label')).toBe(`Activate ${title}`);
  expect(screen.getByRole('button', { name: 'Close previewed window' }).parentElement).toBe(activation().parentElement);
});

it.each([
  { previewSource: 'native-dwm-thumbnail' as const, nativeLiveThumbnailActive: false },
  { previewSource: 'captured-image' as const, nativeLiveThumbnailActive: true }
])('native source/flag takes precedence over capture and error: %j', async (native) => {
  const { container } = await open({ ...native, error: 'Capture failed' });
  expect(activation().classList.contains('preview-surface-native')).toBe(true);
  expect(container.querySelector('.preview-frame-native')?.getAttribute('aria-hidden')).toBe('true');
  expect(container.querySelector('.preview-image')).toBeNull();
  expect(container.querySelector('.preview-empty')).toBeNull();
  await publish();
  expect(activation().classList.contains('preview-surface-native')).toBe(false);
  expect(screen.getByRole('img', { name: `Preview of ${base.title}` }).getAttribute('src')).toBe(base.imageDataUrl);
});

it('unavailable payload keeps actions and error/fallback text; native hide clears and disables them', async () => {
  const { container } = await open({ previewSource: 'unavailable', imageDataUrl: null, error: 'Capture denied' });
  expect(container.querySelector('.preview-empty')?.textContent).toContain('Capture denied');
  expect(activation().getAttribute('aria-disabled')).toBe('false');
  await publish({ previewSource: 'unavailable', imageDataUrl: null });
  expect(container.querySelector('.preview-empty')?.textContent).toContain('Preview unavailable');
  await bridge.handlers.get('task-preview:hide')?.({ payload: null });
  await tick();
  const unavailable = screen.getByRole('button', { name: 'Task preview unavailable' });
  expect(unavailable.getAttribute('aria-disabled')).toBe('true');
  expect(screen.queryByRole('button', { name: 'Close previewed window' })).toBeNull();
  await fireEvent.click(unavailable);
  await fireEvent.keyDown(unavailable, { key: 'Enter' });
  expect(bridge.invoke).not.toHaveBeenCalled();
});

it.each(['click', 'Enter', ' '] as const)('%s activates exact HWND then refreshes and hides preview', async (gesture) => {
  await open();
  if (gesture === 'click') await fireEvent.click(activation());
  else await fireEvent.keyDown(activation(), { key: gesture });
  await waitFor(() => expect(calls('maximize_task_window')).toEqual([['maximize_task_window', { hwnd: base.hwnd }]]));
  await waitFor(() => expect(bridge.emit.mock.calls).toEqual([
    [TASKBAR_REFRESH_WINDOWS_EVENT],
    [TASK_PREVIEW_HIDE_REQUEST_EVENT, { mode: 'immediate', preserveGallery: false }]
  ]));
  expect(screen.queryByRole('button', { name: /^Activate / })).toBeNull();
  expect(calls('close_task_window')).toHaveLength(0);
});

it.each([null, 'gallery-session'] as const)('close preserves gallery only for nonce %s and never activates', async (galleryNonce) => {
  await open({ galleryNonce });
  const close = screen.getByRole('button', { name: 'Close previewed window' });
  expect(activation().contains(close)).toBe(false);
  await fireEvent.click(close);
  const command = galleryNonce ? 'close_task_gallery_previewed_window' : 'close_task_window';
  await waitFor(() => expect(bridge.invoke.mock.calls).toEqual([
    [command, galleryNonce ? { args: { nonce: galleryNonce, hwnd: base.hwnd } } : { hwnd: base.hwnd }]
  ]));
  await waitFor(() => expect(bridge.emit.mock.calls).toEqual([
    [TASKBAR_REFRESH_WINDOWS_EVENT],
    [TASK_PREVIEW_HIDE_REQUEST_EVENT, { mode: 'immediate', preserveGallery: Boolean(galleryNonce) }]
  ]));
  expect(calls('maximize_task_window')).toHaveLength(0);
  expect(calls('hide_task_gallery')).toHaveLength(0);
});

it('pointer entry retains hover; moving to sibling close does not schedule hide, leaving root does', async () => {
  await open();
  const root = screen.getByRole('group', { name: 'Task preview' });
  await fireEvent.pointerEnter(root);
  expect(bridge.emit).toHaveBeenCalledWith(TASK_PREVIEW_HOVER_ENTER_EVENT, { source: 'preview' });
  bridge.emit.mockClear();
  // jsdom lacks PointerEvent; a MouseEvent with the same event name preserves relatedTarget.
  fireEvent(root, new MouseEvent('pointerleave', { relatedTarget: screen.getByRole('button', { name: 'Close previewed window' }) }));
  await tick();
  expect(bridge.emit).not.toHaveBeenCalled();
  fireEvent(root, new MouseEvent('pointerleave', { relatedTarget: document.body }));
  await waitFor(() => expect(bridge.emit).toHaveBeenCalledWith(TASK_PREVIEW_HIDE_REQUEST_EVENT, { mode: 'schedule', preserveGallery: false }));
  expect(activation()).toBeTruthy();
});

it.each(['activate', 'close'] as const)('%s rejection keeps preview actionable without refresh or hide', async (action) => {
  await open({ galleryNonce: 'gallery-session' });
  bridge.invoke.mockRejectedValue(new Error('Denied'));
  const errors = vi.spyOn(console, 'error').mockImplementation(() => {});
  await fireEvent.click(action === 'activate' ? activation() : screen.getByRole('button', { name: 'Close previewed window' }));
  await waitFor(() => expect(errors).toHaveBeenCalledOnce());
  expect(bridge.emit).not.toHaveBeenCalled();
  expect(activation()).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Close previewed window' })).toBeTruthy();
});

it('renders decorative application icon, omits blank/broken icons, and resets on replacement payload', async () => {
  const iconDataUrl = 'data:image/png;base64,app-icon';
  const { container } = await open({ iconDataUrl });
  const icon = container.querySelector<HTMLImageElement>('.preview-header img');
  expect(icon).not.toBeNull();
  expect(icon?.getAttribute('src')).toBe(iconDataUrl);
  expect(icon?.getAttribute('alt')).toBe('');
  expect(screen.queryByRole('img', { name: /app-icon/i })).toBeNull();
  await fireEvent.error(icon!);
  expect(container.querySelector('.preview-header img')).toBeNull();
  await publish({ iconDataUrl: 'data:image/png;base64,replacement' });
  expect(container.querySelector('.preview-header img')?.getAttribute('src')).toBe('data:image/png;base64,replacement');
  await publish({ iconDataUrl: '   ' });
  expect(container.querySelector('.preview-header img')).toBeNull();
  await publish({ iconDataUrl: '' });
  expect(container.querySelector('.preview-header img')).toBeNull();
});

it('ignores unrelated keys and removes update/hide subscriptions on unmount', async () => {
  const view = await open();
  await fireEvent.keyDown(activation(), { key: 'Escape' });
  expect(bridge.invoke).not.toHaveBeenCalled();
  expect(bridge.emit).not.toHaveBeenCalled();
  view.unmount();
  expect(bridge.handlers.size).toBe(0);
});
