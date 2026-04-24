import type { SearchPanelResult } from './searchPanel';
import type { PinnedTaskbarLauncher } from './taskbarLaunchers';
import type { TaskbarWindow } from './taskbarWindows';

const folderResults: SearchPanelResult[] = [
  folderResult('Home', 'shell:Profile', 'User profile folder', 72),
  folderResult('Desktop', 'shell:Desktop', 'Desktop folder', 70),
  folderResult('Documents', 'shell:Personal', 'Documents folder', 68),
  folderResult('Downloads', 'shell:Downloads', 'Downloads folder', 66)
];

const commandResults: SearchPanelResult[] = [
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

export function buildSearchCatalog(
  pinnedLaunchers: PinnedTaskbarLauncher[],
  taskWindows: TaskbarWindow[],
  systemResults: SearchPanelResult[]
): SearchPanelResult[] {
  const appResults = pinnedLaunchers.map((launcher) => ({
    iconDataUrl: launcher.iconDataUrl,
    id: `app:${launcher.shortcutPath}`,
    kind: 'app' as const,
    path: launcher.shortcutPath,
    priority: 100,
    subtitle: 'Pinned app',
    terms: `${launcher.name} ${launcher.shortcutPath} application launch pinned`,
    title: launcher.name
  }));
  const windowResults = taskWindows.map((taskWindow) => ({
    iconDataUrl: taskWindow.iconDataUrl,
    id: `window:${taskWindow.hwnd}`,
    kind: 'window' as const,
    priority: taskWindow.isActive ? 96 : 92,
    subtitle: taskWindow.isMinimized ? 'Minimized window' : taskWindow.processName,
    terms: `${taskWindow.title} ${taskWindow.processName} window focus task`,
    title: taskWindow.title || taskWindow.processName
  }));
  return [...windowResults, ...appResults, ...systemResults, ...folderResults, ...commandResults];
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
    priority,
    subtitle,
    terms: `${title} ${path} folder explorer`,
    title
  };
}
