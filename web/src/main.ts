import './styles/tokens.css';
import './styles/layout.css';
import { Cancelled, Engine } from './engine/engine';
import { initialState, runCancelled, runFailed, runFinished, runStarted, stageBegan, ticked, type AppState, type InputState } from './state';
import { flagsFrom } from './settings/flags';
import { registerServiceWorker } from './pwa/register';
import { parseHash, writeHash } from './settings/hash';
import { createStore, type Store } from './store';
import { applyTheme, preferredTheme } from './theme';
import { mountCompareModes } from './ui/compare-modes';
import { mountDiffView } from './ui/diff-view';
import { mountDownload } from './ui/download';
import { mountDiagnostics } from './ui/diagnostics';
import { mountDiagnosticsPanel } from './ui/diagnostics-panel';
import { mountDropOverlay } from './ui/drop-overlay';
import { mountExamplesMenu } from './ui/examples-menu';
import { mountFocusMode } from './ui/focus-mode';
import { mountLoader } from './ui/load-input';
import { mountInputPane } from './ui/input-pane';
import { mountModeSwitch } from './ui/mode-switch';
import { mountStringsBanner } from './ui/strings-banner';
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
import { mountStatusBar } from './ui/status-bar';
import { mountDetailsToggle } from './ui/details/toggle';
import { mountSummaryChip } from './ui/summary-chip';
import { mountToolbar } from './ui/toolbar';

const TICK_MS = 100;

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function startClock(store: Store<AppState>): void {
  let last = 0;
  const frame = (now: number) => {
    if (store.get().run.status !== 'running') {
      return;
    }
    if (now - last >= TICK_MS) {
      last = now;
      store.update((state) => ticked(state, performance.now()));
    }
    requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

function starter(store: Store<AppState>, engine: Engine): (input: InputState, flags: string) => void {
  return (input, flags) => {
    const state = store.get();
    if (state.engine !== 'ready' || state.run.status === 'running') {
      return;
    }
    store.update((current) => runStarted(current, performance.now(), flags, input));
    startClock(store);
    const staged = (name: string) => store.update((current) => stageBegan(current, name, performance.now()));
    engine.run(input.text, flags, staged).then(
      (result) => store.update((current) => runFinished(current, result, performance.now())),
      (error: unknown) => {
        if (!(error instanceof Cancelled)) {
          store.update((current) => runFailed(current, message(error), performance.now()));
        }
      },
    );
  };
}

function runner(store: Store<AppState>, start: (input: InputState, flags: string) => void): () => void {
  return () => {
    const state = store.get();
    start(state.input, flagsFrom(state.settings));
  };
}

function main(): void {
  const parsed = parseHash(window.location.hash);
  const store = createStore(initialState(preferredTheme(), parsed.settings, parsed.problems));
  const engine = new Engine();
  applyTheme(store.get().theme);
  store.subscribe((state, previous) => {
    if (state.theme !== previous.theme) {
      applyTheme(state.theme);
    }
    if (state.settings !== previous.settings) {
      const hash = writeHash(state.settings);
      window.history.replaceState(null, '', hash === '' ? window.location.pathname + window.location.search : hash);
    }
  });
  window.addEventListener('hashchange', () => {
    const next = parseHash(window.location.hash);
    store.update((state) => ({ ...state, settings: next.settings, settingsProblems: next.problems, theme: next.settings.theme ?? state.theme }));
  });
  const start = starter(store, engine);
  const run = runner(store, start);
  const compare = mountCompareModes(store, new Engine(engine.module, false));
  const steps = mountSteps(store, new Engine(engine.module, false));
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
  mountDiagnostics(store, inputView);
  mountFocusMode(store);
  const loader = mountLoader(store, run);
  const pickExample = mountExamplesMenu(store, loader);
  mountRecentMenu(store, loader);
  mountDropOverlay(store, loader);
  mountShareLink(store, loader);
  mountSummaryChip(store);
  mountOptions(store);
  mountPasses(store);
  mountSettingsPanel(store);
  mountDiagnosticsPanel();
  mountModeSwitch(store);
  mountStringsBanner(store, start, inputView);
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
}

main();
