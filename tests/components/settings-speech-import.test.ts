import { fireEvent, render, waitFor } from '@testing-library/svelte';
import { readFileSync } from 'node:fs';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsPanelSurface from '../../src/components/SettingsPanelSurface.svelte';
import { defaultShellSettings } from '../../src/lib/settings';
import { tauriMocks } from './setup';

// Native IPC mocked, actual Settings component and speech wrappers mounted.
// Persistent disk/restart behavior is independently exercised by Rust acceptance tests.
type ModelStatus = { state: 'missing' | 'loading' | 'ready' | 'error'; source: 'installed' | 'bundled' | null; error: string | null };
const missing: ModelStatus = { state: 'missing', source: null, error: null };
const ready: ModelStatus = { state: 'ready', source: 'installed', error: null };
let status: ModelStatus;
let importModel: () => Promise<unknown>;

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

function statusCalls() {
  return tauriMocks().invoke.mock.calls.filter(([command]) => command === 'get_speech_model_status').length;
}

function mockStatusResponse(response: () => Promise<ModelStatus>) {
  tauriMocks().invoke.mockImplementation((command: string) => {
    if (command === 'load_shell_settings') return Promise.resolve(defaultShellSettings());
    if (command === 'get_speech_model_status') return response();
    if (command === 'import_speech_model') return importModel();
    throw new Error(`Unexpected Settings IPC: ${command}`);
  });
}

beforeEach(() => {
  status = { ...missing };
  importModel = async () => ({ cancelled: true, model: status });
  localStorage.clear();
  const tauri = tauriMocks();
  tauri.listen.mockResolvedValue(vi.fn());
  tauri.invoke.mockImplementation((command: string) => {
    if (command === 'load_shell_settings') return Promise.resolve(defaultShellSettings());
    if (command === 'get_speech_model_status') return Promise.resolve(status);
    if (command === 'import_speech_model') return importModel();
    throw new Error(`Unexpected Settings IPC: ${command}`);
  });
});

async function mountSettings() {
  const view = render(SettingsPanelSurface);
  expect(view.getByRole('heading', { name: 'Settings' })).toBeTruthy();
  await waitFor(() => expect(tauriMocks().invoke).toHaveBeenCalledWith('get_speech_model_status'));
  return view;
}

