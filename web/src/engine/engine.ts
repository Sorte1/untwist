import wasmUrl from './pkg/untwist_wasm_bg.wasm?url';
import RunWorker from './run.worker.ts?worker';
import type { RunResult, WorkerReply } from './types';

export const REPORT_EXTRAS = 'refusals strings';

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
  #warm: Warm | null = null;
  #current: Current | null = null;

  constructor(source: string | Promise<WebAssembly.Module> = wasmUrl, keepWarm = true) {
    this.#module = typeof source === 'string' ? compile(source) : source;
    this.#keepWarm = keepWarm;
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

  async run(source: string, flags: string, onStage: (name: string) => void = () => undefined, extras = REPORT_EXTRAS): Promise<RunResult> {
    const first = await this.#runOnce(source, flags, false, onStage, extras);
    if (first.kind === 'done') {
      return { ...first.result, trap: null, workerMs: first.wallMs };
    }
    const refused = await this.#runOnce(source, flags, true, onStage, extras);
    if (refused.kind !== 'done') {
      throw new Error(`the refused re-run trapped too: ${refused.message}`);
    }
    return { ...refused.result, trap: first.trap, workerMs: first.wallMs + refused.wallMs };
  }

  cancel(): void {
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

  async #runOnce(source: string, flags: string, refused: boolean, onStage: (name: string) => void, extras: string): Promise<Finished> {
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
          worker.postMessage({ kind: 'run', source, flags, refused, extras });
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
