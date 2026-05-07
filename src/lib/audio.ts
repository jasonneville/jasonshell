import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';
import { IPC_EVENTS } from '../ipc/events.js';

export type AudioRefreshReason =
  | 'device-added'
  | 'device-removed'
  | 'default-changed'
  | 'session-changed';

export type AudioDevice = {
  id: string;
  name: string;
  isDefault?: boolean;
};

export type AudioSession = {
  id: string;
  name: string;
  processId?: number | null;
  volumePercent: number;
};

export type AudioState = {
  masterVolumePercent: number;
  outputDevices: AudioDevice[];
  inputDevices: AudioDevice[];
  defaultOutputDeviceId?: string | null;
  defaultInputDeviceId?: string | null;
  sessions: AudioSession[];
};

export type SetVolumeRequest = {
  volumePercent: number;
};

export type SetAppVolumeRequest = SetVolumeRequest & {
  sessionId: string;
};

export type SetDefaultAudioDeviceRequest = {
  deviceId: string;
};

export interface ShowAudioPanelRequest {
  anchorLeft: number;
  anchorWidth: number;
}

export const AUDIO_PANEL_OPEN_EVENT = IPC_EVENTS.audioPanelOpen;
export const AUDIO_PANEL_CLOSED_EVENT = IPC_EVENTS.audioPanelClosed;

export const AUDIO_COMMANDS = {
  showPanel: IPC_COMMANDS.showAudioPanel,
  hidePanel: IPC_COMMANDS.hideAudioPanel,
  getState: IPC_COMMANDS.getAudioState,
  setMasterVolume: IPC_COMMANDS.setMasterVolume,
  setAppVolume: IPC_COMMANDS.setAppVolume,
  setDefaultInputDevice: IPC_COMMANDS.setDefaultAudioInputDevice,
  setDefaultOutputDevice: IPC_COMMANDS.setDefaultAudioOutputDevice
} as const;

export function normalizeVolumePercent(value: number): number {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.max(0, Math.min(100, Math.round(value)));
}

export function getAudioState(): Promise<AudioState> {
  return invoke<AudioState>(AUDIO_COMMANDS.getState);
}

export function showAudioPanel(request: ShowAudioPanelRequest): Promise<void> {
  return invoke(AUDIO_COMMANDS.showPanel, { request });
}

export function hideAudioPanel(): Promise<void> {
  return invoke(AUDIO_COMMANDS.hidePanel);
}

export function setMasterVolume(volumePercent: number): Promise<AudioState> {
  return invoke<AudioState>(AUDIO_COMMANDS.setMasterVolume, {
    request: { volumePercent: normalizeVolumePercent(volumePercent) } satisfies SetVolumeRequest
  });
}

export function setAppVolume(sessionId: string, volumePercent: number): Promise<AudioState> {
  return invoke<AudioState>(AUDIO_COMMANDS.setAppVolume, {
    request: {
      sessionId,
      volumePercent: normalizeVolumePercent(volumePercent)
    } satisfies SetAppVolumeRequest
  });
}

export function setDefaultAudioInputDevice(deviceId: string): Promise<AudioState> {
  return invoke<AudioState>(AUDIO_COMMANDS.setDefaultInputDevice, {
    request: { deviceId } satisfies SetDefaultAudioDeviceRequest
  });
}

export function setDefaultAudioOutputDevice(deviceId: string): Promise<AudioState> {
  return invoke<AudioState>(AUDIO_COMMANDS.setDefaultOutputDevice, {
    request: { deviceId } satisfies SetDefaultAudioDeviceRequest
  });
}
