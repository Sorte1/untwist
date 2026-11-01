import { otherModeFlags } from '../compare';
import { byId, setText } from '../dom';
import type { AppState, InputState } from '../state';
import type { Store } from '../store';
import { BANNER_TEXT, undecodedInSafe } from '../strings-banner';

function detail(count: number): string {
  return `${count.toLocaleString('en-US')} decoder ${count === 1 ? 'call was' : 'calls were'} left in place.`;
}

export function mountStringsBanner(store: Store<AppState>, start: (input: InputState, flags: string) => void): void {
  const banner = byId('strings-banner');
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
      setText(byId('strings-banner-detail'), count === null ? '' : detail(count));
    }
    button.disabled = state.engine !== 'ready' || run.status === 'running';
  });
}
