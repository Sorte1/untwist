import { describe, expect, it } from 'vitest';
import { diagnosticsOf, docPosition, levelLabel, lineColumn, lineIndex, shownMessage, stripLabel, warningCount } from './diagnostics-model';

describe('diagnosticsOf', () => {
  it('reads the report rows and drops malformed ones', () => {
    const report = JSON.stringify({
      diagnostics: [
        { level: 'error', pass: 'parser', message: 'Unexpected token (2:9)', inputSpan: [19, 19] },
        { level: 'loud', pass: 'x', message: 'y' },
        { level: 'info', pass: 'driver', message: 'z', inputSpan: [1] },
        { level: 'info', pass: 'driver', message: 'kept' },
      ],
    });
    const rows = diagnosticsOf(report, '');
    expect(rows.map((row) => row.message)).toEqual(['Unexpected token (2:9)', 'kept']);
    expect(warningCount(rows)).toBe(1);
  });

  it('falls back to the stderr lines as errors when the report has no rows', () => {
    const rows = diagnosticsOf('{"schema":"untwist.report/1"}', 'error: unsupported flag --x\n');
    expect(rows).toEqual([{ level: 'error', pass: 'flags', message: 'error: unsupported flag --x' }]);
    expect(diagnosticsOf('not json', 'a\n')).toHaveLength(1);
  });
});

describe('labels', () => {
  it('shortens the level and drops a level prefix the column already shows', () => {
    expect(levelLabel('warning')).toBe('warn');
    expect(shownMessage({ level: 'warning', pass: 'p', message: 'warning: input 1 was not processed' })).toBe('input 1 was not processed');
    expect(shownMessage({ level: 'info', pass: 'p', message: 'plain: text' })).toBe('plain: text');
  });
});

describe('positions', () => {
  it('maps a UTF-16 offset to a line and a code-point column', () => {
    const index = lineIndex('ab\n😀c\n\nd');
    expect(lineColumn(index, 0)).toEqual({ line: 1, column: 1 });
    expect(lineColumn(index, 3)).toEqual({ line: 2, column: 1 });
    expect(lineColumn(index, 5)).toEqual({ line: 2, column: 2 });
    expect(lineColumn(index, 7)).toEqual({ line: 3, column: 1 });
    expect(lineColumn(index, 8)).toEqual({ line: 4, column: 1 });
  });

  it('moves an offset past each CRLF the editor joins into one line break', () => {
    const text = 'a\r\nb\r\nc';
    expect(docPosition(text, 0)).toBe(0);
    expect(docPosition(text, 3)).toBe(2);
    expect(docPosition(text, 6)).toBe(4);
    expect(docPosition('a\nb', 2)).toBe(2);
  });
});

describe('stripLabel', () => {
  const row = (level: 'error' | 'warning' | 'info') => ({ level, pass: 'p', message: 'm' });

  it('names the diagnostics before a run, with the link notes when there are some', () => {
    expect(stripLabel(null, 0)).toEqual({ text: 'Diagnostics', tone: 'text' });
    expect(stripLabel(null, 2)).toEqual({ text: '2 notes', tone: 'text' });
  });

  it('counts errors first, then warnings, then notes', () => {
    expect(stripLabel([row('error'), row('warning'), row('warning')], 0)).toEqual({ text: '1 error, 2 warnings', tone: 'err' });
    expect(stripLabel([row('warning'), row('info')], 0)).toEqual({ text: '1 warning', tone: 'warn' });
    expect(stripLabel([row('info')], 1)).toEqual({ text: '2 notes', tone: 'text' });
    expect(stripLabel([], 0)).toEqual({ text: 'No warnings', tone: 'muted' });
  });
});
