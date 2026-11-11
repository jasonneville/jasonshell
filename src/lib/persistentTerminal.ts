import {
  listStackTerminals,
  pollStackTerminal,
  readStackTerminal,
  renameStackTerminal,
  resizeStackTerminal,
  startStackTerminal,
  stopStackTerminal,
  stopTerminalPanelSessions,
  writeStackTerminal,
  type StackTerminalOutputChunk,
  type StackTerminalPollResult,
  type StackTerminalReadResult,
  type StackTerminalSession
} from './stackPopup';
import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export type {
  StackTerminalOutputChunk,
  StackTerminalPollResult,
  StackTerminalReadResult,
  StackTerminalSession
};

export function startPersistentTerminal(): Promise<StackTerminalSession> {
  return invoke<StackTerminalSession>(IPC_COMMANDS.startPersistentTerminal);
}

export {
  listStackTerminals,
  pollStackTerminal,
  readStackTerminal,
  renameStackTerminal,
  resizeStackTerminal,
  startStackTerminal,
  stopStackTerminal,
  stopTerminalPanelSessions,
  writeStackTerminal
};
