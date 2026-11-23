import { afterEach, describe, expect, it, vi } from 'vitest';
import { stackGitStatusPathMatchesEntry } from '../../src/lib/stackPopupState';
import type { StackEntry, StackGitFileStatusKind, StackGitStatus } from '../../src/lib/stackPopup';
// FE-25 acceptance. The module does not exist yet: this file is RED until the fixer adds it.
// Required public surface of src/lib/stackRowDisplay.ts (named exports only):
//   createStackDateFormatter(locale?: string | string[]): (modifiedMs: number | null | undefined) => string
//   formatStackModified(modifiedMs: number | null | undefined): string          // module-level singleton formatter
//   stackAttributeLabels(entry: Pick<StackEntry, 'isHidden' | 'isSystem' | 'isReadonly' | 'isSymlink' | 'isReparsePoint'>): readonly string[]
//   buildStackGitStatusIndex(status: StackGitStatus | null, statusPath: string, folderPath: string,
//                            canonicalPath: string | undefined): StackGitStatusIndex
//   stackGitStatusFromIndex(index: StackGitStatusIndex, entry: Pick<StackEntry, 'path' | 'entryType'>): StackGitFileStatusKind | null
//   export type StackGitStatusIndex (opaque)
import {
  buildStackGitStatusIndex, createStackDateFormatter, formatStackModified, stackAttributeLabels, stackGitStatusFromIndex
} from '../../src/lib/stackRowDisplay';

// Oracle: verbatim behaviour of the pre-optimisation StackPopupSurface.stackGitStatusForEntry.
function priority(current: StackGitFileStatusKind | null, next: StackGitFileStatusKind) {
  const rank: Record<StackGitFileStatusKind, number> = { modified: 1, untracked: 2, added: 3, deleted: 4, conflict: 5 };
  return !current || rank[next] > rank[current] ? next : current;
}
function oracle(entry: Pick<StackEntry, 'path' | 'entryType'>, status: StackGitStatus | null, statusPath: string, folderPath: string, canonicalPath?: string) {
  if (!status || statusPath !== folderPath) return null;
  const entryPath = canonicalPath ? `${canonicalPath.replace(/[\\/]+$/, '')}${entry.path.slice(folderPath.replace(/[\\/]+$/, '').length)}` : entry.path;
  let nested: StackGitFileStatusKind | null = null;
  for (const item of status.entries) {
    if (stackGitStatusPathMatchesEntry(entryPath, item.path, false)) return item.status as StackGitFileStatusKind;
    if (entry.entryType === 'Folder' && stackGitStatusPathMatchesEntry(entryPath, item.path, true)) nested = priority(nested, item.status as StackGitFileStatusKind);
  }
  return nested;
}
const item = (path: string, status: StackGitFileStatusKind) => ({ path, relativePath: path, status, staged: false, unstaged: true });
const gitStatus = (...entries: ReturnType<typeof item>[]) => ({ repositoryRoot: 'C:\\repo', branch: 'main', modified: 0, added: 0, deleted: 0, untracked: 0, conflicts: 0, entries }) as StackGitStatus;
const entryOf = (path: string, entryType: 'File' | 'Folder' = 'File') => ({ path, entryType });

