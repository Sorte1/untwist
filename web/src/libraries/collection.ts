import { COLLECTION_FILE, LIBRARIES_DIR, parseCollection, type CollectionIndex, type CollectionShard } from './collection-format';
import { defaultFetcher, fetchContent, fetchFresh, openContentCache, type ContentSource, type Fetcher } from './content';

export interface CollectionFiles {
  readonly base: string;
  readonly text: string;
  readonly index: CollectionIndex;
  readonly filters: Uint8Array<ArrayBuffer> | null;
}

export interface FetchedShard {
  readonly bytes: Uint8Array<ArrayBuffer>;
  readonly source: ContentSource;
  readonly ms: number;
}

export function collectionUrls(files: Pick<CollectionFiles, 'base' | 'index'>): string[] {
  const filters = files.index.filters === undefined ? [] : [files.index.filters.file];
  return [...filters, ...files.index.packages.flatMap((pkg) => pkg.shards.map((shard) => shard.file))].map((file) => new URL(file, files.base).href);
}

async function prune(keep: readonly string[], base: string): Promise<void> {
  const cache = await openContentCache();
  if (cache === null) {
    return;
  }
  const wanted = new Set(keep);
  const collection = new URL(COLLECTION_FILE, base).href;
  try {
    for (const request of await cache.keys()) {
      if (request.url !== collection && !wanted.has(request.url)) {
        await cache.delete(request);
      }
    }
  } catch {
    return;
  }
}

export class LibraryCollection {
  readonly #base: string;
  readonly #fetch: Fetcher;
  #files: Promise<CollectionFiles | null> | null = null;
  readonly #shards = new Map<string, Promise<Uint8Array<ArrayBuffer>>>();

  constructor(base: string = new URL(`${LIBRARIES_DIR}/`, document.baseURI).href, fetcher: Fetcher = defaultFetcher) {
    this.#base = base;
    this.#fetch = fetcher;
  }

  files(): Promise<CollectionFiles | null> {
    if (this.#files === null) {
      const loading = this.#load();
      this.#files = loading;
      loading.then(
        (files) => {
          if (files === null && this.#files === loading) {
            this.#files = null;
          }
        },
        () => {
          if (this.#files === loading) {
            this.#files = null;
          }
        },
      );
    }
    return this.#files;
  }

  async index(): Promise<CollectionIndex | null> {
    return (await this.files())?.index ?? null;
  }

  shards(shards: readonly CollectionShard[]): Promise<(FetchedShard | null)[]> {
    return Promise.all(shards.map((shard) => this.#shard(shard).catch(() => null)));
  }

  async #shard(shard: CollectionShard): Promise<FetchedShard> {
    const started = performance.now();
    const known = this.#shards.get(shard.sha256);
    if (known !== undefined) {
      return { bytes: await known, source: 'memory', ms: performance.now() - started };
    }
    const downloading = fetchContent(new URL(shard.file, this.#base).href, shard, this.#fetch);
    const bytes = downloading.then((content) => content.bytes);
    this.#shards.set(shard.sha256, bytes);
    bytes.catch(() => {
      if (this.#shards.get(shard.sha256) === bytes) {
        this.#shards.delete(shard.sha256);
      }
    });
    const content = await downloading;
    return { bytes: content.bytes, source: content.source, ms: performance.now() - started };
  }

  async #load(): Promise<CollectionFiles | null> {
    const fresh = await fetchFresh(new URL(COLLECTION_FILE, this.#base).href, parseCollection, this.#fetch);
    if (fresh === null) {
      return null;
    }
    const index = fresh.value;
    const filters = index.filters === undefined ? null : await fetchContent(new URL(index.filters.file, this.#base).href, index.filters, this.#fetch).then((content) => content.bytes, () => null);
    if (index.filters !== undefined && filters === null) {
      return null;
    }
    const files = { base: this.#base, text: fresh.text, index, filters };
    void prune(collectionUrls(files), this.#base);
    return files;
  }
}
