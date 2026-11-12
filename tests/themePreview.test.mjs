import assert from 'node:assert/strict';
import test from 'node:test';
import { createCustomTheme, setThemePaletteColor } from '../dist-tests/lib/themePalette.js';
import { applyShellTheme, getSavedThemeState } from '../dist-tests/lib/themes.js';
import {
  THEME_PREVIEW_CHANNEL, createThemeEditor, currentPreviewTheme,
  getThemePreviewStatus, installThemePreviewSync, queryThemePreviewPresence,
  requestPreviewSelection, subscribeThemePreview, validateThemePreviewMessage
} from '../dist-tests/lib/themePreview.js';

let clock = Date.now();
function environment(t) {
  clock += 100_000;
  t.mock.timers.enable({ apis: ['Date', 'setTimeout', 'setInterval'], now: clock });
  const old = Object.fromEntries(['window', 'document', 'BroadcastChannel'].map(key => [key, Object.getOwnPropertyDescriptor(globalThis, key)]));
  const values = new Map([['jasonshell.theme', 'nord']]);
  const writes = [];
  const storage = { getItem: key => values.get(key) ?? null, setItem: (key, value) => { writes.push([key, value]); values.set(key, value); } };
  const properties = new Map();
  const element = { dataset: {}, style: { colorScheme: '', setProperty: (key, value) => properties.set(key, value), removeProperty: key => properties.delete(key) } };
  const document = Object.assign(new EventTarget(), { documentElement: element, visibilityState: 'visible' });
  const window = Object.assign(new EventTarget(), { localStorage: storage });
  const channels = [];
  class Channel {
    closed = false;
    onmessage = null;
    sent = [];
    responder = null;
    constructor(name) { this.name = name; channels.push(this); }
    postMessage(value) { this.sent.push(structuredClone(value)); this.responder?.(value); }
    close() { this.closed = true; }
  }
  for (const [key, value] of Object.entries({ window, document, BroadcastChannel: Channel })) Object.defineProperty(globalThis, key, { configurable: true, value });
  const cleanup = installThemePreviewSync();
  t.after(() => {
    cleanup();
    for (const [key, descriptor] of Object.entries(old)) {
      if (descriptor) Object.defineProperty(globalThis, key, descriptor); else delete globalThis[key];
    }
    t.mock.timers.reset();
  });
  const channel = channels.find(item => item.name === THEME_PREVIEW_CHANNEL);
  assert.ok(channel);
  const theme = createCustomTheme('dracula', 'Remote', 'custom:remote');
  const packet = (changes = {}) => ({ version: 1, type: 'preview', sender: 'remote-owner', owner: 'remote-owner',
    session: `session-${clock}`, startedAt: clock + 1, revision: 1, theme: structuredClone(theme), ...changes });
  const receive = value => channel.onmessage?.({ data: value });
  return { cleanup, storage, values, writes, properties, element, document, window, channel, theme, packet, receive };
}

test('protocol decoder rejects malformed, unbounded, spoofed or invalid-palette packets', t => {
  const env = environment(t);
  assert.ok(validateThemePreviewMessage(env.packet()));
  for (const value of [null, [], {}, env.packet({ version: 2 }), env.packet({ sender: 'other' }),
    env.packet({ type: 'unknown' }), env.packet({ revision: 0 }), env.packet({ revision: 0.5 }),
    env.packet({ startedAt: -1 }), env.packet({ owner: 'bad:owner' }), env.packet({ session: '' }),
    env.packet({ theme: { ...env.theme, palette: {} } }), env.packet({ error: 'x'.repeat(241) }),
    env.packet({ extra: 'x'.repeat(100_000) })]) {
    assert.equal(validateThemePreviewMessage(value), null);
  }
});

