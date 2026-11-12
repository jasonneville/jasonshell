import { fireEvent, render, waitFor, within } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import SettingsPanelSurface from '../../src/components/SettingsPanelSurface.svelte';
import { defaultShellSettings } from '../../src/lib/settings';
import { publishSavedThemeState } from '../../src/lib/themes';
import { persistCustomTheme } from '../../src/lib/themePersistence';
import { tauriMocks } from './setup';

// Exercise production Settings/editor modules, not a mocked theme store or editor.
// Multi-document appearance/transport is owned by the browser fixture.
const key = 'jasonshell.themes.v1';
beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute('data-theme');
  document.documentElement.removeAttribute('style');
  // jsdom does not implement rasterization. This suite tests interaction/state;
  // browser fixtures, not this inert context, establish resolved paint/contrast.
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null);
  const tauri = tauriMocks();
  tauri.listen.mockResolvedValue(vi.fn());
  tauri.invoke.mockImplementation((command: string) => {
    if (command === 'load_shell_settings') return Promise.resolve(defaultShellSettings());
    if (command === 'get_speech_model_status') return Promise.resolve({ state: 'missing', source: null, error: null });
    if (command === 'hide_settings_panel') return Promise.resolve();
    throw new Error(`Unexpected theme-editor IPC: ${command}`);
  });
});

async function openCopy() {
  const view = render(SettingsPanelSurface);
  await fireEvent.click(view.getByRole('button', { name: 'Create copy', exact: true }));
  const name = await view.findByRole('textbox', { name: 'Theme name', exact: true }) as HTMLInputElement;
  return { view, name };
}

