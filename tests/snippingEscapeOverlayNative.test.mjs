// Cancellation-only real-overlay journey. Separate opt-in from the older
// clipboard/save journey: complete/copy/save handlers are absent in this mode.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { lstat, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const enabled = process.platform === 'win32' && process.env.JASONSHELL_SNIP_OVERLAY_ESCAPE_TESTS === '1';
const assertDiagnostics = process.env.JASONSHELL_SNIP_ASSERT_DIAGNOSTICS === '1';
const actualTopbarPointer = process.env.JASONSHELL_SNIP_TOPBAR_POINTER_TESTS === '1';
const callbackFields = ['callback_escape_down', 'callback_escape_up', 'callback_tls_missing', 'callback_owner_missing'];
const diagnosticFields = new Set(['schema', 'process_id', 'enabled', 'observed_generation', 'escape_down', 'escape_up', 'escape_repeat',
  'hook_registered', 'hook_install_failed', 'pump_ticks', 'hook_exit', 'worker_exit', 'shutdown', 'route_admitted', 'route_no_generation',
  'enqueued', 'drained', 'delivery_accepted', 'delivery_rejected', 'begin', 'arm_begin', 'arm_end', 'cancel_accepted', 'cancel_rejected', 'cancel_deferred',
  'phase', 'busy', 'armed', 'deferred', ...callbackFields]);
const boundaryCounters = [...callbackFields, 'escape_down', 'escape_up', 'escape_repeat', 'pump_ticks',
  'route_admitted', 'route_no_generation', 'enqueued', 'drained', 'delivery_accepted', 'delivery_rejected', 'cancel_accepted', 'cancel_rejected'];
