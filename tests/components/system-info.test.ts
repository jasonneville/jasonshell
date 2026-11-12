import { fireEvent, render, waitFor, within } from '@testing-library/svelte';
import { readFileSync } from 'node:fs';
import { tick } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import ProcessManagerSurface from '../../src/components/ProcessManagerSurface.svelte';
import SystemInfoView from '../../src/components/SystemInfoView.svelte';
import type { SystemInfoSnapshot } from '../../src/lib/systemInfo';
import { tauriMocks } from './setup';
import { context, deferred, processRow, snapshot, quotaLimitedSnapshot, metricStripSnapshot } from './fixtures/system-info';

type Context = ReturnType<typeof context>;
type Handler = (event: { payload: unknown }) => void;
const mounted: Array<ReturnType<typeof render>> = [];
afterEach(() => { for (const view of mounted.splice(0)) view.unmount(); });
function actualNativeClosePayload(retired: Context) {
  // Keep mocked lifecycle delivery faithful to the producer under test. A
  // versioned test-only close previously hid the native unit-payload defect.
  // The separate Node contract requires every native producer to be versioned.
  const native=readFileSync('src-tauri/src/process_manager.rs','utf8');
  const argument=native.match(/\.emit\(\s*PROCESS_MANAGER_CLOSED_EVENT,\s*([\s\S]*?)\)\s*;/)?.[1]?.trim();
  if (argument==='()') return null; // Rust unit's actual JSON encoding.
  if (argument && /\bcontext\b/.test(argument)) return {context:{...retired}};
  throw new Error('Native closed-event encoding changed: inspect producer and update the faithful test bridge');
}
async function harness(initial = context(0, 0, 'processes', null, false)) {
  const tauri = tauriMocks(); const handlers = new Map<string, Handler>(); let native = initial;
  let outstandingCommits = 0; let maximumCommits = 0;
  const acknowledgeTab = vi.fn<(committed: Context) => Promise<Context>>().mockImplementation(async (committed) => committed);
  tauri.listen.mockImplementation(async (name: string, handler: Handler) => { handlers.set(name, handler); return vi.fn(); });
  const list = vi.fn<() => Promise<unknown[]>>().mockResolvedValue([processRow(101, 'Alpha'), processRow(202, 'Beta')]);
  const read = vi.fn<() => Promise<unknown>>().mockImplementation(async () => snapshot(native.openGeneration, native.tabRevision));
  const kill = vi.fn<() => Promise<unknown>>().mockResolvedValue(undefined);
  tauri.invoke.mockImplementation(async (command: string, args: Record<string, unknown> = {}) => {
    if (command === 'get_process_manager_context') return native;
    if (command === 'set_process_manager_tab') {
      if (args.openGeneration !== native.openGeneration || !Number.isSafeInteger(args.tabRevision) || Number(args.tabRevision) <= native.tabRevision) throw new Error('stale context');
      native = { ...native, tabRevision: Number(args.tabRevision), activeTab: String(args.tab) };
      outstandingCommits++; maximumCommits = Math.max(maximumCommits,outstandingCommits);
      try { return await acknowledgeTab({ ...native }); } finally { outstandingCommits--; }
    }
    if (command === 'get_system_info_snapshot') return read();
    if (command === 'list_processes') return list();
    if (command === 'kill_process') return kill();
    if (command === 'hide_process_manager') { native = { ...native, visible: false, tabRevision: native.tabRevision + 1 }; return; }
    if (command === 'list_taskbar_process_windows') return [];
    throw new Error(`unexpected IPC ${command}`);
  });
  const view = render(ProcessManagerSurface); mounted.push(view);
  await tick(); expect(handlers.has('process-manager:closed')).toBe(true);
  const open = async (generation = native.openGeneration + 1, focusPid: number | null = null) => { native = context(generation, native.tabRevision + 1, 'processes', focusPid); handlers.get('process-manager:open')?.({ payload: { focusPid, context: native } }); await tick(); };
  const retireClose = () => {
    native = { ...native, visible: false, tabRevision: native.tabRevision + 1 };
    const retired={...native}; const payload=actualNativeClosePayload(retired);
    return {retired,payload,deliver:async()=>{handlers.get('process-manager:closed')?.({payload});await tick();}};
  };
  const close = async () => { await retireClose().deliver(); };
  const tab = async (name: string) => { await fireEvent.click(view.getByRole('tab', { name })); await tick(); };
  const filter = () => view.getByRole('textbox', { name: /Filter processes/ }) as HTMLInputElement;
  return { view, tauri, list, read, kill, open, close, retireClose, tab, filter, handlers, acknowledgeTab, native: () => ({ ...native }), maximumCommits: () => maximumCommits };
}

