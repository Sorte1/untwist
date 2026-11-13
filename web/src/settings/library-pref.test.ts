import { describe, expect, it } from 'vitest';
import { hashSetsLibraries, LIBRARY_NAMES_KEY, storedLibraryNames, storeLibraryNames } from './library-pref';

function memory(): Storage {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => void values.set(key, value),
    removeItem: (key: string) => void values.delete(key),
    clear: () => values.clear(),
    key: () => null,
    length: 0,
  };
}

function broken(): Storage {
  const fail = () => {
    throw new Error('denied');
  };
  return { getItem: fail, setItem: fail, removeItem: fail, clear: fail, key: fail, length: 0 };
}

describe('library names preference', () => {
  it('is unset until the choice is stored, then reads it back', () => {
    const store = memory();
    expect(storedLibraryNames(store)).toBeNull();
    storeLibraryNames(true, store);
    expect(store.getItem(LIBRARY_NAMES_KEY)).toBe('1');
    expect(storedLibraryNames(store)).toBe(true);
    storeLibraryNames(false, store);
    expect(storedLibraryNames(store)).toBe(false);
  });

  it('survives storage that throws', () => {
    expect(storedLibraryNames(broken())).toBeNull();
    expect(() => storeLibraryNames(true, broken())).not.toThrow();
    expect(storedLibraryNames(null)).toBeNull();
  });

  it('knows when the link already says whether libraries are named', () => {
    expect(hashSetsLibraries('#libs=1')).toBe(true);
    expect(hashSetsLibraries('#rename=smart&libs=0')).toBe(true);
    expect(hashSetsLibraries('#rename=smart')).toBe(false);
    expect(hashSetsLibraries('#xlibs=1')).toBe(false);
    expect(hashSetsLibraries('')).toBe(false);
  });
});
