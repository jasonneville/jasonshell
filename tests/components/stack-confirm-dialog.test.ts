import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { expect, it, vi } from 'vitest';
import StackConfirmDialog from '../../src/components/StackConfirmDialog.svelte';

it('delete confirmation is modal, focuses cancel first, disables backdrop, and returns focus', async () => {
  const origin = document.createElement('button');
  origin.textContent = 'Original focus';
  document.body.append(origin);
  origin.focus();
  const onCancel = vi.fn();
  const onConfirm = vi.fn();
  const view = render(StackConfirmDialog, {
    title: 'Delete selected item?', message: 'This cannot be undone.', confirmLabel: 'Delete',
    tone: 'danger', initialFocus: 'cancel', dismissOnBackdrop: false, returnFocus: origin,
    onCancel, onConfirm
  });
  const dialog = view.getByRole('alertdialog');
  expect(dialog.getAttribute('aria-modal')).toBe('true');
  const cancel = view.getByRole('button', { name: 'Cancel' });
  const confirm = view.getByRole('button', { name: 'Delete' });
  await waitFor(() => expect(document.activeElement).toBe(cancel));
  const backdrop = view.getByRole('button', { name: 'Dismiss confirmation dialog' }) as HTMLButtonElement;
  expect(backdrop.disabled).toBe(true);
  backdrop.click(); // Native click respects disabled; synthetic fireEvent bypasses it in jsdom.
  expect(onCancel).not.toHaveBeenCalled();
  await fireEvent.click(confirm);
  expect(onConfirm).toHaveBeenCalledTimes(1);
  await fireEvent.click(cancel);
  expect(onCancel).toHaveBeenCalledTimes(1);
  view.unmount();
  await waitFor(() => expect(document.activeElement).toBe(origin));
  origin.remove();
});

it.each([false, true])('side buttons are consumed before mutation; busy=%s only permits nonbusy Back cancellation', async (busy) => {
  const onCancel = vi.fn();
  const onConfirm = vi.fn();
  const view = render(StackConfirmDialog, { title: 'Delete?', busy, onCancel, onConfirm });
  const dialog = view.getByRole('alertdialog');
  function dispatch(button: number, type = 'pointerdown') {
    const event = new MouseEvent(type, { button, bubbles: true, cancelable: true });
    Object.defineProperty(event, 'pointerType', { value: 'mouse' });
    dialog.dispatchEvent(event);
    return event;
  }
  expect(dispatch(4).defaultPrevented).toBe(true);
  expect(onCancel).not.toHaveBeenCalled();
  expect(dispatch(3).defaultPrevented).toBe(true);
  dispatch(3, 'mousedown'); dispatch(3, 'auxclick');
  expect(onCancel).toHaveBeenCalledTimes(busy ? 0 : 1);
  expect(onConfirm).not.toHaveBeenCalled();
});