it('header keeps filter, refresh and status Processes-only, preserves name/PID filtering, and shares Close across tabs', async () => {
  const h = await harness(); await h.open();
  await waitFor(() => expect(h.view.getByText('Alpha')).toBeTruthy());
  const header = () => within(h.view.container.querySelector('header')!);
  expect(header().getByRole('button', { name: 'Refresh process manager' })).toBeTruthy();
  expect(header().getByRole('status')).toBeTruthy();
  await fireEvent.input(h.filter(), { target: { value: 'Alpha' } });
  expect(h.view.getByText('Alpha')).toBeTruthy(); expect(h.view.queryByText('Beta')).toBeNull();
  await fireEvent.input(h.filter(), { target: { value: '202' } });
  expect(h.view.getByText('Beta')).toBeTruthy(); expect(h.view.queryByText('Alpha')).toBeNull();
  const beforeRefresh = h.list.mock.calls.length;
  await fireEvent.click(header().getByRole('button', { name: 'Refresh process manager' }));
  await waitFor(() => expect(h.list.mock.calls.length).toBeGreaterThan(beforeRefresh));
  expect(h.filter().value).toBe('202');
  await h.tab('System Info');
  expect(header().queryByRole('textbox')).toBeNull();
  expect(header().queryByRole('button', { name: 'Refresh process manager' })).toBeNull();
  expect(header().queryByRole('status')).toBeNull();
  expect(header().getByRole('button', { name: 'Close process manager' })).toBeTruthy();
  await h.tab('Processes'); expect(h.filter().value).toBe('202');
  expect(header().getByRole('button', { name: 'Close process manager' })).toBeTruthy();
  await h.tab('System Info');
  await fireEvent.click(header().getByRole('button', { name: 'Close process manager' }));
  await waitFor(() => expect(h.tauri.invoke).toHaveBeenCalledWith('hide_process_manager'));
});

