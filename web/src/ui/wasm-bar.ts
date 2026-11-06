import { byId, sizeLabel } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import {
  alwaysFetchWasm,
  checkedWasm,
  isAbsoluteUrl,
  MAX_WASM_BYTES,
  parseWasmReport,
  resolvedUrl,
  setAlwaysFetchWasm,
  withSupplied,
  UNCHECKED_WARNING,
  type SuppliedWasm,
  type WasmReferenceInfo,
} from '../wasm/wasm';
import { downloadText } from './details/export';
import type { WasmAdder } from './drop-overlay';

function button(label: string, title: string, onClick: () => void): HTMLButtonElement {
  const element = document.createElement('button');
  element.type = 'button';
  element.className = 'btn btn-small';
  element.textContent = label;
  element.title = title;
  element.addEventListener('click', onClick);
  return element;
}

function row(className: string, ...children: Array<Node | string>): HTMLElement {
  const element = document.createElement('div');
  element.className = className;
  element.append(...children);
  return element;
}

function text(className: string, content: string): HTMLElement {
  const element = document.createElement('span');
  element.className = className;
  element.textContent = content;
  return element;
}

function fileName(path: string): string {
  return path.split('/').pop() ?? path;
}

async function fetchWasm(url: string): Promise<Uint8Array<ArrayBuffer>> {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${url} answered ${response.status}`);
  }
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (bytes.length > MAX_WASM_BYTES) {
    throw new Error(`${url} is over the wasm size limit`);
  }
  return bytes;
}

function consentedUrl(reference: WasmReferenceInfo, asking: boolean): string | null {
  let base: string | null = null;
  if (!isAbsoluteUrl(reference.url)) {
    base = window.prompt(`${reference.url} is relative. Enter the page or script URL it is loaded from to fetch it:`, '');
    if (base === null || base.trim() === '') {
      return null;
    }
  }
  const url = resolvedUrl(reference.url, base);
  if (url === null) {
    return null;
  }
  if (asking && !window.confirm(`Fetch ${url}? The request goes from your browser to that server.`)) {
    return null;
  }
  return url;
}

export function mountWasmBar(store: Store<AppState>, rerun: () => void): WasmAdder {
  const element = byId('wasm-bar');
  const always = byId<HTMLInputElement>('always-fetch-wasm');
  always.checked = alwaysFetchWasm();
  always.addEventListener('change', () => setAlwaysFetchWasm(always.checked));
  const attempted = new Set<string>();
  let message = '';
  const note = (content: string) => {
    message = content;
    render(store.get());
  };

  const supply = (added: readonly SuppliedWasm[]) => {
    if (added.length === 0) {
      return;
    }
    store.update((state) => ({ ...state, wasm: withSupplied(state.wasm, added) }));
    if (store.get().run.result !== null) {
      rerun();
    }
  };

  const useAuto = () => {
    store.update((state) => ({ ...state, settings: { ...state.settings, wasm: 'auto' } }));
    rerun();
  };

  const fetchReference = async (reference: WasmReferenceInfo, asking: boolean) => {
    const url = consentedUrl(reference, asking);
    if (url === null) {
      return;
    }
    attempted.add(reference.url);
    try {
      const bytes = await fetchWasm(url);
      const problem = checkedWasm(reference.name, bytes);
      if (problem !== null) {
        note(problem);
        return;
      }
      supply([{ name: reference.name, bytes, url }]);
    } catch (error) {
      note(`Fetching ${url} failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const render = (state: AppState) => {
    const result = state.run.result;
    const report = parseWasmReport(result?.wasm ?? '');
    const rows: HTMLElement[] = [];
    report.files.forEach((file, index) => {
      const bytes = result?.wasmBytes?.[index];
      const detail = `${file.origin} · ${sizeLabel(file.byteLength)} · ${file.imports} imports, ${file.exports} exports · ${file.renamed} renamed`;
      const save = button('Download', `Download ${file.path}`, () => {
        if (bytes !== undefined) {
          downloadText(fileName(file.path), bytes, 'application/wasm');
        }
      });
      save.dataset.wasmPath = file.path;
      rows.push(row('wasm-row', text('wasm-name', file.path), text('muted', detail), save));
    });
    for (const reference of report.unresolved) {
      const fetchButton = button('Fetch…', `Fetch ${reference.url}`, () => void fetchReference(reference, !alwaysFetchWasm()));
      fetchButton.dataset.wasmUrl = reference.url;
      rows.push(row('wasm-row wasm-missing', text('wasm-name', reference.name), text('muted', `not supplied · loaded from ${reference.url} · drop it here or fetch it`), fetchButton));
    }
    if (report.unchecked) {
      rows.push(row('wasm-row wasm-unchecked', text('wasm-note', UNCHECKED_WARNING), button('Use Auto and rerun', 'Set Wasm check to Auto and run again', useAuto)));
    }
    if (state.wasm.length > 0) {
      const names = state.wasm.map((file) => file.name).join(', ');
      rows.push(row('wasm-row', text('muted', `Supplied: ${names}`), button('Clear', 'Forget the supplied wasm files', () => store.update((current) => ({ ...current, wasm: [] })))));
    }
    if (message !== '') {
      rows.push(row('wasm-row', text('wasm-note', message)));
    }
    element.hidden = rows.length === 0;
    element.replaceChildren(...rows);
  };

  let shownResult: AppState['run']['result'] = null;
  let shownWasm: AppState['wasm'] | null = null;
  store.subscribe((state) => {
    const result = state.run.result;
    if (result === shownResult && state.wasm === shownWasm) {
      return;
    }
    const fresh = result !== shownResult;
    shownResult = result;
    shownWasm = state.wasm;
    render(state);
    if (!fresh || result === null || !alwaysFetchWasm()) {
      return;
    }
    for (const reference of parseWasmReport(result.wasm ?? '').unresolved) {
      if (isAbsoluteUrl(reference.url) && !attempted.has(reference.url)) {
        void fetchReference(reference, false);
      }
    }
  });

  return (files) => {
    void Promise.all(
      files.map(async (file): Promise<SuppliedWasm | null> => {
        const bytes = new Uint8Array(await file.arrayBuffer());
        const problem = checkedWasm(file.name, bytes);
        if (problem !== null) {
          note(problem);
          return null;
        }
        return { name: file.name, bytes, url: null };
      }),
    ).then((loaded) => supply(loaded.filter((file): file is SuppliedWasm => file !== null)));
  };
}
