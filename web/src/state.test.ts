import { describe, expect, it } from 'vitest';
import type { RunResult } from './engine/types';
import {
  exitLabel,
  initialState,
  inputOf,
  limitsLabel,
  limitsTitle,
  runCancelled,
  runFailed,
  runFinished,
  runStarted,
  skippedLabel,
  stageBegan,
  statusLabel,
  ticked,
  utf8Bytes,
  type AppState,
} from './state';

const result = (overrides: Partial<RunResult> = {}): RunResult => ({
  stdout: 'var a = true;\n',
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
  ...overrides,
});

const ready = (): AppState => ({ ...initialState('dark'), engine: 'ready' });

describe('the stages of a run', () => {
  it('keeps each stage the worker announces while the run goes, and starts empty', () => {
    const running = runStarted(ready(), 1000);
    expect(running.run.stages).toEqual([]);
    const staged = stageBegan(stageBegan(running, 'prepare', 1010), 'deobfuscate', 1200);
    expect(staged.run.stages).toEqual([
      { name: 'prepare', at: 1010 },
      { name: 'deobfuscate', at: 1200 },
    ]);
    expect(runStarted(runFinished(staged, result(), 1300), 2000).run.stages).toEqual([]);
  });

  it('ignores a stage that arrives after the run ended', () => {
    const done = runFinished(runStarted(ready(), 1000), result(), 1100);
    expect(stageBegan(done, 'unpack', 1200)).toBe(done);
  });
});

describe('the run states', () => {
  it('goes idle → running → done, with the elapsed time and the exit code', () => {
    const running = runStarted(ready(), 1000);
    expect(running.run.status).toBe('running');
    expect(exitLabel(running.run)).toBe('…');
    expect(ticked(running, 1250).run.elapsedMs).toBe(250);
    const done = runFinished(running, result(), 2840);
    expect(done.run.status).toBe('done');
    expect(done.run.elapsedMs).toBe(1840);
    expect(statusLabel(done)).toBe('Done');
    expect(exitLabel(done.run)).toBe('0');
    expect(limitsLabel(done.run)).toBe('none fired');
  });

  it('stops by limit when limits fired, when the exit code is 3, or on a stack trap', () => {
    const running = runStarted(ready(), 0);
    expect(runFinished(running, result({ limits: 'refused,budget', code: 3 }), 1).run.status).toBe('limit');
    expect(runFinished(running, result({ code: 3 }), 1).run.status).toBe('limit');
    const trapped = runFinished(running, result({ trap: 'stack', limits: 'refused', code: 3 }), 1);
    expect(statusLabel(trapped)).toBe('Stopped by limit (stack)');
    expect(limitsLabel(trapped.run)).toBe('depth refusal, stack trap');
    expect(limitsTitle(trapped.run)).toMatch(/^depth refusal — .*not affected by the fuel setting\nstack trap — /);
  });

  it('finishes as Done with a skipped-step chip when only an optional step was skipped', () => {
    const running = runStarted(ready(), 0);
    for (const limits of ['unpack-text', 'eval-cap', 'budget', 'unpack-text,eval-cap']) {
      const done = runFinished(running, result({ limits }), 1);
      expect(done.run.status, limits).toBe('done');
      expect(statusLabel(done)).toBe('Done');
      expect(skippedLabel(done.run)).toMatch(/^Skipped: /);
    }
    const unpack = runFinished(running, result({ limits: 'unpack-text' }), 1);
    expect(skippedLabel(unpack.run)).toBe('Skipped: unpack text depth');
    expect(skippedLabel(runFinished(running, result(), 1).run)).toBe('');
    expect(runFinished(running, result({ limits: 'refused' }), 1).run.status).toBe('limit');
    expect(runFinished(running, result({ limits: 'unpack-text', code: 3 }), 1).run.status).toBe('limit');
    expect(skippedLabel(runFinished(running, result({ limits: 'refused', code: 3 }), 1).run)).toBe('');
  });

  it('cancels into "killed" and keeps the last result', () => {
    const first = runFinished(runStarted(ready(), 0), result(), 10);
    const cancelled = runCancelled(runStarted(first, 20), 520);
    expect(cancelled.run.status).toBe('cancelled');
    expect(exitLabel(cancelled.run)).toBe('killed');
    expect(limitsLabel(cancelled.run)).toBe('—');
    expect(cancelled.run.result).toBe(first.run.result);
    expect(cancelled.run.elapsedMs).toBe(500);
  });

  it('remembers the flags that produced the shown output, across a cancelled or failed run', () => {
    expect(ready().run.outputFlags).toBeNull();
    const first = runFinished(runStarted(ready(), 0, '--unsafe'), result(), 10);
    expect(first.run.outputFlags).toBe('--unsafe');
    const restarted = runStarted(first, 20, '');
    expect(restarted.run.outputFlags).toBe('--unsafe');
    expect(runCancelled(restarted, 30).run.outputFlags).toBe('--unsafe');
    expect(runFailed(restarted, 'x', 30).run.outputFlags).toBe('--unsafe');
    expect(runFinished(restarted, result(), 30).run.outputFlags).toBe('');
  });

  it('ignores a finish or cancel when nothing runs, and a second start while running', () => {
    const idle = ready();
    expect(runFinished(idle, result(), 5)).toBe(idle);
    expect(runCancelled(idle, 5)).toBe(idle);
    expect(runFailed(idle, 'x', 5)).toBe(idle);
    const running = runStarted(idle, 1);
    expect(runStarted(running, 2)).toBe(running);
  });

  it('labels the engine while it loads or fails', () => {
    expect(statusLabel(initialState('dark'))).toBe('Loading engine');
    expect(statusLabel({ ...initialState('dark'), engine: 'failed' })).toBe('Engine failed to load');
  });
});

describe('large inputs', () => {
  it('counts UTF-8 bytes, including astral characters', () => {
    expect(utf8Bytes('aé€😀')).toBe(1 + 2 + 3 + 4);
  });

  it('marks text over 1 MiB or with a line over 20,000 characters as large', () => {
    expect(inputOf('a\n'.repeat(1000), 'x.js', 1).large).toBe(false);
    expect(inputOf('a'.repeat(20_001), 'x.js', 1).large).toBe(true);
    expect(inputOf('a\n'.repeat(600_000), 'x.js', 1).large).toBe(true);
    expect(inputOf('a\nb', 'x.js', 1).lines).toBe(2);
  });
});
