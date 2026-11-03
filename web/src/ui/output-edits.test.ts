import { ChangeSet, Text } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { editedLines, editMark } from './output-edits';

const doc = (text: string) => Text.of(text.split('\n'));

describe('the output edit mark', () => {
  it('counts each changed line once', () => {
    const before = doc('let sum = 0;\nsum += 1;\nreturn sum;');
    const changes = ChangeSet.of(
      [
        { from: 4, to: 7, insert: 'total' },
        { from: 13, to: 16, insert: 'total' },
        { from: 30, to: 33, insert: 'total' },
      ],
      before.length,
    );
    const after = changes.apply(before);
    expect(editedLines(changes, after)).toBe(3);
    expect(editMark(changes, after)).toBe('Edited · 3 lines');
  });

  it('says nothing while the output is as the run left it', () => {
    const empty = ChangeSet.empty(5);
    expect(editedLines(empty, doc('a = 1'))).toBe(0);
    expect(editMark(empty, doc('a = 1'))).toBe('');
  });

  it('still marks an edit that only removed text', () => {
    const before = doc('a;\nb;');
    const changes = ChangeSet.of([{ from: 2, to: 5 }], before.length);
    expect(editMark(changes, changes.apply(before))).toBe('Edited · 1 line');
  });
});
