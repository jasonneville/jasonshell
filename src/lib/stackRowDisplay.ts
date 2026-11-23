import type { StackEntry, StackGitFileStatusKind, StackGitStatus } from './stackPopup';
import { normalizeStackPathKey } from './stackPopupState';

// Per-row display helpers for the Stack details grid. Everything here is allocation-light so a
// many-row folder does not construct formatters or rescan the Git status list per row.

export function createStackDateFormatter(locale?: string | string[]): (modifiedMs: number | null | undefined) => string {
  const formatter = new Intl.DateTimeFormat(locale, { dateStyle: 'short', timeStyle: 'short' });
  return (modifiedMs) => (modifiedMs ? formatter.format(new Date(modifiedMs)) : '');
}

let sharedFormatter: ((modifiedMs: number | null | undefined) => string) | null = null;
export function formatStackModified(modifiedMs: number | null | undefined): string {
  sharedFormatter ??= createStackDateFormatter();
  return sharedFormatter(modifiedMs);
}

const NO_ATTRIBUTE_LABELS: readonly string[] = Object.freeze([]);

export function stackAttributeLabels(
  entry: Pick<StackEntry, 'isHidden' | 'isSystem' | 'isReadonly' | 'isSymlink' | 'isReparsePoint'>
): readonly string[] {
  if (!entry.isHidden && !entry.isSystem && !entry.isReadonly && !entry.isSymlink && !entry.isReparsePoint) return NO_ATTRIBUTE_LABELS;
  const labels: string[] = [];
  if (entry.isHidden) labels.push('Hidden');
  if (entry.isSystem) labels.push('System');
  if (entry.isReadonly) labels.push('Read-only');
  if (entry.isSymlink || entry.isReparsePoint) labels.push('Link');
  return labels;
}

const STATUS_RANK: Record<StackGitFileStatusKind, number> = { modified: 1, untracked: 2, added: 3, deleted: 4, conflict: 5 };

export type StackGitStatusIndex = {
  readonly active: boolean;
  readonly folderPrefixLength: number;
  readonly canonicalPrefix: string | null;
  readonly exact: ReadonlyMap<string, StackGitFileStatusKind>;
  readonly nested: ReadonlyMap<string, StackGitFileStatusKind>;
};

const INACTIVE_INDEX: StackGitStatusIndex = Object.freeze({
  active: false,
  folderPrefixLength: 0,
  canonicalPrefix: null,
  exact: new Map<string, StackGitFileStatusKind>(),
  nested: new Map<string, StackGitFileStatusKind>()
});

export function buildStackGitStatusIndex(
  status: StackGitStatus | null,
  statusPath: string,
  folderPath: string,
  canonicalPath: string | undefined
): StackGitStatusIndex {
  if (!status || statusPath !== folderPath) return INACTIVE_INDEX;
  const exact = new Map<string, StackGitFileStatusKind>();
  const nested = new Map<string, StackGitFileStatusKind>();
  for (const item of status.entries) {
    const kind = item.status as StackGitFileStatusKind;
    const key = normalizeStackPathKey(item.path);
    if (!exact.has(key)) exact.set(key, kind);
    // Every proper ancestor key (cut at each backslash) rolls the status up by highest rank.
    for (let cut = key.indexOf('\\'); cut !== -1; cut = key.indexOf('\\', cut + 1)) {
      const ancestor = key.slice(0, cut);
      const current = nested.get(ancestor);
      if (!current || STATUS_RANK[kind] > STATUS_RANK[current]) nested.set(ancestor, kind);
    }
  }
  return {
    active: true,
    folderPrefixLength: folderPath.replace(/[\\/]+$/, '').length,
    canonicalPrefix: canonicalPath ? canonicalPath.replace(/[\\/]+$/, '') : null,
    exact,
    nested
  };
}

export function stackGitStatusFromIndex(
  index: StackGitStatusIndex,
  entry: Pick<StackEntry, 'path' | 'entryType'>
): StackGitFileStatusKind | null {
  if (!index.active) return null;
  const entryPath = index.canonicalPrefix !== null ? `${index.canonicalPrefix}${entry.path.slice(index.folderPrefixLength)}` : entry.path;
  const key = normalizeStackPathKey(entryPath);
  const exact = index.exact.get(key);
  if (exact) return exact;
  return entry.entryType === 'Folder' ? index.nested.get(key) ?? null : null;
}
