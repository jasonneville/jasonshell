import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import test from 'node:test';

const source = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const indicatorSource = () => {
  const path = 'src/components/SpeechIndicatorSurface.svelte';
  try { return source(path); }
  catch (error) {
    if (error?.code !== 'ENOENT') throw error;
    assert.fail(`${path} must exist as the dedicated Centerline UI`);
  }
};
const windows = source('src-tauri/src/shell_windows.rs');
const surfaces = source('src/ipc/surfaces.ts');
const loader = source('src/lib/surfaceLoader.ts');
const contracts = source('src-tauri/src/contracts.rs');
const runtime = source('src-tauri/src/speech_runtime.rs');
const events = source('src/ipc/events.ts');
const speech = source('src/lib/speech.ts');
const mic = source('src/components/TopBarMicControl.svelte');
const capabilitiesDir = new URL('../src-tauri/capabilities/', import.meta.url);
const capabilities = readdirSync(capabilitiesDir).filter((name) => name.endsWith('.json'))
  .map((name) => JSON.parse(readFileSync(new URL(name, capabilitiesDir), 'utf8')));

test('hidden speech-indicator webview has event, status-query and window access', () => {
  const indicator = indicatorSource();
  assert.match(indicator, /listenSpeechStatus|listenSpeechVoiceLevel/);
  assert.match(indicator, /getSpeechStatus\(/);
  assert.match(indicator, /getCurrentWindow\(\)\.(?:show|hide)\(/);
  const targets = capabilities.filter((capability) => capability.windows?.includes('speech-indicator'));
  assert.ok(targets.length, 'speech-indicator requires a declared Tauri capability target');
  assert.ok(targets.some(({ permissions }) => permissions?.includes('core:default') && permissions?.includes('core:window:default')),
    'indicator needs nearby panel precedent for event IPC and window methods');
});

test('Centerline reuses the packaged top-bar mic artwork in quiet and speaking states', () => {
  const indicator = indicatorSource();
  const micAsset = mic.match(/new URL\(\s*'([^']*mic_[^']*\.svg)'/)?.[1];
  assert.ok(micAsset, 'top-bar mic art must be a packaged SVG');
  assert.doesNotMatch(indicator, /<div\b[^>]*class="scan-line"|@keyframes scan-line|\.scan-line\s*\{/i);
  assert.ok(indicator.includes(micAsset) && /<img\b|background-image|url\(\$\{/.test(indicator),
    'Centerline must render the same packaged mic art as the top-bar control');
});

test('mic shell stays a fixed translucent 40px circle in quiet and speaking states', () => {
  const indicator = indicatorSource();
  const css = indicator.split('<style>')[1] ?? '';
  assert.match(indicator, /meterSnapshot\.showBars/, 'the existing meter gates the speaking edge');
  assert.match(indicator, /<img\b[^>]*class="mic-glyph"/);
  assert.match(css, /border-radius:\s*(?:50%|999(?:9)?px|50vw)/i, 'quiet mic shell must be circular');
  const shell = css.match(/\.mic-shell\s*\{([^}]+)\}/)?.[1] ?? '';
  assert.match(shell, /(?:^|;)\s*width:\s*40px\b/);
  assert.match(shell, /(?:^|;)\s*height:\s*40px\b/);
  const speaking = css.match(/\.mic-shell--speaking\s*\{([^}]+)\}/)?.[1] ?? '';
  assert.doesNotMatch(speaking, /(?:^|;)\s*(?:width|height|min-width|max-width|padding|gap|border-radius|transform):/i,
    'speaking must not override fixed circle geometry (old speaking width is 88px)');
  assert.match(css, /background:\s*(?:color-mix\(in srgb,\s*var\(--js-color-surface-[\w-]+\)\s*(?:4\d|5\d|6\d)%\s*,\s*transparent\)|rgba?\([^;]*[,/]\s*0\.[4-6]\d*\s*\))/i,
    'local surface-color backdrop should be about half transparent');
  assert.doesNotMatch(shell, /(?:^|;)\s*opacity:\s*0\.[4-6]\b/i,
    'translucency belongs to the backdrop, not the mic glyph or entire component');
  assert.doesNotMatch(css, /transition:\s*[^;]*(?:\ball\b|width|height|padding|gap|border-radius|transform)/i,
    'geometry must not transition');
  assert.match(css, /@media\s*\(prefers-reduced-motion:\s*reduce\)[\s\S]*?transition:\s*none/i);
  assert.match(css, /@media\s*\(forced-colors:\s*active\)[\s\S]*?(?:Canvas|CanvasText|border:\s*\d+px)/i);
  assert.doesNotMatch(indicator, /scan-line|idle-pulse|Math\.random/i);
});

test('speaking adds a steady cyan edge with a short color and inset fade, never waves or pulses', () => {
  const indicator = indicatorSource();
  const css = indicator.split('<style>')[1] ?? '';
  const speaking = css.match(/\.mic-shell--speaking\s*\{([^}]+)\}/)?.[1] ?? '';
  assert.match(speaking, /(?:border(?:-color)?|outline(?:-color)?):[^;]*#8fe3ff/i,
    'speech must brighten the edge to the steady cyan accent');
  assert.match(speaking, /box-shadow:[^;]*inset[^;]*#8fe3ff/i,
    'speech edge gets a restrained inset accent');
  assert.match(css, /transition:[^;]*(?:border-color|outline-color)\s+160ms/i);
  assert.match(css, /transition:[^;]*box-shadow\s+160ms/i);
  assert.doesNotMatch(indicator, /class="(?:waveform|bar)"|barScale|--bar-scale|meterSnapshot\.level/,
    'normalized volume must not drive presentation; internal showBars remains valid');
  assert.doesNotMatch(css, /\.waveform\b|\.bar\b|@keyframes|animation:\s*(?!\s*none\b)[^;]+|pulse/i);
});

test('forced colors keeps a distinguishable speaking edge without geometry changes', () => {
  const css = indicatorSource().split('@media (forced-colors: active)')[1] ?? '';
  const speaking = css.match(/\.mic-shell--speaking\s*\{([^}]+)\}/)?.[1] ?? '';
  assert.match(speaking, /(?:border(?:-color)?|outline(?:-color)?):[^;]*Highlight/i,
    'forced-colors speech must have an explicit distinguishable system-color edge');
  assert.doesNotMatch(speaking, /(?:^|;)\s*(?:width|height|padding|gap|border-radius|transform):/i);
});

test('forced colors replace the packaged image with a visible CanvasText mic mask', () => {
  const indicator = indicatorSource();
  const css = indicator.split('@media (forced-colors: active)')[1] ?? '';
  assert.match(indicator, /<span\b[^>]*class="mic-glyph-contrast"/);
  assert.match(css, /\.mic-glyph\s*\{[^}]*display:\s*none/);
  assert.match(css, /\.mic-glyph-contrast\s*\{[^}]*background:\s*CanvasText[^}]*display:\s*block[^}]*mask:\s*url\('[^']*mic_[^']*\.svg'\)/);
  assert.doesNotMatch(css, /object-position\s*:/, 'forced-colors mic must not rely on moving an image out of view');
});

