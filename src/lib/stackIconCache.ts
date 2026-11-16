const MAX_ICON_CACHE_ENTRIES = 256;
const MAX_ICON_CACHE_BYTES = 2 * 1024 * 1024;

/** LRU budget estimates UTF-16 key/value storage, not engine heap usage. */
export class StackIconCache {
  private readonly entries = new Map<string, string | null>();
  private bytes = 0;

  constructor(
    private readonly maxEntries = MAX_ICON_CACHE_ENTRIES,
    private readonly maxBytes = MAX_ICON_CACHE_BYTES
  ) {}

  get size() { return this.entries.size; }
  get retainedBytes() { return this.bytes; }

  has(key: string) { return this.entries.has(key); }

  get(key: string): string | null | undefined {
    const value = this.entries.get(key);
    if (value !== undefined) {
      this.entries.delete(key);
      this.entries.set(key, value);
    }
    return value;
  }

  set(key: string, value: string | null): this {
    this.remove(key);
    const cost = this.stringBytes(key, value);
    // Oversized images remain row-owned; caching must never prevent display.
    if (cost > this.maxBytes || this.maxEntries <= 0) return this;
    while (this.size && (this.size >= this.maxEntries || this.bytes + cost > this.maxBytes)) {
      const oldest = this.entries.keys().next().value;
      if (oldest === undefined) break;
      this.remove(oldest);
    }
    this.entries.set(key, value);
    this.bytes += cost;
    return this;
  }

  clear() {
    this.entries.clear();
    this.bytes = 0;
  }

  private remove(key: string) {
    const value = this.entries.get(key);
    if (value === undefined) return;
    this.bytes -= this.stringBytes(key, value);
    this.entries.delete(key);
  }

  private stringBytes(key: string, value: string | null) {
    return 2 * (key.length + (value?.length ?? 0));
  }
}
