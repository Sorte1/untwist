import type { Compartment } from '@codemirror/state';
import type { EditorView } from '@codemirror/view';
import { byId, setText } from '../dom';
import type { Pane } from '../lang/edit';
import type { AppState } from '../state';
import type { Store } from '../store';

export function mountLanguage(store: Store<AppState>, view: EditorView, gate: Compartment, module: Promise<WebAssembly.Module>, pane: Pane): void {
  byId(`${pane}-editor`).addEventListener(
    'focusin',
    () => {
      import('../lang/edit').then(
        (lang) => lang.attachLanguage({ view, gate, store, module, pane }),
        () => {
          const status = byId(`${pane}-lang`);
          status.hidden = false;
          setText(status, 'Language features failed to load');
        },
      );
    },
    { once: true },
  );
}
