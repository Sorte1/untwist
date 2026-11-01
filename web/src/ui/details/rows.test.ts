import { describe, expect, it } from 'vitest';
import type { Report } from './report';
import { limitPairs, NOT_RECORDED, passesOf, renamePairs, warningClassPairs } from './rows';

const report: Report = {
  schema: 'untwist.report/1',
  engine: { version: '0.1.0', build: 'abc', nestingBudget: 1, shadowStackBytes: 1, callDepthCap: 2, evaluatorNestingCap: 3, stepBudget: 50_000_000 },
  stages: [
    {
      stage: 'deobfuscate', round: 1, iterations: 2, changes: 5,
      per_pass: [['find-string-array', 1], ['dead-code', 0]], per_pass_ms: [['find-string-array', 0]],
      fixpoint_per_pass: [], fixpoint_per_pass_ms: [], iterations_ms: [], elapsed_ms: 12, stop: 'converged',
    },
  ],
  notes: [],
  limits: { fired: [], nestingBudget: 1, fuelCap: 50_000_000, heapPeak: 0 },
  detection: { bundle: null, modules: 0, families: [] },
  sizes: { inputBytes: 1, outputBytes: 2 },
  stats: {
    decode: { steps: 1_234_567, peakUnitSteps: 1_000_000, units: 3, peakCallDepth: 17 },
    collisions: { boundInScope: 12, freeReference: 3, label: 0, excluded: 1 },
  },
};

describe('the details rows', () => {
  it('lists only the passes that changed something', () => {
    expect(passesOf(report.stages[0] as Report['stages'][number])).toEqual([['find-string-array', 1, 0]]);
  });

  it('explains each fired limit in plain words and shows an unlimited fuel as unlimited', () => {
    const fired = limitPairs({ ...report, limits: { ...report.limits, fired: ['eval-cap'], fuelCap: 18446744073709552000 } });
    expect(fired.find((pair) => pair.label === 'Fired')?.value).toBe('call depth cap');
    expect(fired.find((pair) => pair.label === '  call depth cap')?.value).toBe('fixed by the browser stack, not affected by the fuel setting');
    expect(fired.find((pair) => pair.label === 'Fuel cap')?.value).toBe('unlimited');
    expect(limitPairs(report).find((pair) => pair.label === 'Fuel cap')?.value).toBe('50,000,000 steps per decode unit');
  });

  it('shows the fuel used, the peak call depth and the collision reasons the engine records', () => {
    const limits = limitPairs(report);
    expect(limits.find((pair) => pair.label === 'Fuel used')?.value).toBe('1,234,567 steps over 3 decode units; at most 1,000,000 in one');
    expect(limits.find((pair) => pair.label === 'Evaluator peak call depth')?.value).toBe('17 (cap 2)');
    const rename = renamePairs(report);
    expect(rename.find((pair) => pair.label === 'Collision reasons')?.value).toBe('16 names skipped');
    expect(rename.find((pair) => pair.label === '  already bound in scope')?.value).toBe('12');
    expect(rename.find((pair) => pair.label === '  a free reference uses it')?.value).toBe('3');
    expect(rename.find((pair) => pair.label === '  reserved or kept')?.value).toBe('1');
    expect(rename.some((pair) => pair.label === '  a label uses it')).toBe(false);
  });

  it('says when nothing ran, and keeps an old report without stats as not recorded', () => {
    const idle = { ...report, stats: { decode: { steps: 0, peakUnitSteps: 0, units: 0, peakCallDepth: 0 }, collisions: { boundInScope: 0, freeReference: 0, label: 0, excluded: 0 } } };
    expect(limitPairs(idle).find((pair) => pair.label === 'Fuel used')?.value).toBe('no decode unit ran');
    expect(renamePairs(idle).find((pair) => pair.label === 'Collision reasons')?.value).toBe('none');
    const { stats: _, ...old } = report;
    expect(limitPairs(old).find((pair) => pair.label === 'Fuel used')?.value).toBe(NOT_RECORDED);
    expect(renamePairs(old).find((pair) => pair.label === 'Collision reasons')?.value).toBe(NOT_RECORDED);
  });

  it('counts the warnings per class, most first', () => {
    const rows = [
      { level: 'warning' as const, pass: 'find-string-array', message: 'a' },
      { level: 'info' as const, pass: 'deobfuscate', message: 'b' },
      { level: 'warning' as const, pass: 'unsafe', message: 'c' },
      { level: 'warning' as const, pass: 'find-string-array', message: 'd' },
      { level: 'error' as const, pass: 'parser', message: 'e' },
    ];
    expect(warningClassPairs(rows)).toEqual([
      { label: 'find-string-array', value: '2' },
      { label: 'unsafe', value: '1' },
    ]);
    expect(warningClassPairs([])).toEqual([{ label: 'Warnings', value: 'none' }]);
  });
});