describe('custom theme editor acceptance through production Settings', () => {
  it('builtin exposes Create copy without destructive builtin editing actions', () => {
    const view = render(SettingsPanelSurface);
    expect(view.getByRole('button', { name: 'Create copy', exact: true })).toBeTruthy();
    expect(view.queryByRole('button', { name: 'Delete theme', exact: true })).toBeNull();
    expect(view.queryByRole('button', { name: 'Edit theme', exact: true })).toBeNull();
  });

  it('creating a draft focuses its name, exposes labeled inputs, and persists nothing', async () => {
    const { view, name } = await openCopy();
    await waitFor(() => expect(document.activeElement).toBe(name));
    expect(localStorage.getItem(key)).toBeNull();
    for (const label of ['Background', 'Surface', 'Raised surface', 'Text', 'Muted text', 'Accent', 'Border']) {
      expect(view.getByLabelText(`${label} picker`, { exact: true })).toBeTruthy();
      expect(view.getByRole('textbox', { name: `${label} hex`, exact: true })).toBeTruthy();
      expect(view.getByLabelText(`${label} alpha`, { exact: true })).toBeTruthy();
      expect(view.getByRole('button', { name: `Reset ${label}`, exact: true })).toBeTruthy();
    }
  });

  it('invalid input is accessible, cannot persist, and correction can save', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: 'Valid custom' } });
    const accent = view.getByRole('textbox', { name: 'Accent hex', exact: true });
    const before = localStorage.getItem(key);
    await fireEvent.input(accent, { target: { value: 'url(https://invalid.example/)' } });
    await waitFor(() => expect(accent.getAttribute('aria-invalid')).toBe('true'));
    const describedBy = accent.getAttribute('aria-describedby');
    expect(describedBy).toBeTruthy();
    expect(describedBy!.split(/\s+/).some(id => Boolean(document.getElementById(id)?.textContent?.trim()))).toBe(true);
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    expect(localStorage.getItem(key)).toBe(before);
    expect((accent as HTMLInputElement).value).toContain('url(');
    await fireEvent.input(accent, { target: { value: '#123456' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    await waitFor(() => expect(JSON.parse(localStorage.getItem(key)!).customThemes[0].label).toBe('Valid custom'));
  });

  it('Cancel drops an unsaved draft, restores source control focus, and writes nothing', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: 'Discard me' } });
    await fireEvent.click(view.getByRole('button', { name: 'Cancel', exact: true }));
    await waitFor(() => expect(view.queryByRole('textbox', { name: 'Theme name', exact: true })).toBeNull());
    expect(localStorage.getItem(key)).toBeNull();
    expect(document.activeElement).toBe(view.getByRole('button', { name: 'Create copy', exact: true }));
  });

  it('Save trims name and atomically selects the saved custom palette', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: '  My palette  ' } });
    await fireEvent.input(view.getByRole('textbox', { name: 'Accent hex', exact: true }), { target: { value: '#123456' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    await waitFor(() => expect(localStorage.getItem(key)).not.toBeNull());
    const saved = JSON.parse(localStorage.getItem(key)!);
    expect(saved.version).toBe(1);
    expect(saved.customThemes).toHaveLength(1);
    expect(saved.selectedId).toBe(saved.customThemes[0].id);
    expect(saved.customThemes[0].label).toBe('My palette');
    expect(saved.customThemes[0].palette['--js-color-accent']).toMatch(/^#123456(?:ff)?$/i);
    expect(saved.customThemes[0].startingPalette['--js-color-accent']).not.toBe('#123456');
  });

  it('active delete Cancel leaves bytes identical; confirmation selects Base Dark in same write', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: 'Delete target' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    await waitFor(() => expect(view.getByRole('button', { name: 'Delete theme', exact: true })).toBeTruthy());
    const before = localStorage.getItem(key);
    await fireEvent.click(view.getByRole('button', { name: 'Delete theme', exact: true }));
    const dialog = await view.findByRole('alertdialog');
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel', exact: true }));
    expect(localStorage.getItem(key)).toBe(before);
    await fireEvent.click(view.getByRole('button', { name: 'Delete theme', exact: true }));
    const confirm = await view.findByRole('alertdialog');
    await fireEvent.click(within(confirm).getByRole('button', { name: /delete/i }));
    await waitFor(() => expect(JSON.parse(localStorage.getItem(key)!).selectedId).toBe('base-dark'));
    expect(JSON.parse(localStorage.getItem(key)!).customThemes).toEqual([]);
  });

  it('delete confirmation captures record revision; externally revised record survives confirmation with conflict error', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: 'Original' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    await waitFor(() => expect(view.getByRole('button', { name: 'Delete theme', exact: true })).toBeTruthy());
    const original = JSON.parse(localStorage.getItem(key)!).customThemes[0];
    await fireEvent.click(view.getByRole('button', { name: 'Delete theme', exact: true }));
    const dialog = await view.findByRole('alertdialog');
    persistCustomTheme({ ...original, label: 'Revised externally' }, original.revision);
    publishSavedThemeState();
    const revisedBytes = localStorage.getItem(key);
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete theme', exact: true }));
    expect(localStorage.getItem(key)).toBe(revisedBytes);
    await waitFor(() => expect(view.getByRole('alert').textContent).toMatch(/changed|revision|another|retry/i));
    expect(JSON.parse(localStorage.getItem(key)!).customThemes[0].label).toBe('Revised externally');
  });

  it('name-only edits save a new revision without replacing original reset baseline', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: 'Before rename' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    const original = JSON.parse(localStorage.getItem(key)!).customThemes[0];
    await fireEvent.click(view.getByRole('button', { name: 'Edit theme', exact: true }));
    await fireEvent.input(view.getByRole('textbox', { name: 'Theme name', exact: true }), { target: { value: 'After rename' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    const renamed = JSON.parse(localStorage.getItem(key)!).customThemes[0];
    expect(renamed.label).toBe('After rename'); expect(renamed.revision).toBe(original.revision + 1);
    expect(renamed.startingPalette).toEqual(original.startingPalette);
    expect(renamed.palette).toEqual(original.palette);
  });

  it('empty/intermediate name is retained, cannot save, and correction permits retry', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: '   ' } });
    expect(name.value).toBe('   ');
    expect(name.getAttribute('aria-invalid')).toBe('true');
    expect((view.getByRole('button', { name: 'Save & apply', exact: true }) as HTMLButtonElement).disabled).toBe(true);
    expect(localStorage.getItem(key)).toBeNull();
    await fireEvent.input(name, { target: { value: 'Recovered name' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    expect(JSON.parse(localStorage.getItem(key)!).customThemes[0].label).toBe('Recovered name');
  });

  it('quota failure keeps visible draft, reports failure, and allows successful retry', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: 'Retry me' } });
    const originalWrite = Storage.prototype.setItem;
    const spy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(function (key, value) {
      if (key === 'jasonshell.themes.v1') throw new DOMException('quota', 'QuotaExceededError');
      return originalWrite.call(this, key, value);
    });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    await waitFor(() => expect(view.getByRole('alert').textContent).toMatch(/save|storage|full/i));
    expect(name.value).toBe('Retry me'); expect(localStorage.getItem(key)).toBeNull();
    spy.mockRestore();
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    expect(JSON.parse(localStorage.getItem(key)!).customThemes[0].label).toBe('Retry me');
    expect(view.queryByRole('alert')).toBeNull();
  });

  it('reset colors clears invalid field state and restores baseline rather than latest saved edits', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: 'Reset target' } });
    const baseline = JSON.parse(JSON.stringify((await import('../../src/lib/themePalette')).getPresetThemePalette('base-dark')));
    await fireEvent.input(view.getByRole('textbox', { name: 'Accent hex', exact: true }), { target: { value: '#123456' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    await fireEvent.click(view.getByRole('button', { name: 'Edit theme', exact: true }));
    await fireEvent.input(view.getByRole('textbox', { name: 'Accent hex', exact: true }), { target: { value: '#oops' } });
    await fireEvent.click(view.getByRole('button', { name: 'Reset colors', exact: true }));
    expect(view.getByRole('textbox', { name: 'Accent hex', exact: true }).getAttribute('aria-invalid')).toBe('false');
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    expect(JSON.parse(localStorage.getItem(key)!).customThemes[0].palette).toEqual(baseline);
  });

  it('two tint edits reset independently; global Reset colors restores all exact baseline layers', async () => {
    localStorage.setItem('jasonshell.theme', 'synthwave-84');
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: 'Layer isolation' } });
    const advanced = view.container.querySelector<HTMLDetailsElement>('.theme-advanced')!;
    advanced.open = true;
    const first = view.getByRole('textbox', { name: 'Focus ring tint layer 1 hex', exact: true });
    const second = view.getByRole('textbox', { name: 'Focus ring tint layer 2 hex', exact: true });
    await fireEvent.input(first, { target: { value: '#112233' } });
    await fireEvent.input(second, { target: { value: '#aabbcc' } });
    await fireEvent.click(view.getByRole('button', { name: 'Reset Focus ring tint layer 2', exact: true }));
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    const saved = JSON.parse(localStorage.getItem(key)!).customThemes[0];
    // Hex editing retains the original 0.7 alpha (round(0.7*255) = 0xb3).
    expect(saved.palette['--js-focus-ring']).toBe('0 0 0 2px #112233b3, 0 0 0 6px rgba(54, 249, 246, 0.12)');
    await fireEvent.click(view.getByRole('button', { name: 'Edit theme', exact: true }));
    await fireEvent.click(view.getByRole('button', { name: 'Reset colors', exact: true }));
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    expect(JSON.parse(localStorage.getItem(key)!).customThemes[0].palette['--js-focus-ring']).toBe(saved.startingPalette['--js-focus-ring']);
  });

  it('duplicate snapshots edited colors independently; inactive source deletion preserves duplicate and active selection', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: 'Source' } });
    await fireEvent.input(view.getByRole('textbox', { name: 'Accent hex', exact: true }), { target: { value: '#123456' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    const source = JSON.parse(localStorage.getItem(key)!).customThemes[0];
    await fireEvent.click(view.getByRole('button', { name: 'Duplicate theme', exact: true }));
    await fireEvent.input(view.getByRole('textbox', { name: 'Theme name', exact: true }), { target: { value: 'Independent' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    const duplicated = JSON.parse(localStorage.getItem(key)!);
    expect(duplicated.customThemes).toHaveLength(2);
    const copy = duplicated.customThemes.find((theme: { id: string }) => theme.id !== source.id);
    expect(copy.startingPalette).toEqual(source.palette);
    expect(duplicated.selectedId).toBe(copy.id);
    view.container.querySelector<HTMLDetailsElement>('.theme-saved-list')!.open = true;
    await fireEvent.click(view.getByRole('button', { name: 'Delete Source', exact: true }));
    const dialog = await view.findByRole('alertdialog');
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete theme', exact: true }));
    const after = JSON.parse(localStorage.getItem(key)!);
    expect(after.selectedId).toBe(copy.id); expect(after.customThemes).toEqual([copy]);
  });

  it('externally revised record makes Save fail truthfully while preserving the visible local draft', async () => {
    const { view, name } = await openCopy();
    await fireEvent.input(name, { target: { value: 'Conflict target' } });
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    const original = JSON.parse(localStorage.getItem(key)!).customThemes[0];
    await fireEvent.click(view.getByRole('button', { name: 'Edit theme', exact: true }));
    await fireEvent.input(view.getByRole('textbox', { name: 'Theme name', exact: true }), { target: { value: 'Local edit' } });
    persistCustomTheme({ ...original, label: 'External edit' }, original.revision);
    publishSavedThemeState();
    const before = localStorage.getItem(key);
    await fireEvent.click(view.getByRole('button', { name: 'Save & apply', exact: true }));
    expect(localStorage.getItem(key)).toBe(before);
    expect((view.getByRole('textbox', { name: 'Theme name', exact: true }) as HTMLInputElement).value).toBe('Local edit');
    expect(view.getByRole('alert').textContent).toMatch(/changed|deleted|another/i);
  });
});
