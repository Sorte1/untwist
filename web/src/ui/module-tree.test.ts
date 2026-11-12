import { describe, expect, it } from 'vitest';
import { isSafeRelative, zipPlan } from './download';
import { bundleOf, limitFor, shownModules, TREE_PAGE, treeOrder } from './module-tree';

describe('the module tree', () => {
  it('puts the entry first, then sorts by path', () => {
    const modules = [
      { id: '2', path: 'b.js', isEntry: false },
      { id: '0', path: 'z.js', isEntry: true },
      { id: '1', path: 'a.js', isEntry: false },
    ];
    expect(treeOrder(modules)).toEqual([1, 2, 0]);
  });

  it('shows one page of modules and counts the rest', () => {
    const order = Array.from({ length: TREE_PAGE * 2 + 5 }, (_, at) => at);
    expect(shownModules(order, TREE_PAGE)).toEqual({ shown: order.slice(0, TREE_PAGE), more: TREE_PAGE + 5 });
    expect(shownModules(order, TREE_PAGE * 3)).toEqual({ shown: order, more: 0 });
  });

  it('grows the page to reach a module picked from outside it', () => {
    const order = Array.from({ length: TREE_PAGE * 3 }, (_, at) => TREE_PAGE * 3 - 1 - at);
    expect(limitFor(order, null, TREE_PAGE)).toBe(TREE_PAGE);
    expect(limitFor(order, order[10] ?? 0, TREE_PAGE)).toBe(TREE_PAGE);
    expect(limitFor(order, order[TREE_PAGE] ?? 0, TREE_PAGE)).toBe(TREE_PAGE * 2);
    expect(limitFor(order, order[TREE_PAGE * 2 + 7] ?? 0, TREE_PAGE)).toBe(TREE_PAGE * 3);
    expect(limitFor(order, 99_999, TREE_PAGE)).toBe(TREE_PAGE);
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

  it('writes modules whose paths are Object.prototype names, and refuses a second module at a taken path', () => {
    const bundle = {
      kind: 'webpack',
      modules: [
        { id: '0', path: 'toString', isEntry: true },
        { id: '1', path: 'constructor', isEntry: false },
        { id: '2', path: 'toString', isEntry: false },
        { id: '3', path: '__proto__', isEntry: false },
      ],
    };
    const plan = zipPlan('{}', bundle, ['a();', 'b();', 'c();', 'd();'], '');
    expect(Object.keys(plan.files).sort()).toEqual(['__proto__', 'bundle.json', 'constructor', 'deobfuscated.js', 'toString']);
    expect(Object.getPrototypeOf(plan.files)).toBeNull();
    expect(plan.refused).toEqual(['toString']);
  });
});
