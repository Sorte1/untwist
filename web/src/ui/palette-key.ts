import type { Example } from '../examples/examples';
import type { AppState } from '../state';
import type { Store } from '../store';

export interface PaletteActions {
  readonly run: () => void;
  readonly pickExample: (example: Example) => void;
}

interface Palette {
  toggle(): void;
}

function isPaletteKey(event: KeyboardEvent): boolean {
  return event.key.toLowerCase() === 'k' && (event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey;
}

export function mountPaletteKey(store: Store<AppState>, actions: PaletteActions): void {
  let palette: Promise<Palette> | null = null;
  window.addEventListener(
    'keydown',
    (event) => {
      if (!isPaletteKey(event)) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      palette ??= import('./palette').then((module) => module.createPalette(store, actions));
      palette.then(
        (opened) => opened.toggle(),
        () => {
          palette = null;
        },
      );
    },
    true,
  );
}
