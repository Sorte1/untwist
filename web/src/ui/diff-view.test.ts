import { describe, expect, it } from 'vitest';
import type { RunResult } from '../engine/types';
import { initialState, inputOf, type AppState } from '../state';
import { BIG_DIFF_CAP_BYTES, BIG_DIFF_HUNK_CAP, BIG_DIFF_LINE_CAP, bigView, DIFF_CAP_BYTES, DIFF_CAP_LINES, diffPlan, hunkLabel } from './diff-view';

const report = JSON.stringify({
  bundle: { kind: 'webpack', modules: [{ id: '1', path: 'a.js', isEntry: true }, { id: '2', path: 'b.js', isEntry: false, span: [4, 9] }] },
});

function withResult(result: Partial<RunResult>, selected: number | null, input = 'var a = !0;'): AppState {
  const base = initialState('dark');
  const full: RunResult = {
    stdout: 'var a = true;\n', stderr: '', code: 0, limits: '', report: '{}', bundleJson: null, modules: [],
    trap: null, engineMs: 1, memoryBytes: 1, workerMs: 1, ...result,
  };
  return { ...base, input: inputOf(input, 'x.js', 1), selectedModule: selected, run: { ...base.run, status: 'done', result: full } };
}

describe('the diff plan', () => {
  it('compares the input with the output when there is no bundle', () => {
    expect(diffPlan(withResult({}, null))).toMatchObject({ kind: 'diff', before: 'var a = !0;', after: 'var a = true;\n' });
  });

  it('takes a module span as a UTF-16 slice of stdout, and notes a module without one', () => {
    const state = withResult({ report, modules: ['a();', 'b();'], stdout: '0123456789abc' }, 1);
    expect(diffPlan(state)).toMatchObject({ kind: 'diff', before: '45678', after: 'b();', note: null });
    expect(diffPlan({ ...state, selectedModule: 0 })).toMatchObject({ kind: 'diff', before: '0123456789abc', note: expect.stringContaining('no span') });
  });

  it('sends sides over the in-page caps to the worker diff', () => {
    expect(diffPlan(withResult({}, null, 'a\n'.repeat(DIFF_CAP_LINES + 1))).kind).toBe('big');
    expect(diffPlan(withResult({ stdout: 'b'.repeat(DIFF_CAP_BYTES + 1) }, null)).kind).toBe('big');
  });

  it('turns the diff off only above the worker cap', () => {
    const plan = diffPlan(withResult({ stdout: 'b'.repeat(BIG_DIFF_CAP_BYTES + 1) }, null));
    expect(plan).toMatchObject({ kind: 'off', note: expect.stringContaining('16 MB') });
  });
});

describe('the worker diff view', () => {
  const result = (hunks: number, longestLine = 80) => ({ ranges: new Int32Array(hunks * 4), lineRanges: new Int32Array(hunks * 4), exact: true, linesA: 10, linesB: 10, longestLine, ms: 1 });

  it('shows side by side when the hunks and lines fit', () => {
    expect(bigView(result(3))).toEqual({ kind: 'merge' });
    expect(bigView(result(3, 200_000))).toEqual({ kind: 'merge' });
  });

  it('falls back to a hunk summary for too many hunks or an over-long line', () => {
    expect(bigView(result(BIG_DIFF_HUNK_CAP + 1))).toMatchObject({ kind: 'summary', note: expect.stringContaining('changes') });
    expect(bigView(result(1, BIG_DIFF_LINE_CAP + 1))).toMatchObject({ kind: 'summary', note: expect.stringContaining('line') });
  });

  it('labels a hunk with its line ranges and counts', () => {
    expect(hunkLabel({ fromA: 3, toA: 5, fromB: 3, toB: 4 })).toBe('lines 4–5 → 4: −2 +1');
    expect(hunkLabel({ fromA: 7, toA: 7, fromB: 6, toB: 9 })).toBe('after line 7 → 7–9: +3');
    expect(hunkLabel({ fromA: 0, toA: 1, fromB: 0, toB: 0 })).toBe('line 1 → removed: −1');
  });
});
