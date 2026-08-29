import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const componentPath = (name) => new URL(`../src/components/${name}`, import.meta.url);
const readComponent = (name) => readFileSync(componentPath(name), 'utf8');

const audioPanelSource = readComponent('AudioPanelSurface.svelte');
const stackPopupSource = readComponent('StackPopupSurface.svelte');

function createAsyncUnlistenerRegistry() {
  const unlisteners = [];
  let disposed = false;
  const registerAsyncUnlistener = (registration) => {
    void registration.then((unlisten) => {
      if (disposed) {
        unlisten();
        return;
      }
      unlisteners.push(unlisten);
    });
  };
  const destroy = () => {
    disposed = true;
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
  return { registerAsyncUnlistener, destroy };
}

function firstOnMountBody(source) {
  return source.match(/onMount\(\(\) => \{[\s\S]*?\n  \}\);/)?.[0] ?? '';
}

function listenerBlock(source, eventName) {
  const eventIndex = source.indexOf(`listen(${eventName}`);
  assert.notEqual(eventIndex, -1, `missing listen(${eventName})`);
  const thenIndex = source.indexOf('}).then((unlisten)', eventIndex);
  const helperEndIndex = source.indexOf('}));', eventIndex);
  const endIndex = thenIndex === -1 ? helperEndIndex : thenIndex;
  assert.notEqual(endIndex, -1, `missing async unlisten registration for ${eventName}`);
  return source.slice(eventIndex, endIndex);
}

function assertDisposedGuard(source, componentName) {
  assert.match(source, /const unlisteners: Array<\(\) => void> = \[\]/, `${componentName} should collect resolved unlisteners`);
  assert.match(source, /let disposed = false/, `${componentName} should track destroyed state`);
  assert.match(source, /disposed = true/, `${componentName} should mark destroyed before cleanup`);
  assert.match(
    source,
    /if \((?:disposed|isDisposed\(\))\) \{\s*unlisten\(\);\s*(?:return;\s*)?\}(?:\s*else\s*\{)?[\s\S]*unlisteners\.push\(unlisten\)/,
    `${componentName} should immediately dispose listeners that resolve after destroy`
  );
  assert.doesNotMatch(
    source,
    /\.then\(\(unlisten\) => \{\s*unlisteners\.push\(unlisten\);?\s*\}\)/,
    `${componentName} should not push async listeners without a disposed guard`
  );
}

function assertAsyncListenerLifecycleContract(source, componentName) {
  const mountBody = firstOnMountBody(source);
  assert.match(mountBody, /const unlisteners: Array<\(\) => void> = \[\]/, `${componentName} should collect async unlisteners in onMount`);
  assert.match(mountBody, /disposed = false/, `${componentName} should reset mounted disposed state`);
  assert.match(mountBody, /function registerAsyncUnlistener\(registration: Promise<\(\) => void>\)/, `${componentName} should centralize async listener registration`);
  assert.match(mountBody, /registration\.then\(\(unlisten\) => \{[\s\S]*if \(disposed\) \{\s*unlisten\(\);\s*return;\s*\}[\s\S]*unlisteners\.push\(unlisten\)/, `${componentName} should immediately unlisten listeners that resolve after destroy`);
  assert.match(mountBody, /\.catch\(\(error\) => \{[\s\S]*if \(!disposed\) console\.error/, `${componentName} should handle async listener registration rejection while mounted`);
  assert.match(mountBody, /return \(\) => \{\s*disposed = true;[\s\S]*while \(unlisteners\.length\) \{[\s\S]*unlisteners\.pop\(\)\?\.\(\);[\s\S]*\}/, `${componentName} cleanup should mark disposed first and drain unlisteners exactly once`);
  assert.match(mountBody, /while \(unlisteners\.length\) \{[\s\S]*try \{[\s\S]*unlisteners\.pop\(\)\?\.\(\);[\s\S]*\} catch \(error\)/, `${componentName} cleanup should continue draining if one unlistener throws`);
}

function assertCallbackDisposedGuard(source, componentName, callbackName) {
  const body = source.match(new RegExp(`(?:async )?function ${callbackName}\\([^)]*\\)(?::[^\\{]+)? \\{[\\s\\S]*?\\n  \\}`))?.[0] ?? '';
  assert.match(body, /if \(disposed\) return;/, `${componentName} ${callbackName} should return before state mutation after destroy`);
}

test('audio panel stays idle on hidden persistent mount', () => {
  const mountBodyBeforeOpen = audioPanelSource.match(/onMount\(\(\) => \{[\s\S]*?void listen\(AUDIO_PANEL_OPEN_EVENT/)?.[0] ?? '';

  assert.match(audioPanelSource, /let audioPanelVisible = false/);
  assert.doesNotMatch(mountBodyBeforeOpen, /audioPanelVisible = true/);
  assert.doesNotMatch(mountBodyBeforeOpen, /startAudioRefreshPolling\(\)/);
  assert.doesNotMatch(mountBodyBeforeOpen, /void refreshAudioState\(\)/);
});

test('audio panel own close event stops visible polling state', () => {
  const closeBlock = listenerBlock(audioPanelSource, 'AUDIO_PANEL_CLOSED_EVENT');

  assert.match(closeBlock, /audioPanelVisible = false/);
  assert.match(closeBlock, /stopPendingAudioRefresh\(\)/);
  assert.match(closeBlock, /stopAudioRefreshPolling\(\)/);
});

test('audio refresh events do not schedule hidden-panel work', () => {
  const scheduleBody = audioPanelSource.match(/function scheduleAudioRefresh\(reason: AudioRefreshReason\) \{[\s\S]*?\n  \}/)?.[0] ?? '';
  const closeBody = audioPanelSource.match(/function closeAudioPanel\(\) \{[\s\S]*?\n  \}/)?.[0] ?? '';

  assert.match(scheduleBody, /if \(!audioPanelVisible\) \{\s*return;\s*\}/);
  assert.match(closeBody, /audioPanelVisible = false;[\s\S]*stopPendingAudioRefresh\(\)/);
  assert.match(audioPanelSource, /function stopPendingAudioRefresh\(\)/);
});

test('process manager close invalidates in-flight refreshes', () => {
  const processManagerSource = readComponent('ProcessManagerSurface.svelte');
  const closeBody = processManagerSource.match(/function closeSurface\(\) \{[\s\S]*?\n  \}/)?.[0] ?? '';

  assert.match(closeBody, /inFlightRequest \+= 1/);
  assert.match(closeBody, /isLoading = false/);
  assert.match(processManagerSource, /if \(requestId !== inFlightRequest\) \{\s*return;\s*\}/);
});

test('async unlistener registry calls late-resolving unlisten exactly once after destroy', async () => {
  let resolveRegistration;
  let unlistenCalls = 0;
  const registration = new Promise((resolve) => {
    resolveRegistration = resolve;
  });
  const registry = createAsyncUnlistenerRegistry();

  registry.registerAsyncUnlistener(registration);
  registry.destroy();
  resolveRegistration(() => {
    unlistenCalls += 1;
  });
  await registration;
  await new Promise((resolve) => setImmediate(resolve));
  registry.destroy();

  assert.equal(unlistenCalls, 1);
});

test('audio panel open refreshes immediately before starting polling', () => {
  const openBlock = listenerBlock(audioPanelSource, 'AUDIO_PANEL_OPEN_EVENT');

  assert.match(
    openBlock,
    /audioPanelVisible = true;[\s\S]*void refreshAudioState\(\);[\s\S]*startAudioRefreshPolling\(\)/,
    'audio open should refresh immediately before arming the fallback poll interval'
  );
});

test('stack popup documents the disposed guard reference pattern', () => {
  assertDisposedGuard(stackPopupSource, 'StackPopupSurface.svelte');
});

for (const componentName of [
  'AudioPanelSurface.svelte',
  'TopBar.svelte',
  'BottomBar.svelte',
  'TaskPreviewSurface.svelte',
  'SearchPanelSurface.svelte',
  'ProcessManagerSurface.svelte'
]) {
  test(`${componentName} guards async Tauri listener cleanup after destroy`, () => {
    assertDisposedGuard(readComponent(componentName), componentName);
  });
}

for (const componentName of [
  'CommandPanelSurface.svelte',
  'TrayPanelSurface.svelte'
]) {
  test(`${componentName} uses guarded async listener lifecycle for persistent surface mount`, () => {
    assertAsyncListenerLifecycleContract(readComponent(componentName), componentName);
  });
}

test('command panel async callbacks return when disposed before mutating state', () => {
  const source = readComponent('CommandPanelSurface.svelte');
  assertCallbackDisposedGuard(source, 'CommandPanelSurface.svelte', 'refreshEntries');
  assertCallbackDisposedGuard(source, 'CommandPanelSurface.svelte', 'refreshHistory');
  assertCallbackDisposedGuard(source, 'CommandPanelSurface.svelte', 'handleQuickCommandRunUpdated');
  assertCallbackDisposedGuard(source, 'CommandPanelSurface.svelte', 'saveEntry');
  assertCallbackDisposedGuard(source, 'CommandPanelSurface.svelte', 'confirmDeleteEntry');
  assertCallbackDisposedGuard(source, 'CommandPanelSurface.svelte', 'runEntry');
  assertCallbackDisposedGuard(source, 'CommandPanelSurface.svelte', 'stopEntry');
  assertCallbackDisposedGuard(source, 'CommandPanelSurface.svelte', 'submitPendingInput');
});

test('tray panel async callbacks return when disposed before mutating state', () => {
  const source = readComponent('TrayPanelSurface.svelte');
  assertCallbackDisposedGuard(source, 'TrayPanelSurface.svelte', 'loadTrayIcons');
  assertCallbackDisposedGuard(source, 'TrayPanelSurface.svelte', 'triggerTrayIcon');
});
