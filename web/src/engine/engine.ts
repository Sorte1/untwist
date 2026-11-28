import wasmUrl from './pkg/untwist_wasm_bg.wasm?url';
import RunWorker from './run.worker.ts?worker';
import type { RunResult, WorkerReply } from './types';
import type { SuppliedWasm } from '../wasm/wasm';
import type { CollectionFiles, FetchedShard, LibraryCollection } from '../libraries/collection';
import type { CollectionShard } from '../libraries/collection-format';
import { MARK, tablesOn, type StageMark } from '../progress';

export const REPORT_EXTRAS = 'refusals strings links';

export class Cancelled extends Error {
  constructor() {
    super('cancelled');
  }
}

type Finished = Extract<WorkerReply, { kind: 'done' | 'trap' }> & { readonly wallMs: number };

interface Warm {
  readonly worker: Worker;
  readonly ready: Promise<void>;
}

export type OnProgress = (mark: StageMark) => void;

const NO_PROGRESS: OnProgress = () => undefined;

interface Attempt {
  readonly flags: string;
  readonly extras: string;
  readonly wasm: readonly SuppliedWasm[];
  readonly onProgress: OnProgress;
  readonly libraries: CollectionFiles | null;
}

function marker(onProgress: OnProgress): (name: string) => void {
  return (name) => onProgress({ name, at: performance.now() });
}

interface Current {
  readonly worker: Worker;
  readonly reject: (error: Error) => void;
}

async function compile(url: string): Promise<WebAssembly.Module> {
  try {
    return await WebAssembly.compileStreaming(fetch(url));
  } catch {
    const response = await fetch(url);
    return WebAssembly.compile(await response.arrayBuffer());
  }
}

function awaitReady(worker: Worker, module: Promise<WebAssembly.Module>): Promise<void> {
  return new Promise((resolve, reject) => {
    worker.onmessage = (event: MessageEvent<WorkerReply>) => {
      if (event.data.kind === 'ready') {
        resolve();
      } else if (event.data.kind === 'failed') {
        reject(new Error(event.data.message));
      }
    };
    worker.onerror = (event) => reject(new Error(event.message));
    module.then((compiled) => worker.postMessage({ kind: 'init', module: compiled }), reject);
  });
}

export class Engine {
  readonly #module: Promise<WebAssembly.Module>;
  readonly #keepWarm: boolean;
  readonly #libraries: LibraryCollection | null;
  #warm: Warm | null = null;
  #current: Current | null = null;
  #generation = 0;

  constructor(source: string | Promise<WebAssembly.Module> = wasmUrl, keepWarm = true, libraries: LibraryCollection | null = null) {
    this.#module = typeof source === 'string' ? compile(source) : source;
    this.#keepWarm = keepWarm;
    this.#libraries = libraries;
  }

  get module(): Promise<WebAssembly.Module> {
    return this.#module;
  }

  ready(): Promise<void> {
    return this.#prewarm().ready;
  }

  get running(): boolean {
    return this.#current !== null;
  }

  async run(source: string, flags: string, onProgress: OnProgress = NO_PROGRESS, extras = REPORT_EXTRAS, wasm: readonly SuppliedWasm[] = []): Promise<RunResult> {
    const generation = this.#generation;
    const collection = tablesOn(flags) ? this.#libraries : null;
    const libraries = collection === null ? null : await collection.files().catch(() => null);
    this.#stillCurrent(generation);
    return this.#runTrapped(source, { flags, extras, wasm, onProgress, libraries });
  }

  #sendShards(worker: Worker, request: number, shards: readonly CollectionShard[]): void {
    const answer = (results: readonly (FetchedShard | null)[]) => {
      if (this.#current?.worker === worker) {
        worker.postMessage({ kind: 'shards', request, results });
      }
    };
    const collection = this.#libraries;
    if (collection === null) {
      answer(shards.map(() => null));
      return;
    }
    collection.shards(shards).then(answer, () => answer(shards.map(() => null)));
  }

  #stillCurrent(generation: number): void {
    if (generation !== this.#generation) {
      throw new Cancelled();
    }
  }

  async #runTrapped(source: string, attempt: Attempt): Promise<RunResult> {
    const first = await this.#runOnce(source, attempt, false);
    if (first.kind === 'done') {
      return { ...first.result, trap: null, workerMs: first.wallMs };
    }
    marker(attempt.onProgress)(MARK.retry);
    const refused = await this.#runOnce(source, attempt, true);
    if (refused.kind !== 'done') {
      throw new Error(`the refused re-run trapped too: ${refused.message}`);
    }
    return { ...refused.result, trap: first.trap, workerMs: first.wallMs + refused.wallMs };
  }

  cancel(): void {
    this.#generation += 1;
    const current = this.#current;
    if (current === null) {
      return;
    }
    this.#current = null;
    current.worker.terminate();
    current.reject(new Cancelled());
    this.#rewarm();
  }

  #rewarm(): void {
    if (this.#keepWarm) {
      this.#prewarm();
    }
  }

  #prewarm(): Warm {
    if (this.#warm === null) {
      const worker = new RunWorker();
      const ready = awaitReady(worker, this.#module);
      ready.catch(() => undefined);
      this.#warm = { worker, ready };
    }
    return this.#warm;
  }

  async #runOnce(source: string, attempt: Attempt, refused: boolean): Promise<Finished> {
    const { flags, extras, wasm, onProgress, libraries } = attempt;
    marker(onProgress)(MARK.start);
    const { worker, ready } = this.#prewarm();
    this.#warm = null;
    try {
      return await new Promise<Finished>((resolve, reject) => {
        this.#current = { worker, reject };
        const start = () => {
          if (this.#current?.worker !== worker) {
            return;
          }
          const posted = performance.now();
          worker.onmessage = (event: MessageEvent<WorkerReply>) => {
            const reply = event.data;
            if (reply.kind === 'stage') {
              onProgress({ name: reply.name, at: reply.at - performance.timeOrigin });
            } else if (reply.kind === 'fetch-shards') {
              this.#sendShards(worker, reply.request, reply.shards);
            } else if (reply.kind === 'done' || reply.kind === 'trap') {
              performance.mark('untwist:result-received');
              resolve({ ...reply, wallMs: performance.now() - posted });
            } else if (reply.kind === 'failed') {
              reject(new Error(reply.message));
            }
          };
          worker.onerror = (event) => reject(new Error(event.message));
          worker.postMessage({ kind: 'run', source, flags, refused, extras, wasm, libraries });
        };
        ready.then(start, reject);
      });
    } finally {
      worker.terminate();
      if (this.#current?.worker === worker) {
        this.#current = null;
      }
      this.#rewarm();
    }
  }
}
