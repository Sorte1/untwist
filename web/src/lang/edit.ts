import './lang.css';
import { history, historyKeymap } from '@codemirror/commands';
import { setDiagnostics } from '@codemirror/lint';
import { ChangeSet, type Compartment, type Text } from '@codemirror/state';
import { EditorView, keymap, type ViewUpdate } from '@codemirror/view';
import { byId, linesLabel, setText } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import { access } from '../ui/editor';
import { currentBundle } from '../ui/module-tree';
import { shownText } from '../ui/output-pane';
import { LangClient, resolveModule, type LangStatus, type WorkerLike } from './client';
import { applyProblems, navigation, within, type LangHooks, type ModuleHost } from './extensions';
import LangWorker from './lang.worker.ts?worker';
import { langLook } from './look';
import { hovering, renaming } from './tooltips';

export interface EditDeps {
  readonly view: EditorView;
  readonly gate: Compartment;
  readonly store: Store<AppState>;
  readonly module: Promise<WebAssembly.Module>;
}

export interface EditMode {
  toggle(): void;
}

export function editedLines(changes: ChangeSet, doc: Text): number {
  let count = 0;
  let last = 0;
  changes.iterChangedRanges((_fromA, _toA, fromB, toB) => {
    const first = Math.max(doc.lineAt(fromB).number, last + 1);
    const end = doc.lineAt(toB).number;
    if (end >= first) {
      count += end - first + 1;
      last = end;
    }
  });
  return count;
}

function megabytes(bytes: number): string {
  return `${Math.round(bytes / (1024 * 1024))} MB`;
}

export function statusText(status: LangStatus, capBytes: number | null): string {
  switch (status) {
    case 'loading':
      return 'Language features · loading';
    case 'analysing':
      return 'Language features · analysing';
    case 'ready':
      return 'Language features · ready';
    case 'off':
      return `Language features are off above ${megabytes(capBytes ?? 0)}`;
    case 'failed':
      return 'Language features failed to load';
  }
}

export function problemsText(count: number): string {
  if (count === 0) {
    return 'No problems';
  }
  return `${count.toLocaleString('en-US')} ${count === 1 ? 'problem' : 'problems'}`;
}

function moduleHost(store: Store<AppState>, client: LangClient, view: EditorView): ModuleHost {
  return {
    async open(module, name) {
      const state = store.get();
      const bundle = currentBundle(state);
      const result = state.run.result;
      if (bundle === null || result === null) {
        return false;
      }
      const paths = bundle.modules.map((entry) => entry.path);
      const from = state.selectedModule === null ? null : (paths[state.selectedModule] ?? null);
      const target = resolveModule(from, module, paths);
      if (target === null) {
        return false;
      }
      const index = paths.indexOf(target);
      const span = name === '*' ? [] : await client.exportSpan(result.modules[index] ?? '', name);
      store.update((current) => ({ ...current, selectedModule: index }));
      const { from: start, to } = within(view.state, span[0] ?? 0, span[1] ?? 0);
      view.dispatch({ selection: { anchor: start, head: to }, scrollIntoView: true });
      view.focus();
      return true;
    },
  };
}

interface Footer {
  readonly meta: HTMLElement;
  readonly mark: HTMLElement;
  readonly problems: HTMLElement;
  readonly status: HTMLElement;
  readonly actions: HTMLElement;
  readonly reset: HTMLButtonElement;
  readonly readOnly: HTMLElement;
}

function footer(): Footer {
  const meta = byId('output-edit-meta');
  const mark = Object.assign(document.createElement('span'), { className: 'edit-mark' });
  const problems = Object.assign(document.createElement('span'), { className: 'edit-problems' });
  const status = document.createElement('span');
  meta.replaceChildren(mark, problems, status);
  const actions = byId('output-edit-actions');
  const reset = Object.assign(document.createElement('button'), { type: 'button', className: 'btn', textContent: 'Reset to output' });
  actions.replaceChildren(Object.assign(document.createElement('span'), { textContent: 'Run replaces your edits' }), reset);
  return { meta, mark, problems, status, actions, reset, readOnly: byId('output-readonly') };
}

