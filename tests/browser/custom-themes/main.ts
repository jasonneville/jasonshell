import '../../../src/app.css';
import { mount, unmount } from 'svelte';
import Settings from '../../../src/components/SettingsPanelSurface.svelte';
import ControlPlane from '../../../src/components/ControlPlaneSurface.svelte';
import BaselineSurfaces from './BaselineSurfaces.svelte';
import { applyShellTheme, applyThemeToElement, storedShellThemeId, installShellThemeSync, SHELL_THEMES } from '../../../src/lib/themes';
import { createCustomTheme, THEME_PALETTE_TOKENS } from '../../../src/lib/themePalette';
import { queryThemePreviewPresence } from '../../../src/lib/themePreview';
import { applyShellPreferences, storedShellPreferences, installShellPreferencesSync } from '../../../src/lib/shellPreferences';
import { ledger, emit } from './bridge';
import { SPEECH_VOICE_LEVEL_EVENT } from '../../../src/lib/speech';

const root = document.documentElement;
const target = document.getElementById('app')!;
const kind = new URLSearchParams(location.search).get('surface') ?? 'settings';
// Same production pre-mount path as src/main.ts; capture before mounting any UI.
applyShellTheme(storedShellThemeId(), { storage: null });
applyShellPreferences(storedShellPreferences(), { storage: null, dispatch: false });
const bootstrap = { theme: root.dataset.theme, scheme: root.style.colorScheme,
  accent: getComputedStyle(root).getPropertyValue('--js-color-accent').trim() };
document.getElementById('bootstrap')!.textContent = JSON.stringify(bootstrap);
const cleanups = [installShellThemeSync(), installShellPreferencesSync()];
const instance = kind === 'settings' ? mount(Settings, { target })
  : kind === 'control' ? mount(ControlPlane, { target })
  : kind === 'baseline' ? mount(BaselineSurfaces, { target }) : null;
document.getElementById('hide-fixture')!.onclick = () => { target.hidden = true; };
document.getElementById('show-fixture')!.onclick = () => { target.hidden = false; };

function paint(token: string) {
  const probe = document.createElement('span');
  const property = /--js-(?:shadow-|focus-ring|inset-highlight)/.test(token) ? 'boxShadow' : 'color';
  probe.style[property] = `var(${token})`;
  document.body.append(probe);
  const resolved = getComputedStyle(probe)[property];
  probe.remove();
  return resolved;
}
function snapshot() {
  return { theme: root.dataset.theme, custom: root.dataset.customTheme, accent: paint('--js-color-accent'),
    focus: paint('--js-focus-ring'), overlay: paint('--js-color-surface-overlay'),
    control: paint('--js-color-control'), surface: paint('--js-bg-surface'),
    saved: localStorage.getItem('jasonshell.themes.v1'), storage: Object.fromEntries(Object.keys(localStorage).sort().map(key => [key, localStorage.getItem(key)])), bootstrap, ledger: [...ledger] };
}

function cloneEquivalence() {
  const failures: unknown[] = [];
  let checked = 0;
  const before = { theme: root.dataset.theme, custom: root.dataset.customTheme, style: root.getAttribute('style'),
    focus: root.dataset.shellFocus, transparency: root.dataset.shellTransparency };
  try {
    for (const preferences of ['default', 'strong', 'reduced']) {
      root.dataset.shellFocus = preferences === 'strong' ? 'strong' : 'normal';
      root.dataset.shellTransparency = preferences === 'reduced' ? 'reduced' : 'normal';
      for (const theme of SHELL_THEMES) {
        applyThemeToElement(theme.id);
        const baseline = Object.fromEntries(THEME_PALETTE_TOKENS.map(token => [token, paint(token)]));
        applyThemeToElement(createCustomTheme(theme.id, 'Appearance clone', 'custom:equivalence'));
        for (const token of THEME_PALETTE_TOKENS) {
          const actual = paint(token); checked++;
          if (actual !== baseline[token]) failures.push({ theme: theme.id, preferences, token, expected: baseline[token], actual });
        }
        if (getComputedStyle(root).colorScheme !== theme.mode) failures.push({ theme: theme.id, field: 'colorScheme' });
      }
    }
  } finally {
    for (const [key, value] of Object.entries({ theme: before.theme, customTheme: before.custom, shellFocus: before.focus, shellTransparency: before.transparency })) {
      if (value === undefined) delete root.dataset[key]; else root.dataset[key] = value;
    }
    if (before.style === null) root.removeAttribute('style'); else root.setAttribute('style', before.style);
  }
  return { presets: SHELL_THEMES.length, tokens: THEME_PALETTE_TOKENS.length, checked, failures };
}

// Small read-only observability affordance; tests still drive production UI.
Object.assign(window, { themeFixture: { snapshot, paint, cloneEquivalence, queryPresence: queryThemePreviewPresence, presetIds: SHELL_THEMES.map(theme => theme.id),
  speechLevel: (level: number) => emit(SPEECH_VOICE_LEVEL_EVENT, { nonce: 1, level }),
  dispose: async () => { if (instance) await unmount(instance); cleanups.forEach(cleanup => cleanup()); } } });
document.getElementById('fixture-result')!.textContent = 'fixture ready';
window.addEventListener('pagehide', () => { cleanups.forEach(cleanup => cleanup()); }, { once: true });