describe('git badge index equals the legacy per-row scan', () => {
  const folder = 'C:\\repo';
  const status = gitStatus(
    item('C:\\repo\\src\\a.ts', 'modified'),
    item('C:\\repo\\SRC\\b.ts', 'conflict'),            // case variant, nested conflict outranks modified
    item('c:/repo/docs/readme.md', 'added'),            // forward slashes + drive case
    item('C:\\repo\\docs\\old.md', 'deleted'),
    item('C:\\repo\\build\\', 'untracked'),             // untracked dir with trailing slash
    item('C:\\repo\\build\\out\\x.js', 'modified'),     // exact dir entry first, nested later
    item('C:\\repo\\new-name.ts', 'added'),             // rename target
    item('C:\\repo\\old-name.ts', 'deleted'),           // rename source
    item('C:\\repo\\ignored.log', 'untracked'),
    item('C:\\repo\\dup.txt', 'modified'),
    item('C:\\repo\\dup.txt', 'conflict'),              // duplicate path: first entry wins
    item('C:\\repo\\srcs\\prefix-trap.ts', 'deleted'),  // `src` must not roll up `srcs`
    item('C:\\repo\\%%weird [name]\\f.txt', 'added')
  );
  const entries = [
    entryOf('C:\\repo\\src', 'Folder'), entryOf('C:\\repo\\SRC\\a.ts'), entryOf('C:\\repo\\docs', 'Folder'), entryOf('C:\\repo\\docs\\readme.md'),
    entryOf('C:\\repo\\build', 'Folder'), entryOf('C:\\repo\\build\\out', 'Folder'), entryOf('C:\\repo\\new-name.ts'), entryOf('C:\\repo\\old-name.ts'),
    entryOf('C:\\repo\\ignored.log'), entryOf('C:\\repo\\dup.txt'), entryOf('C:\\repo\\dup.txt', 'Folder'), entryOf('C:\\repo\\srcs', 'Folder'),
    entryOf('C:\\repo\\clean.txt'), entryOf('C:\\repo\\%%weird [name]', 'Folder'), entryOf('C:/repo/src/', 'Folder'), entryOf('c:\\REPO\\Docs', 'Folder')
  ];

  it.each([
    ['same coordinates', undefined],
    ['verified canonical alias', 'D:\\alias\\repo\\']
  ])('matches for every row (%s)', (_name, canonical) => {
    // Rows stay in the displayed folder's coordinates; with an alias the Git status is reported in canonical coordinates.
    const aliasStatus = canonical
      ? gitStatus(...status.entries.map((e) => item(e.path.replace(/^c:[\\/]repo/i, 'D:\\alias\\repo'), e.status as StackGitFileStatusKind)))
      : status;
    const index = buildStackGitStatusIndex(aliasStatus, folder, folder, canonical);
    for (const row of entries) {
      expect(stackGitStatusFromIndex(index, row), row.path).toBe(oracle(row, aliasStatus, folder, folder, canonical));
    }
  });
  it('spot-checks semantic precedence rather than only equality', () => {
    const index = buildStackGitStatusIndex(status, folder, folder, undefined);
    expect(stackGitStatusFromIndex(index, entryOf('C:\\repo\\src', 'Folder'))).toBe('conflict');
    expect(stackGitStatusFromIndex(index, entryOf('C:\\repo\\build', 'Folder'))).toBe('untracked'); // exact match beats nested roll-up
    expect(stackGitStatusFromIndex(index, entryOf('C:\\repo\\dup.txt'))).toBe('modified');
    expect(stackGitStatusFromIndex(index, entryOf('C:\\repo\\srcs', 'Folder'))).toBe('deleted');
    expect(stackGitStatusFromIndex(index, entryOf('C:\\repo\\clean.txt'))).toBeNull();
    expect(stackGitStatusFromIndex(index, entryOf('C:\\repo\\docs\\readme.md'))).toBe('added');
  });
  it('files never inherit roll-up status and null/stale status never badges', () => {
    const index = buildStackGitStatusIndex(status, folder, folder, undefined);
    expect(stackGitStatusFromIndex(index, entryOf('C:\\repo\\src\\a.ts'))).toBe('modified');
    expect(stackGitStatusFromIndex(index, entryOf('C:\\repo\\docs'))).toBeNull(); // a File named docs
    expect(stackGitStatusFromIndex(buildStackGitStatusIndex(null, folder, folder, undefined), entryOf('C:\\repo\\src', 'Folder'))).toBeNull();
    const retired = buildStackGitStatusIndex(status, folder, 'C:\\other-folder', undefined); // statusPath !== folderPath
    for (const row of entries) expect(stackGitStatusFromIndex(retired, row)).toBeNull();
  });
  it('rebuilding on a new status object reflects the change; an empty status clears every badge', () => {
    const first = buildStackGitStatusIndex(gitStatus(item('C:\\repo\\a.txt', 'modified')), folder, folder, undefined);
    expect(stackGitStatusFromIndex(first, entryOf('C:\\repo\\a.txt'))).toBe('modified');
    const second = buildStackGitStatusIndex(gitStatus(item('C:\\repo\\a.txt', 'deleted')), folder, folder, undefined);
    expect(stackGitStatusFromIndex(second, entryOf('C:\\repo\\a.txt'))).toBe('deleted');
    expect(stackGitStatusFromIndex(first, entryOf('C:\\repo\\a.txt'))).toBe('modified'); // old index is immutable
    const clean = buildStackGitStatusIndex(gitStatus(), folder, folder, undefined);
    expect(stackGitStatusFromIndex(clean, entryOf('C:\\repo\\a.txt'))).toBeNull();
  });
  it('lookup does not scan the status list per row (O(1) after build)', () => {
    const many = Array.from({ length: 5000 }, (_, i) => item(`C:\\repo\\dir${i % 50}\\f${i}.ts`, 'modified'));
    const s = gitStatus(...many);
    const index = buildStackGitStatusIndex(s, folder, folder, undefined);
    let reads = 0;
    const counted = new Proxy(s.entries, { get(target, key, receiver) { if (key === 'length' || key === Symbol.iterator || /^\d+$/.test(String(key))) reads += 1; return Reflect.get(target, key, receiver); } });
    (s as { entries: unknown }).entries = counted;
    for (let i = 0; i < 2000; i += 1) stackGitStatusFromIndex(index, entryOf(`C:\\repo\\dir${i % 50}`, 'Folder'));
    expect(reads).toBe(0);
  });
});

