import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const appSource = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const loaderSource = readFileSync(new URL('../src/lib/surfaceLoader.ts', import.meta.url), 'utf8');
const shellSurfaceSource = readFileSync(new URL('../src/lib/shellSurface.ts', import.meta.url), 'utf8');

const expectedSurfaceImports = {
  'top-bar': '../components/TopBar.svelte',
  'bottom-bar': '../components/BottomBar.svelte',
  'task-preview': '../components/TaskPreviewSurface.svelte',
  'search-panel': '../components/SearchPanelSurface.svelte',
  'stack-popup': '../components/StackPopupSurface.svelte',
  'process-manager': '../components/ProcessManagerSurface.svelte',
  'control-plane': '../components/ControlPlaneSurface.svelte',
  'settings-panel': '../components/SettingsPanelSurface.svelte',
  'tray-panel': '../components/TrayPanelSurface.svelte',
  'terminal-panel': '../components/TerminalPanelSurface.svelte',
  'command-panel': '../components/CommandPanelSurface.svelte',
  'audio-panel': '../components/AudioPanelSurface.svelte',
  'calendar-panel': '../components/CalendarPanelSurface.svelte'
};

function shellSurfaceLabels() {
  const typeMatch = shellSurfaceSource.match(/export type ShellSurface =([\s\S]*?);/);
  assert.ok(typeMatch, 'ShellSurface union exists');
  return [...typeMatch[1].matchAll(/\| '([^']+)'/g)]
    .map((match) => match[1])
    .filter((surface) => surface !== 'unknown')
    .sort();
}

test('App lazy-loads routed surfaces instead of statically importing surface components', () => {
  for (const component of Object.values(expectedSurfaceImports).map((path) => path.split('/').at(-1))) {
    assert.doesNotMatch(appSource, new RegExp(`import\\s+\\w+\\s+from ['\"].*${component}`));
  }

  assert.match(appSource, /loadSurfaceComponent\(surface\)/);
  assert.match(appSource, /<SurfaceComponent \/>/);
  assert.match(appSource, /surface === 'unknown'/);
  assert.match(appSource, /console\.error\(`JasonShell failed to load surface component for \$\{surface\}`/);
});

test('surface loader has a dynamic import for every supported shell surface', () => {
  assert.deepEqual(Object.keys(expectedSurfaceImports).sort(), shellSurfaceLabels());
  assert.match(loaderSource, /Record<LoadableShellSurface, SurfaceComponentLoader>/);

  for (const [surface, importPath] of Object.entries(expectedSurfaceImports)) {
    const escapedSurface = surface.replaceAll('-', '[-]');
    assert.match(loaderSource, new RegExp(`['\"]${escapedSurface}['\"]:\\s*\\(\\) => import\\(['\"]${importPath.replaceAll('.', '\\.')}['\"]\\)`));
  }

  assert.match(loaderSource, /if \(surface === 'unknown'\)[\s\S]*return null/);
});
