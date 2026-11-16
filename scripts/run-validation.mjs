import { spawn as nodeSpawn } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

// cmd files need a command interpreter on Windows; explicit Node stages do not.
const npmCommand = process.platform === 'win32' ? (process.env.ComSpec || 'cmd.exe') : 'npm';
export const VALIDATION_STAGES = ['check', 'build', 'test:component', 'test:node', 'cargo:test', 'cargo:check'].map((name) => ({
  name,
  command: npmCommand,
  args: process.platform === 'win32' ? ['/d', '/s', '/c', `npm run ${name}`] : ['run', name]
}));

export async function runValidation({ spawn = nodeSpawn, stages = VALIDATION_STAGES, log = console.log } = {}) {
  const results = [];
  for (const stage of stages) {
    log(`\n=== ${stage.name} ===`);
    const result = await new Promise((resolveResult) => {
      const fail = (error) => resolveResult({ name: stage.name, exitCode: 1, error: String(error) });
      try {
        const child = spawn(stage.command, stage.args, { stdio: 'inherit', shell: false });
        child.once('error', fail);
        child.once('close', (code, signal) => resolveResult({
          name: stage.name, exitCode: signal || code !== 0 ? 1 : 0,
          ...(signal ? { signal } : {}), ...(code !== null ? { code } : {})
        }));
      } catch (error) { fail(error); }
    });
    results.push(result);
    log(`${result.name}: ${result.exitCode === 0 ? 'PASS' : 'FAIL'}${result.error ? ` (${result.error})` : result.signal ? ` (signal ${result.signal})` : ` (exit ${result.code})`}`);
  }
  log('\nValidation summary');
  for (const result of results) log(`${result.name}: ${result.exitCode === 0 ? 'PASS' : 'FAIL'}`);
  return { results, exitCode: results.some((result) => result.exitCode !== 0) ? 1 : 0 };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = (await runValidation()).exitCode;
}
