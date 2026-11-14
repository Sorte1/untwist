import { byId, setText } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import { reportField } from '../report-json';
import { countLabel, groupModules, libraryLabel, libraryRanges, type ModuleGroups } from './module-groups';

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

function entryButton(label: string, current: boolean, isEntry: boolean, pick: () => void, library: string | null = null): HTMLButtonElement {
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
  if (library !== null) {
    button.title = `${label} (library: ${library})`;
    button.append(Object.assign(document.createElement('span'), { className: 'tree-lib-badge', textContent: 'lib' }));
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

interface Section {
  readonly key: string;
  readonly order: readonly number[];
  readonly library: string | null;
}

interface Layout {
  readonly sections: readonly Section[];
  readonly groups: ModuleGroups | null;
}

export function layoutOf(bundle: BundleInfo, ranges: ReadonlyMap<string, string>): Layout {
  const order = treeOrder(bundle.modules);
  const groups = groupModules(bundle.modules, order, ranges);
  if (groups === null) {
    return { sections: [{ key: '', order, library: null }], groups };
  }
  const libraries = groups.libraries.map((group) => ({ key: `lib:${group.name}`, order: group.order, library: group.name }));
  return { sections: [{ key: 'app', order: groups.app, library: null }, ...libraries], groups };
}

function sectionOf(layout: Layout, index: number | null): Section | null {
  return index === null ? null : (layout.sections.find((section) => section.order.includes(index)) ?? null);
}

function group(key: string, label: string, count: number, open: boolean, remember: (key: string, open: boolean) => void): HTMLDetailsElement {
  const details = document.createElement('details');
  details.className = 'tree-group';
  details.dataset['group'] = key;
  details.open = open;
  const summary = document.createElement('summary');
  summary.className = 'tree-group-head';
  summary.append(
    Object.assign(document.createElement('span'), { className: 'tree-group-name', textContent: label }),
    Object.assign(document.createElement('span'), { className: 'tree-group-count', textContent: countLabel(count) }),
  );
  details.append(summary);
  details.addEventListener('toggle', () => remember(key, details.open));
  return details;
}

function openAncestors(item: HTMLElement | undefined): void {
  for (let at = item?.parentElement?.closest('details') ?? null; at !== null; at = at.parentElement?.closest('details') ?? null) {
    at.open = true;
  }
}

interface Drawn {
  readonly bundle: BundleInfo;
  readonly layout: Layout;
  readonly limits: ReadonlyMap<string, number>;
  readonly items: ReadonlyMap<number | null, HTMLButtonElement>;
}

export function mountModuleTree(store: Store<AppState>): void {
  const nav = byId('module-tree');
  const list = byId('module-list');
  let drawn: Drawn | null = null;
  let selected: number | null = null;
  let opened = new Map<string, boolean>();
  const remember = (key: string, open: boolean) => {
    opened = new Map(opened).set(key, open);
  };
  const isOpen = (key: string, fallback: boolean) => opened.get(key) ?? fallback;
  const pick = (index: number | null) => () => store.update((state) => ({ ...state, selectedModule: index }));
  const draw = (bundle: BundleInfo, layout: Layout, limits: ReadonlyMap<string, number>) => {
    const items = new Map<number | null, HTMLButtonElement>([[null, entryButton('bundle', selected === null, false, pick(null))]]);
    const sectionNodes = new Map<string, DocumentFragment>();
    for (const section of layout.sections) {
      const limit = limits.get(section.key) ?? TREE_PAGE;
      const { shown, more } = shownModules(section.order, limit);
      const fragment = document.createDocumentFragment();
      const prefix = section.library === null ? '' : `node_modules/${section.library}/`;
      for (const index of shown) {
        const module = bundle.modules[index] as ModuleInfo;
        const label = prefix !== '' && module.path.startsWith(prefix) ? module.path.slice(prefix.length) : module.path;
        const item = entryButton(label, selected === index, module.isEntry, pick(index), section.library);
        items.set(index, item);
        fragment.append(item);
      }
      if (more > 0) {
        fragment.append(moreButton(more, () => draw(bundle, layout, new Map(limits).set(section.key, limit + TREE_PAGE))));
      }
      sectionNodes.set(section.key, fragment);
    }
    const root = document.createDocumentFragment();
    root.append(items.get(null) as HTMLButtonElement);
    if (layout.groups === null) {
      root.append(sectionNodes.get('') ?? '');
    } else {
      const app = group('app', 'App', layout.groups.app.length, isOpen('app', true), remember);
      app.append(sectionNodes.get('app') ?? '');
      const libraryCount = layout.groups.libraries.reduce((sum, library) => sum + library.order.length, 0);
      const libraries = group('libraries', 'Libraries', libraryCount, isOpen('libraries', false), remember);
      for (const library of layout.groups.libraries) {
        const key = `lib:${library.name}`;
        const node = group(key, libraryLabel(library), library.order.length, isOpen(key, false), remember);
        node.classList.add('tree-library');
        node.append(sectionNodes.get(key) ?? '');
        libraries.append(node);
      }
      root.append(app, libraries);
    }
    list.replaceChildren(root);
    openAncestors(items.get(selected));
    drawn = { bundle, layout, limits, items };
  };
  const limitsFor = (layout: Layout, limits: ReadonlyMap<string, number>): ReadonlyMap<string, number> => {
    const section = sectionOf(layout, selected);
    if (section === null) {
      return limits;
    }
    const current = limits.get(section.key) ?? TREE_PAGE;
    const limit = limitFor(section.order, selected, current);
    return limit === current ? limits : new Map(limits).set(section.key, limit);
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
      const limits = limitsFor(drawn.layout, drawn.limits);
      if (limits === drawn.limits) {
        drawn.items.get(was)?.setAttribute('aria-current', 'false');
        drawn.items.get(selected)?.setAttribute('aria-current', 'true');
        openAncestors(drawn.items.get(selected));
      } else {
        draw(bundle, drawn.layout, limits);
      }
      return;
    }
    const modules = bundle.modules;
    setText(byId('module-count'), `bundle · ${modules.length} ${modules.length === 1 ? 'module' : 'modules'}`);
    opened = new Map();
    const layout = layoutOf(bundle, libraryRanges(state.run.result?.report ?? ''));
    draw(bundle, layout, limitsFor(layout, new Map()));
  });
}
