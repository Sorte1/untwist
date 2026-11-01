import { byId, setText } from '../dom';
import { JSX_HINTS, jsxInactive, modeFromFlags, modeLabel, modeOf, steppedMode, UNSAFE_WARNING, type Mode } from '../settings/mode';
import type { AppState } from '../state';
import type { Store } from '../store';

function radios(): HTMLButtonElement[] {
  return [...document.querySelectorAll<HTMLButtonElement>('#mode-group [role="radio"]')];
}

function choose(store: Store<AppState>, mode: Mode): void {
  const unsafe = mode === 'unsafe';
  store.update((state) => (state.settings.unsafe === unsafe ? state : { ...state, settings: { ...state.settings, unsafe } }));
}

function wire(store: Store<AppState>): void {
  for (const radio of radios()) {
    radio.addEventListener('click', () => choose(store, radio.dataset['mode'] as Mode));
    radio.addEventListener('keydown', (event) => {
      const next = steppedMode(modeOf(store.get().settings), event.key);
      if (next === null) {
        return;
      }
      event.preventDefault();
      choose(store, next);
      radios().find((item) => item.dataset['mode'] === next)?.focus();
    });
  }
}

function paint(state: AppState): void {
  const mode = modeOf(state.settings);
  for (const radio of radios()) {
    const on = radio.dataset['mode'] === mode;
    radio.setAttribute('aria-checked', String(on));
    radio.tabIndex = on ? 0 : -1;
  }
  byId('mode-warning').hidden = mode !== 'unsafe';
  const inactive = jsxInactive(state.settings);
  byId('jsx-opt').toggleAttribute('data-inactive', inactive);
  setText(byId('jsx-hint'), state.settings.unsafe ? JSX_HINTS.active : JSX_HINTS.inactive);
}

export function mountModeSwitch(store: Store<AppState>): void {
  setText(byId('mode-warning-text'), UNSAFE_WARNING);
  wire(store);
  const shown = byId('output-mode');
  let first = true;
  store.subscribe((state, previous) => {
    if (first || state.settings !== previous.settings) {
      paint(state);
    }
    if (first || state.run.outputFlags !== previous.run.outputFlags) {
      const mode = modeFromFlags(state.run.outputFlags);
      setText(shown, modeLabel(mode));
      shown.dataset['mode'] = mode ?? '';
    }
    first = false;
  });
}
