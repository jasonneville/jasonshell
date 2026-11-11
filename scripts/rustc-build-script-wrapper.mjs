import { spawnSync } from 'node:child_process';
import { basename } from 'node:path';

// Cargo composes rustc-wrapper -> rustc-workspace-wrapper -> rustc. Blaze's
// Windows build-script stub currently prints its CLI help when Cargo executes
// build-script-build.exe. Compile only build scripts directly; preserve the
// configured workspace wrapper for every application/library compilation.
const args = process.argv.slice(2);
const crateIndex = args.indexOf('--crate-name');
const buildScript = crateIndex >= 0 && args[crateIndex + 1] === 'build_script_build';
const blazeWrapper = /^ra-jit-rustc(?:\.exe)?$/i.test(basename(args[0] ?? ''));
if (buildScript && blazeWrapper) args.shift();
const command = args.shift();
const result = spawnSync(command, args, { stdio: 'inherit', windowsHide: true });
if (result.error) console.error(result.error.message);
process.exit(result.status ?? 1);
