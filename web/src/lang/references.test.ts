import { EditorState } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { countLabel, rowText, stepped } from './references';

describe('the references panel', () => {
  it('shows a row as its line number and the trimmed line around the match', () => {
    const state = EditorState.create({ doc: 'a;\n    let sum = 0;\n' });
    const from = state.doc.toString().indexOf('sum');
    expect(rowText(state, { from, to: from + 3 })).toEqual({ line: 2, before: 'let ', match: 'sum', after: ' = 0;' });
  });

  it('cuts the start of a long line before the match', () => {
    const state = EditorState.create({ doc: `${'y'.repeat(200)}sum` });
    const row = rowText(state, { from: 200, to: 203 });
    expect(row.before.startsWith('…')).toBe(true);
    expect(row.match).toBe('sum');
  });

  it('counts and steps round the list', () => {
    expect(countLabel(1, 'x')).toBe('1 reference to x');
    expect(countLabel(1234, 'x')).toBe('1,234 references to x');
    const found = { name: 'x', refs: [{ from: 0, to: 1 }, { from: 2, to: 3 }, { from: 4, to: 5 }], selected: 2 };
    expect(stepped(found, 1)).toBe(0);
    expect(stepped({ ...found, selected: 0 }, -1)).toBe(2);
  });
});
