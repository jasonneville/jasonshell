import type { StackGitStatus } from '../../../src/lib/stackPopup';
import { STACK_BROWSER_ROW_HEIGHT_PX } from '../../../src/lib/stackPopupViewModel';

export const root = 'C:\\fixture';
export const child = `${root}\\repo`;
export const ledger: { command: string; args: Record<string, unknown> }[] = [];
export const handlers = new Map<string, Set<(event: { payload: unknown }) => void>>();
export const overrides = new Map<string, (args: Record<string, unknown>) => unknown>();
export const status: StackGitStatus = {
  repositoryRoot: child, branch: 'main', modified: 1, added: 0, deleted: 0, untracked: 0, conflicts: 0,
  entries: [{ path: `${child}\\draft.txt`, relativePath: 'draft.txt', status: 'modified', staged: false, unstaged: true }]
};

export const largeDirectoryCount = 400;
export const fixtureRowHeight = STACK_BROWSER_ROW_HEIGHT_PX;
export function largeFileName(index: number) { return `file-${String(index).padStart(4, '0')}.${index % 2 === 0 ? 'md' : 'txt'}`; }
function folderPage(path: string, names: string[][]) {
  return { path, offset: 0, limit: names.length, total: names.length, hasMore: false, warnings: [], items: names.map(([name, kind]) => ({
    path: `${path}\\${name}`, name, kind, typeLabel: kind, sizeBytes: 8, modifiedAt: null,
    isHidden: false, isReadonly: false, isSystem: false, isSymlink: false, isReparsePoint: false
  })) };
}
export function installLargeDirectoryFixture() {
  overrides.set('read_stack_folder', ({ path }) => folderPage(String(path), path === root
    ? [['repo', 'folder']]
    : Array.from({ length: largeDirectoryCount }, (_, index) => [largeFileName(index), 'file'])));
}

export function resetBridge() { ledger.length = 0; handlers.clear(); overrides.clear(); }
export function listen(event: string, handler: (event: { payload: unknown }) => void) {
  const subscribers = handlers.get(event) ?? new Set();
  subscribers.add(handler);
  handlers.set(event, subscribers);
  return Promise.resolve(() => { subscribers.delete(handler); });
}
export async function emit(event: string, payload?: unknown) { handlers.get(event)?.forEach((handler) => handler({ payload })); }
export async function emitTo(_target: unknown, event: string, payload?: unknown) { await emit(event, payload); }
export function getCurrentWindow() {
  return { label: 'stack-popup', listen, onDragDropEvent: async () => () => {} };
}

export async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  ledger.push({ command, args });
  if (overrides.has(command)) return await overrides.get(command)!(args) as T;
  let result: unknown;
  switch (command) {
    case 'get_stack_popup_request': result = { path: root, requestId: 'fixture-initial' }; break;
    case 'read_stack_folder': {
      const path = String(args.path);
      const names = path === root ? [['repo', 'folder']] : [['draft.txt', 'file'], ['readme.md', 'file']];
      result = folderPage(path, names); break;
    }
    case 'get_stack_git_status': result = args.path === child ? status : null; break;
    case 'resolve_stack_item_icons': result = { items: [], cacheHits: 0, cacheMisses: 0 }; break;
    case 'load_shell_settings': result = { stackBrowser: { terminalProfile: 'windowsTerminal' } }; break;
    case 'read_stack_basic_text_file': result = { path: args.path, content: '# Saved draft', byteLength: 13, identity: 'fixture-identity' }; break;
    case 'stack_git_branches': result = { repositoryRoot: child, currentBranch: 'main', branches: ['main', 'topic'].map((name) => ({ name, current: name === 'main', remote: false, checkedOutElsewhere: false })) }; break;
    case 'stack_git_log': result = { repositoryRoot: child, entries: [{ commitHash: 'abc123', shortHash: 'abc123', authorName: 'Fixture', authorEmail: 'fixture@example.invalid', authoredAt: '2026-01-01T00:00:00Z', subject: 'Fixture commit' }] }; break;
    case 'stack_git_commit_files': result = { repositoryRoot: child, commitHash: 'abc123', files: [{ path: 'draft.txt', relativePath: 'draft.txt', status: 'M', additions: 1, deletions: 1 }] }; break;
    case 'stack_git_stashes': result = { repositoryRoot: child, entries: [{ stashRef: 'stash@{0}', ref: 'stash@{0}', index: 0, branch: 'main', message: 'Fixture stash' }] }; break;
    case 'stack_git_stash_files': result = { repositoryRoot: child, stashRef: 'stash@{0}', files: [{ path: 'draft.txt', relativePath: 'draft.txt', status: 'M', additions: 1, deletions: 1 }] }; break;
    case 'stack_git_diff': case 'stack_git_commit_file_diff': case 'stack_git_stash_file_diff':
      result = { repositoryRoot: child, content: '@@ -1 +1 @@\n-old\n+fixture-added' }; break;
    case 'suggest_stack_paths': case 'list_stack_terminals': case 'list_pinned_stack_folders': result = []; break;
    default: throw new Error(`Fixture refuses unconfigured command: ${command}`);
  }
  return result as T;
}
