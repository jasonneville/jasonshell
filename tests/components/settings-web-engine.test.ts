import { render, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsPanelSurface from '../../src/components/SettingsPanelSurface.svelte';
import { defaultShellSettings } from '../../src/lib/settings';
import { tauriMocks } from './setup';

vi.mock('@tauri-apps/api/webviewWindow', () => ({ getCurrentWebviewWindow: () => ({ label: 'settings-panel' }) }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ label: 'settings-panel', listen: tauriMocks().listen }) }));

// Production Settings with only the native boundary mocked.
let stored: any;
let loadMode: 'ok' | 'fail' | 'missing' | 'pending';
let releaseLoad: () => void;
let saves: any[];
let saveHook: ((settings: any) => Promise<any>) | null;

beforeEach(() => {
  localStorage.clear();
  saves = []; saveHook = null; loadMode = 'ok';
  stored = defaultShellSettings();
  tauriMocks().listen.mockResolvedValue(vi.fn());
  tauriMocks().invoke.mockImplementation(async (command: string, args?: any) => {
    if (command === 'load_shell_settings') {
      if (loadMode === 'fail') throw new Error('load boom');
      if (loadMode === 'pending') await new Promise<void>((resolve) => { releaseLoad = resolve; });
      const copy = structuredClone(stored);
      if (loadMode === 'missing') delete copy.search;
      return copy;
    }
    if (command === 'save_shell_settings') {
      const settings = structuredClone(args.settings);
      saves.push(settings);
      if (saveHook) return saveHook(settings);
      stored = settings;
      return structuredClone(settings);
    }
    if (command === 'get_speech_model_status') return { state: 'missing', source: null, error: null };
    if (command === 'get_standard_hotkey_status') return { revision: 1, bindings: [] };
    if (command === 'get_shell_shutdown_status') return { attemptId: '', state: 'blocked', phase: 'running', canCancel: false, blockers: [], failedSteps: [], completedSteps: [] };
    if (command === 'hide_settings_panel') return null;
    return null;
  });
});

const trigger = (view: ReturnType<typeof render>, label: string) => view.container.querySelector<HTMLButtonElement>(`button.melt-select-trigger[aria-label="${label}"]`)!;
async function choose(view: ReturnType<typeof render>, label: string, option: string) {
  const user = userEvent.setup();
  await user.click(trigger(view, label));
  await user.click(await view.findByRole('option', { name: option, hidden: true }));
}

