import { describe, expect, it } from 'vitest';
import { INDEX_SCHEMA, type TableIndex } from './index-format';
import { tableRows, tableStateText } from './panel-model';

const table = (id: string, bytes: number) => ({ id, label: id, file: `${id}.table`, bytes, sha256: '0'.repeat(64), keys: [] });
const INDEX: TableIndex = { schema: INDEX_SCHEMA, compiledIn: ['tslib'], tables: [table('core-js', 3_800_000), table('lodash', 90_000), table('three', 400_000)] };

describe('tableRows', () => {
  it('lists loaded, failed, built-in and skipped tables with their size and load time', () => {
    const rows = tableRows(INDEX, {
      fetched: ['core-js'],
      failed: ['three'],
      reran: false,
      scanMs: 5,
      loadMs: 40,
      timings: [
        { id: 'core-js', bytes: 3_800_000, ms: 38, source: 'network' },
        { id: 'three', bytes: 400_000, ms: 0, source: null },
      ],
    });
    expect(rows.map((row) => [row.id, row.state, row.bytes, row.ms])).toEqual([
      ['core-js', 'loaded', 3_800_000, 38],
      ['three', 'failed', 400_000, 0],
      ['tslib', 'built-in', null, null],
      ['lodash', 'skipped', 90_000, null],
    ]);
    expect(tableStateText(rows[0]!)).toBe('loaded (downloaded)');
    expect(tableStateText(rows[3]!)).toBe('skipped: too few of its names in the code');
  });

  it('has no rows without an index, and skips every table a run did not scan for', () => {
    expect(tableRows(null, undefined)).toEqual([]);
    expect(tableRows(INDEX, undefined).filter((row) => row.state === 'skipped').map((row) => row.id)).toEqual(['core-js', 'lodash', 'three']);
  });
});
