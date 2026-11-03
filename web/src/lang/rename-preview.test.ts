import { EditorState } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { previewLines, previewNote } from './rename-preview';

const state = EditorState.create({ doc: 'function f() {\n  let sum = 0;\n  sum += sum;\n  return sum;\n}' });
const at = (needle: string, nth = 0) => {
  let index = -1;
  for (let i = 0; i <= nth; i += 1) {
    index = state.doc.toString().indexOf(needle, index + 1);
  }
  return index;
};

describe('the rename preview', () => {
  it('groups the edits by line and marks the new name', () => {
    const edits = [at('sum', 3), at('sum', 0), at('sum', 1), at('sum', 2)].map((from) => [from, from + 3, 'total'] as const);
    const lines = previewLines(state, edits);
    expect(lines.map((line) => line.line)).toEqual([2, 3, 4]);
    expect(lines[0]?.parts).toEqual([
      { text: 'let ', changed: false },
      { text: 'total', changed: true },
      { text: ' = 0;', changed: false },
    ]);
    expect(lines[1]?.parts.filter((part) => part.changed)).toHaveLength(2);
  });

  it('says how many references on how many lines', () => {
    expect(previewNote(4, 3)).toBe('4 references on 3 lines · Enter to rename');
    expect(previewNote(1, 1)).toBe('1 reference on 1 line · Enter to rename');
  });

  it('starts a long line near its first edit', () => {
    const long = EditorState.create({ doc: `${'x'.repeat(100)} sum;` });
    const [line] = previewLines(long, [[101, 104, 'total']]);
    expect(line?.parts[0]).toEqual({ text: '…', changed: false });
    expect(line?.parts.find((part) => part.changed)?.text).toBe('total');
  });
});
