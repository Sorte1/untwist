import { byId, setText } from '../dom';
import { flagsFrom } from '../settings/flags';
import { writeHash } from '../settings/hash';
import { DEFAULT_FUEL, MODERN_CHARS, MODERN_PRESETS, RENAME_HINTS, TYPES_HINTS, WASM_CHECK_HINTS, type RenameMode, type Settings, type TypesChoice, type WasmCheck } from '../settings/settings';
import type { AppState } from '../state';
import type { Store } from '../store';

type BooleanKey = 'unpack' | 'jsx' | 'unsafe' | 'deob' | 'unmin' | 'human' | 'single' | 'libs' | 'split' | 'assume' | 'smap' | 'imap';

const COPIED_MS = 1200;
const MODERN_CUSTOM = 'custom';

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
  for (const button of document.querySelectorAll<HTMLButtonElement>('#wasm-check-group .seg-btn')) {
    button.addEventListener('click', () => change(store, { wasm: button.dataset['check'] as WasmCheck }));
  }
  for (const button of document.querySelectorAll<HTMLButtonElement>('#types-group .seg-btn')) {
    button.addEventListener('click', () => change(store, { types: button.dataset['types'] as TypesChoice }));
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
  const memory = byId<HTMLInputElement>('max-memory');
  memory.addEventListener('input', () => change(store, { mem: count(memory.value) }));
  const time = byId<HTMLInputElement>('time-limit');
  time.addEventListener('input', () => change(store, { time: count(time.value) }));
  for (const button of modernButtons()) {
    button.addEventListener('click', () => {
      const preset = button.dataset['preset'] ?? '';
      const custom = preset === MODERN_CUSTOM;
      const modern = byId('modern');
      modern.hidden = !custom;
      pressModern(preset);
      if (custom) {
        modern.focus();
      } else {
        change(store, { modern: preset });
      }
    });
  }
  const modern = byId<HTMLInputElement>('modern');
  modern.addEventListener('input', () => {
    modern.setCustomValidity(MODERN_CHARS.test(modern.value) ? '' : 'Use letters, digits, spaces, commas, dots, @, _ or -');
    if (MODERN_CHARS.test(modern.value)) {
      change(store, { modern: modern.value });
    }
  });
}

function modernButtons(): NodeListOf<HTMLButtonElement> {
  return document.querySelectorAll<HTMLButtonElement>('#modern-group .seg-btn');
}

function pressModern(preset: string): void {
  for (const button of modernButtons()) {
    button.setAttribute('aria-pressed', String(button.dataset['preset'] === preset));
  }
}

function paintModern(value: string): void {
  const modern = byId<HTMLInputElement>('modern');
  const shown = !modern.hidden || !MODERN_PRESETS.includes(value) ? MODERN_CUSTOM : value;
  modern.hidden = shown !== MODERN_CUSTOM;
  pressModern(shown);
  if (modern.value !== value) {
    modern.value = value;
  }
}

function paintNumber(id: string, value: number | null): void {
  const input = byId<HTMLInputElement>(id);
  if (count(input.value) !== value) {
    input.value = value === null ? '' : String(value);
  }
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
  for (const button of document.querySelectorAll<HTMLButtonElement>('#wasm-check-group .seg-btn')) {
    button.setAttribute('aria-pressed', String(button.dataset['check'] === settings.wasm));
  }
  setText(byId('wasm-check-hint'), WASM_CHECK_HINTS[settings.wasm]);
  for (const button of document.querySelectorAll<HTMLButtonElement>('#types-group .seg-btn')) {
    button.setAttribute('aria-pressed', String(button.dataset['types'] === settings.types));
  }
  setText(byId('types-hint'), TYPES_HINTS[settings.types]);
  const fuel = byId<HTMLInputElement>('fuel');
  byId<HTMLInputElement>('fuel-inf').checked = settings.fuel === null;
  fuel.disabled = settings.fuel === null;
  if (settings.fuel !== null && count(fuel.value) !== settings.fuel) {
    fuel.value = String(settings.fuel);
  }
  paintNumber('max-iterations', settings.iter);
  paintNumber('max-memory', settings.mem);
  paintNumber('time-limit', settings.time);
  paintModern(settings.modern);
  byId('assume-opt').toggleAttribute('data-inactive', !settings.split);
  byId<HTMLInputElement>('imap').disabled = !settings.smap;
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
