import { byId, sizeLabel } from '../dom';
import { ago, entryOf, remembered, settingsLine, type RecentEntry } from '../recent/recent';
import { browserStorage, clearRecent, loadRecent, saveRecent, type KeyValue } from '../recent/storage';
import { parseHash, writeHash } from '../settings/hash';
import type { AppState } from '../state';
import type { Store } from '../store';
import type { Loader } from './load-input';
import './recent.css';

const REOPENED = 'Reopened from recent runs.';
const KEPT_HERE = 'Kept in this browser only. Inputs over 512 KB are not kept.';
const KEPT_IN_TAB = 'Browser storage is off, so runs are kept for this tab only.';
const EMPTY = 'No runs yet. Each input you run is kept here.';

interface Recents {
  readonly storage: KeyValue | null;
  read(): readonly RecentEntry[];
  record(entry: RecentEntry): void;
  clear(): void;
}

function recents(storage: KeyValue | null): Recents {
  let session: readonly RecentEntry[] = loadRecent(storage);
  const read = () => (storage === null ? session : loadRecent(storage));
  return {
    storage,
    read,
    record(entry) {
      const next = remembered(read(), entry);
      session = saveRecent(storage, next) ?? next;
    },
    clear() {
      session = [];
      clearRecent(storage);
    },
  };
}

function text(tag: string, className: string, content: string): HTMLElement {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = content;
  return element;
}

function option(entry: RecentEntry, now: number, pick: () => void): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'example-option recent-option';
  button.setAttribute('role', 'menuitem');
  button.tabIndex = -1;
  button.append(text('span', 'strong', entry.name), text('span', 'example-source', `${sizeLabel(entry.bytes)} · ${ago(entry.time, now)} · ${settingsLine(entry.hash)}`));
  button.addEventListener('click', pick);
  return button;
}

function footer(kept: string, clear: (event: Event) => void): HTMLElement {
  const foot = document.createElement('div');
  foot.className = 'recent-foot';
  const button = text('button', 'btn btn-small', 'Clear') as HTMLButtonElement;
  button.type = 'button';
  button.id = 'recent-clear';
  button.tabIndex = -1;
  button.setAttribute('role', 'menuitem');
  button.addEventListener('click', clear);
  foot.append(text('span', 'hint', kept), button);
  return foot;
}

function stepFocus(menu: HTMLElement, key: string): boolean {
  const items = [...menu.querySelectorAll<HTMLElement>('[role=menuitem]')];
  const at = items.indexOf(document.activeElement as HTMLElement);
  const target = (() => {
    switch (key) {
      case 'ArrowDown':
        return items[(at + 1) % items.length];
      case 'ArrowUp':
        return items[(at - 1 + items.length) % items.length];
      case 'Home':
        return items[0];
      case 'End':
        return items.at(-1);
      default:
        return undefined;
    }
  })();
  target?.focus();
  return target !== undefined;
}

function reopen(store: Store<AppState>, loader: Loader, entry: RecentEntry): void {
  const saved = parseHash(entry.hash).settings;
  store.update((state) => ({
    ...state,
    settings: { ...saved, theme: state.settings.theme, view: state.settings.view, wrap: state.settings.wrap, input: state.settings.input },
  }));
  loader.load(entry.text, entry.name, REOPENED);
}

function recordRuns(store: Store<AppState>, list: Recents): void {
  store.subscribe((state, previous) => {
    const input = state.run.ranInput;
    if (state.run.status !== 'running' || previous.run.status === 'running' || input === null) {
      return;
    }
    const entry = entryOf(input, writeHash(state.settings), Date.now());
    window.setTimeout(() => list.record(entry), 0);
  });
}

export function mountRecentMenu(store: Store<AppState>, loader: Loader): void {
  const button = byId<HTMLButtonElement>('recent');
  const holder = button.parentElement ?? document.body;
  const list = recents(browserStorage());
  let menu: HTMLElement | null = null;
  const close = (refocus: boolean) => {
    menu?.remove();
    menu = null;
    button.setAttribute('aria-expanded', 'false');
    if (refocus) {
      button.focus();
    }
  };
  const render = () => {
    const entries = list.read();
    const now = Date.now();
    const element = menu ?? document.createElement('div');
    element.className = 'examples-menu recent-menu';
    element.id = 'recent-menu';
    element.setAttribute('role', 'menu');
    element.setAttribute('aria-label', 'Recent runs');
    element.replaceChildren(
      text('div', 'label examples-head', 'Recent runs'),
      ...(entries.length === 0 ? [text('p', 'hint recent-empty', EMPTY)] : entries.map((entry) => option(entry, now, () => {
        close(false);
        reopen(store, loader, entry);
      }))),
      footer(list.storage === null ? KEPT_IN_TAB : KEPT_HERE, (event) => {
        event.stopPropagation();
        list.clear();
        render();
        byId('recent-clear').focus();
      }),
    );
    return element;
  };
  button.addEventListener('click', () => {
    if (menu !== null) {
      close(false);
      return;
    }
    menu = render();
    holder.append(menu);
    button.setAttribute('aria-expanded', 'true');
    menu.querySelector<HTMLElement>('[role=menuitem]')?.focus();
    menu.addEventListener('keydown', (event) => {
      if (menu !== null && stepFocus(menu, event.key)) {
        event.preventDefault();
      }
    });
  });
  document.addEventListener('click', (event) => {
    if (menu !== null && event.target instanceof Node && !holder.contains(event.target)) {
      close(false);
    }
  });
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && menu !== null) {
      close(true);
    }
  });
  recordRuns(store, list);
}
