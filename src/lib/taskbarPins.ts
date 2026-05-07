import type { PinnedTaskbarLauncher } from './taskbarLaunchers.js';

export function normalizeTaskbarPinTargetKey(path: string): string {
  const trimmed = path.trim();
  if (!trimmed) {
    return '';
  }
  if (/^[a-zA-Z]:[\\/]/u.test(trimmed) || /^\\\\[^\\]/u.test(trimmed)) {
    return trimmed.replace(/\//g, '\\').toLocaleLowerCase();
  }
  return trimmed.toLocaleLowerCase();
}

export function preserveExplorerTaskbarPins(
  launchers: readonly PinnedTaskbarLauncher[]
): PinnedTaskbarLauncher[] {
  return [...launchers];
}
