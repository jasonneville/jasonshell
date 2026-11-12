import { context, deferred, processRow, quotaLimitedSnapshot, metricStripSnapshot } from '../../components/fixtures/system-info';
type Handler = (event: { payload: unknown }) => void;
let native = context(0, 0, 'processes', null, false);
const handlers = new Map<string, Set<Handler>>();
export const ledger: Array<{ command: string; args: unknown }> = [];
export const trace: Array<{atMs:number;event:string;payload:unknown}>=[];
function record(event:string,payload:unknown) {trace.push({atMs:Math.round(performance.now()),event,payload:structuredClone(payload)});if(trace.length>1000)trace.shift();}
let pendingList: ReturnType<typeof deferred<unknown[]>> | null = null;
let pendingSnapshot: ReturnType<typeof deferred<unknown>> | null = null;
let failNext = false;
let pendingKill: ReturnType<typeof deferred<unknown>> | null = null;
let useMetricStripFixture = false;
export function metricStripFixture(enabled = true) { useMetricStripFixture = enabled; }
export const defaultRows = [processRow(101, 'Alpha'), processRow(202, 'Beta')];
export let rows=[...defaultRows];
export function setRows(next:typeof rows) {rows=next;}
export function nativeContext(){return {...native};}
function emit(name: string, payload: unknown) {record(name,payload);for (const handler of handlers.get(name) ?? []) handler({ payload }); }
export async function listen(name: string, handler: Handler) {
  const set = handlers.get(name) ?? new Set(); handlers.set(name, set); set.add(handler); return () => { set.delete(handler); };
}
export function getCurrentWindow() { return { label: 'process-manager', listen }; }
export async function invoke(command: string, args: Record<string, unknown> = {}) {
  ledger.push({ command, args });if(ledger.length>1000)ledger.shift();record(command,args); document.querySelector('#ledger')!.textContent = JSON.stringify(ledger, null, 2);
  if (command === 'get_process_manager_context') return native;
  if (command === 'set_process_manager_tab') {
    if (args.openGeneration !== native.openGeneration || !Number.isSafeInteger(args.tabRevision) || Number(args.tabRevision) <= native.tabRevision) throw new Error('stale');
    native = { ...native, tabRevision: Number(args.tabRevision), activeTab: String(args.tab) }; return native;
  }
  if (command === 'get_system_info_snapshot') {
    if (pendingSnapshot) { const pending = pendingSnapshot; pendingSnapshot = null; return pending.promise; }
    if (args.openGeneration !== native.openGeneration || args.tabRevision !== native.tabRevision || native.activeTab !== 'system_info' || !native.visible) throw new Error('stale');
    const wire=useMetricStripFixture ? metricStripSnapshot() : quotaLimitedSnapshot().wire;wire.openGeneration=native.openGeneration;wire.tabRevision=native.tabRevision;wire.snapshotRevision=ledger.length;return wire;
  }
  if (command === 'list_processes') {
    if (pendingList) { const pending = pendingList; pendingList = null; return pending.promise; }
    if (failNext) { failNext = false; throw new Error('fixture enumeration unavailable'); } return rows;
  }
  if (command === 'list_taskbar_process_windows') return [];
  if (command === 'hide_process_manager') { close(); return; }
  if (command === 'kill_process') {if(pendingKill){const pending=pendingKill;pendingKill=null;return pending.promise;}return;}
  throw new Error(`unexpected IPC ${command}`);
}
export function open(focusPid: number | null = null) { native = context(native.openGeneration + 1, native.tabRevision + 1, 'processes', focusPid); emit('process-manager:open', { focusPid, context: native }); }
export function retireClose() {native={...native,visible:false,tabRevision:native.tabRevision+1};const context={...native};return {context,deliver:()=>emit('process-manager:closed',{context})};}
export function close() {retireClose().deliver();}
export function invalidClose(payload:unknown){emit('process-manager:closed',payload);}
export function delayList() { pendingList = deferred<unknown[]>(); return pendingList; }
export function delaySnapshot() { pendingSnapshot = deferred<unknown>(); return pendingSnapshot; }
export function failList() { failNext = true; }
export function delayKill(){pendingKill=deferred<unknown>();return pendingKill;}
