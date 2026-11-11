import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, existsSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const source = fileURLToPath(new URL('./rustc-build-script-wrapper.rs', import.meta.url));
const directory = fileURLToPath(new URL('../.local/', import.meta.url));
const output = fileURLToPath(new URL('../.local/rustc-build-script-wrapper.exe', import.meta.url));
const stamp = `${output}.source`;
const contents = readFileSync(source, 'utf8');
if (!existsSync(output) || !existsSync(stamp) || readFileSync(stamp, 'utf8') !== contents) {
  mkdirSync(directory, { recursive: true });
  // Invoke rustc directly: Cargo's configured wrappers must not bootstrap themselves.
  const result = spawnSync('rustc', ['--edition=2021', source, '-o', output], {
    stdio: 'inherit', windowsHide: true,
  });
  if (result.error) console.error(result.error.message);
  if (result.status !== 0) process.exit(result.status ?? 1);
  writeFileSync(stamp, contents);
}
