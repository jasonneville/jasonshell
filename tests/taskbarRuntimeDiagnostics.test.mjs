import assert from 'node:assert/strict';
import test from 'node:test';
import {
  normalizeTaskbarRuntimeDiagnostics,
  normalizeToastListenerStatus
} from '../src/lib/taskbarDiagnostics.ts';

test('normalizes missing taskbar diagnostics safely', () => {
  const diagnostics = normalizeTaskbarRuntimeDiagnostics(undefined);

  assert.equal(diagnostics.toastListenerStatus, 'unavailable');
  assert.equal(normalizeToastListenerStatus('denied'), 'denied');
  assert.equal(normalizeToastListenerStatus('bogus'), 'unavailable');
  assert.deepEqual(diagnostics.unresolvedSample, []);
  assert.equal(diagnostics.knownAppIdCount, 0);
  assert.equal(diagnostics.unresolvedAppIdCount, 0);
  assert.equal(diagnostics.snapshot.toast.unresolvedCount, 0);
});

test('keeps raw taskbar diagnostics bounded and redacted', () => {
  const diagnostics = normalizeTaskbarRuntimeDiagnostics({
    toastListenerStatus: 'allowed',
    statusUpdatedAtMs: 12,
    packageIdentityStatus: { available: true, checked: true, error: 'C:\\secret\\path' },
    currentProcessPackageIdentity: { available: true, summary: 'file:///C:/Users/Alice/App' },
    lastFailure: { kind: 'error', message: 'UNC \\server\\share\\x', timestampMs: 1 },
    unresolvedSample: Array.from({ length: 20 }, (_, index) => `Sample-${index}`),
    counters: { listenerPollAttempts: '3' },
    knownAppIdCount: 2,
    unresolvedAppIdCount: 20,
    snapshot: {
      nativeHooks: { health: { shellHook: 'Healthy', winEvent: 'Degraded' }, lastSignal: { signal: 'Foreground', timestampMs: 7 } },
      snapshotPipeline: { sequence: 4, refreshReason: 'manual', refreshedAtMs: 9, latencyMs: 2 },
      attention: { trackedCount: 3 },
      toast: { status: 'allowed', lastPollAtMs: 11, unresolvedCount: 5 },
      explorer: { tracked: 8, hidden: 2, recreationFailures: 1, hideFailures: 4, lastError: 'C:/secret/log.txt' }
    }
  });

  assert.equal(diagnostics.toastListenerStatus, 'allowed');
  assert.equal(diagnostics.statusUpdatedAtMs, 12);
  assert.equal(diagnostics.packageIdentityStatus.error, '<path>');
  assert.equal(diagnostics.currentProcessPackageIdentity.summary, '<path>');
  assert.equal(diagnostics.lastFailure.message, '<path>');
  assert.equal(diagnostics.counters.listenerPollAttempts, 3);
  assert.equal(diagnostics.unresolvedSample.length, 8);
  assert.equal(diagnostics.snapshot.nativeHooks.lastSignal.signal, 'Foreground');
  assert.equal(diagnostics.snapshot.snapshotPipeline.sequence, 4);
  assert.equal(diagnostics.snapshot.attention.trackedCount, 3);
  assert.equal(diagnostics.snapshot.toast.unresolvedCount, 5);
  assert.equal(diagnostics.snapshot.explorer.lastError, '<path>');
});

test('preserves unique unresolved count beyond sample cap', () => {
  const diagnostics = normalizeTaskbarRuntimeDiagnostics({
    toastListenerStatus: 'allowed',
    statusUpdatedAtMs: 44,
    unresolvedAppIdCount: 136,
    unresolvedSample: ['a', 'b']
  });

  assert.equal(diagnostics.unresolvedAppIdCount, 136);
  assert.deepEqual(diagnostics.unresolvedSample, ['a', 'b']);
});
