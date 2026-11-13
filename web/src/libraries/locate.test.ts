import { describe, expect, it } from 'vitest';
import { locateModules } from './locate';

const known = new Set(['core-js', '@babel/runtime']);

describe('locateModules', () => {
  it('finds each module comment of a known library with its line in the output', () => {
    const output = ['var a = 1;', '// core-js 3.39: internals/global-this.js', 'function x() {}', '  // core-js 3.39: internals/fails.js | internals/has.js', '// lodash 4.17: chunk.js'].join('\n');
    const found = locateModules(output, [], known);
    expect([...found.keys()]).toEqual(['core-js']);
    expect(found.get('core-js')).toEqual([
      { library: 'core-js', range: '3.39', module: 'internals/global-this.js', file: null, line: 2 },
      { library: 'core-js', range: '3.39', module: 'internals/fails.js', file: null, line: 4 },
      { library: 'core-js', range: '3.39', module: 'internals/has.js', file: null, line: 4 },
    ]);
  });

  it('lists shared code under every known library the comment names', () => {
    const output = '// @babel/runtime 7.24–8.1: helpers/typeof.js, same code in core-js 3.0: internals/typeof.js';
    const found = locateModules(output, [], known);
    expect(found.get('@babel/runtime')?.[0]).toMatchObject({ module: 'helpers/typeof.js', line: 1 });
    expect(found.get('core-js')?.[0]).toMatchObject({ module: 'internals/typeof.js', line: 1 });
  });

  it('points into the split files when the run unpacked a bundle', () => {
    const found = locateModules('bundle text', ['let a;', 'x();\n// core-js 3.39: internals/a.js\nf();'], known);
    expect(found.get('core-js')).toEqual([{ library: 'core-js', range: '3.39', module: 'internals/a.js', file: 1, line: 2 }]);
  });

  it('finds nothing when no library is known or the text has no comment', () => {
    expect(locateModules('// core-js 3.39: a.js', [], new Set()).size).toBe(0);
    expect(locateModules('var a = "// not a comment";', [], known).size).toBe(0);
    expect(locateModules('', [], known).size).toBe(0);
  });
});
