import type { EditorView } from '@codemirror/view';
import type { AppState } from '../state';
import type { Store } from '../store';

export function mountLinks(store: Store<AppState>, input: EditorView, output: EditorView): void {
  let loading = false;
  store.subscribe((state) => {
    if (loading || (state.run.result?.links ?? '') === '') {
      return;
    }
    loading = true;
    import('./links/link').then(
      (module) => module.attachLinks(store, input, output),
      () => {
        loading = false;
      },
    );
  });
}
