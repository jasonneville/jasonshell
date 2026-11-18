import { fireEvent, render, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsPanelSurface from '../../src/components/SettingsPanelSurface.svelte';
import { defaultShellSettings } from '../../src/lib/settings';
import { tauriMocks } from './setup';

vi.mock('@tauri-apps/api/webviewWindow', () => ({ getCurrentWebviewWindow: () => ({ label: 'settings-panel', listen: tauriMocks().listen }) }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ label: 'settings-panel', listen: tauriMocks().listen }) }));

beforeEach(() => {
  localStorage.clear();
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
  tauriMocks().listen.mockResolvedValue(vi.fn());
  tauriMocks().invoke.mockImplementation(async (command: string) => {
    if (command === 'load_shell_settings') return defaultShellSettings();
    if (command === 'get_standard_hotkey_status') return { revision: 1, bindings: [] };
    if (command === 'get_speech_model_status') return { state: 'missing', source: null, error: null };
    if (command === 'request_shell_exit') return;
    if (command === 'hide_settings_panel' || command === 'trigger_system_power_action') return;
    throw new Error(`Unexpected native side effect before exit confirmation: ${command}`);
  });
});

function sideEffects() {
  const reads = new Set(['load_shell_settings', 'get_standard_hotkey_status', 'get_speech_model_status']);
  return tauriMocks().invoke.mock.calls.filter(([command]) => !reads.has(String(command)));
}

async function openConfirmation() {
  const view = render(SettingsPanelSurface);
  await waitFor(() => expect(tauriMocks().invoke.mock.calls.some(([command]) => command === 'load_shell_settings')).toBe(true));
  const trigger = view.getByRole('button', { name: 'Exit JasonShell', exact: true });
  trigger.focus();
  await fireEvent.click(trigger);
  const dialog = await view.findByRole('alertdialog');
  return { view, trigger, dialog, controls: within(dialog) };
}

