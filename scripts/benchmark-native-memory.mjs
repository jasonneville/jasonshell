// Fixture-only. Run from repository root; requires rustc and existing serde rlibs.
// No Cargo build, downloads, app index discovery, model, shell, PTY, or publication.
// Counts are requested allocator bytes (realloc includes its new requested size),
// not live/peak allocator bytes or application private bytes. Setup/compile excluded.
// Terminal thread wrapper is made synchronous for deterministic bounded queue tests;
// its actual reader/decoder/body is retained. Icon lookup is a deterministic stub.
// Clipboard here simulates HGLOBAL; benchmark-native-clipboard.mjs uses actual Win32.
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const out=path.resolve('test-results/native-memory'); fs.mkdirSync(out,{recursive:true});
if(process.argv.includes('--capture-before')){
 const directory=path.join(out,'before');
 if(fs.existsSync(path.join(directory,'source-snapshots.json')))throw Error('BEFORE snapshot already exists; refusing overwrite');
 fs.mkdirSync(directory,{recursive:true});
 const files=['snipping/clipboard.rs','search/providers/apps.rs','stack_popup/terminal.rs','process_manager.rs','speech_runtime.rs','search/matcher.rs','search/contracts.rs'];
 fs.writeFileSync(path.join(directory,'source-snapshots.json'),JSON.stringify(Object.fromEntries(files.map(p=>[p,fs.readFileSync(`src-tauri/src/${p}`,'utf8')])),null,2));
 console.log('Captured immutable BEFORE source. Rerun with --before.');process.exit(0);
}
const frozen=process.argv.includes('--before')?JSON.parse(fs.readFileSync(path.join(out,'before/source-snapshots.json'),'utf8')):null;
const read=p=>(frozen?.[p]??fs.readFileSync(`src-tauri/src/${p}`,'utf8')).replaceAll('\r\n','\n');
function item(s,name,kind='fn') {
 const re=new RegExp(`^(?:pub(?:\\(crate\\))? )?${kind} ${name}(?:[<( :{]|$)`,'m');
 const m=re.exec(s); if(!m) throw Error(`missing ${kind} ${name}`);
 const end=s.indexOf('\n}',m.index); if(end<0) throw Error(name);
 return s.slice(m.index,end+2);
}
function span(s,needle){const start=s.indexOf(needle);if(start<0)throw Error(`missing ${needle}`);return s.slice(start,s.indexOf('\n}',start)+2);}
const apps=read('search/providers/apps.rs'),clip=read('snipping/clipboard.rs'),term=read('stack_popup/terminal.rs'),proc=read('process_manager.rs'),speech=read('speech_runtime.rs'),contracts=read('search/contracts.rs');
const plain=s=>s.replace(/^\s*#\[serde[^\n]*\]\s*\n/gm,'');
fs.copyFileSync('tests/fixtures/native-memory/fixtures.rs',path.join(out,'fixtures.rs'));
let generated='use std::sync::Arc;\n';
for(const n of ['SearchResult','SearchResultAction','SearchResultKind','SearchProviderId','SearchProviderCacheState']) generated+=`#[derive(Clone,Debug,PartialEq,serde::Serialize,serde::Deserialize)]\n${n==='SearchResultAction'?'#[serde(tag="kind",rename_all="camelCase")]':'#[serde(rename_all="camelCase")]'}\n${item(contracts,n,n==='SearchResult'?'struct':'enum')}\n`;
for(const n of ['AppIndexEntry','CachedAppIndex','CachedAppEntriesSnapshot']) generated+=`#[derive(Clone,Debug${n==='AppIndexEntry'?',serde::Serialize,serde::Deserialize':''})]\n${n==='AppIndexEntry'?'#[serde(rename_all="camelCase")]':''}\n${item(apps,n,'struct')}\n`;
generated+=`const APP_INDEX_CACHE_VERSION:u32=2;\n#[derive(serde::Serialize,serde::Deserialize)]\n#[serde(rename_all="camelCase")]\n${item(apps,'PersistedAppIndexCache','struct')}\n`;
generated+=item(apps,'From<PersistedAppIndexCache> for CachedAppIndex','impl')+'\n';
let view='PersistedAppIndexCache';
if(apps.includes("struct BorrowedPersistedAppIndexCache<'a>")){
 view='BorrowedPersistedAppIndexCache';
 generated+=`#[derive(serde::Serialize)]\n#[serde(rename_all="camelCase")]\n${item(apps,view,'struct')}\n`;
 generated+=span(apps,"impl<'a> From<&'a CachedAppIndex> for BorrowedPersistedAppIndexCache<'a>")+'\n';
}else generated+=item(apps,'From<&CachedAppIndex> for PersistedAppIndexCache','impl')+'\n';
generated+=`fn serialize_current(cache:&CachedAppIndex)->Vec<u8>{serde_json::to_vec(&${view}::from(cache)).unwrap()}\n`;
for(const n of ['read_persisted_app_index','write_persisted_app_index']) generated+=item(apps,n)+'\n';
generated+=item(apps,'CachedAppIndex','impl')+'\n';
for(const n of ['cached_app_entries_from_cache','cache_age_secs','rank_apps','app_result','score_app','source_rank','app_source_priority_signal','query_tokens','token_terms','normalize_record_key','normalize']) generated+=item(apps,n)+'\n';
generated+=`mod matcher { ${read('search/matcher.rs').split('#[cfg(test)]')[0]} }\nuse matcher::{best_match,full_highlight,MatchData,MatchField};\nuse matcher::query_tokens as match_query_tokens;\n`;
if(clip.includes('const fn crc_table()')){
 generated+=item(clip,'crc_table','const fn')+'\n';
 const table=clip.match(/^static CRC_TABLE\b[^\n]*$/m);if(!table)throw Error('module CRC table declaration missing');
 generated+=table[0]+'\n';
}
if(clip.includes('fn update_adler('))generated+=item(clip,'update_adler')+'\n';
generated+=item(clip,'crc')+'\n'+item(clip,'png_lengths')+'\n'+item(clip,'encode_png')+'\n'+item(clip,'encode_rgba')+'\n';
generated+=item(clip,'prepare').replace('unsafe { RegisterClipboardFormatW(w!("PNG")) }','1u32')+'\n';
generated+=item(term,'decode_terminal_output_chunk')+'\n'+item(term,'flush_terminal_output_decoder')+'\n';
generated+=item(term,'spawn_terminal_reader').replace('thread::spawn(move || {','{').replaceAll('text: text.clone(),','text: counted_text_clone(&text),').replace(/\n    \}\);\n\}$/,'\n    }\n}')+'\n';
for(const n of ['descendant_pids','children_by_parent','enrich_process_tree'])generated+=item(proc,n)+'\n';
const window=item(speech,'transcribe_window');const flatten=window.slice(window.indexOf('{')+1,window.indexOf('    validate_audio(&audio)?;')); if(!flatten.includes('audio'))throw Error('speech flatten seam changed');
generated+=`fn flatten_current(segments:&VecDeque<Box<[f32]>>)->Vec<f32>{ ${flatten} audio }\n`;
generated+=item(speech,'validate_audio')+'\n';
fs.writeFileSync(path.join(out,'generated.rs'),generated);
fs.writeFileSync(path.join(out,frozen?'before-source-hashes.json':'source-hashes.json'),JSON.stringify(Object.fromEntries(['snipping/clipboard.rs','search/providers/apps.rs','stack_popup/terminal.rs','process_manager.rs','speech_runtime.rs'].map(p=>[p,createHash('sha256').update(read(p)).digest('hex')])),null,2));
const deps=process.env.NATIVE_FIXTURE_DEPS??'C:/dev/jasonshell/src-tauri/target/debug/deps';
const lib=n=>path.join(deps,fs.readdirSync(deps).filter(p=>p.startsWith(`lib${n}-`)&&p.endsWith('.rlib')).sort()[0]);
const build=spawnSync('rustc',['--edition=2021','--test','-O',...(apps.includes('entries: Arc<[AppIndexEntry]>')?['--cfg','shared_cache_fixture']:[]),'-L',`dependency=${deps}`,'--extern',`serde=${lib('serde')}`,'--extern',`serde_json=${lib('serde_json')}`,path.join(out,'fixtures.rs'),'-o',path.join(out,'fixtures.exe')],{encoding:'utf8'});
fs.writeFileSync(path.join(out,'environment.json'),JSON.stringify({rustc:spawnSync('rustc',['--version'],{encoding:'utf8'}).stdout.trim(),profile:'rustc --test -O --edition=2021',deps,serde:lib('serde'),serde_json:lib('serde_json'),platform:process.platform,source:frozen?'immutable BEFORE':'working tree',allocationScope:'thread-local requested bytes; realloc new sizes included; fixture-only'},null,2));
fs.writeFileSync(path.join(out,'compile.txt'),build.stdout+build.stderr);if(build.status){console.error(build.stderr);process.exit(build.status);}
const args=process.argv.slice(2).filter(x=>x!=='--before');const run=spawnSync(path.join(out,'fixtures.exe'),['--nocapture','--test-threads=1',...args],{encoding:'utf8'});
fs.writeFileSync(path.join(out,(frozen?'before-supplementary-':'')+(args.includes('--skip')?'baseline.txt':'raw.txt')),run.stdout+run.stderr); console.log(run.stdout+run.stderr);process.exit(run.status??1);
