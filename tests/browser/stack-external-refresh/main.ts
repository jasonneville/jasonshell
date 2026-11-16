import '../../../src/app.css';
import { mount, unmount, tick } from 'svelte';
import StackPopupSurface from '../../../src/components/StackPopupSurface.svelte';
import { root, child, other, context, summary, mutate, notify, deferred, ledger, overrides, emit, setVisible, resetBridge, installLargeDirectory } from './bridge';
let popup: ReturnType<typeof mount> | undefined;
const checks: string[] = [];
const snapshots: {phase:string;summary:number;metadata:number;starts:number;stops:number}[] = [];
const element = (selector:string) => document.querySelector<HTMLElement>(selector);
const count = (command:string) => ledger.filter((entry) => entry.command === command).length;
const row = (path:string) => element(`[data-stack-entry-path="${CSS.escape(path)}"]`);
const settle = () => new Promise((resolve) => setTimeout(resolve, 340));
function assert(value:unknown,label:string) { if (!value) throw new Error(label); checks.push(label); }
async function until(predicate:()=>unknown,label:string) { const deadline=performance.now()+4000; while(!predicate()) { if(performance.now()>deadline) throw new Error(label); await new Promise((resolve)=>setTimeout(resolve,10)); } }
function snapshot(phase:string) { snapshots.push({ phase, summary:count('get_stack_git_summary'), metadata:count('get_stack_git_metadata'), starts:count('start_stack_folder_watch'), stops:count('stop_stack_folder_watch') }); }
async function navigate(path:string) {
  const input=element('input[aria-label="Current folder path"]') as HTMLInputElement; input.value=path; input.dispatchEvent(new Event('input',{bubbles:true})); input.closest('form')!.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true}));
  await until(()=>element('.stack-path')?.getAttribute('title')===path,`Navigation failed: ${path}`); await settle();
}
async function run() {
  if(popup) await unmount(popup); resetBridge(); checks.length=0; snapshots.length=0;
  popup=mount(StackPopupSurface,{target:element('#surface')!});
  await until(()=>count('start_stack_folder_watch')===1&&Boolean(row(`${root}\\draft.txt`)), 'Native watch start missing'); await settle(); snapshot('initial');
  const initial=count('get_stack_git_summary'); const branch=element('.stack-git-branch');
  mutate(1); await notify(); await until(()=>Boolean(row(`${root}\\external-1.txt`)), 'External file row did not auto-update');
  await until(()=>row(`${root}\\draft.txt`)?.getAttribute('data-git-status')==='modified','External Git badge did not auto-update');
  assert(count('get_stack_git_summary')===initial+1,'External event refreshed lightweight summary without manual Refresh');
  assert(count('get_stack_git_status')===0,'External update did not load full workbench enrichment'); snapshot('external');
  let before=count('get_stack_git_summary'); mutate(2); await notify(); await notify(); await notify();
  await until(()=>Boolean(row(`${root}\\external-2.txt`)),'Burst latest row missing'); await settle();
  assert(count('get_stack_git_summary')===before+1,'Three-event burst coalesced into one refresh');
  const held=deferred<ReturnType<typeof summary>>(); let requests=0;
  overrides.set('get_stack_git_summary',({path})=>++requests===1?held.promise:summary(String(path)));
  before=count('get_stack_git_summary'); mutate(3); const old=summary(root); await notify();
  await until(()=>count('get_stack_git_summary')===before+1,'Held summary never started');
  mutate(4); await notify(); await notify(); await notify(); await settle();
  assert(count('get_stack_git_summary')===before+1,'Pending external refresh stayed single-flight'); held.resolve(old);
  await until(()=>count('get_stack_git_summary')===before+2&&Boolean(row(`${root}\\external-4.txt`)),'Dirty followup lost latest mutation'); await settle();
  assert(count('get_stack_git_summary')===before+2,'Events during refresh produced exactly one queued followup'); snapshot('followup');
  const searches=element('input[aria-label="Search current folder"]') as HTMLInputElement;
  row(`${root}\\draft.txt`)!.click(); searches.value='draft'; searches.dispatchEvent(new Event('input',{bubbles:true})); searches.focus();
  before=count('get_stack_git_summary'); mutate(5); await notify(); await until(()=>count('get_stack_git_summary')===before+1,'Focused refresh missing'); await settle();
  assert(document.activeElement===searches&&searches.value==='draft','External update preserved search focus and query');
  assert(row(`${root}\\draft.txt`)?.getAttribute('aria-selected')==='true','External update retained surviving selection');
  searches.value=''; searches.dispatchEvent(new Event('input',{bubbles:true})); await tick();
  before=count('get_stack_git_summary'); await navigate(child); await navigate(root);
  assert(count('get_stack_git_summary')===before,'Ordinary same-repo navigation without changes made zero Git scans');
  assert(count('start_stack_folder_watch')===1,'Same repository reused its existing recursive native subscription');
  assert(element('.stack-git-branch')===branch,'Same-repo external refresh/navigation retained branch DOM');
  window.dispatchEvent(new Event('blur')); await emit('tauri://focus-changed',false); mutate(6); await notify();
  await until(()=>Boolean(row(`${root}\\external-6.txt`)),'Visible-unfocused external update missing'); assert(count('stop_stack_folder_watch')===0,'Visible-unfocused popup kept watching');
  setVisible(false); await emit('stack-popup:closed'); await until(()=>count('stop_stack_folder_watch')===1,'Hide did not stop watch');
  before=count('get_stack_git_summary'); mutate(7); await notify('watch-1'); await settle();
  assert(count('get_stack_git_summary')===before,'Hidden popup rejected retired token event');
  setVisible(true); await emit('stack-popup:open',{path:root,requestId:'browser-external-reopen'});
  await until(()=>count('start_stack_folder_watch')===2&&Boolean(row(`${root}\\external-7.txt`)),'Reopen missed hidden external mutation'); snapshot('reopen');
  await navigate(other); before=count('get_stack_git_summary'); await notify('watch-2'); await settle();
  assert(count('get_stack_git_summary')===before,'Old repository token did not refresh newer repository');
  await unmount(popup); popup=undefined; const starts=count('start_stack_folder_watch');
  await notify(`watch-${starts}`); await settle(); assert(count('get_stack_git_summary')===before,'Unmount retired event ownership'); snapshot('unmounted');
  popup=mount(StackPopupSurface,{target:element('#surface')!});
  await until(()=>count('start_stack_folder_watch')===starts+1&&Boolean(row(`${root}\\draft.txt`)),'Editor phase not ready'); await settle();
  row(`${root}\\draft.txt`)!.dispatchEvent(new MouseEvent('dblclick',{bubbles:true}));
  await until(()=>Boolean(element('.cm-content')),'Production editor missing');
  const content=element('.cm-content')!; content.focus(); const selection=window.getSelection()!; const range=document.createRange(); range.selectNodeContents(content); selection.removeAllRanges(); selection.addRange(range);
  assert(document.execCommand('insertText',false,'unsaved external-agent draft'),'Actual CodeMirror accepted unsaved fixture draft');
  await until(()=>element('.stack-text-editor-draft-state')?.textContent?.includes('Draft changed'),'Editor draft did not become dirty');
  const fileReads=count('read_stack_basic_text_file'); before=count('get_stack_git_summary'); mutate(8); await notify();
  await until(()=>count('get_stack_git_summary')===before+1,'External editor-phase summary missing'); await settle();
  assert(element('.cm-content')===content&&content.textContent==='unsaved external-agent draft','External update preserved actual dirty editor DOM and draft');
  assert(document.activeElement===content,'External update did not steal editor focus');
  assert(count('read_stack_basic_text_file')===fileReads&&count('save_stack_basic_text_file')===0,'External update did not reload or save editor file');
  await unmount(popup); installLargeDirectory(); const largeStarts=count('start_stack_folder_watch');
  popup=mount(StackPopupSurface,{target:element('#surface')!});
  await until(()=>count('start_stack_folder_watch')===largeStarts+1&&Boolean(element('.details-body')),'Large-directory phase not ready'); await settle();
  const body=element('.details-body')!; body.scrollTop=5000; body.dispatchEvent(new Event('scroll')); await settle();
  const selected=element('[data-stack-entry-path][role="row"]')!; selected.click(); const selectedPath=selected.getAttribute('data-stack-entry-path');
  const offset=body.scrollTop; assert(offset>1000,'Large-directory fixture has real browser scroll');
  before=count('get_stack_git_summary'); mutate(9); await notify(); await until(()=>count('get_stack_git_summary')===before+1,'Large-directory external refresh missing'); await settle();
  assert(Math.abs(body.scrollTop-offset)<2,'External refresh preserved real large-folder scroll offset');
  assert(element('[data-stack-entry-path][aria-selected="true"]')?.getAttribute('data-stack-entry-path')===selectedPath,'External refresh preserved large-folder selection');
  await unmount(popup); popup=undefined; snapshot('complete');
}
element('#run')!.onclick=()=>{element('#result')!.textContent='Running';const publish=()=>Object.assign(window,{stackExternalRefreshEvidence:{checks:checks.slice(),snapshots:snapshots.slice(),ledger:ledger.slice()}});void run().then(()=>{element('#result')!.textContent='PASS';publish();}).catch((error)=>{element('#result')!.textContent=`FAIL: ${String(error)}`;publish();});};

