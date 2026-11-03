import './steps.css';
import type { AppState } from '../../state';
import type { Against, StepsState } from '../../steps';
import type { Store } from '../../store';

interface Built {
  readonly steps: StepsState['steps'];
  readonly chips: readonly HTMLButtonElement[];
  readonly select: HTMLSelectElement;
}

const built = new WeakMap<HTMLElement, Built>();

function make<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ''): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}

function choose(store: Store<AppState>, change: Partial<Pick<StepsState, 'at' | 'against'>>): void {
  store.update((state) => ({ ...state, steps: { ...state.steps, ...change } }));
}

function againstOf(value: string): Against {
  return value === 'prev' || value === 'none' ? value : Number(value);
}

function chipsOf(state: StepsState, store: Store<AppState>): HTMLButtonElement[] {
  return state.steps.map((step, index) => {
    const chip = make('button', 'step-chip', step.label);
    chip.type = 'button';
    chip.dataset['index'] = String(index);
    const same = index > 0 && step.code !== null && step.code === state.steps[index - 1]?.code;
    chip.classList.toggle('step-same', same);
    if (step.changes !== null && step.changes > 0) {
      chip.append(make('span', 'step-count', step.changes.toLocaleString('en-US')));
    }
    chip.title = same ? `${step.label}: no change` : step.changes === null ? step.label : `${step.label}: ${step.changes.toLocaleString('en-US')} changes`;
    chip.addEventListener('click', () => choose(store, { at: index }));
    return chip;
  });
}

function selectOf(state: StepsState, store: Store<AppState>): HTMLSelectElement {
  const select = make('select', 'step-select');
  select.id = 'step-against';
  const option = (value: string, label: string) => {
    const element = make('option', '', label);
    element.value = value;
    return element;
  };
  select.append(option('prev', 'the step before'), option('none', 'nothing'), ...state.steps.map((step, index) => option(String(index), step.label)));
  select.addEventListener('change', () => choose(store, { against: againstOf(select.value) }));
  return select;
}

function build(bar: HTMLElement, state: StepsState, store: Store<AppState>): Built {
  const chips = chipsOf(state, store);
  const list = make('div', 'step-list');
  list.setAttribute('role', 'group');
  list.setAttribute('aria-label', 'Steps');
  list.append(...chips);
  list.addEventListener('keydown', (event) => {
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
    const at = Math.min(Math.max(store.get().steps.at + step, 0), chips.length - 1);
    if (step !== 0 && at !== store.get().steps.at) {
      event.preventDefault();
      choose(store, { at });
      chips[at]?.focus();
    }
  });
  const select = selectOf(state, store);
  const label = make('label', 'step-against', 'Compare with');
  label.htmlFor = select.id;
  const against = make('div', 'step-against-row');
  against.append(label, select);
  bar.replaceChildren(list, against);
  return { steps: state.steps, chips, select };
}

export function renderSteps(bar: HTMLElement, store: Store<AppState>): void {
  const state = store.get().steps;
  if (state.status !== 'done') {
    built.delete(bar);
    const text = state.status === 'failed' ? 'Recording the steps failed.' : 'Recording the code after each step…';
    bar.replaceChildren(make('p', 'steps-note', text));
    return;
  }
  const current = built.get(bar);
  const shown = current !== undefined && current.steps === state.steps ? current : build(bar, state, store);
  built.set(bar, shown);
  shown.chips.forEach((chip, index) => chip.setAttribute('aria-pressed', String(index === state.at)));
  shown.chips[state.at]?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  shown.select.value = String(state.against);
}
