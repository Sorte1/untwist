import type { EditorView } from '@codemirror/view';
import { byId, setText } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import { revealInInput } from './diagnostics';
import type { StringsView } from './strings/table';

type Render = (holder: HTMLElement, view: StringsView) => void;

function linkedOf(state: AppState): boolean {
  const ran = state.run.outputInput;
  return ran !== null && ran.revision === state.input.revision && !ran.large;
}

export function mountStringsTab(store: Store<AppState>, input: EditorView): void {
  const tab = byId<HTMLButtonElement>('strings-tab');
  const panel = byId('strings-table');
  let render: Render | null = null;
  let open = false;
  let painted: { result: unknown; linked: boolean } | null = null;
  const paint = () => {
    const state = store.get();
    const ran = state.run.outputInput;
    const result = state.run.result;
    if (!open || render === null || ran === null || result === null) {
      return;
    }
    const linked = linkedOf(state);
    if (painted?.result === result && painted.linked === linked) {
      return;
    }
    painted = { result, linked };
    render(panel, { json: result.decodedStrings ?? '', text: ran.text, linked, reveal: (span) => revealInInput(store, input, ran.text, span) });
  };
  const show = (next: boolean) => {
    open = next;
    painted = null;
    tab.setAttribute('aria-expanded', String(open));
    panel.hidden = !open;
    if (!open) {
      panel.replaceChildren();
    } else if (render === null) {
      import('./strings/table').then(
        (module) => {
          render = module.renderStrings;
          paint();
        },
        () => panel.replaceChildren(document.createTextNode('The strings table failed to load.')),
      );
    } else {
      paint();
    }
  };
  tab.addEventListener('click', () => show(!open));
  store.subscribe((state, previous) => {
    if (state.run.result !== previous.run.result) {
      const count = state.run.result?.decodedCount ?? 0;
      tab.hidden = count === 0;
      setText(byId('strings-count'), count.toLocaleString('en-US'));
      if (count === 0 && open) {
        show(false);
      }
    }
    paint();
  });
}
