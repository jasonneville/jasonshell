type SequenceRange = { first: number; last: number };

/** Lossless string-key deduplication, compacting contiguous safe integers only. */
export class TerminalSequenceTracker {
  private readonly rangesByPrefix = new Map<string, SequenceRange[]>();
  private readonly fallbackKeys = new Set<string>();

  accept(sessionId: string, stream: string | null | undefined, sequence: unknown): boolean {
    if (typeof sequence !== 'number') return true;
    const prefix = `${sessionId}:${stream ?? 'stdout'}:`;
    if (!Number.isSafeInteger(sequence)) {
      const key = `${prefix}${sequence}`;
      if (this.fallbackKeys.has(key)) return false;
      this.fallbackKeys.add(key);
      return true;
    }

    // Mutate private interval storage deliberately: never copy historical keys per byte/chunk.
    let ranges = this.rangesByPrefix.get(prefix);
    if (!ranges) {
      ranges = [];
      this.rangesByPrefix.set(prefix, ranges);
    }
    let low = 0;
    let high = ranges.length;
    while (low < high) {
      const middle = (low + high) >>> 1;
      if (ranges[middle].last < sequence) low = middle + 1;
      else high = middle;
    }
    const next = ranges[low];
    if (next && next.first <= sequence) return false;
    const previous = ranges[low - 1];
    const joinsPrevious = previous && sequence - previous.last === 1;
    const joinsNext = next && next.first - sequence === 1;
    if (joinsPrevious && joinsNext) {
      previous.last = next.last;
      ranges.splice(low, 1);
    } else if (joinsPrevious) {
      previous.last = sequence;
    } else if (joinsNext) {
      next.first = sequence;
    } else {
      ranges.splice(low, 0, { first: sequence, last: sequence });
    }
    return true;
  }

  stats(): { rangeCount: number; fallbackKeyCount: number } {
    let rangeCount = 0;
    for (const ranges of this.rangesByPrefix.values()) rangeCount += ranges.length;
    return { rangeCount, fallbackKeyCount: this.fallbackKeys.size };
  }
}
