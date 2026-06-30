export type TerminalActionId =
  | 'copySelection'
  | 'copyCommand'
  | 'copyCommandOutput'
  | 'rerunCommand'
  | 'search'
  | 'clear'
  | 'openCwdInFiles'
  | 'openExternalTerminalHere'
  | 'restartTerminal'
  | 'stopTerminal'
  | 'paste'
  | 'openDetectedTarget'
  | 'copyDetectedTarget'
  | 'openInVscode'
  | 'openGitWorkbench'
  | 'newSession'
  | 'renameSession'
  | 'splitHorizontal'
  | 'splitVertical'
  | 'closePane'
  | 'focusNextPane'
  | 'focusPreviousPane';

export type TerminalActionState = {
  hasTerminal: boolean;
  hasSession: boolean;
  hasSelection?: boolean;
  hasClipboard?: boolean;
  hasCommand?: boolean;
  hasCommandOutput?: boolean;
  hasCwd?: boolean;
  hasDetectedTarget?: boolean;
  hasRepo?: boolean;
  canCreateSession?: boolean;
  canSplit?: boolean;
  hasMultiplePanes?: boolean;
};

export type TerminalAction = {
  id: TerminalActionId;
  label: string;
  shortcut?: string;
  destructive?: boolean;
  isEnabled: (state: TerminalActionState) => boolean;
};

const canUseTerminal = (state: TerminalActionState) => state.hasTerminal && state.hasSession;

export const terminalActions: TerminalAction[] = [
  { id: 'copySelection', label: 'Copy', shortcut: 'Ctrl+C', isEnabled: (state) => Boolean(state.hasSelection) },
  { id: 'copyCommand', label: 'Copy command', isEnabled: (state) => Boolean(state.hasCommand) },
  { id: 'copyCommandOutput', label: 'Copy command output', shortcut: 'Ctrl+Shift+C', isEnabled: (state) => Boolean(state.hasCommandOutput) },
  { id: 'rerunCommand', label: 'Rerun command', isEnabled: (state) => canUseTerminal(state) && Boolean(state.hasCommand) },
  { id: 'search', label: 'Search', shortcut: 'Ctrl+F', isEnabled: () => true },
  { id: 'clear', label: 'Clear', isEnabled: (state) => Boolean(state.hasTerminal) },
  { id: 'openCwdInFiles', label: 'Reveal cwd in Files', isEnabled: (state) => Boolean(state.hasCwd) },
  { id: 'openExternalTerminalHere', label: 'Open external terminal here', isEnabled: (state) => Boolean(state.hasCwd) },
  { id: 'restartTerminal', label: 'Restart terminal', destructive: true, isEnabled: () => true },
  { id: 'stopTerminal', label: 'Stop terminal', destructive: true, isEnabled: (state) => Boolean(state.hasSession) },
  { id: 'paste', label: 'Paste', shortcut: 'Ctrl+V', isEnabled: (state) => canUseTerminal(state) },
  { id: 'openDetectedTarget', label: 'Open target', isEnabled: (state) => Boolean(state.hasDetectedTarget) },
  { id: 'copyDetectedTarget', label: 'Copy target', isEnabled: (state) => Boolean(state.hasDetectedTarget) },
  { id: 'openInVscode', label: 'Open cwd in VS Code', isEnabled: (state) => Boolean(state.hasCwd) },
  { id: 'openGitWorkbench', label: 'Open Git workbench', isEnabled: (state) => Boolean(state.hasRepo || state.hasCwd) },
  { id: 'newSession', label: 'New session', isEnabled: (state) => state.canCreateSession !== false },
  { id: 'renameSession', label: 'Rename session', isEnabled: (state) => Boolean(state.hasSession) },
  { id: 'splitHorizontal', label: 'Split down', isEnabled: () => true },
  { id: 'splitVertical', label: 'Split right', isEnabled: () => true },
  { id: 'closePane', label: 'Close pane', destructive: true, isEnabled: (state) => Boolean(state.hasSession) },
  { id: 'focusNextPane', label: 'Focus next pane', isEnabled: (state) => Boolean(state.hasMultiplePanes) },
  { id: 'focusPreviousPane', label: 'Focus previous pane', isEnabled: (state) => Boolean(state.hasMultiplePanes) }
];

export function getTerminalAction(id: TerminalActionId) {
  return terminalActions.find((action) => action.id === id);
}

export function enabledTerminalActions(state: TerminalActionState) {
  return terminalActions.filter((action) => action.isEnabled(state));
}
