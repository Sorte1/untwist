import { byId, sizeLabel } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import type { Loader } from './load-input';

const ICON =
  '<svg width="28" height="28" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 2H4.5A1.5 1.5 0 0 0 3 3.5v9A1.5 1.5 0 0 0 4.5 14h7a1.5 1.5 0 0 0 1.5-1.5V6z"></path><path d="M9 2v4h4"></path><path d="M8 12V8.5M6.5 10 8 8.5 9.5 10"></path></svg>';

export function decodeText(bytes: ArrayBuffer): string | null {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return null;
  }
}

export function extraNote(count: number, name: string): string | undefined {
  return count > 1 ? `${count} files given; only the first, ${name}, was loaded` : undefined;
}

export async function loadFiles(files: readonly File[], loader: Loader): Promise<void> {
  const [file] = files;
  if (file === undefined) {
    return;
  }
  const text = decodeText(await file.arrayBuffer());
  if (text === null) {
    loader.note(`${file.name} is not UTF-8 text, so it was not loaded`);
    return;
  }
  loader.load(text, file.name, extraNote(files.length, file.name));
}

function overlayElement(): HTMLElement {
  const overlay = document.createElement('div');
  overlay.className = 'drop-overlay';
  overlay.id = 'drop-overlay';
  overlay.innerHTML = ICON;
  const title = document.createElement('span');
  title.className = 'drop-title';
  const detail = document.createElement('span');
  detail.className = 'muted';
  overlay.append(title, detail);
  return overlay;
}

function describe(overlay: HTMLElement, transfer: DataTransfer): void {
  const files = [...transfer.items].filter((item) => item.kind === 'file');
  const [title, detail] = overlay.querySelectorAll('span');
  const file = transfer.files[0];
  const count = Math.max(files.length, transfer.files.length);
  title?.replaceChildren(count > 1 ? `Drop to load the first of ${count} files` : `Drop to load ${file?.name ?? 'the file'}`);
  detail?.replaceChildren(file === undefined ? 'Read locally, never uploaded' : `${sizeLabel(file.size)} · read locally, never uploaded`);
}

export function mayCarryFiles(types: readonly string[]): boolean {
  return types.length === 0 || types.includes('Files');
}

function carriesFiles(event: DragEvent): boolean {
  return event.dataTransfer !== null && mayCarryFiles([...event.dataTransfer.types]);
}

function wireOpenFile(loader: Loader): void {
  const input = byId<HTMLInputElement>('open-file-input');
  byId('open-file').addEventListener('click', () => input.click());
  input.addEventListener('change', () => {
    const files = [...(input.files ?? [])];
    input.value = '';
    void loadFiles(files, loader);
  });
}

export function mountDropOverlay(store: Store<AppState>, loader: Loader): void {
  const overlay = overlayElement();
  let depth = 0;
  const hide = () => {
    depth = 0;
    overlay.remove();
  };
  const host = () => (store.get().settings.input === 'hidden' ? byId('output-pane') : byId('input-pane'));
  window.addEventListener('dragenter', (event) => {
    if (!carriesFiles(event)) {
      return;
    }
    depth += 1;
    if (event.dataTransfer !== null) {
      describe(overlay, event.dataTransfer);
    }
    host().append(overlay);
  }, true);
  window.addEventListener('dragleave', (event) => {
    if (!carriesFiles(event)) {
      return;
    }
    depth -= 1;
    if (depth <= 0) {
      hide();
    }
  }, true);
  window.addEventListener('dragover', (event) => {
    if (carriesFiles(event)) {
      event.preventDefault();
      event.stopPropagation();
    }
  }, true);
  window.addEventListener('drop', (event) => {
    if (!carriesFiles(event)) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    hide();
    void loadFiles([...(event.dataTransfer?.files ?? [])], loader);
  }, true);
  wireOpenFile(loader);
}
