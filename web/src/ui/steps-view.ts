import { byId } from '../dom';
import { Cancelled, type Engine } from '../engine/engine';
import type { AppState } from '../state';
import { SNAPSHOT_EXTRAS, stepsCancelled, stepsFailed, stepsFinished, stepsStarted, wantsSteps } from '../steps';
import type { Store } from '../store';

type Render = (bar: HTMLElement, store: Store<AppState>) => void;

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function record(store: Store<AppState>, engine: Engine): void {
  const state = store.get();
  const { result: base, outputInput: input, outputFlags: flags } = state.run;
  if (!wantsSteps(state) || base === null || input === null || flags === null) {
    return;
  }
  store.update(stepsStarted);
  engine.run(input.text, flags, undefined, SNAPSHOT_EXTRAS).then(
    (recorded) => store.update((current) => stepsFinished(current, base, input.text, recorded)),
    (error: unknown) => {
      if (!(error instanceof Cancelled)) {
        store.update((current) => stepsFailed(current, base, message(error)));
      }
    },
  );
}

export function mountSteps(store: Store<AppState>, engine: Engine): { readonly cancel: () => void } {
  const bar = byId('steps-bar');
  let render: Render | null = null;
  let loading = false;
  const paint = () => {
    if (render !== null && !bar.hidden) {
      render(bar, store);
    }
  };
  const cancel = () => {
    engine.cancel();
    store.update(stepsCancelled);
  };
  store.subscribe((state, previous) => {
    const on = state.settings.view === 'steps';
    bar.hidden = !on;
    if (!on) {
      if (state.steps.status === 'running') {
        queueMicrotask(cancel);
      }
      return;
    }
    if (wantsSteps(state)) {
      queueMicrotask(() => record(store, engine));
    }
    if (render === null && !loading) {
      loading = true;
      import('./steps/bar').then(
        (module) => {
          render = module.renderSteps;
          paint();
        },
        () => bar.replaceChildren(document.createTextNode('The steps view failed to load.')),
      );
    } else if (state.steps !== previous.steps || state.settings.view !== previous.settings.view) {
      paint();
    }
  });
  return { cancel };
}
