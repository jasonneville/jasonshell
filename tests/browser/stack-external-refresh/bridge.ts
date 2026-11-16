import { handlers, ledger, overrides, resetBridge as resetBase, listen, emit, emitTo, invoke } from '../stack-mouse/bridge';
export { handlers, ledger, overrides, listen, emit, emitTo, invoke };
export const root = 'C:\\external-fixture\\repo';
export const child = `${root}\\src`;
export const other = 'C:\\external-fixture\\other-repo';
export const changeEvent = 'stack-popup:folder-changed';
export let version = 0;
export let visible = true;
export let watchSequence = 0;
let largeDirectory = false;
export function installLargeDirectory() { largeDirectory = true; }
export function mutate(next = version + 1) { version = next; }
export function setVisible(value: boolean) { visible = value; }
export function context(path: string) { const repositoryRoot = path === other ? other : root; return { repositoryRoot, gitDir: `${repositoryRoot}\\.git`, canonicalPath: path }; }
export function page(path: string) {
  const names = path === root ? [['src', 'folder'], ['draft.txt', 'file'], ['readme.md', 'file']] : [['draft.txt', 'file'], ['readme.md', 'file']];
  if (largeDirectory) for (let index = 0; index < 400; index++) names.push([`file-${String(index).padStart(4, '0')}.txt`, 'file']);
  if (version) names.push([`external-${version}.txt`, 'file']);
  return { path, offset: 0, limit: names.length, total: names.length, hasMore: false, warnings: [], items: names.map(([name, kind]) => ({ path: `${path}\\${name}`, name, kind, typeLabel: kind, sizeBytes: 8 + version, modifiedAt: null, isHidden: false, isReadonly: false, isSystem: false, isSymlink: false, isReparsePoint: false })) };
}
export function summary(path: string) { return { ...context(path), branch: 'main', modified: version, added: 0, deleted: 0, untracked: 0, conflicts: 0, entries: version ? [{ path: `${root}\\draft.txt`, relativePath: 'draft.txt', status: 'modified', staged: false, unstaged: true }] : [] }; }
export function getCurrentWindow() {
  return { label: 'stack-popup', listen, isVisible: async () => visible, isFocused: async () => false,
    onFocusChanged: async (handler: (event: { payload: boolean }) => void) => listen('tauri://focus-changed', handler as (event: {payload: unknown}) => void),
    onDestroyed: async (handler: () => void) => listen('tauri://destroyed', handler), onDragDropEvent: async () => () => {} };
}
export function resetBridge() {
  resetBase(); version = 0; visible = true; watchSequence = 0; largeDirectory = false;
  overrides.set('get_stack_popup_request', () => ({ path: root, requestId: 'external-initial' }));
  overrides.set('read_stack_folder', ({ path }) => page(String(path)));
  overrides.set('get_stack_git_context', ({ path }) => context(String(path)));
  overrides.set('get_stack_git_summary', ({ path }) => summary(String(path)));
  overrides.set('get_stack_git_metadata', ({ path }) => ({ ...context(String(path)), remoteRepositoryUrl: null, ahead: null, behind: null }));
  overrides.set('get_stack_git_status', ({ path }) => summary(String(path)));
  overrides.set('start_stack_folder_watch', ({ path }) => ({ watchId: `watch-${++watchSequence}`, canonicalPath: String(path) }));
  overrides.set('stop_stack_folder_watch', () => undefined);
  overrides.set('hide_stack_popup', () => { visible = false; });
}
export async function notify(watchId?: string) {
  const starts = ledger.filter((entry) => entry.command === 'start_stack_folder_watch');
  await emit(changeEvent, { watchId: watchId ?? `watch-${starts.length}` });
}
export function deferred<T>() {
  let resolve!: (value: T) => void; let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
