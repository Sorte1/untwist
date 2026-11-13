import { describe, expect, it } from 'vitest';
import { distinctKeys, hitsNeeded, tablesIn } from './scan';

const coreJs = { id: 'core-js', keys: ['__core-js_shared__', 'IE_PROTO', ' is not a constructor', 'Incorrect invocation', 'filterReject', 'getHostname'] };
const tslib = { id: 'tslib', keys: ['Generator is already executing.', '__values', '__spreadArray', '__extends'] };

describe('the key scan', () => {
  it('needs a few keys, and a small share of a long list', () => {
    expect(hitsNeeded(2)).toBe(2);
    expect(hitsNeeded(10)).toBe(3);
    expect(hitsNeeded(172)).toBe(13);
  });

  it('picks a table once enough of its keys appear', () => {
    const text = 'var s = "__core-js_shared__"; var p = "IE_PROTO"; throw TypeError(x + " is not a constructor");';
    expect(tablesIn(text, [coreJs, tslib])).toEqual(['core-js']);
  });

  it('matches a word key only as a whole word', () => {
    expect(tablesIn('IE_PROTOS filterRejects getHostnames', [coreJs])).toEqual([]);
    expect(tablesIn('IE_PROTO filterReject getHostname', [coreJs])).toEqual(['core-js']);
  });

  it('picks nothing for empty input or no tables', () => {
    expect(tablesIn('', [coreJs])).toEqual([]);
    expect(tablesIn('IE_PROTO filterReject getHostname', [])).toEqual([]);
    expect(tablesIn('anything', [{ id: 'empty', keys: [] }])).toEqual([]);
  });
});

describe('the distinct keys of a table', () => {
  it('drop generic names and keys three tables share', () => {
    const tables = [
      { id: 'a', keys: ['Object', 'shared', 'pair', 'only-a'] },
      { id: 'b', keys: ['shared', 'pair', 'only-b'] },
      { id: 'c', keys: ['shared', 'only-c', 'only-c'] },
    ];
    expect(distinctKeys(tables, new Set(['Object']))).toEqual([
      { id: 'a', keys: ['pair', 'only-a'] },
      { id: 'b', keys: ['pair', 'only-b'] },
      { id: 'c', keys: ['only-c'] },
    ]);
  });
});
