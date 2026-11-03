import { byId, setText } from '../dom';
import { limitsTitle, seconds, skippedLabel, statusLabel, type AppState } from '../state';
import type { Store } from '../store';
import { otherTheme } from '../theme';

export interface ToolbarActions {
  readonly run: () => void;
  readonly cancel: () => void;
}

function themeButtons(): HTMLButtonElement[] {
  return [byId<HTMLButtonElement>('theme'), ...document.querySelectorAll<HTMLButtonElement>('.theme-alt')];
}

function showOne(run: HTMLButtonElement, cancel: HTMLButtonElement, running: boolean): void {
  const focused = document.activeElement;
  cancel.hidden = cancel.disabled;
  run.hidden = running;
  if (focused === run && run.hidden && !cancel.hidden) {
    cancel.focus();
  }
  if (focused === cancel && cancel.hidden && !run.hidden) {
    run.focus();
  }
}

export function mountToolbar(store: Store<AppState>, actions: ToolbarActions): void {
  const run = byId<HTMLButtonElement>('run');
  const cancel = byId<HTMLButtonElement>('cancel');
  const themes = themeButtons();
  run.addEventListener('click', actions.run);
  cancel.addEventListener('click', actions.cancel);
  for (const button of themes) {
    button.addEventListener('click', () =>
      store.update((state) => {
        const next = otherTheme(state.theme);
        return { ...state, theme: next, settings: { ...state.settings, theme: next } };
      }),
    );
  }
  store.subscribe((state) => {
    const running = state.run.status === 'running';
    run.disabled = state.engine !== 'ready' || running;
    cancel.disabled = !running && state.compare.status !== 'running';
    showOne(run, cancel, running);
    setText(byId('status-label'), statusLabel(state));
    const skipped = byId('skipped-chip');
    const label = skippedLabel(state.run);
    skipped.hidden = label === '';
    setText(skipped, label);
    skipped.title = label === '' ? '' : limitsTitle(state.run);
    skipped.setAttribute('aria-label', label === '' ? '' : `${label}. ${limitsTitle(state.run)}`);
    byId('status-dot').dataset['status'] = state.engine === 'failed' ? 'failed' : state.run.status;
    setText(byId('elapsed'), seconds(state.run.elapsedMs));
    const themeLabel = state.theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme';
    for (const button of themes) {
      button.setAttribute('aria-label', themeLabel);
    }
  });
}