describe('Settings-only graceful Exit JasonShell confirmation', () => {
  it('defaults focus to Cancel and admits zero native mutations before confirmation', async () => {
    const { controls } = await openConfirmation();
    await waitFor(() => expect(document.activeElement).toBe(controls.getByRole('button', { name: 'Cancel', exact: true })));
    expect(sideEffects()).toEqual([]);
  });

  it('Cancel restores Exit focus without teardown, saves, hide, or native exit admission', async () => {
    const { view, trigger, controls } = await openConfirmation();
    await fireEvent.click(controls.getByRole('button', { name: 'Cancel', exact: true }));
    await waitFor(() => expect(view.queryByRole('alertdialog')).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(trigger));
    expect(sideEffects()).toEqual([]);
  });

  it('Escape cancels only the confirmation, preserves Settings, and restores trigger focus', async () => {
    const { view, trigger, controls } = await openConfirmation();
    await fireEvent.keyDown(controls.getByRole('button', { name: 'Cancel', exact: true }), { key: 'Escape', code: 'Escape' });
    await waitFor(() => expect(view.queryByRole('alertdialog')).toBeNull());
    await waitFor(() => expect(document.activeElement).toBe(trigger));
    await fireEvent.keyDown(trigger, { key: 'Escape', code: 'Escape', repeat: true });
    await fireEvent.keyDown(trigger, { key: 'Escape', code: 'Escape', repeat: true });
    await fireEvent.keyUp(trigger, { key: 'Escape', code: 'Escape' });
    expect(view.getByRole('heading', { name: 'Settings' })).toBeTruthy();
    expect(sideEffects()).toEqual([]);
  });

  it.each(['new-copy', 'invalid-raw', 'name-only'])('cancelled exit preserves %s theme draft bytes without persistence', async (kind) => {
    const view = render(SettingsPanelSurface);
    await fireEvent.click(view.getByRole('button', { name: 'Create copy', exact: true }));
    const name = await view.findByRole('textbox', { name: 'Theme name', exact: true }) as HTMLInputElement;
    if (kind === 'name-only') await fireEvent.input(name, { target: { value: 'Unsaved name' } });
    const accent = view.getByRole('textbox', { name: 'Accent hex', exact: true }) as HTMLInputElement;
    if (kind === 'invalid-raw') await fireEvent.input(accent, { target: { value: '#not-a-color' } });
    const snapshot = { name: name.value, accent: accent.value, storage: localStorage.getItem('jasonshell.themes.v1') };
    await fireEvent.click(view.getByRole('button', { name: 'Exit JasonShell', exact: true }));
    const dialog = await view.findByRole('alertdialog');
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel', exact: true }));
    expect(name.value).toBe(snapshot.name);
    expect(accent.value).toBe(snapshot.accent);
    expect(localStorage.getItem('jasonshell.themes.v1')).toBe(snapshot.storage);
    expect(sideEffects()).toEqual([]);
  });

  it('warns that unsaved work is lost, submits one path-free request, and stays exiting', async () => {
    const { view, dialog, controls } = await openConfirmation();
    expect(dialog.textContent).toMatch(/unsaved[^.]*\b(?:lost|discarded)\b/i);
    const confirm = controls.getByRole('button', { name: /exit/i });
    await fireEvent.click(confirm);
    await fireEvent.click(confirm);
    await waitFor(() => expect(view.getByText(/exiting jasonshell/i)).toBeTruthy());
    await fireEvent.keyDown(window, { key: 'Escape', code: 'Escape' });
    expect(view.getByText(/exiting jasonshell/i)).toBeTruthy();
    const calls = sideEffects();
    expect(calls).toHaveLength(1);
    expect(calls[0][0]).toBe('request_shell_exit');
    expect(calls[0][1] ?? {}).toEqual({});
    expect(view.queryByRole('button', { name: /retry|save and exit/i })).toBeNull();
  });

  it('does not wait for dirty draft or unavailable exit listeners before native admission', async () => {
    tauriMocks().listen.mockRejectedValue(new Error('renderer listeners unavailable'));
    const { view, trigger, controls } = await openConfirmation();
    await fireEvent.click(controls.getByRole('button', { name: 'Cancel', exact: true }));
    await fireEvent.click(view.getByRole('button', { name: 'Create copy', exact: true }));
    await fireEvent.input(await view.findByRole('textbox', { name: 'Theme name', exact: true }), { target: { value: 'Unsaved name' } });
    const before = localStorage.getItem('jasonshell.themes.v1');
    await fireEvent.click(trigger);
    await fireEvent.click(within(await view.findByRole('alertdialog')).getByRole('button', { name: /exit/i }));
    await waitFor(() => expect(view.getByText(/exiting jasonshell/i)).toBeTruthy());
    expect(localStorage.getItem('jasonshell.themes.v1')).toBe(before);
    expect(sideEffects().map(([command]) => command)).toEqual(['request_shell_exit']);
  });

  it('shows a rejection before native acceptance and allows a fresh user confirmation', async () => {
    const invoke = tauriMocks().invoke;
    const normal = invoke.getMockImplementation()!;
    invoke.mockImplementation((command, ...args) => command === 'request_shell_exit'
      ? Promise.reject(new Error('Settings instance is no longer current')) : normal(command, ...args));
    const { view, trigger, controls } = await openConfirmation();
    await fireEvent.click(controls.getByRole('button', { name: /exit/i }));
    await waitFor(() => expect(view.getByText(/(?:could not|unable to|failed to).{0,30}exit|exit.{0,30}(?:could not|failed)/i)).toBeTruthy());
    expect(view.queryByText(/^exiting jasonshell/i)).toBeNull();
    await fireEvent.click(trigger);
    expect(await view.findByRole('alertdialog')).toBeTruthy();
    expect(sideEffects().map(([command]) => command)).toEqual(['request_shell_exit']);
  });

  it.each([['Sleep', 'sleep'], ['Restart', 'restart'], ['Turn Off', 'shutdown']])('routes %s only through its own mocked power command', async (label, action) => {
    const view = render(SettingsPanelSurface);
    await fireEvent.click(view.getByRole('button', { name: label, exact: true }));
    expect(sideEffects()).toEqual([]);
    await fireEvent.click(view.getByRole('button', { name: `Confirm ${label}`, exact: true }));
    expect(sideEffects()).toEqual([['trigger_system_power_action', { request: { action } }]]);
  });

  it('ordinary Settings close hides without requesting exit', async () => {
    const view = render(SettingsPanelSurface);
    await fireEvent.click(view.getByRole('button', { name: 'Close settings' }));
    expect(sideEffects().map(([command]) => command)).toEqual(['hide_settings_panel']);
  });
});
