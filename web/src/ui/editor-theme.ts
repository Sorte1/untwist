import { applyEditorTheme, loadPicks, savePicks } from '../editor-theme/apply';
import { withPick } from '../editor-theme/themes';
import type { AppState, Theme } from '../state';
import type { Store } from '../store';

const MODES: readonly Theme[] = ['dark', 'light'];

function pickButtons(mode: Theme): NodeListOf<HTMLButtonElement> {
  return document.querySelectorAll<HTMLButtonElement>(`#editor-theme-${mode} .seg-btn`);
}

function pressPick(mode: Theme, pick: string): void {
  for (const button of pickButtons(mode)) {
    button.setAttribute('aria-pressed', String(button.dataset['pick'] === pick));
  }
}

export function mountEditorTheme(store: Store<AppState>): void {
  let picks = loadPicks();
  applyEditorTheme(store.get().theme, picks);
  for (const mode of MODES) {
    pressPick(mode, picks[mode]);
    for (const button of pickButtons(mode)) {
      button.addEventListener('click', () => {
        picks = withPick(picks, mode, button.dataset['pick'] ?? '');
        pressPick(mode, picks[mode]);
        savePicks(picks);
        if (store.get().theme !== mode) {
          store.update((state) => ({ ...state, theme: mode, settings: { ...state.settings, theme: mode } }));
        }
        applyEditorTheme(store.get().theme, picks);
      });
    }
  }
  store.subscribe((state, previous) => {
    if (state.theme !== previous.theme) {
      applyEditorTheme(state.theme, picks);
    }
  });
}
