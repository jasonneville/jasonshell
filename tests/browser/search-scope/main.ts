import '../../../src/app.css';
import { mount, tick, unmount } from 'svelte';
import SearchPanelSurface from '../../../src/components/SearchPanelSurface.svelte';
import MeltSelect from '../../../src/components/melt/MeltSelect.svelte';
import type { SearchPanelPayload, SearchPanelResult } from '../../../src/lib/searchPanel';
import { handlers, publish, ledger } from './bridge';
import { applyShellPreferences } from '../../../src/lib/shellPreferences';

const results: SearchPanelResult[] = [
  { id: 'folder', kind: 'folder', providerId: 'local', title: 'Local folder', path: 'C:\\fixture' },
  { id: 'app', kind: 'app', providerId: 'apps', title: 'First app', recordKey: 'app:first' },
  { id: 'window', kind: 'window', providerId: 'openWindows', title: 'Window' },
  { id: 'setting', kind: 'setting', providerId: 'settings', title: 'Setting' },
  { id: 'command', kind: 'command', providerId: 'commands', title: 'Command' },
  { id: 'indexed', kind: 'file', providerId: 'everything', title: 'Indexed file' },
  { id: 'app', kind: 'app', providerId: 'apps', title: 'Second app', recordKey: 'app:second' }
].map(result => ({ subtitle: 'Fixture supplied result', terms: '', priority: 0, ...result })) as SearchPanelResult[];
const initial: SearchPanelPayload = { query: 'test', results, selectedIndex: 0, presentation: 'centered', statusMessage: '' };
let mountedSearch = mount(SearchPanelSurface, { target: document.getElementById('surface')! });
await tick();
if (!handlers.has('search-panel:update')) throw new Error('Search listener missing');
await publish(initial);
await tick();

