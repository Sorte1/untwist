import { describe, expect, it } from 'vitest';
import { globals, memberBuiltin, namespaceMembers, parseBuiltins, prototypeMembers, signatureOf, splitParams } from './builtins';

describe('the built-in table', () => {
  it('splits parameters at top-level commas only', () => {
    expect(splitParams('a, (b, c) => d, e?')).toEqual(['a', '(b, c) => d', 'e?']);
    expect(splitParams('')).toEqual([]);
  });

  it('parses functions, constructors, namespaces and values', () => {
    const [fn, ctor, ns, value] = parseBuiltins('parseInt(string, radix?): number | Parses.\nnew Map(entries?) | A map.\nMath | Maths.\nNaN: number | Not a number.', false);
    expect(fn).toEqual({ name: 'parseInt', kind: 'function', params: ['string', 'radix?'], returns: 'number', doc: 'Parses.' });
    expect(ctor).toMatchObject({ name: 'Map', kind: 'class', params: ['entries?'] });
    expect(ns).toMatchObject({ name: 'Math', kind: 'namespace', params: null });
    expect(value).toMatchObject({ name: 'NaN', kind: 'property', returns: 'number' });
  });

  it('writes signatures the way the docs show them', () => {
    const max = namespaceMembers('Math').find((member) => member.name === 'max');
    expect(max === undefined ? '' : signatureOf(max, 'Math')).toBe('Math.max(...values): number');
    const map = globals().get('Map');
    expect(map === undefined ? '' : signatureOf(map)).toBe('new Map(entries?)');
  });

  it('has documented globals', () => {
    for (const name of ['console', 'parseInt', 'setTimeout', 'Promise', 'JSON', 'document', 'require']) {
      expect(globals().get(name)?.doc, name).toBeTruthy();
    }
  });

  it('offers the prototype of a known form, then the object methods', () => {
    const names = prototypeMembers('string').map((member) => member.name);
    expect(names).toContain('slice');
    expect(names).toContain('hasOwnProperty');
    expect(prototypeMembers('new Map').map((member) => member.name)).toContain('get');
    expect(prototypeMembers('object').map((member) => member.name)).toEqual(['hasOwnProperty', 'toString', 'valueOf', 'isPrototypeOf', 'propertyIsEnumerable']);
    expect(prototypeMembers(undefined)).toEqual([]);
    expect(prototypeMembers('new Widget')).toEqual([]);
  });

  it('looks a member up on a namespace or on a form', () => {
    expect(memberBuiltin({ name: 'JSON', global: true, form: undefined }, 'parse')?.params).toEqual(['text', 'reviver?']);
    expect(memberBuiltin({ name: 'text', global: false, form: 'string' }, 'split')?.kind).toBe('method');
    expect(memberBuiltin({ name: 'Math', global: false, form: undefined }, 'max')).toBeNull();
  });

  it('parses every line of the data', () => {
    expect(globals().size).toBeGreaterThan(60);
    for (const name of ['console', 'Math', 'JSON', 'Object', 'Array', 'String', 'Number', 'Promise', 'Reflect', 'Symbol', 'Date', 'document', 'module']) {
      expect(namespaceMembers(name).length, name).toBeGreaterThan(0);
    }
  });
});
