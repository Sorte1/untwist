import { EXAMPLES } from '../examples/examples';
import { exampleCommands, moduleCommands, settingCommands } from '../palette/commands';
import { filterCommands, type Command } from '../palette/match';
import type { Settings } from '../settings/settings';
import type { AppState } from '../state';
import type { Store } from '../store';
import { currentBundle } from './module-tree';
import type { PaletteActions } from './palette-key';
import './palette.css';

const SEARCH_ICON =
  '<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" aria-hidden="true"><circle cx="7" cy="7" r="4.5"></circle><path d="M10.5 10.5 14 14"></path></svg>';

function clicking(id: string, group: string, title: string, hint = ''): Command[] {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLButtonElement) || element.disabled) {
    return [];
  }
  return [{ group, title, hint, run: () => element.click() }];
}

function labelOf(id: string, fallback: string): string {
  return document.getElementById(id)?.textContent?.trim() || fallback;
}

function actionCommands(state: AppState, actions: PaletteActions): Command[] {
  const runnable = state.engine === 'ready' && state.run.status !== 'running';
  return [
    ...(runnable ? [{ group: 'Action', title: 'Run', hint: 'Ctrl ↵', run: actions.run }] : []),
    ...clicking('cancel', 'Action', 'Cancel the run'),
    ...clicking('copy-output', 'Action', 'Copy the output'),
    ...clicking('download', 'Action', 'Download the output', labelOf('download-label', '').replace(/^Download /, '')),
    ...clicking('compare-modes', 'Action', labelOf('compare-label', 'Compare modes')),
    ...clicking('copy-link', 'Action', 'Copy the settings link'),
    ...clicking('open-file', 'Action', 'Open a file'),
    ...clicking('clear-input', 'Action', 'Clear the input'),
  ];
}