test('newer preview applies in memory; duplicate/stale revisions cannot revert it', t => {
  const env = environment(t);
  env.receive(env.packet());
  assert.equal(env.element.dataset.theme, 'dracula');
  assert.equal(currentPreviewTheme().id, env.theme.id);
  const next = { ...env.theme, palette: setThemePaletteColor(env.theme.palette, '--js-color-accent', '#123456') };
  env.receive(env.packet({ revision: 3, theme: next }));
  env.receive(env.packet({ revision: 2 }));
  env.receive(env.packet({ revision: 3 }));
  assert.equal(currentPreviewTheme().palette['--js-color-accent'], '#123456');
  assert.deepEqual(env.writes, []);
});

test('cancel/end retires session and rejects delayed snapshots even after sync disposal/reinstall', t => {
  const env = environment(t);
  const preview = env.packet();
  env.receive(preview);
  env.receive(env.packet({ type: 'end', revision: 2, theme: undefined }));
  assert.equal(getThemePreviewStatus().active, false);
  assert.equal(env.element.dataset.theme, 'nord');
  env.receive({ ...preview, revision: 100 });
  assert.equal(getThemePreviewStatus().active, false);
  env.cleanup();
  const reinstall = installThemePreviewSync();
  t.after(reinstall);
  // A retired packet cannot reanimate the old subscriber either.
  env.receive({ ...preview, revision: 101 });
  assert.equal(getThemePreviewStatus().active, false);
});

test('end received before late preview prevents resurrection; older competing session cannot win', t => {
  const env = environment(t);
  env.receive(env.packet({ type: 'end', revision: 2, theme: undefined }));
  env.receive(env.packet({ revision: 1 }));
  assert.equal(getThemePreviewStatus().active, false);
  env.receive(env.packet({ session: 'new-session', startedAt: clock + 2 }));
  env.receive(env.packet({ session: 'older-session', startedAt: clock, revision: 999 }));
  assert.equal(getThemePreviewStatus().session, 'new-session');
});

test('local owner answers late-join queries with current preview and matching presence', t => {
  const env = environment(t);
  const editor = createThemeEditor('nord');
  const draft = editor.getDraft();
  editor.update({ ...draft, palette: setThemePaletteColor(draft.palette, '--js-color-accent', '#123456') });
  env.channel.sent.length = 0;
  env.receive({ version: 1, type: 'query', sender: 'late-renderer', requestId: 'late-request' });
  const preview = env.channel.sent.find(packet => packet.type === 'preview');
  assert.equal(preview.theme.palette['--js-color-accent'], '#123456');
  assert.equal(env.channel.sent.find(packet => packet.type === 'presence').requestId, 'late-request');
  assert.deepEqual(env.writes, []);
});

test('healthy remote owner answering queries survives far beyond a nominal lease', t => {
  const env = environment(t);
  env.receive(env.packet());
  env.channel.responder = message => {
    if (message.type === 'query') env.receive(env.packet({ type: 'presence', requestId: message.requestId, theme: undefined }));
  };
  // More than an hour of ordinary query/response, not unrequested heartbeat.
  for (let i = 0; i < 250; i++) t.mock.timers.tick(15_000);
  assert.equal(getThemePreviewStatus().active, true);
  assert.equal(getThemePreviewStatus().error, null);
});

test('hidden observer and delayed timer cannot falsely retire a healthy hidden owner', t => {
  const env = environment(t);
  env.receive(env.packet());
  env.document.visibilityState = 'hidden';
  queryThemePreviewPresence();
  t.mock.timers.tick(8_000);
  assert.equal(getThemePreviewStatus().active, true);
  env.document.visibilityState = 'visible';
  env.channel.responder = message => {
    if (message.type === 'query') env.receive(env.packet({ type: 'presence', requestId: message.requestId, theme: undefined }));
  };
  env.document.dispatchEvent(new Event('visibilitychange'));
  t.mock.timers.tick(60_000);
  assert.equal(getThemePreviewStatus().active, true);
});