const rows = () => Array.from(document.querySelectorAll<HTMLElement>('#search-results [role="option"]'));
const input = () => document.querySelector<HTMLInputElement>('input[role="combobox"]')!;
const scopeTrigger = () => document.querySelector<HTMLElement>('[role="combobox"][aria-label="Search scope"]')!;
const scopePopup = () => document.getElementById(scopeTrigger().getAttribute('aria-controls')!)!;
const scopeOptions = () => Array.from(scopePopup().querySelectorAll<HTMLElement>('[role="option"]'));
const scopeLabel = () => scopeTrigger().querySelector<HTMLElement>('span:first-child')!;
async function scope(label: string) {
  const select = scopeTrigger();
  if (select?.tagName !== 'BUTTON') throw new Error('Rounded scope popup requires project button combobox; native SELECT cannot certify popup radius');
  if (select.getAttribute('aria-expanded') !== 'true') select.click();
  await tick(); await tick();
  const option = scopeOptions().find(option => option.querySelector('span')?.textContent === label);
  if (!option) throw new Error(`Missing ${label} option`);
  option.click();
  await tick(); await tick();
}
async function refresh(changes: Partial<SearchPanelPayload> = {}) { await publish({ ...initial, ...changes }); await tick(); }
async function run() {
  const failures: string[] = [], evidence: unknown[] = [];
  const check = (ok: boolean, message: string) => { if (!ok) failures.push(message); };
  const key = async (key: string, ctrlKey = false) => { input().dispatchEvent(new KeyboardEvent('keydown', { key, ctrlKey, bubbles: true, cancelable: true })); await tick(); };
  ledger.length = 0;
  await refresh();
  await scope('All');
  check(rows().length === 7, 'All preserves supplied results');
  for (const [label, count] of [['Apps', 2], ['Windows', 1], ['Settings', 1], ['Folders', 1], ['Commands', 1], ['Everything', 1]] as const) {
    await scope(label); check(rows().length === count, `${label} result count`);
  }
  check(!ledger.some(entry => entry.name === 'search-panel:query'), 'scope emits no backend query');
  await scope('Apps');
  await key('Enter');
  const activated = ledger.find(entry => entry.name === 'search-panel:activate')?.payload as { resultIndex?: number; recordKey?: string };
  check(activated?.resultIndex === 1 && activated?.recordKey === 'app:first', 'Enter fallback uses original visible app identity');
  await key('Enter', true);
  check(!ledger.some(entry => entry.name === 'search-panel:pin-folder'), 'Apps cannot pin hidden selected folder');
  await key('ArrowDown');
  const selected = ledger.filter(entry => entry.name === 'search-panel:select').at(-1)?.payload as { resultIndex?: number };
  check(selected?.resultIndex === 6, 'ArrowDown skips hidden rows');
  evidence.push({ stage: 'scoped-keyboard', ledger: [...ledger] });
  await scope('Folders');
  await key('Enter', true);
  check(ledger.some(entry => entry.name === 'search-panel:pin-folder' && entry.payload === 'C:\\fixture'), 'visible folder Ctrl+Enter retains original pin path');
  await scope('Apps');
  const select = scopeTrigger();
  select.focus(); await refresh({ query: 'refresh', selectedIndex: 6 });
  check(document.activeElement === select, 'refresh preserves select focus');
  check(scopeLabel().textContent === 'Apps', 'refresh preserves scope');
  input().value = 'typed query'; input().dispatchEvent(new Event('input', { bubbles: true })); await tick();
  check((ledger.filter(entry => entry.name === 'search-panel:query').at(-1)?.payload as { query?: string })?.query === 'typed query', 'typing emits unmodified query');
  document.querySelector<HTMLButtonElement>('[aria-label="Clear search"]')!.click(); await tick(); await tick();
  check(input().value === '' && document.activeElement === input(), 'clear empties query and focuses input');
  check((ledger.filter(entry => entry.name === 'search-panel:query').at(-1)?.payload as { query?: string })?.query === '', 'clear emits empty query');
  evidence.push({ stage: 'refresh-type-clear', ledger: [...ledger] });
  await refresh({ results: [results[0]], selectedIndex: 0 });
  ledger.length = 0; await key('Enter'); await key('Enter', true);
  check(rows().length === 0, 'empty scope has no rows');
  check(!ledger.some(entry => entry.name === 'search-panel:activate' || entry.name === 'search-panel:pin-folder' || (entry.name === 'search-panel:key' && entry.payload === 'Enter')), 'empty scope cannot activate or pin hidden result');
  await scope('All');
  await refresh({ results: results.map(result => ({ ...result, title: result.title.repeat(30), subtitle: 'Long path segment '.repeat(40) })) });
  for (const width of [720, 420, 320]) {
    document.getElementById('surface')!.style.width = `${width}px`;
    await tick(); await new Promise(requestAnimationFrame);
    const header = document.querySelector('header')!.getBoundingClientRect();
    const controls = [select, input(), document.querySelector('kbd')!].map(el => el.getBoundingClientRect());
    check(controls.every(rect => rect.width > 0 && rect.left >= header.left && rect.right <= header.right + 1), `${width}px controls contained`);
    check(controls[0].right <= controls[1].left && controls[1].right <= controls[2].left, `${width}px no control overlap`);
    check(document.querySelector<HTMLElement>('.search-panel')!.scrollWidth <= width + 1, `${width}px no panel horizontal overflow`);
    evidence.push({ width, header: header.toJSON(), controls: controls.map(rect => rect.toJSON()) });
  }
  await scope('Everything');
  const scopeControl = scopeTrigger();
  const style = getComputedStyle(scopeControl);
  const canvas = document.createElement('canvas');
  const context = canvas.getContext('2d')!;
  context.font = style.font;
  const textWidth = context.measureText('Everything').width;
  const availableWidth = scopeLabel().getBoundingClientRect().width;
  check(scopeControl.getBoundingClientRect().width === 112 && textWidth <= availableWidth + 0.1, 'Everything text fits actual label slot in 112px lens-plus-picker capsule');
  evidence.push({ stage: 'Everything-label', textWidth, availableWidth, pickerWidth: scopeControl.getBoundingClientRect().width });
  const report = { failures, evidence, ledger, boundary: 'Production Svelte/CSS; mocked IPC. No live Tauri/Windows shell.' };
  document.getElementById('results')!.textContent = JSON.stringify(report, null, 2);
  return report;
}
async function runFocus(theme = 'vitesse-dark', width = 320, strong = false) {
  const failures: string[] = [];
  const check = (ok: boolean, message: string) => { if (!ok) failures.push(message); };
  document.documentElement.dataset.theme = theme;
  document.documentElement.dataset.shellFocus = strong ? 'strong' : 'normal';
  document.getElementById('surface')!.style.width = `${width}px`;
  await refresh();
  await scope('Everything');
  input().value = 'Search with breathing room';
  input().dispatchEvent(new Event('input', { bubbles: true }));
  await tick();
  const select = scopeTrigger();
  const clear = document.querySelector<HTMLButtonElement>('[aria-label="Clear search"]')!;
  const header = document.querySelector<HTMLElement>('header')!;
  const sample = (element: HTMLElement) => {
    const style = getComputedStyle(element);
    return { outlineStyle: style.outlineStyle, outlineWidth: style.outlineWidth, outlineColor: style.outlineColor, boxShadow: style.boxShadow, borderLeftWidth: style.borderLeftWidth, paddingLeft: style.paddingLeft, rect: element.getBoundingClientRect().toJSON(), focusVisible: element.matches(':focus-visible') };
  };
  const settle = async () => { await tick(); await Promise.all(document.getAnimations().map(animation => animation.finished.catch(() => undefined))); };
  input().focus(); await settle();
  const queryFocus = sample(input()), headerFocus = sample(header);
  // Bounded sRGB contrast probe, not a WCAG/global native-translucency claim.
  // The fixture document is transparent: test both black and white browser
  // backing extremes while compositing actual ancestor backgrounds and alpha.
  const colorCanvas = document.createElement('canvas');
  colorCanvas.width = colorCanvas.height = 1;
  const colorContext = colorCanvas.getContext('2d', { willReadFrequently: true })!;
  const rgba = (color: string) => {
    colorContext.clearRect(0, 0, 1, 1);
    colorContext.fillStyle = color;
    colorContext.fillRect(0, 0, 1, 1);
    const pixel = colorContext.getImageData(0, 0, 1, 1).data;
    return [pixel[0] / 255, pixel[1] / 255, pixel[2] / 255, pixel[3] / 255];
  };
  const over = (foreground: number[], background: number[]) => {
    const alpha = foreground[3] + background[3] * (1 - foreground[3]);
    return [0, 1, 2].map(index => alpha ? (foreground[index] * foreground[3] + background[index] * background[3] * (1 - foreground[3])) / alpha : 0).concat(alpha);
  };
  const luminance = (color: number[]) => color.slice(0, 3).map(channel => channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4).reduce((sum, channel, index) => sum + channel * [0.2126, 0.7152, 0.0722][index], 0);
  const ratio = (left: number[], right: number[]) => (Math.max(luminance(left), luminance(right)) + 0.05) / (Math.min(luminance(left), luminance(right)) + 0.05);
  const ancestors: HTMLElement[] = [];
  for (let node = header.parentElement; node; node = node.parentElement) ancestors.unshift(node);
  const backgroundLayers = ancestors.map(element => ({ tag: element.tagName, className: element.className, color: getComputedStyle(element).backgroundColor, image: getComputedStyle(element).backgroundImage }));
  const headerColor = getComputedStyle(header).backgroundColor;
  check(backgroundLayers.every(layer => layer.image === 'none') && getComputedStyle(header).backgroundImage === 'none', 'contrast probe supports actual flat backgrounds only; gradient/image requires a pixel-based oracle');
  const focusContrast = [0, 1].map(backing => {
    const panelBackground = backgroundLayers.reduce((background, layer) => over(rgba(layer.color), background), [backing, backing, backing, 1]);
    const headerBackground = over(rgba(headerColor), panelBackground);
    const outlineOverPanel = over(rgba(headerFocus.outlineColor), panelBackground);
    const outlineOverHeader = over(rgba(headerFocus.outlineColor), headerBackground);
    return { backing: backing ? 'white' : 'black', panelBackground, headerBackground, outlineOverPanel, outlineOverHeader, panelRatio: ratio(outlineOverPanel, panelBackground), headerRatio: ratio(outlineOverHeader, headerBackground) };
  });
  check(focusContrast.every(sample => sample.panelRatio >= 3 && sample.headerRatio >= 3), 'header focus color composites to at least 3:1 against actual panel/header backgrounds on black and white backing');
  const dividerRight = select.getBoundingClientRect().right;
  const queryTextLeft = input().getBoundingClientRect().left + parseFloat(getComputedStyle(input()).paddingLeft);
  const separation = queryTextLeft - dividerRight;
  check(queryFocus.boxShadow === 'none', 'query focus must not paint nested box-shadow ring from global focus-visible');
  check(queryFocus.outlineStyle === 'none' || parseFloat(queryFocus.outlineWidth) === 0, 'query focus must not paint nested outline');
  check(headerFocus.outlineStyle !== 'none' && parseFloat(headerFocus.outlineWidth) > 0 && parseFloat(headerFocus.outlineWidth) <= 1, 'header owns single restrained focus outline (at most 1px)');
  check(separation >= 14, 'query text starts at least 14px after fixed scope divider');
  check(select.getBoundingClientRect().width === 112, '112px combined lens-plus-picker capsule preserves divider position');
  select.focus(); await settle();
  const selectFocus = sample(select);
  clear.focus(); await settle();
  const clearFocus = sample(clear);
  for (const [name, style] of [['select', selectFocus], ['clear', clearFocus]] as const) {
    check(style.focusVisible && (style.boxShadow !== 'none' || (style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0)), `${name} retains visible keyboard focus`);
  }
  const controls = [select, input(), clear, document.querySelector<HTMLElement>('kbd')!].map(element => element.getBoundingClientRect());
  const bounds = header.getBoundingClientRect();
  check(controls.every(rect => rect.width > 0 && rect.left >= bounds.left && rect.right <= bounds.right), `${width}px focus controls contained`);
  check(controls.every((rect, index) => index === 0 || controls[index - 1].right <= rect.left), `${width}px focus controls do not overlap`);
  input().focus(); await settle();
  const report = { failures, theme, width, strong, queryFocus, headerFocus, selectFocus, clearFocus, separation, backgroundLayers, headerColor, focusContrast, boundary: 'Production global/theme CSS and mounted component; bounded composited sRGB contrast, not global WCAG/native translucency certification; focus() plus synthetic query setup, not native shell.' };
  document.getElementById('results')!.textContent = JSON.stringify(report, null, 2);
  return report;
}
async function runTypography() {
  const failures: string[] = [], evidence: unknown[] = [];
  const root = document.documentElement;
  const previousStyle = root.getAttribute('style');
  const previousDataset = { ...root.dataset };
  const check = (ok: boolean, message: string) => { if (!ok) failures.push(message); };
  try {
    for (const fontId of ['open-sans', 'google-sans']) {
      // Real preference application, with persistence and broadcasts disabled.
      applyShellPreferences({ fontId }, { storage: null, dispatch: false });
      await tick();
      await scope('Everything');
      const select = scopeTrigger();
      await document.fonts.load(getComputedStyle(select).font, 'Everything');
      await document.fonts.ready;
      const bodyFamily = getComputedStyle(document.body).fontFamily;
      const selectStyle = getComputedStyle(select);
      const optionFonts = scopeOptions().map(option => ({ text: option.querySelector('span')?.textContent, family: getComputedStyle(option).fontFamily, size: getComputedStyle(option).fontSize, weight: getComputedStyle(option).fontWeight }));
      check(selectStyle.fontFamily === bodyFamily, `${fontId}: picker follows actual body UI font family`);
      check(optionFonts.length === 7 && optionFonts.every(option => option.family === bodyFamily), `${fontId}: every popup option follows actual body UI font family`);
      const expectedSize = parseFloat(getComputedStyle(root).fontSize) * 0.8;
      check(Math.abs(parseFloat(selectStyle.fontSize) - expectedSize) < 0.01 && optionFonts.every(option => Math.abs(parseFloat(option.size) - expectedSize) < 0.01), `${fontId}: picker and options use proposed 0.8rem project text size in combined capsule`);
      check(selectStyle.fontWeight === '600' && optionFonts.every(option => option.weight === '600'), `${fontId}: picker and every option use approved project control weight 600`);
      check(document.fonts.check(selectStyle.font, 'Everything'), `${fontId}: actual picker font is loaded before measuring`);
      const canvas = document.createElement('canvas');
      const context = canvas.getContext('2d')!;
      context.font = selectStyle.font;
      const textWidth = context.measureText('Everything').width;
      const availableWidth = scopeLabel().getBoundingClientRect().width;
      check(textWidth <= availableWidth + 0.1, `${fontId}: Everything fits combined capsule label slot`);
      const geometry = [];
      for (const width of [420, 320]) {
        document.getElementById('surface')!.style.width = `${width}px`;
        await tick(); await new Promise(requestAnimationFrame);
        const header = document.querySelector('header')!.getBoundingClientRect();
        const controls = [select, input(), document.querySelector('kbd')!].map(element => element.getBoundingClientRect());
        check(controls[0].width === 112, `${fontId}/${width}: combined capsule stays 112px`);
        check(controls.every(rect => rect.width > 0 && rect.left >= header.left && rect.right <= header.right), `${fontId}/${width}: controls remain contained`);
        check(controls.every((rect, index) => index === 0 || controls[index - 1].right <= rect.left), `${fontId}/${width}: controls do not overlap`);
        geometry.push({ width, controls: controls.map(rect => rect.toJSON()) });
      }
      evidence.push({ fontId, token: getComputedStyle(root).getPropertyValue('--js-font-sans'), bodyFamily, selectFamily: selectStyle.fontFamily, selectSize: selectStyle.fontSize, selectWeight: selectStyle.fontWeight, optionFonts, fontLoaded: document.fonts.check(selectStyle.font, 'Everything'), textWidth, availableWidth, geometry });
    }
  } finally {
    if (previousStyle === null) root.removeAttribute('style'); else root.setAttribute('style', previousStyle);
    for (const key of Object.keys(root.dataset)) if (!(key in previousDataset)) delete root.dataset[key];
    Object.assign(root.dataset, previousDataset);
  }
  const report = { failures, evidence, boundary: 'Production computed styles, loaded web fonts, canvas text metrics, real preference application without persistence. Native OS popup raster/font selection is not certified.' };
  document.getElementById('results')!.textContent = JSON.stringify(report, null, 2);
  return report;
}
async function runDropdown() {
  const failures: string[] = [], evidence: unknown[] = [];
  const check = (ok: boolean, message: string) => { if (!ok) failures.push(message); };
  const trigger = scopeTrigger();
  if (trigger?.tagName !== 'BUTTON') return { failures: ['rounded open popup requires existing project button combobox, not native SELECT'], evidence };
  const settle = async () => { await tick(); await tick(); await new Promise(requestAnimationFrame); };
  const key = async (key: string) => { document.activeElement!.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true })); await settle(); };
  await refresh(); await scope('Everything'); ledger.length = 0;
  for (const width of [720, 420, 320]) {
    document.getElementById('surface')!.style.width = `${width}px`;
    trigger.focus(); await key('ArrowDown');
    const popup = scopePopup();
    check(trigger.getAttribute('aria-expanded') === 'true' && popup.matches(':popover-open'), `${width}: real HTML popup opens`);
    check(document.activeElement === popup, `${width}: popup receives navigation focus`);
    const popupStyle = getComputedStyle(popup), headerStyle = getComputedStyle(document.querySelector('header')!);
    check(parseFloat(popupStyle.borderTopLeftRadius) > 0 && popupStyle.borderTopLeftRadius === headerStyle.borderTopLeftRadius, `${width}: open popup radius equals searchbox radius`);
    const bounds = popup.getBoundingClientRect();
    check(bounds.width > 0 && bounds.left >= 0 && bounds.right <= window.innerWidth && bounds.top >= 0 && bounds.bottom <= window.innerHeight, `${width}: open popup contained in viewport`);
    check(popup.scrollWidth <= popup.clientWidth, `${width}: popup has no horizontal scrollbar with selected Everything/checkmark`);
    check(scopeOptions().every(option => option.getBoundingClientRect().right <= bounds.right - parseFloat(popupStyle.borderRightWidth)), `${width}: option rows stay inside rounded popup`);
    check(scopeOptions().length === 7 && scopeOptions().every(option => option.scrollWidth <= option.clientWidth), `${width}: all option labels fit popup`);
    check(trigger.getBoundingClientRect().width === 112, `${width}: combined lens-plus-picker trigger is 112px`);
    const header = document.querySelector('header')!.getBoundingClientRect();
    const triggerRect = trigger.getBoundingClientRect();
    check(Math.abs(bounds.left - triggerRect.left) < 0.5 && Math.abs(bounds.right - triggerRect.right) < 0.5, `${width}: popup shares combined capsule left/right edges`);
    // Old geometry: header border1 + left padding6 + lens18 + gap10 + picker84.
    check(Math.abs(triggerRect.right - header.left - 119) < 0.5, `${width}: divider X preserves original 18+10+84 geometry`);
    evidence.push({ width, popup: bounds.toJSON(), radius: popupStyle.borderTopLeftRadius, headerRadius: headerStyle.borderTopLeftRadius, clientWidth: popup.clientWidth, scrollWidth: popup.scrollWidth });
    await key('Escape');
    check(trigger.getAttribute('aria-expanded') === 'false' && document.activeElement === trigger, `${width}: Escape closes popup and restores trigger focus`);
  }
  await scope('All');
  const lens = document.querySelector<SVGElement>('.search-panel-lens')!;
  const lensBounds = lens.getBoundingClientRect();
  const hit = document.elementFromPoint(lensBounds.left + lensBounds.width / 2, lensBounds.top + lensBounds.height / 2);
  check(hit === trigger || trigger.contains(hit), 'lens center hit-tests to combined scope trigger');
  hit?.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true })); await settle();
  check(trigger.getAttribute('aria-expanded') === 'true', 'clicking lens opens scope popup');
  if (trigger.getAttribute('aria-expanded') === 'true') await key('Escape');
  trigger.focus(); await key('ArrowDown');
  const popup = scopePopup();
  await refresh({ query: 'refresh while scope open' }); await settle();
  check(trigger.getAttribute('aria-expanded') === 'true' && document.activeElement === popup, 'payload refresh preserves open popup and focus');
  await key('Home');
  check(document.getElementById(popup.getAttribute('aria-activedescendant')!)?.textContent?.includes('All') === true, 'Home highlights All');
  await key('End');
  check(document.getElementById(popup.getAttribute('aria-activedescendant')!)?.textContent?.includes('Everything') === true, 'End highlights Everything');
  await key('Home'); await key('ArrowDown');
  check(document.getElementById(popup.getAttribute('aria-activedescendant')!)?.textContent?.includes('Apps') === true, 'ArrowDown highlights Apps');
  await key('ArrowUp');
  check(document.getElementById(popup.getAttribute('aria-activedescendant')!)?.textContent?.includes('All') === true, 'ArrowUp highlights All');
  await key('w');
  check(document.getElementById(popup.getAttribute('aria-activedescendant')!)?.textContent?.includes('Windows') === true, 'typeahead highlights Windows');
  await key('Enter');
  check(scopeLabel().textContent === 'Windows' && rows().length === 1, 'Enter commits highlighted scope and filters results');
  check(document.activeElement === trigger && trigger.getAttribute('aria-expanded') === 'false', 'selection closes popup and restores trigger focus');
  check(!ledger.some(entry => entry.name === 'search-panel:query' || entry.name === 'hide_search_panel' || (entry.name === 'search-panel:key' && entry.payload === 'Escape')), 'scope keyboard activity never emits backend query or closes search panel');
  const report = { failures, evidence, ledger: [...ledger], boundary: 'Real HTML popover and production component/CSS; programmatic click/focus/keyboard dispatch. Trusted Enter/Space/Tab/outside-pointer checks remain separate CLI evidence.' };
  document.getElementById('results')!.textContent = JSON.stringify(report, null, 2);
  return report;
}
let sharedFocusCleanup: (() => Promise<void>) | undefined;
let appearanceDividerBaseline: string | undefined;
async function runPickerAppearance(stage = 'idle', expectedQueryFocus = false) {
  await tick();
  await document.fonts.ready;
  await Promise.all(document.getAnimations().map(animation => animation.finished.catch(() => undefined)));
  const failures: string[] = [];
  const trigger = scopeTrigger();
  const caret = trigger.querySelector<HTMLElement>('.melt-select-caret')!;
  const label = scopeLabel();
  const lens = document.querySelector<SVGElement>('.search-panel-lens')!;
  const ink = async (element: Element) => {
    const svg = element instanceof SVGGraphicsElement ? element : element.querySelector<SVGGraphicsElement>('svg');
    if (svg) {
      const bounds = svg.getBBox(), matrix = svg.getScreenCTM()!;
      const corners = [[bounds.x, bounds.y], [bounds.x + bounds.width, bounds.y + bounds.height]].map(([x,y]) => new DOMPoint(x,y).matrixTransform(matrix));
      return { source: 'SVG geometry', top: Math.min(...corners.map(point => point.y)), bottom: Math.max(...corners.map(point => point.y)), centerY: (corners[0].y + corners[1].y) / 2 };
    }
    const pseudo = getComputedStyle(element, '::before');
    if (pseudo.content !== 'none' && pseudo.maskImage !== 'none') {
      const parentStyle = getComputedStyle(element), parentBox = element.getBoundingClientRect();
      // This oracle supports the actual centered flex-item mask, not a zero-font
      // legacy glyph. Rasterize its real SVG alpha, including stroke/caps/joins.
      if (parentStyle.alignItems !== 'center' || parentStyle.justifyContent !== 'center' || pseudo.maskSize !== 'contain' || pseudo.maskPosition !== '50% 50%' || pseudo.transform !== 'none') throw new Error('Unsupported pseudo-mask layout; update ink oracle rather than guessing its bounds');
      const width = parseFloat(pseudo.width), height = parseFloat(pseudo.height);
      const url = pseudo.maskImage.match(/^url\(["']?(.*?)["']?\)$/)?.[1];
      if (!url || !width || !height) throw new Error('Pseudo-mask must expose a measurable image and dimensions');
      const image = new Image();
      await new Promise<void>((resolve, reject) => { image.onload = () => resolve(); image.onerror = () => reject(new Error('Cannot rasterize actual caret mask')); image.src = url; });
      const scale = 32, canvas = document.createElement('canvas');
      canvas.width = Math.round(width * scale); canvas.height = Math.round(height * scale);
      const context = canvas.getContext('2d')!;
      context.scale(scale, scale);
      const containScale = Math.min(width / image.naturalWidth, height / image.naturalHeight);
      const inkWidth = image.naturalWidth * containScale, inkHeight = image.naturalHeight * containScale;
      context.drawImage(image, (width - inkWidth) / 2, (height - inkHeight) / 2, inkWidth, inkHeight);
      const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
      let first = canvas.height, last = -1;
      for (let y = 0; y < canvas.height; y++) for (let x = 0; x < canvas.width; x++) if (pixels[(y * canvas.width + x) * 4 + 3] > 0) { first = Math.min(first, y); last = Math.max(last, y); }
      if (last < first) throw new Error('Caret mask has no visible alpha');
      const boxTop = parentBox.top + (parentBox.height - height) / 2;
      const top = boxTop + first / scale, bottom = boxTop + (last + 1) / scale;
      return { source: 'actual CSS pseudo SVG-mask alpha at 32x, centered flex geometry', top, bottom, centerY: (top + bottom) / 2, maskSize: { width, height }, legacyGlyphHidden: parseFloat(parentStyle.fontSize) === 0 };
    }
    const range = document.createRange(); range.selectNodeContents(element);
    const rect = range.getBoundingClientRect();
    const context = document.createElement('canvas').getContext('2d')!;
    context.font = getComputedStyle(element).font;
    const metrics = context.measureText(element.textContent ?? '');
    const baseline = rect.top + metrics.fontBoundingBoxAscent;
    return { source: 'DOM text range plus loaded-font ink metrics', top: baseline - metrics.actualBoundingBoxAscent, bottom: baseline + metrics.actualBoundingBoxDescent, centerY: baseline + (metrics.actualBoundingBoxDescent - metrics.actualBoundingBoxAscent) / 2 };
  };
  const caretInk = await ink(caret), labelInk = await ink(label), lensInk = await ink(lens);
  const style = getComputedStyle(trigger);
  if (stage === 'idle') appearanceDividerBaseline = style.borderRightColor;
  const colorContext = document.createElement('canvas').getContext('2d')!;
  colorContext.fillStyle = style.backgroundColor; colorContext.fillRect(0,0,1,1);
  const rgba = Array.from(colorContext.getImageData(0,0,1,1).data);
  const neutralBackground = rgba[3] === 0 || Math.max(...rgba.slice(0,3)) - Math.min(...rgba.slice(0,3)) <= 1;
  if (Math.abs(caretInk.centerY - lensInk.centerY) > 1 || Math.abs(caretInk.centerY - labelInk.centerY) > 1) failures.push('visible caret ink center aligns within 1px of lens and All text ink centers');
  if ('legacyGlyphHidden' in caretInk && !caretInk.legacyGlyphHidden) failures.push('CSS-mask caret does not also paint legacy Unicode glyph');
  if (!neutralBackground || style.backgroundImage !== 'none') failures.push('pointer/open trigger background is transparent or neutral, not accent-tinted');
  if (style.borderRightColor !== appearanceDividerBaseline) failures.push('8% divider color stays unchanged across idle/hover/open states');
  if ((document.activeElement === input()) !== expectedQueryFocus) failures.push('hover preserves expected actual query focus state');
  const headerFocusStyle = getComputedStyle(document.querySelector('header')!);
  if (expectedQueryFocus && (headerFocusStyle.outlineStyle === 'none' || parseFloat(headerFocusStyle.outlineWidth) <= 0)) failures.push('query-focused hover retains visible composite header focus treatment');
  const report = { failures, stage, queryFocused: document.activeElement === input(), hovered: trigger.matches(':hover'), open: trigger.getAttribute('aria-expanded'), background: style.backgroundColor, backgroundImage: style.backgroundImage, neutralBackground, divider: style.borderRightColor, dividerBaseline: appearanceDividerBaseline, headerOutline: getComputedStyle(document.querySelector('header')!).outline, caretInk, labelInk, lensInk, boundary: 'Production computed styles plus actual SVG geometry or text-range/canvas ink metrics. Trusted hover supplied separately by CLI; not native shell or pixel-perfect glyph raster certification.' };
  document.getElementById('results')!.textContent = JSON.stringify(report, null, 2);
  return report;
}
async function mountSharedControls() {
  await sharedFocusCleanup?.();
  // Isolate default primitive focus from Search's polling/autofocus lifecycle.
  await unmount(mountedSearch);
  const host = document.createElement('div');
  host.id = 'shared-focus-controls';
  host.style.cssText = 'width:200px;max-width:100%;display:grid;gap:12px';
  document.body.prepend(host);
  const controls = ['Default focus control', 'Opt-out focus control'].map((label, index) => {
    const target = document.createElement('div'); host.append(target);
    const props = { label, value: 'all', options: [{ value: 'all', label: 'All' }, { value: 'apps', label: 'Apps' }], ...(index === 1 ? { restoreFocusOnClose: false } : {}) };
    return mount(MeltSelect, { target, props });
  });
  const outside = document.createElement('input');
  // Keep the outside target clear of either two-row popup's real top layer.
  outside.style.marginTop = '120px';
  outside.setAttribute('aria-label', 'Shared focus outside'); host.append(outside);
  await tick(); await tick();
  sharedFocusCleanup = async () => {
    for (const control of controls) await unmount(control);
    host.remove(); sharedFocusCleanup = undefined;
    mountedSearch = mount(SearchPanelSurface, { target: document.getElementById('surface')! });
    await tick(); await publish(initial); await tick();
  };
  return true;
}
Object.assign(window, { searchScopeFixture: { run, runFocus, runTypography, runDropdown, runPickerAppearance, mountSharedControls, removeSharedControls: () => sharedFocusCleanup?.(), refresh, scope, ledger }, searchScopeReady: true });
