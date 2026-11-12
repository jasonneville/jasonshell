import { mount } from 'svelte';
import '../../../src/app.css';
import ProcessManagerSurface from '../../../src/components/ProcessManagerSurface.svelte';
import { open, close, ledger, trace, delayList, delaySnapshot, failList, rows, defaultRows, setRows, nativeContext, retireClose, invalidClose, delayKill, metricStripFixture } from './bridge';
import { snapshot, processRow } from '../../components/fixtures/system-info';
mount(ProcessManagerSurface, { target: document.querySelector('#surface')! });
const surface = document.querySelector('#surface')!;
// Reuse the mounted bridge instance, including Vite's HMR module identity.
Object.assign(window, { __headerFixture: { open, close, setRows, defaultRows } });
Object.assign(window, { __metricStripFixture: { open, close, metricStripFixture } });
const shell=surface.querySelector<HTMLElement>('main')!;shell.style.height='620px';shell.style.width='100%';
const result = document.querySelector('#result')!;
const assert = (condition: unknown, message: string) => { if (!condition) throw new Error(message); };
async function until(check: () => unknown, message: string) {
  const deadline=performance.now()+4000;
  while(performance.now()<deadline) { if (check()) return; await new Promise((resolve) => setTimeout(resolve, 20)); }
  throw new Error(message);
}
const tab = (name: string) => Array.from(surface.querySelectorAll<HTMLButtonElement>('[role="tab"]')).find((node) => node.textContent?.trim() === name);
const filter = () => surface.querySelector<HTMLInputElement>('input[aria-label^="Filter processes"]');
const kills = () => ledger.filter(({ command }) => command === 'kill_process').length;
const button = (name: RegExp) => Array.from(surface.querySelectorAll<HTMLButtonElement>('button')).find((node) => name.test(node.getAttribute('aria-label') ?? node.textContent ?? ''));
document.querySelector<HTMLButtonElement>('#run')!.onclick = async () => {
  const run = document.querySelector<HTMLButtonElement>('#run')!; run.disabled = true; const checks: string[] = [];
  try {
    await until(() => ledger.some(({ command }) => command === 'get_process_manager_context'), 'context registration recovery absent');
    const old = delayList(); open(101); await until(() => ledger.some(({ command }) => command === 'list_processes'), 'opening enumeration absent');
    open(202); old.resolve(rows); await until(() => filter()?.value === '202' && surface.textContent?.includes('Beta'), 'targeted reopen lost'); checks.push('target supersedes pending enumeration');
    button(/^Arm single-process kill confirmation for Beta/)!.click(); window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })); assert(kills() === 0, 'Escape dispatched kill'); checks.push('Escape cancels destructive arming');
    tab('System Info')!.click(); await until(() => surface.textContent?.includes('Fixture CPU'), 'System Info not mounted');
    assert(surface.textContent?.includes('73%') || /73\s+%/.test(surface.textContent ?? ''), 'whole-machine CPU missing'); checks.push('machine metrics independent of PID filter');
    tab('Processes')!.click(); await until(() => filter()?.value === '202', 'filter not preserved');
    button(/^Arm single-process kill confirmation for Beta/)!.click(); assert(kills() === 0, 'tab leave retained destructive arming');
    const late = delaySnapshot(); tab('System Info')!.click(); await until(() => ledger.filter(({ command }) => command === 'get_system_info_snapshot').length >= 2, 'snapshot read absent');
    close(); open(); const stale = snapshot(1, 2); stale.inventory.cpuName.value = 'OLD CPU'; late.resolve(stale);
    await until(() => filter()?.value === '' && tab('Processes')?.getAttribute('aria-selected') === 'true', 'no-PID reopen did not reset'); assert(!surface.textContent?.includes('OLD CPU'), 'late snapshot leaked'); checks.push('dismiss/reopen retires snapshot epoch');
    close(); failList(); open(); await until(() => surface.textContent?.includes('unavailable'), 'enumeration failure not reported');
    close(); open(202); await until(() => surface.textContent?.includes('Beta'), 'failed opening prevented recovery'); checks.push('failure/dismiss/reopen recovery');
    assert(kills() === 0, 'unexpected destructive IPC'); result.textContent = `PASS: ${checks.join('; ')}`;
  } catch (error) { result.textContent = `FAIL: ${String(error)}; passed: ${checks.join('; ')}`; }
  finally { run.disabled = false; }
};

