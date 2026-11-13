import { describe, expect, it } from 'vitest';
import { INDEX_SCHEMA, parseIndex } from './index-format';

const SHA = 'a'.repeat(64);
const table = { id: 'core-js', label: 'core-js', file: 'core-js.table', bytes: 10, sha256: SHA, keys: ['IE_PROTO'] };

describe('the library index', () => {
  it('reads valid tables and drops ones compiled into the engine', () => {
    const text = JSON.stringify({ schema: INDEX_SCHEMA, compiledIn: ['tslib'], tables: [table, { ...table, id: 'tslib', file: 'tslib.table' }] });
    expect(parseIndex(text)).toEqual({ schema: INDEX_SCHEMA, compiledIn: ['tslib'], tables: [table] });
  });

  it('drops a table whose file is not its own or whose fields are malformed', () => {
    const bad = [{ ...table, file: '../x.table' }, { ...table, id: 'Core JS' }, { ...table, sha256: 'x' }, { ...table, bytes: 0 }, { ...table, keys: [1] }];
    expect(parseIndex(JSON.stringify({ schema: INDEX_SCHEMA, tables: bad }))?.tables).toEqual([]);
  });

  it('refuses another schema or malformed text', () => {
    expect(parseIndex(JSON.stringify({ schema: 'other', tables: [table] }))).toBeNull();
    expect(parseIndex('not json')).toBeNull();
    expect(parseIndex('null')).toBeNull();
  });
});
