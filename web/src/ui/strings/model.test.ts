import { describe, expect, it } from 'vitest';
import { decodedStrings, matching, quoted } from './model';

const SOURCE = "var x = b(0x0);\nfoo(b(0x1), b(0x0));";
const at = (needle: string, from = 0): [number, number] => {
  const start = SOURCE.indexOf(needle, from);
  return [start, start + needle.length];
};

const table = (rows: unknown[], more = 0) => JSON.stringify({ rows, more });

describe('the decoded strings table', () => {
  it('lists each decoded call site with its call, value and place in the input', () => {
    const first = at('b(0x0)');
    const second = at('b(0x1)');
    const third = at('b(0x0)', 20);
    const found = decodedStrings(table([[...first, 'He'], [...second, 'llo'], [...third, 'He']]), SOURCE);
    expect(found.rows).toEqual([
      { call: 'b(0x0)', value: 'He', place: '1:9', span: first },
      { call: 'b(0x1)', value: 'llo', place: '2:5', span: second },
      { call: 'b(0x0)', value: 'He', place: '2:13', span: third },
    ]);
    expect(found.distinct).toBe(2);
    expect(found.more).toBe(0);
  });

  it('keeps a site outside the input, without a call or a place', () => {
    const found = decodedStrings(table([[null, null, 'x']], 4), SOURCE);
    expect(found.rows).toEqual([{ call: '', value: 'x', place: '', span: undefined }]);
    expect(found.more).toBe(4);
  });

  it('cuts a long call to one readable line', () => {
    const text = `${'a'.repeat(100)}\nb`;
    expect(decodedStrings(table([[0, text.length, 'v']]), text).rows[0]?.call).toBe(`${'a'.repeat(60)}…`);
  });

  it('reads nothing from an empty or broken table', () => {
    expect(decodedStrings('', SOURCE)).toEqual({ rows: [], distinct: 0, more: 0 });
    expect(decodedStrings('{', SOURCE)).toEqual({ rows: [], distinct: 0, more: 0 });
  });

  it('finds rows by value or call, ignoring case', () => {
    const rows = decodedStrings(table([[...at('b(0x0)'), 'Hello'], [...at('b(0x1)'), 'world']]), SOURCE).rows;
    expect(matching(rows, 'hel').map((row) => row.value)).toEqual(['Hello']);
    expect(matching(rows, '0X1').map((row) => row.value)).toEqual(['world']);
    expect(matching(rows, '  ')).toBe(rows);
  });

  it('shows a value as a quoted string literal', () => {
    expect(quoted('a"b\n\t\\')).toBe('"a\\"b\\n\\t\\\\"');
    expect(quoted('\u0001')).toBe('"\\u0001"');
  });
});
