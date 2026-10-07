import '../../../src/app.css';
import { mount, unmount, tick } from 'svelte';
import TaskGallerySurface from '../../../src/components/TaskGallerySurface.svelte';
import { handlers, ledger, pending, publish } from './bridge';
const icon = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="#82b5ef"/></svg>');
const rows = ['Alpha', 'Beta', 'Gamma — exceptionally long document title that must truncate without moving the reserved close slot'].map((title, index) => ({
  hwnd: String(101 + index), title, processName: 'Fixture Editor', processId: 42, iconDataUrl: icon,
  isActive: index === 0, isMinimized: index === 2
}));
const manyRows = Array.from({ length: 30 }, (_, index) => ({
  ...rows[index % rows.length], hwnd: String(201 + index), title: `Window ${index + 1} — many-window long title`,
  isActive: index === 0, isMinimized: index % 3 === 2
}));
let mounted: ReturnType<typeof mount> | null = null;
let generation = 0;
let nonce = '';
const el = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
async function snapshot(windows = rows) {
  await publish('task-gallery:open', { nonce, groupKey: 'fixture-editor', label: 'Fixture Editor', focusGallery: false, windows });
  await tick(); measure();
}
async function open(reset = false) {
  if (reset) { ledger.length = 0; }
  if (!mounted) {
    mounted = mount(TaskGallerySurface, { target: el('surface') });
    await tick();
    if (!handlers.get('task-gallery:open')?.size) throw new Error('Gallery listener missing');
  }
  nonce = `fixture-${++generation}`;
  await snapshot();
}
function measure() {
  const strip = document.querySelector<HTMLElement>('.task-gallery-strip');
  const viewport = strip?.getBoundingClientRect();
  const geometry = [...document.querySelectorAll<HTMLElement>('[data-gallery-hwnd]')].map((tile) => {
    const primary = tile.querySelector<HTMLElement>('.task-gallery-activate')!;
    const x = tile.querySelector<HTMLElement>('.task-gallery-close')!;
    const title = tile.querySelector<HTMLElement>('.task-gallery-tab-title')!;
    const rect = (node: HTMLElement) => { const r = node.getBoundingClientRect(); return { x:r.x,y:r.y,width:r.width,height:r.height }; };
    const closeRect = x.getBoundingClientRect();
    return { hwnd:tile.dataset.galleryHwnd, tile:rect(tile), activation:rect(primary), close:rect(x),
      closeOpacity:getComputedStyle(x).opacity, closeTabIndex:x.tabIndex, busy:x.getAttribute('aria-busy'),
      titleWidth:title.clientWidth, titleTruncated:title.scrollWidth > title.clientWidth,
      focused:document.activeElement?.getAttribute('aria-label'),
      focusWithin:tile.matches(':focus-within'), closeHasFocus:document.activeElement === x,
      closePointerEvents:getComputedStyle(x).pointerEvents,
      closeFullyWithinStrip:Boolean(viewport && closeRect.left >= viewport.left && closeRect.right <= viewport.right
        && closeRect.top >= viewport.top && closeRect.bottom <= viewport.bottom),
      textColor:getComputedStyle(tile).color, tileBackground:getComputedStyle(tile).backgroundColor,
      focusOutline:getComputedStyle(document.activeElement === x ? x : primary).outlineColor };
  });
  const metrics = { theme:document.documentElement.dataset.theme,
    activeElement:document.activeElement?.getAttribute('aria-label'),
    documentVisibility:document.visibilityState, documentHasFocus:document.hasFocus(),
    strip:strip ? { clientWidth:strip.clientWidth, clientHeight:strip.clientHeight, scrollWidth:strip.scrollWidth,
      scrollHeight:strip.scrollHeight, scrollLeft:strip.scrollLeft, overflowX:getComputedStyle(strip).overflowX } : null,
    tiles:geometry };
  el('summary').textContent = JSON.stringify({ theme:metrics.theme, activeElement:metrics.activeElement,
    documentVisibility:metrics.documentVisibility, documentHasFocus:metrics.documentHasFocus,
    strip:metrics.strip, tileCount:geometry.length,
    fullyVisibleCloseTargets:geometry.filter((tile) => tile.closeFullyWithinStrip).length,
    zeroWidthTitles:geometry.filter((tile) => tile.titleWidth === 0).length,
    closeDimensions:[...new Set(geometry.map((tile) => `${tile.close.width}x${tile.close.height}`))],
    first:geometry[0], last:geometry.length > 3 ? geometry.at(-1) : undefined }, null, 2);
  el('geometry').textContent = JSON.stringify(metrics, null, 2);
  return metrics;
}
async function focusControl(selector: string) {
  document.querySelector<HTMLButtonElement>(selector)?.focus();
  await tick();
  // Global CSS animates opacity; record settled reveal rather than first-frame 0.
  await new Promise((resolve) => window.setTimeout(resolve, 200));
  measure();
}
el('open').onclick = () => { void open(true); };
el('replace').onclick = () => { void open(); };
el('focus-activate').onclick = () => { void focusControl('.task-gallery-activate'); };
el('focus-close').onclick = () => { void focusControl('.task-gallery-close'); };
el('many').onclick = () => { void snapshot(manyRows); };
el('resolve').onclick = () => { pending.splice(0).forEach(({ resolve }) => resolve()); };
el('reject').onclick = () => { pending.splice(0).forEach(({ reject }) => reject(new Error('Mock WM_CLOSE denied'))); };
el('remove').onclick = () => { void snapshot([rows[0], rows[2]]); };
el('closed').onclick = () => { void publish('task-gallery:closed', { nonce }); };
el('unmount').onclick = () => { if (mounted) { void unmount(mounted); mounted = null; } };
el('measure').onclick = measure;
el<HTMLSelectElement>('height').onchange = (event) => { el('surface').style.height = `${(event.target as HTMLSelectElement).value}px`; measure(); };
el<HTMLSelectElement>('width').onchange = (event) => { el('surface').style.width = `${(event.target as HTMLSelectElement).value}px`; measure(); };
el<HTMLSelectElement>('theme').onchange = (event) => {
  const light = (event.target as HTMLSelectElement).value === 'light';
  document.documentElement.dataset.theme = light ? 'base-light' : 'base-dark';
  // Clear old partial palette from earlier fixture HMR; product CSS owns all tokens.
  for (const name of ['surface', 'control', 'control-hover', 'text']) document.documentElement.style.removeProperty(`--js-color-${name}`);
  measure();
};
document.documentElement.dataset.theme = 'base-dark';
Object.assign(window, { galleryFixture: { open, snapshot, measure, focusControl, ledger, pending, rows, manyRows,
  settle: (success = true) => pending.splice(0).forEach((item) => success ? item.resolve() : item.reject(new Error('Mock WM_CLOSE denied'))) } });
const params = new URLSearchParams(location.search);
for (const name of ['height', 'width', 'theme']) {
  const select = el<HTMLSelectElement>(name);
  const value = params.get(name);
  if (value && [...select.options].some((option) => option.value === value)) {
    select.value = value;
    select.dispatchEvent(new Event('change'));
  }
}
void open();
