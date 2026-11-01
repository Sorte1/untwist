import { byId, setText } from '../dom';
import { exitLabel, limitsLabel, limitsTitle, seconds, skippedLabel, type AppState } from '../state';
import type { Store } from '../store';

export function mountStatusBar(store: Store<AppState>): void {
  const exit = byId('exit-code');
  const limits = byId('limits');
  store.subscribe((state) => {
    const run = state.run;
    const finished = run.status === 'done' || run.status === 'limit';
    setText(exit, exitLabel(run));
    exit.dataset['tone'] = finished ? (run.result?.code === 0 ? 'text' : 'err') : 'muted';
    setText(byId('total'), run.status === 'idle' ? '—' : seconds(run.elapsedMs));
    setText(limits, limitsLabel(run));
    limits.title = limitsTitle(run);
    limits.dataset['tone'] = run.status === 'limit' ? 'err' : skippedLabel(run) === '' ? 'text' : 'warn';
  });
}
