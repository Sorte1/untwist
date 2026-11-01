import { byId, setText } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';

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
  try {
    const value = JSON.parse(report) as { bundle?: BundleInfo };
    return value.bundle !== undefined && Array.isArray(value.bundle.modules) ? value.bundle : null;
  } catch {
    return null;
  }
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

export function mountModuleTree(store: Store<AppState>): void {
  const nav = byId('module-tree');
  const list = byId('module-list');
  let shown: BundleInfo | null = null;
  const pick = (index: number | null) => () => store.update((state) => ({ ...state, selectedModule: index }));
  store.subscribe((state, previous) => {
    if (state.run.result === previous.run.result && state.selectedModule === previous.selectedModule && shown !== null) {
      return;
    }
    shown = currentBundle(state);
    nav.hidden = shown === null;
    if (shown === null) {
      list.replaceChildren();
      return;
    }
    const modules = shown.modules;
    setText(byId('module-count'), `bundle · ${modules.length} ${modules.length === 1 ? 'module' : 'modules'}`);
    list.replaceChildren(
      entryButton('bundle', state.selectedModule === null, false, pick(null)),
      ...treeOrder(modules).map((index) => {
        const module = modules[index] as ModuleInfo;
        return entryButton(module.path, state.selectedModule === index, module.isEntry, pick(index));
      }),
    );
  });
}
