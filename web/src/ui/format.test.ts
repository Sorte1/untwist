import { Text } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import { FORMAT_FLAGS, formatOutcome, keptLine } from './format';

const doc = (text: string) => Text.of(text.split('\n'));

describe('format document', () => {
  it('runs only the unminify stage, with no renaming', () => {
    expect(FORMAT_FLAGS.split(' ')).toEqual(['--no-unpack', '--no-deobfuscate', '--no-jsx', '--rename', 'none']);
  });

  it('takes the engine output when it exits cleanly', () => {
    expect(formatOutcome({ stdout: 'var a = true;\n', stderr: '', code: 0, trap: null })).toEqual({ text: 'var a = true;\n' });
  });

  it('refuses with the first line the engine printed on a parse error', () => {
    expect(formatOutcome({ stdout: '', stderr: '\nUnexpected token (1:7)\n\n', code: 1, trap: null })).toEqual({ refused: 'Unexpected token (1:7)' });
    expect(formatOutcome({ stdout: '', stderr: '', code: 2, trap: null })).toEqual({ refused: 'the engine exited with 2' });
  });

  it('refuses after a trap even when the refused re-run printed code', () => {
    expect(formatOutcome({ stdout: 'x;\n', stderr: '', code: 0, trap: 'stack' })).toEqual({ refused: 'the engine ran out of stack' });
  });

  it('keeps the cursor on its line number, clamped to the new text', () => {
    const before = doc('a\nb\nc\nd');
    const after = doc('a;\nb;');
    expect(keptLine(before, before.line(2).from + 1, after)).toBe(after.line(2).from);
    expect(keptLine(before, before.line(4).from, after)).toBe(after.line(2).from);
  });
});
