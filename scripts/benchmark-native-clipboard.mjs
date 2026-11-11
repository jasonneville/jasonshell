// Actual Win32 allocation/prepare tests. Never creates Publisher or calls OLE clipboard APIs.
import fs from 'node:fs';import path from 'node:path';import {spawnSync} from 'node:child_process';
const out=path.resolve('test-results/native-memory'),deps=process.env.NATIVE_FIXTURE_DEPS??'C:/dev/jasonshell/src-tauri/target/debug/deps';
fs.mkdirSync(out,{recursive:true});
const lib=n=>path.join(deps,fs.readdirSync(deps).filter(p=>p.startsWith(`lib${n}-`)&&p.endsWith('.rlib')).sort()[0]);
const frozen=process.argv.includes('--before')?JSON.parse(fs.readFileSync(path.join(out,'before/source-snapshots.json'),'utf8')):null;
let clip=(frozen?.['snipping/clipboard.rs']??fs.readFileSync('src-tauri/src/snipping/clipboard.rs','utf8')).replaceAll('\r\n','\n');
// Instrument existing accounting only in generated, uncommitted test translation unit.
if(!clip.includes('STAGED.fetch_add(bytes, Ordering::SeqCst); Self(bytes)'))throw Error('staging instrumentation seam changed');
clip=clip.replace('STAGED.fetch_add(bytes, Ordering::SeqCst); Self(bytes)','STAGED.fetch_add(bytes, Ordering::SeqCst); TEST_PEAK.fetch_max(STAGED.load(Ordering::SeqCst), Ordering::SeqCst); Self(bytes)');
const geometry=path.resolve('src-tauri/src/snipping/geometry.rs').replaceAll('\\','/');
const tests=fs.readFileSync('tests/fixtures/native-memory/clipboard-tests.rs','utf8');
const start=clip.indexOf('fn encode_rgba('),end=clip.indexOf('\n}',start)+2;
const tableEncoder=clip.slice(start,end).replace('fn encode_rgba(','fn encode_rgba_table(').replaceAll('crc(', 'crc_table(');
fs.writeFileSync(path.join(out,'clipboard-generated.rs'),`#![allow(dead_code)]\n#[path=${JSON.stringify(geometry)}]mod geometry;\nmod clipboard {\n${clip}\nstatic TEST_PEAK:AtomicUsize=AtomicUsize::new(0);\n${tableEncoder}\n${tests}\n}`);
const args=['--edition=2021','--test','-O','-L',`dependency=${deps}`];for(const n of ['serde','windows','windows_core'])args.push('--extern',`${n}=${lib(n)}`);args.push(path.join(out,'clipboard-generated.rs'),'-o',path.join(out,'clipboard.exe'));
const b=spawnSync('rustc',args,{encoding:'utf8'});fs.writeFileSync(path.join(out,'clipboard-compile.txt'),b.stdout+b.stderr);if(b.status){console.error(b.stderr);process.exit(b.status)}
const r=spawnSync(path.join(out,'clipboard.exe'),['--nocapture','--test-threads=1',...process.argv.slice(2).filter(x=>x!=='--before')],{encoding:'utf8'});fs.writeFileSync(path.join(out,(frozen?'before-supplementary-':'')+(process.argv.includes('--skip')?'clipboard-baseline.txt':'clipboard-red.txt')),r.stdout+r.stderr);console.log(r.stdout+r.stderr);process.exit(r.status??1);