test('two unanswered presence probes restore saved state and permanently retire crashed owner session', t => {
  const env = environment(t);
  env.receive(env.packet());
  queryThemePreviewPresence();
  t.mock.timers.tick(8_000);
  assert.equal(getThemePreviewStatus().active, true);
  t.mock.timers.tick(8_000);
  assert.equal(getThemePreviewStatus().active, false);
  assert.match(getThemePreviewStatus().error, /owner|closed|respond/i);
  assert.equal(env.element.dataset.theme, 'nord');
  env.receive(env.packet({ revision: 999 }));
  assert.equal(getThemePreviewStatus().active, false);
});

test('draft clones, reset, validation failure and storage failure preserve the last valid preview', t => {
  const env = environment(t);
  const editor = createThemeEditor('nord');
  const original = editor.getDraft();
  const detached = editor.getDraft(); detached.palette['--js-color-bg'] = '#000000';
  assert.deepEqual(editor.getDraft(), original);
  const next = { ...original, label: 'Changed', palette: setThemePaletteColor(original.palette, '--js-color-accent', '#123456') };
  editor.update(next);
  assert.throws(() => editor.update({ ...next, palette: { ...next.palette, '--js-color-accent': 'url(bad)' } }));
  assert.deepEqual(editor.getDraft(), next);
  env.storage.setItem = () => { throw new Error('quota'); };
  assert.throws(() => editor.save(), /save|full|storage/i);
  assert.deepEqual(editor.getDraft(), next);
  assert.equal(currentPreviewTheme().palette['--js-color-accent'], '#123456');
  editor.reset();
  assert.deepEqual(editor.getDraft().palette, original.startingPalette);
  editor.cancel();
  assert.equal(getThemePreviewStatus().active, false);
  assert.equal(env.element.dataset.theme, 'nord');
  assert.throws(() => editor.update(next), /active/i);
  editor.dispose();
});

test('successful save persists canonical state before end message; cancel restores latest saved selection', t => {
  const env = environment(t);
  const editor = createThemeEditor('nord');
  const original = editor.getDraft();
  editor.update({ ...original, label: 'Saved', palette: setThemePaletteColor(original.palette, '--js-color-accent', '#123456') });
  env.channel.responder = message => {
    if (message.type === 'end') assert.equal(JSON.parse(env.values.get('jasonshell.themes.v1')).selectedId, original.id);
  };
  const saved = editor.save();
  assert.equal(getSavedThemeState().selectedId, saved.id);
  assert.equal(env.element.dataset.customTheme, saved.id);
  const second = createThemeEditor(saved);
  second.update({ ...saved, palette: setThemePaletteColor(saved.palette, '--js-color-accent', '#654321') });
  second.cancel();
  assert.equal(env.properties.get('--js-custom-color-accent'), '#123456');
});

test('dirty local selection decline retains draft, preview and byte-identical persistence', async t => {
  const env = environment(t);
  const editor = createThemeEditor('nord', { onSelectionRequest: async () => false });
  const original = editor.getDraft();
  editor.update({ ...original, label: 'Dirty' });
  const before = env.values.get('jasonshell.themes.v1');
  assert.equal(await requestPreviewSelection('dracula'), false);
  assert.equal(editor.getDraft().label, 'Dirty');
  assert.equal(getThemePreviewStatus().active, true);
  assert.equal(env.values.get('jasonshell.themes.v1'), before);
});

test('dirty local selection acceptance commits selection then clears preview', async t => {
  const env = environment(t);
  const editor = createThemeEditor('nord', { onSelectionRequest: async () => true });
  editor.update({ ...editor.getDraft(), label: 'Dirty' });
  assert.equal(await requestPreviewSelection('dracula'), true);
  assert.equal(getThemePreviewStatus().active, false);
  assert.equal(getSavedThemeState().selectedId, 'dracula');
  assert.equal(env.element.dataset.theme, 'dracula');
});

test('unavailable owner selection times out with recoverable error and no false selection success', async t => {
  const env = environment(t);
  env.receive(env.packet());
  const result = requestPreviewSelection('base-light');
  const rejection = assert.rejects(result, /editor|answer|retry/i);
  t.mock.timers.tick(30_000);
  await rejection;
  assert.equal(getSavedThemeState().selectedId, 'nord');
  assert.deepEqual(env.writes, []);
});

