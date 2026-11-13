import { foldEffect, unfoldEffect } from '@codemirror/language';
import type { EditorView } from '@codemirror/view';
import { byId, setText } from '../dom';
import { libraryFolds } from '../libraries/folds';
import { librariesOf } from '../libraries/libraries';
import type { AppState } from '../state';

export interface LibraryFolding {
  refresh(view: EditorView, state: AppState): void;
}

const HIDDEN_TITLE = 'Library modules are folded; copy and download keep the full output';
const SHOWN_TITLE = 'Fold every matched library module again';

function knownLibraries(state: AppState): ReadonlySet<string> {
  const result = state.run.result;
  return new Set(result === null ? [] : librariesOf(result.report).map((row) => row.library));
}

export function libraryFolding(): LibraryFolding {
  const button = byId<HTMLButtonElement>('library-code');
  const label = byId('library-code-label');
  let shown = false;
  let current: { readonly view: EditorView; readonly state: AppState } | null = null;
  const paint = (count: number) => {
    button.hidden = count === 0;
    button.setAttribute('aria-pressed', String(shown));
    button.title = shown ? SHOWN_TITLE : HIDDEN_TITLE;
    setText(label, shown ? 'Hide library code' : 'Show library code');
  };
  const apply = (view: EditorView, state: AppState) => {
    const folds = libraryFolds(view.state.doc, knownLibraries(state), state.selectedModule !== null);
    const effect = shown ? unfoldEffect : foldEffect;
    if (folds.length > 0) {
      view.dispatch({ effects: folds.map((fold) => effect.of({ from: fold.from, to: fold.to })) });
    }
    paint(folds.length);
  };
  button.addEventListener('click', () => {
    shown = !shown;
    if (current !== null) {
      apply(current.view, current.state);
    }
  });
  return {
    refresh(view, state) {
      current = { view, state };
      apply(view, state);
    },
  };
}
