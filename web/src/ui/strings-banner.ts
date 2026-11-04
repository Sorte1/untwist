import type { EditorView } from '@codemirror/view';
import { otherModeFlags } from '../compare';
import { byId, setText } from '../dom';
import type { AppState, InputState } from '../state';
import type { Store } from '../store';
import { BANNER_TEXT, undecodedInSafe } from '../strings-banner';
import { revealInInput } from './diagnostics';
import type { RefusalView } from './refusals/panel';

type Render = (holder: HTMLElement, view: RefusalView) => void;

function mountWhy(store: Store<AppState>, input: EditorView): (shown: boolean) => void {
  const toggle = byId<HTMLButtonElement>('refusal-why');
  const panel = byId('refusal-why-panel');
  let render: Render | null = null;
  let open = false;
  const paint = () => {
    const { run, input: current } = store.get();
    const ran = run.outputInput;
    if (!open || render === null || run.result === null || ran === null) {
      return;
    }
    const linked = ran.revision === current.revision && !ran.large;
    render(panel, { report: run.result.report, text: ran.text, linked, reveal: (span) => revealInInput(store, input, ran.text, span) });
  };
  const show = (next: boolean) => {
    open = next;
    toggle.setAttribute('aria-expanded', String(open));
    panel.hidden = !open;
    if (!open) {
      panel.replaceChildren();
    } else if (render === null) {
      import('./refusals/panel').then(
        (module) => {
          render = module.renderRefusals;
          paint();
        },
        () => panel.replaceChildren(document.createTextNode('The explanation failed to load.')),
      );
    } else {
      paint();
    }
  };
  toggle.addEventListener('click', () => show(!open));
  store.subscribe((state, previous) => {
    if (open && (state.run.result !== previous.run.result || state.input.revision !== previous.input.revision)) {
      paint();
    }
  });
  return (shown) => {
    if (!shown && open) {
      show(false);
    }
  };
}

function detail(count: number): string {
  return `${count.toLocaleString('en-US')} decoder ${count === 1 ? 'call was' : 'calls were'} left in place.`;
}

export function mountStringsBanner(store: Store<AppState>, start: (input: InputState, flags: string) => void, input: EditorView): void {
  const banner = byId('strings-banner');
  const why = mountWhy(store, input);
  const button = byId<HTMLButtonElement>('run-unsafe');
  setText(byId('strings-banner-text'), BANNER_TEXT);
  button.addEventListener('click', () => {
    const { run } = store.get();
    if (run.outputInput === null || run.outputFlags === null) {
      return;
    }
    const flags = otherModeFlags(run.outputFlags);
    store.update((state) => ({ ...state, settings: { ...state.settings, unsafe: true } }));
    start(run.outputInput, flags);
  });
  let shown: AppState['run']['result'] | undefined;
  store.subscribe((state) => {
    const { run } = state;
    if (run.result !== shown) {
      shown = run.result;
      const count = run.result === null ? null : undecodedInSafe(run.result.report, run.outputFlags);
      banner.hidden = count === null;
      why(count !== null);
      setText(byId('strings-banner-detail'), count === null ? '' : detail(count));
    }
    button.disabled = state.engine !== 'ready' || run.status === 'running';
  });
}
