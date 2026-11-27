import { readFileSync } from 'node:fs';
import { gunzipSync } from 'node:zlib';
import { describe, expect, it } from 'vitest';
import { bloomFile, bloomParams, bloomWithKeys, mix } from './bloom';
import { summariseTable } from './table-text';

const ROOT = new URL('../../../fixtures/library-collection/', import.meta.url);
const PARAMS = bloomParams(readFileSync(new URL('../../../crates/untwist-rename/src/library/collection.rs', import.meta.url), 'utf8'));

interface FixtureIndex {
  readonly packages: readonly { readonly id: string; readonly shards: readonly { readonly file: string }[] }[];
  readonly filters: { readonly file: string };
}

describe('the bloom file', () => {
  it('mixes like the Rust splitmix finaliser', () => {
    expect(mix(0n)).toBe(0xe220a8397b1dcdafn);
    expect(mix(1n)).toBe(0x910a2dec89025cc1n);
  });

  it('is byte-identical to the one Rust wrote for the test collection', () => {
    const index = JSON.parse(readFileSync(new URL('collection.json', ROOT), 'utf8')) as FixtureIndex;
    const entries = index.packages.map((pkg) => {
      const loose = pkg.shards.flatMap((shard) => summariseTable(gunzipSync(readFileSync(new URL(shard.file, ROOT))).toString('utf8')).loose);
      return [pkg.id, bloomWithKeys(loose, PARAMS)] as const;
    });
    const expected = new Uint8Array(readFileSync(new URL(index.filters.file, ROOT)));
    expect(Buffer.from(bloomFile(entries)).equals(Buffer.from(expected))).toBe(true);
  });

  it('keeps the minimum bits for an empty key set', () => {
    const bloom = bloomWithKeys([], PARAMS);
    expect(bloom.count).toBe(PARAMS.minBits);
    expect(bloom.hashes).toBe(PARAMS.hashes);
    expect(bloom.bits).toEqual(new Uint8Array(Number(PARAMS.minBits / 8n)));
  });

  it('reads its parameters from the Rust constants', () => {
    const source = 'pub const BLOOM_BITS_PER_KEY: u64 = 16;\npub const BLOOM_HASHES: u8 = 11;\nconst BLOOM_SEED: u64 = 0x5bd1_e995_9e37_79b9;\nconst BLOOM_MIN_BITS: u64 = 64;';
    expect(bloomParams(source)).toEqual({ bitsPerKey: 16n, hashes: 11, seed: 0x5bd1e9959e3779b9n, minBits: 64n });
    expect(() => bloomParams('')).toThrow(/BLOOM_BITS_PER_KEY/);
  });
});
