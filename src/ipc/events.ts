export const IPC_EVENTS = {
  processManagerOpen: 'process-manager:open',
  processManagerClosed: 'process-manager:closed',
  searchPanelUpdate: 'search-panel:update',
  searchPanelActivate: 'search-panel:activate',
  searchPanelSelect: 'search-panel:select',
  searchPanelPinFolder: 'search-panel:pin-folder',
  searchIndexRefreshed: 'search-index:refreshed',
  stackPopupOpen: 'stack-popup:open',
  stackPinsUpdated: 'stack-pins:updated',
  taskbarRefreshLaunchers: 'taskbar:refresh-launchers',
  taskbarRefreshWindows: 'taskbar:refresh-windows',
  taskPreviewHoverEnter: 'task-preview:hover-enter',
  taskPreviewUpdate: 'task-preview:update',
  taskPreviewHide: 'task-preview:hide',
  topBarPinMenuAction: 'top-bar:pin-menu-action'
} as const;

export type IpcEventName = (typeof IPC_EVENTS)[keyof typeof IPC_EVENTS];
