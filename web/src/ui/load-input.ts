import { byId, setText } from '../dom';
import { inputOf, type AppState } from '../state';
import type { Store } from '../store';

export const DEFAULT_HINT = 'Paste, type or drop a file anywhere';

export interface Loader {
  load(text: string, name: string, note?: string): number;
  note(text: string): void;
}

export function mountLoader(store: Store<AppState>, run: () => void): Loader {
  const hint = byId('input-hint');
  let noted = -1;
  store.subscribe((state, previous) => {
    if (state.input.revision !== previous.input.revision && state.input.revision !== noted) {
      setText(hint, DEFAULT_HINT);
    }
  });
  const note = (text: string) => {
    noted = store.get().input.revision;
    setText(hint, text);
  };
  return {
    load(text, name, message) {
      store.update((state) => ({ ...state, input: inputOf(text, name, state.input.revision + 1) }));
      const revision = store.get().input.revision;
      noted = revision;
      setText(hint, message ?? DEFAULT_HINT);
      run();
      return revision;
    },
    note,
  };
}
