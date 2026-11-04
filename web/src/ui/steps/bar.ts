import './steps.css';
import type { AppState } from '../../state';
import type { Against, StepsState } from '../../steps';
import type { Store } from '../../store';

interface Built {
  readonly steps: StepsState['steps'];
  readonly chips: readonly HTMLButtonElement[];
  readonly against: AgainstMenu;
}

interface AgainstMenu {
  readonly root: HTMLElement;
  readonly show: (value: string) => void;
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

function againstMenu(state: StepsState, store: Store<AppState>): AgainstMenu {
  const root = make('div', 'step-against-menu');
  const trigger = make('button', 'step-select');
  trigger.type = 'button';
  trigger.id = 'step-against';
  trigger.setAttribute('aria-haspopup', 'listbox');
  trigger.setAttribute('aria-expanded', 'false');
  const list = make('div', 'examples-menu step-against-list');
  list.id = 'step-against-list';
  list.setAttribute('role', 'listbox');
  list.setAttribute('aria-label', 'Compare with');
  list.hidden = true;
  trigger.setAttribute('aria-controls', list.id);
  const choices: Array<[string, string]> = [['prev', 'the step before'], ['none', 'nothing'], ...state.steps.map((step, index): [string, string] => [String(index), step.label])];
  const options = choices.map(([value, label]) => {
    const option = make('button', 'example-option step-against-option', label);
    option.type = 'button';
    option.dataset['value'] = value;
    option.setAttribute('role', 'option');
    return option;
  });
  list.append(...options);
  const close = (focus: boolean) => {
    list.hidden = true;
    trigger.setAttribute('aria-expanded', 'false');
    document.removeEventListener('pointerdown', outside, true);
    if (focus) {
      trigger.focus();
    }
  };
  const outside = (event: Event) => {
    if (!root.contains(event.target as Node)) {
      close(false);
    }
  };
  const open = () => {
    list.hidden = false;
    trigger.setAttribute('aria-expanded', 'true');
    document.addEventListener('pointerdown', outside, true);
    (options.find((option) => option.getAttribute('aria-selected') === 'true') ?? options[0])?.focus();
  };
  trigger.addEventListener('click', () => (list.hidden ? open() : close(true)));
  list.addEventListener('click', (event) => {
    const value = (event.target as HTMLElement).closest<HTMLElement>('[role=option]')?.dataset['value'];
    if (value !== undefined) {
      close(true);
      choose(store, { against: againstOf(value) });
    }
  });
  list.addEventListener('keydown', (event) => {
    const at = options.indexOf(document.activeElement as HTMLButtonElement);
    const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0;
    if (event.key === 'Escape') {
      event.preventDefault();
      close(true);
    } else if (step !== 0) {
      event.preventDefault();
      options[Math.min(Math.max(at + step, 0), options.length - 1)]?.focus();
    }
  });
  root.append(trigger, list);
  const show = (value: string) => {
    options.forEach((option) => option.setAttribute('aria-selected', String(option.dataset['value'] === value)));
    trigger.textContent = choices.find(([choice]) => choice === value)?.[1] ?? choices[0]?.[1] ?? '';
  };
  return { root, show };
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
  const menu = againstMenu(state, store);
  const label = make('span', 'step-against', 'Compare with');
  label.id = 'step-against-label';
  menu.root.querySelector('#step-against')?.setAttribute('aria-labelledby', `${label.id} step-against`);
  const against = make('div', 'step-against-row');
  against.append(label, menu.root);
  bar.replaceChildren(list, against);
  return { steps: state.steps, chips, against: menu };
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
  shown.against.show(String(state.against));
}
