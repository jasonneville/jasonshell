import type { ShellSurface } from './shellSurface';

export type SurfaceComponent = any;

type SurfaceComponentModule = {
  default: SurfaceComponent;
};

type LoadableShellSurface = Exclude<ShellSurface, 'unknown'>;

export type SurfaceComponentLoader = () => Promise<SurfaceComponentModule>;

export const surfaceComponentLoaders: Record<LoadableShellSurface, SurfaceComponentLoader> = {
  'top-bar': () => import('../components/TopBar.svelte'),
  'bottom-bar': () => import('../components/BottomBar.svelte'),
  'task-gallery': () => import('../components/TaskGallerySurface.svelte'),
  'task-preview': () => import('../components/TaskPreviewSurface.svelte'),
  'search-panel': () => import('../components/SearchPanelSurface.svelte'),
  'stack-popup': () => import('../components/StackPopupSurface.svelte'),
  'process-manager': () => import('../components/ProcessManagerSurface.svelte'),
  'quick-launch-panel': () => import('../components/QuickLaunchPanelSurface.svelte'),
  'control-plane': () => import('../components/ControlPlaneSurface.svelte'),
  'settings-panel': () => import('../components/SettingsPanelSurface.svelte'),
  'tray-panel': () => import('../components/TrayPanelSurface.svelte'),
  'terminal-panel': () => import('../components/TerminalPanelSurface.svelte'),
  'command-panel': () => import('../components/CommandPanelSurface.svelte'),
  'audio-panel': () => import('../components/AudioPanelSurface.svelte'),
  'calendar-panel': () => import('../components/CalendarPanelSurface.svelte')
};

export function loadSurfaceComponent(surface: ShellSurface): Promise<SurfaceComponentModule> | null {
  if (surface === 'unknown') {
    return null;
  }

  return surfaceComponentLoaders[surface]();
}