function viewCommands(state: AppState): Command[] {
  const diff = state.settings.view === 'diff';
  const diagnostics = document.getElementById('diag-toggle')?.getAttribute('aria-expanded') === 'true';
  return [
    ...clicking('theme', 'View', state.theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'),
    ...(diff ? clicking('view-output', 'View', 'Show the code') : clicking('view-diff', 'View', 'Show the diff')),
    ...(diff ? clicking('diff-wrap', 'View', 'Wrap diff lines', state.settings.wrap ? 'on' : 'off') : []),
    ...(state.settings.input === 'hidden' ? clicking('show-input', 'View', 'Show the input') : clicking('hide-input', 'View', 'Hide the input')),
    ...clicking('settings-toggle', 'View', 'Open settings'),
    ...clicking('diag-toggle', 'View', diagnostics ? 'Hide diagnostics' : 'Show diagnostics'),
  ];
}

function commandsNow(store: Store<AppState>, actions: PaletteActions): Command[] {
  const state = store.get();
  const apply = (patch: Partial<Settings>) => store.update((current) => ({ ...current, settings: { ...current.settings, ...patch } }));
  const pickModule = (index: number | null) => store.update((current) => ({ ...current, selectedModule: index }));
  return [
    ...actionCommands(state, actions),
    ...settingCommands(state.settings, apply),
    ...viewCommands(state),
    ...exampleCommands(EXAMPLES, actions.pickExample),
    ...moduleCommands(currentBundle(state), state.selectedModule, pickModule),
  ];
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ''): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

function deepActive(): HTMLElement | null {
  let active = document.activeElement;
  while (active?.shadowRoot?.activeElement) {
    active = active.shadowRoot.activeElement;
  }
  return active instanceof HTMLElement ? active : null;
}

interface Parts {
  readonly dialog: HTMLDialogElement;
  readonly input: HTMLInputElement;
  readonly list: HTMLElement;
}

function build(): Parts {
  const dialog = element('dialog', 'palette');
  dialog.id = 'palette';
  dialog.setAttribute('aria-label', 'Command palette');
  const search = element('div', 'palette-search');
  search.innerHTML = SEARCH_ICON;
  const input = element('input', 'palette-input');
  input.id = 'palette-input';
  input.type = 'text';
  input.autocomplete = 'off';
  input.spellcheck = false;
  input.placeholder = 'Search commands, examples and modules';
  input.setAttribute('role', 'combobox');
  input.setAttribute('aria-expanded', 'true');
  input.setAttribute('aria-controls', 'palette-list');
  input.setAttribute('aria-autocomplete', 'list');
  input.setAttribute('aria-label', 'Command');
  search.append(input);
  const list = element('div', 'palette-list');
  list.id = 'palette-list';
  list.setAttribute('role', 'listbox');
  list.setAttribute('aria-label', 'Commands');
  const foot = element('div', 'palette-foot');
  foot.append(element('span', '', '↑ ↓ to move'), element('span', '', '↵ to run'), element('span', '', 'Esc to close'));
  dialog.append(search, list, foot);
  document.body.append(dialog);
  return { dialog, input, list };
}

function optionOf(command: Command, index: number): HTMLElement {
  const option = element('div', 'palette-item');
  option.id = `palette-opt-${index}`;
  option.setAttribute('role', 'option');
  option.dataset['group'] = command.group;
  option.append(element('span', 'palette-group', command.group), element('span', 'palette-title', command.title));
  if (command.hint !== '') {
    option.append(element('span', 'palette-hint', command.hint));
  }
  return option;
}

export function createPalette(store: Store<AppState>, actions: PaletteActions): { toggle(): void } {
  const { dialog, input, list } = build();
  let all: Command[] = [];
  let shown: Command[] = [];
  let active = 0;
  let returnTo: HTMLElement | null = null;
  const mark = (next: number, scroll: boolean) => {
    active = shown.length === 0 ? 0 : (next + shown.length) % shown.length;
    list.querySelectorAll('[role=option]').forEach((option, index) => option.setAttribute('aria-selected', String(index === active)));
    const current = list.children[active];
    if (shown.length === 0 || current === undefined) {
      input.removeAttribute('aria-activedescendant');
      return;
    }
    input.setAttribute('aria-activedescendant', current.id);
    if (scroll) {
      current.scrollIntoView({ block: 'nearest' });
    }
  };
  const render = () => {
    shown = filterCommands(all, input.value);
    list.replaceChildren(...(shown.length === 0 ? [element('div', 'palette-empty', 'No matching command')] : shown.map(optionOf)));
    mark(0, true);
  };
  const close = () => {
    dialog.close();
  };
  const execute = (index: number) => {
    const command = shown[index];
    if (command === undefined) {
      return;
    }
    close();
    command.run();
  };
  input.addEventListener('input', render);
  input.addEventListener('keydown', (event) => {
    const steps: Readonly<Record<string, number>> = { ArrowDown: 1, ArrowUp: -1, PageDown: 8, PageUp: -8 };
    const step = steps[event.key];
    if (step !== undefined) {
      event.preventDefault();
      mark(active + step, true);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      execute(active);
    }
  });
  list.addEventListener('click', (event) => {
    const option = event.target instanceof Element ? event.target.closest('[role=option]') : null;
    if (option !== null) {
      execute([...list.children].indexOf(option));
    }
  });
  list.addEventListener('mousemove', (event) => {
    const option = event.target instanceof Element ? event.target.closest('[role=option]') : null;
    const index = option === null ? -1 : [...list.children].indexOf(option);
    if (index !== -1 && index !== active) {
      mark(index, false);
    }
  });
  dialog.addEventListener('click', (event) => {
    if (event.target === dialog) {
      close();
    }
  });
  dialog.addEventListener('close', () => {
    returnTo?.focus();
    returnTo = null;
  });
  return {
    toggle() {
      if (dialog.open) {
        close();
        return;
      }
      returnTo = deepActive();
      all = commandsNow(store, actions);
      input.value = '';
      render();
      dialog.showModal();
      input.focus();
    },
  };
}
