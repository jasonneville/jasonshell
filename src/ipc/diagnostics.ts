export type DiagnosticLevel = 'debug' | 'info' | 'warn' | 'error';

export interface DiagnosticEntry {
  timestampEpochMs: number;
  level: DiagnosticLevel;
  source: string;
  message: string;
  fields?: Record<string, unknown>;
}

export interface DiagnosticExport {
  generatedAtEpochMs: number;
  entries: DiagnosticEntry[];
}

const SECRET_KEY_PATTERN = /(token|secret|password|credential|api[_-]?key|authorization|cookie)/iu;
const SECRET_VALUE_PATTERN = /(bearer\s+)[a-z0-9._~+/=-]+|([a-z0-9._%+-]+:)[a-z0-9._%+-]+(@)/giu;
const REDACTED = '[REDACTED]';

export function redactDiagnosticMessage(message: string): string {
  return message.replace(SECRET_VALUE_PATTERN, (_match, bearerPrefix, userPrefix, atSuffix) => {
    if (bearerPrefix) {
      return `${bearerPrefix}${REDACTED}`;
    }
    if (userPrefix && atSuffix) {
      return `${userPrefix}${REDACTED}${atSuffix}`;
    }
    return REDACTED;
  });
}

function redactDiagnosticValue(value: unknown): unknown {
  if (typeof value === 'string') {
    return redactDiagnosticMessage(value);
  }
  if (Array.isArray(value)) {
    return value.map(redactDiagnosticValue);
  }
  if (value && typeof value === 'object') {
    return redactDiagnosticFields(value as Record<string, unknown>);
  }
  return value;
}

export function redactDiagnosticFields(fields: Record<string, unknown> = {}): Record<string, unknown> {
  return Object.fromEntries(
    Object.entries(fields).map(([key, value]) => [
      key,
      SECRET_KEY_PATTERN.test(key) ? REDACTED : redactDiagnosticValue(value)
    ])
  );
}

export function createDiagnosticsRingBuffer(capacity = 200) {
  const entries: DiagnosticEntry[] = [];
  const boundedCapacity = Math.max(1, Math.floor(capacity));

  return {
    record(entry: Omit<DiagnosticEntry, 'timestampEpochMs'> & { timestampEpochMs?: number }) {
      entries.push({
        ...entry,
        timestampEpochMs: entry.timestampEpochMs ?? Date.now(),
        message: redactDiagnosticMessage(entry.message),
        fields: redactDiagnosticFields(entry.fields)
      });
      while (entries.length > boundedCapacity) {
        entries.shift();
      }
    },
    export(): DiagnosticExport {
      return {
        generatedAtEpochMs: Date.now(),
        entries: entries.map((entry) => ({
          ...entry,
          fields: entry.fields ? { ...entry.fields } : undefined
        }))
      };
    }
  };
}
