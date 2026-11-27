import { createHash } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { COLLECTION_SCHEMA, parseCollection, safeFile, shardOwner } from './collection-format';
import { LibraryCollection, collectionUrls } from './collection';
import { fetchContent } from './content';

const BASE = 'https://untwist.test/libraries/';
const SHARD = new Uint8Array([0x1f, 0x8b, 8, 0, 1, 2, 3]);
const BLOOM = new Uint8Array([85, 84, 66, 76, 79, 79, 77, 49, 0, 0, 0, 0]);
const sha = (bytes: Uint8Array) => createHash('sha256').update(bytes).digest('hex');
const SHARD_SHA = sha(SHARD);
const BLOOM_SHA = sha(BLOOM);

const index = (overrides: Record<string, unknown> = {}) =>
  JSON.stringify({
    schema: COLLECTION_SCHEMA,
    version: '2026.12.1',
    shape_epoch: 1,
    untwist: '>=0.6.0',
    packages: [{ id: 'core-js', label: 'core-js', shards: [{ file: `shards/${SHARD_SHA}.utl`, bytes: SHARD.length, sha256: SHARD_SHA }], keys: ['IE_PROTO'], curated: true }],
    filters: { file: `shards/${BLOOM_SHA}.bloom`, sha256: BLOOM_SHA, bytes: BLOOM.length },
    ...overrides,
  });

function server(files: Record<string, Uint8Array | string>, seen: string[] = []) {
  return async (input: string): Promise<Response> => {
    const url = new URL(input);
    seen.push(url.pathname);
    const body = files[url.pathname];
    return body === undefined ? new Response(null, { status: 404 }) : new Response(body as BodyInit);
  };
}

const FILES = { '/libraries/collection.json': index(), [`/libraries/shards/${SHARD_SHA}.utl`]: SHARD, [`/libraries/shards/${BLOOM_SHA}.bloom`]: BLOOM };

describe('the collection index', () => {
  it('reads the Rust serde shape with its defaults', () => {
    const parsed = parseCollection(index());
    expect(parsed?.packages[0]?.shards[0]).toEqual({ file: `shards/${SHARD_SHA}.utl`, bytes: SHARD.length, sha256: SHARD_SHA, versions: '', rows: 0 });
    expect(parsed?.packages[0]?.versions).toEqual([]);
    expect(parsed?.packages[0]?.curated).toBe(true);
    expect(parsed === null ? null : shardOwner(parsed, SHARD_SHA)?.pkg.id).toBe('core-js');
  });

  it('refuses another schema, unsafe paths and short hashes', () => {
    expect(parseCollection('{')).toBeNull();
    expect(parseCollection(index({ schema: 'untwist.library-index/1' }))).toBeNull();
    expect(parseCollection(index({ filters: { file: '../secret', sha256: BLOOM_SHA } }))).toBeNull();
    expect(parseCollection(index({ packages: [{ id: 'a', label: 'a', shards: [{ file: 'shards/a.utl', bytes: 1, sha256: 'abc' }] }] }))).toBeNull();
    expect(['shards/a.utl', 'a'].map(safeFile)).toEqual([true, true]);
    expect(['', '/a', 'a/../b', 'a//b', './a', 'a\\b'].map(safeFile)).toEqual([false, false, false, false, false, false]);
  });
});

describe('the library collection', () => {
  it('fetches the index fresh and the bloom file by its hash, once per page', async () => {
    const seen: string[] = [];
    const collection = new LibraryCollection(BASE, server(FILES, seen));
    const files = await collection.files();
    await collection.files();
    expect(files?.index.version).toBe('2026.12.1');
    expect(files?.filters).toEqual(BLOOM);
    expect(seen).toEqual(['/libraries/collection.json', `/libraries/shards/${BLOOM_SHA}.bloom`]);
    expect(files === null ? [] : collectionUrls(files)).toEqual([`${BASE}shards/${BLOOM_SHA}.bloom`, `${BASE}shards/${SHARD_SHA}.utl`]);
  });

  it('gives no collection when the bloom file does not match, and tries again later', async () => {
    const seen: string[] = [];
    const collection = new LibraryCollection(BASE, server({ ...FILES, [`/libraries/shards/${BLOOM_SHA}.bloom`]: new Uint8Array([1]) }, seen));
    expect(await collection.files()).toBeNull();
    expect(await collection.index()).toBeNull();
    expect(seen.filter((path) => path.endsWith('collection.json'))).toHaveLength(2);
  });
});

describe('content-addressed fetches', () => {
  it('check the size and SHA-256 the collection names', async () => {
    const url = `${BASE}shards/${SHARD_SHA}.utl`;
    expect((await fetchContent(url, { sha256: SHARD_SHA, bytes: SHARD.length }, server(FILES))).source).toBe('network');
    await expect(fetchContent(url, { sha256: SHARD_SHA, bytes: SHARD.length + 1 }, server(FILES))).rejects.toThrow(/does not match/);
    await expect(fetchContent(url, { sha256: BLOOM_SHA, bytes: 0 }, server(FILES))).rejects.toThrow(/does not match/);
    await expect(fetchContent(`${BASE}shards/missing.utl`, { sha256: SHARD_SHA, bytes: 0 }, server(FILES))).rejects.toThrow(/HTTP 404/);
  });
});
