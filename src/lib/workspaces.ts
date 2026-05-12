import { invoke } from '@tauri-apps/api/core';
import { IPC_COMMANDS } from '../ipc/commands.js';
import type { SearchPanelResult } from './searchPanel';
import type { StackPin } from './stackPopup';

export type WorkspacePinKind = 'folder' | 'file';
export type WorkspaceEnvValueSource = 'literal' | 'inherited' | 'prompt';
export type WorkspaceStartupMode = 'manualOnly' | 'suggestOnly';

export interface WorkspaceToolDefaults {
  terminal?: string | null;
  editor?: string | null;
  shell?: string | null;
}

export interface WorkspacePin {
  id: string;
  label: string;
  path: string;
  kind: WorkspacePinKind;
}

export interface WorkspaceEnvDeclaration {
  name: string;
  value?: string | null;
  valueSource: WorkspaceEnvValueSource;
}

export interface WorkspaceTaskDeclaration {
  id: string;
  name: string;
  command: string;
  args: string[];
  cwd?: string | null;
  env: WorkspaceEnvDeclaration[];
  exposeInSearch: boolean;
  pinned: boolean;
}

export interface WorkspaceStartupCommand {
  id: string;
  label: string;
  command: string;
  args: string[];
  cwd?: string | null;
  env: WorkspaceEnvDeclaration[];
}

export interface WorkspaceStartupSafety {
  mode: WorkspaceStartupMode;
  taskIds: string[];
  commands: WorkspaceStartupCommand[];
  env: WorkspaceEnvDeclaration[];
}

export interface WorkspaceRestorationReservation {
  status: 'reserved-not-implemented';
}

export interface WorkspaceProfile {
  id: string;
  name: string;
  rootPath: string;
  aliases: string[];
  pins: WorkspacePin[];
  toolDefaults: WorkspaceToolDefaults;
  tasks: WorkspaceTaskDeclaration[];
  startup: WorkspaceStartupSafety;
  restoration: WorkspaceRestorationReservation;
}

export interface WorkspaceActivationPin {
  id: string;
  label: string;
  path: string;
  workspaceId: string;
}

export interface WorkspaceActivationTask {
  id: string;
  name: string;
  command: string;
  args: string[];
  cwd?: string | null;
  pinned: boolean;
  willExecuteOnActivation: boolean;
}

export interface WorkspaceActivationPlan {
  workspace: WorkspaceProfile;
  layout: {
    activeWorkspaceId: string;
    rootPath: string;
    aliases: string[];
    windowAppRestorationStatus: 'reserved-not-implemented';
  };
  search: {
    biasRoots: string[];
    aliases: string[];
    resultBoost: number;
  };
  pins: {
    topBar: WorkspaceActivationPin[];
  };
  tasks: {
    exposed: WorkspaceActivationTask[];
  };
  startup: {
    mode: WorkspaceStartupMode;
    willExecute: false;
    reason: string;
    taskIds: string[];
    commands: WorkspaceStartupCommand[];
    env: WorkspaceEnvDeclaration[];
  };
  restoration: WorkspaceRestorationReservation;
}

export const WORKSPACE_COMMANDS = {
  list: IPC_COMMANDS.listWorkspaces,
  create: IPC_COMMANDS.createWorkspace,
  update: IPC_COMMANDS.updateWorkspace,
  delete: IPC_COMMANDS.deleteWorkspace,
  activate: IPC_COMMANDS.activateWorkspace
} as const;

export function listWorkspaces(): Promise<WorkspaceProfile[]> {
  return invoke(WORKSPACE_COMMANDS.list);
}

export function createWorkspace(workspace: WorkspaceProfile): Promise<WorkspaceProfile> {
  return invoke(WORKSPACE_COMMANDS.create, { workspace });
}

export function updateWorkspace(workspace: WorkspaceProfile): Promise<WorkspaceProfile> {
  return invoke(WORKSPACE_COMMANDS.update, { workspace });
}

export function deleteWorkspace(id: string): Promise<WorkspaceProfile[]> {
  return invoke(WORKSPACE_COMMANDS.delete, { id });
}

export function activateWorkspace(id: string): Promise<WorkspaceActivationPlan> {
  return invoke(WORKSPACE_COMMANDS.activate, { id });
}

export function workspacePinsFromActivationPlan(plan: WorkspaceActivationPlan): StackPin[] {
  return plan.pins.topBar.map((pin) => ({
    id: `workspace:${pin.workspaceId}:${pin.id}`,
    name: pin.label,
    path: pin.path
  }));
}

export function workspaceSearchResults(plan: WorkspaceActivationPlan): SearchPanelResult[] {
  const workspaceTerms = [
    plan.workspace.name,
    plan.workspace.rootPath,
    ...plan.workspace.aliases,
    'workspace project profile activate'
  ].join(' ');
  const workspaceResult: SearchPanelResult = {
    id: `workspace:${plan.workspace.id}`,
    kind: 'command',
    title: plan.workspace.name,
    subtitle: `Active workspace: ${plan.workspace.rootPath}`,
    terms: workspaceTerms,
    priority: 118,
    path: plan.workspace.rootPath
  };
  const pinResults = plan.pins.topBar.map<SearchPanelResult>((pin) => ({
    id: `workspace:${plan.workspace.id}:pin:${pin.id}`,
    kind: 'folder',
    title: pin.label,
    subtitle: `Workspace pin: ${plan.workspace.name}`,
    terms: `${pin.label} ${pin.path} ${workspaceTerms} pin folder workspace`,
    priority: 112,
    path: pin.path
  }));
  const taskResults = plan.tasks.exposed.map<SearchPanelResult>((task) => ({
    id: `workspace:${plan.workspace.id}:task:${task.id}`,
    kind: 'command',
    title: task.name,
    subtitle: `Workspace task: ${task.command} ${task.args.join(' ')}`.trim(),
    terms: `${task.name} ${task.command} ${task.args.join(' ')} ${workspaceTerms} task run`,
    priority: task.pinned ? 116 : 104
  }));
  return [workspaceResult, ...pinResults, ...taskResults];
}

export function applyWorkspaceSearchBias(
  results: SearchPanelResult[],
  plan: WorkspaceActivationPlan | null
): SearchPanelResult[] {
  if (!plan) {
    return results;
  }
  const biasRoots = plan.search.biasRoots.map(normalizePath).filter(Boolean);
  if (!biasRoots.length) {
    return results;
  }
  return results.map((result) => {
    const path = normalizePath(result.path ?? '');
    const isWorkspacePath = Boolean(path && biasRoots.some((root) => path === root || path.startsWith(`${root}\\`)));
    if (!isWorkspacePath) {
      return result;
    }
    return {
      ...result,
      priority: result.priority + plan.search.resultBoost,
      terms: `${result.terms} ${plan.search.aliases.join(' ')} active workspace`
    };
  });
}

export function startupExecutionSummary(plan: WorkspaceActivationPlan): string {
  return plan.startup.willExecute
    ? 'Startup execution enabled'
    : `Startup execution blocked: ${plan.startup.reason}`;
}

function normalizePath(path: string): string {
  return path.trim().replace(/\//g, '\\').replace(/\\+$/u, '').toLocaleLowerCase();
}
