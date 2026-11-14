import { browserStorage, type KeyValue } from '../recent/storage';

export const REPO_API = 'https://api.github.com/repos/Sorte1/untwist';
export const STARS_KEY = 'untwist.githubStars';
export const STARS_TTL_MS = 6 * 60 * 60 * 1000;
export const RETRY_AFTER_MS = 60 * 60 * 1000;
const FETCH_TIMEOUT_MS = 8000;
const COMPACT_FROM = 1000;

interface CachedStars {
  readonly count: number | null;
  readonly at: number;
}

export type Cached = { readonly kind: 'count'; readonly count: number } | { readonly kind: 'wait' } | { readonly kind: 'none' };

const NONE: Cached = { kind: 'none' };

function isCount(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

export function cachedStars(raw: string | null, now: number): Cached {
  if (raw === null) {
    return NONE;
  }
  try {
    const value = JSON.parse(raw) as Partial<CachedStars> | null;
    const at = value?.at;
    if (typeof at !== 'number' || now < at) {
      return NONE;
    }
    if (isCount(value?.count)) {
      return now - at < STARS_TTL_MS ? { kind: 'count', count: value.count } : NONE;
    }
    return value?.count === null && now - at < RETRY_AFTER_MS ? { kind: 'wait' } : NONE;
  } catch {
    return NONE;
  }
}

export function starsLabel(count: number): string {
  if (count < COMPACT_FROM) {
    return String(count);
  }
  const thousands = count / 1000;
  return `${thousands < 10 ? Math.floor(thousands * 10) / 10 : Math.floor(thousands)}k`;
}

export async function fetchStars(fetcher: typeof fetch = fetch): Promise<number | null> {
  try {
    const response = await fetcher(REPO_API, { headers: { accept: 'application/vnd.github+json' }, credentials: 'omit', signal: AbortSignal.timeout(FETCH_TIMEOUT_MS) });
    if (!response.ok) {
      return null;
    }
    const body = (await response.json()) as { stargazers_count?: unknown } | null;
    return isCount(body?.stargazers_count) ? body.stargazers_count : null;
  } catch {
    return null;
  }
}

function readCache(store: KeyValue | null, now: number): Cached {
  try {
    return cachedStars(store?.getItem(STARS_KEY) ?? null, now);
  } catch {
    return NONE;
  }
}

function writeCache(store: KeyValue | null, count: number | null, now: number): void {
  try {
    store?.setItem(STARS_KEY, JSON.stringify({ count, at: now }));
  } catch {
    return;
  }
}

function paint(link: HTMLElement, badge: HTMLElement, count: number): void {
  const word = count === 1 ? 'star' : 'stars';
  badge.textContent = starsLabel(count);
  badge.hidden = false;
  link.setAttribute('aria-label', `untwist on GitHub, ${count.toLocaleString('en-US')} ${word}`);
  link.title = `untwist on GitHub · ${count.toLocaleString('en-US')} ${word}`;
}

export async function showStars(link: HTMLElement, badge: HTMLElement, store: KeyValue | null = browserStorage(), fetcher: typeof fetch = fetch): Promise<void> {
  const cached = readCache(store, Date.now());
  if (cached.kind === 'count') {
    paint(link, badge, cached.count);
  }
  if (cached.kind !== 'none' || !navigator.onLine) {
    return;
  }
  const count = await fetchStars(fetcher);
  writeCache(store, count, Date.now());
  if (count !== null) {
    paint(link, badge, count);
  }
}
