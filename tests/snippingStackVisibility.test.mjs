import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import ts from 'typescript';

const source = readFileSync(new URL('../src/components/TopBar.svelte', import.meta.url), 'utf8');
const adapter = readFileSync(new URL('../src/lib/stackPopup.ts', import.meta.url), 'utf8');
function productionFunction(name) {
  const start = source.search(new RegExp(`  (?:async )?function ${name}\\(`));
  assert.notEqual(start, -1, `production ${name} exists`);
  const end = source.indexOf('\n  }', start);
  assert.notEqual(end, -1);
  return source.slice(start, end + 4);
}

// Execute the actual production pointer/start handlers, not a rewritten policy.
// IPC adapters model the verified native toggle contract: visible -> guarded
// close request; hidden + latest_request -> reopen. No HWND/capture claims.
function fixture({ visible = false, cachedOpen = false, latest = true, reject = false, kind = 'snip' } = {}) {
  const state = { visible, toggles: 0, closeRequests: 0, captured: [], starts: 0, reject, ipc: [] };
  class Element {
    constructor(classes = [], parentElement = null) { this.classes = classes; this.parentElement = parentElement; }
    closest(selector) {
      if (this.classes.includes(selector)) return this;
      return this.parentElement?.closest(selector) ?? null;
    }
  }
  // Start from the icon child, not just its enclosing button.
  const target = new Element([], new Element(kind === 'snip' ? ['.snip-button']
    : kind === 'stack' ? ['.stack-browser-button'] : []));
  const invoke = async (command, args) => {
    state.ipc.push({ command, args });
    assert.equal(command, 'toggle_stack_popup');
    if (!args?.closeOnly) state.toggles++;
    if (state.visible) { state.closeRequests++; return true; }
    if (args?.closeOnly) return false;
    if (latest) { state.visible = true; return true; }
    return null;
  };
  const adapters = ['toggleStackPopup', 'requestStackPopupClose'].map(name => {
    const start = adapter.search(new RegExp(`export (?:async )?function ${name}\\(`));
    assert.notEqual(start, -1);
    return adapter.slice(start, adapter.indexOf('\n}', start) + 2).replace('export ', '');
  }).join('\n');
  const script = `
    let stackBrowserOpen = ${cachedOpen}, snipStartPending = false, snipPreparing = null;
    let snipStartError = '', snipBarMounted = true;
    let soundCommandTransition = Promise.resolve(), disposed = false;
    let commandOpen = false, terminalOpen = false, audioOpen = false, trayOpen = false, searchOpen = false;
    let commandControl, terminalControl, soundControl, trayControl, searchControl;
    const isPinnedFolderPointerTarget = () => ${kind === 'pin'};
    const closeCommandPanel = async () => {}, closeTerminalPanel = closeCommandPanel,
      closeAudioPanel = closeCommandPanel, closeTrayPanel = closeCommandPanel, closePanel = closeCommandPanel;
    ${adapters}
    ${productionFunction('queueSoundCommandTransition')}
    ${productionFunction('handleTopBarPointerDown')}
    ${productionFunction('requestSnip')}
    return { pointer: async () => { handleTopBarPointerDown({ target }); await soundCommandTransition; },
      start: requestSnip, toggle: toggleStackPopup,
      error: () => snipStartError };
  `;
  const js = ts.transpileModule(script, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText;
  const handlers = new Function('Node', 'Element', 'target', 'invoke', 'IPC_COMMANDS', 'tick', 'startSnip', js)(
    Element, Element, target, invoke, { toggleStackPopup: 'toggle_stack_popup' }, async () => {}, async () => {
      state.starts++;
      if (state.reject) throw new Error('capture-failed');
      state.captured.push(state.visible);
    });
  return { state, handlers };
}

for (const scenario of [
  { name: 'never opened', latest: false },
  { name: 'explicit close', latest: true },
  { name: 'blur close with stale renderer open state', latest: true, cachedOpen: true }
]) {
  test(`snip pointerdown never reopens Stack: ${scenario.name}`, async () => {
    const { state, handlers } = fixture(scenario);
    await handlers.pointer();
    await handlers.start();
    assert.equal(state.visible, false, 'closed Stack must stay hidden before capture');
    assert.deepEqual(state.captured, [false]);
    assert.equal(state.toggles, 0, 'snipping must not toggle Stack');
  });
}

test('intentionally open Stack stays visible and capturable, without dirty-close request', async () => {
  const { state, handlers } = fixture({ visible: true, cachedOpen: true });
  await handlers.pointer();
  await handlers.start();
  assert.deepEqual(state.captured, [true]);
  assert.equal(state.closeRequests, 0, 'snip must not enqueue a later guarded close');
  assert.equal(state.toggles, 0);
});

test('failed snip and retry do not mutate Stack or retain stale start feedback', async () => {
  const { state, handlers } = fixture({ cachedOpen: true, reject: true });
  await handlers.pointer();
  await handlers.start();
  assert.match(handlers.error(), /Try again/);
  state.reject = false;
  await handlers.pointer();
  await handlers.start();
  assert.equal(handlers.error(), '');
  assert.deepEqual(state.captured, [false]);
  assert.equal(state.toggles, 0);
});

test('repeated snips preserve deliberately open Stack', async () => {
  const { state, handlers } = fixture({ visible: true, cachedOpen: true });
  for (let i = 0; i < 3; i++) { await handlers.pointer(); await handlers.start(); }
  assert.deepEqual(state.captured, [true, true, true]);
  assert.equal(state.closeRequests, 0);
});

test('ordinary outside dismissal executes production closeOnly adapter against stale hidden Stack', async () => {
  const { state, handlers } = fixture({ cachedOpen: true, kind: 'ordinary' });
  await handlers.pointer();
  assert.deepEqual(state.ipc, [{ command: 'toggle_stack_popup', args: { closeOnly: true } }]);
  assert.equal(state.visible, false);
  assert.equal(state.toggles, 0);
  assert.equal(state.closeRequests, 0);
});

test('ordinary outside dismissal retains visible dirty guard until it accepts closure', async () => {
  const { state, handlers } = fixture({ visible: true, cachedOpen: true, kind: 'ordinary' });
  await handlers.pointer();
  assert.equal(state.closeRequests, 1);
  assert.equal(state.visible, true, 'native close request is not an immediate hide');
  assert.equal(state.toggles, 0);
  assert.equal(state.ipc[0].args.closeOnly, true);
});

for (const kind of ['pin', 'stack']) {
  test(`${kind} pointerdown retains its dedicated switch/toggle path`, async () => {
    const { state, handlers } = fixture({ visible: true, cachedOpen: true, kind });
    await handlers.pointer();
    assert.deepEqual(state.ipc, []);
    assert.equal(state.visible, true);
  });
}

test('ordinary launcher adapter still toggles retained Stack open and requests guarded close', async () => {
  const { state, handlers } = fixture();
  assert.equal(await handlers.toggle(), true);
  assert.equal(state.visible, true);
  assert.deepEqual(state.ipc[0], { command: 'toggle_stack_popup', args: undefined });
  assert.equal(await handlers.toggle(), true);
  assert.equal(state.closeRequests, 1);
  assert.equal(state.visible, true, 'ordinary toggle also waits for dirty-editor guard');
});

test('ordinary launcher without retained request preserves null fallback', async () => {
  const { state, handlers } = fixture({ latest: false });
  assert.equal(await handlers.toggle(), null);
  assert.equal(state.visible, false);
});

test('native close-only branch precedes reopening and keeps visible dirty-editor guard', () => {
  const native = readFileSync(new URL('../src-tauri/src/stack_popup.rs', import.meta.url), 'utf8');
  const start = native.indexOf('pub fn toggle_stack_popup(');
  const body = native.slice(start, native.indexOf('\npub(crate) fn restore_stack_popup_topmost', start));
  assert.match(body, /close_only: Option<bool>/);
  assert.match(body, /if popup\.is_visible\(\)\.unwrap_or\(false\) \{[\s\S]*STACK_POPUP_CLOSE_REQUESTED[\s\S]*Ok\(Some\(true\)\)[\s\S]*else if close_only\.unwrap_or\(false\) \{[\s\S]*Ok\(Some\(false\)\)[\s\S]*else if popup_window::latest_stack_popup_request[\s\S]*reopen_stack_popup_window/);
  assert.doesNotMatch(body, /\.hide\(|hide_stack_popup_window/);
});

test('accepted native Stack blur runs shared hide lifecycle, resetting holds and publishing closure', () => {
  const main = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
  const start = main.search(/if window\.label\(\) == shell_windows::STACK_POPUP_LABEL\s*&& matches!\(event, WindowEvent::Focused\(false\)\)/);
  assert.notEqual(start, -1);
  const blur = main.slice(start, main.indexOf('if window.label() == shell_windows::SEARCH_PANEL_LABEL', start));
  assert.match(blur, /suppress_stack_popup_focus_loss[\s\S]*TOP_BAR_LABEL[\s\S]*stack_popup::hide_stack_popup\(window\.app_handle\(\)\.clone\(\)\)/);
  assert.doesNotMatch(blur, /window\.hide\(/);
  const popup = readFileSync(new URL('../src-tauri/src/stack_popup/popup_window.rs', import.meta.url), 'utf8');
  const hide = popup.slice(popup.indexOf('pub(crate) fn hide_stack_popup_window('), popup.indexOf('pub(crate) fn latest_stack_popup_request('));
  assert.match(hide, /focus_loss_hold_count = 0;[\s\S]*restore_focus_after_hold = false;[\s\S]*\.hide\(\)[\s\S]*\.emit_to\(TOP_BAR_LABEL, "stack-popup:closed", \(\)\)/);
});
