import { describe, expect, it } from 'vitest';
import type { RecentEntry } from './recent';
import { clearRecent, loadRecent, RECENT_KEY, saveRecent, usable, type KeyValue } from './storage';

function memory(quota = Infinity): KeyValue & { readonly items: Map<string, string> } {
  const items = new Map<string, string>();
  return {
    items,
    getItem: (key) => items.get(key) ?? null,
    setItem: (key, value) => {
      if (value.length > quota) {
        throw new DOMException('full', 'QuotaExceededError');
      }
      items.set(key, value);
    },
    removeItem: (key) => {
      items.delete(key);
    },
  };
}

const broken: KeyValue = {
  getItem: () => {
    throw new DOMException('denied', 'SecurityError');
  },
  setItem: () => {
    throw new DOMException('denied', 'SecurityError');
  },
  removeItem: () => {
    throw new DOMException('denied', 'SecurityError');
  },
};

function entry(text: string, time: number): RecentEntry {
  return { name: 'a.js', text, bytes: text.length, time, hash: '' };
}

describe('recent storage', () => {
  it('saves and loads the list under one key', () => {
    const store = memory();
    const list = [entry('b', 2), entry('a', 1)];
    expect(saveRecent(store, list)).toEqual(list);
    expect([...store.items.keys()]).toEqual([RECENT_KEY]);
    expect(loadRecent(store)).toEqual(list);
  });

  it('drops the oldest runs until the list fits the quota', () => {
    const store = memory(JSON.stringify([entry('c'.repeat(100), 3), entry('b'.repeat(100), 2)]).length);
    const saved = saveRecent(store, [entry('c'.repeat(100), 3), entry('b'.repeat(100), 2), entry('a'.repeat(100), 1)]);
    expect(saved?.map((item) => item.time)).toEqual([3, 2]);
    expect(loadRecent(store).map((item) => item.time)).toEqual([3, 2]);
  });

  it('gives null when not even one run fits', () => {
    expect(saveRecent(memory(10), [entry('a'.repeat(100), 1)])).toBeNull();
  });

  it('works without storage or with storage that throws', () => {
    for (const store of [null, broken]) {
      expect(loadRecent(store)).toEqual([]);
      expect(saveRecent(store, [entry('a', 1)])).toBeNull();
      expect(clearRecent(store)).toBe(false);
    }
  });

  it('tells usable storage from storage that refuses writes', () => {
    const store = memory();
    expect(usable(store)).toBe(store);
    expect(store.items.size).toBe(0);
    expect(usable(broken)).toBeNull();
    expect(usable(null)).toBeNull();
  });

  it('clears the list', () => {
    const store = memory();
    saveRecent(store, [entry('a', 1)]);
    expect(clearRecent(store)).toBe(true);
    expect(loadRecent(store)).toEqual([]);
  });
});
