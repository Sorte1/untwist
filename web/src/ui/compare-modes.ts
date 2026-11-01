import { byId, setText } from '../dom';
import { canCompare, compareCancelled, compareFailed, compareFinished, compareLabel, compareStarted } from '../compare';
import { Cancelled, type Engine } from '../engine/engine';
import type { AppState } from '../state';
import type { Store } from '../store';

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export interface CompareControl {
  readonly cancel: () => void;
}

export function mountCompareModes(store: Store<AppState>, engine: Engine): CompareControl {
  const button = byId<HTMLButtonElement>('compare-modes');
  button.addEventListener('click', () => {
    store.update(compareStarted);
    const { compare } = store.get();
    if (compare.status !== 'running') {
      return;
    }
    performance.mark('untwist:compare-start');
    engine.run(compare.source, compare.flags).then(
      (result) => {
        performance.mark('untwist:compare-done', { detail: { memoryBytes: result.memoryBytes, workerMs: result.workerMs } });
        store.update((state) => compareFinished(state, compare.base, result));
      },
      (error: unknown) => {
        if (!(error instanceof Cancelled)) {
          store.update((state) => compareFailed(state, compare.base, message(error)));
        }
      },
    );
  });
  store.subscribe((state) => {
    button.disabled = !canCompare(state);
    setText(byId('compare-label'), compareLabel(state));
    button.title = state.compare.status === 'failed' ? `The comparison failed: ${state.compare.error ?? 'unknown error'}` : compareLabel(state);
    button.setAttribute('aria-busy', String(state.compare.status === 'running'));
  });
  return {
    cancel: () => {
      engine.cancel();
      store.update(compareCancelled);
    },
  };
}