class Controller implements EditMode {
  readonly #deps: EditDeps;
  readonly #footer = footer();
  readonly #client: LangClient;
  #on = false;
  #changes: ChangeSet;
  #status: LangStatus = 'loading';
  #problems = 0;

  constructor(deps: EditDeps) {
    this.#deps = deps;
    this.#changes = ChangeSet.empty(deps.view.state.doc.length);
    this.#client = new LangClient(() => new LangWorker() as unknown as WorkerLike, deps.module, {
      status: (status) => {
        this.#status = status;
        this.#paint();
      },
      analyzed: (analyzed) => {
        this.#problems = analyzed.problems.length;
        performance.mark('untwist:lang-analyzed', { detail: { ms: analyzed.ms, memoryBytes: analyzed.memoryBytes } });
        if (this.#on) {
          applyProblems(this.#deps.view, analyzed.problems);
        }
        this.#paint();
      },
    });
    this.#footer.reset.addEventListener('click', () => this.#reset());
    deps.store.subscribe((state, previous) => {
      if (state.run.result !== previous.run.result || state.selectedModule !== previous.selectedModule) {
        this.#rebase();
      }
    });
  }

  toggle(): void {
    this.#on = !this.#on;
    const { view, gate } = this.#deps;
    view.dispatch({ effects: gate.reconfigure(this.#on ? this.#extensions() : access(false)) });
    if (this.#on) {
      this.#analyse();
      view.focus();
    } else {
      view.dispatch(setDiagnostics(view.state, []));
    }
    this.#paint();
  }

  #extensions() {
    const { store, view } = this.#deps;
    const hooks: LangHooks = { client: this.#client, host: moduleHost(store, this.#client, view) };
    const rename = renaming(hooks);
    return [
      access(true),
      history(),
      keymap.of(historyKeymap),
      navigation(hooks, rename.open),
      rename.extension,
      hovering(hooks, rename.open),
      langLook,
      EditorView.updateListener.of((update) => this.#updated(update)),
    ];
  }

  #updated(update: ViewUpdate): void {
    if (!update.docChanged) {
      return;
    }
    this.#changes = this.#changes.compose(update.changes);
    this.#client.changed(() => update.view.state.doc.toString());
    this.#paint();
  }

  #analyse(): void {
    const view = this.#deps.view;
    this.#client.changed(() => view.state.doc.toString());
    this.#client.flush();
  }

  #rebase(): void {
    this.#changes = ChangeSet.empty(this.#deps.view.state.doc.length);
    if (this.#on) {
      this.#analyse();
    }
    this.#paint();
  }

  #reset(): void {
    const view = this.#deps.view;
    const shown = shownText(this.#deps.store.get()).text;
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: shown }, selection: { anchor: 0 } });
    this.#changes = ChangeSet.empty(view.state.doc.length);
    if (this.#on) {
      this.toggle();
    }
    this.#paint();
  }

  #paint(): void {
    const { meta, mark, problems, status, actions, readOnly } = this.#footer;
    meta.hidden = !this.#on;
    actions.hidden = !this.#on;
    readOnly.hidden = this.#on;
    byId('edit-output').setAttribute('aria-pressed', String(this.#on));
    const edited = this.#changes.empty ? 0 : editedLines(this.#changes, this.#deps.view.state.doc);
    setText(mark, edited === 0 && this.#changes.empty ? 'No edits yet' : `Edited · ${linesLabel(edited)}`);
    const shown = this.#status === 'ready' ? this.#problems : 0;
    setText(problems, problemsText(shown));
    problems.dataset['count'] = String(shown);
    setText(status, statusText(this.#status, this.#client.capBytes));
  }
}

export function createEditMode(deps: EditDeps): EditMode {
  return new Controller(deps);
}
