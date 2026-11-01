import { describe, expect, it } from 'vitest';
import { isSafeRelative, zipPlan } from './download';
import { bundleOf, treeOrder } from './module-tree';

describe('the module tree', () => {
  it('puts the entry first, then sorts by path', () => {
    const modules = [
      { id: '2', path: 'b.js', isEntry: false },
      { id: '0', path: 'z.js', isEntry: true },
      { id: '1', path: 'a.js', isEntry: false },
    ];
    expect(treeOrder(modules)).toEqual([1, 2, 0]);
  });

  it('reads the bundle section of a report, and nothing from a report without one', () => {
    expect(bundleOf('{"bundle":{"kind":"webpack","modules":[]}}')?.kind).toBe('webpack');
    expect(bundleOf('{"schema":"untwist.report/1"}')).toBeNull();
  });
});

describe('the zip download', () => {
  it('accepts only safe relative paths, as the CLI does', () => {
    expect(isSafeRelative('src/a.js')).toBe(true);
    for (const path of ['', '/abs.js', '..\\x.js', 'a/../b.js', 'C:/x.js', '../up.js']) {
      expect(isSafeRelative(path)).toBe(false);
    }
  });

  it('writes bundle.json, deobfuscated.js and every safe module, and refuses the rest', () => {
    const bundle = { kind: 'webpack', modules: [{ id: '0', path: 'a.js', isEntry: true }, { id: '1', path: '../b.js', isEntry: false }] };
    const plan = zipPlan('{}', bundle, ['a();', 'b();'], 'all();\n');
    expect(Object.keys(plan.files).sort()).toEqual(['a.js', 'bundle.json', 'deobfuscated.js']);
    expect(plan.refused).toEqual(['../b.js']);
  });
});