const count=(command:string)=>ledger.filter(entry=>entry.command===command).length;
const delay=(ms:number)=>new Promise(resolve=>setTimeout(resolve,ms));
const selected=(name:string)=>tab(name)?.getAttribute('aria-selected')==='true';
const system=()=>surface.querySelector<HTMLElement>('[aria-label="System information details"]');
const select=async(name:string)=>{tab(name)!.click();await until(()=>selected(name)&&nativeContext().activeTab===(name==='Processes'?'processes':'system_info'),`tab ${name} not committed`);};
const outcome=document.querySelector('#extended-result')!;
const evidence={status:'Not run',checks:[] as string[],measurements:{} as Record<string,unknown>,trace:[] as typeof trace,ledger:[] as typeof ledger};
let trustedKeyCount=0;window.addEventListener('keydown',event=>{if(event.isTrusted)trustedKeyCount++;});
Object.assign(window,{__systemInfoEvidence:evidence});
document.querySelector<HTMLButtonElement>('#extended')!.onclick=async()=>{
  const run=document.querySelector<HTMLButtonElement>('#extended')!;run.disabled=true;evidence.status='RUNNING';evidence.checks=[];trustedKeyCount=0;
  const passed=(name:string)=>{evidence.checks.push(name);outcome.textContent=`RUNNING: ${evidence.checks.length} · ${name}`;};
  try{
    close();setRows([...defaultRows,...Array.from({length:80},(_,i)=>processRow(1000+i,`Worker-${String(i).padStart(3,'0')}`))]);open();
    await until(()=>surface.textContent?.includes('Worker-000'),'initial process rows missing');
    assert(selected('Processes')&&filter()?.value===''&&nativeContext().activeTab==='processes','opening defaults');
    const cpu=Array.from(surface.querySelectorAll('[role="columnheader"]')).find(node=>node.textContent?.includes('CPU'))!;
    assert(cpu.getAttribute('aria-sort')==='descending','opening CPU sort not default');passed('opening defaults Processes / no filter / CPU descending');
    const activeCount=count('list_processes');await until(()=>count('list_processes')>activeCount,'active process interval did not poll');passed('real active process interval advances');
    filter()!.value='Worker';filter()!.dispatchEvent(new Event('input',{bubbles:true}));
    const name=Array.from(surface.querySelectorAll<HTMLElement>('[role="columnheader"]')).find(node=>/^Name/.test(node.textContent??''))!;name.click();
    const group=surface.querySelector<HTMLButtonElement>('button[aria-expanded]')!;const controls=group.getAttribute('aria-controls');if(group.getAttribute('aria-expanded')==='true')group.click();
    const grid=surface.querySelector<HTMLElement>('[role="grid"]')!;await delay(50);grid.scrollTop=220;grid.dispatchEvent(new Event('scroll'));assert(grid.scrollTop===220&&grid.scrollHeight>grid.clientHeight,'fixture has no actual process overflow');
    const beforeTab=count('list_processes');await select('System Info');await until(()=>system()?.textContent?.includes('Fixture CPU'),'system snapshot missing');
    const overviewSummary=Array.from(system()!.querySelectorAll<HTMLElement>('summary')).find(node=>node.textContent?.trim()==='Overview sampling details');
    assert(overviewSummary,'Overview sampling disclosure missing');
    assert(!overviewSummary!.closest('details')!.open,'Overview sampling details should start collapsed');
    overviewSummary!.click();
    await until(()=>overviewSummary!.closest('details')!.open&&system()!.innerText.includes('Partial'),'Overview sampling disclosure did not reveal visible Partial state');
    const text=system()!.innerText;
    for(const section of ['Overview','Storage','Sensors','Network','Hardware'])assert(Array.from(system()!.querySelectorAll('h2')).some(h=>h.textContent===section),`section ${section} missing`);
    assert(text.includes('73%')&&text.includes('Fixture VPN')&&text.includes('192.0.2.7')&&text.includes('2001:db8::7'),'independent metrics / local adapters missing');
    assert(text.includes('Caller-visible capacity (quota-aware)')&&text.includes('100 GiB')&&text.includes('300 GiB')&&text.includes('25 GiB'),'quota/free/available copy or values wrong');
    assert(!text.includes('Total capacity')&&!text.includes('512 GiB')&&!/0\s*°C|MAC address|serial number|UUID|product key/i.test(text),'misleading/private telemetry');
    assert(/provider not verified/i.test(text)&&/Unavailable/.test(text)&&/Partial/.test(text),'partial/unavailable sensor truth missing');
    evidence.measurements.systemText=text;passed('five System sections, independent totals, per-adapter addresses, quota-aware capacity, truthful partial/unavailable sensors');
    const reads=count('get_system_info_snapshot');await until(()=>count('get_system_info_snapshot')>reads,'active system interval did not poll');assert(count('list_processes')===beforeTab,'inactive Processes still polled');passed('real system interval advances while process interval stays retired');
    await select('Processes');await until(()=>filter()?.value==='Worker','process filter lost');
    const restored=Array.from(surface.querySelectorAll<HTMLButtonElement>('button[aria-controls]')).find(node=>node.getAttribute('aria-controls')===controls)!;
    assert(restored.getAttribute('aria-expanded')==='false','group collapse lost');assert(name.getAttribute('aria-sort')==='ascending','explicit Name sort lost');assert(grid.scrollTop===220,'process viewport lost');
    evidence.measurements.processCheckpoint={filter:filter()!.value,sort:name.getAttribute('aria-sort'),groupControls:controls,expanded:restored.getAttribute('aria-expanded'),scrollTop:grid.scrollTop,scrollHeight:grid.scrollHeight,clientHeight:grid.clientHeight};passed('filter / explicit sort / collapsed group / real scroll offset survive tab roundtrip');
    close();setRows([...defaultRows]);const pendingOld=delayList();open(101);const firstLists=count('list_processes');open(202);pendingOld.resolve([processRow(101,'STALE ENUMERATION')]);
    await until(()=>filter()?.value==='202'&&surface.textContent?.includes('Beta'),'latest targeted opening lost');assert(!surface.innerHTML.includes('STALE ENUMERATION'),'stale enumeration published');assert(count('list_processes')<=firstLists+1,'opening enumeration queued without bound');passed('latest targeted PID wins pending old enumeration with bounded admission');
    const retired=retireClose();const pendingCurrent=delayList();open(202);retired.deliver();invalidClose(null);invalidClose({});pendingCurrent.resolve([processRow(202,'Current PID after delayed close')]);
    await until(()=>surface.textContent?.includes('Current PID after delayed close'),'retired/invalid close cancelled current result');
    const afterClose=count('list_processes');await until(()=>count('list_processes')>afterClose,'retired close stopped current polling');passed('retired generation close and unversioned notices cannot cancel newer PID result or polling');
    const oldEpoch=nativeContext();const late=delaySnapshot();await select('System Info');close();open(202);const stale=snapshot(oldEpoch.openGeneration,oldEpoch.tabRevision+1);stale.inventory.cpuName.value='STALE CPU';late.resolve(stale);
    await until(()=>selected('Processes')&&filter()?.value==='202'&&surface.textContent?.includes('Beta'),'reopen after telemetry pending lost');assert(!surface.textContent?.includes('STALE CPU'),'retired snapshot leaked');passed('late retired telemetry cannot leak across hide/reopen');
    close();failList();open();await until(()=>surface.textContent?.includes('unavailable'),'enumeration failure not reported');close();open();await until(()=>surface.textContent?.includes('Alpha'),'reopen after failure did not recover');assert(filter()?.value==='','no-PID opening retained targeted filter');passed('failure / dismissal / no-PID reopen recover and reset intent');
    const killBefore=kills();const hidesBefore=count('hide_process_manager');button(/^Arm single-process kill confirmation for Alpha/)!.click();window.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}));
    assert(kills()===killBefore&&count('hide_process_manager')===hidesBefore,'Escape did not cancel arming safely');
    button(/^Arm single-process kill confirmation for Alpha/)!.click();await select('System Info');await select('Processes');assert(!!button(/^Arm single-process kill confirmation for Alpha/),'tab leave retained kill arm');passed('Escape and leaving Processes clear destructive arming');
    const dispatched=delayKill();button(/^Arm single-process kill confirmation for Alpha/)!.click();await until(()=>!!button(/^Confirm kill for Alpha/),'confirmation control did not render');button(/^Confirm kill for Alpha/)!.click();await until(()=>kills()===killBefore+1,'confirmed mock kill not dispatched');await select('System Info');const processCount=count('list_processes');dispatched.resolve(undefined);await delay(50);
    assert(selected('System Info')&&count('list_processes')===processCount,'kill completion restored old tab or restarted enumeration');const killCall=ledger.filter(entry=>entry.command==='kill_process').at(-1)!;assert((killCall.args as {pid:number}).pid===101,'wrong mock target');passed('confirmed identity-bound mock kill completion is tab-neutral');
    button(/^Close process manager$/)!.click();await until(()=>count('hide_process_manager')>hidesBefore,'close control did not invoke native hide');const quietProcesses=count('list_processes');const quietReads=count('get_system_info_snapshot');await delay(2200);
    assert(count('list_processes')===quietProcesses&&count('get_system_info_snapshot')===quietReads,'closed surface polling continued');open(202);await until(()=>selected('Processes')&&filter()?.value==='202'&&surface.textContent?.includes('Beta'),'explicit reopen failed');passed('close control invokes native hide and both readers remain stopped until explicit reopen');
    (surface as HTMLElement).style.width='360px';tab('Processes')!.focus();tab('Processes')!.dispatchEvent(new KeyboardEvent('keydown',{key:'ArrowRight',bubbles:true,cancelable:true}));await until(()=>selected('System Info')&&document.activeElement===tab('System Info'),'ArrowRight tab focus/selection failed');
    await until(()=>!!system()?.textContent?.includes('Fixture CPU'),'narrow system panel missing');const focused=tab('System Info')!;
    await until(()=>focused.matches(':focus-visible')&&Number(getComputedStyle(focused).boxShadow.match(/([\d.]+)px$/)?.[1]??0)>=1,'keyboard focus ring did not finish painting');
    const focusStyle=getComputedStyle(focused);const bounds=focused.getBoundingClientRect();const frame=shell.getBoundingClientRect();
    assert(trustedKeyCount>0,'start this keyboard journey with wmux browser type on its Run button, not a mouse-only click');
    assert(focused.matches(':focus-visible')&&focusStyle.boxShadow!=='none','visible keyboard focus ring missing');assert(bounds.left>=frame.left&&bounds.right<=frame.right+1,'focused tab outside narrow host');assert(document.documentElement.scrollWidth<=innerWidth+1,'outer horizontal overflow in narrow host');
    evidence.measurements.narrow={viewportWidth:innerWidth,hostWidth:frame.width,activeElement:document.activeElement?.id,trustedKeyCount,focusVisible:focused.matches(':focus-visible'),focusShadow:focusStyle.boxShadow,outerScrollWidth:document.documentElement.scrollWidth};
    focused.dispatchEvent(new KeyboardEvent('keydown',{key:'Home',bubbles:true,cancelable:true}));await until(()=>selected('Processes')&&document.activeElement===tab('Processes'),'Home tab focus failed');tab('Processes')!.dispatchEvent(new KeyboardEvent('keydown',{key:'End',bubbles:true,cancelable:true}));await until(()=>selected('System Info')&&document.activeElement===tab('System Info'),'End tab focus failed');passed('360px host: ArrowRight / Home / End keep visible focus and correct tab without outer overflow');
    evidence.status='PASS';evidence.trace=[...trace];evidence.ledger=[...ledger];outcome.textContent=`PASS: ${evidence.checks.length} checks · ${evidence.checks.join('; ')}`;
  }catch(error){evidence.status='FAIL';evidence.measurements.failureStack=error instanceof Error?error.stack:String(error);evidence.trace=[...trace];evidence.ledger=[...ledger];outcome.textContent=`FAIL: ${String(error)} · passed ${evidence.checks.length}: ${evidence.checks.join('; ')}`;}
  finally{run.disabled=false;}
};