it('every opening selects Processes; targeted open supersedes unresolved enumeration with bounded admission', async () => {
  const h = await harness(); const old = deferred<unknown[]>(); h.list.mockReturnValueOnce(old.promise);
  await h.open(1, 101); await waitFor(() => expect(h.list).toHaveBeenCalledTimes(1));
  await h.open(2, 202); await h.open(3, 202);
  expect(h.filter().value).toBe('202');
  expect(h.view.getByRole('tab', { name: 'Processes' }).getAttribute('aria-selected')).toBe('true');
  expect(h.list.mock.calls.length).toBeLessThanOrEqual(2);
  old.resolve([processRow(101, 'Stale old generation')]);
  await waitFor(() => expect(h.view.getByText('Beta')).toBeTruthy());
  expect(h.view.queryByText('Stale old generation')).toBeNull();
  expect(h.list.mock.calls.length).toBeLessThanOrEqual(3);
});
it('no-PID reopening clears prior PID and restores default CPU sort', async () => {
  const h = await harness(); await h.open(1, 202); await waitFor(() => expect(h.filter().value).toBe('202')); await h.open(2);
  expect(h.filter().value).toBe('');
  await waitFor(() => expect(h.view.getByRole('columnheader', { name: /CPU/ }).getAttribute('aria-sort')).toBe('descending'));
});
it('cold mount recovers already-visible exact-generation PID after listeners attach', async () => {
  const h = await harness(context(7, 13, 'processes', 202));
  await waitFor(() => expect(h.filter().value).toBe('202'));
  expect(h.tauri.invoke).toHaveBeenCalledWith('get_process_manager_context');
  await waitFor(() => expect(h.view.getByText('Beta')).toBeTruthy());
});
it('late context recovery cannot overwrite newer open event', async () => {
  const tauri = tauriMocks(); const recovery = deferred<Context>(); const handlers = new Map<string, Handler>();
  tauri.listen.mockImplementation(async (name: string, handler: Handler) => { handlers.set(name, handler); return vi.fn(); });
  tauri.invoke.mockImplementation((command: string) => command === 'get_process_manager_context' ? recovery.promise : Promise.resolve([]));
  const view = render(ProcessManagerSurface); mounted.push(view);
  await waitFor(() => expect(tauri.invoke).toHaveBeenCalledWith('get_process_manager_context'));
  handlers.get('process-manager:open')?.({ payload: { focusPid: 202, context: context(9, 19, 'processes', 202) } });
  recovery.resolve(context(8, 17, 'processes', 101)); await tick(); await tick();
  expect((view.getByRole('textbox', { name: /Filter processes/ }) as HTMLInputElement).value).toBe('202');
});
it('stale enumeration rejection/finally cannot replace current generation or release its loading guard', async () => {
  const h = await harness(); const old = deferred<unknown[]>(); const current = deferred<unknown[]>();
  h.list.mockReturnValueOnce(old.promise).mockReturnValueOnce(current.promise);
  await h.open(1); await h.close(); await h.open(2, 202); old.reject(new Error('old failure')); await tick(); await tick();
  expect(h.view.queryByText('Process list unavailable')).toBeNull(); current.resolve([processRow(202, 'Current generation')]);
  await waitFor(() => expect(h.view.getByText('Current generation')).toBeTruthy());
});
it('preserves filter, explicit sort, group expansion and process viewport across tabs', async () => {
  const h = await harness(); await h.open(); await waitFor(() => expect(h.view.getByText('Alpha')).toBeTruthy());
  await fireEvent.input(h.filter(), { target: { value: 'Alpha' } }); await fireEvent.click(h.view.getByRole('columnheader', { name: /^Name/ }));
  h.view.getByRole('grid', { name: 'Running processes' }).scrollTop = 144;
  const toggle = h.view.getAllByRole('button').find((button) => button.hasAttribute('aria-expanded'))!;
  const controls = toggle.getAttribute('aria-controls'); expect(controls).toBeTruthy();
  expect(toggle.getAttribute('aria-expanded')).toBe('true'); await fireEvent.click(toggle);
  expect(toggle.getAttribute('aria-expanded')).toBe('false');
  await h.tab('System Info'); await h.tab('Processes'); expect(h.filter().value).toBe('Alpha');
  expect(h.view.getByRole('columnheader', { name: /^Name/ }).getAttribute('aria-sort')).toBe('ascending');
  expect(h.view.getByRole('grid', { name: 'Running processes' }).scrollTop).toBe(144);
  const restored = h.view.getAllByRole('button').find((button) => button.getAttribute('aria-controls') === controls);
  expect(restored).toBeTruthy(); expect(restored?.getAttribute('aria-expanded')).toBe('false');
});
it('leaving Processes clears kill arming; Escape cancels first and never terminates', async () => {
  const h = await harness(); await h.open(); await waitFor(() => expect(h.view.getByText('Alpha')).toBeTruthy());
  await fireEvent.click(h.view.getByRole('button', { name: /Arm single-process kill confirmation for Alpha/ })); await fireEvent.keyDown(window, { key: 'Escape' });
  expect(h.kill).not.toHaveBeenCalled(); expect(h.tauri.invoke.mock.calls.some(([command]) => command === 'hide_process_manager')).toBe(false);
  await fireEvent.click(h.view.getByRole('button', { name: /Arm single-process kill confirmation for Alpha/ })); await h.tab('System Info'); await h.tab('Processes');
  await fireEvent.click(h.view.getByRole('button', { name: /Arm single-process kill confirmation for Alpha/ })); expect(h.kill).not.toHaveBeenCalled();
});
it('dispatched kill settles tab-neutrally without restarting process polling', async () => {
  const h = await harness(); const killing = deferred<unknown>(); h.kill.mockReturnValueOnce(killing.promise);
  await h.open(); await waitFor(() => expect(h.view.getByText('Alpha')).toBeTruthy());
  await fireEvent.click(h.view.getByRole('button', { name: /Arm single-process kill confirmation for Alpha/ })); await fireEvent.click(h.view.getByRole('button', { name: /Confirm kill for Alpha/ }));
  await waitFor(() => expect(h.kill).toHaveBeenCalledTimes(1)); await h.tab('System Info'); const count = h.list.mock.calls.length;
  killing.resolve(undefined); await tick(); await tick(); expect(h.view.getByRole('tab', { name: 'System Info' }).getAttribute('aria-selected')).toBe('true'); expect(h.list).toHaveBeenCalledTimes(count);
});
it('polls only active tab then stops readers on native close', async () => {
  vi.useFakeTimers(); const h = await harness(); await h.open(); await vi.advanceTimersByTimeAsync(0);
  expect(h.list.mock.calls.length).toBeGreaterThan(0); const initialProcesses = h.list.mock.calls.length;
  await vi.advanceTimersByTimeAsync(2100); expect(h.list.mock.calls.length).toBeGreaterThan(initialProcesses); expect(h.read).not.toHaveBeenCalled();
  await h.tab('System Info'); await vi.advanceTimersByTimeAsync(0); expect(h.read.mock.calls.length).toBeGreaterThan(0);
  const processes = h.list.mock.calls.length; const initialReads = h.read.mock.calls.length;
  await vi.advanceTimersByTimeAsync(4100); expect(h.list).toHaveBeenCalledTimes(processes); expect(h.read.mock.calls.length).toBeGreaterThan(initialReads);
  await h.tab('Processes'); await vi.advanceTimersByTimeAsync(0); const inactiveReads = h.read.mock.calls.length; const resumedProcesses = h.list.mock.calls.length;
  await vi.advanceTimersByTimeAsync(2100); expect(h.list.mock.calls.length).toBeGreaterThan(resumedProcesses); expect(h.read).toHaveBeenCalledTimes(inactiveReads);
  await h.close(); const closedProcesses = h.list.mock.calls.length; const closedReads = h.read.mock.calls.length;
  await vi.advanceTimersByTimeAsync(5000); expect(h.list).toHaveBeenCalledTimes(closedProcesses); expect(h.read).toHaveBeenCalledTimes(closedReads);
});
it('late system snapshot cannot replace newer generation data', async () => {
  const h = await harness(); const old = deferred<unknown>(); h.read.mockReturnValueOnce(old.promise);
  await h.open(); await h.tab('System Info'); await waitFor(() => expect(h.read).toHaveBeenCalled());
  await h.tab('Processes'); await h.close(); await h.open(2); await h.tab('System Info');
  const stale = snapshot(1, 2); stale.inventory.cpuName.value = 'Stale CPU sentinel'; old.resolve(stale); await tick(); await tick(); expect(h.view.queryByText('Stale CPU sentinel')).toBeNull();
});
it('renders independent machine totals, network/storage/inventory and truthful unavailable sensors', async () => {
  const h = await harness(); await h.open(1, 202); await h.tab('System Info'); await waitFor(() => expect(h.view.getByText('Fixture CPU')).toBeTruthy());
  expect(h.view.getByText(/73\s*%/)).toBeTruthy(); expect(h.view.getByText('Fixture VPN')).toBeTruthy(); expect(h.view.getByText('192.0.2.7')).toBeTruthy(); expect(h.view.getByText('Fixture Board')).toBeTruthy();
  expect(h.view.getAllByText(/unavailable|not verified/i).length).toBeGreaterThan(0); expect(h.view.queryByRole('grid', { name: 'Running processes' })).toBeNull(); expect(h.view.container.textContent).not.toMatch(/0\s*°C|serial number|MAC address|product key|UUID/i);
});
it('rejects wrong generation/revision snapshot', async () => {
  const h = await harness(); h.read.mockResolvedValueOnce(snapshot(999, 999)); await h.open(); await h.tab('System Info'); await waitFor(() => expect(h.read).toHaveBeenCalled()); expect(h.view.queryByText('Fixture CPU')).toBeNull();
});

