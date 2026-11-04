import type { RunResult } from './engine/types';
import type { AppState } from './state';

export const SNAPSHOT_EXTRAS = 'snapshots';

export type StepsStatus = 'idle' | 'running' | 'done' | 'failed';
export type Against = number | 'prev' | 'none';

export interface Step {
  readonly label: string;
  readonly changes: number | null;
  readonly code: string | null;
}

export interface StepsState {
  readonly status: StepsStatus;
  readonly base: RunResult | null;
  readonly steps: readonly Step[];
  readonly at: number;
  readonly against: Against;
  readonly error: string | null;
}

export const NO_STEPS: StepsState = { status: 'idle', base: null, steps: [], at: 0, against: 'prev', error: null };

export type StepSides =
  | { readonly kind: 'off'; readonly note: string }
  | { readonly kind: 'code'; readonly text: string; readonly name: string; readonly note: string | null }
  | { readonly kind: 'diff'; readonly before: string; readonly after: string; readonly beforeName: string; readonly afterName: string; readonly note: string | null };

interface SnapshotRow {
  readonly stage: string;
  readonly round: number;
  readonly layer: number;
  readonly kept: 'text' | 'unchanged' | 'over-cap';
}

interface StageRow {
  readonly stage: string;
  readonly round: number;
  readonly changes: number;
}

function rowsOf(report: string): { snapshots: SnapshotRow[]; stages: StageRow[] } {
  try {
    const value = JSON.parse(report) as { snapshots?: unknown; stages?: unknown };
    const snapshots = Array.isArray(value.snapshots) ? (value.snapshots as SnapshotRow[]) : [];
    const stages = Array.isArray(value.stages) ? (value.stages as StageRow[]) : [];
    return { snapshots, stages };
  } catch {
    return { snapshots: [], stages: [] };
  }
}

function labelOf(row: SnapshotRow): string {
  const round = row.round > 1 ? ` (round ${row.round})` : '';
  const layer = row.layer > 0 ? ` · layer ${row.layer + 1}` : '';
  return `${row.stage}${round}${layer}`;
}

export function stepsOf(input: string, result: RunResult): Step[] {
  const { snapshots, stages } = rowsOf(result.report);
  const codes = result.snapshots ?? [];
  const steps: Step[] = [{ label: 'Input', changes: null, code: input }];
  let previous: string | null = input;
  snapshots.forEach((row, index) => {
    const code = row.kept === 'text' ? codes[index] ?? null : row.kept === 'unchanged' ? previous : null;
    const changes = stages.find((stage) => stage.stage === row.stage && stage.round === row.round)?.changes ?? null;
    steps.push({ label: labelOf(row), changes, code });
    previous = code;
  });
  steps.push({ label: 'Output', changes: null, code: result.stdout });
  return steps;
}

export function firstChange(steps: readonly Step[]): number {
  const index = steps.findIndex((step, at) => at > 0 && step.code !== steps[at - 1]?.code);
  return index === -1 ? steps.length - 1 : index;
}

export function stepsStarted(state: AppState): AppState {
  return { ...state, steps: { ...NO_STEPS, status: 'running', base: state.run.result } };
}

function settles(state: AppState, base: RunResult): boolean {
  return state.steps.status === 'running' && state.steps.base === base && state.run.result === base;
}

export function stepsFinished(state: AppState, base: RunResult, input: string, recorded: RunResult): AppState {
  if (!settles(state, base)) {
    return state;
  }
  const steps = stepsOf(input, recorded);
  return { ...state, steps: { ...state.steps, status: 'done', steps, at: firstChange(steps) } };
}

export function stepsFailed(state: AppState, base: RunResult, error: string): AppState {
  return settles(state, base) ? { ...state, steps: { ...state.steps, status: 'failed', error } } : state;
}

export function stepsCancelled(state: AppState): AppState {
  return state.steps.status === 'running' ? { ...state, steps: NO_STEPS } : state;
}

export function wantsSteps(state: AppState): boolean {
  const { run, steps } = state;
  return (
    state.settings.view === 'steps' &&
    state.engine === 'ready' &&
    run.status !== 'running' &&
    run.result !== null &&
    run.outputInput !== null &&
    run.outputFlags !== null &&
    steps.base !== run.result
  );
}

export function stepSides(state: AppState): StepSides {
  const { steps, run } = state;
  if (steps.base !== run.result || steps.status === 'running' || steps.status === 'idle') {
    return { kind: 'off', note: run.result === null ? 'Run something to step through what each stage did.' : 'Recording the code after each step…' };
  }
  if (steps.status === 'failed') {
    return { kind: 'off', note: `Recording the steps failed: ${steps.error ?? 'unknown error'}` };
  }
  const after = steps.steps[steps.at];
  if (after === undefined) {
    return { kind: 'off', note: 'This run recorded no steps.' };
  }
  if (after.code === null) {
    return { kind: 'off', note: `The code after ${after.label} was not kept: this run's steps passed the 24 MB cap.` };
  }
  const against = steps.against === 'prev' ? steps.at - 1 : steps.against;
  const before = against === 'none' ? undefined : steps.steps[against];
  if (before === undefined) {
    return { kind: 'code', text: after.code, name: after.label, note: null };
  }
  if (before.code === null) {
    return { kind: 'off', note: `The code after ${before.label} was not kept: this run's steps passed the 24 MB cap.` };
  }
  const note = before.code === after.code ? `${after.label} changed nothing.` : null;
  return { kind: 'diff', before: before.code, after: after.code, beforeName: before.label, afterName: after.label, note };
}
