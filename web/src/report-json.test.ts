import { describe, expect, it } from 'vitest';
import { reportField, reportValue } from './report-json';

describe('reportValue', () => {
  it('parses a report once and hands back the same value for the same text', () => {
    const json = JSON.stringify({ stages: [], bundle: { kind: 'webpack', modules: [] } });
    const first = reportValue(json);
    expect(first).toEqual({ stages: [], bundle: { kind: 'webpack', modules: [] } });
    expect(reportValue(json)).toBe(first);
    expect(reportValue(`${json}`)).toBe(first);
  });

  it('keeps a few recent reports, so alternating between two parses neither again', () => {
    const a = '{"a":1}';
    const b = '{"b":2}';
    const first = reportValue(a);
    const second = reportValue(b);
    expect(reportValue(a)).toBe(first);
    expect(reportValue(b)).toBe(second);
  });

  it('gives undefined for text that is not JSON', () => {
    expect(reportValue('not json')).toBeUndefined();
    expect(reportValue('')).toBeUndefined();
  });
});

describe('reportField', () => {
  it('reads one own field of the report and nothing from other values', () => {
    expect(reportField('{"notes":[1]}', 'notes')).toEqual([1]);
    expect(reportField('{"notes":[1]}', 'stages')).toBeUndefined();
    expect(reportField('{"notes":[1]}', 'constructor')).toBeUndefined();
    expect(reportField('null', 'notes')).toBeUndefined();
    expect(reportField('[1]', 'length')).toBeUndefined();
  });
});
