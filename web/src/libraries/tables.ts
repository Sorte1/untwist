import ScanWorker from './scan.worker.ts?worker';
import { parseIndex, INDEX_FILE, TABLES_DIR, type IndexedTable, type TableIndex } from './index-format';
import type { KeyedTable } from './scan';

export const TABLE_CACHE = 'library-tables';
const GZIP_MAGIC = [0x1f, 0x8b] as const;

export interface LoadedTable {
  readonly id: string;
  readonly bytes: Uint8Array<ArrayBuffer>;
}

export type TableSource = 'memory' | 'cache' | 'network';

export interface TableTiming {
  readonly id: string;
  readonly bytes: number;
  readonly ms: number;
  readonly source: TableSource | null;
}

export interface TableFetch {
  readonly tables: readonly LoadedTable[];
  readonly failed: readonly string[];
  readonly timings: readonly TableTiming[];
}

interface Fetched {
  readonly bytes: Uint8Array<ArrayBuffer>;
  readonly source: TableSource;
}

export interface ScanResult {
  readonly ids: readonly string[];
  readonly ms: number;
}

export type Scanner = (text: string, tables: readonly KeyedTable[], signal?: AbortSignal) => Promise<ScanResult>;

function aborted(): Error {
  return new DOMException('cancelled', 'AbortError');
}

function workerScan(text: string, tables: readonly KeyedTable[], signal?: AbortSignal): Promise<ScanResult> {
  if (signal?.aborted === true) {
    return Promise.reject(aborted());
  }
  const worker = new ScanWorker();
  const result = new Promise<ScanResult>((resolve, reject) => {
    worker.onmessage = (event: MessageEvent<ScanResult>) => resolve(event.data);
    worker.onerror = (event) => reject(new Error(event.message));
    signal?.addEventListener('abort', () => reject(aborted()), { once: true });
    worker.postMessage({ text, tables });
  });
  const done = () => worker.terminate();
  result.then(done, done);
  return result;
}

function isGzip(bytes: Uint8Array): boolean {
  return bytes.length > GZIP_MAGIC.length && bytes[0] === GZIP_MAGIC[0] && bytes[1] === GZIP_MAGIC[1];
}

