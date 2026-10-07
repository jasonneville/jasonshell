import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { before, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import picomatch from 'picomatch';
import { createServer, resolveConfig } from 'vite';
import WebSocket from 'ws';

const projectRoot = fileURLToPath(new URL('../', import.meta.url));
const configFile = fileURLToPath(new URL('../vite.config.ts', import.meta.url));
const remoteHost = '192.0.2.1';
let localConfig;
let remoteConfig;

before(async () => {
  const previousHost = process.env.TAURI_DEV_HOST;
  try {
    delete process.env.TAURI_DEV_HOST;
    localConfig = await resolveConfig({ root: projectRoot, configFile }, 'serve');
    process.env.TAURI_DEV_HOST = remoteHost;
    remoteConfig = await resolveConfig({ root: projectRoot, configFile }, 'serve');
  } finally {
    if (previousHost === undefined) delete process.env.TAURI_DEV_HOST;
    else process.env.TAURI_DEV_HOST = previousHost;
  }
});

function ignores(config, filename) {
  const ignored = config.server.watch?.ignored ?? [];
  const patterns = Array.isArray(ignored) ? ignored : [ignored];
  assert.ok(patterns.every((pattern) => typeof pattern === 'string'), 'watch exclusions are glob strings');
  // Chokidar normalizes Windows separators before applying its glob matcher.
  return picomatch(patterns, { dot: true })(filename.replaceAll('\\', '/'));
}

test('resolved Vite watcher excludes the Rust target directory and all build descendants', () => {
  const targetPaths = [
    'src-tauri/target',
    'src-tauri/target/',
    'src-tauri/target/debug/jason-shell.exe',
    'src-tauri/target/release/build/generated/out/index.html',
    'src-tauri/target/.fingerprint/dependency/build-script.json'
  ];
  for (const config of [localConfig, remoteConfig]) {
    for (const relativePath of targetPaths) {
      for (const filename of [relativePath, `${config.root}/${relativePath}`, `${config.root}/${relativePath}`.replaceAll('/', '\\')]) {
        assert.equal(ignores(config, filename), true, `Rust build output must be ignored: ${filename}`);
      }
    }
  }
});

test('resolved Vite watcher keeps frontend and adjacent native source watchable', () => {
  const sourcePaths = [
    'src/main.ts',
    'src/App.svelte',
    'src/components/TopBar.svelte',
    'src-tauri/src/main.rs',
    'src-tauri/tauri.conf.json',
    'src-tauri/targeting/source.rs',
    'src-tauri/target-backup/source.rs',
    'src/target/source.ts'
  ];
  for (const config of [localConfig, remoteConfig]) {
    assert.notEqual(config.server.watch, null, 'source watching must not be disabled globally');
    for (const relativePath of sourcePaths) {
      for (const filename of [relativePath, `${config.root}/${relativePath}`, `${config.root}/${relativePath}`.replaceAll('/', '\\')]) {
        assert.equal(ignores(config, filename), false, `Source must remain watchable: ${filename}`);
      }
    }
  }
});

test('resolved local Vite config preserves the existing dev port and HMR', () => {
  assert.equal(localConfig.server.host, false);
  assert.equal(localConfig.server.port, 1420);
  assert.equal(localConfig.server.strictPort, true);
  assert.equal(localConfig.server.hmr.port, 1421);
  assert.ok(localConfig.plugins.some((plugin) => plugin.name === 'vite-plugin-terminal'));
});

test('resolved remote-host Vite config preserves Tauri host and websocket HMR', () => {
  assert.equal(remoteConfig.server.host, remoteHost);
  assert.equal(remoteConfig.server.port, 1420);
  assert.equal(remoteConfig.server.strictPort, true);
  assert.equal(remoteConfig.server.hmr.host, remoteHost);
  assert.equal(remoteConfig.server.hmr.port, 1421);
  assert.equal(remoteConfig.server.hmr.protocol, 'ws');
});

function waitForEvent(emitter, event, predicate = () => true) {
  return new Promise((resolve, reject) => {
    const cleanup = () => {
      clearTimeout(timer);
      emitter.off(event, listener);
    };
    const listener = (...args) => {
      if (!predicate(...args)) return;
      cleanup();
      resolve(args);
    };
    const timer = setTimeout(() => {
      cleanup();
      reject(new Error(`Timed out waiting for ${event}`));
    }, 10_000);
    emitter.on(event, listener);
  });
}

async function waitForCondition(predicate, description) {
  const deadline = Date.now() + 10_000;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error(`Timed out waiting for ${description}`);
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
}

test('actual Vite watcher excludes Rust outputs while delivering source changes and websocket HMR', { timeout: 25_000 }, async (t) => {
  const temporaryRoot = process.platform === 'win32' && process.env.LOCALAPPDATA
    ? path.join(process.env.LOCALAPPDATA, 'Temp', 'opencode')
    : tmpdir();
  await mkdir(temporaryRoot, { recursive: true });
  const fixture = await mkdtemp(path.join(temporaryRoot, 'vite-watcher-contract-'));
  let server;
  let socket;
  t.after(async () => {
    socket?.terminate();
    try {
      await server?.close();
    } finally {
      await rm(fixture, { recursive: true, force: true });
    }
  });

  const frontendFile = path.join(fixture, 'src', 'main.ts');
  const nativeFile = path.join(fixture, 'src-tauri', 'src', 'main.rs');
  const outputFile = path.join(fixture, 'src-tauri', 'target', 'debug', 'build-output.txt');
  for (const filename of [frontendFile, nativeFile, outputFile]) {
    await mkdir(path.dirname(filename), { recursive: true });
  }
  const frontendSource = (value) => `export const value = ${value};\nif (import.meta.hot) import.meta.hot.accept();\n`;
  await writeFile(frontendFile, frontendSource(1));
  await writeFile(nativeFile, '// fixture native source\n');
  await writeFile(outputFile, 'fixture Rust output\n');
  await writeFile(path.join(fixture, 'index.html'), '<script type="module" src="/src/main.ts"></script>');

  let watcherReady;
  server = await createServer({
    configFile: false,
    root: fixture,
    cacheDir: path.join(fixture, '.vite'),
    logLevel: 'silent',
    plugins: [{
      name: 'test-watch-readiness',
      configureServer(instance) {
        watcherReady = waitForEvent(instance.watcher, 'ready');
      }
    }],
    server: {
      host: '127.0.0.1',
      port: 0,
      strictPort: true,
      watch: localConfig.server.watch,
      hmr: { host: '127.0.0.1' }
    }
  });
  await server.listen();
  await watcherReady;
  // Missing optional env files can produce an early ready event during root traversal.
  const watchesFile = (filename) => Object.entries(server.watcher.getWatched()).some(([directory, entries]) =>
    path.resolve(directory) === path.dirname(filename) && entries.includes(path.basename(filename))
  );
  await waitForCondition(() => watchesFile(frontendFile) && watchesFile(nativeFile), 'source watcher registration');

  const watched = server.watcher.getWatched();
  const targetDirectory = path.join(fixture, 'src-tauri', 'target').replaceAll('\\', '/');
  const targetDirectories = Object.keys(watched).filter((directory) => {
    const normalized = directory.replaceAll('\\', '/');
    return normalized === targetDirectory || normalized.startsWith(`${targetDirectory}/`);
  });
  assert.deepEqual(targetDirectories, []);
  assert.ok(watchesFile(frontendFile));
  assert.ok(watchesFile(nativeFile));

  const address = server.httpServer.address();
  assert.equal(address.address, '127.0.0.1');
  const response = await fetch(`http://127.0.0.1:${address.port}/src/main.ts`);
  assert.equal(response.status, 200);
  assert.match(await response.text(), /value = 1/);

  socket = new WebSocket(`ws://127.0.0.1:${address.port}`, 'vite-hmr');
  await waitForEvent(socket, 'message', (data) => JSON.parse(data.toString()).type === 'connected');
  const sourceChanged = waitForEvent(server.watcher, 'change', (filename) => path.resolve(filename) === frontendFile);
  const updated = waitForEvent(socket, 'message', (data) => {
    const payload = JSON.parse(data.toString());
    return payload.type === 'update' && payload.updates.some((update) => update.path === '/src/main.ts');
  });
  await writeFile(frontendFile, frontendSource(2));
  await Promise.all([sourceChanged, updated]);

  const nativeChanged = waitForEvent(server.watcher, 'change', (filename) => path.resolve(filename) === nativeFile);
  await writeFile(nativeFile, '// fixture native source changed\n');
  await nativeChanged;
  assert.match(await readFile(frontendFile, 'utf8'), /value = 2/);
  const updatedResponse = await fetch(`http://127.0.0.1:${address.port}/src/main.ts`);
  assert.equal(updatedResponse.status, 200);
  assert.match(await updatedResponse.text(), /value = 2/);
  t.diagnostic('target watched directories=0; frontend/native change events received; websocket HMR update received');
});
