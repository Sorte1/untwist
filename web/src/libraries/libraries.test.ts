import { describe, expect, it } from 'vitest';
import { commentShape, libraryComment, librariesOf } from './libraries';

const known = new Set(['core-js', '@babel/runtime']);

describe('the library list of a report', () => {
  it('is empty for a report without libraries or with a malformed field', () => {
    expect(librariesOf('{}')).toEqual([]);
    expect(librariesOf('{"libraries":{"core-js":1}}')).toEqual([]);
    expect(librariesOf('not json')).toEqual([]);
  });

  it('reads the engine rows and sorts the largest library first', () => {
    const report = JSON.stringify({
      libraries: [
        { library: 'tslib', range: '2.6–2.8', modules: 1 },
        { library: 'core-js', range: '3.39', modules: 243 },
        { range: 'no name', modules: 3 },
      ],
    });
    expect(librariesOf(report)).toEqual([
      { library: 'core-js', range: '3.39', modules: 243 },
      { library: 'tslib', range: '2.6–2.8', modules: 1 },
    ]);
  });
});

describe('a library comment line', () => {
  it('reads the library, range and module of a known library', () => {
    expect(libraryComment('  // core-js 3.39: internals/get-built-in.js', known)).toEqual({
      library: 'core-js',
      range: '3.39',
      modules: ['internals/get-built-in.js'],
      also: [],
    });
  });

  it('reads every module of a shape several modules share', () => {
    expect(commentShape('// core-js 3.0–3.39: internals/a.js | internals/b.js')?.modules).toEqual(['internals/a.js', 'internals/b.js']);
  });

  it('reads the other libraries of a shape they share', () => {
    expect(libraryComment('// @babel/runtime 7.24–8.1: helpers/typeof.js, same code in @swc/helpers 0.5: _type_of.js', known)).toEqual({
      library: '@babel/runtime',
      range: '7.24–8.1',
      modules: ['helpers/typeof.js'],
      also: [{ library: '@swc/helpers', range: '0.5', modules: ['_type_of.js'] }],
    });
  });

  it('keeps a shared comment when only another library it names is listed', () => {
    expect(libraryComment('// @swc/helpers 0.5: _type_of.js, same code in core-js 3.39: a.js', known)?.library).toBe('@swc/helpers');
  });

  it('ignores a comment of the same shape whose library the report does not list', () => {
    expect(libraryComment('// note this: keep it', known)).toBeNull();
    expect(libraryComment('// lodash 4.17: map.js', known)).toBeNull();
  });

  it('ignores a trailing comment, a comment without a module and a broken shared part', () => {
    expect(libraryComment('a(); // core-js 3.39: a.js', known)).toBeNull();
    expect(libraryComment('// core-js 3.39', known)).toBeNull();
    expect(libraryComment('// core-js 3.39: a.js, same code in nothing', known)).toBeNull();
  });
});
