import { describe, expect, it } from 'vitest';
import { StackIconCache } from '../../src/lib/stackIconCache';

const bytes = (key: string, value: string | null) => 2 * (key.length + (value?.length ?? 0));

describe('production StackIconCache direct byte/entry LRU contract', () => {
  it('distinguishes a retained negative result from an absent key', () => {
    const cache = new StackIconCache();
    expect(cache.size).toBe(0); expect(cache.retainedBytes).toBe(0);
    expect(cache.get('missing')).toBeUndefined(); expect(cache.has('missing')).toBe(false);
    expect(cache.set('negative', null)).toBe(cache);
    expect(cache.has('negative')).toBe(true); expect(cache.get('negative')).toBeNull();
    expect(cache.size).toBe(1); expect(cache.retainedBytes).toBe(bytes('negative', null));
  });

  it('replacement subtracts prior key/value charge without growing count or evicting unrelated entries', () => {
    const cache = new StackIconCache(2, 100);
    cache.set('a', '12345678').set('b', '123');
    expect(cache.retainedBytes).toBe(26);
    cache.set('a', 'x');
    expect(cache.size).toBe(2); expect(cache.retainedBytes).toBe(12);
    expect(cache.get('b')).toBe('123'); expect(cache.get('a')).toBe('x');
    cache.set('a', null);
    expect(cache.size).toBe(2); expect(cache.retainedBytes).toBe(10);
    cache.set('a', '12345');
    expect(cache.retainedBytes).toBe(20);
  });

  it('get promotes positive and negative entries while has and missing lookups do not change recency', () => {
    const cache = new StackIconCache(2, 100);
    cache.set('a', 'A').set('b', null);
    expect(cache.get('a')).toBe('A'); cache.set('c', 'C');
    expect(cache.has('b')).toBe(false); expect(cache.has('a')).toBe(true);
    expect(cache.has('a')).toBe(true); expect(cache.get('missing')).toBeUndefined();
    cache.set('d', null);
    expect(cache.has('a')).toBe(false); expect(cache.has('c')).toBe(true);
    expect(cache.get('d')).toBeNull(); cache.set('e', 'E');
    expect(cache.has('c')).toBe(false); expect(cache.has('d')).toBe(true);
    expect(cache.size).toBe(2); expect(cache.retainedBytes).toBe(bytes('d', null) + bytes('e', 'E'));
  });

  it('replacement becomes most recent under count eviction', () => {
    const cache = new StackIconCache(2, 100);
    cache.set('a', 'A').set('b', 'B').set('a', 'new').set('c', 'C');
    expect(cache.has('b')).toBe(false); expect(cache.get('a')).toBe('new'); expect(cache.get('c')).toBe('C');
    expect(cache.retainedBytes).toBe(12);
  });

  it('accepts exact byte boundary and evicts oldest until both budgets fit', () => {
    const cache = new StackIconCache(8, 12);
    cache.set('a', '12').set('b', '34');
    expect(cache.size).toBe(2); expect(cache.retainedBytes).toBe(12);
    cache.set('c', '12345');
    expect(cache.size).toBe(1); expect(cache.retainedBytes).toBe(12);
    expect(cache.has('a')).toBe(false); expect(cache.has('b')).toBe(false); expect(cache.get('c')).toBe('12345');
  });

  it('growing replacement removes its old charge then evicts only necessary older entries', () => {
    const cache = new StackIconCache(3, 20);
    cache.set('a', '1').set('b', '2').set('c', '3');
    cache.set('b', '12345678');
    expect(cache.has('a')).toBe(false); expect(cache.has('c')).toBe(false);
    expect(cache.get('b')).toBe('12345678'); expect(cache.retainedBytes).toBe(18); expect(cache.size).toBe(1);
  });

  it('oversized insertion does not evict unrelated entries; oversized replacement retires old value without storing new payload', () => {
    const cache = new StackIconCache(3, 20);
    cache.set('a', '1').set('b', null);
    cache.set('oversize', 'X'.repeat(20));
    expect(cache.size).toBe(2); expect(cache.retainedBytes).toBe(6);
    expect(cache.get('a')).toBe('1'); expect(cache.get('b')).toBeNull(); expect(cache.has('oversize')).toBe(false);
    cache.set('a', 'X'.repeat(20));
    expect(cache.has('a')).toBe(false); expect(cache.has('b')).toBe(true);
    expect(cache.size).toBe(1); expect(cache.retainedBytes).toBe(2);
  });

  it('charges keys for nulls and UTF-16 code units for non-ASCII strings', () => {
    const cache = new StackIconCache(20, 100);
    cache.set('📁', 'é');
    expect(cache.retainedBytes).toBe(6);
    cache.set('negative-path', null);
    expect(cache.retainedBytes).toBe(6 + 26);
    const small = new StackIconCache(20, 10);
    small.set('too-long-key', null);
    expect(small.size).toBe(0); expect(small.retainedBytes).toBe(0);
  });

  it.each([0, -1])('disabled entry budget %i retains nothing', (maxEntries) => {
    const cache = new StackIconCache(maxEntries, 100);
    cache.set('a', 'A').set('b', null);
    expect(cache.size).toBe(0); expect(cache.retainedBytes).toBe(0);
  });

  it('default negative-result churn stays at 256 entries and exact remaining key charge', () => {
    const cache = new StackIconCache();
    let expected = 0;
    for (let i = 0; i < 10000; i++) cache.set(`c:\\fixture\\${i}.exe`, null);
    for (let i = 10000 - 256; i < 10000; i++) { const key = `c:\\fixture\\${i}.exe`; expect(cache.get(key)).toBeNull(); expected += bytes(key, null); }
    expect(cache.size).toBe(256); expect(cache.retainedBytes).toBe(expected);
    expect(cache.has('c:\\fixture\\0.exe')).toBe(false);
    console.info('ram-icon-cache-negative-churn', JSON.stringify({ operations: 10000, retainedEntries: cache.size, retainedBytes: cache.retainedBytes }));
  });

  it('default large-icon churn stays below 2 MiB independently of the entry-count cap', () => {
    const cache = new StackIconCache();
    for (let i = 0; i < 1000; i++) {
      cache.set(`c:\\fixture\\${i}.exe`, `data:image/png;base64,${String(i).padStart(4, '0')}${'A'.repeat(8192)}`);
      expect(cache.retainedBytes).toBeLessThanOrEqual(2 * 1024 * 1024);
      expect(cache.size).toBeLessThanOrEqual(256);
    }
    expect(cache.size).toBeLessThan(256);
    expect(cache.has('c:\\fixture\\999.exe')).toBe(true);
    expect(cache.has('c:\\fixture\\0.exe')).toBe(false);
    console.info('ram-icon-cache-large-churn', JSON.stringify({ operations: 1000, retainedEntries: cache.size, retainedBytes: cache.retainedBytes }));
  });

  it('clear releases all logical charge and permits reuse without a negative counter', () => {
    const cache = new StackIconCache(2, 100);
    cache.set('a', 'A').set('b', null); cache.clear(); cache.clear();
    expect(cache.size).toBe(0); expect(cache.retainedBytes).toBe(0); expect(cache.has('a')).toBe(false); expect(cache.get('b')).toBeUndefined();
    cache.set('c', 'CCC'); expect(cache.size).toBe(1); expect(cache.retainedBytes).toBe(8);
    cache.set('c', null); expect(cache.retainedBytes).toBe(2);
    cache.clear(); expect(cache.retainedBytes).toBe(0);
  });
});