it('unknown snapshot schema version fails closed without displaying telemetry', async () => {
  const h = await harness(); h.read.mockResolvedValueOnce({ ...snapshot(1,2), schemaVersion: 999 });
  await h.open(); await h.tab('System Info'); await waitFor(() => expect(h.read).toHaveBeenCalled()); await tick(); await tick();
  expect(h.view.queryByText('Fixture CPU')).toBeNull();
  expect(h.view.queryByText('Fixture Board')).toBeNull();
});

it('reopening from System Info selects Processes even while telemetry is pending', async () => {
  const h = await harness(); h.read.mockReturnValueOnce(deferred<unknown>().promise);
  await h.open(); await h.tab('System Info'); await h.open(2, 202);
  expect(h.view.getByRole('tab', { name: 'Processes' }).getAttribute('aria-selected')).toBe('true');
  expect(h.filter().value).toBe('202');
});

it('delayed older open event cannot replace latest native opening intent', async () => {
  const h = await harness(); await h.open(5, 202);
  h.handlers.get('process-manager:open')?.({ payload: { focusPid: 101, context: context(4, 1, 'processes', 101) } }); await tick();
  expect(h.filter().value).toBe('202');
});

it('snapshot failure on an inactive tab neither announces stale failure nor blocks process refresh', async () => {
  const h = await harness(); const old = deferred<unknown>(); h.read.mockReturnValueOnce(old.promise);
  await h.open(); await h.tab('System Info'); await waitFor(() => expect(h.read).toHaveBeenCalled()); await h.tab('Processes');
  await waitFor(() => expect(h.view.getByRole('status').textContent).toMatch(/2 processes/));
  const status = h.view.getByRole('status').textContent; old.reject(new Error('old snapshot failure')); await tick(); await tick();
  expect(h.view.getByRole('status').textContent).toBe(status);
  const before = h.list.mock.calls.length; await fireEvent.click(h.view.getByRole('button', { name: 'Refresh process manager' }));
  await waitFor(() => expect(h.list.mock.calls.length).toBeGreaterThan(before));
});

