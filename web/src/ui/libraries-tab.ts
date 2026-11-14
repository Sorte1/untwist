import { EditorSelection } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { byId, setText } from '../dom';
import { librariesOf, type LibraryRow } from '../libraries/libraries';
import type { ModuleHit } from '../libraries/locate';
import type { LibraryTables } from '../libraries/tables';
import type { TableIndex } from '../libraries/index-format';
import type { AppState } from '../state';
import type { Store } from '../store';
import type { LibrariesView } from './libraries/panel';
import { currentBundle } from './module-tree';

type Render = (holder: HTMLElement, view: LibrariesView) => void;

interface Painted {
  readonly result: unknown;
  readonly libs: boolean;
  readonly index: TableIndex | null;
}

export function revealModule(store: Store<AppState>, view: EditorView, hit: ModuleHit): void {
  store.update((state) => ({
    ...state,
    selectedModule: hit.file,
    settings: state.settings.view === 'output' ? state.settings : { ...state.settings, view: 'output' },
  }));
  const doc = view.state.doc;
  const line = doc.line(Math.min(Math.max(1, hit.line), doc.lines));
  view.dispatch({ selection: EditorSelection.single(line.from, line.to), effects: EditorView.scrollIntoView(line.from, { y: 'center' }) });
  view.focus();
}

function rowsOf(state: AppState): LibraryRow[] {
  const result = state.run.result;
  return result === null ? [] : librariesOf(result.report);
}

export function mountLibrariesTab(store: Store<AppState>, output: EditorView, tables: LibraryTables, run: () => void): void {
  const tab = byId<HTMLButtonElement>('libraries-tab');
  const panel = byId('libraries-panel');
  const count = byId('libraries-count');
  let render: Render | null = null;
  let open = false;
  let index: TableIndex | null = null;
  let painted: Painted | null = null;
  let rows: LibraryRow[] = [];
  const turnOn = () => {
    store.update((state) => ({ ...state, settings: { ...state.settings, libs: true } }));
    run();
  };
  const paint = () => {
    const state = store.get();
    if (!open || render === null) {
      return;
    }
    const next: Painted = { result: state.run.result, libs: state.settings.libs, index };
    if (painted !== null && painted.result === next.result && painted.libs === next.libs && painted.index === next.index) {
      return;
    }
    painted = next;
    render(panel, {
      libraries: state.settings.libs,
      rows,
      result: state.run.result,
      index,
      reveal: (hit) => revealModule(store, output, hit),
      fileName: (file) => currentBundle(store.get())?.modules[file]?.path ?? `module ${file + 1}`,
      turnOn,
    });
  };
  const show = (next: boolean) => {
    open = next;
    painted = null;
    tab.setAttribute('aria-expanded', String(open));
    panel.hidden = !open;
    if (!open) {
      panel.replaceChildren();
      return;
    }
    tables.index().then((loaded) => {
      index = loaded;
      paint();
    }, () => undefined);
    if (render !== null) {
      paint();
      return;
    }
    import('./libraries/panel').then(
      (module) => {
        render = module.renderLibraries;
        paint();
      },
      () => panel.replaceChildren(document.createTextNode('The libraries panel failed to load.')),
    );
  };
  tab.addEventListener('click', () => show(!open));
  store.subscribe((state, previous) => {
    if (state.run.result !== previous.run.result) {
      rows = rowsOf(state);
      const total = rows.reduce((sum, row) => sum + row.modules, 0);
      setText(count, rows.length === 0 ? '' : String(rows.length));
      tab.title = rows.length === 0 ? 'Libraries' : `${rows.length} libraries, ${total} modules`;
    }
    if (state.run.result !== previous.run.result || state.settings.libs !== previous.settings.libs) {
      paint();
    }
  });
}
