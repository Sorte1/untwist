import './styles/tokens.css';
import './styles/editor-themes.css';
import './styles/site.css';
import './styles/layout.css';
import { byId } from './dom';
import { Cancelled, Engine, REPORT_EXTRAS } from './engine/engine';
import { initialState, runCancelled, runFailed, runFinished, runStarted, stagesAdded, stagesClosed, ticked, type AppState, type InputState } from './state';
import { FOLDS_MARK, MARK, type StageMark } from './progress';
import type { RunResult } from './engine/types';
import { hashSetsLibraries, storedLibraryNames, storeLibraryNames } from './settings/library-pref';
import { flagsFrom } from './settings/flags';
import { LibraryTables } from './libraries/tables';
import { registerServiceWorker } from './pwa/register';
import { settingsFromHash, writeHash } from './settings/hash';
import { createStore, type Store } from './store';
import { applyTheme, preferredTheme } from './theme';
import { mountCompareModes } from './ui/compare-modes';
import { mountDiffView } from './ui/diff-view';
import { mountDownload } from './ui/download';
import { mountDiagnostics } from './ui/diagnostics';
import { mountDiagnosticsPanel } from './ui/diagnostics-panel';
import { mountDropOverlay } from './ui/drop-overlay';
import { mountEditorTheme } from './ui/editor-theme';
import { mountExamplesMenu } from './ui/examples-menu';
import { mountFocusMode } from './ui/focus-mode';
import { mountLoader } from './ui/load-input';
import { mountInputPane } from './ui/input-pane';
import { mountIntro } from './ui/intro';
import { mountModeSwitch } from './ui/mode-switch';
import { mountStringsBanner } from './ui/strings-banner';
import { mountSplitBanner } from './ui/split-banner';
import { mountModuleTree } from './ui/module-tree';
import { mountModuleSearch } from './ui/module-search';
import { mountOptions } from './ui/options';
import { mountOutputPane } from './ui/output-pane';
import { mountPaletteKey } from './ui/palette-key';
import { mountPasses } from './ui/passes';
import { mountRecentMenu } from './ui/recent-menu';
import { mountRunProgress } from './ui/run-progress';
import { mountSettingsPanel } from './ui/settings-panel';
import { mountShareLink } from './ui/share-link';
import { mountSteps } from './ui/steps-view';
import { mountStringsTab } from './ui/strings-tab';
import { mountLibrariesTab } from './ui/libraries-tab';
import { mountLinks } from './ui/link-view';
import { mountStatusBar } from './ui/status-bar';
import { mountDetailsToggle } from './ui/details/toggle';
import { mountSummaryChip } from './ui/summary-chip';
import { mountToolbar } from './ui/toolbar';
import { mountWasmBar } from './ui/wasm-bar';

const TICK_MS = 100;
const PAINT_WAIT_MS = 100;
const IDLE_WAIT_MS = 3000;

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

interface Marks {
  readonly add: (mark: StageMark) => void;
  readonly flush: () => void;
}

function bufferedMarks(store: Store<AppState>): Marks {
  let pending: StageMark[] = [];
  return {
    add: (mark) => {
      pending = [...pending, mark];
    },
    flush: () => {
      if (pending.length > 0) {
        const batch = pending;
        pending = [];
        store.update((state) => stagesAdded(state, batch));
      }
    },
  };
}

function startClock(store: Store<AppState>, marks: Marks): void {
  let last = 0;
  const frame = (now: number) => {
    if (store.get().run.status !== 'running') {
      return;
    }
    marks.flush();
    if (now - last >= TICK_MS) {
      last = now;
      store.update((state) => ticked(state, performance.now()));
    }
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

function afterPaint(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => window.setTimeout(resolve, 0));
    window.setTimeout(resolve, PAINT_WAIT_MS);
  });
}

function foldsSince(started: number): StageMark[] {
  const entry = performance.getEntriesByName(FOLDS_MARK).at(-1);
  return entry !== undefined && entry.startTime >= started ? [{ name: MARK.folds, at: entry.startTime }] : [];
}

async function deliver(store: Store<AppState>, marks: Marks, result: RunResult): Promise<void> {
  marks.add({ name: MARK.editor, at: performance.now() });
  marks.flush();
  store.update((state) => ticked(state, performance.now()));
  await afterPaint();
  const started = performance.now();
  store.update((state) => runFinished(state, result, performance.now()));
  const ended = performance.now();
  store.update((state) => stagesClosed(state, [...foldsSince(started), { name: MARK.end, at: ended }], ended));
}