it('registration-delay recovery restores an opening missed before listeners existed and releases listeners', async () => {
  const tauri = tauriMocks(); const openRegistration = deferred<() => void>(); const closeRegistration = deferred<() => void>();
  const releases = [vi.fn(), vi.fn()];
  tauri.listen.mockImplementation((name: string) => name === 'process-manager:open' ? openRegistration.promise : closeRegistration.promise);
  tauri.invoke.mockImplementation((command: string) => Promise.resolve(command === 'get_process_manager_context' ? context(6, 12, 'processes', 202) : command === 'list_processes' ? [processRow(202, 'Recovered opening')] : []));
  const view = render(ProcessManagerSurface); mounted.push(view);
  openRegistration.resolve(releases[0]); closeRegistration.resolve(releases[1]);
  await waitFor(() => expect((view.getByRole('textbox', { name: /Filter processes/ }) as HTMLInputElement).value).toBe('202'));
  await waitFor(() => expect(view.getByText('Recovered opening')).toBeTruthy());
  view.unmount(); mounted.splice(mounted.indexOf(view),1); for (const release of releases) expect(release).toHaveBeenCalledTimes(1);
});

it('snapshot revision never regresses and null unchanged response preserves current data', async () => {
  vi.useFakeTimers(); const h = await harness(); const newest = snapshot(1,2,8); const older = snapshot(1,2,7); older.inventory.cpuName.value = 'Older revision sentinel';
  h.read.mockResolvedValueOnce(newest).mockResolvedValueOnce(older).mockResolvedValue(null);
  await h.open(); await h.tab('System Info'); await vi.advanceTimersByTimeAsync(0); expect(h.view.getByText('Fixture CPU')).toBeTruthy();
  expect(h.read).toHaveBeenCalledTimes(1); await vi.advanceTimersByTimeAsync(4100);
  expect(h.read.mock.calls.length).toBeGreaterThanOrEqual(3); expect(h.view.queryByText('Older revision sentinel')).toBeNull(); expect(h.view.getByText('Fixture CPU')).toBeTruthy();
});

it('tab and snapshot IPC carry only frozen epoch/revision selectors', async () => {
  const h = await harness(); await h.open(4); await h.tab('System Info'); await waitFor(() => expect(h.read).toHaveBeenCalled());
  const activate = h.tauri.invoke.mock.calls.find(([command]) => command === 'set_process_manager_tab')!;
  expect(activate[1]).toEqual({ openGeneration: 4, tabRevision: 2, tab: 'system_info' });
  const read = h.tauri.invoke.mock.calls.find(([command]) => command === 'get_system_info_snapshot')!;
  expect(read[1]).toEqual(expect.objectContaining({ openGeneration: 4, tabRevision: 2 }));
  expect(Object.keys(read[1] as object).every((key) => ['openGeneration','tabRevision','afterRevision'].includes(key))).toBe(true);
});

it('delayed native generation-1 close cannot retire pending PID-202 generation-2 data or its polling', async () => {
  vi.useFakeTimers();const h=await harness();await h.open(1,101);await vi.advanceTimersByTimeAsync(0);
  const oldClose=h.retireClose();expect(oldClose.retired).toEqual(context(1,2,'processes',101,false));
  const pending=deferred<unknown[]>();h.list.mockReturnValueOnce(pending.promise);
  await h.open(2,202);await vi.advanceTimersByTimeAsync(0);
  const afterOpen=h.list.mock.calls.length;expect(h.filter().value).toBe('202');
  await oldClose.deliver();pending.resolve([processRow(202,'Current PID 202 after delayed close')]);await vi.advanceTimersByTimeAsync(0);
  expect(h.native()).toEqual(context(2,3,'processes',202));
  expect(h.filter().value).toBe('202');expect(h.view.getByText('Current PID 202 after delayed close')).toBeTruthy();
  await vi.advanceTimersByTimeAsync(2100);expect(h.list.mock.calls.length).toBeGreaterThan(afterOpen);
  expect(h.view.getByRole('tab',{name:'Processes'}).getAttribute('aria-selected')).toBe('true');
});

