import { CURRENT_SETTINGS_VERSION, SETTINGS_SCHEMA, type ShellSettings } from '../../lib/settings.js';
import type { GitWorkspaceStatus, TaskHistoryEntry, TaskProcessMetadata } from '../../lib/devTools';
import type { ProcessInfo } from '../../lib/processManager';
import { formatProcessCpu, formatProcessMemory } from '../../lib/processManagerState.js';
import type {
  DeveloperSearchProviderGroup,
  DeveloperSearchResponse
} from '../search/developerProviders';
import type { WorkspaceProfile } from '../../lib/workspaces';

export type ControlPlaneSectionId =
  | 'settings'
  | 'workspaces'
  | 'git'
  | 'tasks'
  | 'processes'
  | 'providers';

export type ControlPlaneLoadState = 'idle' | 'loading' | 'ready' | 'stale' | 'error';
export type ControlPlaneItemTone = 'neutral' | 'good' | 'warning' | 'danger';
export type ControlPlaneKeyAction =
  | 'none'
  | 'activate-section'
  | 'close-panel'
  | 'focus-first-section'
  | 'focus-last-section'
  | 'focus-next-section'
  | 'focus-previous-section'
  | 'refresh-active-section';

export interface ControlPlaneAction {
  id: string;
  label: string;
  ariaLabel: string;
  disabled?: boolean;
}

export interface ControlPlaneItem {
  id: string;
  title: string;
  meta: string;
  detail?: string;
  tone: ControlPlaneItemTone;
}

export interface ControlPlaneSection {
  id: ControlPlaneSectionId;
  label: string;
  description: string;
  status: string;
  count: number;
  ariaLabel: string;
  items: ControlPlaneItem[];
  actions: ControlPlaneAction[];
}

export interface ControlPlaneProviderBudget {
  perProviderLimit: number;
  totalLimit: number;
}

export interface ControlPlaneInput {
  settings?: ShellSettings | null;
  settingsState?: ControlPlaneLoadState;
  workspaces?: readonly WorkspaceProfile[];
  activeWorkspaceId?: string | null;
  gitStatuses?: Readonly<Record<string, GitWorkspaceStatus | undefined>>;
  taskHistory?: readonly TaskHistoryEntry[];
  taskProcessMetadata?: readonly TaskProcessMetadata[];
  processes?: readonly ProcessInfo[];
  providerResponse?: DeveloperSearchResponse | null;
  providerBudget?: Partial<ControlPlaneProviderBudget>;
  activeSectionId?: ControlPlaneSectionId;
  itemLimit?: number;
}

export interface ControlPlaneViewModel {
  activeSectionId: ControlPlaneSectionId;
  sections: ControlPlaneSection[];
  totals: {
    settingsLoaded: boolean;
    workspaceCount: number;
    dirtyRepositoryCount: number;
    runningTaskCount: number;
    processCount: number;
    providerResultCount: number;
  };
}

const DEFAULT_ITEM_LIMIT = 6;
const DEFAULT_PROVIDER_BUDGET: ControlPlaneProviderBudget = {
  perProviderLimit: 8,
  totalLimit: 40
};

const SECTION_ORDER: ControlPlaneSectionId[] = [
  'settings',
  'workspaces',
  'git',
  'tasks',
  'processes',
  'providers'
];

