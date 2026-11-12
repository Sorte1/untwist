import type { Zippable } from 'fflate';
import { byId, setText } from '../dom';
import type { AppState } from '../state';
import type { Store } from '../store';
import { downloadText } from './details/export';
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

export function zipPlan(bundleJson: string, bundle: BundleInfo, codes: readonly string[], stdout: string): ZipPlan {
  const files: Zippable = Object.assign(Object.create(null) as Zippable, {
    [BUNDLE_JSON]: strToU8(bundleJson),
    [DEOBFUSCATED]: strToU8(withoutFinalNewline(stdout)),
  });
  const refused: string[] = [];
  bundle.modules.forEach((module, index) => {
    if (isSafeRelative(module.path) && !Object.hasOwn(files, module.path)) {
      files[module.path] = strToU8(codes[index] ?? '');
    } else {
      refused.push(module.path);
    }
  });
  return { files, refused };
}

function zipping(state: AppState): boolean {
  return currentBundle(state) !== null && state.run.result?.bundleJson !== null && !state.settings.single;
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
    if (!zipping(state) || bundle === null || result.bundleJson === null) {
      downloadText(DEOBFUSCATED, withoutFinalNewline(stdout), 'text/javascript');
      return;
    }
    const codes = index === null ? result.modules : result.modules.map((code, at) => (at === index ? shown() : code));
    const plan = zipPlan(result.bundleJson, bundle, codes, stdout);
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
    setText(byId('download-label'), zipping(state) ? 'Download .zip' : 'Download .js');
  });
}
