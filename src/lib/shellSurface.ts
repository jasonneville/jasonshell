export type ShellSurface =
  | 'top-bar'
  | 'bottom-bar'
  | 'task-gallery'
  | 'task-preview'
  | 'search-panel'
  | 'stack-popup'
  | 'process-manager'
  | 'quick-launch-panel'
  | 'control-plane'
  | 'settings-panel'
  | 'tray-panel'
  | 'terminal-panel'
  | 'command-panel'
  | 'audio-panel'
  | 'speech-history-panel'
  | 'calendar-panel'
  | 'speech-indicator'
  | 'context-menu-overlay'
  | 'snip-overlay'
  | 'snip-preview'
  | 'unknown';

type SurfaceMeta = {
  title: string;
  subtitle: string;
};

export const shellSurfaceMetadata: Record<ShellSurface, SurfaceMeta> = {
  'snip-overlay': { title: 'Screen snip', subtitle: 'Screen region selection' },
  'snip-preview': { title: 'Screen snip', subtitle: 'Captured region preview' },
  'bottom-bar': {
    subtitle: 'Primary workspace command rail',
    title: 'JasonShell Taskbar'
  },
  'task-gallery': {
    subtitle: 'Grouped task window gallery',
    title: 'JasonShell Task Gallery'
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
  'quick-launch-panel': {
    subtitle: 'Explorer pin launcher list',
    title: 'JasonShell Quick Launch'
  },
  'control-plane': {
    subtitle: 'Settings and developer dashboard',
    title: 'JasonShell Control Plane'
  },
  'settings-panel': {
    subtitle: 'Quick shell preferences',
    title: 'JasonShell Settings'
  },
  'tray-panel': {
    subtitle: 'Notification area icon relay',
    title: 'JasonShell Tray'
  },
  'terminal-panel': {
    subtitle: 'Persistent JasonShell terminal',
    title: 'JasonShell Terminal'
  },
  'command-panel': {
    subtitle: 'Quick command launcher and editor',
    title: 'JasonShell Commands'
  },
  'audio-panel': {
    subtitle: 'Quick audio controls',
    title: 'JasonShell Sound'
  },
  'speech-history-panel': {
    subtitle: 'Recent in-session speech transcripts',
    title: 'JasonShell Speech History'
  },
  'calendar-panel': {
    subtitle: 'Clock calendar and timezone details',
    title: 'JasonShell Calendar'
  },
  'speech-indicator': {
    subtitle: 'Noninteractive speech capture level',
    title: 'JasonShell Speech Indicator'
  },
  'context-menu-overlay': {
    subtitle: 'Shared top and bottom bar context menu',
    title: 'JasonShell Context Menu'
  },
  unknown: {
    subtitle: 'Surface route unavailable',
    title: 'JasonShell'
  }
};

export function resolveSurfaceFromLabel(label: string | undefined): ShellSurface {
  if (label === 'snip-overlay' || label === 'snip-preview') return label;
  if (label && label.length <= 96 && /^[\x20-\x7e]+$/.test(label)) {
    const match = /^(snip-overlay)-([1-9][0-9]*)-m(?:[0-9]|[12][0-9]|3[01])$/.exec(label)
      ?? /^(snip-preview)-([1-9][0-9]*)$/.exec(label);
    // Render routing only: native concrete-window registry owns authorization.
    if (match && BigInt(match[2]) <= 18446744073709551615n) return match[1] as ShellSurface;
  }
  if (
    label === 'top-bar'
    || label === 'bottom-bar'
    || label === 'task-gallery'
    || label === 'task-preview'
    || label === 'search-panel'
    || label === 'stack-popup'
    || label === 'process-manager'
    || label === 'quick-launch-panel'
    || label === 'control-plane'
    || label === 'settings-panel'
    || label === 'tray-panel'
    || label === 'terminal-panel'
    || label === 'command-panel'
    || label === 'audio-panel'
    || label === 'speech-history-panel'
    || label === 'calendar-panel'
    || label === 'speech-indicator'
    || label === 'context-menu-overlay'
  ) {
    return label;
  }

  return 'unknown';
}
