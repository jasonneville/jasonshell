// Safe extracted-producer/consumer fixture integration. No Svelte mount, Tauri or PTY.
import fs from 'node:fs';import vm from 'node:vm';import assert from 'node:assert/strict';
import {stripTypeScriptTypes} from 'node:module';
import {applySearchPanelPayload,defaultSearchPanelViewState} from '../../../dist-tests/lib/searchPanelState.js';
import {searchEngineProgressToPanelPayload,searchEngineResponseToPanelPayload} from '../../../dist-tests/lib/searchEngine.js';
const source=fs.readFileSync('src/components/TerminalPanelSurface.svelte','utf8').replaceAll('\r\n','\n');
function body(name){const start=source.indexOf(`  function ${name}(`);assert(start>=0);const end=source.indexOf('\n  }',start);assert(end>=0);return source.slice(start,end+4).replaceAll(': StackTerminalOutputChunk','').replaceAll(': TerminalPaneRuntime','');}
const trace=JSON.parse(fs.readFileSync('test-results/native-memory/terminal-trace.json','utf8'));
let makeTracker=()=>new Set();
if(source.includes('import { TerminalSequenceTracker }')){
 const trackerSource=fs.readFileSync('src/lib/terminalSequenceTracker.ts','utf8');
 const {TerminalSequenceTracker}=await import(`data:text/javascript;base64,${Buffer.from(stripTypeScriptTypes(trackerSource)).toString('base64')}`);
 makeTracker=()=>new TerminalSequenceTracker();
}
const context={renderedSequences:makeTracker(),renderedSequenceKeysBySession:new Map(),out:[],isRuntimeCurrent:r=>!r.disposed};
context.writeTerminalOutput=t=>context.out.push(t);context.writeTerminalOutputForRuntime=(r,t)=>context.out.push(t);
vm.createContext(context);vm.runInContext(body('writeTerminalChunk')+'\n'+body('writeTerminalChunkForRuntime'),context);
for(const c of trace.pushed)context.writeTerminalChunk(c);for(const c of trace.polled)context.writeTerminalChunk(c);
assert.equal(context.out.join(''),trace.pushed.map(c=>c.text).join(''));assert.equal(context.out.length,trace.pushed.length);
context.out=[];const runtime={session:{sessionId:'fixture'},renderedSequences:makeTracker(),disposed:false};
for(const c of trace.polled)context.writeTerminalChunkForRuntime(runtime,c);for(const c of trace.pushed)context.writeTerminalChunkForRuntime(runtime,c);
assert.equal(context.out.join(''),trace.pushed.map(c=>c.text).join(''));assert.equal(context.out.length,trace.pushed.length);
context.writeTerminalChunkForRuntime(runtime,{...trace.pushed[0],sessionId:'other'});assert.equal(context.out.length,trace.pushed.length);
runtime.disposed=true;context.writeTerminalChunkForRuntime(runtime,{...trace.pushed[0],sequence:999});assert.equal(context.out.length,trace.pushed.length);
console.log(`terminal native extracted producer -> production extracted consumer: ${trace.pushed.length} push/poll chunks deduped, both delivery orders; stale runtime/other session rejected`);
const results=JSON.parse(fs.readFileSync('test-results/native-memory/search-ranking-trace.json','utf8'));
assert.equal(results.length,30);
const generatedAt='2026-11-11T00:00:00Z';
let state=applySearchPanelPayload(defaultSearchPanelViewState,searchEngineProgressToPanelPayload({query:'fixture',sequence:10,phase:'local',results,providerTimings:[],statusMessage:'local',generatedAt}));
assert.equal(state.results.length,30);assert.equal(state.phase,'local');
state=applySearchPanelPayload(state,searchEngineProgressToPanelPayload({query:'fixture',sequence:10,phase:'provider',results,providerTimings:[],statusMessage:'provider',generatedAt}));
state=applySearchPanelPayload(state,searchEngineProgressToPanelPayload({query:'fixture',sequence:10,phase:'complete',results,providerTimings:[],statusMessage:'complete',generatedAt}));
state=applySearchPanelPayload(state,searchEngineResponseToPanelPayload({query:'fixture',sequence:10,results,providerTimings:[],health:[],generatedAt}));
assert.equal(state.phase,'complete');assert.equal(state.results[0].title,results[0].title);assert.deepEqual(state.results.map(r=>r.recordKey),results.map(r=>r.recordKey));
for(const stale of [{query:'fix',sequence:9},{query:'other',sequence:10}]){const next=applySearchPanelPayload(state,searchEngineProgressToPanelPayload({...stale,phase:'provider',results:[],providerTimings:[],statusMessage:'stale',generatedAt}));assert.deepEqual(next,state);}
console.log('search native ranked/serialized 30 results -> production conversion -> panel state: local/provider/final complete; stale sequence/query rejected');
