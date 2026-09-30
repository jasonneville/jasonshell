import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');

function body(text, start) {
  const open = text.indexOf('{', text.indexOf(start));
  assert.ok(open >= 0, `missing body: ${start}`);
  let depth = 0;
  for (let i = open; i < text.length; i += 1) {
    if (text[i] === '{') depth += 1;
    if (text[i] === '}' && --depth === 0) return text.slice(open + 1, i);
  }
  assert.fail(`unterminated body: ${start}`);
}

const hook = source('src-tauri/src/windows_key_hook.rs');
const runtime = source('src-tauri/src/speech_runtime.rs');
const target = source('src-tauri/src/speech_target.rs');
const mic = source('src/components/TopBarMicControl.svelte');
const topBar = source('src/components/TopBar.svelte');
const speechWrapper = source('src/lib/speech.ts');

test('WM_HOTKEY speech path prepares original target before emitting UI toggle', () => {
  assert.match(hook, /WM_HOTKEY\s*=>\s*\{\s*dispatch_hotkey\(/);
  const speech = body(body(hook, 'fn dispatch_hotkey('), 'ConfiguredHotkeyAction::Speech =>');
  const prepare = speech.search(/\.prepare_speech_paste_target\s*\(/);
  const emit = speech.indexOf('SPEECH_TOGGLE');
  assert.ok(prepare >= 0 && prepare < emit, 'prepare runtime snapshot synchronously before emitting toggle');
  assert.match(speech, /state::<SpeechRuntimeState>\(\)/);
});

test('preparation does not overwrite active recording target on second hotkey', () => {
  assert.ok(/fn prepare_speech_paste_target\s*\(/.test(runtime), 'runtime must expose pre-emit preparation API');
  const prepare = body(runtime, 'fn prepare_speech_paste_target(');
  assert.match(prepare, /inner\.capture\.is_some\(\)/);
  assert.ok(prepare.indexOf('inner.capture.is_some()') < prepare.indexOf('inner.paste_target ='),
    'recording guard must precede target assignment');
});

test('start chooses prepared target without requiring late foreground snapshot', () => {
  const start = body(runtime, 'fn start_speech_capture(');
  assert.match(start, /inner\.paste_target\.take\(\)/);
  const fallback = start.indexOf('capture_speech_paste_target()');
  assert.ok(fallback < 0 || start.indexOf('inner.paste_target.take()') < fallback,
    'prepared snapshot must be selected before any fallback capture');
  assert.doesNotMatch(start, /let fallback_target\s*=\s*crate::speech_target::capture_speech_paste_target\(\)/,
    'eager fallback can fail even if the prepared snapshot is valid');
});

test('pointer-down capture remains available independently of hotkey route', () => {
  assert.match(mic, /captureSpeechPasteTarget\s*\(/);
  assert.match(runtime, /fn capture_speech_paste_target\s*\(/);
});

test('clipboard publishes before paste; exact window and focus identity fail closed', () => {
  const delivery = runtime.slice(runtime.indexOf('let paste_target = state'), runtime.indexOf('fn transcribe_window('));
  const write = delivery.indexOf('write_unicode_text(text)');
  const paste = delivery.indexOf('paste_to_captured_target(paste_target)');
  assert.ok(write >= 0 && paste > write, 'clipboard copy precedes optional paste');
  assert.doesNotMatch(delivery.slice(paste), /(?:clear|empty)_clipboard|write_unicode_text\s*\(/,
    'paste failure must not clear or replace the already-copied transcript');
  const pasteBody = body(target, 'fn paste_to_captured_target(');
  assert.match(pasteBody, /target_identity\(hwnd\)\s*!=\s*Some\(target\)/);
  assert.match(pasteBody, /GetForegroundWindow\(\)/);
  assert.ok(pasteBody.indexOf('TargetChanged') < pasteBody.indexOf('unsafe { SendInput('),
    'changed target aborts before injecting input');
  assert.match(target, /focus_hwnd/);
  assert.match(target, /process_id/);
  assert.match(target, /thread_id/);
});

test('rapid idle hotkeys reserve one prepared target before native capture and cannot replace it', () => {
  const prepare = body(runtime, 'fn prepare_speech_paste_target(');
  const snapshot = prepare.indexOf('crate::speech_target::capture_speech_paste_target()');
  assert.ok(snapshot >= 0, 'first hotkey must snapshot foreground synchronously');
  const guard = prepare.slice(0, snapshot);
  assert.ok(/(?:prepared|reservation|reserved|pending|paste_target)/.test(guard) &&
    /(?:is_some|matches!|compare_exchange|swap)/.test(guard),
  'second pre-start hotkey must detect ownership before capturing another foreground');
  assert.ok(/(?:prepared|reservation|reserved|pending|paste_target)/.test(prepare.slice(snapshot)) &&
    /(?:is_none|is_some|matches!|compare_exchange)/.test(prepare.slice(snapshot)),
  'revalidate reservation when storing snapshot so concurrent hotkeys cannot overwrite first owner');
});

test('worker delivery takes a target belonging to its nonce, never an untagged global slot', () => {
  const handoff = runtime.slice(runtime.indexOf('let paste_target = state'), runtime.indexOf('publish_recorded_if_current(', runtime.indexOf('let paste_target = state')));
  assert.ok(handoff.includes('nonce'), 'target handoff must check worker session nonce');
  assert.ok(!/\.paste_target\s*\.take\(\)/.test(handoff) ||
    /(?:paste_target|target)[\s\S]*?(?:nonce|generation)[\s\S]*?\.take\(\)/.test(handoff),
  'do not take a bare target from a newer session after an old worker passes preflight');
  const start = body(runtime, 'fn start_speech_capture(');
  assert.ok(/(?:paste_target|prepared_target)[\s\S]*nonce|nonce[\s\S]*(?:paste_target|prepared_target)/.test(start),
    'start must associate its target with the returned session nonce');
});

test('ignored or failed activation invalidates preparation; manual pointer capture owns its new activation', () => {
  const clickCapture = body(runtime, 'fn capture_speech_paste_target(');
  assert.doesNotMatch(clickCapture, /if inner\.paste_target\.is_none\(\)\s*\{\s*inner\.paste_target\s*=\s*Some\(target\)/,
    'manual click cannot silently reuse an unrelated idle hotkey snapshot');
  const prepare = body(runtime, 'fn prepare_speech_paste_target(');
  assert.ok(/(?:reservation|reserved|owner|pending|generation)/.test(prepare),
    'preparation must record ownership so ignored events can be invalidated');
  assert.ok(/(?:cancel|clear|invalidate|release|reset)_speech_(?:paste_target|target|preparation|activation)/.test(hook + mic + runtime),
    'ignored or failed activation needs an explicit release path, not indefinite preservation');
});

test('native clipboard and input cannot indefinitely hold shutdown delivery mutex', () => {
  const publish = body(runtime, 'fn publish_recorded_if_current<');
  const lock = publish.indexOf('.delivery');
  const native = publish.indexOf('publish()');
  assert.ok(lock >= 0 && native > lock, 'inspect delivery gate and native publisher in same routine');
  const beforeNative = publish.slice(lock, native);
  assert.ok(/drop\(_delivery\)|(?:delivery|_delivery)\s*=\s*None/.test(beforeNative) ||
    /(?:native_(?:delivery_)?timeout|(?:clipboard|paste|input)_deadline|bounded_native_delivery|nonblocking_native_delivery)/i.test(beforeNative),
  'release delivery mutex before blocking native work, or document and enforce bounded liveness');
});

test('expired hotkey events cannot consume a newer reservation across the entire activation path', async (t) => {
  await t.test('native hotkey emits its own newly prepared reservation ID, not an empty toggle', () => {
    const speech = body(body(hook, 'fn dispatch_hotkey('), 'ConfiguredHotkeyAction::Speech =>');
    assert.match(speech, /(?:let|if let)\s+(?:Some\()?\s*(\w*reservation\w*|\w*activation\w*)\s*\)?\s*=\s*[\s\S]*?\.prepare_speech_paste_target\(\)/i);
    const emitted = speech.slice(speech.indexOf('SPEECH_TOGGLE'));
    assert.match(emitted, /SPEECH_TOGGLE\s*,\s*(?!\(\s*\))(?:\w*reservation\w*|\w*activation\w*)/i,
      'event payload must identify this hotkey reservation');
  });

  await t.test('TopBar listener forwards event payload to mic toggle unchanged', () => {
    assert.match(topBar, /listen(?:<[^>]+>)?\(SPEECH_HOTKEY_TOGGLE_EVENT,\s*\((\w+)\)\s*=>\s*\{\s*void micControl\?\.toggleSpeech\(\1\.payload\)/,
      'a delayed first event must retain its own reservation ID through the listener');
  });

  await t.test('mic toggle forwards hotkey ID or its own manual capture ID to start', () => {
    assert.match(mic, /export async function toggleSpeech\([^)]*(?:reservation|activation)/i);
    const toggle = body(mic, 'export async function toggleSpeech(');
    assert.match(toggle, /startSpeechCapture\([^)]*(?:reservation|activation)/i,
      'start must not use a no-argument lookup of the current reservation');
    assert.match(mic, /captureSpeechPasteTarget\(\)/);
    assert.match(mic, /(?:pasteTargetCapture|targetCapture)[\s\S]*?(?:reservation|activation)/i,
      'manual pointer path must pass its own captured reservation ID');
  });

  await t.test('typed start IPC sends reservation ID in request instead of invoking without arguments', () => {
    const wrapper = body(speechWrapper, 'function startSpeechCapture(');
    assert.match(speechWrapper, /function startSpeechCapture\([^)]*(?:reservation|activation)/i);
    assert.match(wrapper, /invoke<StartSpeechCaptureResponse>\(IPC_COMMANDS\.startSpeechCapture,\s*\{\s*request\s*\}/,
      'request containing activation ID must cross IPC');
  });

  await t.test('native start matches requested ID and expiry before consuming target or starting session', () => {
    const start = body(runtime, 'fn start_speech_capture(');
    const signature = runtime.slice(runtime.indexOf('fn start_speech_capture('), runtime.indexOf('{', runtime.indexOf('fn start_speech_capture(')));
    assert.match(signature, /request\s*:\s*\w*(?:Start|Speech)\w*Request/,
      'start must receive an explicit activation request');
    const consume = start.search(/(?:\.prepared\.take\(\)|\.paste_target\.take\(\)|\.controller\.start\()/);
    const guard = start.slice(0, consume);
    assert.ok(consume >= 0 && /request\.(?:reservation_id|reservationId|activation_id|activationId)/.test(guard) &&
      /\.reservation_id|\.activation_id/.test(guard) && /prepared_at\.elapsed\(\)|expires_at|deadline/.test(guard) &&
      /(?:return Err|ok_or|filter|is_none_or|is_some_and)/.test(guard),
    'reject stale/mismatched activation BEFORE taking target or starting recording');
  });
});

test('delayed Stop hotkey carries original nonce and cannot stop a newer recording', () => {
  const prepare = body(runtime, 'fn prepare_speech_paste_target(');
  assert.ok(/SpeechHotkeyActivation::Stop\s*\{\s*nonce\s*:|SpeechHotkeyActivation::Stop\s*\(\s*(?:nonce|\w+\.nonce)/.test(prepare),
    'native Stop must embed the currently active nonce, not a bare Stop marker');
  const listener = topBar.slice(topBar.indexOf('listen<{kind:'), topBar.indexOf('listen<{kind:') + 330);
  assert.match(listener, /kind:\s*'stop'\s*;\s*nonce:\s*number/);
  assert.match(listener, /toggleSpeech\(event\.payload\)/);
  const toggle = body(mic, 'export async function toggleSpeech(');
  assert.match(mic, /kind:\s*'stop'\s*;\s*nonce:\s*number/);
  assert.ok(/activation\?\.kind\s*===\s*'stop'[\s\S]*?activation\.nonce\s*!==\s*micModel\.nonce/.test(toggle.slice(0, toggle.indexOf('beginSpeechStop('))),
    'reject stale Stop before transitioning current recording to stopping');
  assert.match(toggle, /stopSpeechCapture\(\{\s*nonce\s*\}\)/,
    'stop IPC must preserve the nonce validated against the event');
  assert.match(body(runtime, 'fn stop_speech_capture('), /request\.nonce/);
});

test('failed hotkey emission cancels only its own reservation, never a replacement manual capture', () => {
  const speech = body(body(hook, 'fn dispatch_hotkey('), 'ConfiguredHotkeyAction::Speech =>');
  assert.match(speech, /cancel_speech_preparation\(\s*(?:reservation\.reservation_id|reservation\.reservationId|reservation_id|reservationId)\s*\)/,
    'emit failure must pass the exact emitted reservation ID');
  const cancel = body(runtime, 'fn cancel_speech_preparation(');
  const signature = runtime.slice(runtime.indexOf('fn cancel_speech_preparation('), runtime.indexOf('{', runtime.indexOf('fn cancel_speech_preparation(')));
  assert.match(signature, /(?:reservation_id|reservationId)\s*:\s*u64/);
  assert.ok(/(?:prepared|inner\.prepared)[\s\S]*?reservation_id\s*==\s*reservation_id/.test(cancel) &&
    /inner\.prepared\s*=\s*None|inner\.prepared\.take\(\)/.test(cancel),
  'clear only when current prepared owner equals the failed event reservation');
});

test('shutdown and generation are checked immediately before potentially restoring focus', () => {
  const delivery = runtime.slice(runtime.indexOf('let _commit = state.commit.lock()', runtime.indexOf('wait_for_finalizing(&app, nonce)')),
    runtime.indexOf('publish_recorded_if_current(', runtime.indexOf('wait_for_finalizing(&app, nonce)')) + 2400);
  const focus = delivery.indexOf('prepare_captured_target(paste_target)');
  assert.ok(focus >= 0, 'focus preparation must be located in worker delivery path');
  const preceding = delivery.slice(delivery.lastIndexOf('write_unicode_text(text)', focus), focus);
  assert.ok(/state\.shutting_down\.load\(Ordering::Acquire\)/.test(preceding) &&
    /state\.generation\.load\(Ordering::Acquire\)\s*!=\s*nonce\.0/.test(preceding) &&
    /(?:return Err|\?)/.test(preceding),
  'after clipboard publication, reject shutdown or stale session immediately before native focus preparation');
  assert.match(target, /fn prepare_captured_target\s*\(/);
});

test('admitted focus work has a shutdown-drained lease without holding a mutex over Win32 focus', () => {
  const focus = runtime.indexOf('crate::speech_target::prepare_captured_target(paste_target)');
  assert.ok(focus >= 0, 'native focus operation must remain explicit');
  const beforeFocus = runtime.slice(Math.max(0, focus - 850), focus);
  const afterFocus = runtime.slice(focus, focus + 300);
  assert.ok(/(?:lease|admission|in_flight|inflight)/i.test(beforeFocus) &&
    /(?:lease|admission|in_flight|inflight)/i.test(runtime.slice(0, focus)),
  'admit and track focus work before invoking potentially blocking Win32 focus');
  assert.doesNotMatch(beforeFocus.slice(-350), /(?:let\s+_\w+\s*=\s*state\.\w+\.lock\(\)|\.lock\(\)[\s\S]*?prepare_captured_target)/,
    'no runtime mutex guard should span native focus preparation');
  assert.ok(/(?:lease|admission|in_flight|inflight)/i.test(afterFocus) ||
    /(?:struct|impl)\s+\w*(?:Lease|Admission)\w*/.test(runtime),
  'admission must be released after focus, including error paths (prefer RAII)');
  const shutdown = body(runtime, 'fn shutdown(state:');
  assert.ok(/(?:lease|admission|in_flight|inflight|wait_for_focus)/i.test(shutdown),
    'shutdown must drain admitted focus before returning');
  assert.ok(/#\[test\][\s\S]{0,140}fn\s+\w*(?:focus|admission|lease)\w*(?:shutdown|drain|wait)\w*\(/i.test(runtime) ||
    /#\[test\][\s\S]{0,140}fn\s+\w*(?:shutdown|drain|wait)\w*(?:focus|admission|lease)\w*\(/i.test(runtime),
  'Rust regression must deterministically exercise shutdown waiting on admitted focus work');
});

test('nonce-bound Stop during starting is deferred and dispatched only after matching start settlement', () => {
  const toggle = body(mic, 'export async function toggleSpeech(');
  const startSettlement = toggle.indexOf('acceptSpeechStart(micModel, response)');
  assert.ok(startSettlement >= 0, 'native start response must still establish recording state');
  assert.ok(/activation\?\.kind\s*===\s*'stop'[\s\S]*?micModel\.state\s*===\s*'starting'/.test(toggle.slice(0, toggle.indexOf('beginSpeechStart('))),
    'starting-state hotkey Stop must be handled before generic recording-only rejection');
  const deferred = mic.match(/(?:let|const)\s+(\w*(?:pending|deferred)\w*(?:Stop|Nonce)\w*|\w*Stop\w*(?:pending|deferred)\w*)\s*[:=]/i);
  assert.ok(deferred, 'retain the Stop event nonce while start IPC is in flight');
  assert.ok(toggle.slice(0, toggle.indexOf('beginSpeechStart(')).includes(deferred[1]),
    'starting-state Stop must store its event nonce');
  const afterStart = toggle.slice(startSettlement);
  assert.ok(afterStart.includes(deferred[1]) && /response\.nonce/.test(afterStart) &&
    /stopSpeechCapture\(\{\s*nonce\s*\}\)/.test(afterStart),
  'settled start must compare deferred nonce to response nonce before native stop');
  assert.match(toggle, /if \(micModel\.state === 'recording' && micModel\.nonce !== null\)/,
    'direct click-stop remains available for a recording session');
});
