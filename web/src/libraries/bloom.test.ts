import { readFileSync } from 'node:fs';
import { gunzipSync } from 'node:zlib';
import { describe, expect, it } from 'vitest';
import { bloomFile, bloomWithKeys, mix } from './bloom';
import { summariseTable } from './table-text';

const ROOT = new URL('../../../fixtures/library-collection/', import.meta.url);

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
      return [pkg.id, bloomWithKeys(loose)] as const;
    });
    const expected = new Uint8Array(readFileSync(new URL(index.filters.file, ROOT)));
    expect(Buffer.from(bloomFile(entries)).equals(Buffer.from(expected))).toBe(true);
  });

  it('keeps 64 bits for an empty key set', () => {
    const bloom = bloomWithKeys([]);
    expect(bloom.count).toBe(64n);
    expect(bloom.bits).toEqual(new Uint8Array(8));
  });
});
