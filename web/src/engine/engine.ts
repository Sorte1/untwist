import wasmUrl from './pkg/untwist_wasm_bg.wasm?url';
import RunWorker from './run.worker.ts?worker';
import type { LibraryTablesUsed, RunResult, WorkerReply } from './types';
import type { SuppliedWasm } from '../wasm/wasm';
import type { LibraryTables, LoadedTable, TableTiming } from '../libraries/tables';
import { librariesOn, MARK, tablesMark, type StageMark } from '../progress';
import { sizeLabel } from '../dom';

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
  readonly timings: readonly TableTiming[];
}

interface PrepareStep {
  readonly mark: (name: string) => void;
  readonly generation: number;
  readonly signal: AbortSignal;
  readonly name: string;
}

const NO_TABLES: Prepared = { tables: [], failed: [], scanMs: 0, loadMs: 0, timings: [] };

export type OnProgress = (mark: StageMark) => void;

const NO_PROGRESS: OnProgress = () => undefined;

interface Attempt {
  readonly flags: string;
  readonly extras: string;
  readonly wasm: readonly SuppliedWasm[];
  readonly onProgress: OnProgress;
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
  readonly #libraries: LibraryTables | null;
  #warm: Warm | null = null;
  #current: Current | null = null;
  #generation = 0;
  #abort = new AbortController();

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

  async run(source: string, flags: string, onProgress: OnProgress = NO_PROGRESS, extras = REPORT_EXTRAS, wasm: readonly SuppliedWasm[] = []): Promise<RunResult> {
    const generation = this.#generation;
    const signal = this.#abort.signal;
    const mark = marker(onProgress);
    const attempt: Attempt = { flags, extras, wasm, onProgress };
    const libraries = librariesOn(flags) ? this.#libraries : null;
    const prepared = await this.#prepare(libraries, source, [], NO_TABLES, { mark, generation, signal, name: MARK.scan });
    const result = await this.#runTrapped(source, attempt, prepared.tables);
    if (libraries === null) {
      return result;
    }
    const more = await this.#prepare(libraries, result.stdout, prepared.tables.map((table) => table.id).concat(prepared.failed), prepared, { mark, generation, signal, name: MARK.scanOutput });
    if (more.tables.length === prepared.tables.length) {
      return { ...result, libraryTables: used(prepared, result, false) };
    }
    mark(MARK.rerun);
    const rerun = await this.#runTrapped(source, attempt, more.tables);
    return { ...rerun, workerMs: result.workerMs + rerun.workerMs, libraryTables: used(more, rerun, true) };
  }

  async #prepare(libraries: LibraryTables | null, text: string, skip: readonly string[], known: Prepared, step: PrepareStep): Promise<Prepared> {
    if (libraries === null) {
      return known;
    }
    step.mark(step.name);
    const scanStarted = performance.now();
    const wanted = await libraries.wanted(text, skip, step.signal).catch(() => []);
    this.#stillCurrent(step.generation);
    const scanMs = known.scanMs + performance.now() - scanStarted;
    if (wanted.length === 0) {
      return { ...known, scanMs };
    }
    step.mark(tablesMark(wanted, sizeLabel));
    const loadStarted = performance.now();
    const fetched = await libraries.load(wanted, step.signal);
    this.#stillCurrent(step.generation);
    return {
      tables: [...known.tables, ...fetched.tables],
      failed: [...known.failed, ...fetched.failed],
      timings: [...known.timings, ...fetched.timings],
      scanMs,
      loadMs: known.loadMs + performance.now() - loadStarted,
    };
  }

  #stillCurrent(generation: number): void {
    if (generation !== this.#generation) {
      throw new Cancelled();
    }
  }

  async #runTrapped(source: string, attempt: Attempt, tables: readonly LoadedTable[]): Promise<RunResult> {
    const bytes = tables.map((table) => table.bytes);
    const first = await this.#runOnce(source, attempt, false, bytes);
    if (first.kind === 'done') {
      return { ...first.result, trap: null, workerMs: first.wallMs };
    }
    marker(attempt.onProgress)(MARK.retry);
    const refused = await this.#runOnce(source, attempt, true, bytes);
    if (refused.kind !== 'done') {
      throw new Error(`the refused re-run trapped too: ${refused.message}`);
    }
    return { ...refused.result, trap: first.trap, workerMs: first.wallMs + refused.wallMs };
  }

  cancel(): void {
    this.#generation += 1;
    this.#abort.abort();
    this.#abort = new AbortController();
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

  async #runOnce(source: string, attempt: Attempt, refused: boolean, tables: readonly Uint8Array<ArrayBuffer>[]): Promise<Finished> {
    const { flags, extras, wasm, onProgress } = attempt;
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
    timings: prepared.timings,
  };
}