async function runReviewRegressions() {
  const results: {name:string;status:string;error?:string;checks:string[];ledger:typeof ledger}[] = [];
  for (const name of ['navigation-during-context', 'missing-trusted-git']) {
    if (popup) await unmount(popup); popup=undefined; resetBridge(); checks.length=0;
    try {
      if (name==='missing-trusted-git') overrides.set('get_stack_git_context',()=>Promise.reject(new Error('Trusted Git executable unavailable')));
      popup=mount(StackPopupSurface,{target:element('#surface')!});
      await until(()=>Boolean(row(`${root}\\draft.txt`)),'Initial folder rows missing');
      await until(()=>count('start_stack_folder_watch')===1,`Plain-folder watch expected1 observed${count('start_stack_folder_watch')} despite rejected Git context`);
      await settle();
      if(name==='navigation-during-context') {
        const before=count('get_stack_git_summary'); const initialContexts=count('get_stack_git_context');
        const held=deferred<ReturnType<typeof context>>(); let requests=0;
        overrides.set('get_stack_git_context',({path})=>++requests===1?held.promise:context(String(path)));
        mutate(2); await notify();
        await until(()=>count('get_stack_git_context')===initialContexts+1,'External context request did not start');
        await navigate(child); assert(count('get_stack_git_summary')===before,'Held external context did not trigger a navigation scan');
        held.resolve(context(other));
        await until(()=>count('get_stack_git_summary')===before+1,'Lost-work summary followup expected delta1 after stale context release');
        await settle(); assert(count('get_stack_git_summary')===before+1,'Exactly one dirty summary followup survived navigation');
        assert(count('start_stack_folder_watch')===1&&count('stop_stack_folder_watch')===0,'Dirty followup reused same native subscription');
        assert(element('.stack-path')?.getAttribute('title')===child&&Boolean(row(`${child}\\external-2.txt`))&&!row(`${root}\\external-2.txt`),'No stale rows published after context release');
        assert(element('.stack-git-count.git-status-modified')?.textContent?.includes('2'),'Latest Git summary published after navigation');
        assert(!ledger.filter((call)=>call.command==='get_stack_git_summary').some((call)=>call.args.path===other),'Obsolete context identity never used for summary');
      } else {
        assert(count('get_stack_git_summary')===0&&!element('.stack-git-branch'),'Missing Git has no fabricated repository identity');
        mutate(3); await notify(); await until(()=>Boolean(row(`${root}\\external-3.txt`)),'Plain-folder external rows did not refresh while Git context rejects');
        assert(!element('[data-git-status]')&&count('get_stack_git_summary')===0,'Plain-folder refresh publishes no wrong Git badges or scans');
        assert(count('start_stack_folder_watch')===1,'Plain-folder external refresh reuses native subscription');
      }
      results.push({name,status:'PASS',checks:checks.slice(),ledger:ledger.slice()});
    } catch(error) { results.push({name,status:'FAIL',error:String(error),checks:checks.slice(),ledger:ledger.slice()}); }
    if(popup) await unmount(popup); popup=undefined;
  }
  Object.assign(window,{stackExternalRefreshReviewEvidence:results});
  element('#review-result')!.textContent=results.every((result)=>result.status==='PASS')?'PASS':`FAIL: ${results.filter((result)=>result.status==='FAIL').map((result)=>`${result.name}: ${result.error}`).join('; ')}`;
}
element('#run-review')!.onclick=()=>{element('#review-result')!.textContent='Running';void runReviewRegressions();};
