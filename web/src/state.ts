import type { RunResult } from './engine/types';
import { explainedLine, firedLimits, PARTIAL_LIMITS, type ExplainedLimit } from './limits';
import type { StageMark } from './progress';
import { DEFAULTS, type Settings } from './settings/settings';
import { NO_STEPS, type StepsState } from './steps';
import type { SuppliedWasm } from './wasm/wasm';

export type Theme = 'dark' | 'light';
export type EngineStatus = 'loading' | 'ready' | 'failed';
export type RunStatus = 'idle' | 'running' | 'done' | 'cancelled' | 'limit' | 'failed';

export const EDITOR_BYTES_CAP = 1024 * 1024;
export const EDITOR_LINE_CAP = 20_000;
export const PREVIEW_CHARS = 64 * 1024;

export interface InputState {
  readonly text: string;
  readonly name: string;
  readonly bytes: number;
  readonly lines: number;
  readonly large: boolean;
  readonly revision: number;
}

export interface RunState {
  readonly status: RunStatus;
  readonly startedAt: number;
  readonly elapsedMs: number;
  readonly result: RunResult | null;
  readonly error: string | null;
  readonly ranFlags: string | null;
  readonly outputFlags: string | null;
  readonly ranInput: InputState | null;
  readonly outputInput: InputState | null;
  readonly stages: readonly StageMark[];
}

export type CompareStatus = 'idle' | 'running' | 'done' | 'failed';

export interface CompareState {
  readonly status: CompareStatus;
  readonly base: RunResult | null;
  readonly source: string;
  readonly flags: string;
  readonly result: RunResult | null;
  readonly error: string | null;
}

export const NO_COMPARE: CompareState = { status: 'idle', base: null, source: '', flags: '', result: null, error: null };

export interface AppState {
  readonly theme: Theme;
  readonly engine: EngineStatus;
  readonly engineError: string | null;
  readonly input: InputState;
  readonly run: RunState;
  readonly settings: Settings;
  readonly settingsProblems: readonly string[];
  readonly selectedModule: number | null;
  readonly compare: CompareState;
  readonly steps: StepsState;
  readonly wasm: readonly SuppliedWasm[];
}

export function utf8Bytes(text: string): number {
  let bytes = 0;
  for (let index = 0; index < text.length; index += 1) {
    const unit = text.charCodeAt(index);
    if (unit < 0x80) {
      bytes += 1;
    } else if (unit < 0x800) {
      bytes += 2;
    } else if (unit >= 0xd800 && unit <= 0xdbff) {
      bytes += 4;
      index += 1;
    } else {
      bytes += 3;
    }
  }
  return bytes;
}

export function lineStats(text: string): { lines: number; longest: number } {
  let lines = 1;
  let longest = 0;
  let start = 0;
  for (let end = text.indexOf('\n'); end !== -1; end = text.indexOf('\n', start)) {
    longest = Math.max(longest, end - start);
    lines += 1;
    start = end + 1;
  }
  return { lines, longest: Math.max(longest, text.length - start) };
}

export function inputOf(text: string, name: string, revision: number): InputState {
  const bytes = utf8Bytes(text);
  const { lines, longest } = lineStats(text);
  const large = bytes > EDITOR_BYTES_CAP || longest > EDITOR_LINE_CAP;
  return { text, name, bytes, lines, large, revision };
}

export function initialState(theme: Theme, settings: Settings = DEFAULTS, settingsProblems: readonly string[] = []): AppState {
  return {
    theme: settings.theme ?? theme,
    settings,
    settingsProblems,
    selectedModule: null,
    engine: 'loading',
    engineError: null,
    input: inputOf('', 'untitled.js', 0),
    run: { status: 'idle', startedAt: 0, elapsedMs: 0, result: null, error: null, ranFlags: null, outputFlags: null, ranInput: null, outputInput: null, stages: [] },
    compare: NO_COMPARE,
    steps: NO_STEPS,
    wasm: [],
  };
}

