import type { CollectionFiles, FetchedShard } from '../libraries/collection';
import { shardOwner, type CollectionPackage, type CollectionShard } from '../libraries/collection-format';
import { sizeLabel } from '../dom';
import { MARK, tablesMark } from '../progress';
import type { LibraryTablesUsed, TableTiming } from './types';

export interface LibraryApi {
  readonly attach: (index: string, filters: Uint8Array) => string;
  readonly wanted: (source: string) => string;
  readonly load: (bytes: Uint8Array) => string;
}

interface Wanted {
  readonly pkg: CollectionPackage;
  readonly shard: CollectionShard;
}

export type ShardFetcher = (shards: readonly CollectionShard[]) => Promise<readonly (FetchedShard | null)[]>;

interface Outcome {
  readonly wanted: Wanted;
  readonly content: FetchedShard | null;
}

function shasOf(json: string): string[] {
  try {
    const parsed: unknown = JSON.parse(json);
    return Array.isArray(parsed) ? parsed.filter((sha): sha is string => typeof sha === 'string') : [];
  } catch {
    return [];
  }
}

export class LibraryStage {
  readonly #api: LibraryApi;
  readonly #files: CollectionFiles;
  readonly #stage: (name: string) => void;
  readonly #fetch: ShardFetcher;
  readonly #error: string;
  readonly #tried = new Set<string>();
  readonly #loaded = new Set<string>();
  readonly #failed = new Set<string>();
  readonly #timings = new Map<string, TableTiming>();
  #scanMs = 0;
  #loadMs = 0;

  constructor(api: LibraryApi, files: CollectionFiles, stage: (name: string) => void, fetcher: ShardFetcher) {
    this.#api = api;
    this.#files = files;
    this.#stage = stage;
    this.#fetch = fetcher;
    this.#error = api.attach(files.text, files.filters ?? new Uint8Array());
  }

  async take(texts: readonly string[], mark: string): Promise<number> {
    this.#stage(mark);
    const scanStarted = performance.now();
    const shas = new Set(texts.filter((text) => text !== '').flatMap((text) => shasOf(this.#api.wanted(text))));
    this.#scanMs += performance.now() - scanStarted;
    const wanted = [...shas]
      .filter((sha) => !this.#tried.has(sha))
      .map((sha) => shardOwner(this.#files.index, sha))
      .filter((owner): owner is Wanted => owner !== null);
    if (wanted.length === 0) {
      return 0;
    }
    wanted.forEach(({ shard }) => this.#tried.add(shard.sha256));
    this.#stage(tablesMark(wanted.map(({ pkg, shard }) => ({ label: pkg.label, bytes: shard.bytes })), sizeLabel));
    const loadStarted = performance.now();
    const fetched = await this.#fetch(wanted.map(({ shard }) => shard)).catch(() => []);
    const outcomes = wanted.map((entry, at) => ({ wanted: entry, content: fetched[at] ?? null }));
    this.#stage(MARK.loadTables);
    const loaded = outcomes.filter((outcome) => this.#loadOne(outcome)).length;
    this.#loadMs += performance.now() - loadStarted;
    return loaded;
  }

  #loadOne(outcome: Outcome): boolean {
    const { pkg, shard } = outcome.wanted;
    const id = outcome.content === null ? '' : this.#api.load(outcome.content.bytes);
    const previous = this.#timings.get(pkg.id);
    this.#timings.set(pkg.id, {
      id: pkg.id,
      bytes: (previous?.bytes ?? 0) + shard.bytes,
      ms: (previous?.ms ?? 0) + (outcome.content?.ms ?? 0),
      source: previous?.source ?? outcome.content?.source ?? null,
    });
    if (id === '') {
      this.#failed.add(pkg.id);
      return false;
    }
    this.#loaded.add(pkg.id);
    return true;
  }

  used(reran: boolean): LibraryTablesUsed {
    return {
      collection: this.#files.index.version,
      error: this.#error,
      fetched: [...this.#loaded].filter((id) => !this.#failed.has(id)),
      failed: [...this.#failed],
      reran,
      scanMs: this.#scanMs,
      loadMs: this.#loadMs,
      timings: [...this.#timings.values()],
    };
  }
}
