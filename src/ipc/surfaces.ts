export const SHELL_SURFACES = {
  snipOverlay: 'snip-overlay',
  snipPreview: 'snip-preview',
  topBar: 'top-bar',
  bottomBar: 'bottom-bar',
  taskGallery: 'task-gallery',
  taskPreview: 'task-preview',
  searchPanel: 'search-panel',
  stackPopup: 'stack-popup',
  processManager: 'process-manager',
  quickLaunchPanel: 'quick-launch-panel',
  controlPlane: 'control-plane',
  settingsPanel: 'settings-panel',
  trayPanel: 'tray-panel',
  terminalPanel: 'terminal-panel',
  commandPanel: 'command-panel',
  audioPanel: 'audio-panel',
  speechHistoryPanel: 'speech-history-panel',
  calendarPanel: 'calendar-panel',
  speechIndicator: 'speech-indicator',
  contextMenuOverlay: 'context-menu-overlay'
} as const;

export type KnownShellSurface = (typeof SHELL_SURFACES)[keyof typeof SHELL_SURFACES];
export type ShellSurface = KnownShellSurface | 'unknown';

export const SHELL_SURFACE_TITLES: Record<KnownShellSurface, string> = {
  [SHELL_SURFACES.snipOverlay]: 'Screen snip',
  [SHELL_SURFACES.snipPreview]: 'Screen snip preview',
  [SHELL_SURFACES.topBar]: 'JasonShell Top Bar',
  [SHELL_SURFACES.bottomBar]: 'JasonShell Taskbar',
  [SHELL_SURFACES.taskGallery]: 'Task Gallery',
  [SHELL_SURFACES.taskPreview]: 'Task Preview',
  [SHELL_SURFACES.searchPanel]: 'Search Panel',
  [SHELL_SURFACES.stackPopup]: 'Stack Browser',
  [SHELL_SURFACES.processManager]: 'Process Manager',
  [SHELL_SURFACES.quickLaunchPanel]: 'Quick Launch Panel',
  [SHELL_SURFACES.controlPlane]: 'Control Plane',
  [SHELL_SURFACES.settingsPanel]: 'Settings Panel',
  [SHELL_SURFACES.trayPanel]: 'Tray Panel',
  [SHELL_SURFACES.terminalPanel]: 'Terminal Panel',
  [SHELL_SURFACES.commandPanel]: 'Command Panel',
  [SHELL_SURFACES.audioPanel]: 'Audio Panel',
  [SHELL_SURFACES.speechHistoryPanel]: 'Speech History Panel',
  [SHELL_SURFACES.calendarPanel]: 'Calendar Panel',
  [SHELL_SURFACES.speechIndicator]: 'Speech Indicator',
  [SHELL_SURFACES.contextMenuOverlay]: 'Context Menu Overlay'
};

export function isKnownShellSurface(label: string): label is KnownShellSurface {
  return Object.values(SHELL_SURFACES).includes(label as KnownShellSurface);
}
