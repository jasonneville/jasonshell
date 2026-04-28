export const SHELL_SURFACES = {
  topBar: 'top-bar',
  bottomBar: 'bottom-bar',
  taskPreview: 'task-preview',
  searchPanel: 'search-panel',
  stackPopup: 'stack-popup',
  processManager: 'process-manager',
  controlPlane: 'control-plane'
} as const;

export type KnownShellSurface = (typeof SHELL_SURFACES)[keyof typeof SHELL_SURFACES];
export type ShellSurface = KnownShellSurface | 'unknown';

export const SHELL_SURFACE_TITLES: Record<KnownShellSurface, string> = {
  [SHELL_SURFACES.topBar]: 'JasonShell Top Bar',
  [SHELL_SURFACES.bottomBar]: 'JasonShell Taskbar',
  [SHELL_SURFACES.taskPreview]: 'Task Preview',
  [SHELL_SURFACES.searchPanel]: 'Search Panel',
  [SHELL_SURFACES.stackPopup]: 'Stack Browser',
  [SHELL_SURFACES.processManager]: 'Process Manager',
  [SHELL_SURFACES.controlPlane]: 'Control Plane'
};

export function isKnownShellSurface(label: string): label is KnownShellSurface {
  return Object.values(SHELL_SURFACES).includes(label as KnownShellSurface);
}
