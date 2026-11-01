import type { RunResult } from './engine/types';
import { modeFromFlags } from './settings/mode';
import { NO_COMPARE, type AppState } from './state';

export interface ComparePlan {
  readonly safe: string;
  readonly unsafe: string;
}

export function otherModeFlags(flags: string): string {
  const words = flags.split(' ').filter((word) => word !== '');
  return words.includes('--unsafe') ? words.filter((word) => word !== '--unsafe').join(' ') : [...words, '--unsafe'].join(' ');
}

export function canCompare(state: AppState): boolean {
  return (
    state.engine === 'ready' &&
    state.run.status !== 'running' &&
    state.compare.status !== 'running' &&
    state.run.result !== null &&
    state.run.outputFlags !== null &&
    state.run.outputInput !== null
  );
}

export function compareLabel(state: AppState): string {
  if (state.compare.status === 'running') {
    return 'Comparing…';
  }
  return modeFromFlags(state.run.outputFlags) === 'unsafe' ? 'Compare with Safe' : 'Compare with Unsafe';
}

export function compareStarted(state: AppState): AppState {
  if (!canCompare(state)) {
    return state;
  }
  const compare = {
    status: 'running' as const,
    base: state.run.result,
    source: state.run.outputInput?.text ?? '',
    flags: otherModeFlags(state.run.outputFlags ?? ''),
    result: null,
    error: null,
  };
  return { ...state, compare };
}

function settles(state: AppState, base: RunResult | null): boolean {
  return state.compare.status === 'running' && state.compare.base === base;
}

export function compareFinished(state: AppState, base: RunResult | null, result: RunResult): AppState {
  if (!settles(state, base)) {
    return state;
  }
  return { ...state, compare: { ...state.compare, status: 'done', result }, settings: { ...state.settings, view: 'diff' } };
}

export function compareFailed(state: AppState, base: RunResult | null, error: string): AppState {
  if (!settles(state, base)) {
    return state;
  }
  return { ...state, compare: { ...state.compare, status: 'failed', error } };
}

export function compareCancelled(state: AppState): AppState {
  return state.compare.status === 'running' ? { ...state, compare: NO_COMPARE } : state;
}

export function comparePlan(state: AppState): ComparePlan | null {
  const { compare, run } = state;
  if (compare.status !== 'done' || compare.result === null || compare.base === null || compare.base !== run.result) {
    return null;
  }
  const baseUnsafe = modeFromFlags(run.outputFlags) === 'unsafe';
  return baseUnsafe
    ? { safe: compare.result.stdout, unsafe: compare.base.stdout }
    : { safe: compare.base.stdout, unsafe: compare.result.stdout };
}
