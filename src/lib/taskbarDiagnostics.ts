import { invoke } from '@tauri-apps/api/core';

export type ToastListenerStatus = 'starting' | 'allowed' | 'denied' | 'unavailable' | 'error';

export type TaskbarRuntimeDiagnostics = {
  toastListenerStatus: ToastListenerStatus;
  statusUpdatedAtMs: number;
  packageIdentityStatus: { available: boolean; checked: boolean; error?: string | null };
  currentProcessPackageIdentity: { available: boolean; summary?: string | null };
  explorerTaskbarDiagnostics: {
    tracked: number;
    hidden: number;
    recreationFailures: number;
    hideFailures: number;
    lastError?: string | null;
  };
  lastSuccess?: { kind: string; message: string; timestampMs: number } | null;
  lastFailure?: { kind: string; message: string; timestampMs: number } | null;
  counters: {
    listenerStartAttempts: number;
    listenerPollAttempts: number;
    listenerPollSuccesses: number;
    listenerPollFailures: number;
    deniedRequests: number;
  };
  knownAppIdCount: number;
  unresolvedAppIdCount: number;
  unresolvedSample: string[];
  snapshot: {
    nativeHooks: { health: { shellHook: string; winEvent: string }; lastSignal?: { signal: string; timestampMs: number } | null };
    snapshotPipeline: { sequence: number; refreshReason: string; refreshedAtMs: number; latencyMs: number };
    attention: { trackedCount: number };
    toast: { status: ToastListenerStatus; lastPollAtMs: number; unresolvedCount: number };
    explorer: { tracked: number; hidden: number; recreationFailures: number; hideFailures: number; lastError?: string | null };
  };
};

const TOAST_STATUSES: ToastListenerStatus[] = ['starting', 'allowed', 'denied', 'unavailable', 'error'];
const MAX_UNRESOLVED_SAMPLE = 8;

export function normalizeToastListenerStatus(value: unknown): ToastListenerStatus {
  return typeof value === 'string' && TOAST_STATUSES.includes(value as ToastListenerStatus)
    ? (value as ToastListenerStatus)
    : 'unavailable';
}

function safeCount(value: unknown, max = Number.MAX_SAFE_INTEGER): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) return 0;
  return Math.min(max, Math.max(0, Math.floor(value)));
}

function redactIncomingString(value: string): string {
  const normalized = value.replace(/\\/g, '/');
  return /(?:file:\/\/\/|[A-Za-z]:\/|\/\/|\/server\/|\/Users\/|\/AppData\/)/i.test(normalized)
    ? '<path>'
    : normalized;
}

function sanitizeIncoming(value: unknown): unknown {
  if (typeof value === 'string') return redactIncomingString(value);
  if (Array.isArray(value)) return value.map(sanitizeIncoming);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value as Record<string, unknown>).map(([k, v]) => [k, sanitizeIncoming(v)]));
  }
  return value;
}

