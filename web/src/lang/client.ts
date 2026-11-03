export const DEBOUNCE_MS = 300;
export const RESPAWN_FLOOR_BYTES = 64 * 1024 * 1024;
export const RESPAWN_RATIO = 32;

export type QueryOp = 'definition' | 'references' | 'rename' | 'completions' | 'describe';

export interface Problem {
  readonly message: string;
  readonly severity: 'error' | 'warning';
  readonly from: number;
  readonly to: number;
}

export interface Analyzed {
  readonly available: boolean;
  readonly problems: readonly Problem[];
  readonly ms: number;
  readonly memoryBytes: number;
}

export type LangRequest =
  | { readonly kind: 'init'; readonly module: WebAssembly.Module }
  | { readonly kind: 'analyze'; readonly gen: number; readonly text: string }
  | { readonly kind: 'query'; readonly id: number; readonly op: QueryOp; readonly pos: number; readonly name: string }
  | { readonly kind: 'export'; readonly id: number; readonly text: string; readonly name: string };

export type LangReply =
  | { readonly kind: 'ready'; readonly capBytes: number }
  | { readonly kind: 'failed'; readonly message: string }
  | { readonly kind: 'analyzed'; readonly gen: number; readonly available: boolean; readonly problems: string; readonly ms: number; readonly memoryBytes: number }
  | { readonly kind: 'answer'; readonly id: number; readonly gen: number; readonly value: unknown };

export interface WorkerLike {
  onmessage: ((event: MessageEvent<LangReply>) => void) | null;
  postMessage(message: LangRequest): void;
  terminate(): void;
}

export type LangStatus = 'idle' | 'loading' | 'analysing' | 'ready' | 'off' | 'failed';

export interface LangEvents {
  readonly status: (status: LangStatus) => void;
  readonly analyzed: (analyzed: Analyzed) => void;
}

export function utf8Length(text: string): number {
  let bytes = text.length;
  for (let index = 0; index < text.length; index += 1) {
    const unit = text.charCodeAt(index);
    if (unit >= 0x80) {
      bytes += unit < 0x800 || (unit >= 0xd800 && unit <= 0xdfff) ? 1 : 2;
    }
  }
  return bytes;
}

export function aboveCap(text: string, capBytes: number): boolean {
  if (text.length > capBytes) {
    return true;
  }
  if (text.length * 3 <= capBytes) {
    return false;
  }
  return utf8Length(text) > capBytes;
}

export function wasteful(memoryBytes: number, textBytes: number): boolean {
  return memoryBytes > RESPAWN_FLOOR_BYTES && memoryBytes > textBytes * RESPAWN_RATIO;
}

export function resolveModule(from: string | null, request: string, paths: readonly string[]): string | null {
  if (!request.startsWith('.')) {
    return paths.includes(request) ? request : null;
  }
  const base = from === null ? [] : from.split('/').slice(0, -1);
  const parts = [...base];
  for (const part of request.split('/')) {
    if (part === '..') {
      parts.pop();
    } else if (part !== '.' && part !== '') {
      parts.push(part);
    }
  }
  const joined = parts.join('/');
  const candidates = [joined, `${joined}.js`, `${joined}/index.js`];
  return candidates.find((candidate) => paths.includes(candidate)) ?? null;
}

export function parseProblems(json: string): Problem[] {
  try {
    const value: unknown = JSON.parse(json);
    return Array.isArray(value) ? (value as Problem[]) : [];
  } catch {
    return [];
  }
}

interface Waiter {
  readonly gen: number;
  readonly resolve: (value: unknown) => void;
}

export class LangClient {
  readonly #spawn: () => WorkerLike;
  readonly #module: Promise<WebAssembly.Module>;
  readonly #events: LangEvents;
  #worker: WorkerLike | null = null;
  #capBytes: number | null = null;
  #gen = 0;
  #textBytes = 0;
  #timer: ReturnType<typeof setTimeout> | null = null;
  #source: (() => string) | null = null;
  #latest: (() => string) | null = null;
  #nextId = 1;
  #waiters = new Map<number, Waiter>();
  #ready: Promise<boolean> | null = null;

  constructor(spawn: () => WorkerLike, module: Promise<WebAssembly.Module>, events: LangEvents) {
    this.#spawn = spawn;
    this.#module = module;
    this.#events = events;
  }

  get capBytes(): number | null {
    return this.#capBytes;
  }

