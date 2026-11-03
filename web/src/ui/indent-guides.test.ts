import { Text } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { detectUnit, indentLevels, leadingColumns, MAX_LEVEL } from './indent-guides';

const doc = (text: string) => Text.of(text.split('\n'));

describe('indent guides', () => {
  it('measure leading whitespace in columns, with tabs to the next stop', () => {
    expect(leadingColumns('    x', 4)).toBe(4);
    expect(leadingColumns('\tx', 4)).toBe(4);
    expect(leadingColumns('  \tx', 4)).toBe(4);
    expect(leadingColumns('x', 4)).toBe(0);
    expect(leadingColumns('   ', 4)).toBeNull();
    expect(leadingColumns('', 4)).toBeNull();
  });

  it('detect the indent unit from the smallest step in', () => {
    expect(detectUnit(doc('a {\n  b {\n    c;\n  }\n}'), 4)).toBe(2);
    expect(detectUnit(doc('a {\n    b;\n}'), 4)).toBe(4);
    expect(detectUnit(doc('a {\n\tb;\n}'), 4)).toBe(4);
    expect(detectUnit(doc('flat;\nlines;'), 4)).toBe(8);
  });

  it('give each line its depth and a blank line the depth of its block', () => {
    const text = doc('function f() {\n  if (a) {\n    b();\n\n  }\n\n  c();\n}');
    expect(indentLevels(text, 1, text.lines, 2, 4)).toEqual([0, 1, 2, 2, 1, 1, 1, 0]);
  });

  it('look past the window for the lines around a blank edge', () => {
    const text = doc('{\n  a;\n\n  b;\n}');
    expect(indentLevels(text, 3, 3, 2, 4)).toEqual([1]);
  });

  it('stop at the deepest level they draw', () => {
    const deep = doc(`${' '.repeat(2 * (MAX_LEVEL + 5))}x`);
    expect(indentLevels(deep, 1, 1, 2, 4)).toEqual([MAX_LEVEL]);
  });
});