it('unversioned or non-retired close notices cannot cancel pending current opening or its polling', async () => {
  vi.useFakeTimers();const h=await harness();const pending=deferred<unknown[]>();h.list.mockReturnValueOnce(pending.promise);
  await h.open(2,202);await vi.advanceTimersByTimeAsync(0);const count=h.list.mock.calls.length;
  for(const payload of [null,{}, {context:context(2,2,'processes',202,true)}]) {
    h.handlers.get('process-manager:closed')?.({payload});await tick();
  }
  pending.resolve([processRow(202,'Current after invalid close notice')]);await vi.advanceTimersByTimeAsync(0);
  expect(h.view.getByText('Current after invalid close notice')).toBeTruthy();expect(h.filter().value).toBe('202');
  await vi.advanceTimersByTimeAsync(2100);expect(h.list.mock.calls.length).toBeGreaterThan(count);
});

it.each(['manager-mounted','direct-view'])('%s labels quota-limited capacity without claiming physical disk capacity', async (mode) => {
  const quota=quotaLimitedSnapshot();
  let view:ReturnType<typeof render>;
  if(mode==='manager-mounted') {
    const h=await harness();h.read.mockResolvedValue(quota.wire);await h.open();await h.tab('System Info');view=h.view;
  } else {
    view=render(SystemInfoView,{snapshot:quota.wire as SystemInfoSnapshot});mounted.push(view);
  }
  await waitFor(()=>expect(view.getByRole('region',{name:'Storage'})).toBeTruthy());
  const storage=within(view.getByRole('region',{name:'Storage'}));
  const capacity=storage.getByText('Caller-visible capacity (quota-aware)').closest('.system-info-metric')!;
  expect(capacity.querySelector('dd strong')?.textContent).toBe('100 GiB');
  expect(storage.getByText('Total free space').closest('.system-info-metric')?.querySelector('dd strong')?.textContent).toBe('300 GiB');
  expect(storage.getByText('Available to current caller (quota-aware)').closest('.system-info-metric')?.querySelector('dd strong')?.textContent).toBe('25 GiB');
  expect(quota.physicalCapacityBytes).toBe(512*1024**3);
  expect(storage.queryByText('512 GiB')).toBeNull();expect(storage.queryByText('Total capacity')).toBeNull();
  expect(view.container.textContent).not.toMatch(/physical (?:disk )?capacity/i);
});

it('metric strip retains dynamic storage, GPU, adapter, address and inventory facts without inferred sensor zeros', () => {
  const view = render(SystemInfoView, { snapshot: metricStripSnapshot() as SystemInfoSnapshot }); mounted.push(view);
  for (const name of ['Overview', 'Storage', 'Sensors', 'Network', 'Hardware']) expect(view.getByRole('region', { name })).toBeTruthy();
  const storage = within(view.getByRole('region', { name: 'Storage' }));
  for (const value of ['100 GiB', '300 GiB', '25 GiB', '64 GiB', '16 GiB', 'ReFS']) expect(storage.getByText(value)).toBeTruthy();
  expect(storage.getByRole('heading', { name: /Z:\\/ })).toBeTruthy();
  const network = within(view.getByRole('region', { name: 'Network' }));
  expect(network.getByRole('heading', { name: /^Fixture VPN/ })).toBeTruthy();
  expect(network.getByRole('heading', { name: /^Fixture Ethernet with a deliberately long adapter name/ })).toBeTruthy();
  for (const value of ['192.0.2.7', '198.51.100.24', '198.51.100.25', '2001:db8:1234:5678:90ab:cdef:1234:5678', '198.51.100.1']) expect(network.getByText(value, { exact: false })).toBeTruthy();
  expect(view.container.textContent).toMatch(/busiest engine, not total GPU/i);
  expect(view.container.textContent).toMatch(/Fixture secondary GPU/);
  expect(view.container.textContent).toMatch(/not summed/i);
  expect(view.container.textContent).not.toMatch(/0\s*°C|MAC address|serial number|product key|UUID/i);
  expect(within(view.getByRole('region', { name: 'Sensors' })).getAllByText('Unavailable').length).toBeGreaterThanOrEqual(2);
});

