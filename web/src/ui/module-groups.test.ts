import { describe, expect, it } from 'vitest';
import { countLabel, groupModules, libraryLabel, libraryRanges, packageOf } from './module-groups';
import { treeOrder } from './module-tree';

const module = (id: string, path: string, isEntry = false) => ({ id, path, isEntry });

describe('library groups in the module tree', () => {
  it('reads the package of a node_modules path, scoped or not', () => {
    expect(packageOf('node_modules/core-js/internals/export.js')).toBe('core-js');
    expect(packageOf('node_modules/@babel/runtime/helpers/typeof.js')).toBe('@babel/runtime');
    expect(packageOf('node_modules/core-js')).toBeNull();
    expect(packageOf('node_modules/@babel/runtime')).toBeNull();
    expect(packageOf('src/node_modules/x/a.js')).toBeNull();
    expect(packageOf('index.js')).toBeNull();
  });

  it('reads the version range of each library from the report', () => {
    const report = JSON.stringify({ libraries: [{ library: 'core-js', range: '3.39', modules: 4 }, { library: 'tslib', range: '', modules: 1 }] });
    expect([...libraryRanges(report)]).toEqual([
      ['core-js', '3.39'],
      ['tslib', ''],
    ]);
    expect(libraryRanges('{}').size).toBe(0);
  });

  it('splits app modules from library modules and groups libraries by package', () => {
    const modules = [
      module('0', 'index.js', true),
      module('1', 'node_modules/core-js/internals/export.js'),
      module('2', 'helpers.js'),
      module('3', 'node_modules/core-js/internals/global-this.js'),
      module('4', 'node_modules/@babel/runtime/helpers/typeof.js'),
    ];
    const groups = groupModules(modules, treeOrder(modules), new Map([['core-js', '3.39']]));
    expect(groups).toEqual({
      app: [0, 2],
      libraries: [
        { name: '@babel/runtime', range: '', order: [4] },
        { name: 'core-js', range: '3.39', order: [1, 3] },
      ],
    });
  });

  it('leaves the tree flat when no module is a library module', () => {
    const modules = [module('0', 'index.js', true), module('1', 'lib.js')];
    expect(groupModules(modules, treeOrder(modules), new Map([['core-js', '3.39']]))).toBeNull();
  });

  it('labels a library with its range and counts modules', () => {
    expect(libraryLabel({ name: 'core-js', range: '3.39', order: [] })).toBe('core-js 3.39');
    expect(libraryLabel({ name: 'tslib', range: '', order: [] })).toBe('tslib');
    expect(countLabel(1)).toBe('1 module');
    expect(countLabel(1200)).toBe('1,200 modules');
  });
});