async function sha256(bytes: Uint8Array<ArrayBuffer>): Promise<string | null> {
  const subtle = globalThis.crypto?.subtle;
  if (subtle === undefined) {
    return null;
  }
  const digest = new Uint8Array(await subtle.digest('SHA-256', bytes));
  return [...digest].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

async function openCache(): Promise<Cache | null> {
  try {
    return typeof caches === 'undefined' ? null : await caches.open(TABLE_CACHE);
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

async function remember(cache: Cache | null, table: IndexedTable, key: string, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
  if (cache === null) {
    return;
  }
  try {
    for (const old of await cache.keys()) {
      const url = new URL(old.url);
      if (url.pathname.endsWith(`/${table.file}`) && url.href !== key) {
        await cache.delete(old);
      }
    }
  } catch {
    return;
  }
  await store(cache, key, bytes, 'application/octet-stream');
}

async function store(cache: Cache | null, key: string, body: string | Uint8Array<ArrayBuffer>, type: string): Promise<void> {
  try {
    await cache?.put(key, new Response(body, { headers: { 'Content-Type': type } }));
  } catch {
    return;
  }
}

export class LibraryTables {
  readonly #base: URL;
  readonly #scan: Scanner;
  readonly #fetch: typeof fetch;
  #index: Promise<TableIndex | null> | null = null;
  readonly #bytes = new Map<string, Promise<Uint8Array<ArrayBuffer>>>();

  constructor(base: URL = new URL(`${TABLES_DIR}/`, document.baseURI), scan: Scanner = workerScan, fetcher: typeof fetch = (input, init) => fetch(input, init)) {
    this.#base = base;
    this.#scan = scan;
    this.#fetch = fetcher;
  }

  index(): Promise<TableIndex | null> {
    if (this.#index === null) {
      const loading = this.#loadIndex();
      this.#index = loading;
      loading.then((index) => {
        if (index === null && this.#index === loading) {
          this.#index = null;
        }
      }, () => undefined);
    }
    return this.#index;
  }

  async #loadIndex(): Promise<TableIndex | null> {
    const url = new URL(INDEX_FILE, this.#base).href;
    const cache = await openCache();
    try {
      const response = await this.#fetch(url, { cache: 'no-cache' });
      const text = response.ok ? await response.text() : '';
      const index = parseIndex(text);
      if (index !== null) {
        await store(cache, url, text, 'application/json');
        return index;
      }
    } catch {
      return this.#cachedIndex(cache, url);
    }
    return this.#cachedIndex(cache, url);
  }

  async #cachedIndex(cache: Cache | null, url: string): Promise<TableIndex | null> {
    const bytes = await cached(cache, url);
    return bytes === null ? null : parseIndex(new TextDecoder().decode(bytes));
  }

  async wanted(text: string, skip: readonly string[] = [], signal?: AbortSignal): Promise<readonly IndexedTable[]> {
    const index = await this.index();
    const candidates = index?.tables.filter((table) => !skip.includes(table.id)) ?? [];
    if (candidates.length === 0 || text === '') {
      return [];
    }
    const { ids } = await this.#scan(text, candidates.map((table) => ({ id: table.id, keys: table.keys })), signal);
    return candidates.filter((table) => ids.includes(table.id));
  }

  async load(tables: readonly IndexedTable[], signal?: AbortSignal): Promise<TableFetch> {
    const settled = await Promise.allSettled(tables.map((table) => this.#timed(table, signal)));
    const loaded: LoadedTable[] = [];
    const failed: string[] = [];
    const timings: TableTiming[] = [];
    settled.forEach((outcome, at) => {
      const table = tables[at];
      const id = table?.id ?? '';
      if (outcome.status === 'fulfilled') {
        loaded.push({ id, bytes: outcome.value.bytes });
        timings.push({ id, bytes: table?.bytes ?? 0, ms: outcome.value.ms, source: outcome.value.source });
      } else {
        failed.push(id);
        timings.push({ id, bytes: table?.bytes ?? 0, ms: 0, source: null });
      }
    });
    return { tables: loaded, failed, timings };
  }

  async #timed(table: IndexedTable, signal?: AbortSignal): Promise<Fetched & { readonly ms: number }> {
    const started = performance.now();
    const fetched = await this.#bytesOf(table, signal);
    return { ...fetched, ms: performance.now() - started };
  }

  async #bytesOf(table: IndexedTable, signal?: AbortSignal): Promise<Fetched> {
    const known = this.#bytes.get(table.id);
    if (known !== undefined) {
      return { bytes: await known, source: 'memory' };
    }
    const downloading = this.#download(table, signal);
    const loading = downloading.then((fetched) => fetched.bytes);
    this.#bytes.set(table.id, loading);
    loading.catch(() => {
      if (this.#bytes.get(table.id) === loading) {
        this.#bytes.delete(table.id);
      }
    });
    return downloading;
  }

  async #download(table: IndexedTable, signal?: AbortSignal): Promise<Fetched> {
    const url = new URL(table.file, this.#base);
    url.searchParams.set('v', table.sha256.slice(0, 16));
    const key = url.href;
    const cache = await openCache();
    const hit = await cached(cache, key);
    if (hit !== null && (await this.#valid(table, hit))) {
      return { bytes: hit, source: 'cache' };
    }
    const response = await this.#fetch(url, signal === undefined ? undefined : { signal });
    if (!response.ok) {
      throw new Error(`${table.file}: HTTP ${response.status}`);
    }
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (!(await this.#valid(table, bytes))) {
      throw new Error(`${table.file} does not match its index entry`);
    }
    await remember(cache, table, key, bytes);
    return { bytes, source: 'network' };
  }

  async #valid(table: IndexedTable, bytes: Uint8Array<ArrayBuffer>): Promise<boolean> {
    if (bytes.length !== table.bytes || !isGzip(bytes)) {
      return false;
    }
    const digest = await sha256(bytes);
    return digest === null || digest === table.sha256;
  }
}
