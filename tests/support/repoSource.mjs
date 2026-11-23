import { readFileSync } from 'node:fs';

/** Reads a repo-relative text file; a missing/unreadable file throws instead of yielding ''. */
export function readRepoFile(relativeFromRepoRoot) {
  const url = new URL(`../../${relativeFromRepoRoot}`, import.meta.url);
  try {
    return readFileSync(url, 'utf8');
  } catch (error) {
    throw new Error(`Required source file is missing or unreadable: ${relativeFromRepoRoot} (${error.code ?? error.message})`, { cause: error });
  }
}
