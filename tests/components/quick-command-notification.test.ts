import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import userEvent from '@testing-library/user-event';
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

// VIS-20b: announcement, hover/focus pause (WCAG 2.2.1) through the EXISTING
// act_on_quick_command_notification command with new action strings.
type Act = { id: string; action: string };
type BannerFixture = { id: string; label: string; status: 'success' | 'failure' };
function mockNotification(initial: BannerFixture, options: { pause?: () => Promise<void> } = {}) {
  const tauri = tauriMocks();
  const acts: Act[] = [];
  let current: BannerFixture | null = initial;
  let channel: { onmessage: (payload: null) => void } | undefined;
  tauri.invoke.mockImplementation(async (command, args) => {
    if (command === 'get_quick_command_notification') { if (args?.channel) channel = args.channel; return current; }
    if (command === 'act_on_quick_command_notification') {
      acts.push({ id: args.id, action: args.action });
      if (args.action === 'pause' && options.pause) await options.pause();
      if (args.action === 'dismiss' || args.action === 'open') current = null;
    }
  });
  const acknowledged = (id: string) => tauri.invoke.mock.calls.some(([, args]) => args?.renderedId === id);
  return {
    acts, acknowledged,
    replace(next: BannerFixture) { current = next; channel?.onmessage(null); },
    pauseResume: () => acts.filter((entry) => entry.action === 'pause' || entry.action === 'resume')
  };
}

it.each([['success', 'status', 'Succeeded'], ['failure', 'alert', 'Failed']])('announces a %s banner through a %s live region that contains the label and verdict', async (status, role, verdict) => {
  mockNotification({ id: 'a', label: 'Build release', status: status as BannerFixture['status'] });
  const view = render(QuickCommandNotificationSurface);
  const region = await view.findByRole(role);
  expect(region.textContent).toContain('Build release');
  expect(region.textContent).toContain(verdict);
  // The other severity must not be announced for this banner.
  expect(view.queryByRole(role === 'status' ? 'alert' : 'status')).toBeNull();
});

it('hover pauses once, repeated hover is idempotent, leaving resumes once, for the current id only', async () => {
  const native = mockNotification({ id: 'a', label: 'Build', status: 'success' });
  const user = userEvent.setup();
  const view = render(QuickCommandNotificationSurface);
  const open = await view.findByRole('button', { name: /open build output/i });
  await waitFor(() => expect(native.acknowledged('a')).toBe(true));
  await user.hover(open);
  await waitFor(() => expect(native.pauseResume()).toEqual([{ id: 'a', action: 'pause' }]));
  await user.hover(view.getByRole('button', { name: /dismiss notification/i }));
  await user.hover(open);
  expect(native.pauseResume()).toEqual([{ id: 'a', action: 'pause' }]);
  await user.unhover(open);
  await waitFor(() => expect(native.pauseResume()).toEqual([{ id: 'a', action: 'pause' }, { id: 'a', action: 'resume' }]));
  await user.unhover(open);
  expect(native.pauseResume()).toHaveLength(2);
  expect(native.acts.some((entry) => entry.action === 'dismiss' || entry.action === 'open')).toBe(false);
});

it('focus-within pauses; moving focus inside keeps it paused; blur out resumes; hover+focus overlap resumes only after both end', async () => {
  const native = mockNotification({ id: 'f', label: 'Build', status: 'failure' });
  const user = userEvent.setup();
  const view = render(QuickCommandNotificationSurface);
  const open = await view.findByRole('button', { name: /open build output/i });
  const dismiss = view.getByRole('button', { name: /dismiss notification/i });
  await waitFor(() => expect(native.acknowledged('f')).toBe(true));
  open.focus();
  await waitFor(() => expect(native.pauseResume()).toEqual([{ id: 'f', action: 'pause' }]));
  dismiss.focus();
  await user.hover(dismiss);
  expect(native.pauseResume()).toEqual([{ id: 'f', action: 'pause' }]);
  await user.unhover(dismiss); // still focused: must stay paused
  await Promise.resolve();
  expect(native.pauseResume()).toEqual([{ id: 'f', action: 'pause' }]);
  dismiss.blur();
  await waitFor(() => expect(native.pauseResume()).toEqual([{ id: 'f', action: 'pause' }, { id: 'f', action: 'resume' }]));
});

