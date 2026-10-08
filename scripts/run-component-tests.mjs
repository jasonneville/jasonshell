import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

// Vite canonicalizes Windows drive letters to uppercase. Node's ESM cache is
// case-sensitive: a lowercase CLI path otherwise creates a second Vitest runner.
const cliPath = fileURLToPath(new URL('../node_modules/vitest/vitest.mjs', import.meta.url));
const canonicalCliPath = process.platform === 'win32'
  ? cliPath.replace(/^[a-z]:/, (drive) => drive.toUpperCase())
  : cliPath;
const result = spawnSync(process.execPath, [canonicalCliPath, 'run', '--config', 'vitest.config.ts', ...process.argv.slice(2)], {
  stdio: 'inherit'
});
if (result.error) throw result.error;
if (result.signal) throw new Error(`Component test runner terminated by ${result.signal}`);
process.exitCode = result.status ?? 1;
