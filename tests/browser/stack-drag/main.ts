import '../../../src/app.css';
import { mount } from 'svelte';
import StackPopupSurface from '../../../src/components/StackPopupSurface.svelte';
import { emit, ledger, listen, overrides } from '../stack-mouse/bridge';

export { emit, emitTo, invoke, listen } from '../stack-mouse/bridge';

export function getCurrentWindow() {
  return {
    label: 'stack-popup', listen,
    isVisible: async () => true, isFocused: async () => true,
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
let holdPrepare = false;
let releasePrepare: (() => void) | undefined;

overrides.set('read_stack_folder', ({ path }) => {
  const items = entries.filter(entry => entry.path.slice(0, entry.path.lastIndexOf('\\')) === path).map(entry => ({
    ...entry, name: entry.path.split('\\').pop(), typeLabel: entry.kind, sizeBytes: 8, modifiedAt: null,
    isHidden: false, isReadonly: false, isSystem: false, isSymlink: false, isReparsePoint: false,
  }));
  return { path, offset: 0, limit: items.length, total: items.length, hasMore: false, warnings: [], items };
});
for (const cut of [true, false]) {
  overrides.set(cut ? 'cut_stack_items' : 'copy_stack_items', ({ paths }) => { clipboard = { paths: Array.from(paths as string[]), cut }; });
}
overrides.set('paste_stack_items', ({ destination }) => {
  if (!clipboard) throw new Error('Paste without staging');
  const pasted = clipboard.paths.map(path => {
    const index = entries.findIndex(entry => entry.path === path);
    const proposed = `${destination}\\${path.split('\\').pop()}`;
    const target = entries.some(entry => entry.path === proposed) ? `${proposed} (copy)` : proposed;
    const item = { path: target, kind: entries[index]?.kind ?? 'file' };
    if (clipboard?.cut && index >= 0) entries.splice(index, 1);
    entries.push(item);
    return target;
  });
  clipboard = undefined;
  return { pasted, failures: [] };
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
  for (const path of paths) {
    const item = entries.find(entry => entry.path === path);
    if (!item) throw new Error(`Missing source ${path}`);
    item.path = `${destination}\\${path.split('\\').pop()}`;
  }
  return { moved: paths.length, failures: [] };
});
overrides.set('start_stack_folder_watch', ({ path }) => ({ watchId: 'fixture-watch', canonicalPath: path }));

mount(StackPopupSurface, { target: document.querySelector('#surface')! });
Object.assign(window, { fixture: {
  root, dest, paths, external, ledger, entries, holdPrepare: () => { holdPrepare = true; }, releasePrepare: () => releasePrepare?.(),
  drop: (paths: string[], position: Point) => emit('fixture:native-drag', {
    type: 'drop', paths, position: physical(position),
  }),
} });
