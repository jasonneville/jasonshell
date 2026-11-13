import { readFileSync, readdirSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import assert from 'node:assert/strict';
export const median = xs => {
  assert(xs.length > 0, 'No samples');
  const a = [...xs].sort((a,b) => a-b), m = Math.floor(a.length/2);
  return a.length % 2 ? a[m] : (a[m-1]+a[m])/2;
};
export function aggregate(sample) {
  const valid = sample.processes.filter(p => Number.isFinite(p.privateBytes));
  const roots = valid.filter(p => p.isRoot);
  assert.equal(roots.length, 1, 'Exactly one identity-matched root required');
  assert.equal(valid.length, sample.processes.length, 'Unreadable process metrics invalidate a sample');
  const root = roots[0], children = valid.filter(p => !p.isRoot);
  const sum = (ps,k) => ps.reduce((n,p) => n+p[k],0);
  return {
    elapsedSeconds:sample.elapsedSeconds,phase:sample.phase,processCount:valid.length,
    rootPrivateMiB:root.privateBytes/1048576, descendantsPrivateMiB:sum(children,'privateBytes')/1048576,
    treePrivateMiB:sum(valid,'privateBytes')/1048576,
    rootWorkingSetMiB:root.workingSet/1048576,descendantsWorkingSetMiB:sum(children,'workingSet')/1048576,
    treeWorkingSetMiB:sum(valid,'workingSet')/1048576,
    liveCpuMs:sum(valid,'cpuMs'),threads:sum(valid,'threads'),handles:sum(valid,'handles'),
  };
}
const range = xs => ({median:median(xs),min:Math.min(...xs),max:Math.max(...xs)});
if (process.argv[2] === '--self-test') {
  assert.equal(median([1,8,3]),3); assert.equal(median([4,2]),3);
  const root={privateBytes:1048576,workingSet:2097152,cpuMs:4,threads:2,handles:10,isRoot:true};
  const child={...root,privateBytes:2097152,isRoot:false};
  assert.equal(aggregate({processes:[root,child]}).treePrivateMiB,3);
  assert.equal(aggregate({processes:[root,child]}).descendantsPrivateMiB,2);
  assert.throws(() => aggregate({processes:[root,root]}));
  assert.throws(() => aggregate({processes:[root,{error:'denied'}]}));
  console.log('PASS: median, private-byte decomposition, duplicate-root and unreadable-process rejection');
} else if(process.argv[2]) {
  const dir=process.argv[2], output=join(dir,'summary.json');
  assert(!existsSync(output),'Refuse overwriting summary');
  const attempts=[];
  for(const name of readdirSync(dir).filter(n => /^pair\d+-(baseline|current)$/.test(n))) {
    const load=f => JSON.parse(readFileSync(join(dir,name,f),'utf8').replace(/^\uFEFF/,''));
    const result=load('result.json');
    assert(result.normalExit && result.exitCode === 0 && result.restored && !result.failure && !result.survivors.length,`Invalid attempt ${name}`);
    const raw=load('series.json'), samples=raw.map(aggregate);
    const phases={};
    const phaseNames=['cold-idle','warm-idle',...(['surface-workflow','post-surface-quiet'].filter(p=>samples.some(s=>s.phase===p)))];
    for(const phase of phaseNames) {
      const s=samples.filter(s => s.phase === phase);
      assert(s.length>=10,`Too few ${phase} samples in ${name}`);
      phases[phase]={sampleCount:s.length,elapsedRange:[s[0].elapsedSeconds,s.at(-1).elapsedSeconds]};
      for(const k of Object.keys(s[0]).filter(k => !['elapsedSeconds','phase'].includes(k))) phases[phase][k]=range(s.map(x=>x[k]));
    }
    let workflow=null;
    if(existsSync(join(dir,name,'workflow.json'))) {
      workflow=load('workflow.json'); assert(!workflow.failure,'Failed native workflow');
      assert.equal(workflow.events.filter(e=>e.name==='process-row-count' && e.count>0).length,3,'Three populated process snapshots required');
      workflow={...workflow,observedReadinessMs: Object.fromEntries(['process-open','terminal-open'].map(prefix=>[prefix,range(workflow.events.filter(e=>e.name.startsWith(prefix)).map(e=>e.durationMs))]))};
    }
    attempts.push({name,variant:name.endsWith('baseline')?'baseline':'current',binaryHash:result.binaryHash,nativeBarsVisibleSeconds:result.readySeconds,phases,workflow,sampledPeakTreePrivateMiB:Math.max(...samples.map(s=>s.treePrivateMiB))});
  }
  const comparison={};
  for(const variant of ['baseline','current']) {
    const a=attempts.filter(a=>a.variant===variant); assert(a.length>=3,'Need three pairs');
    comparison[variant]={attempts:a.length,nativeBarsVisibleSeconds:range(a.map(x=>x.nativeBarsVisibleSeconds)),sampledPeakTreePrivateMiB:range(a.map(x=>x.sampledPeakTreePrivateMiB)),phases:{}};
    for(const phase of Object.keys(a[0].phases)) {
      comparison[variant].phases[phase]={};
      for(const k of Object.keys(a[0].phases[phase]).filter(k=>!['sampleCount','elapsedRange'].includes(k))) comparison[variant].phases[phase][k]=range(a.map(x=>x.phases[phase][k].median));
    }
  }
  const summary={attempts,comparison,limits:['Fresh process/profile, not cold OS page cache','Sampled descendant tree: sub-second children or unobserved reparenting may be missed','Working-set sums double-count shared pages; private bytes are committed private allocation, not resident RAM','Live CPU totals omit already-exited processes; no end-to-end action speed claim','Native bar visibility is not renderer readiness or first-use latency','Optional workflow: process close/reopen and terminal hide/show only, with UIA accessibility overhead; no backend terminal restart, controlled output, icon-folder churn or diff-viewer stress; no optimization savings established','Isolated identifier/profile has no imported speech model; not representative of a model-loaded user profile']};
  writeFileSync(output,JSON.stringify(summary,null,2)+'\n');
  console.log(JSON.stringify(comparison,null,2));
}
