import { child, root, overrides, resetBridge as resetBase, status } from '../stack-mouse/bridge';
export { child, root, ledger, handlers, overrides, listen, emit, emitTo, getCurrentWindow, invoke } from '../stack-mouse/bridge';
export const deep = `${child}\\src`;
export const nested = `${child}\\nested`;
export const submodule = `${child}\\module`;
export const linked = `${root}\\linked`;
export const outside = `${root}\\outside`;
export const sibling = `${root}\\repo-other`;
export const junction = `${child}\\escape`;
export const context = (path: string) => {
  if (path === outside || path === root) return null;
  const repositoryRoot = path === deep ? child : path === junction ? sibling : path;
  return { repositoryRoot, gitDir: `${repositoryRoot}\\.git`, canonicalPath: path === junction ? sibling : path };
};
export const summary = (path: string, branch = path === child || path === deep ? 'main' : 'other') => ({
  ...status, repositoryRoot: context(path)?.repositoryRoot ?? path, branch,
  entries: [{ path: `${child}\\src\\draft.txt`, relativePath: 'src/draft.txt', status: 'modified', staged: false, unstaged: true }]
});
function page(path: string) {
  const names = path === child ? [['src', 'folder'], ['nested', 'folder'], ['module', 'folder'], ['escape', 'folder'], ['draft.txt', 'file'], ['archive.zip', 'file']]
    : path === root ? [['repo', 'folder'], ['linked', 'folder'], ['outside', 'folder'], ['repo-other', 'folder']]
    : [['draft.txt', 'file']];
  return { path, offset: 0, limit: names.length, total: names.length, hasMore: false, warnings: [], items: names.map(([name, kind]) => ({
    path: `${path}\\${name}`, name, kind, typeLabel: kind, sizeBytes: 8, modifiedAt: null,
    isHidden: false, isReadonly: false, isSystem: false, isSymlink: false, isReparsePoint: false
  })) };
}
export function resetBridge() {
  resetBase();
  overrides.set('get_stack_popup_request', () => ({ path: child, requestId: 'refresh-fixture' }));
  overrides.set('read_stack_folder', ({ path }) => page(String(path)));
  overrides.set('get_stack_git_context', ({ path }) => context(String(path)));
  overrides.set('get_stack_git_summary', ({ path }) => summary(String(path)));
  overrides.set('get_stack_git_metadata', ({ path }) => ({ ...context(String(path)), remoteRepositoryUrl: 'https://example.invalid/repo', ahead: 2, behind: 1 }));
  overrides.set('get_stack_git_status', ({ path }) => context(String(path)) ? summary(String(path)) : null);
  overrides.set('save_stack_basic_text_file', ({ path, content }) => ({ path, content, identity: 'saved', byteLength: String(content).length }));
  overrides.set('new_stack_folder', ({ parent, name }) => ({ ...page(String(parent)).items[0], path: `${parent}\\${name}`, name, kind: 'folder' }));
  overrides.set('new_stack_text_file', ({ parent }) => ({ ...page(String(parent)).items[0], path: `${parent}\\New Text File.txt`, name: 'New Text File.txt', kind: 'file' }));
  overrides.set('rename_stack_item', ({ path, newName }) => ({ ...page(child).items[4], path: String(path).replace(/[^\\]+$/, String(newName)), name: newName }));
  overrides.set('paste_stack_items', () => ({ pasted: [{ ...page(child).items[4], path: `${child}\\incoming.txt`, name: 'incoming.txt' }], failures: [] }));
  overrides.set('list_stack_open_with_candidates', () => []);
  for (const command of ['copy_stack_items', 'cut_stack_items', 'delete_stack_item', 'begin_stack_popup_focus_loss_hold', 'end_stack_popup_focus_loss_hold', 'extract_stack_archive']) overrides.set(command, () => undefined);
}
export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
