import { describe, expect, it } from 'vitest';
import type { RunResult } from './engine/types';
import { initialState, type AppState } from './state';
import { DEFAULTS } from './settings/settings';
import { firstChange, NO_STEPS, stepSides, stepsFailed, stepsFinished, stepsOf, stepsStarted, wantsSteps, type Step } from './steps';

const snapshots = [
  { stage: 'prepare', round: 1, layer: 0, kept: 'text' },
  { stage: 'deobfuscate', round: 1, layer: 0, kept: 'text' },
  { stage: 'unminify', round: 1, layer: 0, kept: 'unchanged' },
  { stage: 'deobfuscate', round: 2, layer: 0, kept: 'over-cap' },
  { stage: 'cleanup', round: 1, layer: 1, kept: 'unchanged' },
];
const stages = [
  { stage: 'prepare', round: 1, changes: 0 },
  { stage: 'deobfuscate', round: 1, changes: 9 },
  { stage: 'unminify', round: 1, changes: 0 },
  { stage: 'deobfuscate', round: 2, changes: 1 },
];

const result = (overrides: Partial<RunResult> = {}): RunResult => ({
  stdout: 'out;\n',
  stderr: '',
  code: 0,
  limits: '',
  report: JSON.stringify({ stages, snapshots }),
  bundleJson: null,
  modules: [],
  trap: null,
  engineMs: 1,
  memoryBytes: 1,
  workerMs: 1,
  snapshots: ['a;', 'b;', null, null, null],
  ...overrides,
});

function ran(base: RunResult, view: AppState['settings']['view'] = 'steps'): AppState {
  const state = initialState('dark', { ...DEFAULTS, view });
  const input = { text: 'in;', name: 'x.js', bytes: 3, lines: 1, large: false, revision: 1 };
  return { ...state, engine: 'ready', run: { ...state.run, status: 'done', result: base, outputInput: input, outputFlags: '' } };
}

describe('the pass steps', () => {
  it('run from the input through each recorded stage to the output', () => {
    const steps = stepsOf('in;', result());
    expect(steps.map((step) => [step.label, step.changes, step.code])).toEqual([
      ['Input', null, 'in;'],
      ['prepare', 0, 'a;'],
      ['deobfuscate', 9, 'b;'],
      ['unminify', 0, 'b;'],
      ['deobfuscate (round 2)', 1, null],
      ['cleanup · layer 2', null, null],
      ['Output', null, 'out;\n'],
    ]);
  });

  it('start at the first step that changed the code', () => {
    const same = (code: string): Step => ({ label: code, changes: 0, code });
    expect(firstChange([same('a'), same('a'), same('b')])).toBe(2);
    expect(firstChange([same('a'), same('a')])).toBe(1);
  });

  it('record only for the steps view, a finished run and a result not recorded yet', () => {
    const base = result();
    expect(wantsSteps(ran(base))).toBe(true);
    expect(wantsSteps(ran(base, 'diff'))).toBe(false);
    const started = stepsStarted(ran(base));
    expect(started.steps).toMatchObject({ status: 'running', base });
    expect(wantsSteps(started)).toBe(false);
    expect(wantsSteps({ ...ran(base), engine: 'loading' })).toBe(false);
  });

  it('keep a recording only for the run it was started for', () => {
    const base = result();
    const started = stepsStarted(ran(base));
    const finished = stepsFinished(started, base, 'in;', result());
    expect(finished.steps.status).toBe('done');
    expect(finished.steps.at).toBe(1);
    const moved = { ...started, run: { ...started.run, result: result({ stdout: 'other;' }) } };
    expect(stepsFinished(moved, base, 'in;', result())).toBe(moved);
    expect(stepsFailed(started, base, 'boom').steps).toMatchObject({ status: 'failed', error: 'boom' });
  });

  it('diff the chosen step against the one before, another step or nothing', () => {
    const base = result();
    const done = stepsFinished(stepsStarted(ran(base)), base, 'in;', result());
    expect(stepSides(done)).toEqual({ kind: 'diff', before: 'in;', after: 'a;', beforeName: 'Input', afterName: 'prepare', note: null });
    const at = (index: number, against: AppState['steps']['against']) => ({ ...done, steps: { ...done.steps, at: index, against } });
    expect(stepSides(at(3, 'prev'))).toMatchObject({ kind: 'diff', note: 'unminify changed nothing.' });
    expect(stepSides(at(6, 1))).toMatchObject({ kind: 'diff', before: 'a;', after: 'out;\n', beforeName: 'prepare', afterName: 'Output' });
    expect(stepSides(at(2, 'none'))).toEqual({ kind: 'code', text: 'b;', name: 'deobfuscate', note: null });
    expect(stepSides(at(0, 'prev'))).toEqual({ kind: 'code', text: 'in;', name: 'Input', note: null });
    expect(stepSides(at(4, 'prev'))).toMatchObject({ kind: 'off', note: expect.stringContaining('24 MB cap') });
    expect(stepSides(at(6, 4))).toMatchObject({ kind: 'off', note: expect.stringContaining('deobfuscate (round 2) was not kept') });
  });

  it('say what is happening while there are no steps to show', () => {
    expect(stepSides(ran(result()))).toMatchObject({ kind: 'off', note: 'Recording the code after each step…' });
    expect(stepSides(initialState('dark'))).toMatchObject({ kind: 'off', note: expect.stringContaining('Run something') });
    const base = result();
    expect(stepSides(stepsFailed(stepsStarted(ran(base)), base, 'trap'))).toMatchObject({ kind: 'off', note: 'Recording the steps failed: trap' });
    expect(NO_STEPS.status).toBe('idle');
  });
});
