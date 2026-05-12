import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';
import { IPC_EVENTS } from '../ipc/events.js';
import type { WorkspaceProfile, WorkspaceTaskDeclaration } from './workspaces';

export type WorkspaceToolReference = {
  id?: string | null;
  name?: string | null;
  rootPath: string;
};

export type CommandTemplate = {
  executable: string;
  args: string[];
  cwd?: string | null;
};

export type ToolLaunchRequest = {
  workspace: WorkspaceToolReference;
  filePath?: string | null;
  fileLine?: number | null;
  template?: CommandTemplate | null;
};

export type ToolLaunchPlan = {
  executable: string;
  args: string[];
  cwd: string;
  workspaceId?: string | null;
  workspacePath: string;
  targetPath?: string | null;
  usesShell: false;
};

export type GitWorkspaceStatus = {
  isRepository: boolean;
  branch?: string | null;
  upstream?: string | null;
  headOid?: string | null;
  isClean: boolean;
  hasChanges: boolean;
  ahead: number;
  behind: number;
  hasConflicts: boolean;
  isRebasing: boolean;
  isMerging: boolean;
  summary: string;
};

export type WorkspaceTaskRequest = {
  workspaceId?: string | null;
  taskId: string;
};

export type TaskSpawnResponse = {
  taskId: string;
  processId: number;
};

export type TaskHistoryEntry = {
  taskId: string;
  workspaceId?: string | null;
  workspacePath: string;
  label: string;
  executable: string;
  args: string[];
  processId: number;
  startedAtEpochMs: number;
  finishedAtEpochMs?: number | null;
  exitCode?: number | null;
  canceled: boolean;
};

export type TaskProcessMetadata = {
  taskId: string;
  processId: number;
  workspaceId?: string | null;
  workspacePath: string;
  label: string;
  startedAtEpochMs: number;
};

export type TaskOutputEvent = {
  taskId: string;
  stream: 'stdout' | 'stderr';
  chunk: string;
  sequence: number;
  timestampEpochMs: number;
};

export type TaskCompletedEvent = {
  taskId: string;
  exitCode?: number | null;
  canceled: boolean;
  success: boolean;
  timestampEpochMs: number;
};

export const DEV_TOOL_COMMANDS = {
  buildTerminalLaunchPlan: IPC_COMMANDS.buildTerminalLaunchPlan,
  buildEditorLaunchPlan: IPC_COMMANDS.buildEditorLaunchPlan,
  getWorkspaceGitStatus: IPC_COMMANDS.getWorkspaceGitStatus,
  spawnWorkspaceTask: IPC_COMMANDS.spawnWorkspaceTask,
  cancelWorkspaceTask: IPC_COMMANDS.cancelWorkspaceTask,
  listWorkspaceTaskHistory: IPC_COMMANDS.listWorkspaceTaskHistory,
  listJasonshellTaskProcessMetadata: IPC_COMMANDS.listJasonshellTaskProcessMetadata
} as const;

export const TASK_EVENTS = {
  started: IPC_EVENTS.taskStarted,
  output: IPC_EVENTS.taskOutput,
  completed: IPC_EVENTS.taskCompleted
} as const;

export function buildTerminalLaunchPlan(request: ToolLaunchRequest): Promise<ToolLaunchPlan> {
  return invoke(DEV_TOOL_COMMANDS.buildTerminalLaunchPlan, { request });
}

export function buildEditorLaunchPlan(request: ToolLaunchRequest): Promise<ToolLaunchPlan> {
  return invoke(DEV_TOOL_COMMANDS.buildEditorLaunchPlan, { request });
}

export function getWorkspaceGitStatus(path: string): Promise<GitWorkspaceStatus> {
  return invoke(DEV_TOOL_COMMANDS.getWorkspaceGitStatus, { path });
}

export function spawnWorkspaceTask(request: WorkspaceTaskRequest): Promise<TaskSpawnResponse> {
  return invoke(DEV_TOOL_COMMANDS.spawnWorkspaceTask, { request });
}

export function cancelWorkspaceTask(taskId: string): Promise<void> {
  return invoke(DEV_TOOL_COMMANDS.cancelWorkspaceTask, { taskId });
}

export function listWorkspaceTaskHistory(): Promise<TaskHistoryEntry[]> {
  return invoke(DEV_TOOL_COMMANDS.listWorkspaceTaskHistory);
}

export function listJasonshellTaskProcessMetadata(): Promise<TaskProcessMetadata[]> {
  return invoke(DEV_TOOL_COMMANDS.listJasonshellTaskProcessMetadata);
}

export function workspaceToolReference(workspace: WorkspaceProfile): WorkspaceToolReference {
  return {
    id: workspace.id,
    name: workspace.name,
    rootPath: workspace.rootPath
  };
}

export function terminalLaunchRequestFromWorkspace(workspace: WorkspaceProfile): ToolLaunchRequest {
  return {
    workspace: workspaceToolReference(workspace),
    template: terminalTemplateFromDefault(workspace.toolDefaults.terminal ?? null)
  };
}

export function editorLaunchRequestFromWorkspace(
  workspace: WorkspaceProfile,
  filePath?: string | null,
  fileLine?: number | null
): ToolLaunchRequest {
  return {
    workspace: workspaceToolReference(workspace),
    filePath,
    fileLine,
    template: editorTemplateFromDefault(workspace.toolDefaults.editor ?? null, Boolean(filePath))
  };
}

export function workspaceTaskRequest(
  workspace: WorkspaceProfile,
  task: WorkspaceTaskDeclaration
): WorkspaceTaskRequest {
  if (!workspace.tasks.some((declared) => declared.id === task.id)) {
    throw new Error(`Workspace task is not declared: ${task.id}`);
  }
  return {
    workspaceId: workspace.id,
    taskId: task.id
  };
}

export function rankTaskHistoryForWorkspace(
  entries: TaskHistoryEntry[],
  workspaceId: string,
  limit = 6
): TaskHistoryEntry[] {
  return entries
    .filter((entry) => entry.workspaceId === workspaceId)
    .sort((left, right) => right.startedAtEpochMs - left.startedAtEpochMs)
    .slice(0, limit);
}

function terminalTemplateFromDefault(value: string | null): CommandTemplate | null {
  const normalized = value?.trim().toLocaleLowerCase();
  if (!normalized) {
    return null;
  }
  if (normalized.includes('windows terminal') || normalized === 'wt' || normalized === 'wt.exe') {
    return { executable: 'wt.exe', args: ['-d', '{workspacePath}'], cwd: '{workspacePath}' };
  }
  if (normalized.includes('powershell') || normalized === 'pwsh') {
    return { executable: 'pwsh.exe', args: ['-NoLogo'], cwd: '{workspacePath}' };
  }
  return null;
}

function editorTemplateFromDefault(value: string | null, hasFile: boolean): CommandTemplate | null {
  const normalized = value?.trim().toLocaleLowerCase();
  if (!normalized) {
    return null;
  }
  if (normalized.includes('vs code') || normalized === 'code' || normalized === 'code.exe') {
    return {
      executable: 'code',
      args: hasFile ? ['--goto', '{filePath}:{fileLine}'] : ['{workspacePath}'],
      cwd: '{workspacePath}'
    };
  }
  return null;
}