it('pause is best-effort: a rejected or slow pause neither shows an error nor blocks dismiss', async () => {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  const native = mockNotification({ id: 'p', label: 'Build', status: 'success' }, { pause: () => gate.then(() => Promise.reject(new Error('stale'))) });
  const user = userEvent.setup();
  const view = render(QuickCommandNotificationSurface);
  const open = await view.findByRole('button', { name: /open build output/i });
  await waitFor(() => expect(native.acknowledged('p')).toBe(true));
  await user.hover(open);
  await waitFor(() => expect(native.pauseResume()).toEqual([{ id: 'p', action: 'pause' }]));
  // Pause is still pending: dismiss must not wait for it.
  await user.click(view.getByRole('button', { name: /dismiss notification/i }));
  await waitFor(() => expect(native.acts).toContainEqual({ id: 'p', action: 'dismiss' }));
  release();
  await tick();
  await Promise.resolve();
  expect(view.queryByText(/action unavailable/i)).toBeNull();
});

it('a replacement banner never receives the previous banner pause or resume', async () => {
  const native = mockNotification({ id: 'one', label: 'First', status: 'success' });
  const user = userEvent.setup();
  const view = render(QuickCommandNotificationSurface);
  const first = await view.findByRole('button', { name: /open first output/i });
  await waitFor(() => expect(native.acknowledged('one')).toBe(true));
  await user.hover(first);
  await waitFor(() => expect(native.pauseResume()).toEqual([{ id: 'one', action: 'pause' }]));
  native.replace({ id: 'two', label: 'Second', status: 'failure' });
  const second = await view.findByRole('button', { name: /open second output/i });
  await user.unhover(second);
  await tick();
  // Per banner id the sequence alternates pause/resume starting with pause; 'one' is never resumed
  // after 'two' became current, and a banner that was never paused is never resumed.
  for (const id of ['one', 'two']) {
    const sequence = native.pauseResume().filter((entry) => entry.id === id).map((entry) => entry.action);
    sequence.forEach((action, index) => expect(action).toBe(index % 2 === 0 ? 'pause' : 'resume'));
  }
  const entries = native.pauseResume();
  const firstTwo = entries.findIndex((entry) => entry.id === 'two');
  if (firstTwo >= 0) expect(entries.slice(firstTwo).every((entry) => entry.id === 'two')).toBe(true);
});

it.each([
  ['window blur', () => window.dispatchEvent(new Event('blur'))],
  ['document hidden', () => { Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => 'hidden' }); Object.defineProperty(document, 'hidden', { configurable: true, get: () => true }); document.dispatchEvent(new Event('visibilitychange')); }]
])('hover state is cleared (resume sent once) when the %s happens without a pointer-leave', async (_name, trigger) => {
  const native = mockNotification({ id: 'h', label: 'Build', status: 'success' });
  const user = userEvent.setup();
  const view = render(QuickCommandNotificationSurface);
  const open = await view.findByRole('button', { name: /open build output/i });
  await waitFor(() => expect(native.acknowledged('h')).toBe(true));
  await user.hover(open);
  await waitFor(() => expect(native.pauseResume()).toEqual([{ id: 'h', action: 'pause' }]));
  trigger();
  await waitFor(() => expect(native.pauseResume()).toEqual([{ id: 'h', action: 'pause' }, { id: 'h', action: 'resume' }]));
  await user.unhover(open);
  expect(native.pauseResume()).toHaveLength(2);
  Reflect.deleteProperty(document, 'visibilityState');
  Reflect.deleteProperty(document, 'hidden');
});

it('direct A to B replacement while the pointer is not over the card never pauses B', async () => {
  const native = mockNotification({ id: 'A', label: 'First', status: 'success' });
  const user = userEvent.setup();
  const view = render(QuickCommandNotificationSurface);
  const first = await view.findByRole('button', { name: /open first output/i });
  await waitFor(() => expect(native.acknowledged('A')).toBe(true));
  await user.hover(first);
  await waitFor(() => expect(native.pauseResume()).toEqual([{ id: 'A', action: 'pause' }]));
  await user.unhover(first);
  await waitFor(() => expect(native.pauseResume()).toHaveLength(2));
  native.replace({ id: 'B', label: 'Second', status: 'failure' });
  await view.findByRole('button', { name: /open second output/i });
  await waitFor(() => expect(native.acknowledged('B')).toBe(true));
  await tick();
  await new Promise((resolve) => setTimeout(resolve, 50));
  expect(native.pauseResume().filter((entry) => entry.id === 'B')).toEqual([]);
});