describe('Settings "Web search engine"', () => {
  it('shows the loaded engine', async () => {
    stored.search = { ...stored.search, webEngine: 'bing' };
    const view = render(SettingsPanelSurface);
    await waitFor(() => expect(trigger(view, 'Web search engine').textContent).toContain('Bing'));
  });

  it.each(['missing', 'fail'] as const)('shows Google when settings are %s', async (mode) => {
    loadMode = mode;
    const view = render(SettingsPanelSurface);
    await waitFor(() => expect(trigger(view, 'Web search engine').textContent).toContain('Google'));
  });

  it('is disabled until settings load, then enabled; stays disabled after a failed load', async () => {
    loadMode = 'pending';
    const view = render(SettingsPanelSurface);
    await waitFor(() => expect(trigger(view, 'Web search engine')).toBeTruthy());
    expect(trigger(view, 'Web search engine').matches(':disabled')).toBe(true);
    await waitFor(() => expect(releaseLoad).toBeTypeOf('function'));
    releaseLoad();
    await waitFor(() => expect(trigger(view, 'Web search engine').matches(':disabled')).toBe(false));

    view.unmount();
    loadMode = 'fail';
    const failed = render(SettingsPanelSurface);
    await waitFor(() => expect(failed.getByRole('alert').textContent).toContain('load boom'));
    expect(trigger(failed, 'Web search engine').matches(':disabled')).toBe(true);
  });

  it('choosing Brave saves once with search.webEngine brave and every other field unchanged', async () => {
    const view = render(SettingsPanelSurface);
    await waitFor(() => expect(trigger(view, 'Web search engine').matches(':disabled')).toBe(false));
    const before = structuredClone(stored);
    await choose(view, 'Web search engine', 'Brave');
    await waitFor(() => expect(saves).toHaveLength(1));
    const expected = structuredClone(before);
    expected.search = { ...expected.search, webEngine: 'brave' };
    expect(saves[0]).toEqual(expected);
    await waitFor(() => expect(trigger(view, 'Web search engine').textContent).toContain('Brave'));
  });

  it('changing terminal then engine before the first save resolves: the second save carries both', async () => {
    let finishFirst!: () => void;
    saveHook = (settings) => {
      if (saves.length === 1) return new Promise((resolve) => { finishFirst = () => { stored = settings; resolve(structuredClone(settings)); }; });
      stored = settings;
      return Promise.resolve(structuredClone(settings));
    };
    const view = render(SettingsPanelSurface);
    await waitFor(() => expect(trigger(view, 'Web search engine').matches(':disabled')).toBe(false));
    await choose(view, 'Stack Browser terminal', 'PowerShell');
    await waitFor(() => expect(saves).toHaveLength(1));
    await choose(view, 'Web search engine', 'Brave');
    // Saves are serialized: the second waits for the first, then reloads and merges.
    await new Promise((resolve) => setTimeout(resolve, 30));
    expect(saves).toHaveLength(1);
    expect(saves[0].stackBrowser.terminalProfile).toBe('powershell');
    finishFirst();
    await waitFor(() => expect(saves).toHaveLength(2));
    expect(saves[1].stackBrowser.terminalProfile).toBe('powershell');
    expect(saves[1].search.webEngine).toBe('brave');
  });

  it('a save merges into freshly loaded settings: bar height/lock changed after mount survive', async () => {
    const view = render(SettingsPanelSurface);
    await waitFor(() => expect(trigger(view, 'Web search engine').matches(':disabled')).toBe(false));
    // Another surface saved new bar settings after Settings mounted.
    stored.ui = { ...stored.ui, topBarHeightLogical: 41.2, bottomBarHeightLogical: 37.7, lockTopBarHeight: false, lockBottomBarHeight: false };
    stored.taskHistory = [{ marker: 'newer' }];
    await choose(view, 'Web search engine', 'Brave');
    await waitFor(() => expect(saves).toHaveLength(1));
    expect(saves[0].ui).toEqual(stored.ui);
    expect(saves[0].taskHistory).toEqual([{ marker: 'newer' }]);
    expect(saves[0].search.webEngine).toBe('brave');
  });

  it('a field changed natively between mount and save (bar height, lock, hotkeys) is kept', async () => {
    const view = render(SettingsPanelSurface);
    await waitFor(() => expect(trigger(view, 'Web search engine').matches(':disabled')).toBe(false));
    stored.hotkeys = { ...stored.hotkeys, search: 'Ctrl+Alt+K' };
    await choose(view, 'Stack Browser terminal', 'Git Bash');
    await waitFor(() => expect(saves).toHaveLength(1));
    expect(saves[0].hotkeys.search).toBe('Ctrl+Alt+K');
    expect(saves[0].stackBrowser.terminalProfile).toBe('gitBash');
  });

  it('a rejected first save does not block the next save', async () => {
    let calls = 0;
    saveHook = async (settings) => { if (++calls === 1) throw new Error('first fails'); stored = settings; return structuredClone(settings); };
    const view = render(SettingsPanelSurface);
    await waitFor(() => expect(trigger(view, 'Web search engine').matches(':disabled')).toBe(false));
    await choose(view, 'Web search engine', 'Brave');
    await waitFor(() => expect(view.getByRole('alert').textContent).toContain('first fails'));
    await choose(view, 'Stack Browser terminal', 'PowerShell');
    await waitFor(() => expect(saves).toHaveLength(2));
    expect(saves[1].stackBrowser.terminalProfile).toBe('powershell');
    await waitFor(() => expect(view.queryByRole('alert')).toBeNull());
  });

  it('a rejected save shows the alert; the next successful save clears it', async () => {
    saveHook = async () => { throw new Error('disk full'); };
    const view = render(SettingsPanelSurface);
    await waitFor(() => expect(trigger(view, 'Web search engine').matches(':disabled')).toBe(false));
    await choose(view, 'Web search engine', 'Brave');
    await waitFor(() => expect(view.getByRole('alert').textContent).toContain('disk full'));
    saveHook = null;
    await choose(view, 'Web search engine', 'Bing');
    await waitFor(() => expect(view.queryByRole('alert')).toBeNull());
    expect(saves.at(-1).search.webEngine).toBe('bing');
  });
});
