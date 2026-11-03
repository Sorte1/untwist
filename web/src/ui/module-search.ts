import { EditorSelection } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { byId, setText } from '../dom';
import { searchModules, snippetOf, HIT_LIMIT, type Found, type Hit } from '../search/modules';
import type { AppState } from '../state';
import type { Store } from '../store';
import { currentBundle, treeOrder, type BundleInfo } from './module-tree';
import './module-search.css';

const DEBOUNCE_MS = 120;

function plural(count: number, one: string, many: string): string {
  return `${count.toLocaleString('en-US')} ${count === 1 ? one : many}`;
}

function noteOf(found: Found, total: number): string {
  if (found.hits.length === 0) {
    return `No matches in ${plural(total, 'module', 'modules')}`;
  }
  const where = plural(found.modules, 'module', 'modules');
  return found.capped ? `First ${HIT_LIMIT.toLocaleString('en-US')} matches in ${where}` : `${plural(found.hits.length, 'match', 'matches')} in ${where}`;
}

function span(className: string, text: string): HTMLElement {
  const element = document.createElement('span');
  element.className = className;
  element.textContent = text;
  return element;
}

function hitButton(hit: Hit, path: string, text: string, jump: (hit: Hit) => void): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'hit-item';
  button.dataset['path'] = path;
  button.dataset['line'] = String(hit.line);
  button.dataset['column'] = String(hit.column);
  button.title = `${path}:${hit.line}:${hit.column}`;
  const snip = snippetOf(text, hit.from, hit.to);
  const body = span('hit-text', '');
  const mark = document.createElement('mark');
  mark.textContent = snip.match;
  body.append(document.createTextNode(snip.before), mark, document.createTextNode(snip.after));
  button.append(span('hit-line', String(hit.line)), body);
  button.addEventListener('click', () => jump(hit));
  return button;
}

function groups(found: Found, bundle: BundleInfo, texts: readonly string[], jump: (hit: Hit) => void): HTMLElement[] {
  const byModule = new Map<number, Hit[]>();
  for (const hit of found.hits) {
    byModule.set(hit.module, [...(byModule.get(hit.module) ?? []), hit]);
  }
  return [...byModule].map(([module, hits]) => {
    const path = bundle.modules[module]?.path ?? `module ${module}`;
    const group = document.createElement('div');
    group.className = 'hit-group';
    group.setAttribute('role', 'group');
    group.setAttribute('aria-label', path);
    const head = span('hit-path', path);
    head.append(span('hit-count', String(hits.length)));
    group.append(head, ...hits.map((hit) => hitButton(hit, path, texts[module] ?? '', jump)));
    return group;
  });
}

function stepHits(box: HTMLElement, input: HTMLInputElement, key: string): boolean {
  const items = [...box.querySelectorAll<HTMLElement>('.hit-item')];
  const at = items.indexOf(document.activeElement as HTMLElement);
  if (key === 'ArrowDown' && at < items.length - 1) {
    items[at + 1]?.focus();
    return true;
  }
  if (key === 'ArrowUp' && at >= 0) {
    (at === 0 ? input : items[at - 1])?.focus();
    return true;
  }
  return false;
}

export function mountModuleSearch(store: Store<AppState>, view: EditorView): void {
  const nav = byId('module-tree');
  const input = byId<HTMLInputElement>('module-search');
  const box = byId('module-hits');
  const list = byId('module-list');
  const note = byId('module-search-note');
  let first: Hit | null = null;
  let timer = 0;
  const jump = (hit: Hit) => {
    store.update((state) => ({
      ...state,
      selectedModule: hit.module,
      settings: state.settings.view === 'diff' ? { ...state.settings, view: 'output' } : state.settings,
    }));
    const length = view.state.doc.length;
    const from = Math.min(hit.from, length);
    view.dispatch({ selection: EditorSelection.single(from, Math.min(hit.to, length)), effects: EditorView.scrollIntoView(from, { y: 'center' }) });
    view.focus();
  };
  const paint = () => {
    const state = store.get();
    const result = state.run.result;
    const bundle = currentBundle(state);
    const searching = input.value.trim() !== '' && bundle !== null && result !== null;
    list.hidden = searching;
    box.hidden = !searching;
    note.hidden = !searching;
    nav.classList.toggle('tree-searching', searching);
    if (!searching) {
      first = null;
      box.replaceChildren();
      return;
    }
    const found = searchModules(result.modules, treeOrder(bundle.modules), input.value);
    first = found.hits[0] ?? null;
    box.replaceChildren(...groups(found, bundle, result.modules, jump));
    setText(note, noteOf(found, bundle.modules.length));
  };
  input.addEventListener('input', () => {
    window.clearTimeout(timer);
    timer = window.setTimeout(paint, DEBOUNCE_MS);
  });
  input.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && input.value !== '') {
      event.preventDefault();
      event.stopPropagation();
      input.value = '';
      paint();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      window.clearTimeout(timer);
      paint();
      if (first !== null) {
        jump(first);
      }
    } else if (event.key === 'ArrowDown') {
      window.clearTimeout(timer);
      paint();
      if (stepHits(box, input, 'ArrowDown')) {
        event.preventDefault();
      }
    }
  });
  box.addEventListener('keydown', (event) => {
    if (stepHits(box, input, event.key)) {
      event.preventDefault();
    }
  });
  store.subscribe((state, previous) => {
    if (state.run.result !== previous.run.result && input.value.trim() !== '') {
      paint();
    }
  });
}