export function buildControlPlaneViewModel(input: ControlPlaneInput): ControlPlaneViewModel {
  const itemLimit = positiveInteger(input.itemLimit, DEFAULT_ITEM_LIMIT);
  const settings = input.settings ?? null;
  const workspaces = [...(input.workspaces ?? settings?.workspaces ?? [])];
  const activeWorkspaceId = input.activeWorkspaceId ?? settings?.ui.activeWorkspaceId ?? null;
  const gitStatuses = input.gitStatuses ?? {};
  const taskHistory = [...(input.taskHistory ?? [])];
  const taskProcessMetadata = [...(input.taskProcessMetadata ?? [])];
  const processes = [...(input.processes ?? [])];
  const providerResponse = input.providerResponse ?? null;
  const providerBudget = {
    ...DEFAULT_PROVIDER_BUDGET,
    ...input.providerBudget
  };

  const sections = [
    settingsSection(settings, input.settingsState ?? (settings ? 'ready' : 'idle'), activeWorkspaceId),
    workspacesSection(workspaces, activeWorkspaceId, itemLimit),
    gitSection(workspaces, gitStatuses, itemLimit),
    tasksSection(taskHistory, taskProcessMetadata, itemLimit),
    processesSection(processes, itemLimit),
    providersSection(providerResponse, providerBudget, itemLimit)
  ];

  const activeSectionId = input.activeSectionId && sections.some((section) => section.id === input.activeSectionId)
    ? input.activeSectionId
    : sections[0].id;

  return {
    activeSectionId,
    sections,
    totals: {
      settingsLoaded: Boolean(settings),
      workspaceCount: workspaces.length,
      dirtyRepositoryCount: Object.values(gitStatuses).filter((status) => status?.hasChanges).length,
      runningTaskCount: taskProcessMetadata.length,
      processCount: processes.length,
      providerResultCount: providerResponse?.results.length ?? 0
    }
  };
}

export function filterControlPlaneSections(
  sections: readonly ControlPlaneSection[],
  query: string
): ControlPlaneSection[] {
  const tokens = normalizeText(query).split(' ').filter(Boolean);
  if (!tokens.length) {
    return [...sections];
  }

  return sections.filter((section) => {
    const haystack = normalizeText([
      section.id,
      section.label,
      section.description,
      section.status,
      ...section.items.flatMap((item) => [item.title, item.meta, item.detail ?? ''])
    ].join(' '));
    return tokens.every((token) => haystack.includes(token));
  });
}

export function nextControlPlaneSectionId(
  sections: readonly ControlPlaneSection[],
  currentId: ControlPlaneSectionId,
  direction: 'previous' | 'next'
): ControlPlaneSectionId {
  if (!sections.length) {
    return currentId;
  }

  const currentIndex = Math.max(0, sections.findIndex((section) => section.id === currentId));
  const delta = direction === 'next' ? 1 : -1;
  const nextIndex = (currentIndex + delta + sections.length) % sections.length;
  return sections[nextIndex].id;
}

export function controlPlaneKeyActionFromEvent(event: {
  key: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
}): ControlPlaneKeyAction {
  if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase() === 'r') {
    return 'refresh-active-section';
  }

  switch (event.key) {
    case 'ArrowDown':
    case 'ArrowRight':
      return 'focus-next-section';
    case 'ArrowLeft':
    case 'ArrowUp':
      return 'focus-previous-section';
    case 'Home':
      return 'focus-first-section';
    case 'End':
      return 'focus-last-section';
    case 'Enter':
    case ' ':
      return 'activate-section';
    case 'Escape':
      return 'close-panel';
    default:
      return 'none';
  }
}

export function controlPlaneSectionTabLabel(section: ControlPlaneSection, selected: boolean): string {
  const selectedLabel = selected ? 'selected' : 'not selected';
  return `${section.label}, ${section.count} ${pluralize('item', section.count)}, ${section.status}, ${selectedLabel}`;
}

export function controlPlaneActionLabel(section: ControlPlaneSection, action: ControlPlaneAction): string {
  return `${action.label}: ${section.label}`;
}

function settingsSection(
  settings: ShellSettings | null,
  state: ControlPlaneLoadState,
  activeWorkspaceId: string | null
): ControlPlaneSection {
  const loaded = Boolean(settings);
  const version = settings?.version ?? CURRENT_SETTINGS_VERSION;
  const status = state === 'error'
    ? 'Persistence unavailable'
    : loaded
      ? `Settings loaded from ${SETTINGS_SCHEMA} v${version}`
      : 'Settings persistence not loaded';

  return section('settings', 'Settings', 'Persistence and shell configuration health', status, [
    item('settings-schema', 'Schema', SETTINGS_SCHEMA, `Current frontend contract version ${CURRENT_SETTINGS_VERSION}`, 'neutral'),
    item(
      'settings-version',
      'Persistence version',
      `v${version}`,
      version === CURRENT_SETTINGS_VERSION ? 'Matches frontend contract' : 'Version mismatch requires migration',
      version === CURRENT_SETTINGS_VERSION ? 'good' : 'warning'
    ),
    item(
      'settings-active-workspace',
      'Active workspace',
      activeWorkspaceId ? 'Configured' : 'None selected',
      activeWorkspaceId ? safeBoundedText(activeWorkspaceId) : 'No active workspace is persisted',
      activeWorkspaceId ? 'good' : 'neutral'
    ),
    item(
      'settings-diagnostics',
      'Diagnostics export',
      settings?.ui.enableDiagnosticsExport ? 'Enabled' : 'Disabled',
      'Only the explicit diagnostics flag is shown; secret-like fields are not rendered',
      settings?.ui.enableDiagnosticsExport ? 'warning' : 'neutral'
    ),
    item('settings-secrets', 'Secret display', 'Blocked', 'Control-plane summaries never render persisted secret values', 'good')
  ], [
    action('refresh-settings', 'Refresh', 'Refresh shell settings persistence status', state === 'loading'),
    action('save-settings', 'Save', 'Save the current non-secret shell settings snapshot', !loaded)
  ]);
}