test('Centerline has its own transparent, nonactivating, hidden native surface above bottom bar', () => {
  assert.match(windows, /SPEECH_INDICATOR_LABEL:\s*&str\s*=\s*"speech-indicator"/);
  assert.match(windows, /build_speech_indicator_window\(app\)/);
  const builder = windows.match(/fn build_speech_indicator_window\([\s\S]*?\n\}/)?.[0] ?? '';
  for (const setting of [/SPEECH_INDICATOR_LABEL/, /\.always_on_top\(true\)/, /\.transparent\(true\)/, /\.focused\(false\)/, /\.skip_taskbar\(true\)/, /\.visible\(false\)/]) {
    assert.match(builder, setting);
  }
  assert.match(windows, /speech_indicator[\s\S]*?(?:WS_EX_NOACTIVATE|set_ignore_cursor_events)/i);
  assert.match(windows, /speech_indicator[\s\S]*?(?:WS_EX_TOOLWINDOW|skip_taskbar)/i);
  assert.match(windows, /SPEECH_INDICATOR_LABEL[\s\S]*?BOTTOM_BAR_HEIGHT_LOGICAL/);
  assert.match(windows, /SPEECH_INDICATOR_LABEL[\s\S]*?(?:logical_width|monitor_width|screen_width)[\s\S]*?\/\s*2\.0/);
  assert.match(surfaces, /speechIndicator:\s*'speech-indicator'/);
  assert.match(loader, /'speech-indicator':\s*\(\)\s*=>\s*import\('[^']*SpeechIndicatorSurface\.svelte'\)/);
  const canonicalSurfaces = contracts.match(/pub mod surfaces\s*\{[\s\S]*?pub const ALL:\s*&\[&str\]\s*=\s*&\[([\s\S]*?)\];/)?.[1] ?? '';
  assert.match(canonicalSurfaces, /SPEECH_INDICATOR/, 'native canonical surface enumeration must include the indicator');
});

test('indicator tracks only matching recording nonce and hides on every nonrecording state', () => {
  const indicator = indicatorSource();
  assert.match(indicator, /listenSpeechStatus/);
  assert.match(indicator, /status\s*===\s*'recording'/);
  assert.match(indicator, /nonce/);
  assert.match(indicator, /(?:idle|transcribing|copied|error)/);
  assert.match(indicator, /(?:hideSpeechIndicator|\.hide\()/);
  assert.match(indicator, /(?:popup|overlay|panel)/i);
  assert.doesNotMatch(indicator, /\.setFocus\(|\.focus\(/);
});

test('nonrecording status hides the indicator even when its nonce differs from the recording nonce', () => {
  const indicator = indicatorSource();
  const handler = indicator.match(/listenSpeechStatus\(\(event\) => \{([\s\S]*?)\}\)\.then\(/)?.[1];
  assert.ok(handler, 'speech status listener must own indicator visibility');
  assert.match(handler, /event\.status\s*===\s*'recording'\s*&&\s*event\.nonce\s*!==\s*null[\s\S]*?showSpeechIndicator\(event\.nonce\)/);
  const nonrecording = handler.slice(handler.indexOf('showSpeechIndicator(event.nonce)'));
  assert.match(nonrecording, /hideSpeechIndicator\(\)/);
  assert.doesNotMatch(nonrecording, /if\s*\([^)]*(?:event\.nonce\s*===\s*activeNonce|activeNonce\s*===\s*event\.nonce)[^)]*\)\s*hideSpeechIndicator\(\)/,
    'a terminal status for another nonce must not leave a visible indicator stuck');
  assert.match(indicator, /event\.nonce\s*===\s*activeNonce\)\s*applyVoiceLevel\(event\.level\)/);
});

test('native speech lifecycle owns hidden indicator visibility, independent of its renderer', () => {
  assert.match(windows, /fn build_speech_indicator_window\([\s\S]*?\.visible\(false\)/);
  const publisher = runtime.match(/fn emit\(app: &AppHandle, event: &SpeechStatusEvent\)[\s\S]*?\n\}/)?.[0] ?? '';
  assert.match(publisher, /get_webview_window\(\s*(?:crate::shell_windows::)?SPEECH_INDICATOR_LABEL\s*\)/,
    'native status publication must locate the indicator window by its dedicated label');
  assert.match(publisher, /SpeechStatusKind::Recording[\s\S]*?\.show\(\)/,
    'native recording transition must show the initially hidden window');
  assert.match(publisher, /(?:SpeechStatusKind::(?:Idle|Transcribing|Copied|Error)|_)[\s\S]*?\.hide\(\)/,
    'native nonrecording transitions must hide it without waiting for Svelte listeners');
  const directBroadcasts = [...runtime.matchAll(/app\.emit\(contracts::events::SPEECH_STATUS_CHANGED,\s*event\)/g)];
  assert.equal(directBroadcasts.length, 1,
    'all status paths, including cap/timeout/error/reset, must use the native visibility publisher');
  assert.match(publisher, /app\.emit\(contracts::events::SPEECH_STATUS_CHANGED, event\)/);
});

test('native visibility I/O runs outside the serialized commit gate and reports failures safely', () => {
  const publisher = runtime.match(/fn emit\(app: &AppHandle, event: &SpeechStatusEvent\)[\s\S]*?\n\}/)?.[0] ?? '';
  assert.ok(publisher, 'status publisher must remain identifiable');
  const visibilityAction = /(?:indicator|window)\.(?:show|hide)\(\)/;
  assert.doesNotMatch(publisher, visibilityAction,
    'emit is called under commit locks; it must not perform native window I/O itself');
  const executor = runtime.match(/fn \w*(?:visibility|indicator)\w*\([\s\S]*?\n\}/i)?.[0] ?? '';
  assert.match(executor, /get_webview_window\(\s*(?:crate::shell_windows::)?SPEECH_INDICATOR_LABEL\s*\)/);
  assert.match(executor, /(?:\.show\(\)|\.hide\(\))/);
  assert.match(executor, /(?:\.is_err\(\)|if let Err\(|match\s+[^\n]*\{|\.map_err\(|\?)/,
    'native show/hide failure must be checked rather than assigned to an ignored result');
  assert.match(executor, /(?:debug_diagnostic\(|eprintln!|return Err\(|\.map_err\()/,
    'visibility failure must have bounded diagnostic or returned error handling');
  assert.doesNotMatch(executor, /(?:transcript|audio_samples|sample_buffer|raw_audio)/i);
  assert.match(runtime, /drop\((?:_?commit|guard)\);[\s\S]{0,450}\b\w*(?:visibility|indicator)\w*\(/i,
    'at least one lifecycle path must explicitly release commit before native visibility I/O');
  assert.match(executor, /(?:generation\.load|controller\.status\(\)|state\.inner\.lock)/,
    'visibility executor must check current generation/state before applying a delayed decision');
});

test('stale recording visibility plan cannot overtake a newer terminal transition', () => {
  const executor = runtime.match(/fn apply_indicator_visibility\([\s\S]*?\n\}/)?.[0] ?? '';
  assert.match(executor, /let\s+result\s*=\s*if\s+event\.status\s*==\s*SpeechStatusKind::Recording/);
  const action = executor.indexOf('indicator.show()');
  assert.ok(action >= 0, 'the recording plan shows the native window');
  const afterAction = executor.slice(action);
  // A pre-I/O status check alone is racy: stop can commit after validation but
  // before show(). Revalidate after the action and compensate with hide() if
  // recording/nonce/generation no longer matches, or use a shared ordering gate
  // held across validation and I/O that terminal transition also participates in.
  const compensatingHide = /(?:generation\.load|controller\.status\(\))[^]*?(?:SpeechStatusKind::Recording|event\.nonce)[^]*?indicator\.hide\(\)/.test(afterAction);
  const gate = runtime.match(/\b([a-z_]*visibility[a-z_]*(?:gate|lock|order)[a-z_]*)\s*:\s*Mutex<\(\)>/i)?.[1];
  const serializedWithTransition = gate &&
    new RegExp(`state\\.${gate}\\.lock\\(`).test(executor) &&
    new RegExp(`state\\.${gate}\\.lock\\(`).test(runtime.slice(0, runtime.indexOf('fn apply_indicator_visibility(')));
  assert.ok(compensatingHide || serializedWithTransition,
    'recording show must be corrected after concurrent stop or serialized with the terminal transition');
  assert.doesNotMatch(executor, /state\.commit\.lock\(/,
    'native visibility I/O must not acquire the lifecycle commit gate');
});

test('old recording compensation cannot hide a newer recording session', () => {
  const executor = runtime.match(/fn apply_indicator_visibility\([\s\S]*?\n\}/)?.[0] ?? '';
  const correction = executor.slice(executor.indexOf('recording_still_current'));
  assert.match(correction, /indicator\.hide\(\)/, 'stale recording plans must still correct visibility');
  // Merely checking state before hide is a TOCTOU race: recording B may commit
  // between the check for A and the native hide. Require a visibility ordering
  // gate shared by correction and recording transitions, or a post-hide reconcile
  // that restores B from authoritative state (without holding commit over I/O).
  const gate = runtime.match(/\b([a-z_]*visibility[a-z_]*(?:gate|lock|order)[a-z_]*)\s*:\s*Mutex<\(\)>/i)?.[1];
  const correctionSerialized = Boolean(gate &&
    new RegExp(`state\\.${gate}\\.lock\\(`).test(executor.slice(0, executor.indexOf('indicator.hide()', executor.indexOf('recording_still_current')))) &&
    new RegExp(`state\\.${gate}\\.lock\\(`).test(runtime.slice(0, runtime.indexOf('fn apply_indicator_visibility('))));
  const correctionEnd = correction.indexOf('indicator.hide()') + 'indicator.hide()'.length;
  const afterCorrection = correction.slice(correctionEnd);
  const restoresNewRecording = /(?:controller\.status\(\)|generation\.load)[\s\S]*?SpeechStatusKind::Recording[\s\S]*?indicator\.show\(\)/.test(afterCorrection);
  assert.ok(correctionSerialized || restoresNewRecording,
    'compensating hide must be serialized with recording transitions or reconcile a newer recording afterward');
  assert.doesNotMatch(executor, /state\.commit\.lock\(/,
    'do not hold the lifecycle commit lock across native window I/O');
});

test('reconciliation show cannot outlive a newer terminal status', () => {
  const executor = runtime.match(/fn apply_indicator_visibility\([\s\S]*?\n\}/)?.[0] ?? '';
  const compensation = executor.indexOf('recording_still_current');
  assert.ok(compensation >= 0, 'stale recording correction must remain present');
  const reconciliation = executor.slice(compensation);
  const show = reconciliation.lastIndexOf('indicator.show()');
  assert.ok(show >= 0, 'new recording recovery must remain present');
  // Reading recording before show is not enough: stop can commit/hide before
  // this late show. Check after the action and converge to hidden if terminal,
  // or run all transitions through a serialized visibility dispatcher.
  const afterShow = reconciliation.slice(show + 'indicator.show()'.length);
  const convergesAfterShow = /(?:controller\.status\(\)|generation\.load|\b(?:reconcile|converge|sync)_?\w*\()/i.test(afterShow) &&
    /(?:indicator\.hide\(\)|\b(?:reconcile|converge|sync)_?\w*\()/i.test(afterShow);
  const orderedDispatcher = /\b(?:visibility|indicator)_(?:dispatcher|queue|worker|serial)\b/i.test(executor) &&
    !/std::thread::spawn\(move \|\|/.test(executor);
  assert.ok(convergesAfterShow || orderedDispatcher,
    'late reconciliation show must revalidate and hide terminal status or use ordered visibility dispatch');
  assert.doesNotMatch(executor, /state\.commit\.lock\(/,
    'native window operations must stay outside lifecycle commit');
});

test('indicator hydrates the current speech status after subscribing, including an already recording session', () => {
  const indicator = indicatorSource();
  assert.match(indicator, /listenSpeechStatus/);
  // A late webview subscriber cannot recover a missed recording event from listen alone.
  // Hydration may use an existing command wrapper or a newly wired status query, but
  // must read authoritative native state rather than infer it from voice-level traffic.
  assert.match(indicator, /(?:getSpeechStatus|speechStatusSnapshot|currentSpeechStatus)\s*\(/,
    'indicator must query the authoritative current speech status on mount');
  assert.match(indicator, /showSpeechIndicator\(.*nonce/);
});

test('late hydration cannot replay recording after a newer status event', () => {
  const indicator = indicatorSource();
  const mount = indicator.match(/onMount\(\(\) => \{([\s\S]*?)\n  \}\);/)?.[1] ?? '';
  assert.match(mount, /listenSpeechStatus\(\(event\) => \{/);
  assert.match(mount, /getSpeechStatus\(\)/);
  // Revision may have any local name, but the same value must be advanced by
  // every observed event, captured before querying, and compared before apply.
  const increment = mount.match(/\b([A-Za-z_$][\w$]*)\s*(?:\+\+|\+=\s*1)\s*;/)?.[1];
  assert.ok(increment, 'each observed status must advance a local hydration revision');
  const listener = mount.slice(mount.indexOf('listenSpeechStatus((event) => {'), mount.indexOf('}).then((unlisten) => {'));
  assert.match(listener, new RegExp(`\\b${increment}\\s*(?:\\+\\+|\\+=\\s*1)\\s*;`));
  const hydration = mount.slice(mount.indexOf('getSpeechStatus()'));
  assert.match(mount.slice(0, mount.indexOf('getSpeechStatus()')),
    new RegExp(`\\b(?:const|let)\\s+([A-Za-z_$][\\w$]*)\\s*=\\s*${increment}\\s*;`),
    'snapshot the revision before requesting native status');
  const snapshot = mount.slice(0, mount.indexOf('getSpeechStatus()')).match(new RegExp(`\\b(?:const|let)\\s+([A-Za-z_$][\\w$]*)\\s*=\\s*${increment}\\s*;`))?.[1];
  assert.match(hydration, new RegExp(`${increment}\\s*===\\s*${snapshot}|${snapshot}\\s*===\\s*${increment}`),
    'apply hydration only if no status event has arrived since the query began');
});

test('top bar retains its primary mic action and has no fake pause or resume control', () => {
  assert.match(mic, /<MeltActionButton[\s\S]*?onClick=\{\(\) => \{ void toggleSpeech\(\); \}\}/);
  assert.match(mic, /startSpeechCapture/);
  assert.match(mic, /stopSpeechCapture/);
  assert.doesNotMatch(mic, /pauseSpeech|resumeSpeech|simulatePause|simulateResume/i);
});

test('voice-level IPC contains only a bounded normalized level and session nonce', () => {
  assert.match(contracts, /speech:voice-level/);
  assert.match(events, /speechVoiceLevel:\s*'speech:voice-level'/);
  const payload = events.match(/(?:interface|type) SpeechVoiceLevelEvent\s*(?:=\s*)?\{([^}]+)\}/)?.[1] ?? '';
  assert.match(payload, /nonce:\s*SpeechSessionNonce/);
  assert.match(payload, /level:\s*number/);
  assert.doesNotMatch(payload, /samples?|pcm|transcript|audio|content|blob|buffer/i);
  assert.match(speech, /listenSpeechVoiceLevel/);
  assert.match(runtime, /speech:voice-level|SPEECH_VOICE_LEVEL/i);
  const publisher = runtime.match(/fn schedule_voice_level_updates\([\s\S]*?\n\}/)?.[0] ?? '';
  assert.match(publisher, /app\.emit_to\(\s*(?:crate::shell_windows::)?(?:shell_windows::)?SPEECH_INDICATOR_LABEL\s*,\s*contracts::events::SPEECH_VOICE_LEVEL\s*,/,
    'only the indicator webview should receive microphone activity');
  assert.doesNotMatch(publisher, /app\.emit\(\s*contracts::events::SPEECH_VOICE_LEVEL/);
  assert.match(runtime, /(?:clamp\(0\.0,\s*1\.0\)|\.min\(1\.0\)\.max\(0\.0\))/);
  const callback = runtime.match(/build_input_stream\([\s\S]*?\n\s*\)\s*(?:\?|;)/)?.[0] ?? '';
  assert.doesNotMatch(callback, /\.emit\(|emit_to\(|speech:voice-level|SPEECH_VOICE_LEVEL/);
});

test('quiet recording shows mic artwork; real voice levels gate the speaking edge with a silence hold', () => {
  const indicator = indicatorSource();
  assert.match(indicator, /<img\b[^>]*class="mic-glyph"/);
  assert.doesNotMatch(indicator, /<div\b[^>]*class="scan-line"|@keyframes scan-line|\.scan-line\s*\{/i);
  assert.match(indicator, /(?:voiceLevel|voice_level|level)/);
  assert.match(indicator, /(?:threshold|noiseFloor|noise_floor)/i);
  assert.match(indicator, /(?:decay|holdMs|releaseMs)/i);
  assert.match(indicator, /meterSnapshot\.showBars/);
  assert.doesNotMatch(indicator, /Math\.random|fakeLevel|simulatedLevel|setInterval\([^)]*random/i);
  assert.doesNotMatch(indicator, /pauseSpeech|resumeSpeech|simulatePause|simulateResume/i);
});

test('meter wiring filters stale nonce, uses a testable 300ms silence hold and clears on terminal', () => {
  const indicator = indicatorSource();
  assert.match(indicator, /event\.nonce\s*===\s*activeNonce/);
  assert.match(indicator, /hideSpeechIndicator\(\)/);
  assert.match(indicator, /(?:createSpeechIndicatorMeter|createVoiceMeter)/,
    'component should consume the tested pure meter rather than resetting on every scalar event');
});

test('indicator respects reduced motion, forced colors and noninteractive accessibility', () => {
  const indicator = indicatorSource();
  assert.match(indicator, /prefers-reduced-motion:\s*reduce/);
  assert.match(indicator, /forced-colors:\s*active/);
  assert.match(indicator, /aria-hidden|role="status"/);
  assert.doesNotMatch(indicator, /<button|tabindex="0"|autofocus/i);
});
