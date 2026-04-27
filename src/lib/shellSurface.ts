export type ShellSurface =
  | 'top-bar'
  | 'bottom-bar'
  | 'task-preview'
  | 'search-panel'
  | 'stack-popup'
  | 'process-manager'
  | 'unknown';

type SurfaceMeta = {
  title: string;
  subtitle: string;
};

export const shellSurfaceMetadata: Record<ShellSurface, SurfaceMeta> = {
  'bottom-bar': {
    subtitle: 'Primary workspace command rail',
    title: 'JasonShell Taskbar'
  },
  'top-bar': {
    subtitle: 'Primary workspace status rail',
    title: 'JasonShell Menu Bar'
  },
  'task-preview': {
    subtitle: 'Primary workspace hover preview',
    title: 'JasonShell Task Preview'
  },
  'search-panel': {
    subtitle: 'Primary workspace command palette',
    title: 'JasonShell Search'
  },
  'stack-popup': {
    subtitle: 'Pinned folder stack browser',
    title: 'JasonShell Stack'
  },
  'process-manager': {
    subtitle: 'Running process monitor',
    title: 'JasonShell Process Manager'
  },
  unknown: {
    subtitle: 'Surface route unavailable',
    title: 'JasonShell'
  }
};

export function resolveSurfaceFromLabel(label: string | undefined): ShellSurface {
  if (
    label === 'top-bar'
    || label === 'bottom-bar'
    || label === 'task-preview'
    || label === 'search-panel'
    || label === 'stack-popup'
    || label === 'process-manager'
  ) {
    return label;
  }

  return 'unknown';
}