function workspacesSection(
  workspaces: readonly WorkspaceProfile[],
  activeWorkspaceId: string | null,
  itemLimit: number
): ControlPlaneSection {
  const ordered = [...workspaces].sort((left, right) => {
    if (left.id === activeWorkspaceId) {
      return -1;
    }
    if (right.id === activeWorkspaceId) {
      return 1;
    }
    return left.name.localeCompare(right.name, undefined, { sensitivity: 'base' });
  });

  const items = boundItems(ordered, itemLimit).map((workspace) => item(
    `workspace-${workspace.id}`,
    workspace.name,
    workspace.id === activeWorkspaceId ? 'Active workspace' : 'Workspace profile',
    `${workspace.rootPath} • ${workspace.tasks.length} ${pluralize('task', workspace.tasks.length)} • ${workspace.pins.length} ${pluralize('pin', workspace.pins.length)}`,
    workspace.id === activeWorkspaceId ? 'good' : 'neutral'
  ));

  return section(
    'workspaces',
    'Workspaces',
    'Workspace profiles, pins, declared task counts, and planning-only workspace startup and restoration',
    workspaces.length ? `${workspaces.length} workspace ${pluralize('profile', workspaces.length)}` : 'No workspaces configured',
    addOverflowItem(items, workspaces.length, itemLimit, 'workspace'),
    [action('activate-workspace', 'Activate', 'Return the selected workspace activation plan; startup commands and restoration are not executed', workspaces.length === 0)]
  );
}

function gitSection(
  workspaces: readonly WorkspaceProfile[],
  gitStatuses: Readonly<Record<string, GitWorkspaceStatus | undefined>>,
  itemLimit: number
): ControlPlaneSection {
  const statusItems = workspaces.map((workspace) => {
    const status = gitStatuses[workspace.id] ?? gitStatuses[workspace.rootPath];
    if (!status) {
      return item(`git-${workspace.id}`, workspace.name, 'Git status not loaded', workspace.rootPath, 'neutral');
    }
    const tone: ControlPlaneItemTone = status.hasConflicts || status.isRebasing || status.isMerging
      ? 'danger'
      : status.hasChanges || status.ahead > 0 || status.behind > 0
        ? 'warning'
        : 'good';
    return item(
      `git-${workspace.id}`,
      workspace.name,
      status.isRepository ? status.summary : 'Not a git repository',
      gitDetail(status),
      tone
    );
  });
  const dirtyCount = Object.values(gitStatuses).filter((status) => status?.hasChanges).length;

  return section(
    'git',
    'Git',
    'Repository status summaries from workspace git contracts',
    dirtyCount ? `${dirtyCount} workspace ${pluralize('repository', dirtyCount)} with changes` : 'No loaded repository changes',
    addOverflowItem(boundItems(statusItems, itemLimit), statusItems.length, itemLimit, 'repository'),
    [action('refresh-git', 'Refresh', 'Refresh workspace git status summaries', workspaces.length === 0)]
  );
}

