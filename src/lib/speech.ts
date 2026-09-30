import { invoke } from '@tauri-apps/api/core';
import { listen, type Event, type UnlistenFn } from '@tauri-apps/api/event';
import {
  IPC_COMMANDS,
  type CopySpeechHistoryTranscriptRequest,
  type StartSpeechCaptureResponse,
  type StartSpeechCaptureRequest,
  type StopSpeechCaptureRequest
} from '../ipc/commands.js';
import {
  IPC_EVENTS,
  type SpeechHistoryEntry,
  type SpeechStatusEvent,
  type SpeechStatusResponse
} from '../ipc/events.js';

export const SPEECH_STATUS_CHANGED_EVENT = IPC_EVENTS.speechStatusChanged;
export const SPEECH_HISTORY_PANEL_OPEN_EVENT = IPC_EVENTS.speechHistoryPanelOpen;
export const SPEECH_HISTORY_PANEL_CLOSED_EVENT = IPC_EVENTS.speechHistoryPanelClosed;

export interface ShowSpeechHistoryPanelRequest {
  anchorLeft: number;
  anchorWidth: number;
}

export function startSpeechCapture(reservation: StartSpeechCaptureRequest): Promise<StartSpeechCaptureResponse> {
  const request = reservation;
  return invoke<StartSpeechCaptureResponse>(IPC_COMMANDS.startSpeechCapture, { request });
}

export function captureSpeechPasteTarget(capture = true): Promise<number> {
  return invoke<number>(IPC_COMMANDS.captureSpeechPasteTarget, { capture });
}

export function stopSpeechCapture(request: StopSpeechCaptureRequest): Promise<SpeechStatusResponse> {
  return invoke<SpeechStatusResponse>(IPC_COMMANDS.stopSpeechCapture, { request });
}

export function getSpeechHistory(): Promise<SpeechHistoryEntry[]> {
  return invoke<SpeechHistoryEntry[]>(IPC_COMMANDS.getSpeechHistory);
}

export function copySpeechHistoryTranscript(
  request: CopySpeechHistoryTranscriptRequest
): Promise<void> {
  return invoke(IPC_COMMANDS.copySpeechHistoryTranscript, { request });
}

export function showSpeechHistoryPanel(request: ShowSpeechHistoryPanelRequest): Promise<void> {
  return invoke(IPC_COMMANDS.showSpeechHistoryPanel, { request });
}

export function hideSpeechHistoryPanel(): Promise<void> {
  return invoke(IPC_COMMANDS.hideSpeechHistoryPanel);
}

// SpeechStatusEvent preserves legacy payloads while typing optional finalizationReason metadata.
export function listenSpeechStatus(
  handler: (event: SpeechStatusEvent) => void
): Promise<UnlistenFn> {
  return listen<SpeechStatusEvent>(SPEECH_STATUS_CHANGED_EVENT, (event: Event<SpeechStatusEvent>) => {
    handler(event.payload);
  });
}
