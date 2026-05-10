import {
  readStackTerminal,
  resizeStackTerminal,
  stopStackTerminal,
  writeStackTerminal,
  type StackTerminalOutputChunk,
  type StackTerminalReadResult,
  type StackTerminalSession
} from './stackPopup';
import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';

export type {
  StackTerminalOutputChunk,
  StackTerminalReadResult,
  StackTerminalSession
};

export function startPersistentTerminal(): Promise<StackTerminalSession> {
  return invoke<StackTerminalSession>(IPC_COMMANDS.startPersistentTerminal);
}

export {
  readStackTerminal,
  resizeStackTerminal,
  stopStackTerminal,
  writeStackTerminal
};
