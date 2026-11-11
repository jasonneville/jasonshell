import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdtempSync, mkdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const repository = fileURLToPath(new URL('..', import.meta.url));

function configuredEntrypoint() {
  const config = readFileSync(join(repository, '.cargo/config.toml'), 'utf8');
  const wrapper = config.match(/^rustc-wrapper\s*=\s*"([^"]+)"/m)?.[1];
  assert.ok(wrapper, 'Cargo must configure a compiler wrapper');
  return isAbsolute(wrapper) ? wrapper : resolve(repository, wrapper);
}

function runEntrypoint(entrypoint, args, options) {
  if (/\.cmd$/i.test(entrypoint)) {
    // Mirror the old Cargo batch boundary, rather than bypassing it with Node.
    const line = `""${entrypoint}" ${args.map((arg) => `"${arg.replaceAll('"', '\\"')}"`).join(' ')}"`;
    return spawnSync(process.env.ComSpec ?? 'cmd.exe', ['/d', '/s', '/c', line], {
      ...options, windowsVerbatimArguments: true,
    });
  }
  return spawnSync(entrypoint, args, options);
}

test('Windows compiler wrapper routes build scripts without disabling Blaze for application code', {
  skip: process.platform !== 'win32' && 'Windows native executable launch contract',
}, async (t) => {
  const prepared = spawnSync(process.execPath, [join(repository, 'scripts/prepare-rustc-wrapper.mjs')], {
    cwd: repository, encoding: 'utf8', timeout: 60_000,
  });
  assert.equal(prepared.status, 0, prepared.stderr || prepared.error?.message);
  const entrypoint = configuredEntrypoint();
  const temporary = join(process.env.TEMP ?? tmpdir(), 'opencode');
  mkdirSync(temporary, { recursive: true });
  const root = mkdtempSync(join(temporary, 'rustc wrapper fixture '));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const compiler = join(root, 'real compiler.exe');
  const blaze = join(root, 'ra-jit-rustc.exe');
  const mixedCaseBlaze = join(root, 'RA-JIT-RUSTC.EXE');
  const unrelated = join(root, 'not-ra-jit-rustc.exe');
  copyFileSync(process.execPath, compiler);
  copyFileSync(process.execPath, blaze);
  copyFileSync(process.execPath, unrelated);
  const preload = join(root, 'record invocation.cjs');
  const input = join(root, 'input with spaces.js');
  writeFileSync(input, 'throw new Error("fixture entrypoint must never execute");');
  // Native Node EXEs stand in for rustc/workspace wrappers. The preload records
  // dispatch and exits before any compiler, wrapper, build script, or app runs.
  writeFileSync(preload, `
    if (${JSON.stringify([compiler, blaze, unrelated].map((path) => path.toLowerCase()))}.includes(process.execPath.toLowerCase())) {
      console.log(JSON.stringify({ executable: process.execPath, args: process.argv.slice(1) }));
      console.error('fixture stderr');
      process.exit(Number(process.env.FIXTURE_EXIT ?? 0));
    }
  `);
  const env = { ...process.env, NODE_OPTIONS: `--require ${JSON.stringify(preload)}` };
  const run = (args, exit = 0) => runEntrypoint(entrypoint, args, {
    cwd: root, encoding: 'utf8', env: { ...env, FIXTURE_EXIT: String(exit) }, timeout: 30_000,
  });
  const expectRoute = (args, executable, forwarded, exit = 0) => {
    const result = run(args, exit);
    assert.equal(result.status, exit, result.stderr);
    assert.deepEqual(JSON.parse(result.stdout), { executable, args: forwarded });
    assert.equal(result.stderr.trim(), 'fixture stderr');
  };
  await t.test('bootstrap reuses its current native executable without needing rustc again', () => {
    const before = statSync(entrypoint, { bigint: true }).mtimeNs;
    const bytes = readFileSync(entrypoint);
    const source = readFileSync(join(repository, 'scripts/rustc-build-script-wrapper.rs'), 'utf8');
    assert.equal(readFileSync(`${entrypoint}.source`, 'utf8'), source);
    // An empty tool search directory makes an unnecessary rustc launch fail,
    // while leaving the running build and the real environment untouched.
    const withoutPath = Object.fromEntries(Object.entries(process.env)
      .filter(([key]) => key.toLowerCase() !== 'path'));
    const reused = spawnSync(process.execPath, [join(repository, 'scripts/prepare-rustc-wrapper.mjs')], {
      cwd: root, encoding: 'utf8', timeout: 30_000, env: { ...withoutPath, PATH: root },
    });
    assert.equal(reused.status, 0, reused.stderr || reused.error?.message);
    assert.equal(statSync(entrypoint, { bigint: true }).mtimeNs, before);
    assert.deepEqual(readFileSync(entrypoint), bytes);
    assert.equal(readFileSync(`${entrypoint}.source`, 'utf8'), source);
  });
  for (const [name, wrapper] of [['Blaze', blaze], ['case-insensitive Blaze filename', mixedCaseBlaze]]) {
    await t.test(`bypasses ${name} only for build_script_build`, () => {
      const args = [input, '--crate-name', 'build_script_build', '--cfg', 'feature="quoted value"'];
      expectRoute([wrapper, compiler, ...args], compiler, args);
    });
  }
  await t.test('preserves Blaze for application, library, and compiler metadata queries', () => {
    for (const tail of [
      ['--crate-name', 'jason_shell'], ['--crate-name', 'some_library', '--crate-type', 'lib'],
      ['--crate-name', 'build_script_build_extra'], ['-vV'], ['--print', 'cfg'], ['--crate-name'],
    ]) {
      const args = [compiler, input, ...tail];
      expectRoute([blaze, ...args], blaze, args);
    }
  });
  await t.test('does not bypass unrelated workspace wrappers for build scripts', () => {
    const args = [compiler, input, '--crate-name', 'build_script_build'];
    expectRoute([unrelated, ...args], unrelated, args);
  });
  await t.test('works without any workspace wrapper', () => {
    const args = [input, '--crate-name', 'build_script_build'];
    expectRoute([compiler, ...args], compiler, args);
  });
  await t.test('forwards nonzero exit status, stdout, and stderr on both routes', () => {
    for (const crate of ['build_script_build', 'jason_shell']) {
      const args = [input, '--crate-name', crate];
      expectRoute([blaze, compiler, ...args], crate === 'build_script_build' ? compiler : blaze,
        crate === 'build_script_build' ? args : [compiler, ...args], 37);
    }
  });
  await t.test('reports compiler spawn failure with nonzero status', () => {
    const missing = join(root, 'missing compiler.exe');
    for (const args of [
      [missing, input],
      [blaze, missing, input, '--crate-name', 'build_script_build'],
      [missing, compiler, input, '--crate-name', 'jason_shell'],
    ]) {
      const result = run(args);
      assert.equal(result.status, 1);
      assert.equal(result.stdout, '');
      assert.match(result.stderr, /failed to launch compiler:/);
    }
  });
  await t.test('reports a missing compiler command with nonzero status', () => {
    const result = run([]);
    assert.equal(result.status, 1);
    assert.equal(result.stdout, '');
    assert.match(result.stderr, /missing compiler command/);
  });
  await t.test('configured Cargo entrypoint preserves over 12k characters and quoting on both routes', () => {
    const payload = Array.from({ length: 160 }, (_, index) =>
      `feature_${index}="${'x'.repeat(64)} value"`);
    const special = ['', 'space value', 'embedded"quote', 'trailing slash\\', 'space and slash\\',
      'backslash\\"quote', 'Unicode-λ-雪', '%PATH%', '&|<>^!'];
    assert.ok([...payload, ...special].join(' ').length > 12_000);
    assert.ok([...payload, ...special].join(' ').length < 20_000,
      'fixture must stay below the native Windows command-line limit after quoting');
    for (const crate of ['build_script_build', 'jason_shell']) {
      const args = [input, '--crate-name', crate, ...payload, ...special];
      const result = runEntrypoint(entrypoint, [blaze, compiler, ...args], {
        cwd: root, encoding: 'utf8', env: { ...env, FIXTURE_EXIT: '31' }, timeout: 30_000,
      });
      assert.ifError(result.error);
      assert.equal(result.status, 31, result.stderr);
      assert.deepEqual(JSON.parse(result.stdout), {
        executable: crate === 'build_script_build' ? compiler : blaze,
        args: crate === 'build_script_build' ? args : [compiler, ...args],
      });
      assert.equal(result.stderr.trim(), 'fixture stderr');
    }
  });
  await t.test('entrypoint launches from a different directory with spaces and forwards exit status', () => {
    const copied = join(root, 'wrapper directory');
    mkdirSync(copied);
    const copiedEntrypoint = join(copied, basename(entrypoint));
    copyFileSync(entrypoint, copiedEntrypoint);
    for (const crate of ['build_script_build', 'jason_shell']) {
      const result = runEntrypoint(copiedEntrypoint, [blaze, compiler, input, '--crate-name', crate], {
        cwd: root, encoding: 'utf8', timeout: 30_000,
        env: { ...env, FIXTURE_EXIT: '29' },
      });
      assert.equal(result.status, 29, result.stderr);
      const record = JSON.parse(result.stdout);
      assert.equal(record.executable, crate === 'build_script_build' ? compiler : blaze);
      assert.deepEqual(record.args, crate === 'build_script_build'
        ? [input, '--crate-name', crate] : [compiler, input, '--crate-name', crate]);
    }
  });
});
