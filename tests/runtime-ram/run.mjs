import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const cli = fileURLToPath(new URL('../../node_modules/vitest/vitest.mjs', import.meta.url)).replace(/^[a-z]:/, (drive) => drive.toUpperCase());
const result = spawnSync(process.execPath, [cli, 'run', '--config', 'tests/runtime-ram/vitest.config.ts', ...process.argv.slice(2)], { stdio: 'inherit' });
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
