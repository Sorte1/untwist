import { Compartment } from '@codemirror/state';
import { EditorView } from '@codemirror/view';
import { byId, linesLabel, setText, sizeLabel } from '../dom';
import { inputOf, PREVIEW_CHARS, type AppState, type InputState } from '../state';
import type { Store } from '../store';
import { codeEditing } from './editing';
import { access as accessOf, baseExtensions, makeEditor, replaceDoc } from './editor';

const UNTITLED = 'untitled.js';

function metaOf(input: InputState): string {
  return `${sizeLabel(input.bytes)} · ${linesLabel(input.lines)}`;
}

function pastedText(view: EditorView, pasted: string): string {
  const { from, to } = view.state.selection.main;
  const doc = view.state.doc;
  return doc.sliceString(0, from) + pasted + doc.sliceString(to);
}

export function mountInputPane(store: Store<AppState>, module: Promise<WebAssembly.Module>): EditorView {
  const access = new Compartment();
  let shownRevision = 0;
  let applying = false;
  const setInput = (text: string, name: string) => {
    store.update((state) => ({ ...state, input: inputOf(text, name, state.input.revision + 1) }));
  };
  const typed = EditorView.updateListener.of((update) => {
    if (update.docChanged && !applying && !store.get().input.large) {
      const next = inputOf(update.state.doc.toString(), store.get().input.name, store.get().input.revision + 1);
      shownRevision = next.revision;
      store.update((state) => ({ ...state, input: next }));
    }
  });
  const editing = codeEditing(module);
  const view = makeEditor(byId('input-editor'), 'Input code', [baseExtensions(true, access.of(accessOf(true))), editing.extension, typed]);
  editing.attach(view, byId('input-editor'));
  view.dom.addEventListener(
    'paste',
    (event) => {
      const pasted = event.clipboardData?.getData('text/plain') ?? '';
      const text = pastedText(view, pasted);
      if (!inputOf(text, UNTITLED, 0).large) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      setInput(text, store.get().input.name);
    },
    true,
  );
  byId('clear-input').addEventListener('click', () => setInput('', UNTITLED));
  const chip = byId('input-chip');
  store.subscribe((state) => {
    const input = state.input;
    setText(byId('input-name'), input.name);
    setText(byId('input-meta'), metaOf(input));
    chip.hidden = !input.large;
    setText(chip, input.large ? `${sizeLabel(input.bytes)} · preview` : '');
    if (input.revision === shownRevision) {
      return;
    }
    shownRevision = input.revision;
    applying = true;
    view.dispatch({ effects: access.reconfigure(accessOf(!input.large)) });
    replaceDoc(view, input.large ? input.text.slice(0, PREVIEW_CHARS) : input.text);
    applying = false;
  });
  return view;
}
