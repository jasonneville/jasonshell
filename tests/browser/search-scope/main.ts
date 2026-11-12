import '../../../src/app.css';
import { mount, tick } from 'svelte';
import SearchPanelSurface from '../../../src/components/SearchPanelSurface.svelte';
import type { SearchPanelPayload, SearchPanelResult } from '../../../src/lib/searchPanel';
import { handlers, publish, ledger } from './bridge';

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
mount(SearchPanelSurface, { target: document.getElementById('surface')! });
await tick();
if (!handlers.has('search-panel:update')) throw new Error('Search listener missing');
await publish(initial);
await tick();

const rows = () => Array.from(document.querySelectorAll<HTMLElement>('[role="listbox"] [role="option"]'));
const input = () => document.querySelector<HTMLInputElement>('input[role="combobox"]')!;
async function scope(label: string) {
  const select = document.querySelector('select');
  if (!select) throw new Error('Missing native scope select');
  const option = Array.from(select.options).find(option => option.text === label);
  if (!option) throw new Error(`Missing ${label} option`);
  select.value = option.value;
  select.dispatchEvent(new Event('change', { bubbles: true }));
  await tick();
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
  const select = document.querySelector('select')!;
  select.focus(); await refresh({ query: 'refresh', selectedIndex: 6 });
  check(document.activeElement === select, 'refresh preserves select focus');
  check(select.selectedOptions[0].text === 'Apps', 'refresh preserves scope');
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
  const scopeControl = document.querySelector('select')!;
  const style = getComputedStyle(scopeControl);
  const canvas = document.createElement('canvas');
  const context = canvas.getContext('2d')!;
  context.font = style.font;
  const textWidth = context.measureText('Everything').width;
  const availableWidth = scopeControl.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight) - 18;
  check(scopeControl.getBoundingClientRect().width === 84 && textWidth <= availableWidth, 'Everything text fits 84px native picker with arrow allowance');
  evidence.push({ stage: 'Everything-label', textWidth, availableWidth, pickerWidth: scopeControl.getBoundingClientRect().width });
  const report = { failures, evidence, ledger, boundary: 'Production Svelte/CSS; mocked IPC. No live Tauri/Windows shell.' };
  document.getElementById('results')!.textContent = JSON.stringify(report, null, 2);
  return report;
}
Object.assign(window, { searchScopeFixture: { run, refresh, scope, ledger }, searchScopeReady: true });
