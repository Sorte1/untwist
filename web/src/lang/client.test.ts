import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { aboveCap, DEBOUNCE_MS, LangClient, resolveModule, utf8Length, wasteful, type LangReply, type LangRequest, type WorkerLike } from './client';

class FakeWorker implements WorkerLike {
  onmessage: ((event: MessageEvent<LangReply>) => void) | null = null;
  readonly sent: LangRequest[] = [];
  terminated = false;

  postMessage(message: LangRequest): void {
    this.sent.push(message);
    if (message.kind === 'init') {
      this.reply({ kind: 'ready', capBytes: 100 });
    }
  }

  terminate(): void {
    this.terminated = true;
  }

  reply(message: LangReply): void {
    this.onmessage?.({ data: message } as MessageEvent<LangReply>);
  }

  analyses(): Extract<LangRequest, { kind: 'analyze' }>[] {
    return this.sent.filter((m): m is Extract<LangRequest, { kind: 'analyze' }> => m.kind === 'analyze');
  }
}

function setup() {
  const workers: FakeWorker[] = [];
  const statuses: string[] = [];
  const problems: number[] = [];
  const client = new LangClient(
    () => {
      const worker = new FakeWorker();
      workers.push(worker);
      return worker;
    },
    Promise.resolve({} as WebAssembly.Module),
    { status: (s) => statuses.push(s), analyzed: (a) => problems.push(a.problems.length) },
  );
  return { client, workers, statuses, problems };
}

async function settle(): Promise<void> {
  for (let i = 0; i < 5; i += 1) {
    await Promise.resolve();
  }
}

describe('sizes', () => {
  test('utf8Length counts bytes the way the engine does', () => {
    expect(utf8Length('aé😀€')).toBe(1 + 2 + 4 + 3);
  });

  test('aboveCap compares UTF-8 bytes against the cap', () => {
    expect(aboveCap('a'.repeat(11), 10)).toBe(true);
    expect(aboveCap('a'.repeat(3), 10)).toBe(false);
    expect(aboveCap('ééééé', 9)).toBe(true);
    expect(aboveCap('ééééé', 10)).toBe(false);
  });

  test('a worker is wasteful only when its memory dwarfs the text and passes the floor', () => {
    expect(wasteful(200 * 1024 * 1024, 1024)).toBe(true);
    expect(wasteful(200 * 1024 * 1024, 10 * 1024 * 1024)).toBe(false);
    expect(wasteful(10 * 1024 * 1024, 1)).toBe(false);
  });
});

describe('resolveModule', () => {
  const paths = ['index.js', '802.js', 'lib/util.js', 'lib/index.js', '_runtime/webpack.js'];

  test('relative requests resolve against the importing module', () => {
    expect(resolveModule('index.js', './802.js', paths)).toBe('802.js');
    expect(resolveModule('lib/util.js', '../802', paths)).toBe('802.js');
    expect(resolveModule('index.js', './lib', paths)).toBe('lib/index.js');
    expect(resolveModule('index.js', './_runtime/webpack.js', paths)).toBe('_runtime/webpack.js');
  });

  test('a request that names no module resolves to null', () => {
    expect(resolveModule('index.js', './missing.js', paths)).toBeNull();
    expect(resolveModule('index.js', 'react', paths)).toBeNull();
  });
});

describe('LangClient', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  test('edits are analysed once, after the debounce', async () => {
    const { client, workers } = setup();
    client.changed(() => 'a');
    client.changed(() => 'ab');
    vi.advanceTimersByTime(DEBOUNCE_MS - 1);
    await settle();
    expect(workers).toHaveLength(0);
    vi.advanceTimersByTime(1);
    await settle();
    expect(workers[0]?.analyses().map((m) => m.text)).toEqual(['ab']);
  });

  test('an empty text is clean without starting a worker', async () => {
    const { client, workers, statuses, problems } = setup();
    client.changed(() => '');
    client.flush();
    await settle();
    expect(workers).toHaveLength(0);
    expect(statuses).toEqual(['idle']);
    expect(problems).toEqual([0]);
    client.changed(() => 'a');
    client.flush();
    await settle();
    expect(workers[0]?.analyses().map((m) => m.text)).toEqual(['a']);
  });

  test('a cancelled edit is never analysed', async () => {
    const { client, workers } = setup();
    client.changed(() => 'a');
    client.cancel();
    vi.advanceTimersByTime(DEBOUNCE_MS);
    await settle();
    expect(workers).toHaveLength(0);
  });

  test('a stale analysis and a stale answer are dropped', async () => {
    const { client, workers, problems } = setup();
    client.changed(() => 'a');
    client.flush();
    await settle();
    const worker = workers[0]!;
    const first = worker.analyses()[0]!;
    const answer = client.query('references', 0);
    await settle();
    const query = worker.sent.find((m) => m.kind === 'query')!;
    client.changed(() => 'ab');
    worker.reply({ kind: 'analyzed', gen: first.gen, available: true, problems: '[{}]', ms: 1, memoryBytes: 1 });
    worker.reply({ kind: 'answer', id: (query as { id: number }).id, gen: first.gen, value: new Uint32Array([0, 1]) });
    expect(problems).toEqual([]);
    expect(await answer).toBeNull();
  });

  test('a fresh answer is delivered and a text above the cap is not sent', async () => {
    const { client, workers, statuses } = setup();
    client.changed(() => 'x'.repeat(200));
    client.flush();
    await settle();
    expect(workers[0]?.analyses()).toEqual([]);
    expect(statuses).toContain('off');
    client.changed(() => 'a');
    const answer = client.query('definition', 0);
    await settle();
    const worker = workers[0]!;
    const gen = worker.analyses()[0]!.gen;
    const query = worker.sent.find((m) => m.kind === 'query') as { id: number };
    worker.reply({ kind: 'answer', id: query.id, gen, value: '{"span":[0,1]}' });
    expect(await answer).toBe('{"span":[0,1]}');
  });

  test('a worker left large by an earlier text is replaced and the text re-analysed', async () => {
    const { client, workers } = setup();
    client.changed(() => 'a');
    client.flush();
    await settle();
    const first = workers[0]!;
    first.reply({ kind: 'analyzed', gen: first.analyses()[0]!.gen, available: true, problems: '[]', ms: 1, memoryBytes: 512 * 1024 * 1024 });
    await settle();
    expect(first.terminated).toBe(true);
    expect(workers[1]?.analyses().map((m) => m.text)).toEqual(['a']);
  });
});
