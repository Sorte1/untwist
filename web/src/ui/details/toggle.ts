import { byId } from '../../dom';
import type { AppState } from '../../state';
import type { Store } from '../../store';

type Render = (container: HTMLElement, result: AppState['run']['result'], flags: string) => void;

export function mountDetailsToggle(store: Store<AppState>, flags: () => string): void {
  const tab = byId<HTMLButtonElement>('details-tab');
  const container = byId('details');
  let render: Render | null = null;
  let open = false;
  const paint = () => {
    if (open && render !== null) {
      render(container, store.get().run.result, flags());
    }
  };
  tab.addEventListener('click', () => {
    open = !open;
    tab.setAttribute('aria-expanded', String(open));
    container.hidden = !open;
    if (!open) {
      container.replaceChildren();
      return;
    }
    if (render === null) {
      import('./drawer').then((module) => {
        render = module.renderDrawer;
        paint();
      }, () => container.replaceChildren(document.createTextNode('The details view failed to load.')));
      return;
    }
    paint();
  });
  store.subscribe((state, previous) => {
    if (state.run.result !== previous.run.result) {
      paint();
    }
  });
}
