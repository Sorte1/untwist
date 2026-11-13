import { byId } from '../../dom';
import type { AppState } from '../../state';
import { timelineRows, type TimelineRow } from '../../progress';
import { runProgressOf } from '../run-progress';
import type { Store } from '../../store';

type Render = (container: HTMLElement, result: AppState['run']['result'], flags: string, timeline: readonly TimelineRow[]) => void;

export function mountDetailsToggle(store: Store<AppState>, flags: () => string): void {
  const tab = byId<HTMLButtonElement>('details-tab');
  const container = byId('details');
  let render: Render | null = null;
  let open = false;
  const paint = () => {
    if (open && render !== null) {
      const run = store.get().run;
      render(container, run.result, flags(), run.status === 'running' || run.stages.length === 0 ? [] : timelineRows(runProgressOf(run)));
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
    if (state.run.result !== previous.run.result || (state.run.status !== 'running' && state.run.stages !== previous.run.stages)) {
      paint();
    }
  });
}
