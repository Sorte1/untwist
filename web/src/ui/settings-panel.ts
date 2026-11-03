import { byId, setText } from '../dom';
import { changedCount } from '../settings/settings';
import type { AppState } from '../state';
import type { Store } from '../store';

export function mountSettingsPanel(store: Store<AppState>): void {
  const panel = byId('settings');
  const toggle = byId<HTMLButtonElement>('settings-toggle');
  const count = byId('settings-count');
  panel.addEventListener('toggle', (event) => {
    const open = (event as ToggleEvent).newState === 'open';
    toggle.setAttribute('aria-expanded', String(open));
  });
  let shown = -1;
  store.subscribe((state) => {
    const changed = changedCount(state.settings);
    if (changed === shown) {
      return;
    }
    shown = changed;
    count.hidden = changed === 0;
    setText(count, String(changed));
    toggle.title = changed === 0 ? 'Settings' : `Settings: ${changed} changed from the defaults`;
  });
}