export function statusOf(result: RunResult): RunStatus {
  const partial = result.trap !== null || result.code === 3 || result.limits.split(',').some((name) => PARTIAL_LIMITS.includes(name));
  return partial ? 'limit' : 'done';
}

export function runStarted(state: AppState, now: number, flags = '', input: InputState = state.input): AppState {
  if (state.run.status === 'running') {
    return state;
  }
  return { ...state, run: { ...state.run, status: 'running', startedAt: now, elapsedMs: 0, error: null, ranFlags: flags, ranInput: input, stages: [] } };
}

export function stagesAdded(state: AppState, marks: readonly StageMark[]): AppState {
  if (state.run.status !== 'running' || marks.length === 0) {
    return state;
  }
  return { ...state, run: { ...state.run, stages: [...state.run.stages, ...marks] } };
}

export function stagesClosed(state: AppState, marks: readonly StageMark[], now: number): AppState {
  if (state.run.status !== 'done' && state.run.status !== 'limit') {
    return state;
  }
  return { ...state, run: { ...state.run, elapsedMs: now - state.run.startedAt, stages: [...state.run.stages, ...marks] } };
}

export function ticked(state: AppState, now: number): AppState {
  if (state.run.status !== 'running') {
    return state;
  }
  return { ...state, run: { ...state.run, elapsedMs: now - state.run.startedAt } };
}

export function runFinished(state: AppState, result: RunResult, now: number): AppState {
  if (state.run.status !== 'running') {
    return state;
  }
  const elapsedMs = now - state.run.startedAt;
  return { ...state, selectedModule: null, run: { ...state.run, status: statusOf(result), elapsedMs, result, error: null, outputFlags: state.run.ranFlags, outputInput: state.run.ranInput } };
}

export function runCancelled(state: AppState, now: number): AppState {
  if (state.run.status !== 'running') {
    return state;
  }
  return { ...state, run: { ...state.run, status: 'cancelled', elapsedMs: now - state.run.startedAt } };
}

export function runFailed(state: AppState, error: string, now: number): AppState {
  if (state.run.status !== 'running') {
    return state;
  }
  return { ...state, run: { ...state.run, status: 'failed', elapsedMs: now - state.run.startedAt, error } };
}

export function seconds(ms: number): string {
  return `${(ms / 1000).toFixed(2)} s`;
}

export function statusLabel(state: AppState): string {
  if (state.engine === 'loading') {
    return 'Loading engine';
  }
  if (state.engine === 'failed') {
    return 'Engine failed to load';
  }
  const { status, result } = state.run;
  const labels: Record<RunStatus, string> = {
    idle: 'Ready',
    running: 'Running',
    done: 'Done',
    cancelled: 'Cancelled',
    failed: 'Failed',
    limit: result?.trap === null || result === null ? 'Stopped by limit' : `Stopped by limit (${result.trap})`,
  };
  return labels[status];
}

export function exitLabel(run: RunState): string {
  if (run.status === 'running') {
    return '…';
  }
  if (run.status === 'cancelled') {
    return 'killed';
  }
  return run.status === 'done' || run.status === 'limit' ? String(run.result?.code ?? '—') : '—';
}

function firedOf(run: RunState): ExplainedLimit[] | null {
  if (run.status !== 'done' && run.status !== 'limit') {
    return null;
  }
  return firedLimits(run.result?.limits ?? '', run.result?.trap ?? null);
}

export function limitsLabel(run: RunState): string {
  const fired = firedOf(run);
  if (fired === null) {
    return '—';
  }
  return fired.length === 0 ? 'none fired' : fired.map((limit) => limit.name).join(', ');
}

export function limitsTitle(run: RunState): string {
  return (firedOf(run) ?? []).map(explainedLine).join('\n');
}

export function skippedLabel(run: RunState): string {
  const fired = run.status === 'done' ? firedOf(run) ?? [] : [];
  return fired.length === 0 ? '' : `Skipped: ${fired.map((limit) => limit.name).join(', ')}`;
}
