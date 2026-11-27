export const CONTENT_CACHE = 'library-shards';
const RETIRED_CACHES = ['library-tables'];

export type ContentSource = 'memory' | 'cache' | 'network';

export interface Content {
  readonly bytes: Uint8Array<ArrayBuffer>;
  readonly source: ContentSource;
}

export interface ContentRef {
  readonly sha256: string;
  readonly bytes: number;
}

export type Fetcher = (input: string, init?: RequestInit) => Promise<Response>;

export const defaultFetcher: Fetcher = (input, init) => fetch(input, init);

let retired: Promise<void> | null = null;

function retireOldCaches(): Promise<void> {
  retired ??= Promise.all(RETIRED_CACHES.map((name) => caches.delete(name).catch(() => false))).then(() => undefined);
  return retired;
}

export async function openContentCache(): Promise<Cache | null> {
  try {
    if (typeof caches === 'undefined') {
      return null;
    }
    await retireOldCaches();
    return await caches.open(CONTENT_CACHE);
  } catch {
    return null;
  }
}

async function cached(cache: Cache | null, key: string): Promise<Uint8Array<ArrayBuffer> | null> {
  try {
    const hit = await cache?.match(key);
    return hit === undefined ? null : new Uint8Array(await hit.arrayBuffer());
  } catch {
    return null;
  }
}

async function store(cache: Cache | null, key: string, body: string | Uint8Array<ArrayBuffer>, type: string): Promise<void> {
  try {
    await cache?.put(key, new Response(body, { headers: { 'Content-Type': type } }));
  } catch {
    return;
  }
}

export async function sha256Hex(bytes: Uint8Array<ArrayBuffer>): Promise<string | null> {
  const subtle = globalThis.crypto?.subtle;
  if (subtle === undefined) {
    return null;
  }
  const digest = new Uint8Array(await subtle.digest('SHA-256', bytes));
  return [...digest].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

async function matches(ref: ContentRef, bytes: Uint8Array<ArrayBuffer>): Promise<boolean> {
  if (ref.bytes > 0 && bytes.length !== ref.bytes) {
    return false;
  }
  const digest = await sha256Hex(bytes);
  return digest === null || digest === ref.sha256;
}

export async function fetchContent(url: string, ref: ContentRef, fetcher: Fetcher = defaultFetcher, signal?: AbortSignal): Promise<Content> {
  const cache = await openContentCache();
  const hit = await cached(cache, url);
  if (hit !== null && (await matches(ref, hit))) {
    return { bytes: hit, source: 'cache' };
  }
  const response = await fetcher(url, signal === undefined ? {} : { signal });
  if (!response.ok) {
    throw new Error(`${url}: HTTP ${response.status}`);
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (!(await matches(ref, bytes))) {
    throw new Error(`${url} does not match the SHA-256 the collection names`);
  }
  await store(cache, url, bytes, 'application/octet-stream');
  return { bytes, source: 'network' };
}

export async function fetchFresh<T>(url: string, parse: (text: string) => T | null, fetcher: Fetcher = defaultFetcher): Promise<{ readonly text: string; readonly value: T } | null> {
  const cache = await openContentCache();
  try {
    const response = await fetcher(url, { cache: 'no-cache' });
    const text = response.ok ? await response.text() : '';
    const value = parse(text);
    if (value !== null) {
      await store(cache, url, text, 'application/json');
      return { text, value };
    }
  } catch {
    return fromCache(cache, url, parse);
  }
  return fromCache(cache, url, parse);
}

async function fromCache<T>(cache: Cache | null, url: string, parse: (text: string) => T | null): Promise<{ readonly text: string; readonly value: T } | null> {
  const bytes = await cached(cache, url);
  if (bytes === null) {
    return null;
  }
  const text = new TextDecoder().decode(bytes);
  const value = parse(text);
  return value === null ? null : { text, value };
}
