import { byId, setText } from '../dom';
import { EXAMPLES, loadExample, type Example } from '../examples/examples';
import type { AppState } from '../state';
import type { Store } from '../store';
import type { Loader } from './load-input';

function baseName(path: string): string {
  return path.slice(path.lastIndexOf('/') + 1);
}

function option(example: Example, pick: (example: Example) => void): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'example-option';
  button.setAttribute('role', 'option');
  button.dataset['source'] = example.source;
  const name = document.createElement('span');
  name.className = 'strong';
  name.textContent = example.name;
  const source = document.createElement('span');
  source.className = 'example-source';
  source.textContent = example.source;
  button.append(name, source);
  button.addEventListener('click', () => pick(example));
  return button;
}

function menuElement(pick: (example: Example) => void): HTMLElement {
  const menu = document.createElement('div');
  menu.className = 'examples-menu';
  menu.id = 'examples-menu';
  menu.setAttribute('role', 'listbox');
  menu.setAttribute('aria-label', 'Example inputs');
  const head = document.createElement('div');
  head.className = 'label examples-head';
  head.textContent = 'From test fixtures';
  menu.append(head, ...EXAMPLES.map((example) => option(example, pick)));
  return menu;
}

function dismissOnOutside(holder: HTMLElement, open: () => boolean, close: () => void): void {
  document.addEventListener('click', (event) => {
    if (open() && event.target instanceof Node && !holder.contains(event.target)) {
      close();
    }
  });
  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && open()) {
      close();
    }
  });
}

export function mountExamplesMenu(store: Store<AppState>, loader: Loader): void {
  const button = byId<HTMLButtonElement>('examples');
  const holder = button.parentElement ?? document.body;
  let menu: HTMLElement | null = null;
  let picked: { readonly example: Example; readonly revision: number } | null = null;
  const close = () => {
    menu?.remove();
    menu = null;
    button.setAttribute('aria-expanded', 'false');
  };
  const paint = () => {
    const current = picked !== null && picked.revision === store.get().input.revision ? picked.example : null;
    setText(byId('example-name'), current?.name ?? 'none');
    for (const item of menu?.querySelectorAll<HTMLElement>('[role=option]') ?? []) {
      item.setAttribute('aria-selected', String(item.dataset['source'] === current?.source));
    }
  };
  const pick = (example: Example) => {
    close();
    loadExample(example.source).then(
      (text) => {
        picked = { example, revision: loader.load(text, baseName(example.source)) };
        paint();
      },
      () => loader.note(`The example ${example.name} failed to load.`),
    );
  };
  button.addEventListener('click', () => {
    if (menu !== null) {
      close();
      return;
    }
    menu = menuElement(pick);
    holder.append(menu);
    button.setAttribute('aria-expanded', 'true');
    paint();
  });
  dismissOnOutside(holder, () => menu !== null, () => {
    close();
    button.focus();
  });
  store.subscribe((state, previous) => {
    if (state.input.revision !== previous.input.revision) {
      paint();
    }
  });
}
