import type { Compartment } from '@codemirror/state';
import { EditorView, type ViewUpdate } from '@codemirror/view';
import { byId, setText } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import { currentBundle } from '../ui/module-tree';
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

class Language {
  readonly #deps: EditDeps;
  readonly #client: LangClient;
  readonly #problemsLabel = byId('output-problems');
  readonly #statusLabel = byId('output-lang');
  #status: LangStatus = 'loading';
  #problems = 0;

  constructor(deps: EditDeps) {
    this.#deps = deps;
    this.#client = new LangClient(() => new LangWorker() as unknown as WorkerLike, deps.module, {
      status: (status) => {
        this.#status = status;
        this.#paint();
      },
      analyzed: (analyzed) => {
        this.#problems = analyzed.problems.length;
        performance.mark('untwist:lang-analyzed', { detail: { ms: analyzed.ms, memoryBytes: analyzed.memoryBytes } });
        applyProblems(this.#deps.view, analyzed.problems);
        this.#paint();
      },
    });
    deps.view.dispatch({ effects: deps.gate.reconfigure(this.#extensions()) });
    deps.store.subscribe((state, previous) => {
      if (state.run.result !== previous.run.result || state.selectedModule !== previous.selectedModule) {
        this.#analyse();
      }
    });
    this.#analyse();
    this.#paint();
  }

  #extensions() {
    const { store, view } = this.#deps;
    const hooks: LangHooks = { client: this.#client, host: moduleHost(store, this.#client, view) };
    const rename = renaming(hooks);
    return [
      navigation(hooks, rename.open),
      rename.extension,
      hovering(hooks, rename.open),
      langLook,
      EditorView.updateListener.of((update) => this.#updated(update)),
    ];
  }

  #updated(update: ViewUpdate): void {
    if (update.docChanged) {
      this.#client.changed(() => update.view.state.doc.toString());
    }
  }

  #analyse(): void {
    const view = this.#deps.view;
    this.#client.changed(() => view.state.doc.toString());
    this.#client.flush();
  }

  #paint(): void {
    const shown = this.#status === 'ready' ? this.#problems : 0;
    this.#problemsLabel.hidden = false;
    this.#statusLabel.hidden = false;
    setText(this.#problemsLabel, problemsText(shown));
    this.#problemsLabel.dataset['count'] = String(shown);
    setText(this.#statusLabel, statusText(this.#status, this.#client.capBytes));
  }
}

export function attachLanguage(deps: EditDeps): void {
  new Language(deps);
}