describe('date formatting', () => {
  afterEach(() => vi.restoreAllMocks());
  it('reuses one Intl.DateTimeFormat for any number of rows and still formats like the old inline call', () => {
    const real = Intl.DateTimeFormat;
    let constructed = 0;
    vi.spyOn(Intl, 'DateTimeFormat').mockImplementation(function (this: unknown, ...args: ConstructorParameters<typeof real>) { constructed += 1; return new real(...args); } as never);
    const format = createStackDateFormatter();
    const stamps = Array.from({ length: 500 }, (_, i) => 1_700_000_000_000 + i * 86_400_000);
    const out = stamps.map((ms) => format(ms));
    for (let i = 0; i < 100; i += 1) formatStackModified(stamps[i]);
    expect(constructed).toBeLessThanOrEqual(2); // one for createStackDateFormatter, at most one lazy singleton
    const legacy = new real(undefined, { dateStyle: 'short', timeStyle: 'short' });
    expect(out).toEqual(stamps.map((ms) => legacy.format(new Date(ms))));
    expect(formatStackModified(stamps[3])).toBe(legacy.format(new Date(stamps[3])));
  });
  it('keeps the empty-cell contract for missing timestamps', () => {
    const format = createStackDateFormatter('en-US');
    for (const empty of [null, undefined, 0]) expect(format(empty)).toBe('');
    expect(formatStackModified(null)).toBe('');
    expect(createStackDateFormatter('en-US')(Date.UTC(2024, 0, 2, 3, 4))).toMatch(/2024|24/);
  });
});

describe('attribute labels', () => {
  const base = { isHidden: false, isSystem: false, isReadonly: false, isSymlink: false, isReparsePoint: false };
  it('keeps order Hidden, System, Read-only, Link and treats symlink or reparse point as one Link', () => {
    expect(stackAttributeLabels(base)).toEqual([]);
    expect(stackAttributeLabels({ isHidden: true, isSystem: true, isReadonly: true, isSymlink: true, isReparsePoint: true })).toEqual(['Hidden', 'System', 'Read-only', 'Link']);
    expect(stackAttributeLabels({ ...base, isReparsePoint: true })).toEqual(['Link']);
  });
  it('returns a stable shared empty result for plain rows (no per-render allocation)', () => {
    expect(stackAttributeLabels(base)).toBe(stackAttributeLabels({ ...base }));
  });
});
