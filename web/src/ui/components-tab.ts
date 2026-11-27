import { byId, setText } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import { restoredOf, type Restored } from './components/components';
import type { ComponentsView } from './components/panel';

type Render = (holder: HTMLElement, view: ComponentsView) => void;

export function restoredComponents(state: AppState): Restored {
  return restoredOf(state.run.result?.svelte);
}

export function mountComponentsTab(store: Store<AppState>, run: () => void): void {
  const tab = byId<HTMLButtonElement>('components-tab');
  const panel = byId('components-panel');
  let render: Render | null = null;
  let open = false;
  let painted: { result: unknown; enabled: boolean } | null = null;
  const turnOn = () => {
    store.update((state) => ({ ...state, settings: { ...state.settings, svelte: true } }));
    run();
  };
  const paint = () => {
    const state = store.get();
    if (!open || render === null) {
      return;
    }
    const next = { result: state.run.result, enabled: state.settings.svelte };
    if (painted?.result === next.result && painted.enabled === next.enabled) {
      return;
    }
    painted = next;
    const restored = restoredComponents(state);
    render(panel, { enabled: state.settings.svelte, components: restored.files, kept: restored.kept, turnOn });
  };
  const show = (next: boolean) => {
    open = next;
    painted = null;
    tab.setAttribute('aria-expanded', String(open));
    panel.hidden = !open;
    if (!open) {
      panel.replaceChildren();
    } else if (render === null) {
      import('./components/panel').then(
        (module) => {
          render = module.renderComponents;
          paint();
        },
        () => panel.replaceChildren(document.createTextNode('The components panel failed to load.')),
      );
    } else {
      paint();
    }
  };
  tab.addEventListener('click', () => show(!open));
  store.subscribe((state, previous) => {
    if (state.run.result !== previous.run.result) {
      const restored = restoredComponents(state);
      const count = restored.files.length;
      const kept = restored.kept.length;
      tab.hidden = count + kept === 0;
      setText(byId('components-count'), count === 0 ? '' : String(count));
      tab.title = `${count} restored Svelte components${kept > 0 ? `, ${kept} kept as JavaScript` : ''}`;
      if (tab.hidden && open) {
        show(false);
      }
    }
    if (state.run.result !== previous.run.result || state.settings.svelte !== previous.settings.svelte) {
      paint();
    }
  });
}