test('status subscription and disposal release resources and reject outstanding remote requests', async t => {
  const env = environment(t);
  const statuses = [];
  const unsubscribe = subscribeThemePreview(value => statuses.push(value));
  env.receive(env.packet());
  const result = requestPreviewSelection('base-light');
  const rejection = assert.rejects(result, /disposed/i);
  unsubscribe(); const count = statuses.length;
  env.cleanup(); env.cleanup();
  await rejection;
  assert.equal(env.channel.closed, true);
  assert.equal(getThemePreviewStatus().active, false);
  assert.equal(statuses.length, count);
  t.mock.timers.tick(100_000);
  assert.equal(getThemePreviewStatus().active, false);
});

test('selection acknowledgements bind request owner/session and accept end-before-ack ordering', async t => {
  const env = environment(t);
  env.receive(env.packet());
  const pending = requestPreviewSelection('base-light');
  const request = env.channel.sent.find(packet => packet.type === 'select');
  assert.equal(request.expiresAt, clock + 30_000);
  env.receive({ version: 1, type: 'selection-result', sender: 'wrong-owner', owner: 'wrong-owner', session: request.session, requestId: request.requestId, accepted: true });
  env.receive({ version: 1, type: 'selection-result', sender: request.owner, owner: request.owner, session: 'wrong-session', requestId: request.requestId, accepted: true });
  env.receive(env.packet({ type: 'end', revision: 2, theme: undefined }));
  env.receive({ version: 1, type: 'selection-result', sender: request.owner, owner: request.owner, session: request.session, requestId: request.requestId, accepted: false });
  assert.equal(await pending, false);
  assert.equal(getSavedThemeState().selectedId, 'nord');
});

test('remote confirmation cannot commit after request expiry, and duplicate request ID asks once', async t => {
  const env = environment(t);
  let answer;
  let questions = 0;
  const editor = createThemeEditor('nord', { onSelectionRequest: () => { questions++; return new Promise(resolve => { answer = resolve; }); } });
  editor.update({ ...editor.getDraft(), label: 'Dirty' });
  const session = env.channel.sent.find(packet => packet.type === 'preview');
  const request = { version: 1, type: 'select', sender: 'other-renderer', owner: session.owner, session: session.session,
    requestId: 'unique-request', selectedId: 'dracula', expiresAt: clock + 30_000 };
  env.receive(request); env.receive(request);
  assert.equal(questions, 1);
  t.mock.timers.tick(30_001);
  answer(true);
  await Promise.resolve(); await Promise.resolve(); await Promise.resolve();
  assert.equal(getSavedThemeState().selectedId, 'nord');
  assert.equal(getThemePreviewStatus().active, true);
  assert.ok(env.channel.sent.some(packet => packet.type === 'selection-result' && packet.accepted === false));
  assert.deepEqual(env.writes, []);
});

test('a disposed editor cannot commit a later accepted asynchronous confirmation', async t => {
  const env = environment(t);
  let answer;
  const editor = createThemeEditor('nord', { onSelectionRequest: () => new Promise(resolve => { answer = resolve; }) });
  const pending = requestPreviewSelection('dracula');
  const rejection = assert.rejects(pending, /changed|retry/i);
  editor.dispose(); answer(true);
  await rejection;
  assert.equal(getSavedThemeState().selectedId, 'nord');
  assert.deepEqual(env.writes, []);
});

test('observer timer lag restarts presence rather than treating elapsed wall time as proof of owner death', t => {
  const env = environment(t);
  env.receive(env.packet());
  queryThemePreviewPresence();
  // Jump wall time, then deliver the previously scheduled deadline: a resumed
  // observer must not infer an uninterrupted eight-second response opportunity.
  t.mock.timers.setTime(clock + 120_000);
  t.mock.timers.tick(8_000);
  assert.equal(getThemePreviewStatus().active, true);
  assert.equal(getThemePreviewStatus().error, null);
});