  changed(source: () => string): void {
    this.#gen += 1;
    this.#source = source;
    this.#latest = source;
    if (this.#timer !== null) {
      clearTimeout(this.#timer);
    }
    this.#timer = setTimeout(() => this.flush(), DEBOUNCE_MS);
  }

  flush(): void {
    if (this.#timer !== null) {
      clearTimeout(this.#timer);
      this.#timer = null;
    }
    const source = this.#source;
    this.#source = null;
    if (source !== null) {
      void this.#analyze(source(), this.#gen);
    }
  }

  async query(op: QueryOp, pos: number, name = ''): Promise<unknown> {
    this.flush();
    if (!(await this.#start())) {
      return null;
    }
    return this.#ask(this.#gen, (id) => ({ kind: 'query', id, op, pos, name }));
  }

  async exportSpan(text: string, name: string): Promise<readonly number[]> {
    if (!(await this.#start()) || aboveCap(text, this.#capBytes ?? 0)) {
      return [];
    }
    const value = await this.#ask(this.#gen, (id) => ({ kind: 'export', id, text, name }));
    return value instanceof Uint32Array ? [...value] : [];
  }

  cancel(): void {
    if (this.#timer !== null) {
      clearTimeout(this.#timer);
      this.#timer = null;
    }
    this.#source = null;
  }

  dispose(): void {
    if (this.#timer !== null) {
      clearTimeout(this.#timer);
    }
    this.#stop();
  }

  #ask(gen: number, request: (id: number) => LangRequest): Promise<unknown> {
    const id = this.#nextId;
    this.#nextId += 1;
    return new Promise((resolve) => {
      this.#waiters.set(id, { gen, resolve });
      this.#worker?.postMessage(request(id));
    });
  }

  async #analyze(text: string, gen: number): Promise<void> {
    if (text === '' && this.#worker === null) {
      this.#events.status('idle');
      this.#events.analyzed({ available: true, problems: [], ms: 0, memoryBytes: 0 });
      return;
    }
    if (!(await this.#start()) || gen !== this.#gen) {
      return;
    }
    if (aboveCap(text, this.#capBytes ?? 0)) {
      this.#events.status('off');
      this.#events.analyzed({ available: false, problems: [], ms: 0, memoryBytes: 0 });
      return;
    }
    this.#textBytes = text.length;
    this.#events.status('analysing');
    this.#worker?.postMessage({ kind: 'analyze', gen, text });
  }

  #start(): Promise<boolean> {
    if (this.#ready === null) {
      const worker = this.#spawn();
      this.#worker = worker;
      this.#ready = new Promise((resolve) => {
        worker.onmessage = (event) => this.#receive(event.data, resolve);
        this.#module.then(
          (module) => worker.postMessage({ kind: 'init', module }),
          () => resolve(false),
        );
      });
    }
    return this.#ready;
  }

  #receive(reply: LangReply, ready: (ok: boolean) => void): void {
    if (reply.kind === 'ready') {
      this.#capBytes = reply.capBytes;
      ready(true);
    } else if (reply.kind === 'failed') {
      this.#events.status('failed');
      ready(false);
      this.#stop();
    } else if (reply.kind === 'analyzed') {
      this.#analyzed(reply);
    } else {
      this.#answer(reply.id, reply.gen, reply.value);
    }
  }

  #analyzed(reply: Extract<LangReply, { kind: 'analyzed' }>): void {
    if (reply.gen !== this.#gen) {
      return;
    }
    const problems = parseProblems(reply.problems);
    this.#events.status(reply.available ? 'ready' : 'off');
    performance.mark('untwist:lang-analyzed', { detail: { ms: reply.ms, memoryBytes: reply.memoryBytes, available: reply.available } });
    this.#events.analyzed({ available: reply.available, problems, ms: reply.ms, memoryBytes: reply.memoryBytes });
    if (wasteful(reply.memoryBytes, this.#textBytes)) {
      this.#respawn();
    }
  }

  #answer(id: number, gen: number, value: unknown): void {
    const waiter = this.#waiters.get(id);
    this.#waiters.delete(id);
    waiter?.resolve(gen === this.#gen && waiter.gen === this.#gen ? value : null);
  }

  #respawn(): void {
    this.#stop();
    this.#gen += 1;
    this.#source = this.#latest;
    this.flush();
  }

  #stop(): void {
    this.#worker?.terminate();
    this.#worker = null;
    this.#ready = null;
    for (const waiter of this.#waiters.values()) {
      waiter.resolve(null);
    }
    this.#waiters.clear();
  }
}