export function normalizeTaskbarRuntimeDiagnostics(value: unknown): TaskbarRuntimeDiagnostics {
  const source = (sanitizeIncoming(value) ?? {}) as Record<string, unknown>;
  const snapshot = (source.snapshot ?? {}) as Record<string, unknown>;
  const nativeHooks = (snapshot.nativeHooks ?? {}) as Record<string, unknown>;
  const nativeHooksHealth = (nativeHooks.health ?? {}) as Record<string, unknown>;
  const lastSignal = (nativeHooks.lastSignal ?? null) as Record<string, unknown> | null;
  const snapshotPipeline = (snapshot.snapshotPipeline ?? {}) as Record<string, unknown>;
  const attention = (snapshot.attention ?? {}) as Record<string, unknown>;
  const toast = (snapshot.toast ?? {}) as Record<string, unknown>;
  const snapshotExplorer = (snapshot.explorer ?? {}) as Record<string, unknown>;
  const counters = (source.counters ?? {}) as Record<string, unknown>;
  const packageIdentityStatus = (source.packageIdentityStatus ?? {}) as Record<string, unknown>;
  const currentProcessPackageIdentity = (source.currentProcessPackageIdentity ?? {}) as Record<string, unknown>;
  const explorerTaskbarDiagnostics = (source.explorerTaskbarDiagnostics ?? {}) as Record<string, unknown>;
  return {
    toastListenerStatus: normalizeToastListenerStatus(source.toastListenerStatus),
    statusUpdatedAtMs: safeCount(source.statusUpdatedAtMs),
    packageIdentityStatus: {
      available: Boolean(packageIdentityStatus.available),
      checked: Boolean(packageIdentityStatus.checked),
      error: typeof packageIdentityStatus.error === 'string' ? (packageIdentityStatus.error as string) : null
    },
    currentProcessPackageIdentity: {
      available: Boolean(currentProcessPackageIdentity.available),
      summary: typeof currentProcessPackageIdentity.summary === 'string'
        ? String(currentProcessPackageIdentity.summary)
        : null
    },
    explorerTaskbarDiagnostics: {
      tracked: safeCount(explorerTaskbarDiagnostics.tracked),
      hidden: safeCount(explorerTaskbarDiagnostics.hidden),
      recreationFailures: safeCount(explorerTaskbarDiagnostics.recreationFailures),
      hideFailures: safeCount(explorerTaskbarDiagnostics.hideFailures),
      lastError: typeof explorerTaskbarDiagnostics.lastError === 'string'
        ? explorerTaskbarDiagnostics.lastError
        : null
    },
    lastSuccess: source.lastSuccess && typeof source.lastSuccess === 'object'
      ? { ...(source.lastSuccess as { kind: string; message: string }), timestampMs: safeCount((source.lastSuccess as { timestampMs?: unknown }).timestampMs) }
      : null,
    lastFailure: source.lastFailure && typeof source.lastFailure === 'object'
      ? { ...(source.lastFailure as { kind: string; message: string }), timestampMs: safeCount((source.lastFailure as { timestampMs?: unknown }).timestampMs) }
      : null,
    counters: {
      listenerStartAttempts: Number(counters.listenerStartAttempts ?? 0),
      listenerPollAttempts: Number(counters.listenerPollAttempts ?? 0),
      listenerPollSuccesses: Number(counters.listenerPollSuccesses ?? 0),
      listenerPollFailures: Number(counters.listenerPollFailures ?? 0),
      deniedRequests: Number(counters.deniedRequests ?? 0)
    },
    knownAppIdCount: Number(source.knownAppIdCount ?? 0),
    unresolvedAppIdCount: Number(source.unresolvedAppIdCount ?? 0),
    unresolvedSample: Array.isArray(source.unresolvedSample)
      ? source.unresolvedSample.filter((item) => typeof item === 'string').slice(0, MAX_UNRESOLVED_SAMPLE)
      : [],
    snapshot: {
      nativeHooks: {
        health: {
          shellHook: typeof nativeHooksHealth.shellHook === 'string' ? nativeHooksHealth.shellHook : 'unknown',
          winEvent: typeof nativeHooksHealth.winEvent === 'string' ? nativeHooksHealth.winEvent : 'unknown'
        },
        lastSignal: lastSignal
          ? { signal: typeof lastSignal.signal === 'string' ? lastSignal.signal : '', timestampMs: safeCount(lastSignal.timestampMs) }
          : null
      },
      snapshotPipeline: {
        sequence: safeCount(snapshotPipeline.sequence),
        refreshReason: typeof snapshotPipeline.refreshReason === 'string' ? snapshotPipeline.refreshReason : 'unknown',
        refreshedAtMs: safeCount(snapshotPipeline.refreshedAtMs),
        latencyMs: safeCount(snapshotPipeline.latencyMs)
      },
      attention: {
        trackedCount: safeCount(attention.trackedCount)
      },
      toast: {
        status: normalizeToastListenerStatus(toast.status),
        lastPollAtMs: safeCount(toast.lastPollAtMs),
        unresolvedCount: safeCount(toast.unresolvedCount)
      },
      explorer: {
        tracked: safeCount(snapshotExplorer.tracked),
        hidden: safeCount(snapshotExplorer.hidden),
        recreationFailures: safeCount(snapshotExplorer.recreationFailures),
        hideFailures: safeCount(snapshotExplorer.hideFailures),
        lastError: typeof snapshotExplorer.lastError === 'string' ? snapshotExplorer.lastError : null
      }
    }
  };
}

export async function getTaskbarRuntimeDiagnostics(): Promise<TaskbarRuntimeDiagnostics> {
  const payload = await invoke<unknown>('get_taskbar_runtime_diagnostics');
  return normalizeTaskbarRuntimeDiagnostics(payload);
}
