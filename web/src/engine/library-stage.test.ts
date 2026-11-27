import { createHash } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { COLLECTION_SCHEMA, type CollectionIndex } from '../libraries/collection-format';
import { LibraryCollection, type CollectionFiles } from '../libraries/collection';
import { LibraryStage, type LibraryApi } from './library-stage';

const BASE = 'https://untwist.test/libraries/';
const CORE = new Uint8Array([1, 2, 3]);
const LODASH = new Uint8Array([4, 5]);
const sha = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex');
const shard = (bytes: Uint8Array) => ({ file: `shards/${sha(bytes)}.utl`, bytes: bytes.length, sha256: sha(bytes), versions: '', rows: 1 });
const INDEX: CollectionIndex = {
  schema: COLLECTION_SCHEMA,
  version: 'test',
  shape_epoch: 1,
  untwist: '',
  packages: [
    { id: 'core-js', label: 'core-js', versions: [], keys: [], shards: [shard(CORE)] },
    { id: 'lodash', label: 'lodash', versions: [], keys: [], shards: [shard(LODASH)] },
  ],
};
const FILES: CollectionFiles = { base: BASE, text: '{}', index: INDEX, filters: null };

function fakeApi(wants: Record<string, readonly Uint8Array[]>, loaded: string[]): LibraryApi {
  return {
    attach: () => '',
    wanted: (text) => JSON.stringify(Object.entries(wants).filter(([needle]) => text.includes(needle)).flatMap(([, list]) => list.map(sha))),
    load: (bytes) => {
      const id = sha(bytes) === sha(CORE) ? 'core-js' : sha(bytes) === sha(LODASH) ? 'lodash' : '';
      if (id !== '') {
        loaded.push(id);
      }
      return id;
    },
  };
}

function server(files: Record<string, Uint8Array>, seen: string[]) {
  const collection = new LibraryCollection(BASE, async (input: string): Promise<Response> => {
    const path = new URL(input).pathname;
    seen.push(path);
    const body = files[path];
    return body === undefined ? new Response(null, { status: 404 }) : new Response(body as BodyInit);
  });
  return (shards: Parameters<LibraryCollection['shards']>[0]) => collection.shards(shards);
}

const SERVED = { [`/libraries/${shard(CORE).file}`]: CORE, [`/libraries/${shard(LODASH).file}`]: LODASH };

describe('the library stage of a run', () => {
  it('fetches nothing when no shard is wanted', async () => {
    const seen: string[] = [];
    const stages: string[] = [];
    const stage = new LibraryStage(fakeApi({ IE_PROTO: [CORE] }, []), FILES, (name) => stages.push(name), server(SERVED, seen));
    expect(await stage.take(['console.log(1)'], 'scan')).toBe(0);
    expect(seen).toEqual([]);
    expect(stages).toEqual(['scan']);
    expect(stage.used(false)).toMatchObject({ fetched: [], failed: [], reran: false, timings: [] });
  });

  it('fetches the wanted shards once, and only new ones for the output', async () => {
    const seen: string[] = [];
    const loaded: string[] = [];
    const stages: string[] = [];
    const stage = new LibraryStage(fakeApi({ IE_PROTO: [CORE], lodash: [LODASH] }, loaded), FILES, (name) => stages.push(name), server(SERVED, seen));
    expect(await stage.take(['IE_PROTO'], 'scan')).toBe(1);
    expect(await stage.take(['IE_PROTO', 'lodash'], 'scan-output')).toBe(1);
    expect(await stage.take(['IE_PROTO lodash'], 'scan-output')).toBe(0);
    expect(loaded).toEqual(['core-js', 'lodash']);
    expect(seen).toHaveLength(2);
    expect(stages).toEqual(['scan', 'tables:core-js (3 B)', 'load-tables', 'scan-output', 'tables:lodash (2 B)', 'load-tables', 'scan-output']);
    const used = stage.used(true);
    expect(used).toMatchObject({ collection: 'test', fetched: ['core-js', 'lodash'], failed: [], reran: true });
    expect(used.timings?.map((row) => [row.id, row.bytes, row.source])).toEqual([
      ['core-js', 3, 'network'],
      ['lodash', 2, 'network'],
    ]);
  });

  it('reports a shard that does not download or does not match as failed', async () => {
    const seen: string[] = [];
    const stage = new LibraryStage(fakeApi({ IE_PROTO: [CORE] }, []), FILES, () => undefined, server({ [`/libraries/${shard(CORE).file}`]: LODASH }, seen));
    expect(await stage.take(['IE_PROTO'], 'scan')).toBe(0);
    expect(stage.used(false)).toMatchObject({ fetched: [], failed: ['core-js'] });
    expect(stage.used(false).timings?.[0]?.source).toBeNull();
  });

  it('takes a shard from memory in a later run of the same page', async () => {
    const seen: string[] = [];
    const fetcher = server(SERVED, seen);
    await new LibraryStage(fakeApi({ IE_PROTO: [CORE] }, []), FILES, () => undefined, fetcher).take(['IE_PROTO'], 'scan');
    const second = new LibraryStage(fakeApi({ IE_PROTO: [CORE] }, []), FILES, () => undefined, fetcher);
    expect(await second.take(['IE_PROTO'], 'scan')).toBe(1);
    expect(second.used(false).timings?.[0]?.source).toBe('memory');
    expect(seen).toHaveLength(1);
  });
});
