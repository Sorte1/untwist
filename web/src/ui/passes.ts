import { byId, setText } from '../dom';
import { stages, withPass } from '../settings/passes';
import type { AppState } from '../state';
import type { Store } from '../store';

function stageGroup(stage: string, rows: ReturnType<typeof stages>[number][1]): HTMLFieldSetElement {
  const group = document.createElement('fieldset');
  group.className = 'pass-stage';
  const legend = document.createElement('legend');
  legend.className = 'muted';
  legend.textContent = stage;
  group.append(legend);
  for (const pass of rows) {
    const label = document.createElement('label');
    label.className = 'check pass-check';
    const box = document.createElement('input');
    box.type = 'checkbox';
    box.checked = true;
    box.dataset['pass'] = pass.name;
    label.append(box, pass.name);
    if (pass.unsafe) {
      const tag = document.createElement('span');
      tag.className = 'muted';
      tag.textContent = ' (unsafe)';
      label.append(tag);
    }
    group.append(label);
  }
  return group;
}

function offCount(off: string): number {
  return off === '' ? 0 : off.split(',').length;
}

function paint(off: string): void {
  const names = new Set(off.split(','));
  for (const box of document.querySelectorAll<HTMLInputElement>('#passes-list input[data-pass]')) {
    box.checked = !names.has(box.dataset['pass'] ?? '');
  }
  const count = offCount(off);
  setText(byId('passes-off'), count === 0 ? '' : `${count} off`);
  byId<HTMLButtonElement>('passes-reset').disabled = count === 0;
}

export function mountPasses(store: Store<AppState>): void {
  const list = byId('passes-list');
  list.replaceChildren(...stages().map(([stage, rows]) => stageGroup(stage, rows)));
  const change = (off: string) => store.update((state) => ({ ...state, settings: { ...state.settings, off } }));
  list.addEventListener('change', (event) => {
    const box = event.target as HTMLInputElement;
    const name = box.dataset['pass'];
    if (name !== undefined) {
      change(withPass(store.get().settings.off, name, box.checked));
    }
  });
  byId('passes-reset').addEventListener('click', () => change(''));
  let shown: string | null = null;
  store.subscribe((state) => {
    if (state.settings.off !== shown) {
      shown = state.settings.off;
      paint(shown);
    }
  });
}
