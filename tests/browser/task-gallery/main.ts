import '../../../src/app.css';
import { mount, unmount, tick } from 'svelte';
import TaskGallerySurface from '../../../src/components/TaskGallerySurface.svelte';
import BottomBar from '../../../src/components/BottomBar.svelte';
import ContextMenuOverlaySurface from '../../../src/components/ContextMenuOverlaySurface.svelte';
import { handlers, ledger, pending, pendingSync, publish, state, openNative, syncResult, capturedTestImage } from './bridge';
const icon = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="#82b5ef"/></svg>');
const rows = ['Alpha', 'Beta', 'Gamma — exceptionally long document title that must truncate without moving Close'].map((title, index) => ({
  hwnd: String(101 + index), title, processName: 'Fixture Editor', processId: 42, iconDataUrl: icon,
  isActive: index === 0, isMinimized: index === 2,
  activityState: 'idle', attentionState: 'idle', toastCount: 0
}));
const manyRows = Array.from({ length: 30 }, (_, index) => ({
  ...rows[index % rows.length], hwnd: String(201 + index), title: `Window ${index + 1} — many-window long title`,
  isActive: index === 0, isMinimized: index % 3 === 2
}));
let mounted: ReturnType<typeof mount> | null = null;
let generation = 0;
let snapshotSequence = 100;
let nonce = '';
const el = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
state.onLayout = (layout) => {
  nonce = state.open.nonce;
  el('surface').style.width = `${layout.panelWidth}px`;
  el('surface').style.height = `${layout.panelHeight}px`;
};
async function mountGallery() {
  if (!mounted) { mounted = mount(TaskGallerySurface, { target: el('surface') }); await tick(); }
  if (!handlers.get('task-gallery:open')?.size) throw new Error('Gallery listener missing');
}
async function snapshot(windows = rows, revision?: number) {
  if (revision !== undefined) {
    await publish('task-gallery:open', { ...state.open, windows, revision });
  } else {
    await openNative({ nonce, groupKey: 'fixture-editor', label: 'Fixture Editor', focusGallery: false, windows });
  }
  await tick(); return measure();
}
async function open(reset = false, windows = rows) {
  if (reset) ledger.length = 0;
  await mountGallery();
  nonce = `fixture-${++generation}`;
  await snapshot(windows);
}
async function bottomSnapshot(windows = rows) {
  state.rows = windows;
  await publish('taskbar:windows-snapshot', { sequence: ++snapshotSequence, windows });
  await tick();
}
function configure(width = 960, height = 600, theme = 'dark') {
  state.availableWidth = width; state.availableHeight = height;
  document.documentElement.dataset.theme = theme === 'light' ? 'base-light' : 'base-dark';
}
function measure() {
  const viewport = document.querySelector<HTMLElement>('.task-gallery-viewport, .task-gallery-strip');
  const rect = (node: Element | null) => {
    if (!node) return null;
    const r = node.getBoundingClientRect();
    return { left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
  };
  const bounds = rect(viewport);
  const tiles = [...document.querySelectorAll<HTMLElement>('[data-gallery-hwnd]')].map((tile) => {
    const primary = tile.querySelector<HTMLButtonElement>('.task-gallery-activate')!;
    const x = tile.querySelector<HTMLButtonElement>('.task-gallery-close')!;
    const title = tile.querySelector<HTMLElement>('.task-gallery-tab-title');
    const closeBounds = rect(x)!;
    return { hwnd: tile.dataset.galleryHwnd, tile: rect(tile), activation: rect(primary), close: closeBounds,
      closeTabIndex: x.tabIndex, busy: x.getAttribute('aria-busy'),
      titleWidth: title?.clientWidth, titleTruncated: Boolean(title && title.scrollWidth > title.clientWidth),
      closeFullyWithinViewport: Boolean(bounds && closeBounds.left >= bounds.left && closeBounds.right <= bounds.right
        && closeBounds.top >= bounds.top && closeBounds.bottom <= bounds.bottom),
      textColor: getComputedStyle(tile).color, tileBackground: getComputedStyle(tile).backgroundColor };
  });
  const metrics = { theme: document.documentElement.dataset.theme, layout: state.open?.layout,
    activeElement: document.activeElement?.getAttribute('aria-label'), panel: rect(document.querySelector('.task-gallery-panel')),
    viewport: viewport ? { ...bounds, clientWidth: viewport.clientWidth, clientHeight: viewport.clientHeight,
      scrollWidth: viewport.scrollWidth, scrollHeight: viewport.scrollHeight, scrollTop: viewport.scrollTop,
      overflowX: getComputedStyle(viewport).overflowX, overflowY: getComputedStyle(viewport).overflowY } : null, tiles };
  el('summary').textContent = JSON.stringify(metrics, null, 2);
  return metrics;
}
async function focusControl(selector: string) {
  document.querySelector<HTMLButtonElement>(selector)?.focus(); await tick(); measure();
}
async function unmountGallery() { if (mounted) { await unmount(mounted); mounted = null; } }
el('open').onclick = () => { void open(true); };
el('replace').onclick = () => { void open(); };
el('focus-activate').onclick = () => { void focusControl('.task-gallery-activate'); };
el('focus-close').onclick = () => { void focusControl('.task-gallery-close'); };
el('many').onclick = () => { void snapshot(manyRows); };
el('resolve').onclick = () => { pending.splice(0).forEach(({ resolve }) => resolve()); };
el('reject').onclick = () => { pending.splice(0).forEach(({ reject }) => reject(new Error('Mock WM_CLOSE denied'))); };
el('remove').onclick = () => { void snapshot([rows[0], rows[2]]); };
el('closed').onclick = () => { void publish('task-gallery:closed', { nonce }); };
el('unmount').onclick = () => { void unmountGallery(); };
el('measure').onclick = measure;
for (const name of ['height', 'width', 'theme']) el<HTMLSelectElement>(name).onchange = () => {
  configure(Number(el<HTMLSelectElement>('width').value), Number(el<HTMLSelectElement>('height').value), el<HTMLSelectElement>('theme').value);
  void snapshot(state.open?.windows ?? rows);
};
const params = new URLSearchParams(location.search);
for (const name of ['height', 'width', 'theme']) {
  const select = el<HTMLSelectElement>(name), value = params.get(name);
  if (value && [...select.options].some((option) => option.value === value)) select.value = value;
}
configure(Number(el<HTMLSelectElement>('width').value), Number(el<HTMLSelectElement>('height').value), el<HTMLSelectElement>('theme').value);
state.rows = rows;
mount(BottomBar, { target: el('bottom-bar-host') });
mount(ContextMenuOverlaySurface, { target: el('menu-host') });
Object.assign(window, { galleryFixture: { open, snapshot, bottomSnapshot, configure, measure, focusControl,
  unmount: unmountGallery, mountGallery, ledger, pending, pendingSync, rows, manyRows, state, publish, syncResult, capturedTestImage,
  settle: (success = true) => pending.splice(0).forEach((item) => success ? item.resolve() : item.reject(new Error('Mock WM_CLOSE denied'))) } });
void open();