const boundaryDelta = (before, after) => Object.fromEntries(boundaryCounters.map(key => [key, after[key] - before[key]]));
const delay = ms => new Promise(r => setTimeout(r, ms));
async function viteReady() {
  try {
    const r = await fetch('http://localhost:1420/src/lib/snipping.ts', { signal: AbortSignal.timeout(700) });
    const source = await r.text();
    return r.ok && source.includes('observeSnipContext') && source.includes('snipBarReady');
  } catch { return false; }
}
async function terminateOwned(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  const killer = spawn('taskkill.exe', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
  await once(killer, 'exit');
}
test('actual synthetic overlays cancel from owned native Escape without publication', { skip: !enabled, timeout: 120_000 }, async t => {
  let vite, child, exited, directory, protocolError, buffer = '';
  const messages = [];
  const fixtureId = `snip-journey-${process.pid}-${process.hrtime.bigint()}`;
  const wait = async (predicate, stage) => {
    const deadline = Date.now() + 14_000;
    while (Date.now() < deadline) {
      if (protocolError) throw protocolError;
      const index = messages.findIndex(predicate);
      if (index !== -1) return messages.splice(index, 1)[0];
      if (child && (child.exitCode !== null || child.signalCode !== null)) throw Error(`native fixture exited before ${stage}: ${child.exitCode}`);
      await delay(20);
    }
    throw Error(`native fixture timeout: ${stage}`);
  };
  try {
    const approved = join(process.env.LOCALAPPDATA, 'Temp/opencode');
    assert.ok(!(await lstat(approved)).isSymbolicLink());
    if (assertDiagnostics) {
      // Leader creates this only AFTER implementation/review. This test must
      // never enable diagnostics for an already-running installed user shell.
      const marker = await lstat(join(approved, 'snip-escape-diagnostics.enable'));
      assert.ok(marker.isFile() && !marker.isSymbolicLink(), 'leader-owned diagnostic opt-in must exist before launch');
    }
    directory = join(approved, fixtureId); await mkdir(directory);
    await writeFile(join(directory, 'parent-owner'), fixtureId, { flag: 'wx' });
    if (!await viteReady()) {
      vite = spawn('pwsh', ['-NoProfile', '-Command', 'npm run dev -- --port 1420 --strictPort'], { cwd: root, stdio: 'ignore' });
      const deadline = Date.now() + 12_000;
      while (!await viteReady() && Date.now() < deadline) await delay(100);
      assert.ok(await viteReady(), 'owned Vite must serve this product snipping module');
    }
    child = spawn(join(root, 'src-tauri/target/debug/examples/snip_synthetic_journey.exe'), ['--synthetic-escape-only'], {
      cwd: root, stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, JASONSHELL_SNIP_JOURNEY_FIXTURE_ID: fixtureId }
    });
    exited = once(child, 'exit');
    child.on('error', error => { protocolError = error; });
    child.stderr.on('data', () => {});
    child.stdout.setEncoding('utf8');
    child.stdout.on('data', text => {
      buffer += text;
      if (buffer.length > 16_384) { protocolError = Error('bounded native protocol exceeded'); return; }
      let end;
      while ((end = buffer.indexOf('\n')) !== -1) {
        const line = buffer.slice(0, end).trim(); buffer = buffer.slice(end + 1);
        if (!line) continue;
        try {
          const value = JSON.parse(line);
          if (value.error || value.renderer?.error) protocolError ??= Error(`native failure: op=${value.op ?? value.renderer?.op ?? 'startup'} code=${value.code ?? value.renderer?.code ?? value.error ?? value.renderer.error}`);
          messages.push(value);
          if (messages.length > 128) protocolError = Error('bounded native metadata queue exceeded');
        } catch { protocolError = Error('invalid native metadata'); }
      }
    });
    const ready = await wait(m => m.stage === 'ready', 'native READY');
    assert.equal(ready.fixtureId, fixtureId); assert.equal(ready.synthetic, true);
    assert.equal(ready.source, 'owned-offscreen-dib'); assert.equal(ready.escapeOnly, true);
    assert.equal(ready.publicationHandlersRegistered, false, 'clipboard publication must be unreachable');
    const diagnosticPath = join(approved, `snip-escape-diagnostics-${ready.processId}.json`);
    const diagnosticSnapshot = async predicate => {
      const deadline = Date.now() + 3000;
      while (Date.now() < deadline) {
        let value;
        try {
          const stat = await lstat(diagnosticPath);
          assert.ok(stat.isFile() && !stat.isSymbolicLink());
          assert.ok(stat.size <= 4096, 'diagnostic file is a bounded snapshot, never an append-only history');
          value = JSON.parse(await readFile(diagnosticPath, 'utf8'));
        } catch (error) {
          if (error.code !== 'ENOENT' && !(error instanceof SyntaxError)) throw error;
          await delay(25); continue;
        }
        assert.equal(value.process_id, ready.processId, 'read only exact owned synthetic process diagnostics');
        assert.equal(value.schema, 1); assert.equal(value.enabled, true);
        for (const key of diagnosticFields) assert.ok(Object.hasOwn(value, key), `required diagnostic field missing: ${key}`);
        for (const [key, scalar] of Object.entries(value)) {
          assert.ok(diagnosticFields.has(key), `unexpected diagnostic field ${key}`);
          assert.ok(['enabled', 'busy', 'armed', 'deferred'].includes(key) ? typeof scalar === 'boolean' : Number.isSafeInteger(scalar) && scalar >= 0,
            'diagnostics contain counters/enum numbers only, never typed text, raw keys, tokens, titles or paths');
        }
        // Cross-counter snapshots are approximate. Retry a transient ordering
        // mismatch rather than demanding transactional equality with route
        // counters (repeat, stopped and missing-owner paths intentionally differ).
        if (value.callback_escape_down < value.escape_down || value.callback_escape_up < value.escape_up
          || value.callback_tls_missing + value.callback_owner_missing > value.callback_escape_down + value.callback_escape_up) {
          await delay(25); continue;
        }
        if (lastDiagnostic) for (const key of callbackFields) {
          assert.ok(value[key] >= lastDiagnostic[key], `Escape-only callback counter must not regress: ${key}`);
        }
        if (predicate(value)) return value;
        await delay(25);
      }
      throw Error('deferred diagnostic snapshot did not expose required native stages');
    };
    let lastDiagnostic;
    if (assertDiagnostics) lastDiagnostic = await diagnosticSnapshot(d => d.hook_registered >= 1 && d.pump_ticks >= 1);
    const send = (op, payload = {}) => child.stdin.write(`${JSON.stringify({ ...payload, op })}\n`);
    const action = async (op, payload = {}) => { send(op, payload); return (await wait(m => m.renderer?.op === op, op)).renderer; };
    const inspect = async () => { send('inspect'); return wait(m => m.stage === 'inspect', 'inspect'); };
    await action('bar-ready');
    let previousGeneration = 0n;
    // One spawn, one managed runtime/listener, one foreground target. No settle
    // delay between a verified key-up/closed generation and the next start.
    for (let cycle = 1; cycle <= (actualTopbarPointer ? 8 : 24); cycle++) {
      const entry = actualTopbarPointer && cycle > 4 ? 'topbar-pointer' : cycle % 2 === 1 ? 'start-native' : actualTopbarPointer ? 'topbar-pointer' : 'start';
      const dragging = actualTopbarPointer ? cycle > 4 : Math.floor((cycle - 1) / 2) % 2 === 1;
      const repeat = !actualTopbarPointer && cycle % 3 === 0;
      t.diagnostic(`cycle=${cycle}, entry=${entry}, backendSelectionBound=${dragging}, repeatDuringClose=${repeat}`);
      let token;
      if (actualTopbarPointer) {
        assert.equal((await action('focus-target')).foregroundIsTarget, true);
      }
      if (entry === 'topbar-pointer') {
        const point = await action('bar-button-point');
        const pointer = await action('topbar-pointer', point);
        const click = (await wait(m => m.renderer?.op === 'topbar-ui-click', 'trusted actual button click')).renderer;
        t.diagnostic(`actualTopbarClick=${JSON.stringify({ pointer, click })}`);
        assert.equal(click.trusted, true, 'actual native pointer must invoke production button event path');
        assert.equal(click.documentFocused, true, 'actual topbar WebView must own keyboard focus at click');
        assert.equal(pointer.foregroundIsTopbar, true);
      } else {
        token = await action(entry);
      }
      const frozen = await wait(m => m.stage === 'synthetic-dib-frozen', 'generated pixels');
      assert.deepEqual([frozen.width, frozen.height, frozen.source], [640, 400, 'owned-offscreen-dib']);
      if (entry === 'topbar-pointer') {
        const deadline = Date.now() + 4000;
        let staged;
        do { staged = await inspect(); if (staged.phase === 'Some(Selecting)' && staged.nativeHelperCount === 1) break; await delay(20); } while (Date.now() < deadline);
        assert.equal(staged.phase, 'Some(Selecting)'); assert.equal(staged.nativeHelperCount, 1);
        assert.equal(staged.foregroundIsTopbar, true, 'do not replace real topbar foreground with external focus');
      }
      const overlay = await action('overlay-ready');
      token ??= overlay;
      assert.equal(BigInt(token.generation), previousGeneration + 1n, 'same coordinator advances exactly one generation per retry');
      previousGeneration = BigInt(token.generation);
      assert.equal(overlay.generation, token.generation); assert.equal(overlay.captureId, token.captureId);
      // Probe disabled handlers before any monitor binding: even an accidental
      // registration regression cannot complete/publish an unbound selection.
      assert.equal((await action('deny-publication')).denied, 3);
      if (dragging && actualTopbarPointer) {
        const point = await action('overlay-drag-point');
        const pointer = await action('overlay-pointer', point);
        const dom = (await wait(m => m.renderer?.op === 'overlay-ui-pointer', 'trusted held pointer-down')).renderer;
        t.diagnostic(`actualOverlayDrag=${JSON.stringify({ pointer, dom })}`);
        assert.equal(dom.trusted, true); assert.equal(pointer.foregroundIsOverlay, true); assert.equal(pointer.buttonHeld, true);
      } else if (dragging) {
        const begin = await action('begin-only');
        assert.equal(begin.generation, token.generation); assert.equal(begin.captureId, token.captureId);
      }
      const before = await inspect();
      assert.equal(before.processId, ready.processId, 'all cycles must reuse the same native process/runtime');
      assert.equal(before.phase, 'Some(Selecting)'); assert.equal(before.readinessCount, 1);
      assert.equal(before.nativeHelperCount, 1); assert.equal(before.visibleHelperCount, 1);
      assert.equal(before.publicationAttempts, 0);
      if (entry === 'topbar-pointer') assert.equal(dragging ? before.foregroundIsSnip : before.foregroundIsTopbar, true);
      let diagnosticBeforePulse;
      if (assertDiagnostics) {
        diagnosticBeforePulse = await diagnosticSnapshot(d => d.begin >= cycle && d.arm_end >= cycle
          && d.phase === 2 && d.armed === true && d.busy === false);
        t.diagnostic(`cycle=${cycle}, preEscapePrivacySafeDiagnostics=${JSON.stringify(diagnosticBeforePulse)}`);
      }
      if (entry === 'topbar-pointer') {
        const focus = await action(dragging ? 'overlay-keyboard-focus' : 'bar-keyboard-focus');
        t.diagnostic(`cycle=${cycle}, immediatelyBeforeEscapeDOMFocus=${JSON.stringify(focus)}`);
        assert.equal(focus.documentFocused, true, 'owned document focus must remain immediately before Escape; do not refocus');
        if (dragging) assert.equal(focus.pointerCaptured, true, 'hold real pointer capture until Escape disposes overlay');
      }
      const pulse = await action(entry === 'topbar-pointer' ? 'escape-focused' : repeat ? 'escape-repeat' : 'escape-pulse');
      t.diagnostic(`cycle=${cycle}, generation=${token.generation}, key=${JSON.stringify(pulse)}`);
      assert.equal(pulse.pulseMs, repeat ? 80 : 20);
      assert.equal(pulse.repeatCount, repeat ? 3 : 0);
      assert.equal(pulse.released, true, `Escape release must be verified before retry cycle ${cycle + 1}`);
      if (actualTopbarPointer) assert.equal(pulse.focusScope, dragging ? 'overlay' : entry === 'topbar-pointer' ? 'top-bar' : 'external-target');
      if (!(actualTopbarPointer && dragging)) assert.equal(pulse.foregroundUnchanged, true);
      let after; const deadline = Date.now() + 2000;
      do {
        after = await inspect();
        if (after.phase === 'None' && after.nativeHelperCount === 0) break;
        await delay(20);
      } while (Date.now() < deadline);
      t.diagnostic(`cycle=${cycle}, entry=${entry}, bound=${dragging}, afterEscape=${JSON.stringify(after)}`);
      if (after.phase !== 'None') {
        if (assertDiagnostics) {
          const failed = await diagnosticSnapshot(() => true);
          t.diagnostic(`failedEscapePrivacySafeDiagnostics=${JSON.stringify(failed)}`);
          t.diagnostic(`cycle=${cycle}, failedEscapeBoundaryDelta=${JSON.stringify(boundaryDelta(diagnosticBeforePulse, failed))}`);
        }
        // Exercise actual authorized cancel IPC solely for owned cleanup; report
        // its result separately, never reinterpret it as Escape success.
        send('cancel'); await delay(100);
        const cleanup = await inspect(); t.diagnostic(`direct cancel cleanup=${JSON.stringify(cleanup)}`);
      }
      assert.equal(after.phase, 'None', `native Escape must cancel actual overlay generation on cycle ${cycle}`);
      if (actualTopbarPointer && dragging) await action('mouse-release'); // Never release before cancellation verdict.
      if (repeat) {
        assert.equal(pulse.phaseBeforeRelease, 'None', 'repeat cycle must finish cancellation before key-up');
        assert.equal(pulse.nativeHelpersBeforeRelease, 0, 'key-up must be injected AFTER actual overlay disposal');
      }
      assert.equal(after.nativeHelperCount, 0); assert.equal(after.visibleHelperCount, 0);
      assert.equal(after.windowCount, 0); assert.equal(after.frameBytes, 0); assert.equal(after.imageBytes, 0);
      assert.equal(after.publicationAttempts, 0);
      if (assertDiagnostics) {
        const previous = lastDiagnostic;
        lastDiagnostic = await diagnosticSnapshot(d => d.escape_down > previous.escape_down && d.escape_up > previous.escape_up
          && d.route_admitted > previous.route_admitted && d.enqueued > previous.enqueued
          && d.drained > previous.drained
          && (actualTopbarPointer && dragging
            ? d.delivery_accepted + d.delivery_rejected > previous.delivery_accepted + previous.delivery_rejected
            : d.delivery_accepted > previous.delivery_accepted && d.cancel_accepted > previous.cancel_accepted)
          && d.begin >= cycle && d.arm_begin >= cycle && d.arm_end >= cycle
          && d.observed_generation === Number(token.generation) && d.pump_ticks > previous.pump_ticks
          && d.phase === 0 && d.busy === false && d.armed === false && d.deferred === false
          && (!repeat || d.escape_repeat >= previous.escape_repeat + 3));
        t.diagnostic(`cycle=${cycle}, privacySafeDiagnostics=${JSON.stringify(lastDiagnostic)}`);
        t.diagnostic(`cycle=${cycle}, successfulEscapeBoundaryDelta=${JSON.stringify(boundaryDelta(diagnosticBeforePulse, lastDiagnostic))}`);
      }
    }
    if (assertDiagnostics) {
      // Raw Escape observation must remain visible OUTSIDE Selecting, rather
      // than logging only admitted events and concealing a stuck pressed latch.
      const previous = lastDiagnostic;
      const pulse = await action('escape-pulse'); assert.equal(pulse.released, true);
      lastDiagnostic = await diagnosticSnapshot(d => d.escape_down > previous.escape_down
        && d.escape_up > previous.escape_up && d.route_no_generation > previous.route_no_generation);
      assert.equal(lastDiagnostic.enqueued, previous.enqueued, 'inactive Escape must not enqueue cancellation');
      assert.equal(lastDiagnostic.observed_generation, 0, 'inactive Escape must expose missing Selecting generation, not a stale target');
      const inactive = await inspect(); assert.equal(inactive.phase, 'None'); assert.equal(inactive.nativeHelperCount, 0);
      assert.equal(inactive.publicationAttempts, 0);
      t.diagnostic(`inactiveEscapePrivacySafeDiagnostics=${JSON.stringify(lastDiagnostic)}`);
      await action('diagnostic-stop');
      const terminal = await diagnosticSnapshot(d => d.shutdown >= 1 && d.hook_exit >= 1 && d.worker_exit >= 1);
      t.diagnostic(`listenerStoppedPrivacySafeDiagnostics=${JSON.stringify(terminal)}`);
    }
  } finally {
    if (child) {
      if (child.exitCode === null && child.signalCode === null) {
        child.stdin.end('{"op":"mouse-release"}\n{"op":"shutdown"}\n'); await Promise.race([exited, delay(6000)]);
      }
      await terminateOwned(child); if (exited) await exited;
    }
    await terminateOwned(vite);
    if (directory) {
      await rm(directory, { recursive: true, force: true, maxRetries: 12, retryDelay: 250 });
      await assert.rejects(lstat(directory), { code: 'ENOENT' });
      t.diagnostic('exact parent-owned profile removed after native process exit');
    }
  }
  assert.equal(child.exitCode, 0);
});
