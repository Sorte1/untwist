import { byId } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';

export function introShown(state: AppState): boolean {
  return state.run.result === null && state.run.status === 'idle';
}

export function mountIntro(store: Store<AppState>): void {
  const intro = byId('intro');
  store.subscribe((state) => {
    const hidden = !introShown(state);
    if (intro.hidden !== hidden) {
      intro.hidden = hidden;
    }
  });
}
