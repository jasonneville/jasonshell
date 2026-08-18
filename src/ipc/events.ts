export const IPC_EVENTS = {
  // Convenience subset for shared cross-window frontend wrappers. Rust event
  // authority lives in src-tauri/src/contracts.rs; feature-local constants may
  // exist for events not consumed across generic wrappers.
  audioPanelOpen: 'audio-panel:open',
  audioPanelClosed: 'audio-panel:closed',
  processManagerOpen: 'process-manager:open',
  processManagerClosed: 'process-manager:closed',
  trayPanelOpen: 'tray-panel:open',
  trayPanelClosed: 'tray-panel:closed',
  terminalPanelOpen: 'terminal-panel:open',
  terminalPanelClosed: 'terminal-panel:closed',
  commandPanelClosed: 'command-panel:closed',
  calendarPanelOpen: 'calendar-panel:open',
  calendarPanelClosed: 'calendar-panel:closed',
  searchPanelUpdate: 'search-panel:update',
  searchPanelActivate: 'search-panel:activate',
  searchPanelSelect: 'search-panel:select',
  searchPanelPinFolder: 'search-panel:pin-folder',
  searchPanelInteraction: 'search-panel:interaction',
  searchPanelClosed: 'search-panel:closed',
  searchIndexRefreshed: 'search-index:refreshed',
  quickCommandRunUpdated: 'quick-command:run-updated',
  quickLaunchPanelOpen: 'quick-launch-panel:open',
  quickLaunchPanelClosed: 'quick-launch-panel:closed',
  stackPopupOpen: 'stack-popup:open',
  stackTerminalClosed: 'stack-terminal:closed',
  stackTerminalCwd: 'stack-terminal:cwd',
  stackTerminalOutput: 'stack-terminal:output',
  stackPinsUpdated: 'stack-pins:updated',
  taskStarted: 'task:started',
  taskOutput: 'task:output',
  taskCompleted: 'task:completed',
  taskbarRefreshLaunchers: 'taskbar:refresh-launchers',
  taskbarRefreshWindows: 'taskbar:refresh-windows',
  taskPreviewHoverEnter: 'task-preview:hover-enter',
  taskPreviewUpdate: 'task-preview:update',
  taskPreviewHide: 'task-preview:hide',
  topBarPinMenuAction: 'top-bar:pin-menu-action'
} as const;

export type IpcEventName = (typeof IPC_EVENTS)[keyof typeof IPC_EVENTS];
