import type { Zippable } from 'fflate';
import { byId, setText } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import { downloadText } from './details/export';
import { restoredComponents } from './components-tab';
import type { RestoredComponent } from './components/components';
import { currentBundle, type BundleInfo } from './module-tree';

export function isSafeRelative(path: string): boolean {
  if (path === '' || path.startsWith('/') || path.includes('\\')) {
    return false;
  }
  if (path.split('/').some((segment) => segment === '..')) {
    return false;
  }
  const first = path.split('/')[0] ?? '';
  return !(first.length >= 2 && first[1] === ':');
}

export interface ZipPlan {
  readonly files: Zippable;
  readonly refused: readonly string[];
}

const encoder = new TextEncoder();
const strToU8 = (text: string): Uint8Array => encoder.encode(text);
const BUNDLE_JSON = 'bundle.json';
const withoutFinalNewline = (text: string): string => (text.endsWith('\n') ? text.slice(0, -1) : text);
const DEOBFUSCATED = 'deobfuscated.js';
const DEOBFUSCATED_TS = 'deobfuscated.ts';

function place(files: Zippable, refused: string[], path: string, text: string): void {
  if (isSafeRelative(path) && !Object.hasOwn(files, path)) {
    files[path] = strToU8(text);
  } else {
    refused.push(path);
  }
}

export function zipPlan(bundleJson: string | null, bundle: BundleInfo | null, codes: readonly string[], stdout: string, components: readonly RestoredComponent[] = []): ZipPlan {
  const files: Zippable = Object.assign(Object.create(null) as Zippable, { [DEOBFUSCATED]: strToU8(withoutFinalNewline(stdout)) });
  if (bundleJson !== null) {
    files[BUNDLE_JSON] = strToU8(bundleJson);
  }
  const refused: string[] = [];
  bundle?.modules.forEach((module, index) => place(files, refused, module.path, codes[index] ?? ''));
  for (const component of components) {
    place(files, refused, component.path, component.code);
  }
  return { files, refused };
}

function bundled(state: AppState): boolean {
  return currentBundle(state) !== null && state.run.result?.bundleJson !== null && !state.settings.single;
}

function zipping(state: AppState): boolean {
  return bundled(state) || restoredComponents(state).files.length > 0;
}

export function mountDownload(store: Store<AppState>, shown: () => string): void {
  const button = byId<HTMLButtonElement>('download');
  const note = byId('download-note');
  button.addEventListener('click', () => {
    const state = store.get();
    const result = state.run.result;
    const bundle = currentBundle(state);
    if (result === null) {
      return;
    }
    const index = state.selectedModule;
    const stdout = index === null ? shown() : result.stdout;
    if (!zipping(state)) {
      downloadText(state.settings.types === 'ts' ? DEOBFUSCATED_TS : DEOBFUSCATED, withoutFinalNewline(stdout), 'text/javascript');
      return;
    }
    const codes = index === null ? result.modules : result.modules.map((code, at) => (at === index ? shown() : code));
    const tree = bundled(state) ? bundle : null;
    const plan = zipPlan(tree === null ? null : result.bundleJson, tree, codes, stdout, restoredComponents(state).files);
    import('fflate').then(
      ({ zipSync }) => downloadText('deobfuscated.zip', zipSync(plan.files, { level: 6 }), 'application/zip'),
      () => {
        note.hidden = false;
        setText(note, 'The zip library failed to load.');
      },
    );
    note.hidden = plan.refused.length === 0;
    setText(note, plan.refused.length === 0 ? '' : `Left out ${plan.refused.length} unsafe path(s): ${plan.refused.join(', ')}`);
  });
  store.subscribe((state) => {
    button.disabled = state.run.result === null;
    setText(byId('download-label'), zipping(state) ? 'Download .zip' : state.settings.types === 'ts' ? 'Download .ts' : 'Download .js');
  });
}
