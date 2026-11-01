import { describe, expect, it } from 'vitest';
import { chipLabel, summaryOf } from './summary-chip';

describe('the run summary chip', () => {
  it('counts the stages, sums their iterations and their elapsed time', () => {
    const report = JSON.stringify({ stages: [{ iterations: 3, elapsed_ms: 1200 }, { iterations: 9, elapsed_ms: 640 }] });
    const summary = summaryOf(report);
    expect(summary).toEqual({ stages: 2, rounds: 12, totalMs: 1840 });
    expect(chipLabel(summary ?? { stages: 0, rounds: 0, totalMs: 0 })).toBe('2 stages · 12 rounds · 1.84 s');
  });

  it('gives nothing for a report without stages', () => {
    expect(summaryOf('{"schema":"untwist.report/1"}')).toBeNull();
    expect(summaryOf('not json')).toBeNull();
  });
});
