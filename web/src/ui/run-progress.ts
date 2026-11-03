import { byId, setText } from '../dom';
import { progressOf, stageLine, type RunProgress } from '../progress';
import { seconds, type AppState } from '../state';
import type { Store } from '../store';
import './run-progress.css';

const DONE_SUFFIX = ', done';

function stepItem(label: string, state: string): HTMLLIElement {
  const item = document.createElement('li');
  item.className = 'run-step';
  item.dataset['state'] = state;
  item.textContent = label;
  if (state === 'current') {
    item.setAttribute('aria-current', 'step');
  }
  if (state === 'done') {
    const done = document.createElement('span');
    done.className = 'sr-only';
    done.textContent = DONE_SUFFIX;
    item.append(done);
  }
  return item;
}

function stepsKey(progress: RunProgress): string {
  return progress.steps.map((step) => `${step.phase}:${step.state}`).join(' ');
}

export function mountRunProgress(store: Store<AppState>): void {
  const list = byId('run-steps');
  const line = byId('running-stage');
  let shownSteps = '';
  store.subscribe((state) => {
    const run = state.run;
    if (run.status !== 'running') {
      return;
    }
    const progress = progressOf(run.ranFlags ?? '', run.stages, run.startedAt + run.elapsedMs);
    const key = stepsKey(progress);
    if (key !== shownSteps) {
      shownSteps = key;
      list.replaceChildren(...progress.steps.map((step) => stepItem(step.label, step.state)));
    }
    setText(line, stageLine(progress));
    line.title = progress.stage === null ? '' : `${seconds(progress.stageMs)} in this stage`;
  });
}
