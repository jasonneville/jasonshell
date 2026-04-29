import type { SearchPanelResult } from './searchPanel';
import type { PinnedTaskbarLauncher } from './taskbarLaunchers';
import type { TaskbarWindow } from './taskbarWindows';
import type { WorkspaceActivationPlan } from './workspaces.js';
import { applyWorkspaceSearchBias, workspaceSearchResults } from './workspaces.js';

const folderResults: SearchPanelResult[] = [
  folderResult('Home', 'shell:Profile', 'User profile folder', 72),
  folderResult('Desktop', 'shell:Desktop', 'Desktop folder', 70),
  folderResult('Documents', 'shell:Personal', 'Documents folder', 68),
  folderResult('Downloads', 'shell:Downloads', 'Downloads folder', 66)
];

const commandResults: SearchPanelResult[] = [
  {
    id: 'command:open-control-plane',
    kind: 'command',
    priority: 92,
    subtitle: 'Open settings and developer dashboard',
    terms: 'developer dashboard settings control plane git changes task history control panel providers diagnostics',
    title: 'Open developer dashboard'
  },
  {
    id: 'command:refresh-search',
    kind: 'command',
    priority: 86,
    subtitle: 'Reload apps, windows, files, and folders',
    terms: 'refresh reload search apps windows files folders taskbar catalog',
    title: 'Refresh search results'
  },
  {
    id: 'command:hide-search',
    kind: 'command',
    priority: 64,
    subtitle: 'Close the search panel',
    terms: 'close hide dismiss search panel escape',
    title: 'Hide search panel'
  }
];

const settingResults: SearchPanelResult[] = [
  {
    id: 'setting:windows-settings',
    kind: 'setting',
    path: 'ms-settings:',
    providerId: 'commands',
    priority: 118,
    recordKey: 'setting:windows-settings',
    subtitle: 'Open Windows Settings',
    terms: 'windows settings system settings display bluetooth network apps privacy update personalization control panel',
    title: 'Windows Settings'
  },
  {
    id: 'setting:control-panel',
    kind: 'setting',
    path: 'control.exe',
    providerId: 'commands',
    priority: 116,
    recordKey: 'setting:control-panel',
    subtitle: 'Open classic Control Panel',
    terms: 'control panel classic settings windows system applets devices programs network power',
    title: 'Control Panel'
  }
];

export function buildSearchCatalog(
  pinnedLaunchers: PinnedTaskbarLauncher[],
  taskWindows: TaskbarWindow[],
  systemResults: SearchPanelResult[],
  workspacePlan: WorkspaceActivationPlan | null = null
): SearchPanelResult[] {
  const appResults = pinnedLaunchers.map((launcher) => ({
    iconDataUrl: launcher.iconDataUrl,
    id: `app:${launcher.shortcutPath}`,
    kind: 'app' as const,
    path: launcher.shortcutPath,
    providerId: 'apps',
    priority: 100,
    recordKey: `app:${launcher.shortcutPath.toLocaleLowerCase()}`,
    subtitle: 'Pinned app',
    terms: `${launcher.name} ${launcher.shortcutPath} application launch pinned`,
    title: launcher.name
  }));
  const windowResults = taskWindows.map((taskWindow) => ({
    iconDataUrl: taskWindow.iconDataUrl,
    id: `window:${taskWindow.hwnd}`,
    kind: 'window' as const,
    providerId: 'openWindows',
    priority: taskWindow.isActive ? 96 : 92,
    recordKey: `window:${taskWindow.hwnd}`,
    subtitle: taskWindow.isMinimized ? 'Minimized window' : taskWindow.processName,
    terms: `${taskWindow.title} ${taskWindow.processName} window focus task`,
    title: taskWindow.title || taskWindow.processName,
    topMost: taskWindow.isActive
  }));
  return applyWorkspaceSearchBias(
    [
      ...workspaceSearchResultsOrEmpty(workspacePlan),
      ...windowResults,
      ...appResults,
      ...systemResults,
      ...folderResults,
      ...settingResults,
      ...commandResults
    ],
    workspacePlan
  );
}

function workspaceSearchResultsOrEmpty(workspacePlan: WorkspaceActivationPlan | null): SearchPanelResult[] {
  return workspacePlan ? workspaceSearchResults(workspacePlan) : [];
}

function folderResult(
  title: string,
  path: string,
  subtitle: string,
  priority: number
): SearchPanelResult {
  return {
    id: `folder:${path}`,
    kind: 'folder',
    path,
    providerId: 'warmedCache',
    priority,
    recordKey: `folder:${path.toLocaleLowerCase()}`,
    subtitle,
    terms: `${title} ${path} folder explorer`,
    title
  };
}