function tasksSection(
  taskHistory: readonly TaskHistoryEntry[],
  taskProcessMetadata: readonly TaskProcessMetadata[],
  itemLimit: number
): ControlPlaneSection {
  const runningItems = taskProcessMetadata.map((task) => item(
    `task-running-${task.taskId}-${task.processId}`,
    task.label,
    `Running PID ${task.processId}`,
    `${task.workspacePath} • started ${formatRelativeTime(task.startedAtEpochMs)}`,
    'warning'
  ));
  const historyItems = [...taskHistory]
    .sort((left, right) => right.startedAtEpochMs - left.startedAtEpochMs)
    .map((task) => item(
      `task-history-${task.taskId}-${task.startedAtEpochMs}`,
      task.label,
      task.canceled ? 'Canceled' : task.exitCode === null || task.exitCode === undefined ? 'No exit code' : `Exit ${task.exitCode}`,
      `${task.executable} ${task.args.join(' ')} • ${task.workspacePath}`,
      task.canceled || (task.exitCode ?? 0) !== 0 ? 'warning' : 'good'
    ));
  const combined = [...runningItems, ...historyItems];

  return section(
    'tasks',
    'Tasks',
    'Running task metadata and bounded task history',
    taskProcessMetadata.length
      ? `${taskProcessMetadata.length} running ${pluralize('task', taskProcessMetadata.length)}`
      : `${taskHistory.length} task history ${pluralize('entry', taskHistory.length)}`,
    addOverflowItem(boundItems(combined, itemLimit), combined.length, itemLimit, 'task'),
    [action('refresh-tasks', 'Refresh', 'Refresh task process metadata and task history')]
  );
}

function processesSection(processes: readonly ProcessInfo[], itemLimit: number): ControlPlaneSection {
  const sorted = [...processes].sort((left, right) => {
    const cpuDelta = (right.cpuPercent ?? -1) - (left.cpuPercent ?? -1);
    if (cpuDelta !== 0) {
      return cpuDelta;
    }
    return (right.memoryBytes ?? -1) - (left.memoryBytes ?? -1);
  });
  const killableCount = processes.filter((process) => process.isKillable).length;
  const workspaceCount = processes.filter((process) => process.workspaceHint).length;
  const processItems = boundItems(sorted, itemLimit).map((process) => item(
    `process-${process.pid}`,
    process.name,
    `PID ${process.pid} • ${process.status}`,
    `${formatProcessCpu(process.cpuPercent)} CPU • ${formatProcessMemory(process.memoryBytes)} memory${process.workspaceHint ? ` • workspace ${process.workspaceHint.label}` : ''}`,
    process.isKillable ? 'neutral' : 'warning'
  ));

  return section(
    'processes',
    'Processes',
    'Bounded process summaries from the process-manager contract',
    `${processes.length} processes • ${killableCount} killable • ${workspaceCount} workspace-linked`,
    addOverflowItem(processItems, processes.length, itemLimit, 'process'),
    [action('refresh-processes', 'Refresh', 'Refresh process-manager snapshot')]
  );
}

function providersSection(
  response: DeveloperSearchResponse | null,
  budget: ControlPlaneProviderBudget,
  itemLimit: number
): ControlPlaneSection {
  const groups = response?.groups ?? [];
  const providerItems = boundItems(groups, itemLimit).map((group) => providerGroupItem(group, budget));
  const resultCount = response?.results.length ?? 0;
  const budgetDetail = `Bounded to ${budget.totalLimit} total and ${budget.perProviderLimit} per provider`;
  const status = response
    ? `${resultCount} provider ${pluralize('result', resultCount)} for "${safeBoundedText(response.query || 'empty query')}"`
    : 'Provider snapshot not loaded';

  return section(
    'providers',
    'Providers',
    'Search provider health and rendering budgets',
    `${status} • ${budgetDetail}`,
    addOverflowItem(providerItems, groups.length, itemLimit, 'provider'),
    [action('refresh-providers', 'Refresh', 'Refresh provider summaries')]
  );
}

function providerGroupItem(
  group: DeveloperSearchProviderGroup,
  budget: ControlPlaneProviderBudget
): ControlPlaneItem {
  const overBudget = group.results.length > budget.perProviderLimit;
  return item(
    `provider-${group.providerId}`,
    group.label,
    `${group.results.length} ${pluralize('result', group.results.length)}`,
    overBudget ? `Over per-provider budget ${budget.perProviderLimit}` : `Within per-provider budget ${budget.perProviderLimit}`,
    overBudget ? 'warning' : 'good'
  );
}

