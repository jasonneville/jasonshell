import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

const runnerUrl = new URL('../scripts/run-validation.mjs', import.meta.url);
const stageNames = ['check', 'build', 'test:component', 'test:node', 'cargo:test', 'cargo:check'];

test('validate delegates to the all-stage runner with exactly the six existing gates', async () => {
  const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
  assert.equal(pkg.scripts.validate, 'node scripts/run-validation.mjs');
  const { VALIDATION_STAGES } = await import(runnerUrl.href);
  assert.deepEqual(VALIDATION_STAGES.map((stage) => stage.name), stageNames);
  for (const stage of VALIDATION_STAGES) {
    assert.ok(stage.command);
    assert.equal(stage.args.at(-1), process.platform === 'win32' ? `npm run ${stage.name}` : stage.name);
    assert.ok(stage.args.includes(process.platform === 'win32' ? '/c' : 'run'));
  }
});

test('runner continues after nonzero, signal, and spawn error and summarizes every stage', async () => {
  const { runValidation, VALIDATION_STAGES } = await import(runnerUrl.href);
  const calls = [];
  const output = [];
  const spawn = (command, args, options) => {
    const index = calls.length;
    calls.push({ command, args, options });
    const child = new EventEmitter();
    queueMicrotask(() => {
      if (index === 2) {
        child.emit('error', new Error('fixture spawn failed'));
        child.emit('close', -2, null);
      } else {
        child.emit('close', index === 0 ? 7 : index === 1 ? null : 0, index === 1 ? 'SIGTERM' : null);
      }
    });
    return child;
  };
  const result = await runValidation({ spawn, log: (line) => output.push(String(line)) });
  assert.equal(calls.length, 6);
  assert.equal(result.exitCode, 1);
  assert.deepEqual(result.results.map((stage) => stage.name), stageNames);
  assert.equal(result.results.length, VALIDATION_STAGES.length);
  assert.deepEqual(calls, VALIDATION_STAGES.map((stage) => ({
    command: stage.command, args: stage.args, options: { stdio: 'inherit', shell: false }
  })));
  const summaryStart = output.findIndex((line) => line.includes('Validation summary'));
  assert.ok(summaryStart >= 0, 'final summary must exist');
  assert.deepEqual(output.slice(summaryStart + 1), stageNames.map((name, index) => `${name}: ${index < 3 ? 'FAIL' : 'PASS'}`));
  assert.match(output.join('\n'), /fixture spawn failed/);
  assert.match(output.join('\n'), /SIGTERM/);
  assert.equal(result.results.filter((stage) => stage.exitCode === 0).length, 3);
});

test('runner succeeds only when every stage succeeds and survives synchronous spawn throws', async () => {
  const { runValidation } = await import(runnerUrl.href);
  for (const throws of [false, true]) {
    let calls = 0;
    const result = await runValidation({
      spawn() {
        const index = calls++;
        if (throws && index === 0) throw new Error('synchronous fixture failure');
        const child = new EventEmitter();
        queueMicrotask(() => child.emit('close', 0, null));
        return child;
      },
      log() {}
    });
    assert.equal(calls, 6);
    assert.equal(result.exitCode, throws ? 1 : 0);
  }
});

for (const firstExit of [0, 9]) test(`subprocess acceptance: child exit ${firstExit} preserves later execution and truthful aggregate exit`, () => {
  const code = `
    import { runValidation } from ${JSON.stringify(runnerUrl.href)};
    const stages = [
      { name: 'first', command: process.execPath, args: ['-e', 'console.log("FIRST_EXECUTED"); process.exit(${firstExit})'] },
      { name: 'later', command: process.execPath, args: ['-e', 'console.log("LATER_EXECUTED")'] }
    ];
    const result = await runValidation({ stages });
    process.exitCode = result.exitCode;
  `;
  const child = spawnSync(process.execPath, ['--input-type=module', '-e', code], { encoding: 'utf8', timeout: 10000 });
  assert.equal(child.error, undefined);
  assert.equal(child.status, firstExit === 0 ? 0 : 1);
  assert.match(child.stdout, /FIRST_EXECUTED/);
  assert.match(child.stdout, /LATER_EXECUTED/);
  assert.match(child.stdout, /first/);
  assert.match(child.stdout, /later/);
  assert.ok(child.stdout.indexOf('FIRST_EXECUTED') < child.stdout.indexOf('LATER_EXECUTED'));
  assert.match(child.stdout, new RegExp(`Validation summary[\\s\\S]*first: ${firstExit === 0 ? 'PASS' : 'FAIL'}[\\s\\S]*later: PASS`));
});
