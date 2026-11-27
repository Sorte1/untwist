import { describe, expect, it } from 'vitest';
import { COLLECTION_SCHEMA, type CollectionIndex } from './collection-format';
import { builtInIds, tableRows, tableStateText } from './panel-model';

const pkg = (id: string, bytes: number) => ({ id, label: id, versions: [], keys: [], curated: true, shards: [{ file: `shards/${id}.utl`, bytes, sha256: '0'.repeat(64), versions: '', rows: 1 }] });
const INDEX: CollectionIndex = { schema: COLLECTION_SCHEMA, version: 'test', shape_epoch: 1, untwist: '', packages: [pkg('core-js', 3_800_000), pkg('lodash', 90_000), pkg('three', 400_000)] };
const USED = {
  collection: 'test',
  error: '',
  fetched: ['core-js'],
  failed: ['three'],
  reran: false,
  scanMs: 5,
  loadMs: 40,
  timings: [
    { id: 'core-js', bytes: 3_800_000, ms: 38, source: 'network' as const },
    { id: 'three', bytes: 400_000, ms: 0, source: null },
  ],
};

describe('tableRows', () => {
  it('lists loaded, failed, built-in and skipped packages with their size and load time', () => {
    const rows = tableRows(INDEX, USED, ['tslib']);
    expect(rows.map((row) => [row.id, row.state, row.bytes, row.ms])).toEqual([
      ['core-js', 'loaded', 3_800_000, 38],
      ['three', 'failed', 400_000, 0],
      ['tslib', 'built-in', null, null],
      ['lodash', 'skipped', 90_000, null],
    ]);
    expect(tableStateText(rows[0]!)).toBe('loaded (downloaded)');
    expect(tableStateText(rows[3]!)).toBe('skipped: none of its code is in this run');
  });

  it('has no rows without a collection, and skips every package a run did not want', () => {
    expect(tableRows(null, undefined)).toEqual([]);
    expect(tableRows(INDEX, undefined).filter((row) => row.state === 'skipped').map((row) => row.id)).toEqual(['core-js', 'lodash', 'three']);
  });

  it('takes the built-in tables from the engine list, without the collection packages', () => {
    expect(builtInIds('tslib,core-js,,swc-helpers', INDEX, USED)).toEqual(['tslib', 'swc-helpers']);
    expect(builtInIds(undefined, INDEX, undefined)).toEqual([]);
  });
});