it('metric strip makes sample provenance and availability reachable through native progressive disclosure', async () => {
  const view = render(SystemInfoView, { snapshot: metricStripSnapshot() as SystemInfoSnapshot }); mounted.push(view);
  const disclosures = Array.from(view.container.querySelectorAll('details'));
  expect(disclosures.length).toBeGreaterThan(0);
  for (const details of disclosures) {
    const summary = details.querySelector('summary'); expect(summary?.textContent?.trim()).toBeTruthy();
    await fireEvent.click(summary!); // Native keyboard operation is certified in the browser, not jsdom.
  }
  const metadata = disclosures.map(node => node.textContent).join(' ');
  for (const source of ['PDH whole-machine', 'GetDiskFreeSpaceExW', 'Fixture mapped capacity', 'Fixture Ethernet receive baseline', 'Fixture Ethernet stale send', 'Fixture secondary GPU unavailable']) expect(metadata).toContain(source);
  expect(metadata).toMatch(/provider not verified/i); expect(metadata).toMatch(/baseline/i); expect(metadata).toMatch(/stale/i);
  expect(metadata).toMatch(/Updated|sampled|timestamp/i);
  for (const state of [/ready/i, /partial/i, /warming[ _-]up/i, /unavailable/i]) expect(metadata).toMatch(state);
  expect(metadata).toMatch(/percent|%/i); expect(metadata).toMatch(/bytes|GiB|KiB|B\/s/i);
});

it('metric strip empty collections remain explicit unavailable/empty results rather than fabricated rows', () => {
  const wire = metricStripSnapshot(); wire.live.gpus = []; wire.live.network = []; wire.storage.roots = [];
  const view = render(SystemInfoView, { snapshot: wire as SystemInfoSnapshot }); mounted.push(view);
  expect(view.getByText(/GPU busiest-engine usage unavailable/i)).toBeTruthy();
  expect(view.getByText(/No storage roots reported/i)).toBeTruthy();
  expect(view.getByText(/No network adapters reported/i)).toBeTruthy();
  expect(view.container.textContent).not.toMatch(/0\s*°C/);
});

it('rapid System Info→Processes→System Info coalesces behind the first same-target ACK', async () => {
  vi.useFakeTimers(); const h = await harness(); await h.open(); await vi.advanceTimersByTimeAsync(0);
  const firstAck = deferred<Context>(); h.acknowledgeTab.mockReturnValueOnce(firstAck.promise);
  const processes = h.list.mock.calls.length;
  await h.tab('System Info'); await h.tab('Processes'); await h.tab('System Info');
  expect(h.acknowledgeTab).toHaveBeenCalledTimes(1); expect(h.maximumCommits()).toBe(1);
  expect(h.native()).toEqual(context(1,2,'system_info'));
  expect(h.view.getByRole('tab', { name: 'System Info' }).getAttribute('aria-selected')).toBe('true');
  await vi.advanceTimersByTimeAsync(4100); expect(h.list).toHaveBeenCalledTimes(processes); expect(h.read).not.toHaveBeenCalled();
  firstAck.resolve(context(1,2,'system_info')); await vi.advanceTimersByTimeAsync(0);
  expect(h.acknowledgeTab).toHaveBeenCalledTimes(1); expect(h.maximumCommits()).toBe(1);
  expect(h.view.getByRole('tab', { name: 'System Info' }).getAttribute('aria-selected')).toBe('true');
  expect(h.read.mock.calls.length).toBeGreaterThan(0); const reads = h.read.mock.calls.length;
  await vi.advanceTimersByTimeAsync(4100); expect(h.read.mock.calls.length).toBeGreaterThan(reads); expect(h.list).toHaveBeenCalledTimes(processes);
  expect(h.native()).toEqual(context(1,2,'system_info'));
  for (const [,args] of h.tauri.invoke.mock.calls.filter(([command]) => command === 'get_system_info_snapshot')) expect(args).toEqual(expect.objectContaining({openGeneration:1,tabRevision:2}));
});