function section(
  id: ControlPlaneSectionId,
  label: string,
  description: string,
  status: string,
  items: ControlPlaneItem[],
  actions: ControlPlaneAction[]
): ControlPlaneSection {
  return {
    id,
    label,
    description,
    status,
    count: items.filter((entry) => !entry.id.endsWith('-overflow')).length,
    ariaLabel: `${label} section: ${status}`,
    items,
    actions
  };
}

function item(
  id: string,
  title: string,
  meta: string,
  detail: string,
  tone: ControlPlaneItemTone
): ControlPlaneItem {
  return {
    id,
    title: safeBoundedText(title),
    meta: safeBoundedText(meta),
    detail: safeBoundedText(detail),
    tone
  };
}

function action(id: string, label: string, ariaLabel: string, disabled = false): ControlPlaneAction {
  return { id, label, ariaLabel, disabled };
}

function addOverflowItem(
  items: ControlPlaneItem[],
  totalCount: number,
  limit: number,
  label: string
): ControlPlaneItem[] {
  const overflowCount = totalCount - limit;
  if (overflowCount <= 0) {
    return items;
  }
  return [
    ...items,
    item(`${label}-overflow`, `${overflowCount} more ${label} ${pluralize('item', overflowCount)}`, 'Bounded rendering', 'Open the source surface for the full authoritative list', 'neutral')
  ];
}

function boundItems<T>(items: readonly T[], limit: number): T[] {
  return items.slice(0, positiveInteger(limit, DEFAULT_ITEM_LIMIT));
}

function gitDetail(status: GitWorkspaceStatus): string {
  const parts = [
    status.branch ? `branch ${status.branch}` : null,
    status.upstream ? `upstream ${status.upstream}` : null,
    status.ahead ? `ahead ${status.ahead}` : null,
    status.behind ? `behind ${status.behind}` : null,
    status.isRebasing ? 'rebase in progress' : null,
    status.isMerging ? 'merge in progress' : null
  ].filter(Boolean);
  return parts.length ? parts.join(' • ') : status.isClean ? 'Clean working tree' : 'Repository state loaded';
}

function formatRelativeTime(epochMs: number): string {
  if (!Number.isFinite(epochMs)) {
    return 'unknown time';
  }
  const seconds = Math.max(0, Math.round((Date.now() - epochMs) / 1000));
  if (seconds < 60) {
    return `${seconds}s ago`;
  }
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) {
    return `${minutes}m ago`;
  }
  return `${Math.round(minutes / 60)}h ago`;
}

function safeBoundedText(value: string, maxLength = 96): string {
  const trimmed = value.trim().replace(/\s+/g, ' ');
  const bounded = trimmed.length > maxLength ? `${trimmed.slice(0, maxLength - 1)}…` : trimmed;
  return redactSecretLikeText(bounded);
}

function redactSecretLikeText(value: string): string {
  return value
    .replace(
      /\b([A-Za-z0-9_.-]*(?:token|secret|password|credential|api[_-]?key|authorization|cookie)[A-Za-z0-9_.-]*)\b\s*[:=]\s*\S+/giu,
      '$1=[REDACTED]'
    )
    .replace(
      /(--[A-Za-z0-9_.-]*(?:token|secret|password|credential|api[_-]?key|authorization|cookie)[A-Za-z0-9_.-]*)\s+\S+/giu,
      '$1 [REDACTED]'
    )
    .replace(/\bbearer\s+\S+/giu, 'Bearer [REDACTED]')
    .replace(
      /\b(?:ghp_|gho_|github_pat_|xoxb-|sk-|akia)[A-Za-z0-9_.-]+/giu,
      '[REDACTED]'
    );
}

function normalizeText(value: string): string {
  return value.trim().toLocaleLowerCase().replace(/\s+/g, ' ');
}

function positiveInteger(value: number | undefined, fallback: number): number {
  return value !== undefined && Number.isFinite(value) && value > 0 ? Math.floor(value) : fallback;
}

function pluralize(word: string, count: number): string {
  return count === 1 ? word : `${word}s`;
}
