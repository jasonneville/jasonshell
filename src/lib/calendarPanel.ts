import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';
import { IPC_EVENTS } from '../ipc/events.js';

export interface ShowCalendarPanelRequest {
  anchorLeft: number;
  anchorWidth: number;
}

export const CALENDAR_PANEL_OPEN_EVENT = IPC_EVENTS.calendarPanelOpen;
export const CALENDAR_PANEL_CLOSED_EVENT = IPC_EVENTS.calendarPanelClosed;

export const CALENDAR_PANEL_COMMANDS = {
  showPanel: IPC_COMMANDS.showCalendarPanel,
  hidePanel: IPC_COMMANDS.hideCalendarPanel
} as const;

export function showCalendarPanel(request: ShowCalendarPanelRequest): Promise<void> {
  return invoke(CALENDAR_PANEL_COMMANDS.showPanel, { request });
}

export function hideCalendarPanel(): Promise<void> {
  return invoke(CALENDAR_PANEL_COMMANDS.hidePanel);
}
