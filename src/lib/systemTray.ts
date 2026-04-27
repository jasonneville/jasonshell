import { invoke } from '@tauri-apps/api/core';

export type SystemTrayMouseButton = 'left' | 'right';

export interface SystemTrayIconSnapshot {
  id: string;
  commandId: number;
  index: number;
  label: string;
  iconDataUrl: string;
  hasNativeIcon?: boolean;
}

export interface InvokeSystemTrayIconRequest {
  id: string;
  button: SystemTrayMouseButton;
}

export const EMPTY_TRAY_ICON_DATA_URL = 'data:image/gif;base64,R0lGODlhAQABAAAAACw=';

export function normalizeTrayIcons(icons: SystemTrayIconSnapshot[]) {
  const seen = new Set<string>();
  const normalized: SystemTrayIconSnapshot[] = [];
  for (const icon of icons) {
    if (!icon.id || seen.has(icon.id)) {
      continue;
    }
    seen.add(icon.id);
    normalized.push({
      ...icon,
      label: icon.label?.trim() || `Notification area icon ${icon.index + 1}`,
      iconDataUrl: icon.iconDataUrl || EMPTY_TRAY_ICON_DATA_URL,
      hasNativeIcon: Boolean(icon.hasNativeIcon && icon.iconDataUrl)
    });
  }
  return normalized;
}

export function trayClickRequest(id: string, button: SystemTrayMouseButton): InvokeSystemTrayIconRequest {
  return { id, button };
}

export async function listSystemTrayIcons() {
  return normalizeTrayIcons(await invoke<SystemTrayIconSnapshot[]>('list_system_tray_icons'));
}

export async function invokeSystemTrayIcon(request: InvokeSystemTrayIconRequest) {
  await invoke('invoke_system_tray_icon', { request });
}
