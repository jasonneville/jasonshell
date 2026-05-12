import { rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const distTests = path.resolve(repoRoot, 'dist-tests');
const relative = path.relative(repoRoot, distTests);

if (relative !== 'dist-tests' || path.isAbsolute(relative) || relative.startsWith('..')) {
  throw new Error(`Refusing to clean unexpected dist-tests path: ${distTests}`);
}

rmSync(distTests, { recursive: true, force: true });
