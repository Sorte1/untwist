import { describe, expect, it } from 'vitest';
import type { RunResult } from './engine/types';
import {
  canCompare,
  compareCancelled,
  compareFailed,
  compareFinished,
  compareLabel,
  comparePlan,
  compareStarted,
  otherModeFlags,
} from './compare';
import { initialState, runFinished, runStarted, type AppState } from './state';

const result = (stdout: string): RunResult => ({
  stdout,
  stderr: '',
  code: 0,
  limits: '',
  report: '{}',
  bundleJson: null,
  modules: [],
  trap: null,
  engineMs: 1,
  memoryBytes: 1,
  workerMs: 2,
});

function ran(flags: string, stdout = 'safe;\n'): AppState {
  const ready: AppState = { ...initialState('dark'), engine: 'ready' };
  return runFinished(runStarted(ready, 0, flags), result(stdout), 10);
}

describe('compare Safe with Unsafe', () => {
  it('flips only the unsafe flag of the flags that produced the output', () => {
    expect(otherModeFlags('')).toBe('--unsafe');
    expect(otherModeFlags('--no-unpack --rename unique')).toBe('--no-unpack --rename unique --unsafe');
    expect(otherModeFlags('--no-jsx --unsafe --single-file')).toBe('--no-jsx --single-file');
    expect(otherModeFlags('--unsafe')).toBe('');
  });

  it('is offered only for a finished output while nothing runs', () => {
    const ready: AppState = { ...initialState('dark'), engine: 'ready' };
    expect(canCompare(ready)).toBe(false);
    const done = ran('');
    expect(canCompare(done)).toBe(true);
    expect(canCompare(runStarted(done, 20, ''))).toBe(false);
    expect(canCompare(compareStarted(done))).toBe(false);
    expect(canCompare({ ...done, engine: 'loading' })).toBe(false);
  });

  it('labels the action by the mode the output was not made in', () => {
    expect(compareLabel(ran(''))).toBe('Compare with Unsafe');
    expect(compareLabel(ran('--unsafe'))).toBe('Compare with Safe');
    expect(compareLabel(compareStarted(ran('')))).toBe('Comparing…');
  });

  it('runs the other mode on the input that produced the output and opens the diff with Safe on the left', () => {
    const done = ran('--rename simple', 'safe;\n');
    const started = compareStarted(done);
    expect(started.compare.status).toBe('running');
    expect(started.compare.flags).toBe('--rename simple --unsafe');
    expect(started.compare.source).toBe(done.run.outputInput?.text);
    const finished = compareFinished(started, done.run.result, result('unsafe;\n'));
    expect(finished.compare.status).toBe('done');
    expect(finished.settings.view).toBe('diff');
    expect(comparePlan(finished)).toEqual({ safe: 'safe;\n', unsafe: 'unsafe;\n' });
    const reversed = compareStarted(ran('--unsafe', 'unsafe;\n'));
    const back = compareFinished(reversed, reversed.compare.base, result('safe;\n'));
    expect(comparePlan(back)).toEqual({ safe: 'safe;\n', unsafe: 'unsafe;\n' });
  });

  it('drops a comparison once a new output replaces the one it was made for', () => {
    const started = compareStarted(ran(''));
    const finished = compareFinished(started, started.compare.base, result('unsafe;\n'));
    const rerun = runFinished(runStarted(finished, 20, ''), result('safe again;\n'), 30);
    expect(comparePlan(rerun)).toBeNull();
    expect(compareFinished(started, result('other;\n'), result('x'))).toBe(started);
  });

  it('settles a failed or cancelled comparison without a plan', () => {
    const started = compareStarted(ran(''));
    const failed = compareFailed(started, started.compare.base, 'boom');
    expect(failed.compare.status).toBe('failed');
    expect(failed.compare.error).toBe('boom');
    expect(comparePlan(failed)).toBeNull();
    expect(compareCancelled(started).compare.status).toBe('idle');
    expect(compareCancelled(ran(''))).toEqual(ran(''));
  });
});
