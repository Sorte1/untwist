import { byId } from '../dom';
import { applyEditorTheme, loadPicks, savePicks } from '../editor-theme/apply';
import { withPick } from '../editor-theme/themes';
import type { AppState, Theme } from '../state';
import type { Store } from '../store';

const MODES: readonly Theme[] = ['dark', 'light'];

export function mountEditorTheme(store: Store<AppState>): void {
  let picks = loadPicks();
  applyEditorTheme(store.get().theme, picks);
  for (const mode of MODES) {
    const select = byId<HTMLSelectElement>(`editor-theme-${mode}`);
    select.value = picks[mode];
    select.addEventListener('change', () => {
      picks = withPick(picks, mode, select.value);
      select.value = picks[mode];
      savePicks(picks);
      if (store.get().theme !== mode) {
        store.update((state) => ({ ...state, theme: mode, settings: { ...state.settings, theme: mode } }));
      }
      applyEditorTheme(store.get().theme, picks);
    });
  }
  store.subscribe((state, previous) => {
    if (state.theme !== previous.theme) {
      applyEditorTheme(state.theme, picks);
    }
  });
}
