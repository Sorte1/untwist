import { Compartment } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { byId, linesLabel, setText } from '../dom';
import { seconds, type AppState } from '../state';
import type { Store } from '../store';
import { currentBundle } from './module-tree';
import { codeEditing } from './editing';
import { baseExtensions, makeEditor, outputLook } from './editor';
import { outputEdits } from './output-edits';

const COPIED_MS = 1200;

export function shownText(state: AppState): { readonly text: string; readonly name: string } {
  const result = state.run.result;
  const index = state.selectedModule;
  if (result === null || index === null) {
    return { text: result?.stdout ?? '', name: 'output.js' };
  }
  const bundle = currentBundle(state);
  return { text: result.modules[index] ?? '', name: bundle?.modules[index]?.path ?? `module ${index}` };
}

function mountLanguage(store: Store<AppState>, view: EditorView, gate: Compartment, module: Promise<WebAssembly.Module>): void {
  byId('output-editor').addEventListener(
    'focusin',
    () => {
      import('../lang/edit').then(
        (lang) => lang.attachLanguage({ view, gate, store, module }),
        () => {
          const status = byId('output-lang');
          status.hidden = false;
          setText(status, 'Language features failed to load');
        },
      );
    },
    { once: true },
  );
}

function mountCopy(view: EditorView): void {
  const copy = byId<HTMLButtonElement>('copy-output');
  const copyLabel = byId('copy-label');
  let timer = 0;
  copy.addEventListener('click', () => {
    navigator.clipboard.writeText(view.state.doc.toString()).then(
      () => {
        setText(copyLabel, 'Copied');
        window.clearTimeout(timer);
        timer = window.setTimeout(() => setText(copyLabel, 'Copy'), COPIED_MS);
      },
      () => setText(copyLabel, 'Copy failed'),
    );
  });
}

export function mountOutputPane(store: Store<AppState>, module: Promise<WebAssembly.Module>): EditorView {
  const gate = new Compartment();
  const edits = outputEdits();
  const editing = codeEditing(module);
  const view = makeEditor(byId('output-editor'), 'Output code', [baseExtensions(true), editing.extension, gate.of([]), outputLook, edits.extension]);
  editing.attach(view, byId('output-editor'));
  mountCopy(view);
  mountLanguage(store, view, gate, module);
  byId('reset-output').addEventListener('click', () => {
    edits.replace(view, shownText(store.get()).text);
    view.focus();
  });
  store.subscribe((state, previous) => {
    const result = state.run.result;
    if (result !== previous.run.result || state.selectedModule !== previous.selectedModule) {
      const shown = shownText(state);
      edits.replace(view, shown.text);
      requestAnimationFrame(() => performance.mark('untwist:output-shown'));
      setText(byId('output-meta'), `${shown.name} · ${linesLabel(result === null ? 0 : view.state.doc.lines)}`);
    }
    byId<HTMLButtonElement>('copy-output').disabled = result === null;
    const running = state.run.status === 'running';
    byId('running').hidden = !running;
    if (running) {
      setText(byId('running-elapsed'), seconds(state.run.elapsedMs));
    }
  });
  return view;
}
