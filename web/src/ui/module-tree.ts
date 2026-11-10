import { byId, setText } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import { reportField } from '../report-json';

export interface ModuleInfo {
  readonly id: string;
  readonly path: string;
  readonly isEntry: boolean;
  readonly span?: readonly [number, number];
}

export interface BundleInfo {
  readonly kind: string;
  readonly modules: readonly ModuleInfo[];
}

export function bundleOf(report: string): BundleInfo | null {
  const bundle = reportField(report, 'bundle') as { modules?: unknown } | null | undefined;
  return typeof bundle === 'object' && bundle !== null && Array.isArray(bundle.modules) ? (bundle as BundleInfo) : null;
}

export function treeOrder(modules: readonly ModuleInfo[]): number[] {
  return modules
    .map((module, index) => ({ module, index }))
    .sort((a, b) => Number(b.module.isEntry) - Number(a.module.isEntry) || (a.module.path < b.module.path ? -1 : a.module.path > b.module.path ? 1 : 0))
    .map((item) => item.index);
}

function entryButton(label: string, current: boolean, isEntry: boolean, pick: () => void): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'tree-item';
  button.setAttribute('aria-current', String(current));
  const name = document.createElement('span');
  name.className = 'tree-name';
  name.textContent = label;
  button.append(name);
  if (isEntry) {
    button.append(Object.assign(document.createElement('span'), { className: 'tree-entry', textContent: 'entry' }));
  }
  button.addEventListener('click', pick);
  return button;
}

export function currentBundle(state: AppState): BundleInfo | null {
  const result = state.run.result;
  return result === null || result.modules.length === 0 ? null : bundleOf(result.report);
}

export const TREE_PAGE = 500;

export interface ShownModules {
  readonly shown: readonly number[];
  readonly more: number;
}

export function shownModules(order: readonly number[], limit: number): ShownModules {
  return { shown: order.slice(0, limit), more: Math.max(0, order.length - limit) };
}

export function limitFor(order: readonly number[], selected: number | null, limit: number): number {
  const at = selected === null ? -1 : order.indexOf(selected);
  return at < limit ? limit : (Math.floor(at / TREE_PAGE) + 1) * TREE_PAGE;
}

function moreButton(more: number, grow: () => void): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'tree-item tree-more';
  button.textContent = `Show ${Math.min(more, TREE_PAGE).toLocaleString('en-US')} more of ${more.toLocaleString('en-US')}`;
  button.addEventListener('click', grow);
  return button;
}

interface Drawn {
  readonly bundle: BundleInfo;
  readonly order: readonly number[];
  readonly limit: number;
  readonly items: ReadonlyMap<number | null, HTMLButtonElement>;
}

export function mountModuleTree(store: Store<AppState>): void {
  const nav = byId('module-tree');
  const list = byId('module-list');
  let drawn: Drawn | null = null;
  let selected: number | null = null;
  const pick = (index: number | null) => () => store.update((state) => ({ ...state, selectedModule: index }));
  const draw = (bundle: BundleInfo, order: readonly number[], limit: number) => {
    const { shown, more } = shownModules(order, limit);
    const items = new Map<number | null, HTMLButtonElement>([[null, entryButton('bundle', selected === null, false, pick(null))]]);
    for (const index of shown) {
      const module = bundle.modules[index] as ModuleInfo;
      items.set(index, entryButton(module.path, selected === index, module.isEntry, pick(index)));
    }
    const fragment = document.createDocumentFragment();
    for (const item of items.values()) {
      fragment.append(item);
    }
    if (more > 0) {
      fragment.append(moreButton(more, () => draw(bundle, order, limit + TREE_PAGE)));
    }
    list.replaceChildren(fragment);
    drawn = { bundle, order, limit, items };
  };
  store.subscribe((state, previous) => {
    if (state.run.result === previous.run.result && state.selectedModule === previous.selectedModule && drawn !== null) {
      return;
    }
    const bundle = currentBundle(state);
    const was = selected;
    selected = state.selectedModule;
    nav.hidden = bundle === null;
    if (bundle === null) {
      drawn = null;
      list.replaceChildren();
      return;
    }
    if (drawn !== null && drawn.bundle === bundle) {
      const limit = limitFor(drawn.order, selected, drawn.limit);
      if (limit === drawn.limit) {
        drawn.items.get(was)?.setAttribute('aria-current', 'false');
        drawn.items.get(selected)?.setAttribute('aria-current', 'true');
      } else {
        draw(bundle, drawn.order, limit);
      }
      return;
    }
    const modules = bundle.modules;
    setText(byId('module-count'), `bundle · ${modules.length} ${modules.length === 1 ? 'module' : 'modules'}`);
    const order = treeOrder(modules);
    draw(bundle, order, limitFor(order, selected, TREE_PAGE));
  });
}
