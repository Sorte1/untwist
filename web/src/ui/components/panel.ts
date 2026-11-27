import { downloadText } from '../details/export';
import type { RestoredComponent } from './components';
import './components.css';

export interface ComponentsView {
  readonly enabled: boolean;
  readonly components: readonly RestoredComponent[];
  readonly kept: readonly string[];
  readonly turnOn: () => void;
}

const COPIED_MS = 1200;

function make(tag: string, className: string, text?: string): HTMLElement {
  const element = document.createElement(tag);
  element.className = className;
  if (text !== undefined) {
    element.textContent = text;
  }
  return element;
}

function button(label: string, act: (self: HTMLButtonElement) => void): HTMLButtonElement {
  const element = make('button', 'btn btn-small', label) as HTMLButtonElement;
  element.type = 'button';
  element.addEventListener('click', () => act(element));
  return element;
}

function fileName(path: string): string {
  return path.split('/').pop() ?? path;
}

function item(component: RestoredComponent): HTMLElement {
  const holder = make('div', 'comp-item');
  const head = make('div', 'comp-head');
  const copy = button('Copy', (self) => {
    navigator.clipboard.writeText(component.code).then(
      () => {
        self.textContent = 'Copied';
        setTimeout(() => (self.textContent = 'Copy'), COPIED_MS);
      },
      () => (self.textContent = 'Copy failed'),
    );
  });
  const save = button('Download', () => downloadText(fileName(component.path), component.code, 'text/plain'));
  head.append(make('span', 'comp-path mono', component.path), copy, save);
  holder.append(head, make('pre', 'comp-code mono', component.code));
  return holder;
}

export function renderComponents(holder: HTMLElement, view: ComponentsView): void {
  if (!view.enabled) {
    const note = make('p', 'comp-note muted', 'Svelte restore is off. Turn it on to rebuild compiled Svelte 5 components as .svelte files; the code output stays the same.');
    holder.replaceChildren(note, button('Restore Svelte components', () => view.turnOn()));
    return;
  }
  const parts: HTMLElement[] = [];
  if (view.components.length === 0) {
    parts.push(make('p', 'comp-note muted', 'No compiled Svelte 5 component was found, or none could be mapped in full.'));
  } else {
    parts.push(make('p', 'comp-note muted', 'A view for reading: each file is rebuilt from the compiled component, and the downloaded zip holds them beside the modules.'));
    parts.push(...view.components.map(item));
  }
  if (view.kept.length > 0) {
    const list = make('ul', 'comp-kept muted');
    for (const line of view.kept) {
      list.append(make('li', '', line));
    }
    parts.push(make('p', 'comp-note muted', 'Kept as JavaScript, with the reason:'), list);
  }
  holder.replaceChildren(...parts);
}
