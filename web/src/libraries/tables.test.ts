import { createHash } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { INDEX_SCHEMA } from './index-format';
import { LibraryTables, type Scanner } from './tables';

const BASE = new URL('https://untwist.test/libraries/');
const BYTES = new Uint8Array([0x1f, 0x8b, 8, 0, 1, 2, 3]);
const SHA = createHash('sha256').update(BYTES).digest('hex');
const ENTRY = { id: 'core-js', label: 'core-js', file: 'core-js.table', bytes: BYTES.length, sha256: SHA, keys: ['IE_PROTO'] };

function server(files: Record<string, Uint8Array | string>, seen: string[] = []): typeof fetch {
  return async (input) => {
    const url = new URL(String(input));
    seen.push(url.pathname + url.search);
    const body = files[url.pathname];
    return body === undefined ? new Response(null, { status: 404 }) : new Response(body as BodyInit);
  };
}

const index = (tables: unknown[]) => JSON.stringify({ schema: INDEX_SCHEMA, compiledIn: ['tslib'], tables });
const scanAll: Scanner = async (_text, tables) => ({ ids: tables.map((table) => table.id), ms: 0 });

describe('the library tables', () => {
  it('fetch a wanted table once and keep it for later runs', async () => {
    const seen: string[] = [];
    const tables = new LibraryTables(BASE, scanAll, server({ '/libraries/index.json': index([ENTRY]), '/libraries/core-js.table': BYTES }, seen));
    const wanted = await tables.wanted('IE_PROTO');
    expect(wanted.map((table) => table.id)).toEqual(['core-js']);
    const first = await tables.load(wanted);
    const second = await tables.load(wanted);
    expect(first.tables[0]?.bytes).toEqual(BYTES);
    expect(second.tables[0]?.bytes).toEqual(BYTES);
    expect(seen.filter((path) => path.startsWith('/libraries/core-js.table'))).toEqual([`/libraries/core-js.table?v=${SHA.slice(0, 16)}`]);
  });

  it('skip tables already loaded and scan nothing when none are left', async () => {
    let scans = 0;
    const scan: Scanner = async (text, keyed) => {
      scans += 1;
      return scanAll(text, keyed);
    };
    const tables = new LibraryTables(BASE, scan, server({ '/libraries/index.json': index([ENTRY]) }));
    expect(await tables.wanted('IE_PROTO', ['core-js'])).toEqual([]);
    expect(scans).toBe(0);
  });

  it('report a table that is missing or does not match its index entry as failed', async () => {
    const wrong = new Uint8Array([0x1f, 0x8b, 8, 0, 9, 9, 9]);
    const missing = new LibraryTables(BASE, scanAll, server({ '/libraries/index.json': index([ENTRY]) }));
    expect(await missing.load([ENTRY])).toMatchObject({ tables: [], failed: ['core-js'] });
    const changed = new LibraryTables(BASE, scanAll, server({ '/libraries/core-js.table': wrong }));
    expect(await changed.load([ENTRY])).toMatchObject({ tables: [], failed: ['core-js'] });
  });

  it('want nothing without an index, and ask for the index again on the next run', async () => {
    const seen: string[] = [];
    const tables = new LibraryTables(BASE, scanAll, server({}, seen));
    expect(await tables.wanted('IE_PROTO')).toEqual([]);
    expect(await tables.wanted('IE_PROTO')).toEqual([]);
    expect(seen).toEqual(['/libraries/index.json', '/libraries/index.json']);
  });

  it('hand the cancel signal to the scan and to the table download', async () => {
    const signals: Array<AbortSignal | undefined> = [];
    const scan: Scanner = async (text, keyed, signal) => {
      signals.push(signal);
      return scanAll(text, keyed);
    };
    const fetcher: typeof fetch = async (input, init) => {
      signals.push(init?.signal ?? undefined);
      return server({ '/libraries/index.json': index([ENTRY]), '/libraries/core-js.table': BYTES })(input, init);
    };
    const tables = new LibraryTables(BASE, scan, fetcher);
    const abort = new AbortController();
    const wanted = await tables.wanted('IE_PROTO', [], abort.signal);
    await tables.load(wanted, abort.signal);
    expect(signals.filter((signal) => signal === abort.signal)).toHaveLength(2);
  });

  it('count a download stopped by cancel as failed, and fetch it again on the next run', async () => {
    const abort = new AbortController();
    const fetcher: typeof fetch = async (input, init) => {
      if (init?.signal?.aborted === true) {
        throw new DOMException('cancelled', 'AbortError');
      }
      return server({ '/libraries/core-js.table': BYTES })(input, init);
    };
    const tables = new LibraryTables(BASE, scanAll, fetcher);
    abort.abort();
    expect(await tables.load([ENTRY], abort.signal)).toMatchObject({ tables: [], failed: ['core-js'] });
    expect((await tables.load([ENTRY])).tables.map((table) => table.id)).toEqual(['core-js']);
  });

  it('time each table and say where it came from', async () => {
    const tables = new LibraryTables(BASE, scanAll, server({ '/libraries/core-js.table': BYTES }));
    const first = await tables.load([ENTRY]);
    expect(first.timings).toEqual([{ id: 'core-js', bytes: BYTES.length, ms: expect.any(Number), source: 'network' }]);
    const second = await tables.load([ENTRY]);
    expect(second.timings[0]?.source).toBe('memory');
    const missing = await new LibraryTables(BASE, scanAll, server({})).load([ENTRY]);
    expect(missing.timings).toEqual([{ id: 'core-js', bytes: BYTES.length, ms: 0, source: null }]);
  });
});
