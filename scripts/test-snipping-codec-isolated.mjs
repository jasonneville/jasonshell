// Pure existing codec tests, compiled unchanged against existing cached rlibs.
// No Cargo/application link, downloads, publication, capture, model or shell launch.
// rustc -O is a correctness affordance, not the historical Cargo release benchmark.
import fs from 'node:fs';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
const root=process.cwd();
const out=process.env.SNIPPING_MERGE_RESULTS??'C:/Users/jnev1/AppData/Local/Temp/opencode/master-merge-validation';
const deps=process.env.NATIVE_FIXTURE_DEPS??path.join(root,'src-tauri/target/debug/deps');
fs.mkdirSync(out,{recursive:true});
const args=['--edition=2021','--test','-O','-L',`dependency=${deps}`];
for(const name of ['serde','windows','windows_core','sha2','png','flate2']){
 const library=fs.readdirSync(deps).filter(file=>file.startsWith(`lib${name}-`)&&file.endsWith('.rlib')).sort()[0];
 if(!library)throw Error(`missing existing cached ${name} rlib; do not install dependencies`);
 args.push('--extern',`${name}=${path.join(deps,library)}`);
}
const executable=path.join(out,'snipping-png-codec.exe');
args.push(path.join(root,'src-tauri/tests/snipping_png_codec.rs'),'-o',executable);
const label=process.env.SNIPPING_MERGE_LABEL??'current';
fs.writeFileSync(path.join(out,`${label}-codec-command.json`),JSON.stringify({program:'rustc',args,cwd:root},null,2));
const build=spawnSync('rustc',args,{encoding:'utf8'});
fs.writeFileSync(path.join(out,`${label}-codec-compile.txt`),(build.stdout??'')+(build.stderr??''));
if(build.error)throw build.error;
if(build.status!==0){console.error(build.stderr);process.exit(build.status??1);}
const run=spawnSync(executable,['--nocapture','--test-threads=1',...process.argv.slice(2)],{encoding:'utf8'});
fs.writeFileSync(path.join(out,`${label}-codec-tests.txt`),(run.stdout??'')+(run.stderr??''));
console.log((run.stdout??'')+(run.stderr??''));
if(run.error)throw run.error;
process.exit(run.status??1);
