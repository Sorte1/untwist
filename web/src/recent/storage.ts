import { parseRecent, type RecentEntry } from './recent';

export const RECENT_KEY = 'untwist:recent:v1';

const PROBE_KEY = 'untwist:probe';
const QUOTA_ERRORS: readonly string[] = ['QuotaExceededError', 'NS_ERROR_DOM_QUOTA_REACHED'];

export interface KeyValue {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export function usable(store: KeyValue | null): KeyValue | null {
  if (store === null) {
    return null;
  }
  try {
    store.setItem(PROBE_KEY, '1');
    store.removeItem(PROBE_KEY);
    return store;
  } catch {
    return null;
  }
}

export function browserStorage(): KeyValue | null {
  try {
    return usable(window.localStorage);
  } catch {
    return null;
  }
}

export function loadRecent(store: KeyValue | null): RecentEntry[] {
  try {
    return parseRecent(store?.getItem(RECENT_KEY) ?? null);
  } catch {
    return [];
  }
}

export function saveRecent(store: KeyValue | null, list: readonly RecentEntry[]): readonly RecentEntry[] | null {
  if (store === null) {
    return null;
  }
  for (let kept = list.length; kept > 0; kept -= 1) {
    const fitted = list.slice(0, kept);
    try {
      store.setItem(RECENT_KEY, JSON.stringify(fitted));
      return fitted;
    } catch (error) {
      if (!(error instanceof Error && QUOTA_ERRORS.includes(error.name))) {
        return null;
      }
    }
  }
  return null;
}

export function clearRecent(store: KeyValue | null): boolean {
  if (store === null) {
    return false;
  }
  try {
    store.removeItem(RECENT_KEY);
    return true;
  } catch {
    return false;
  }
}
