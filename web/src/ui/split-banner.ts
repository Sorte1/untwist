import { byId, setText } from '../dom';
import { flagsFrom } from '../settings/flags';
import { SPLIT_TEXT, splitsWhenAssumed } from '../split-banner';
import type { AppState, InputState } from '../state';
import type { Store } from '../store';

export function mountSplitBanner(store: Store<AppState>, start: (input: InputState, flags: string) => void): void {
  const banner = byId('split-banner');
  const button = byId<HTMLButtonElement>('split-assume');
  setText(byId('split-banner-text'), SPLIT_TEXT);
  button.addEventListener('click', () => {
    const { run } = store.get();
    if (run.outputInput === null) {
      return;
    }
    store.update((state) => ({ ...state, settings: { ...state.settings, split: true, assume: true } }));
    start(run.outputInput, flagsFrom(store.get().settings));
  });
  let shown: AppState['run']['result'] | undefined;
  store.subscribe((state) => {
    const { run } = state;
    if (run.result !== shown) {
      shown = run.result;
      banner.hidden = run.result === null || !splitsWhenAssumed(run.result.report);
    }
    button.disabled = state.engine !== 'ready' || run.status === 'running';
  });
}
