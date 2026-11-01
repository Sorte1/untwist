import { byId, setText } from '../dom';
import { flagsFrom } from '../settings/flags';
import { writeHash } from '../settings/hash';
import { DEFAULT_FUEL, RENAME_HINTS, type RenameMode, type Settings } from '../settings/settings';
import type { AppState } from '../state';
import type { Store } from '../store';

type BooleanKey = 'unpack' | 'jsx' | 'unsafe' | 'deob' | 'unmin' | 'human' | 'single';

const COPIED_MS = 1200;

function change(store: Store<AppState>, patch: Partial<Settings>): void {
  store.update((state) => ({ ...state, settings: { ...state.settings, ...patch } }));
}

function count(raw: string): number | null {
  const value = Number(raw);
  return raw.trim() !== '' && Number.isSafeInteger(value) && value >= 1 ? value : null;
}

function wireToggles(store: Store<AppState>): void {
  for (const button of document.querySelectorAll<HTMLButtonElement>('.switch[data-key]')) {
    const key = button.dataset['key'] as BooleanKey;
    button.addEventListener('click', () => change(store, { [key]: !store.get().settings[key] }));
  }
  for (const box of document.querySelectorAll<HTMLInputElement>('#advanced input[type=checkbox][data-key]')) {
    const key = box.dataset['key'] as BooleanKey;
    const inverted = box.dataset['inverted'] === '1';
    box.addEventListener('change', () => change(store, { [key]: inverted ? !box.checked : box.checked }));
  }
  for (const button of document.querySelectorAll<HTMLButtonElement>('#rename-group .seg-btn')) {
    button.addEventListener('click', () => change(store, { rename: button.dataset['mode'] as RenameMode }));
  }
}

function wireNumbers(store: Store<AppState>): void {
  const fuel = byId<HTMLInputElement>('fuel');
  const infinite = byId<HTMLInputElement>('fuel-inf');
  infinite.addEventListener('change', () => change(store, { fuel: infinite.checked ? null : count(fuel.value) ?? DEFAULT_FUEL }));
  fuel.addEventListener('input', () => {
    const value = count(fuel.value);
    if (value !== null) {
      change(store, { fuel: value });
    }
  });
  const iterations = byId<HTMLInputElement>('max-iterations');
  iterations.addEventListener('input', () => change(store, { iter: count(iterations.value) }));
}

function wireLink(store: Store<AppState>): void {
  const label = byId('copy-link-label');
  let timer = 0;
  byId('copy-link').addEventListener('click', () => {
    navigator.clipboard.writeText(window.location.href).then(
      () => {
        setText(label, 'Link copied');
        window.clearTimeout(timer);
        timer = window.setTimeout(() => setText(label, 'Copy settings link'), COPIED_MS);
      },
      () => setText(label, 'Copy failed'),
    );
  });
  store.subscribe((state) => setText(byId('query'), writeHash(state.settings) || '#'));
}

function paint(settings: Settings): void {
  for (const button of document.querySelectorAll<HTMLButtonElement>('.switch[data-key]')) {
    button.setAttribute('aria-checked', String(settings[button.dataset['key'] as BooleanKey]));
  }
  for (const box of document.querySelectorAll<HTMLInputElement>('#advanced input[type=checkbox][data-key]')) {
    const value = settings[box.dataset['key'] as BooleanKey];
    box.checked = box.dataset['inverted'] === '1' ? !value : value;
  }
  for (const button of document.querySelectorAll<HTMLButtonElement>('#rename-group .seg-btn')) {
    button.setAttribute('aria-pressed', String(button.dataset['mode'] === settings.rename));
  }
  setText(byId('rename-hint'), RENAME_HINTS[settings.rename]);
  const fuel = byId<HTMLInputElement>('fuel');
  byId<HTMLInputElement>('fuel-inf').checked = settings.fuel === null;
  fuel.disabled = settings.fuel === null;
  if (settings.fuel !== null && count(fuel.value) !== settings.fuel) {
    fuel.value = String(settings.fuel);
  }
  const iterations = byId<HTMLInputElement>('max-iterations');
  if (count(iterations.value) !== settings.iter) {
    iterations.value = settings.iter === null ? '' : String(settings.iter);
  }
}

export function mountOptions(store: Store<AppState>): void {
  wireToggles(store);
  wireNumbers(store);
  wireLink(store);
  let first = true;
  store.subscribe((state, previous) => {
    if (first || state.settings !== previous.settings) {
      first = false;
      paint(state.settings);
    }
    const ran = state.run.ranFlags;
    byId('settings-changed').hidden = ran === null || ran === flagsFrom(state.settings);
  });
}