function starter(store: Store<AppState>, engine: Engine): (input: InputState, flags: string) => void {
  return (input, flags) => {
    const state = store.get();
    if (state.engine !== 'ready' || state.run.status === 'running') {
      return;
    }
    store.update((current) => runStarted(current, performance.now(), flags, input));
    const marks = bufferedMarks(store);
    startClock(store, marks);
    engine.run(input.text, flags, marks.add, REPORT_EXTRAS, store.get().wasm).then(
      (result) => deliver(store, marks, result),
      (error: unknown) => {
        if (!(error instanceof Cancelled)) {
          store.update((current) => runFailed(current, message(error), performance.now()));
        }
      },
    );
  };
}

function initialSettings(hash: string): ReturnType<typeof settingsFromHash> {
  const parsed = settingsFromHash(hash);
  const stored = hashSetsLibraries(hash) ? null : storedLibraryNames();
  return stored === null ? parsed : { ...parsed, settings: { ...parsed.settings, libs: stored } };
}

function runner(store: Store<AppState>, start: (input: InputState, flags: string) => void): () => void {
  return () => {
    const state = store.get();
    start(state.input, flagsFrom(state.settings));
  };
}

function whenIdle(task: () => void): void {
  const idle = window.requestIdleCallback as typeof window.requestIdleCallback | undefined;
  if (idle === undefined) {
    window.setTimeout(task, PAINT_WAIT_MS);
  } else {
    idle.call(window, task, { timeout: IDLE_WAIT_MS });
  }
}

function showStarsLater(engineReady: Promise<unknown>): void {
  const show = () =>
    whenIdle(() => {
      import('./github/stars').then(
        ({ showStars }) => showStars(byId('github-link'), byId('github-stars')),
        () => undefined,
      );
    });
  engineReady.then(show, show);
}

function main(): void {
  const parsed = initialSettings(window.location.hash);
  const store = createStore(initialState(preferredTheme(), parsed.settings, parsed.problems));
  const libraries = new LibraryTables();
  const engine = new Engine(undefined, true, libraries);
  applyTheme(store.get().theme);
  mountEditorTheme(store);
  store.subscribe((state, previous) => {
    if (state.theme !== previous.theme) {
      applyTheme(state.theme);
    }
    if (state.settings.libs !== previous.settings.libs) {
      storeLibraryNames(state.settings.libs);
    }
    if (state.settings !== previous.settings) {
      const hash = writeHash(state.settings);
      window.history.replaceState(null, '', hash === '' ? window.location.pathname + window.location.search : hash);
    }
  });
  window.addEventListener('hashchange', () => {
    const next = initialSettings(window.location.hash);
    store.update((state) => ({ ...state, settings: next.settings, settingsProblems: next.problems, theme: next.settings.theme ?? state.theme }));
  });
  const start = starter(store, engine);
  const run = runner(store, start);
  const compare = mountCompareModes(store, new Engine(engine.module, false, libraries));
  const steps = mountSteps(store, new Engine(engine.module, false, libraries));
  const cancel = () => {
    engine.cancel();
    compare.cancel();
    steps.cancel();
    store.update((state) => runCancelled(state, performance.now()));
  };
  mountToolbar(store, { run, cancel });
  const inputView = mountInputPane(store, engine.module);
  const outputView = mountOutputPane(store, engine.module);
  mountModuleTree(store);
  mountModuleSearch(store, outputView);
  mountDownload(store, () => outputView.state.doc.toString());
  mountDiffView(store, () => outputView.state.doc.toString());
  mountStatusBar(store);
  mountRunProgress(store);
  mountIntro(store);
  mountDiagnostics(store, inputView);
  mountFocusMode(store);
  const loader = mountLoader(store, run);
  const addWasm = mountWasmBar(store, run);
  const pickExample = mountExamplesMenu(store, loader);
  mountRecentMenu(store, loader);
  mountDropOverlay(store, loader, addWasm);
  mountShareLink(store, loader);
  mountSummaryChip(store);
  mountOptions(store);
  mountPasses(store);
  mountSettingsPanel(store);
  mountDiagnosticsPanel();
  mountModeSwitch(store);
  mountStringsBanner(store, start, inputView);
  mountSplitBanner(store, start);
  mountStringsTab(store, inputView);
  mountLibrariesTab(store, outputView, libraries, run);
  mountLinks(store, inputView, outputView);
  mountDetailsToggle(store, () => flagsFrom(store.get().settings));
  mountPaletteKey(store, { run, pickExample });
  requestAnimationFrame(() => performance.mark('untwist:editors-painted'));
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      run();
    }
  });
  store.update((state) => ({ ...state }));
  engine.ready().then(
    () => {
      performance.mark('untwist:engine-ready');
      store.update((state) => ({ ...state, engine: 'ready' }));
    },
    (error: unknown) => store.update((state) => ({ ...state, engine: 'failed', engineError: message(error) })),
  );
  registerServiceWorker(engine.ready());
  showStarsLater(engine.ready());
}

main();
