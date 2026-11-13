export const LIBRARY_NAMES_KEY = 'untwist.libraryNames';

type StorageLike = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>;

function storage(): StorageLike | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function hashSetsLibraries(hash: string): boolean {
  return /(?:^#?|&)libs=/.test(hash);
}

export function storedLibraryNames(store: StorageLike | null = storage()): boolean | null {
  try {
    const value = store?.getItem(LIBRARY_NAMES_KEY);
    return value === '1' ? true : value === '0' ? false : null;
  } catch {
    return null;
  }
}

export function storeLibraryNames(on: boolean, store: StorageLike | null = storage()): void {
  try {
    store?.setItem(LIBRARY_NAMES_KEY, on ? '1' : '0');
  } catch {
    return;
  }
}
