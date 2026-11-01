import { Compartment } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { byId, linesLabel, setText } from '../dom';
import { seconds, type AppState } from '../state';
import type { Store } from '../store';
import { currentBundle } from './module-tree';
import { access, baseExtensions, makeEditor, outputLook, replaceDoc } from './editor';

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

function mountEditToggle(store: Store<AppState>, view: EditorView, gate: Compartment, module: Promise<WebAssembly.Module>): void {
  const edit = byId<HTMLButtonElement>('edit-output');
  let mode: Promise<{ toggle(): void }> | null = null;
  edit.addEventListener('click', () => {
    mode ??= import('../lang/edit').then((lang) => lang.createEditMode({ view, gate, store, module }));
    mode.then((loaded) => loaded.toggle(), () => edit.setAttribute('title', 'Editing failed to load'));
  });
  store.subscribe((state) => {
    edit.disabled = state.run.result === null;
  });
}

export function mountOutputPane(store: Store<AppState>, module: Promise<WebAssembly.Module>): EditorView {
  const gate = new Compartment();
  const view = makeEditor(byId('output-editor'), 'Output code', [baseExtensions(false, gate.of(access(false))), outputLook]);
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
  mountEditToggle(store, view, gate, module);
  store.subscribe((state, previous) => {
    const result = state.run.result;
    if (result !== previous.run.result || state.selectedModule !== previous.selectedModule) {
      const shown = shownText(state);
      replaceDoc(view, shown.text);
      requestAnimationFrame(() => performance.mark('untwist:output-shown'));
      setText(byId('output-meta'), `${shown.name} · ${linesLabel(result === null ? 0 : view.state.doc.lines)}`);
    }
    copy.disabled = result === null;
    const running = state.run.status === 'running';
    byId('running').hidden = !running;
    if (running) {
      setText(byId('running-elapsed'), seconds(state.run.elapsedMs));
    }
  });
  return view;
}
