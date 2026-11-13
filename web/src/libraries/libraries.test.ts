import { describe, expect, it } from 'vitest';
import { libraryComment, librariesOf } from './libraries';

const known = new Set(['core-js', 'tslib']);

describe('the library list of a report', () => {
  it('is empty for a report without libraries or with a malformed field', () => {
    expect(librariesOf('{}')).toEqual([]);
    expect(librariesOf('{"libraries":{"core-js":1}}')).toEqual([]);
    expect(librariesOf('not json')).toEqual([]);
  });

  it('takes counted rows and sorts the largest library first', () => {
    const report = JSON.stringify({
      libraries: [
        { library: 'tslib', range: '2.x', modules: 1 },
        { library: 'core-js', range: '3.30.0 - 3.38.1', modules: 412 },
      ],
    });
    expect(librariesOf(report)).toEqual([
      { library: 'core-js', range: '3.30.0 - 3.38.1', modules: 412 },
      { library: 'tslib', range: '2.x', modules: 1 },
    ]);
  });

  it('groups per-module rows of one library and range, and counts module lists', () => {
    const report = JSON.stringify({
      libraries: [
        { name: 'core-js', version: '3.x', path: 'internals/get-built-in.js' },
        { name: 'core-js', version: '3.x', path: 'internals/own-keys.js' },
        { library: 'tslib', range: '2.x', modules: ['a', 'b'] },
        { range: 'no name' },
      ],
    });
    expect(librariesOf(report)).toEqual([
      { library: 'core-js', range: '3.x', modules: 2 },
      { library: 'tslib', range: '2.x', modules: 2 },
    ]);
  });
});

describe('a library comment line', () => {
  it('reads the library, range and path of a known library', () => {
    expect(libraryComment('  // core-js 3.30.0 - 3.38.1: internals/get-built-in.js', known)).toEqual({
      library: 'core-js',
      range: '3.30.0 - 3.38.1',
      path: 'internals/get-built-in.js',
    });
  });

  it('ignores a comment of the same shape whose library the report does not list', () => {
    expect(libraryComment('// note this: keep it', known)).toBeNull();
    expect(libraryComment('// lodash 4.x: map.js', known)).toBeNull();
  });

  it('ignores a trailing comment and a comment without a path', () => {
    expect(libraryComment('a(); // core-js 3.x: a.js', known)).toBeNull();
    expect(libraryComment('// core-js 3.x', known)).toBeNull();
  });
});
