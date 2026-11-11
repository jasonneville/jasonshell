import { readFileSync } from 'node:fs';
import ts from 'typescript';
import vm from 'node:vm';

export function readSource(path) {
  return readFileSync(new URL(`../../${path}`, import.meta.url), 'utf8');
}

// Execute the real declaration, not a second implementation of deduplication.
export function surfaceFunction(path, name, bindings) {
  const source = path.endsWith('.svelte') ? readSource(path).split('<script lang="ts">')[1].split('</script>')[0] : readSource(path);
  const tree = ts.createSourceFile(path + '.ts', source, ts.ScriptTarget.Latest, true);
  const declaration = tree.statements.find((node) => ts.isFunctionDeclaration(node) && node.name?.text === name);
  if (!declaration) throw new Error(`Missing production function ${name}`);
  const javascript = ts.transpileModule(declaration.getText(tree).replace(/^export\s+/, ''), { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText;
  const context = vm.createContext(bindings);
  vm.runInContext(javascript, context);
  return { invoke: (...args) => context[name](...args), context };
}

export function trackerClass(bindings = {}) {
  const source = readSource('src/lib/terminalSequenceTracker.ts');
  const javascript = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText;
  const context = vm.createContext({ exports: {}, ...bindings });
  vm.runInContext(javascript, context);
  return context.exports.TerminalSequenceTracker;
}

export async function sequenceTracker() {
  return new (trackerClass())();
}

export function setOracle() {
  const keys = new Set();
  return (sessionId, stream, sequence) => {
    if (typeof sequence !== 'number') return true;
    const key = `${sessionId}:${stream ?? 'stdout'}:${sequence}`;
    if (keys.has(key)) return false;
    keys.add(key);
    return true;
  };
}
