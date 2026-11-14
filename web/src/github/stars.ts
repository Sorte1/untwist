import { browserStorage, type KeyValue } from '../recent/storage';

export const REPO_API = 'https://api.github.com/repos/Sorte1/untwist';
export const STARS_KEY = 'untwist.githubStars';
export const STARS_TTL_MS = 6 * 60 * 60 * 1000;
const FETCH_TIMEOUT_MS = 8000;
const COMPACT_FROM = 1000;

interface CachedStars {
  readonly count: number;
  readonly at: number;
}

function isCount(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

export function cachedStars(raw: string | null, now: number): number | null {
  if (raw === null) {
    return null;
  }
  try {
    const value = JSON.parse(raw) as Partial<CachedStars> | null;
    const fresh = isCount(value?.count) && typeof value?.at === 'number' && now >= value.at && now - value.at < STARS_TTL_MS;
    return fresh ? (value.count as number) : null;
  } catch {
    return null;
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

function readCache(store: KeyValue | null, now: number): number | null {
  try {
    return cachedStars(store?.getItem(STARS_KEY) ?? null, now);
  } catch {
    return null;
  }
}

function writeCache(store: KeyValue | null, count: number, now: number): void {
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
  if (cached !== null) {
    paint(link, badge, cached);
    return;
  }
  if (!navigator.onLine) {
    return;
  }
  const count = await fetchStars(fetcher);
  if (count !== null) {
    writeCache(store, count, Date.now());
    paint(link, badge, count);
  }
}
