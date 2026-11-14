import { byId, setText } from '../dom';
import { attemptLine, durationLabel, librariesOn, progressOf, stageLine, timelineText, type RunProgress, type Step } from '../progress';
import { seconds, type AppState, type RunState } from '../state';
import type { Store } from '../store';
import './run-progress.css';

const DONE_SUFFIX = ', done';

function stepItem(step: Step): HTMLLIElement {
  const item = document.createElement('li');
  item.className = 'run-step';
  item.dataset['state'] = step.state;
  item.dataset['phase'] = step.phase;
  const label = document.createElement('span');
  label.className = 'run-step-label';
  label.textContent = step.label;
  const time = document.createElement('span');
  time.className = 'run-step-ms';
  item.append(label, time);
  if (step.state === 'current') {
    item.setAttribute('aria-current', 'step');
  }
  if (step.state === 'done') {
    const done = document.createElement('span');
    done.className = 'sr-only';
    done.textContent = DONE_SUFFIX;
    label.append(done);
  }
  return item;
}

function stepsKey(progress: RunProgress): string {
  return progress.steps.map((step) => `${step.phase}:${step.state}`).join(' ');
}

export function runProgressOf(run: RunState): RunProgress {
  const flags = run.ranFlags ?? '';
  return progressOf(flags, run.stages, run.startedAt + run.elapsedMs, { libraries: librariesOn(flags), editor: true });
}

function paintTimes(list: HTMLElement, progress: RunProgress): void {
  progress.steps.forEach((step, at) => {
    const time = list.children[at]?.querySelector('.run-step-ms');
    if (time !== null && time !== undefined) {
      setText(time as HTMLElement, step.state === 'pending' ? '' : durationLabel(step.ms));
    }
  });
}

function statusStage(progress: RunProgress): string {
  const current = progress.steps.find((step) => step.state === 'current');
  if (current === undefined) {
    return '';
  }
  return `${current.label} · ${durationLabel(current.ms)}`;
}

export function mountRunProgress(store: Store<AppState>): void {
  const list = byId('run-steps');
  const line = byId('running-stage');
  const attempt = byId('running-attempt');
  const status = byId('status-stage');
  const total = byId('elapsed');
  let shownSteps = '';
  store.subscribe((state, previous) => {
    const run = state.run;
    if (run.status !== 'running') {
      setText(status, '');
      if (run.stages !== previous.run.stages || run.status !== previous.run.status) {
        total.title = run.stages.length === 0 ? '' : timelineText(runProgressOf(run));
      }
      return;
    }
    const progress = runProgressOf(run);
    const key = stepsKey(progress);
    if (key !== shownSteps) {
      shownSteps = key;
      list.replaceChildren(...progress.steps.map(stepItem));
    }
    paintTimes(list, progress);
    setText(line, stageLine(progress));
    line.title = progress.stage === null ? '' : `${seconds(progress.stageMs)} in this stage`;
    const retry = attemptLine(progress);
    attempt.hidden = retry === '';
    setText(attempt, retry === '' ? '' : `${retry} · earlier attempt ${durationLabel(progress.earlierMs)}`);
    setText(status, statusStage(progress));
    total.title = '';
  });
}
