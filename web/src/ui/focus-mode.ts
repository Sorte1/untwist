import { byId, setText } from '../dom';
import type { InputVisibility } from '../settings/settings';
import type { AppState } from '../state';
import type { Store } from '../store';

export function showInput(store: Store<AppState>, input: InputVisibility = 'shown'): void {
  store.update((state) => (state.settings.input === input ? state : { ...state, settings: { ...state.settings, input } }));
}

export function mountFocusMode(store: Store<AppState>): void {
  const toggle = byId<HTMLButtonElement>('focus-toggle');
  const rail = byId<HTMLButtonElement>('show-input');
  byId('hide-input').addEventListener('click', () => showInput(store, 'hidden'));
  rail.addEventListener('click', () => showInput(store));
  toggle.addEventListener('click', () => showInput(store, store.get().settings.input === 'hidden' ? 'shown' : 'hidden'));
  let first = true;
  store.subscribe((state, previous) => {
    if (!first && state.settings.input === previous.settings.input && state.input.name === previous.input.name) {
      return;
    }
    first = false;
    const hidden = state.settings.input === 'hidden';
    byId('input-pane').hidden = hidden;
    rail.hidden = !hidden;
    byId('output-pane').classList.toggle('pane-tall', hidden);
    toggle.setAttribute('aria-pressed', String(hidden));
    toggle.setAttribute('aria-label', hidden ? 'Show input pane' : 'Hide input for a full-width output');
    setText(byId('input-rail-label'), `Input · ${state.input.name}`);
  });
}