it('late intermediate Processes ACK cannot restore the old tab or overlap the next System Info commit', async () => {
  vi.useFakeTimers(); const h = await harness(); await h.open(); await vi.advanceTimersByTimeAsync(0);
  const a = deferred<Context>(); const b = deferred<Context>(); const c = deferred<Context>();
  h.acknowledgeTab.mockReturnValueOnce(a.promise).mockReturnValueOnce(b.promise).mockReturnValueOnce(c.promise);
  const processes = h.list.mock.calls.length;
  await h.tab('System Info'); await h.tab('Processes'); expect(h.acknowledgeTab).toHaveBeenCalledTimes(1);
  a.resolve(context(1,2,'system_info')); await vi.advanceTimersByTimeAsync(0);
  expect(h.acknowledgeTab).toHaveBeenCalledTimes(2); expect(h.native()).toEqual(context(1,3,'processes'));
  await h.tab('System Info'); expect(h.acknowledgeTab).toHaveBeenCalledTimes(2);
  b.resolve(context(1,3,'processes')); await vi.advanceTimersByTimeAsync(0);
  expect(h.view.getByRole('tab', { name: 'System Info' }).getAttribute('aria-selected')).toBe('true');
  expect(h.acknowledgeTab).toHaveBeenCalledTimes(3); expect(h.maximumCommits()).toBe(1);
  expect(h.native()).toEqual(context(1,4,'system_info'));
  await vi.advanceTimersByTimeAsync(4100); expect(h.list).toHaveBeenCalledTimes(processes); expect(h.read).not.toHaveBeenCalled();
  c.resolve(context(1,4,'system_info')); await vi.advanceTimersByTimeAsync(0);
  expect(h.read.mock.calls.length).toBeGreaterThan(0); expect(h.view.getByRole('tab', { name: 'System Info' }).getAttribute('aria-selected')).toBe('true');
  await vi.advanceTimersByTimeAsync(4100); expect(h.list).toHaveBeenCalledTimes(processes); expect(h.maximumCommits()).toBe(1);
  expect(h.native()).toEqual(context(1,4,'system_info'));
  expect(h.tauri.invoke.mock.calls.filter(([command]) => command === 'set_process_manager_tab').map(([,args]) => args)).toEqual([
    {openGeneration:1,tabRevision:2,tab:'system_info'}, {openGeneration:1,tabRevision:3,tab:'processes'}, {openGeneration:1,tabRevision:4,tab:'system_info'}
  ]);
});

it.each(['Processes','System Info'])('document hidden retires %s polling and pending data; becoming visible alone never reopens', async (activeTab) => {
  vi.useFakeTimers(); const visibility = vi.spyOn(document,'visibilityState','get').mockReturnValue('visible');
  const h = await harness(); await h.open(); await vi.advanceTimersByTimeAsync(0);
  if (activeTab === 'System Info') { await h.tab(activeTab); await vi.advanceTimersByTimeAsync(0); }
  const lateProcesses = deferred<unknown[]>(); const lateSnapshot = deferred<unknown>();
  if (activeTab === 'Processes') h.list.mockReturnValueOnce(lateProcesses.promise);
  else h.read.mockReturnValueOnce(lateSnapshot.promise);
  const initial = activeTab === 'Processes' ? h.list.mock.calls.length : h.read.mock.calls.length;
  await vi.advanceTimersByTimeAsync(activeTab === 'Processes' ? 1100 : 2100);
  expect(activeTab === 'Processes' ? h.list.mock.calls.length : h.read.mock.calls.length).toBeGreaterThan(initial);
  const clear = vi.spyOn(window,'clearInterval'); const beforeClear = clear.mock.calls.length;
  visibility.mockReturnValue('hidden'); await fireEvent(document,new Event('visibilitychange'));
  expect(clear.mock.calls.length).toBeGreaterThan(beforeClear);
  const processCount = h.list.mock.calls.length; const readCount = h.read.mock.calls.length;
  const stale = snapshot(); stale.inventory.cpuName.value = 'Hidden late CPU sentinel';
  lateProcesses.resolve([processRow(101,'Hidden late process sentinel')]); lateSnapshot.resolve(stale);
  await vi.advanceTimersByTimeAsync(5100);
  expect(h.list).toHaveBeenCalledTimes(processCount); expect(h.read).toHaveBeenCalledTimes(readCount);
  expect(h.view.queryByText('Hidden late CPU sentinel')).toBeNull(); expect(h.view.queryByText('Hidden late process sentinel')).toBeNull();
  expect(h.tauri.invoke.mock.calls.some(([command]) => command === 'hide_process_manager')).toBe(false);
  visibility.mockReturnValue('visible'); await fireEvent(document,new Event('visibilitychange')); await vi.advanceTimersByTimeAsync(5100);
  expect(h.list).toHaveBeenCalledTimes(processCount); expect(h.read).toHaveBeenCalledTimes(readCount);
  await h.close(); await h.open(2); await vi.advanceTimersByTimeAsync(0);
  expect(h.view.getByRole('tab', { name: 'Processes' }).getAttribute('aria-selected')).toBe('true');
  expect(h.list.mock.calls.length).toBeGreaterThan(processCount);
  const reopened = h.list.mock.calls.length; await vi.advanceTimersByTimeAsync(1100); expect(h.list.mock.calls.length).toBeGreaterThan(reopened);
});
