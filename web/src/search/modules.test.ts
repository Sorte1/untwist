import { describe, expect, it } from 'vitest';
import { mappedRange, nearestMatch, searchModules, snippetOf } from './modules';

const MODULES = ['const a = require("./b");\nexport default a;', 'function Foo() {}\nfoo();\nFOO = 1;', 'nothing here'];

describe('searchModules', () => {
  it('finds every match, case-insensitively, in the order given, with its line and column', () => {
    const found = searchModules(MODULES, [1, 0, 2], 'foo');
    expect(found.capped).toBe(false);
    expect(found.hits.map((hit) => [hit.module, hit.line, hit.column, hit.from, hit.to])).toEqual([
      [1, 1, 10, 9, 12],
      [1, 2, 1, 18, 21],
      [1, 3, 1, 25, 28],
    ]);
  });

  it('treats the query as plain text', () => {
    expect(searchModules(['a.b axb (x)'], [0], '.').hits).toHaveLength(1);
    expect(searchModules(['a.b axb (x)'], [0], '(x)').hits.map((hit) => hit.from)).toEqual([8]);
  });

  it('finds nothing for an empty or blank query', () => {
    expect(searchModules(MODULES, [0, 1, 2], '').hits).toEqual([]);
    expect(searchModules(MODULES, [0, 1, 2], '   ').hits).toEqual([]);
  });

  it('stops at the limit and says so', () => {
    const found = searchModules(['e'.repeat(50), 'e'.repeat(50)], [0, 1], 'e', 60);
    expect(found.hits).toHaveLength(60);
    expect(found.capped).toBe(true);
    expect(found.hits.at(-1)?.module).toBe(1);
  });

  it('counts the modules with a match', () => {
    expect(searchModules(MODULES, [0, 1, 2], 'e').modules).toBe(2);
  });
});

describe('snippetOf', () => {
  it('cuts the line around the match', () => {
    const text = 'first line\n    const value = someLongFunctionName(argumentOne, argumentTwo, argumentThree, argumentFour);\nlast';
    const from = text.indexOf('someLong');
    const snip = snippetOf(text, from, from + 8);
    expect(snip.match).toBe('someLong');
    expect(snip.before).toBe('const value = ');
    expect(snip.after.startsWith('FunctionName(argumentOne')).toBe(true);
    expect(snip.after.endsWith('…')).toBe(true);
  });

  it('elides a long start of line', () => {
    const text = `${'x'.repeat(60)}needle`;
    const snip = snippetOf(text, 60, 66);
    expect(snip.before.startsWith('…')).toBe(true);
    expect(snip.before.length).toBeLessThanOrEqual(25);
    expect(snip.after).toBe('');
  });
});

describe('nearestMatch', () => {
  it('finds the occurrence closest to a position', () => {
    const text = 'foo bar foo baz foo';
    expect(nearestMatch(text, 'foo', 9)).toEqual({ from: 8, to: 11 });
    expect(nearestMatch(text, 'foo', 0)).toEqual({ from: 0, to: 3 });
    expect(nearestMatch(text, 'foo', 30)).toEqual({ from: 16, to: 19 });
  });

  it('gives null when the text does not hold the needle', () => {
    expect(nearestMatch('bar', 'foo', 0)).toBeNull();
    expect(nearestMatch('bar', '', 0)).toBeNull();
  });
});

describe('mappedRange', () => {
  const original = 'foo bar foo baz foo';

  it('keeps the range of an unedited text and of a match before the edit', () => {
    expect(mappedRange(original, original, { from: 8, to: 11 })).toEqual({ from: 8, to: 11 });
    expect(mappedRange(original, `${original}\n// added`, { from: 8, to: 11 })).toEqual({ from: 8, to: 11 });
  });

  it('shifts a match after the edit by what the edit added or removed', () => {
    expect(mappedRange(original, `// added\n${original}`, { from: 8, to: 11 })).toEqual({ from: 17, to: 20 });
    expect(mappedRange(original, original.slice(4), { from: 8, to: 11 })).toEqual({ from: 4, to: 7 });
  });

  it('falls back to the nearest same text for a match inside the edit, and to a caret without one', () => {
    expect(mappedRange(original, 'foo bar  foo baz foo', { from: 8, to: 11 })).toEqual({ from: 9, to: 12 });
    expect(mappedRange('abc require def', 'abc def', { from: 4, to: 11 })).toEqual({ from: 4, to: 4 });
  });
});