describe('Settings speech model import acceptance (mocked native IPC)', () => {
  it('opens Settings, imports once, becomes ready immediately, then remounts ready without another import', async () => {
    let finish!: (value: unknown) => void;
    importModel = () => new Promise((resolve) => { finish = resolve; });
    const view = await mountSettings();
    expect(view.container.textContent).toMatch(/missing|not installed|import.*model/i);
    const button = view.getByRole('button', { name: 'Import speech model' });
    await fireEvent.click(button);
    await waitFor(() => expect((button as HTMLButtonElement).disabled).toBe(true));
    expect(button.textContent).toMatch(/importing|installing|working/i);
    await fireEvent.click(button);
    expect(tauriMocks().invoke.mock.calls.filter(([command]) => command === 'import_speech_model')).toHaveLength(1);
    // Picker path is owned entirely by backend; renderer sends no arbitrary file path.
    expect(tauriMocks().invoke).toHaveBeenCalledWith('import_speech_model');
    status = ready;
    finish({ cancelled: false, model: ready });
    await waitFor(() => expect(view.container.textContent).toMatch(/ready/i));
    expect((view.getByRole('button', { name: 'Import speech model' }) as HTMLButtonElement).disabled).toBe(false);
    view.unmount();
    const restarted = await mountSettings();
    await waitFor(() => expect(restarted.container.textContent).toMatch(/ready/i));
    expect(restarted.container.textContent).toMatch(/installed/i);
    expect(tauriMocks().invoke.mock.calls.filter(([command]) => command === 'import_speech_model')).toHaveLength(1);
  });

  it('native picker cancellation preserves installed ready state, without error', async () => {
    status = ready;
    const view = await mountSettings();
    await waitFor(() => expect(view.container.textContent).toMatch(/ready/i));
    await fireEvent.click(view.getByRole('button', { name: 'Import speech model' }));
    await waitFor(() => expect((view.getByRole('button', { name: 'Import speech model' }) as HTMLButtonElement).disabled).toBe(false));
    expect(view.container.textContent).toMatch(/ready/i);
    expect(view.queryByRole('alert')).toBeNull();
  });

  it('failed import displays actionable backend message, retains ready model, permits retry', async () => {
    status = ready;
    importModel = async () => { throw new Error('Choose a complete Parakeet TDT int8 .tar or .tar.gz archive.'); };
    const view = await mountSettings();
    await fireEvent.click(view.getByRole('button', { name: 'Import speech model' }));
    await waitFor(() => expect(view.getByRole('alert').textContent).toContain('Choose a complete Parakeet'));
    expect(view.container.textContent).toMatch(/ready/i);
    expect((view.getByRole('button', { name: 'Import speech model' }) as HTMLButtonElement).disabled).toBe(false);
    importModel = async () => ({ cancelled: false, model: ready });
    await fireEvent.click(view.getByRole('button', { name: 'Import speech model' }));
    await waitFor(() => expect(view.queryByRole('alert')).toBeNull());
  });

  it.each([
    [{ state: 'missing', source: null, error: null }, /missing|not installed/i],
    [{ state: 'loading', source: 'installed', error: null }, /loading/i],
    [{ state: 'error', source: 'installed', error: 'Model cannot load. Import a compatible archive.' }, /cannot load|unavailable|error/i],
    [{ state: 'ready', source: 'bundled', error: null }, /bundled/i]
  ] as const)('renders truthful model state %j', async (model, label) => {
    status = model;
    const view = await mountSettings();
    await waitFor(() => expect(view.container.textContent).toMatch(label));
    if (model.state !== 'ready') expect(view.container.textContent).not.toMatch(/model.*ready|ready.*model/i);
  });

  it('polls loading through the component timer, announces ready, then stops polling', async () => {
    vi.useFakeTimers();
    status = { state: 'loading', source: 'installed', error: null };
    const view = render(SettingsPanelSurface);
    await tick();
    await vi.advanceTimersByTimeAsync(0);
    await tick();
    expect(view.getByRole('status').textContent).toMatch(/loading speech model/i);
    expect(statusCalls()).toBe(1);
    status = ready;
    await vi.advanceTimersByTimeAsync(1000);
    await tick();
    expect(statusCalls()).toBe(2);
    expect(view.getByRole('status').textContent).toMatch(/ready \(installed\)/i);
    await vi.advanceTimersByTimeAsync(5000);
    expect(statusCalls()).toBe(2);
  });

  it.each(['missing response', 'rejected response'])('ignores stale initial %s after successful import', async (outcome) => {
    const initial = deferred<ModelStatus>();
    mockStatusResponse(() => initial.promise);
    importModel = async () => ({ cancelled: false, model: ready });
    const view = await mountSettings();
    expect(view.getByRole('status').textContent).toMatch(/checking/i);
    await fireEvent.click(view.getByRole('button', { name: 'Import speech model' }));
    await waitFor(() => expect(view.getByRole('status').textContent).toMatch(/ready \(installed\)/i));
    if (outcome === 'missing response') initial.resolve(missing);
    else initial.reject(new Error('Outdated status check failed'));
    await initial.promise.catch(() => undefined);
    await tick();
    expect(view.getByRole('status').textContent).toMatch(/ready \(installed\)/i);
    expect(view.queryByRole('alert')).toBeNull();
  });

  it.each([
    [ready, /ready \(installed\)/i],
    [missing, /not installed/i]
  ] as const)('refreshes unknown initial status after rejected import and retains actionable error (%j)', async (recovered, label) => {
    const initial = deferred<ModelStatus>();
    const recovery = deferred<ModelStatus>();
    const message = 'Choose a complete Parakeet TDT int8 .tar or .tar.gz archive.';
    mockStatusResponse(() => statusCalls() === 1 ? initial.promise : recovery.promise);
    importModel = async () => { throw new Error(message); };
    const view = await mountSettings();
    try {
      expect(view.getByRole('status').textContent).toMatch(/checking/i);
      await fireEvent.click(view.getByRole('button', { name: 'Import speech model' }));
      await waitFor(() => expect(view.getByRole('alert').textContent).toContain(message));
      // Failure cannot leave an unknown snapshot permanently stuck at Checking.
      await waitFor(() => expect(statusCalls()).toBe(2));
      recovery.resolve(recovered);
      await recovery.promise;
      await tick();
      await waitFor(() => expect(view.getByRole('status').textContent).toMatch(label));
      expect(view.getByRole('alert').textContent).toContain(message);
      expect((view.getByRole('button', { name: 'Import speech model' }) as HTMLButtonElement).disabled).toBe(false);
      expect(tauriMocks().invoke).toHaveBeenCalledWith('import_speech_model');
      initial.resolve({ state: 'loading', source: 'bundled', error: null });
      await initial.promise;
      await tick();
      expect(view.getByRole('status').textContent).toMatch(label);
      expect(view.getByRole('alert').textContent).toContain(message);
    } finally {
      view.unmount();
      initial.resolve(missing);
      recovery.resolve(recovered);
    }
  });

  it('unmount cancels scheduled loading poll without further IPC', async () => {
    vi.useFakeTimers();
    status = { state: 'loading', source: 'installed', error: null };
    const view = render(SettingsPanelSurface);
    await tick();
    await vi.advanceTimersByTimeAsync(0);
    await tick();
    expect(view.getByRole('status').textContent).toMatch(/loading/i);
    view.unmount();
    const before = tauriMocks().invoke.mock.calls.length;
    await vi.advanceTimersByTimeAsync(5000);
    expect(tauriMocks().invoke).toHaveBeenCalledTimes(before);
    expect(view.container.textContent).toBe('');
  });

  it.each(['resolve', 'reject'])('ignores in-flight poll %s after unmount without rescheduling', async (outcome) => {
    vi.useFakeTimers();
    const pending = deferred<ModelStatus>();
    mockStatusResponse(() => statusCalls() === 1
      ? Promise.resolve({ state: 'loading', source: 'installed', error: null })
      : pending.promise);
    const view = render(SettingsPanelSurface);
    await tick();
    await vi.advanceTimersByTimeAsync(1000);
    expect(statusCalls()).toBe(2);
    view.unmount();
    const before = tauriMocks().invoke.mock.calls.length;
    if (outcome === 'resolve') pending.resolve({ state: 'loading', source: 'installed', error: null });
    else pending.reject(new Error('Late native status error'));
    await pending.promise.catch(() => undefined);
    await tick();
    await vi.advanceTimersByTimeAsync(5000);
    expect(tauriMocks().invoke).toHaveBeenCalledTimes(before);
    expect(view.container.textContent).toBe('');
  });

  it('ignores late import completion after unmount without scheduling loading polls', async () => {
    vi.useFakeTimers();
    const pending = deferred<unknown>();
    importModel = () => pending.promise;
    const view = render(SettingsPanelSurface);
    await tick();
    await fireEvent.click(view.getByRole('button', { name: 'Import speech model' }));
    expect((view.getByRole('button', { name: /installing speech model/i }) as HTMLButtonElement).disabled).toBe(true);
    view.unmount();
    const before = tauriMocks().invoke.mock.calls.length;
    pending.resolve({ cancelled: false, model: { state: 'loading', source: 'installed', error: null } });
    await pending.promise;
    await tick();
    await vi.advanceTimersByTimeAsync(5000);
    expect(tauriMocks().invoke).toHaveBeenCalledTimes(before);
    expect(view.container.textContent).toBe('');
  });

  it('uses polite semantic status and native button inside shared Settings styling wrapper', async () => {
    status = ready;
    const view = await mountSettings();
    const announcement = view.getByRole('status');
    expect(announcement.getAttribute('aria-live')).toBe('polite');
    const button = view.getByRole('button', { name: 'Import speech model' });
    expect(button.tagName).toBe('BUTTON');
    expect(button.getAttribute('type')).toBe('button');
    expect(button.closest('.settings-section')?.getAttribute('aria-labelledby')).toBe('speech-model-heading');
    expect(button.parentElement?.classList.contains('speech-model-actions')).toBe(true);
    expect(button.parentElement?.getAttribute('aria-busy')).toBe('false');
    // Static primitive/style contract supplements DOM semantics; not visual QA.
    const source = readFileSync('src/components/SettingsPanelSurface.svelte', 'utf8');
    const css = readFileSync('src/components/SettingsPanelSurface.css', 'utf8');
    expect(source).toMatch(/<MeltActionButton\s+disabled=\{speechModelBusy\}\s+onClick=\{handleSpeechModelImport\}/);
    expect(css).toMatch(/\.google-font-installer button,\s*\.speech-model-actions button\s*\{/);
    expect(css).toMatch(/\.speech-model-actions\s*\{[^}]*flex-wrap:\s*wrap;/);
    expect(css).toMatch(/\.speech-model-actions button:focus-visible\s*\{[^}]*outline:/);
  });
});
