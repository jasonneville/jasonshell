import '../../../src/app.css';
import { mount } from 'svelte';
import StackPopupSurface from '../../../src/components/StackPopupSurface.svelte';
import { emit, ledger, listen, overrides } from '../stack-mouse/bridge';

export { emit, emitTo, invoke, listen } from '../stack-mouse/bridge';

let visible = true;
export function getCurrentWindow() {
  return {
    label: 'stack-popup', listen,
    isVisible: async () => visible, isFocused: async () => visible,
    scaleFactor: async () => devicePixelRatio,
    onDragDropEvent: (handler: Parameters<typeof listen>[1]) => listen('fixture:native-drag', handler),
  };
}

const root = 'C:\\fixture';
const dest = `${root}\\Dest`;
const paths = ['alpha.txt', 'beta.txt'].map(name => `${root}\\${name}`);
const external = 'C:\\elsewhere\\incoming.txt';
const entries = [{ path: dest, kind: 'folder' }, ...paths.map(path => ({ path, kind: 'file' }))];
let clipboard: { paths: string[]; cut: boolean } | undefined;
type Point = { x: number; y: number };
const physical = ({ x, y }: Point) => ({ x: Math.round(x * devicePixelRatio), y: Math.round(y * devicePixelRatio) });
let receipt: { requestId: string; paths: string[] } | undefined;
type UndoEntry = { original: string; target: string };
let undoBatch: { id: string; entries: UndoEntry[] } | undefined;
let undoSequence = 0;
function recordMove(entries: UndoEntry[]) {
  const id = `fixture-opaque-undo-${++undoSequence}`;
  undoBatch = { id, entries };
  return id;
}
let holdPrepare = false;
let releasePrepare: (() => void) | undefined;
let holdCut = false;
let releaseCut: (() => void) | undefined;

overrides.set('read_stack_folder', ({ path }) => {
  const items = entries.filter(entry => entry.path.slice(0, entry.path.lastIndexOf('\\')) === path).map(entry => ({
    ...entry, name: entry.path.split('\\').pop(), typeLabel: entry.kind, sizeBytes: 8, modifiedAt: null,
    isHidden: false, isReadonly: false, isSystem: false, isSymlink: false, isReparsePoint: false,
  }));
  return { path, offset: 0, limit: items.length, total: items.length, hasMore: false, warnings: [], items };
});
for (const cut of [true, false]) {
  overrides.set(cut ? 'cut_stack_items' : 'copy_stack_items', async ({ paths }) => {
    clipboard = { paths: Array.from(paths as string[]), cut };
    if (cut && holdCut) {
      holdCut = false;
      await new Promise<void>(resolve => { releaseCut = resolve; });
    }
  });
}
overrides.set('paste_stack_items', ({ destination }) => {
  if (!clipboard) throw new Error('Paste without staging');
  const moved: UndoEntry[] = [];
  const pasted = clipboard.paths.map(path => {
    const index = entries.findIndex(entry => entry.path === path);
    const proposed = `${destination}\\${path.split('\\').pop()}`;
    const target = entries.some(entry => entry.path === proposed) ? `${proposed} (copy)` : proposed;
    const item = { path: target, kind: entries[index]?.kind ?? 'file' };
    if (clipboard?.cut && index >= 0) {
      entries.splice(index, 1);
      moved.push({ original: path, target });
    }
    entries.push(item);
    return target;
  });
  clipboard = undefined;
  return { pasted, failures: [], ...(moved.length ? { undoMoveId: recordMove(moved) } : {}) };
});
overrides.set('start_stack_file_drag', ({ paths }) => ({ requestId: 'fixture-outbound', localDrop: null, status: 'cancelled', effect: 'none', itemCount: (paths as string[]).length }));
overrides.set('prepare_stack_local_drop', async ({ paths }) => {
  const requestId = 'fixture-native-request';
  const source = [...paths as string[]];
  if (holdPrepare) await new Promise<void>(resolve => { releasePrepare = resolve; });
  receipt = { requestId, paths: source };
  return { requestId };
});
overrides.set('commit_stack_local_drop', ({ requestId, destination }) => {
  if (!receipt || receipt.requestId !== requestId || destination !== dest) throw new Error('Invalid local drop commit');
  const { paths } = receipt;
  receipt = undefined;
  const moved = paths.map(path => {
    const item = entries.find(entry => entry.path === path);
    if (!item) throw new Error(`Missing source ${path}`);
    item.path = `${destination}\\${path.split('\\').pop()}`;
    return { original: path, target: item.path };
  });
  return { moved: paths.length, failures: [], undoMoveId: recordMove(moved) };
});
overrides.set('undo_stack_move', args => {
  if (!undoBatch || args.undoMoveId !== undoBatch.id || Object.keys(args).length !== 1) throw new Error('Invalid undo');
  const batch = undoBatch.entries;
  undoBatch = undefined;
  for (const { original, target } of batch) {
    if (!target) {
      if (entries.some(entry => entry.path === original)) throw new Error('Unsafe fixture restore');
      entries.push({ path: original, kind: 'file' });
      continue;
    }
    const item = entries.find(entry => entry.path === target);
    if (!item || entries.some(entry => entry.path === original)) throw new Error('Unsafe fixture restore');
    item.path = original;
  }
  return { moved: batch.length, failures: [] };
});
let nextDeleteUntrackable = false;
overrides.set('delete_stack_items', ({ paths, operationId }) => {
  if (typeof operationId !== 'string' || !operationId) throw new Error('Missing delete operation ID');
  const deleted = (paths as string[]).map(original => {
    const index = entries.findIndex(entry => entry.path === original && entry.kind === 'file');
    if (index < 0) throw new Error(`Missing fixture file ${original}`);
    entries.splice(index, 1);
    return { original, target: '' };
  });
  if (nextDeleteUntrackable) {
    nextDeleteUntrackable = false;
    undoBatch = undefined;
    return { deleted: deleted.length, failures: [], undoMoveId: null };
  }
  return { deleted: deleted.length, failures: [], undoMoveId: recordMove(deleted) };
});
overrides.set('start_stack_folder_watch', ({ path }) => ({ watchId: 'fixture-watch', canonicalPath: path }));
overrides.set('stop_stack_folder_watch', () => undefined);
overrides.set('begin_stack_popup_focus_loss_hold', () => undefined);
overrides.set('end_stack_popup_focus_loss_hold', () => undefined);

mount(StackPopupSurface, { target: document.querySelector('#surface')! });
Object.assign(window, { fixture: {
  root, dest, paths, external, ledger, entries, holdPrepare: () => { holdPrepare = true; }, releasePrepare: () => releasePrepare?.(),
  holdCut: () => { holdCut = true; }, releaseCut: () => releaseCut?.(),
  untrackableDelete: () => { nextDeleteUntrackable = true; },
  hide: () => { visible = false; return emit('stack-popup:closed'); },
  reopen: () => { visible = true; return emit('stack-popup:open', { path: root, requestId: 'fixture-reopen' }); },
  failWatch: () => emit('stack-popup:folder-watch-error', { watchId: 'fixture-watch', message: 'Fixture watcher error' }),
  shiftDrop: (path: string, target: Element) => {
    const dataTransfer = new DataTransfer();
    dataTransfer.setData('application/x-jasonshell-stack-paths', JSON.stringify([path]));
    target.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer, shiftKey: true }));
  },
  drop: (paths: string[], position: Point) => emit('fixture:native-drag', {
    type: 'drop', paths, position: physical(position),
  }),
} });
