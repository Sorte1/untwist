import wasmUrl from './pkg/untwist_wasm_bg.wasm?url';
import RunWorker from './run.worker.ts?worker';
import type { LibraryTablesUsed, RunResult, WorkerReply } from './types';
import type { SuppliedWasm } from '../wasm/wasm';
import type { LibraryTables, LoadedTable } from '../libraries/tables';
import { TABLES_MARK } from '../progress';

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

interface Prepared {
  readonly tables: readonly LoadedTable[];
  readonly failed: readonly string[];
  readonly scanMs: number;
  readonly loadMs: number;
}

const NO_TABLES: Prepared = { tables: [], failed: [], scanMs: 0, loadMs: 0 };

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
  readonly #libraries: LibraryTables | null;
  #warm: Warm | null = null;
  #current: Current | null = null;
  #generation = 0;

  constructor(source: string | Promise<WebAssembly.Module> = wasmUrl, keepWarm = true, libraries: LibraryTables | null = null) {
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

  async run(source: string, flags: string, onStage: (name: string) => void = () => undefined, extras = REPORT_EXTRAS, wasm: readonly SuppliedWasm[] = []): Promise<RunResult> {
    const generation = this.#generation;
    const prepared = await this.#prepare(source, [], NO_TABLES, onStage, generation);
    const result = await this.#runTrapped(source, flags, onStage, extras, wasm, prepared.tables);
    const more = await this.#prepare(result.stdout, prepared.tables.map((table) => table.id).concat(prepared.failed), prepared, onStage, generation);
    if (more.tables.length === prepared.tables.length) {
      return { ...result, libraryTables: used(prepared, result, false) };
    }
    const rerun = await this.#runTrapped(source, flags, onStage, extras, wasm, more.tables);
    return { ...rerun, workerMs: result.workerMs + rerun.workerMs, libraryTables: used(more, rerun, true) };
  }

  async #prepare(text: string, skip: readonly string[], known: Prepared, onStage: (name: string) => void, generation: number): Promise<Prepared> {
    const libraries = this.#libraries;
    if (libraries === null) {
      return known;
    }
    const scanStarted = performance.now();
    const wanted = await libraries.wanted(text, skip).catch(() => []);
    this.#stillCurrent(generation);
    const scanMs = known.scanMs + performance.now() - scanStarted;
    if (wanted.length === 0) {
      return { ...known, scanMs };
    }
    onStage(`${TABLES_MARK}${wanted.map((table) => table.label).join(', ')}`);
    const loadStarted = performance.now();
    const fetched = await libraries.load(wanted);
    this.#stillCurrent(generation);
    return {
      tables: [...known.tables, ...fetched.tables],
      failed: [...known.failed, ...fetched.failed],
      scanMs,
      loadMs: known.loadMs + performance.now() - loadStarted,
    };
  }

  #stillCurrent(generation: number): void {
    if (generation !== this.#generation) {
      throw new Cancelled();
    }
  }

  async #runTrapped(source: string, flags: string, onStage: (name: string) => void, extras: string, wasm: readonly SuppliedWasm[], tables: readonly LoadedTable[]): Promise<RunResult> {
    const bytes = tables.map((table) => table.bytes);
    const first = await this.#runOnce(source, flags, false, onStage, extras, wasm, bytes);
    if (first.kind === 'done') {
      return { ...first.result, trap: null, workerMs: first.wallMs };
    }
    const refused = await this.#runOnce(source, flags, true, onStage, extras, wasm, bytes);
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

  async #runOnce(source: string, flags: string, refused: boolean, onStage: (name: string) => void, extras: string, wasm: readonly SuppliedWasm[], tables: readonly Uint8Array<ArrayBuffer>[]): Promise<Finished> {
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
              onStage(reply.name);
            } else if (reply.kind === 'done' || reply.kind === 'trap') {
              performance.mark('untwist:result-received');
              resolve({ ...reply, wallMs: performance.now() - posted });
            } else if (reply.kind === 'failed') {
              reject(new Error(reply.message));
            }
          };
          worker.onerror = (event) => reject(new Error(event.message));
          worker.postMessage({ kind: 'run', source, flags, refused, extras, wasm, tables });
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

function used(prepared: Prepared, result: RunResult, reran: boolean): LibraryTablesUsed {
  const loaded = new Set((result.tablesLoaded ?? '').split(','));
  return {
    fetched: prepared.tables.map((table) => table.id).filter((id) => loaded.has(id)),
    failed: [...prepared.failed, ...prepared.tables.map((table) => table.id).filter((id) => !loaded.has(id))],
    reran,
    scanMs: prepared.scanMs,
    loadMs: prepared.loadMs,
  };
}
