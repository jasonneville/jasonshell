import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { expect, it, vi } from 'vitest';
import QuickCommandNotificationSurface from '../../src/components/QuickCommandNotificationSurface.svelte';
import { tauriMocks } from './setup';

it.each([['success', 'Succeeded'], ['failure', 'Failed']])('renders a trailing verdict for %s', async (status, verdict) => {
  tauriMocks().invoke.mockResolvedValue({ id: 'release', label: 'Build release', status });
  const view = render(QuickCommandNotificationSurface);
  const open = await view.findByRole('button', { name: new RegExp(`Open Build release output.*${verdict}`, 'i') });
  const dismiss = view.getByRole('button', { name: 'Dismiss notification' });
  expect(open.textContent?.trim().replace(/\s+/g, ' ')).toBe(`Build release ${verdict}`);
  const icon = open.querySelector('svg[aria-hidden="true"]');
  expect(icon).not.toBeNull();
  expect(view.getByText(verdict, { exact: true }).compareDocumentPosition(icon!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(open.contains(dismiss)).toBe(false);
  expect(dismiss.querySelector('svg[aria-hidden="true"] circle')).not.toBeNull();
});

it('registers a private channel, acknowledges a rendered banner, and ignores stale reads after dismissal', async () => {
  const tauri = tauriMocks();
  let channel: { onmessage: (payload: null) => void } | undefined;
  const acknowledgements: Array<{ id: string; domReady: boolean }> = [];
  const banner = { id: 'opaque', label: 'Build', status: 'failure' };
  let current: typeof banner | null = banner;
  let staleRead: Promise<typeof banner> | null = null;
  const actions: unknown[] = [];
  tauri.invoke.mockImplementation(async (command, args) => {
    if (command === 'get_quick_command_notification') {
      if (args?.channel) channel = args.channel;
      if (args?.renderedId) acknowledgements.push({ id: args.renderedId, domReady: !!document.querySelector('button[aria-label*="Open Build output"]') });
      return staleRead ?? current;
    }
    if (command === 'act_on_quick_command_notification') { actions.push(args); current = null; }
  });

  const view = render(QuickCommandNotificationSurface);
  await waitFor(() => expect(channel?.onmessage).toBeTypeOf('function'));
  expect(await view.findByRole('button', { name: /open build output.*failed/i })).toBeTruthy();
  await waitFor(() => expect(acknowledgements).toContainEqual({ id: 'opaque', domReady: true }));
  expect(tauri.listen).not.toHaveBeenCalled();
  let releaseStaleRead!: (value: typeof banner) => void;
  staleRead = new Promise(resolve => { releaseStaleRead = resolve; });
  channel!.onmessage(null);
  staleRead = null;
  await fireEvent.click(view.getByRole('button', { name: /dismiss notification/i }));
  await waitFor(() => expect(view.queryByRole('button', { name: /dismiss notification/i })).toBeNull());
  expect(actions).toEqual([{ id: 'opaque', action: 'dismiss' }]);
  releaseStaleRead(banner);
  await Promise.resolve();
  await tick();
  expect(view.queryByRole('button', { name: /open build output/i })).toBeNull();
});

it.each([false, true])('handles a delayed ACK response after a newer channel pull: %s', async (superseded) => {
  const first = { id: 'a', label: 'Build A', status: 'success' };
  const replacement = { id: 'b', label: 'Build B', status: 'success' };
  const latest = { id: 'c', label: 'Build C', status: 'failure' };
  let channel!: { onmessage: (payload: null) => void };
  let finishAck!: (banner: typeof first) => void;
  const pendingAck = new Promise<typeof first>(resolve => { finishAck = resolve; });
  const acknowledged: string[] = [];
  tauriMocks().invoke.mockImplementation(async (command, args) => {
    if (command !== 'get_quick_command_notification') return;
    if (args?.channel) { channel = args.channel; return first; }
    if (args?.renderedId) {
      acknowledged.push(args.renderedId);
      if (args.renderedId === first.id) return pendingAck;
      return args.renderedId === latest.id ? latest : replacement;
    }
    return latest;
  });
  const view = render(QuickCommandNotificationSurface);
  await waitFor(() => expect(acknowledged).toEqual([first.id]));
  if (superseded) {
    channel.onmessage(null);
    await waitFor(() => expect(acknowledged).toEqual([first.id, latest.id]));
  }
  finishAck(replacement); // Native marks B pulled in this reply and sends no wake for it.
  const expected = superseded ? latest : replacement;
  await view.findByRole('button', { name: new RegExp(`open ${expected.label} output`, 'i') });
  await new Promise(resolve => setTimeout(resolve, 0));
  expect(acknowledged).toEqual([first.id, expected.id]);
  expect(view.queryByRole('button', { name: /open build a output/i })).toBeNull();
  if (superseded) expect(view.queryByRole('button', { name: /open build b output/i })).toBeNull();
});
