import { describe, expect, it } from 'vitest';
import { distinctKeys } from './keys';

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
